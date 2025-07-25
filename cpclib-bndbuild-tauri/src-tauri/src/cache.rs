use std::io::Write;
use std::ops::Deref;
use std::sync::Arc;

use cpclib_bndbuild::BndBuilder;
use cpclib_bndbuild::cpclib_common::event::EventObserver;
use cpclib_bndbuild::event::{BndBuilderObserved, BndBuilderObserver, BndBuilderObserverRc};
use log;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::CommandEvent;

#[derive(Debug)]
pub struct TauriBndBuilderObserver {
    app_handle: AppHandle
}

impl TauriBndBuilderObserver {
    pub fn new(app_handle: &AppHandle) -> Self {
        Self {
            app_handle: app_handle.clone()
        }
    }
}

impl EventObserver for TauriBndBuilderObserver {
    fn emit_stdout(&self, s: &str) {
        self.app_handle.emit("event-stdout", s).unwrap();
    }

    fn emit_stderr(&self, s: &str) {
        self.app_handle.emit("event-stderr", s).unwrap();
    }
}

impl BndBuilderObserver for TauriBndBuilderObserver {
    fn update(&mut self, event: cpclib_bndbuild::event::BndBuilderEvent) {
        match event {
            cpclib_bndbuild::event::BndBuilderEvent::ChangeState(state) => {
                // ignore that or use it to change a css class
            },
            cpclib_bndbuild::event::BndBuilderEvent::StartRule { rule, nb, out_of } => {
                #[derive(Serialize, Clone)]
                struct StartRule<'a> {
                    rule: &'a str,
                    nb: usize,
                    out_of: usize
                }
                self.app_handle
                    .emit(
                        "event-start_rule",
                        StartRule {
                            rule: rule.as_str(),
                            nb,
                            out_of
                        }
                    )
                    .unwrap();
            },
            cpclib_bndbuild::event::BndBuilderEvent::StopRule(rule) => {
                self.app_handle
                    .emit("event-stop_rule", rule.as_str())
                    .unwrap();
                self.app_handle
                    .notification()
                    .builder()
                    .title("BNDBuild - Rule built")
                    .body(format!("{} is successful", rule.as_str()))
                    .show()
                    .unwrap()
            },
            cpclib_bndbuild::event::BndBuilderEvent::FailedRule(utf8_path) => {
                self.app_handle
                    .emit("event-failed_rule", utf8_path)
                    .unwrap();
                self.app_handle
                    .notification()
                    .builder()
                    .title("BNDBuild - Rule failed")
                    .body(format!("{utf8_path} failed"))
                    .show()
                    .unwrap()
            },
            // TODO it would be better to add id to tasks to identify them
            cpclib_bndbuild::event::BndBuilderEvent::StartTask(utf8_path, task) => {
                #[derive(Serialize, Clone)]
                struct StartTask<'a> {
                    rule: Option<&'a str>,
                    cmd: String,
                    task_id: usize
                }
                self.app_handle
                    .emit(
                        "event-task_start",
                        StartTask {
                            rule: utf8_path.as_ref().map(|s| s.as_str()),
                            cmd: task.to_string(),
                            task_id: task.id()
                        }
                    )
                    .unwrap();
            },
            cpclib_bndbuild::event::BndBuilderEvent::StopTask(utf8_path, task, duration) => {
                #[derive(Serialize, Clone)]
                struct StopTask<'a> {
                    rule: Option<&'a str>,
                    task_id: usize,
                    duration_milliseconds: usize
                }
                self.app_handle
                    .emit(
                        "event-task_stop",
                        StopTask {
                            rule: utf8_path.as_ref().map(|s| s.as_str()),
                            task_id: task.id(),
                            duration_milliseconds: duration.as_millis() as _
                        }
                    )
                    .unwrap();
            },
            // TODO a channel way
            cpclib_bndbuild::event::BndBuilderEvent::TaskStdout(utf8_path, task, content) => {
                #[derive(Serialize, Clone)]
                struct TaskStdout<'a> {
                    rule: &'a str,
                    task_id: usize,
                    content: &'a str
                }
                self.app_handle
                    .emit(
                        "event-task_stdout",
                        TaskStdout {
                            rule: utf8_path.as_str(),
                            task_id: task.id(),
                            content
                        }
                    )
                    .unwrap();
            },
            cpclib_bndbuild::event::BndBuilderEvent::TaskStderr(utf8_path, task, content) => {
                #[derive(Serialize, Clone)]
                struct TaskStderr<'a> {
                    rule: &'a str,
                    task_id: usize,
                    content: &'a str
                }
                self.app_handle
                    .emit(
                        "event-task_stderr",
                        TaskStderr {
                            rule: utf8_path.as_str(),
                            task_id: task.id(),
                            content
                        }
                    )
                    .unwrap();
            },
            cpclib_bndbuild::event::BndBuilderEvent::Stdout(o) => {
                self.app_handle.emit("event-stdout", o).unwrap();
            },
            cpclib_bndbuild::event::BndBuilderEvent::Stderr(o) => {
                self.app_handle.emit("event-stderr", o).unwrap();
            }
        }
    }
}

#[derive(Debug)]
pub struct CachedBndBuilder {
    pub(crate) builder: Arc<BndBuilder>,
    svg: String
}

impl Deref for CachedBndBuilder {
    type Target = BndBuilder;

    fn deref(&self) -> &Self::Target {
        &self.builder
    }
}
impl CachedBndBuilder {
    pub async fn new(mut builder: BndBuilder, app_handle: &AppHandle) -> Result<Self, String> {
        let dot = builder.to_dot(true); // TODO handle compressed and not compressed version and show them on demand
        log::info!("Try to convert {:?}", &dot);

        // XXX the sidecar way does not work when bundled. No time to really look why
        // let shell = app_handle.shell();
        // let sidecar_command = shell.sidecar("dot").unwrap();
        //
        // let svg: String = tauri::async_runtime::block_on(async move {
        // let (mut rx, mut child) = sidecar_command
        // .args(["-Kdot", "-Tsvg"])
        // .set_raw_out(true)
        // .spawn()
        // .unwrap();
        //
        // child.write(dot.as_bytes()).unwrap();
        // drop(child); // XXX ensure stdin is closed
        //
        // let mut svg = Vec::new();
        //
        // while let Some(event) = rx.recv().await {
        // match event {
        // CommandEvent::Stdout(line) => {
        // svg.extend(line.into_iter());
        // },
        // CommandEvent::Stderr(line) => {
        // dbg!(String::from_utf8_lossy(&line));
        // },
        // CommandEvent::Error(e) => {
        // return Err(format!("Error with graphviz conversion. {}", e));
        // },
        // CommandEvent::Terminated(terminated_payload) => break,
        // _ => todo!()
        // }
        // }
        //
        // Ok(String::from_utf8_lossy(&svg).into_owned())
        // })?;

        #[cfg(target_os = "windows")]
        let dot_command = {
            // we provide graphviz because every installation is a nightmare on windows
            let resource_path = app_handle
                .path()
                .resolve(
                    "resources/Graphviz-12.2.1-win64/bin/dot.exe",
                    tauri::path::BaseDirectory::Resource
                )
                .map_err(|e| format!("Unable to resolve graphvis path {e}"))?;
            if !dbg!(&resource_path).exists() {
                log::error!(
                    "graphviz does not seem to be reachable {:?}",
                    &resource_path
                );
                return Err(format!("{:?} does not exists", resource_path.to_str()));
            }
            let resource_path = std::path::absolute(&resource_path)
                .map_err(|e| format!("Unable to absolutize {resource_path:?}. {e}"))?;

            resource_path.to_string_lossy().into_owned()
        };
        #[cfg(not(target_os = "windows"))]
        let dot_command = {
            // we expect graphviz to be installed on the host
            "dot"
        };

        log::info!("Try to launch : {:?}", &dot_command);

        // this stil does not seem to work in bundled way
        // let mut child = std::process::Command::new(dot_command)
        // .arg(format!("-Tsvg"))
        // .stdin(std::process::Stdio::piped())
        // .stdout(std::process::Stdio::piped())
        // .spawn()
        // .map_err(|e| format!("Unable to spawn dot. {e}"))?;
        //
        // send the dot file to the program
        // child
        // .stdin
        // .take()
        // .unwrap()
        // .write_all(dot.as_bytes())
        // .map_err(|e| "Error while piping the dot content".to_owned())?;
        // let output = child
        // .wait_with_output()
        // .map_err(|e| "Error while retreiving the dot output".to_owned())?;
        //
        //
        //
        // let svg = String::from_utf8_lossy(&output.stdout).into_owned();

        use tauri_plugin_shell::ShellExt;

        let shell = app_handle.shell();
        let svg: String = {
            let (mut rx, mut child) = shell
                .command(dot_command)
                .args(["-Kdot", "-Tsvg"])
                .set_raw_out(true)
                .spawn()
                .map_err(|e| format!("Unable to spawn dot. {e}"))?;

            child
                .write(dot.as_bytes())
                .map_err(|e| "Error while piping the dot content".to_owned())?;
            drop(child); // XXX ensure stdin is closed

            let mut svg = Vec::new();

            while let Some(event) = rx.recv().await {
                match event {
                    CommandEvent::Stdout(line) => {
                        svg.extend(line.into_iter());
                    },
                    CommandEvent::Stderr(line) => {
                        let line = String::from_utf8_lossy(&line);
                        log::info!("Obtaines line: {}", &line);
                        drop(line);
                    },
                    CommandEvent::Error(e) => {
                        return Err(format!("Error with graphviz conversion. {e}"));
                    },
                    CommandEvent::Terminated(terminated_payload) => break,
                    _ => todo!()
                }
            }

            String::from_utf8_lossy(&svg).into_owned()
        };

        log::info!("Obtained svg {}", &svg);

        builder.add_observer(BndBuilderObserverRc::new(TauriBndBuilderObserver::new(
            app_handle
        )));

        Ok(Self {
            builder: Arc::new(builder),
            svg
        })
    }

    pub fn svg(&self) -> &str {
        &self.svg
    }
}
