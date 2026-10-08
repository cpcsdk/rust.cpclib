//! Drive a live emulator: boot a snapshot or a disc, read and write memory,
//! type, take screenshots - to test a demo or measure a player on real
//! (emulated) hardware.
//!
//! ```python
//! from cpclib_python.emu import Emulator
//!
//! with Emulator("amspiritlite", snapshot="demo.sna") as emu:
//!     emu.read_memory(0x4000, 16)
//!     open("shot.png", "wb").write(emu.screenshot())
//! ```
//!
//! Most emulators need a real display to run against (`headless=True` makes a
//! private virtual one, on Linux). SugarBox, AMSpiriT lite and ACE are driven
//! through their own APIs.

#![allow(unsafe_op_in_unsafe_fn)]

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Mutex;
use std::sync::mpsc::{Sender, channel};
use std::thread::JoinHandle;
use std::time::Duration;

use cpclib_common::camino::{Utf8Path, Utf8PathBuf};
use cpclib_runner::emucontrol::{EmulatorConf, RobotHandle};
use cpclib_runner::event::EventObserver;
use cpclib_runner::runner::emulator::cadence::CadenceVersion;
use cpclib_runner::runner::emulator::caprice_forever::CapriceForeverVersion;
use cpclib_runner::runner::emulator::cpcemu::CpcEmuVersion;
use cpclib_runner::runner::emulator::cpcemupower::CpcEmuPowerVersion;
use cpclib_runner::runner::emulator::emulator1984::Emulator1984Version;
use cpclib_runner::runner::emulator::retrovm::RetroVmVersion;
use cpclib_runner::runner::emulator::{
    AceVersion, AmspiritLiteVersion, AmspiritVersion, CpcecVersion, Emulator, SugarBoxV2Version,
    WinapeVersion
};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyBytes;

/// The emulators' launch chatter goes to stderr.
#[derive(Clone, Copy, Debug, Default)]
struct StderrObserver;

impl EventObserver for StderrObserver {
    fn emit_stdout(&self, s: &str) {
        eprint!("{s}");
    }

    fn emit_stderr(&self, s: &str) {
        eprint!("{s}");
    }
}

/// The emulators a session can drive.
const EMULATORS: &[&str] = &[
    "ace",
    "amspirit",
    "amspiritlite",
    "cadence",
    "capriceforever",
    "cpcemu",
    "cpcec",
    "cpcemupower",
    "emulator1984",
    "retrovm",
    "winape",
    "sugarbox"
];

/// The backends driven through a debug/web API, for which a snapshot given at
/// launch is not reliably honoured: it is loaded through the API as well.
const API_DRIVEN_SNAPSHOT_LOAD: &[&str] = &["sugarbox", "amspiritlite", "ace"];

fn parse_emulator(name: &str) -> PyResult<Emulator> {
    Ok(match name.to_ascii_lowercase().as_str() {
        "ace" => Emulator::Ace(AceVersion::default()),
        "amspirit" => Emulator::Amspirit(AmspiritVersion::default()),
        "amspiritlite" => Emulator::AmspiritLite(AmspiritLiteVersion::default()),
        "cadence" => Emulator::Cadence(CadenceVersion::default()),
        "capriceforever" => Emulator::CapriceForever(CapriceForeverVersion::default()),
        "cpcemu" => Emulator::CpcEmu(CpcEmuVersion::default()),
        "cpcec" => Emulator::Cpcec(CpcecVersion::default()),
        "cpcemupower" => Emulator::CpcEmuPower(CpcEmuPowerVersion::default()),
        "emulator1984" => Emulator::Emulator1984(Emulator1984Version::default()),
        "retrovm" => Emulator::RetroVm(RetroVmVersion::default()),
        "winape" => Emulator::Winape(WinapeVersion::default()),
        "sugarbox" | "sugarboxv2" => Emulator::SugarBoxV2(SugarBoxV2Version::default()),
        other => {
            return Err(PyValueError::new_err(format!(
                "unknown emulator '{other}' - expected one of: {}",
                EMULATORS.join(", ")
            )));
        }
    })
}

/// What the actor thread, the only one that touches the emulator, is asked to do.
enum Message {
    Run(Box<dyn FnOnce(&mut RobotHandle) + Send>),
    Close
}

/// A running emulator.
#[pyclass(name = "Emulator")]
pub struct PyEmulator {
    name: String,
    tx: Mutex<Option<Sender<Message>>>,
    thread: Mutex<Option<JoinHandle<()>>>
}

impl PyEmulator {
    /// Runs `f` on the emulator's thread, with the GIL released, and waits for it.
    fn ask<R: Send + 'static>(
        &self,
        py: Python,
        f: impl FnOnce(&mut RobotHandle) -> Result<R, String> + Send + 'static
    ) -> PyResult<R> {
        let tx = self
            .tx
            .lock()
            .expect("emulator lock poisoned")
            .clone()
            .ok_or_else(|| PyRuntimeError::new_err("the emulator is closed"))?;
        let (reply_tx, reply_rx) = channel();
        tx.send(Message::Run(Box::new(move |robot| {
            // a driver that panics (a window that vanished, ...) is an error
            // for the caller, not a dead session without an answer
            let result = catch_unwind(AssertUnwindSafe(|| f(robot)))
                .unwrap_or_else(|_| Err("the emulator driver failed unexpectedly".to_string()));
            let _ = reply_tx.send(result);
        })))
        .map_err(|_| PyRuntimeError::new_err("the emulator is gone"))?;
        py.detach(move || reply_rx.recv())
            .map_err(|_| PyRuntimeError::new_err("the emulator is gone"))?
            .map_err(PyRuntimeError::new_err)
    }

    /// Asks the emulator to close; with `wait`, until it has (the window is
    /// then gone: a following launch does not find it).
    fn shutdown(&self, py: Option<Python>, wait: bool) {
        if let Some(tx) = self.tx.lock().expect("emulator lock poisoned").take() {
            let _ = tx.send(Message::Close);
        }
        if wait && let Some(thread) = self.thread.lock().expect("emulator lock poisoned").take() {
            let join = move || {
                let _ = thread.join();
            };
            match py {
                Some(py) => py.detach(join),
                None => join()
            }
        }
    }
}

#[pymethods]
impl PyEmulator {
    /// Launches `emulator` (see `emulators()`), optionally booting `snapshot`
    /// or with discs in the drives. Blocks until it is up. `headless=True` runs
    /// it on a private virtual display (Linux), which needs the machine to be
    /// otherwise free of other such sessions.
    #[new]
    #[pyo3(signature = (emulator="ace", snapshot=None, drive_a=None, drive_b=None, headless=false))]
    fn new(
        py: Python,
        emulator: &str,
        snapshot: Option<String>,
        drive_a: Option<String>,
        drive_b: Option<String>,
        headless: bool
    ) -> PyResult<Self> {
        let name = emulator.to_ascii_lowercase();
        let emulator = parse_emulator(&name)?;
        let snapshot = snapshot.map(Utf8PathBuf::from);
        let conf = EmulatorConf::builder()
            .transparent(false)
            .break_on_bad_vbl(false)
            .break_on_bad_hbl(false)
            .maybe_snapshot(snapshot.clone())
            .maybe_drive_a(drive_a.map(Utf8PathBuf::from))
            .maybe_drive_b(drive_b.map(Utf8PathBuf::from))
            .build();

        let (tx, rx) = channel::<Message>();
        let (ready_tx, ready_rx) = channel::<Result<(), String>>();
        let thread = std::thread::Builder::new()
            .name(format!("cpclib-python-emulator-{name}"))
            .spawn(move || {
                let observer = StderrObserver;
                let launched = if headless {
                    RobotHandle::launch_headless(&emulator, &conf, &observer)
                }
                else {
                    RobotHandle::launch(&emulator, &conf, &observer)
                };
                let mut robot = match launched {
                    Ok(robot) => {
                        let _ = ready_tx.send(Ok(()));
                        robot
                    },
                    Err(e) => {
                        let _ = ready_tx.send(Err(e));
                        return;
                    }
                };
                while let Ok(message) = rx.recv() {
                    match message {
                        Message::Run(job) => job(&mut robot),
                        Message::Close => {
                            robot.close();
                            break;
                        }
                    }
                }
            })
            .map_err(|e| {
                PyRuntimeError::new_err(format!("cannot start the emulator thread: {e}"))
            })?;

        py.detach(move || ready_rx.recv())
            .map_err(|_| {
                PyRuntimeError::new_err("the emulator launch ended without reporting readiness")
            })?
            .map_err(PyRuntimeError::new_err)?;

        let session = Self {
            name: name.clone(),
            tx: Mutex::new(Some(tx)),
            thread: Mutex::new(Some(thread))
        };
        if let Some(snapshot) = snapshot
            && API_DRIVEN_SNAPSHOT_LOAD.contains(&name.as_str())
        {
            // the debug server binds early, but not necessarily before the window shows
            py.detach(|| std::thread::sleep(Duration::from_millis(750)));
            session.ask(py, move |robot| robot.load_snapshot(&snapshot))?;
        }
        Ok(session)
    }

    /// The emulator's name.
    #[getter]
    fn name(&self) -> &str {
        &self.name
    }

    /// Types `text` (`\n` is Enter).
    fn type_text(&self, py: Python, text: String) -> PyResult<()> {
        self.ask(py, move |robot| {
            robot.type_text(&text);
            Ok(())
        })
    }

    /// `count` bytes of the CPU's memory from `address`.
    fn read_memory<'py>(
        &self,
        py: Python<'py>,
        address: u16,
        count: u16
    ) -> PyResult<Bound<'py, PyBytes>> {
        let bytes = self.ask(py, move |robot| robot.read_memory(address, count))?;
        Ok(PyBytes::new(py, &bytes))
    }

    /// Writes `data` at `address` (not every emulator can).
    fn write_memory(&self, py: Python, address: u16, data: Vec<u8>) -> PyResult<()> {
        self.ask(py, move |robot| robot.write_memory(address, &data))
    }

    /// Loads a snapshot.
    fn load_snapshot(&self, py: Python, path: String) -> PyResult<()> {
        self.ask(py, move |robot| robot.load_snapshot(Utf8Path::new(&path)))
    }

    /// Inserts a disc image in `drive` (0 = A, 1 = B).
    #[pyo3(signature = (path, drive=0))]
    fn load_disc(&self, py: Python, path: String, drive: u8) -> PyResult<()> {
        self.ask(py, move |robot| {
            robot.load_disc(drive, Utf8Path::new(&path))
        })
    }

    /// The disc in `drive`, as a `.dsk` image (what the program wrote to it).
    #[pyo3(signature = (drive=0))]
    fn save_disc<'py>(&self, py: Python<'py>, drive: u8) -> PyResult<Bound<'py, PyBytes>> {
        let bytes = self.ask(py, move |robot| robot.save_disc(drive))?;
        Ok(PyBytes::new(py, &bytes))
    }

    /// A screenshot, as PNG bytes.
    fn screenshot<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let png = self.ask(py, |robot| robot.screenshot())?;
        Ok(PyBytes::new(py, &png))
    }

    /// Closes the emulator, and waits for it to be gone. Safe to call twice.
    fn close(&self, py: Python) {
        self.shutdown(Some(py), true);
    }

    fn __enter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    #[pyo3(signature = (*_args))]
    fn __exit__(&self, py: Python, _args: &Bound<'_, pyo3::types::PyTuple>) {
        self.shutdown(Some(py), true);
    }

    fn __repr__(&self) -> String {
        let open = self.tx.lock().expect("emulator lock poisoned").is_some();
        format!(
            "Emulator({:?}, {})",
            self.name,
            if open { "running" } else { "closed" }
        )
    }
}

impl Drop for PyEmulator {
    fn drop(&mut self) {
        self.shutdown(None, false);
    }
}

/// The emulators `Emulator` can drive.
#[pyfunction]
fn emulators() -> Vec<&'static str> {
    EMULATORS.to_vec()
}

pub fn emu(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyEmulator>()?;
    m.add_function(wrap_pyfunction!(emulators, m)?)?;
    Ok(())
}
