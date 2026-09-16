//! Emulator session management: one dedicated OS thread per live
//! `RobotHandle`, driven by a command channel (actor pattern).
//!
//! `RobotHandle`'s methods are synchronous/blocking (real process I/O,
//! window automation), while the MCP server runs on tokio - calling them
//! directly from an async tool handler would stall a tokio worker for the
//! duration of a real autotype sequence or window-capture screenshot. Each
//! session gets its own thread instead; a `std::sync::mpsc` channel is the
//! mailbox, which also **is** the concurrency answer: one mailbox per
//! session serializes calls against the same session automatically, while
//! different sessions run fully in parallel on their own threads.
//!
//! Sessions are named by an explicit UUID rather than one implicit global
//! session, so multiple interleaved tool-call streams never cross-talk, and
//! `list_sessions` can show what's actually alive. An idle-timeout sweep
//! force-closes sessions nobody has touched in a while so a forgotten
//! `start_emulator` doesn't leak an emulator process forever.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use camino::Utf8PathBuf;
use cpclib_runner::emucontrol::{EmulatorConf, RobotHandle};
use cpclib_runner::event::EventObserver;
use cpclib_runner::runner::emulator::Emulator;
use tokio::sync::oneshot;

use crate::error::{ToolError, ToolErrorKind};

/// Routes `Robot`/`cpclib_runner` progress output to `tracing` (stderr)
/// instead of stdout - stdout is reserved for MCP protocol frames.
#[derive(Debug, Clone, Copy, Default)]
struct TracingObserver;

impl EventObserver for TracingObserver {
    fn emit_stdout(&self, s: &str) {
        tracing::info!(target: "cpclib_mcp::robot", "{}", s.trim_end());
    }

    fn emit_stderr(&self, s: &str) {
        tracing::warn!(target: "cpclib_mcp::robot", "{}", s.trim_end());
    }
}

/// One request to a session's actor thread. Every variant carries its own
/// one-shot reply channel; `Close` is the only one the actor loop itself
/// intercepts to know when to stop.
enum Command {
    TypeText(String, oneshot::Sender<Result<(), String>>),
    ReadMemory {
        address: u16,
        count: u16,
        reply: oneshot::Sender<Result<Vec<u8>, String>>
    },
    WriteMemory {
        address: u16,
        data: Vec<u8>,
        reply: oneshot::Sender<Result<(), String>>
    },
    LoadSnapshot {
        path: Utf8PathBuf,
        reply: oneshot::Sender<Result<(), String>>
    },
    LoadDisc {
        drive: u8,
        path: Utf8PathBuf,
        reply: oneshot::Sender<Result<(), String>>
    },
    SaveDisc {
        drive: u8,
        reply: oneshot::Sender<Result<Vec<u8>, String>>
    },
    Screenshot {
        reply: oneshot::Sender<Result<Vec<u8>, String>>
    },
    Close {
        reply: oneshot::Sender<()>
    }
}

struct SessionEntry {
    tx: std::sync::mpsc::Sender<Command>,
    last_used: Arc<Mutex<Instant>>,
    emulator: String,
    started_at: Instant
}

/// One live session's public shape, as reported by `list_sessions`.
pub struct SessionInfo {
    pub id: String,
    pub emulator: String,
    pub idle_seconds: u64,
    pub age_seconds: u64
}

pub struct SessionManager {
    sessions: Mutex<HashMap<String, SessionEntry>>,
    idle_timeout: Duration
}

impl SessionManager {
    pub fn new(idle_timeout: Duration) -> Arc<Self> {
        Arc::new(Self {
            sessions: Mutex::new(HashMap::new()),
            idle_timeout
        })
    }

    /// Starts the periodic idle-timeout sweep. Must be called from inside a
    /// running tokio runtime (e.g. right after `#[tokio::main]` enters).
    pub fn spawn_idle_sweep(self: &Arc<Self>) {
        let manager = Arc::clone(self);
        tokio::spawn(async move {
            let sweep_interval = Duration::from_secs(30).min(manager.idle_timeout);
            loop {
                tokio::time::sleep(sweep_interval).await;
                manager.sweep_idle().await;
            }
        });
    }

    async fn sweep_idle(&self) {
        let expired: Vec<String> = {
            let sessions = self.sessions.lock().unwrap();
            sessions
                .iter()
                .filter(|(_, entry)| {
                    entry.last_used.lock().unwrap().elapsed() >= self.idle_timeout
                })
                .map(|(id, _)| id.clone())
                .collect()
        };
        for id in expired {
            tracing::warn!(
                target: "cpclib_mcp::session",
                session_id = %id,
                "force-closing idle emulator session (past the idle timeout)"
            );
            let _ = self.close(&id).await;
        }
    }

    fn touch(&self, id: &str) -> Result<std::sync::mpsc::Sender<Command>, ToolError> {
        let sessions = self.sessions.lock().unwrap();
        let entry = sessions.get(id).ok_or_else(|| {
            ToolError::new(
                ToolErrorKind::Robot,
                format!("session '{id}' does not exist or has expired")
            )
        })?;
        *entry.last_used.lock().unwrap() = Instant::now();
        Ok(entry.tx.clone())
    }

    async fn dispatch<T: Send + 'static>(
        &self,
        id: &str,
        build: impl FnOnce(oneshot::Sender<T>) -> Command
    ) -> Result<T, ToolError> {
        let tx = self.touch(id)?;
        let (reply_tx, reply_rx) = oneshot::channel();
        tx.send(build(reply_tx)).map_err(|_| {
            ToolError::new(
                ToolErrorKind::Robot,
                format!("session '{id}'s actor thread is gone")
            )
        })?;
        reply_rx.await.map_err(|_| {
            ToolError::new(
                ToolErrorKind::Robot,
                format!("session '{id}'s actor thread dropped the reply without answering")
            )
        })
    }

    fn robot_err(id: &str, e: String) -> ToolError {
        ToolError::with_details(
            ToolErrorKind::Robot,
            e,
            serde_json::json!({ "session_id": id })
        )
    }

    /// Launches a new emulator session on its own actor thread. Blocking
    /// (a real process spawn plus a fixed settle sleep) - awaited here, but
    /// the actual work happens off the tokio runtime, on the new thread.
    pub async fn start(
        &self,
        emulator: Emulator,
        emulator_label: String,
        conf: EmulatorConf
    ) -> Result<String, ToolError> {
        let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<Command>();
        let (ready_tx, ready_rx) = oneshot::channel::<Result<(), String>>();

        std::thread::Builder::new()
            .name(format!("cpclib-mcp-robot-{emulator_label}"))
            .spawn(move || {
                let observer = TracingObserver;
                let mut robot = match RobotHandle::launch(&emulator, &conf, &observer) {
                    Ok(robot) => {
                        let _ = ready_tx.send(Ok(()));
                        robot
                    },
                    Err(e) => {
                        let _ = ready_tx.send(Err(e));
                        return;
                    }
                };
                while let Ok(cmd) = cmd_rx.recv() {
                    match cmd {
                        Command::TypeText(text, reply) => {
                            robot.type_text(&text);
                            let _ = reply.send(Ok(()));
                        },
                        Command::ReadMemory {
                            address,
                            count,
                            reply
                        } => {
                            let _ = reply.send(robot.read_memory(address, count));
                        },
                        Command::WriteMemory {
                            address,
                            data,
                            reply
                        } => {
                            let _ = reply.send(robot.write_memory(address, &data));
                        },
                        Command::LoadSnapshot { path, reply } => {
                            let _ = reply.send(robot.load_snapshot(&path));
                        },
                        Command::LoadDisc { drive, path, reply } => {
                            let _ = reply.send(robot.load_disc(drive, &path));
                        },
                        Command::SaveDisc { drive, reply } => {
                            let _ = reply.send(robot.save_disc(drive));
                        },
                        Command::Screenshot { reply } => {
                            let _ = reply.send(robot.screenshot());
                        },
                        Command::Close { reply } => {
                            robot.close();
                            let _ = reply.send(());
                            break;
                        }
                    }
                }
            })
            .map_err(|e| ToolError::io(format!("cannot spawn the session's actor thread: {e}")))?;

        ready_rx
            .await
            .map_err(|_| {
                ToolError::new(
                    ToolErrorKind::Robot,
                    "the emulator launch thread ended without reporting readiness"
                )
            })?
            .map_err(|e| ToolError::new(ToolErrorKind::Robot, e))?;

        let id = uuid::Uuid::new_v4().to_string();
        self.sessions.lock().unwrap().insert(
            id.clone(),
            SessionEntry {
                tx: cmd_tx,
                last_used: Arc::new(Mutex::new(Instant::now())),
                emulator: emulator_label,
                started_at: Instant::now()
            }
        );
        Ok(id)
    }

    pub async fn type_text(&self, id: &str, text: String) -> Result<(), ToolError> {
        self.dispatch(id, |reply| Command::TypeText(text, reply))
            .await?
            .map_err(|e| Self::robot_err(id, e))
    }

    pub async fn read_memory(&self, id: &str, address: u16, count: u16) -> Result<Vec<u8>, ToolError> {
        self.dispatch(id, |reply| {
            Command::ReadMemory {
                address,
                count,
                reply
            }
        })
        .await?
        .map_err(|e| Self::robot_err(id, e))
    }

    pub async fn write_memory(&self, id: &str, address: u16, data: Vec<u8>) -> Result<(), ToolError> {
        self.dispatch(id, |reply| {
            Command::WriteMemory {
                address,
                data,
                reply
            }
        })
        .await?
        .map_err(|e| Self::robot_err(id, e))
    }

    pub async fn load_snapshot(&self, id: &str, path: Utf8PathBuf) -> Result<(), ToolError> {
        self.dispatch(id, |reply| Command::LoadSnapshot { path, reply })
            .await?
            .map_err(|e| Self::robot_err(id, e))
    }

    pub async fn load_disc(&self, id: &str, drive: u8, path: Utf8PathBuf) -> Result<(), ToolError> {
        self.dispatch(id, |reply| {
            Command::LoadDisc {
                drive,
                path,
                reply
            }
        })
        .await?
        .map_err(|e| Self::robot_err(id, e))
    }

    pub async fn save_disc(&self, id: &str, drive: u8) -> Result<Vec<u8>, ToolError> {
        self.dispatch(id, |reply| Command::SaveDisc { drive, reply })
            .await?
            .map_err(|e| Self::robot_err(id, e))
    }

    pub async fn screenshot(&self, id: &str) -> Result<Vec<u8>, ToolError> {
        self.dispatch(id, |reply| Command::Screenshot { reply })
            .await?
            .map_err(|e| Self::robot_err(id, e))
    }

    pub async fn close(&self, id: &str) -> Result<(), ToolError> {
        let removed = self.sessions.lock().unwrap().remove(id);
        let Some(entry) = removed
        else {
            return Err(ToolError::new(
                ToolErrorKind::Robot,
                format!("session '{id}' does not exist or has already expired")
            ));
        };
        let (reply_tx, reply_rx) = oneshot::channel();
        if entry.tx.send(Command::Close { reply: reply_tx }).is_ok() {
            let _ = reply_rx.await;
        }
        Ok(())
    }

    pub async fn close_all(&self) {
        let ids: Vec<String> = self.sessions.lock().unwrap().keys().cloned().collect();
        for id in ids {
            let _ = self.close(&id).await;
        }
    }

    pub fn list(&self) -> Vec<SessionInfo> {
        let sessions = self.sessions.lock().unwrap();
        sessions
            .iter()
            .map(|(id, entry)| {
                SessionInfo {
                    id: id.clone(),
                    emulator: entry.emulator.clone(),
                    idle_seconds: entry.last_used.lock().unwrap().elapsed().as_secs(),
                    age_seconds: entry.started_at.elapsed().as_secs()
                }
            })
            .collect()
    }
}
