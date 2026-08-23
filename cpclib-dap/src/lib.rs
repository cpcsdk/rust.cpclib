//! Source-level debugging for Amstrad CPC assembly, over the Debug Adapter
//! Protocol.
//!
//! The emulator we drive (1984js) already implements a DAP server - stepping,
//! memory, disassembly, instruction breakpoints. What it cannot do is talk
//! about *source*: its `stackTrace` returns `line: 0` and an address, and it
//! only accepts breakpoints as addresses. Everything an editor wants is phrased
//! in files and lines.
//!
//! So this is not a debugger. It is the translation between the two, sitting
//! between the editor (north) and the emulator (south):
//!
//! * requests that are purely about source - `setBreakpoints` - are answered
//!   here, turned into addresses, and forwarded as `setInstructionBreakpoints`;
//! * answers that mention an address - `stackTrace` - are annotated on the way
//!   back with the file and line that address came from;
//! * everything else - stepping, memory, disassembly - is forwarded untouched.
//!
//! The mapping comes from [`cpclib_project::srcmap::SourceMap`], built by the
//! assembler during its listing pass.

pub mod amspiritlite;
pub mod basic;
pub mod callstack;
pub mod disassemble;
pub mod inspect;
pub mod launch;
pub mod peer;
pub mod protocol;
pub mod session;

use std::io::{Read, Write};
use std::path::PathBuf;

use serde_json::{Value, json};

/// A peer reached through the loopback server: frames out over SSE, frames in
/// over POST.
struct ServedPeer {
    server: cpclib_runner::web::ServerHandle,
    buffer: Vec<u8>
}

impl peer::DapPeer for ServedPeer {
    fn send(&mut self, message: Value) -> std::io::Result<()> {
        // Unframed: the transport downstream is Server-Sent Events, which is
        // line-oriented, and the page rebuilds the `Content-Length` frame the
        // emulator's parser wants. See `web::server::event_stream`.
        let body = serde_json::to_string(&message)?;
        self.server.send(body).map_err(std::io::Error::other)
    }

    fn drain(&mut self) -> Vec<Value> {
        while let Some(frame) = self.server.try_recv() {
            self.buffer.extend_from_slice(frame.as_bytes());
        }
        protocol::decode(&mut self.buffer)
    }

    /// Nothing to do with it: 1984js implements `next` itself, and decides
    /// what a step over means inside the emulator where we cannot reach.
    /// Stepping over a `defs` there is still one `NOP` per press.
    fn note_line_at_pc(&mut self, _line: peer::LineAtPc) {}
}

/// Which emulator a session is talking to.
///
/// Two backends, chosen by the launch configuration: the wasm emulator served
/// in an editor tab, and AMSpiriT Lite in its own window. They differ in almost
/// everything except that both end up behind [`peer::DapPeer`] - which is the
/// point of that trait, and why the session above knows about neither.
enum Backend {
    /// 1984js, reached through the loopback server: frames out over SSE, in
    /// over POST.
    Served(ServedPeer),
    /// AMSpiriT Lite, reached over its HTTP debug API.
    AmspiritLite(amspiritlite::AmspiritLitePeer)
}

impl peer::DapPeer for Backend {
    fn send(&mut self, message: Value) -> std::io::Result<()> {
        transcript().record("-> emulator", &message);
        match self {
            Self::Served(peer) => peer.send(message),
            Self::AmspiritLite(peer) => peer.send(message)
        }
    }

    fn drain(&mut self) -> Vec<Value> {
        match self {
            Self::Served(peer) => peer.drain(),
            Self::AmspiritLite(peer) => peer.drain()
        }
    }

    // Every arm of this trait has to be forwarded by hand, and the one that
    // was not is what made a step over on a `defs` go on stepping one `NOP` at
    // a time in the real editor while the tests were green: the session's
    // answer reached this enum and stopped here, at a defaulted no-op nobody
    // had noticed inheriting. `note_line_at_pc` has no default any more, so
    // the next one cannot be forgotten silently.
    fn note_line_at_pc(&mut self, line: peer::LineAtPc) {
        match self {
            Self::Served(peer) => peer.note_line_at_pc(line),
            Self::AmspiritLite(peer) => peer.note_line_at_pc(line)
        }
    }

    fn quirks(&self) -> peer::Quirks {
        match self {
            Self::Served(peer) => peer.quirks(),
            Self::AmspiritLite(peer) => peer.quirks()
        }
    }

    fn supports(&self, command: &str) -> bool {
        match self {
            Self::Served(peer) => peer.supports(command),
            Self::AmspiritLite(peer) => peer.supports(command)
        }
    }
}

/// A transcript of the whole conversation, when one was asked for.
///
/// Debugging a debug adapter from the outside is guesswork: a missing pane says
/// nothing about which message failed. Set `[dap] log` in `cpclib-lsp.toml` and
/// every message in both directions is written there, in order, with a marker
/// for who sent it.
///
/// Configured in the same file as everything else deliberately: a debug session
/// is started from several places - a CodeLens, the palette, F5, a `launch.json`
/// - and a setting that has to be repeated in each of them is a setting nobody
/// turns on when they need it.
struct Transcript(Option<std::sync::Mutex<std::fs::File>>);

/// The one transcript, reachable from the backend as well as from the loop.
///
/// The loop sees editor traffic in both directions but only the *answers* the
/// emulator gives - what we ask it goes out from inside the session, several
/// call frames down. That half-blind log cost three rounds of diagnosis on a
/// question the transcript should have answered outright ("which breakpoints
/// were actually armed?"), so the backend writes to it too.
static TRANSCRIPT: std::sync::OnceLock<Transcript> = std::sync::OnceLock::new();

fn transcript() -> &'static Transcript {
    TRANSCRIPT.get_or_init(Transcript::open)
}

impl Transcript {
    /// Read `[dap] log` from the project the adapter was started in.
    ///
    /// The working directory is the editor's workspace folder, which is where
    /// `cpclib-lsp.toml` lives; the same file the language server reads.
    fn open() -> Self {
        let root = std::env::current_dir().ok();

        // Say what was found and where. A transcript that silently fails to
        // appear is worse than none: it looks like the setting was ignored, and
        // the next thing anyone does is change the setting rather than the
        // path.
        let found = cpclib_project::config::find_config_file(root.as_deref());
        match &found {
            Some(path) => eprintln!("cpclib-dap: configuration read from {}", path.display()),
            None => {
                eprintln!(
                    "cpclib-dap: no cpclib-lsp.toml found from {} upwards; using defaults, \
                     so [dap] log is off",
                    root.as_deref()
                        .unwrap_or(std::path::Path::new("."))
                        .display()
                )
            }
        }

        let configured = cpclib_project::config::load_config(root.as_deref())
            .config
            .dap
            .log;
        if configured.trim().is_empty() {
            eprintln!("cpclib-dap: [dap] log is empty, so no transcript is written");
            return Self(None);
        }

        // Relative to the configuration file, not to wherever the adapter
        // happened to be started: the path is written next to the settings that
        // name it, and stays put however the session was launched.
        let path = std::path::Path::new(configured.trim());
        let path = if path.is_absolute() {
            path.to_path_buf()
        }
        else {
            found
                .as_deref()
                .and_then(std::path::Path::parent)
                .map(|dir| dir.join(path))
                .unwrap_or_else(|| root.clone().unwrap_or_default().join(path))
        };

        match std::fs::File::create(&path) {
            Ok(file) => {
                eprintln!(
                    "cpclib-dap: writing the protocol transcript to {}",
                    path.display()
                );
                Self(Some(std::sync::Mutex::new(file)))
            },
            Err(e) => {
                eprintln!("cpclib-dap: cannot write {}: {e}", path.display());
                Self(None)
            }
        }
    }

    fn record(&self, direction: &str, message: &Value) {
        let Some(file) = &self.0
        else {
            return;
        };
        if let Ok(mut file) = file.lock() {
            let _ = writeln!(
                file,
                "{direction} {}",
                serde_json::to_string(message).unwrap_or_default()
            );
            let _ = file.flush();
        }
    }
}

/// Run the adapter: DAP on stdio towards the editor, the served emulator on the
/// other side.
///
/// Stdout carries protocol frames and nothing else - the same discipline the
/// language server keeps - so every diagnostic goes to stderr.
pub fn run_stdio() -> std::io::Result<()> {
    let mut output = std::io::stdout();
    let mut seq = 1i64;
    let mut session: Option<session::Session<Backend>> = None;
    let transcript = transcript();
    // The DAP spec only allows `progressStart`/`progressEnd` towards a
    // client that declared it accepts them, in its own `initialize`
    // arguments - unlike every other event this adapter sends, which no
    // client capability gates. Learned once, at `initialize`, and used for
    // every `launch` after: assembling a program with no cached source map
    // is the one operation slow enough (a real demo's full build, driven a
    // second time) to be worth saying anything about at all.
    let mut client_accepts_progress = false;

    // stdin on its own thread so the emulator can be polled while the editor is
    // quiet, and vice versa.
    let (editor_tx, editor_rx) = std::sync::mpsc::channel::<Value>();
    std::thread::spawn(move || {
        let mut input = std::io::stdin();
        let mut buffer = Vec::new();
        let mut chunk = [0u8; 8192];
        loop {
            match input.read(&mut chunk) {
                Ok(0) | Err(_) => return,
                Ok(read) => buffer.extend_from_slice(&chunk[..read])
            }
            for message in protocol::decode(&mut buffer) {
                if editor_tx.send(message).is_err() {
                    return;
                }
            }
        }
    });

    let emit = |message: &Value, output: &mut std::io::Stdout| -> std::io::Result<()> {
        transcript.record("-> editor  ", message);
        output.write_all(protocol::encode(message).as_bytes())?;
        output.flush()
    };

    loop {
        // Anything the editor said.
        while let Ok(message) = editor_rx.try_recv() {
            transcript.record("<- editor  ", &message);
            let command = message
                .get("command")
                .and_then(Value::as_str)
                .unwrap_or_default();
            match command {
                "initialize" => {
                    client_accepts_progress = message
                        .get("arguments")
                        .and_then(|a| a.get("supportsProgressReporting"))
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    let answer = protocol::response(
                        &message,
                        session::Session::<peer::RecordingPeer>::capabilities(),
                        seq
                    );
                    seq += 1;
                    emit(&answer, &mut output)?;
                },
                "launch" => {
                    if client_accepts_progress {
                        seq += 1;
                        emit(
                            &protocol::event(
                                "progressStart",
                                json!({
                                    "progressId": "launch",
                                    "title": "CPC: preparing the debug session",
                                    "message": "assembling…"
                                }),
                                seq
                            ),
                            &mut output
                        )?;
                    }
                    let outcome = start_session(&message);
                    if client_accepts_progress {
                        seq += 1;
                        emit(
                            &protocol::event(
                                "progressEnd",
                                json!({ "progressId": "launch" }),
                                seq
                            ),
                            &mut output
                        )?;
                    }
                    match outcome {
                        Ok((started, url, notices)) => {
                            session = Some(started);
                            seq += 1;
                            emit(&protocol::response(&message, json!({}), seq), &mut output)?;

                            // What the program asked for that this emulator
                            // cannot do. Said once, at the start, rather than
                            // discovered when a breakpoint quietly fails to
                            // behave as written.
                            for notice in notices {
                                seq += 1;
                                emit(
                                    &protocol::event(
                                        "output",
                                        json!({
                                            "category": "console",
                                            "output": format!("{notice}\n")
                                        }),
                                        seq
                                    ),
                                    &mut output
                                )?;
                            }

                            // An empty URL means there is nothing for the
                            // editor to show: the emulator has its own window.
                            if !url.is_empty() {
                                seq += 1;
                                emit(
                                    &protocol::event(
                                        "cpclib/emulatorReady",
                                        json!({ "url": url }),
                                        seq
                                    ),
                                    &mut output
                                )?;
                            }
                            seq += 1;
                            emit(&protocol::event("initialized", json!({}), seq), &mut output)?;
                        },
                        Err(problem) => {
                            seq += 1;
                            emit(&protocol::failure(&message, &problem, seq), &mut output)?;
                        }
                    }
                },
                "disconnect" | "terminate" => {
                    seq += 1;
                    emit(&protocol::response(&message, json!({}), seq), &mut output)?;
                    return Ok(());
                },
                _ => {
                    match session.as_mut() {
                        Some(session) => {
                            for answer in session.on_editor_message(&message)? {
                                emit(&answer, &mut output)?;
                            }
                        },
                        None => {
                            seq += 1;
                            emit(
                                &protocol::failure(&message, "no debug session is running", seq),
                                &mut output
                            )?;
                        }
                    }
                },
            }
        }

        // Anything the emulator said.
        if let Some(active) = session.as_mut() {
            let incoming = {
                use peer::DapPeer;
                active.peer_mut().drain()
            };
            for message in incoming {
                transcript.record("<- emulator", &message);
                // Attach, watches and breakpoint replies are recognised inside
                // the session, which knows which requests were its own.
                for answer in active.on_emulator_message(&message) {
                    emit(&answer, &mut output)?;
                }
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

/// Build the program and put the emulator in front of it.
///
/// Two ways in, because a project and a single file are different questions:
///
/// * `program` - assemble this file and debug what comes out;
/// * `rule` - build a bndbuild rule the normal way, then debug the snapshot its
///   emulator command names. The project's own build flags apply, which is the
///   whole point: a real demo's assembly arguments live in the build file, not
///   in a debug configuration.
fn start_session(
    request: &Value
) -> Result<(session::Session<Backend>, String, Vec<String>), String> {
    let arguments = request.get("arguments").cloned().unwrap_or(json!({}));
    // Problems found while working out what to debug, said in the console once
    // the session exists. Degrading quietly is what made a broken build look
    // like a working one.
    let mut early_notices: Vec<String> = Vec::new();

    let (snapshot, source_map, program_breakpoints, image, entry_point) = if let Some(rule) =
        arguments.get("rule").and_then(Value::as_str)
    {
        let build_file = match arguments.get("buildFile").and_then(Value::as_str) {
            Some(given) => PathBuf::from(given),
            None => {
                nearest_build_file()
                    .ok_or("no build file found; set \"buildFile\" in the launch configuration")?
            },
        };
        let launched = launch::build_rule_for_debug(&build_file, rule)?;
        let snapshot = std::fs::read(&launched.snapshot).map_err(|e| {
            format!(
                "{} was not produced by '{rule}': {e}",
                launched.snapshot.display()
            )
        })?;

        // The snapshot came from the project's own build; the *source map* has
        // to come from an assemble we drive. The build file says which source
        // it assembles, which is exact - falling back to searching for a `RUN`
        // only when it does not.
        let root = cpclib_project::root::project_root_or_own_dir(&build_file);
        let config = cpclib_project::config::load_config(root.as_deref())
            .config
            .asm;
        // The program's own `BREAKPOINT` directives come from the same
        // assemble as the map; a build we did not drive cannot report them.
        let (map, breakpoints, image, entry_point) = match &launched.entry {
            Some(entry) => {
                // A failed assemble ends the launch, and says why.
                //
                // It used to be swallowed here, which was the worst of both
                // worlds: the emulator started anyway on whatever snapshot
                // happened to be on disc - *the previous build's* - with an
                // empty source map, so nothing mapped, no breakpoint worked,
                // and the program that ran was not the program on screen.
                // Reported as "the process did not stop with an error and
                // launched the previously generated artefact".
                // The build may have left the map behind (`basm --sourcemap`),
                // in which case the program does not have to be assembled a
                // second time to say where its lines went.
                if let Some(cached) = launch::cached_for_debug(entry, &config, &mut early_notices) {
                    early_notices.push(
                        "source map read from the file the build wrote                          (basm --sourcemap) - the program was not assembled again"
                            .to_string()
                    );
                    (
                        cached.source_map,
                        cached.breakpoints,
                        cached.image,
                        cached.entry_point
                    )
                }
                else {
                    let built = launch::assemble_for_debug(entry, &config).map_err(|problem| {
                        format!(
                            "{entry} could not be assembled, so there is nothing to debug:\n\
                         {problem}",
                            entry = entry.display()
                        )
                    })?;
                    (
                        built.source_map,
                        built.breakpoints,
                        built.image,
                        built.entry_point
                    )
                }
            },
            None => {
                let (map, problem) = source_map_for_project(&build_file, &config);
                early_notices.extend(problem);
                (map, Vec::new(), Vec::new(), None)
            }
        };
        (snapshot, map, breakpoints, image, entry_point)
    }
    else if let Some(program) = arguments.get("program").and_then(Value::as_str) {
        let entry = PathBuf::from(program);
        let root = cpclib_project::root::project_root_or_own_dir(&entry);
        let config = cpclib_project::config::load_config(root.as_deref())
            .config
            .asm;
        // A direct-file launch has no build behind it, so nothing else was
        // ever going to write a map for it - unlike the `rule` branch above,
        // where the cache comes from the project's own build. This is
        // cpclib-dap's own cache of its *previous* launch of this exact
        // entry (see `launch::write_program_cache`), so re-launching an
        // unmodified file does not re-assemble it.
        let built = match launch::cached_program_for_debug(&entry, &config, &mut early_notices) {
            Some(cached) => {
                early_notices.push(
                    "source map read from a previous debug launch of this file - it was not assembled again"
                        .to_string()
                );
                cached
            },
            None => launch::assemble_for_debug(&entry, &config)?
        };
        (
            built.snapshot,
            built.source_map,
            built.breakpoints,
            built.image,
            built.entry_point
        )
    }
    else {
        return Err(
            "the launch configuration named neither a \"program\" nor a \"rule\"".to_string()
        );
    };

    // Which emulator this session talks to. They differ in everything except
    // that both end up behind `DapPeer`.
    // The launch configuration wins; the project's own setting is the default,
    // so a rule can be debugged either way without editing the project.
    // The project this session is for, from whichever of the two launch shapes
    // named it.
    let named_path = arguments
        .get("program")
        .or_else(|| arguments.get("buildFile"))
        .and_then(Value::as_str)
        .map(PathBuf::from);
    // `_or_own_dir`, not plain `project_root`: a lone `.asm` with no `.git`/
    // `Makefile`/etc. anywhere above it - a scratch file, an example folder -
    // made `project_root` return `None`, which skipped `find_config_file`
    // entirely and silently defaulted to 1984js regardless of what the
    // project's own `cpclib-lsp.toml` said, since that file was never even
    // looked for. `find_config_file` itself already searches upward from
    // whatever root it is given, so the fallback (the entry's own directory)
    // is enough for it to find a `cpclib-lsp.toml` sitting right there.
    let dap_config = named_path
        .as_deref()
        .and_then(cpclib_project::root::project_root_or_own_dir)
        .map(|root| {
            cpclib_project::config::load_config(Some(root.as_path()))
                .config
                .dap
        })
        .unwrap_or_default();
    let configured_emulator = dap_config.emulator.clone();
    let chosen_emulator = arguments
        .get("emulator")
        .and_then(Value::as_str)
        .unwrap_or(&configured_emulator)
        .to_string();
    let wants_lite = chosen_emulator.eq_ignore_ascii_case("amspiritlite");

    let (backend, url) = if wants_lite {
        // An instance already serving, named by the configuration or found at
        // the emulator's own default port.
        //
        // *Not* launched from here yet: starting it, loading the snapshot and
        // waiting for the port is a separate piece of work, and connecting to
        // one you started yourself is useful today rather than after it.
        // An `endpoint` names an emulator the user is already running - useful
        // for attaching to one with a window arranged how they like it.
        // Without one, it is started here with the program loaded, which is
        // what `F5` should do.
        // `cpclib-lsp.toml` is the place these are set; a launch attribute is
        // an override for the occasion, not the normal way to configure this.
        let attached_to = arguments
            .get("endpoint")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| Some(dap_config.endpoint.clone()).filter(|value| !value.trim().is_empty()));

        let mut started: Option<std::process::Child> = None;
        let endpoint = match attached_to.as_deref() {
            Some(named) => {
                amspiritlite::wait_until_listening(named, std::time::Duration::from_secs(5))
                    .map_err(|e| {
                        format!(
                            "{e}. Start AMSpiriT Lite with --web-server, or drop \
                             \"endpoint\" to have this session start it."
                        )
                    })?;
                named.to_string()
            },
            None => {
                let port = arguments
                    .get("port")
                    .and_then(Value::as_u64)
                    .and_then(|port| u16::try_from(port).ok())
                    .unwrap_or(dap_config.port);
                let (endpoint, child) =
                    amspiritlite::launch(&snapshot, port, &cpclib_common::event::DiscardObserver)?;
                started = Some(child);
                // Said out loud, because the emulator left behind is still on
                // screen and still running the previous build: without this the
                // user has two windows and no way to tell which one this
                // session is driving.
                if !endpoint.ends_with(&format!(":{port}")) {
                    early_notices.push(format!(
                        "port {port} was already answering - an emulator left behind by an \
                         earlier session, most likely - so this one serves on {endpoint} \
                         instead. The older window is not the one being debugged; close it."
                    ));
                }
                endpoint
            }
        };

        let peer = amspiritlite::AmspiritLitePeer::connect(&endpoint)
            .map_err(|e| format!("cannot reach AMSpiriT Lite at {endpoint}: {e}"))?;
        // Only an emulator this session started is closed with it; one the
        // user started is theirs.
        let peer = match started {
            Some(child) => peer.owning(child),
            None => peer
        };
        // No URL for the editor to open.
        //
        // Handing it this endpoint makes it show the emulator's *own* debug
        // client in a tab - and that page is not a viewer. It opens its own
        // event stream and posts `{"paused": false}` in several places, so it
        // resumes the machine behind this session's back: a breakpoint stops
        // the emulator, the page starts it again, and the stop is gone before
        // anyone can see it. AMSpiriT Lite has its own window; there is nothing
        // to show in an editor tab.
        (Backend::AmspiritLite(peer), String::new())
    }
    else {
        let web_root = cpclib_runner::web::js1984::install()?;
        let server = cpclib_runner::web::serve(&web_root, Some(snapshot))
            .map_err(|e| format!("cannot serve the emulator: {e}"))?;
        let url = server.debug_url();
        (
            Backend::Served(ServedPeer {
                server,
                buffer: Vec::new()
            }),
            url
        )
    };

    let mut session = session::Session::new(backend, source_map);
    // The call stack is reconstructed from the stack contents, which needs the
    // program's own bytes to tell a return address from a number shaped like
    // one. Without an image the stack stays one frame deep, as before.
    if !image.is_empty() {
        session = session.with_image(image);
    }
    session = match arguments.get("topOfStack").and_then(Value::as_str) {
        Some(given) => {
            match protocol::parse_address_reference(given).and_then(|a| u16::try_from(a).ok()) {
                Some(top) => session.with_top_of_stack(top),
                None => {
                    // A label is as good an answer as a number, and more
                    // likely what was written.
                    match session
                        .map()
                        .address_of_symbol(given)
                        .and_then(|a| u16::try_from(a).ok())
                    {
                        Some(top) => session.with_top_of_stack(top),
                        None => session.top_of_stack_from_symbols()
                    }
                }
            }
        },
        None => session.top_of_stack_from_symbols()
    };
    // What the sources looked like at build time, so an edit made mid-session
    // is reported rather than quietly placing breakpoints at stale addresses.
    session.record_source_state();
    let mut notices = session.adopt_program_breakpoints(&program_breakpoints);
    notices.splice(0..0, early_notices);
    // Said before the program runs, because after it has run it is too late:
    // an unexplained stop is the thing being prevented.
    notices.extend(session.program_breakpoint_notice());

    // Banking is the one limitation that shows up as *silence* - a stop with
    // no source line - so it is said up front rather than left to be puzzled
    // over.
    if session.map().has_banked_ambiguity() {
        let pages = session
            .map()
            .pages()
            .iter()
            .map(|page| page.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        notices.push(format!(
            "This program has code at the same address in more than one page ({pages}). \
             The emulator does not report which page is selected, so it is worked \
             out by comparing the bytes really in memory against each page's assembled \
             image - accurate unless a routine has patched itself heavily, and reported \
             as unresolved when two pages match equally well."
        ));
    }
    // Labels the launch configuration asked to be told about. Unknown ones are
    // reported rather than silently watched, because a watch on nothing looks
    // exactly like a variable that is never written.
    let watch_labels: Vec<String> = arguments
        .get("watchLabels")
        .and_then(Value::as_array)
        .map(|labels| {
            labels
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    notices.extend(session.add_watch_labels(&watch_labels));

    // `stopOnEntry` is a breakpoint on the address the snapshot starts at.
    // There is nothing else it could be: the emulator begins running the moment
    // the snapshot loads, and the only way to be there first is to have asked
    // it to stop there.
    if arguments.get("stopOnEntry").and_then(Value::as_bool) == Some(true) {
        match entry_point {
            Some(address) => session.stop_on_entry(address),
            None => {
                notices.push(
                    "stopOnEntry was asked for, but the snapshot does not say where it \
                     starts; the program runs from the beginning."
                        .to_string()
                )
            },
        }
    }

    // Only one emulator can be debugged today; a configuration naming another
    // gets that emulator, and is told so rather than left wondering why the
    // setting did nothing.
    if !chosen_emulator.eq_ignore_ascii_case("1984js")
        && !chosen_emulator.eq_ignore_ascii_case("amspiritlite")
    {
        notices.push(format!(
            "\"{chosen_emulator}\" cannot be debugged; this session uses 1984js. \
             Debuggable emulators: 1984js, amspiritlite - set one in the launch \
             configuration, or as `emulator` under `[dap]` in cpclib-lsp.toml."
        ));
    }
    if wants_lite {
        notices.push(format!(
            "Debugging through AMSpiriT Lite, in its own window. Its web page is \
             deliberately not opened here: that page drives the emulator too, and \
             would resume it behind this session. It knows its own banking, so \
             addresses in paged code resolve exactly rather than by comparing bytes."
        ));
    }

    // The emulator refuses everything until attached, so ask immediately; the
    // held breakpoints go out when it answers. These go through
    // `send_own_request` so their answers are recognised as ours and never
    // forwarded to the editor, which numbers its own requests from 1 too.
    session
        .send_own_request("initialize", json!({"supportsMemoryEvent": true}))
        .map_err(|e| e.to_string())?;
    session
        .send_own_request("attach", json!({}))
        .map_err(|e| e.to_string())?;

    Ok((session, url, notices))
}

/// The nearest build file, searching upwards from the working directory.
fn nearest_build_file() -> Option<PathBuf> {
    let names = ["bndbuild.yml", "build.bnd", "bnd.build", "bndbuild.yaml"];
    let mut directory = std::env::current_dir().ok()?;
    loop {
        for name in names {
            let candidate = directory.join(name);
            if candidate.exists() {
                return Some(candidate);
            }
        }
        if !directory.pop() {
            return None;
        }
    }
}

/// The source map for whatever program `build_file` builds.
///
/// Best effort: a project whose entry cannot be resolved still gets a debug
/// session, just one where breakpoints report themselves unverified rather than
/// silently landing at the wrong address.
fn source_map_for_project(
    build_file: &std::path::Path,
    config: &cpclib_project::config::AsmConfig
) -> (cpclib_project::srcmap::SourceMap, Option<String>) {
    let Some(root) = cpclib_project::root::project_root_or_own_dir(build_file)
    else {
        return Default::default();
    };
    let workspace = cpclib_project::entry::scan_workspace(&root);
    let graph = cpclib_project::entry::graph_of(&workspace);

    // The entry cannot be resolved by asking "which program does *this file*
    // belong to" here: the file in hand is the build file, which is not in the
    // include graph at all. What is wanted is the program the project builds -
    // the file that declares the `RUN`.
    let path = match config.entry.as_deref() {
        Some(configured) => root.join(configured),
        None => {
            match graph.sole_run_root() {
                Some(path) => path.to_path_buf(),
                None => return Default::default()
            }
        },
    };
    // Still best-effort - this branch has no entry of its own to insist on, and
    // the snapshot the rule built is debuggable without a map - but the reason
    // is handed back so the console can say why nothing maps, instead of
    // leaving "my breakpoints do nothing" to be puzzled over.
    match launch::assemble_for_debug(&path, config) {
        Ok(built) => (built.source_map, None),
        Err(problem) => {
            (
                Default::default(),
                Some(format!(
                    "no source map: {} could not be assembled, so lines and \
                     breakpoints will not resolve.\n{problem}",
                    path.display()
                ))
            )
        },
    }
}
