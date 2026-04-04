//! Builder pattern implementations for bndbuild commands
//!
//! This module provides type-safe builder patterns for constructing bndbuild
//! commands from Python. Builders construct actual Task instances using the
//! proper internal structures, ensuring compile-time validation.

use cpclib_bndbuild::runners::ay::YmCruncher;
use cpclib_bndbuild::runners::tracker::{SongConverter, Tracker};
use cpclib_bndbuild::task::{InnerTask, StandardTaskArguments, Task};
use cpclib_common::clap::{Arg, ArgMatches, CommandFactory};
use cpclib_sna::SnapshotCli;
use pyo3::prelude::*;

use crate::bndbuild::PyBndTask;

/// Builder for snapshot manipulation commands
///
/// This builder wraps the actual `SnapshotCli` structure from cpclib-sna,
/// ensuring that all parameters match the real command-line interface.
///
/// Example:
/// ```python
/// from cpclib_python.bndbuild import SnapshotBuilder
///
/// builder = SnapshotBuilder()
/// builder.in_snapshot("base.sna")
/// builder.set_token("CRTC_REG:6", "20")
/// builder.put_data(0x4000, 0xFF)  # Write byte 0xFF at address 0x4000
/// builder.output("output.sna")
/// task = builder.build()
/// ```
#[pyclass(name = "SnapshotBuilder")]
#[derive(Default, Clone)]
pub struct PySnapshotBuilder {
    /// Load a snapshot file as base
    in_snapshot: Option<String>,

    /// Output snapshot file
    output: Option<String>,

    /// Set token values (TOKEN, VALUE pairs)
    set_tokens: Vec<(String, String)>,

    /// Put byte values into memory (ADDRESS, BYTE pairs)
    put_data: Vec<(u32, u8)>,

    /// Load files at addresses (FILE, ADDRESS pairs)
    load_files: Vec<(String, String)>,

    /// Snapshot version (1, 2, or 3)
    sna_version: String
}

#[pymethods]
impl PySnapshotBuilder {
    #[new]
    pub fn new() -> Self {
        Self {
            sna_version: "3".to_string(),
            ..Default::default()
        }
    }

    /// Load a snapshot file as base (fluent API - returns self for chaining)
    pub fn in_snapshot(&mut self, path: &str) -> Self {
        self.in_snapshot = Some(path.to_string());
        self.clone()
    }

    /// Set the output snapshot path (fluent API - returns self for chaining)
    pub fn output(&mut self, path: &str) -> Self {
        self.output = Some(path.to_string());
        self.clone()
    }

    /// Set a memory token value (fluent API - returns self for chaining)
    ///
    /// Use token:index format for array values, e.g., "CRTC_REG:6" to set CRTC register 6
    pub fn set_token(&mut self, token: &str, value: &str) -> Self {
        self.set_tokens.push((token.to_string(), value.to_string()));
        self.clone()
    }

    /// Put a byte value at a specific memory address (fluent API - returns self for chaining)
    ///
    /// Example: put_data(0x4000, 0xFF) writes byte 0xFF at address 0x4000
    pub fn put_data(&mut self, address: u32, byte: u8) -> Self {
        self.put_data.push((address, byte));
        self.clone()
    }

    /// Load a file at a specific address (fluent API - returns self for chaining)
    pub fn load(&mut self, file: &str, address: &str) -> Self {
        self.load_files
            .push((file.to_string(), address.to_string()));
        self.clone()
    }

    /// Set snapshot version (1, 2, or 3) (fluent API - returns self for chaining)
    pub fn version(&mut self, ver: &str) -> Self {
        self.sna_version = ver.to_string();
        self.clone()
    }

    /// Build the task from the collected parameters
    ///
    /// This constructs an actual SnapshotCli instance for compile-time type safety,
    /// then converts it to a command-line string representation.
    pub fn build(&self, py: Python) -> PyResult<Py<PyBndTask>> {
        // Build a real SnapshotCli instance from the builder to ensure
        // field names and types match the real CLI structure.
        let snapshot_cli = PySnapshot::build_parser(self);
        let cmd = SnapshotCli::command();
        let argv = snapshot_cli.build_argv();
        let matches = cmd
            .clone()
            .try_get_matches_from(argv)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

        let std_args = StandardTaskArguments::from((&cmd, &matches));
        let inner = InnerTask::Snapshot(std_args);
        let task = Task::from(inner);

        PyBndTask::new_from_task(py, task)
    }

    fn __repr__(&self) -> String {
        format!(
            "SnapshotBuilder(in={:?}, out={:?}, tokens={}, bytes={})",
            self.in_snapshot,
            self.output,
            self.set_tokens.len(),
            self.put_data.len()
        )
    }
}

/// Builder for ArkosTracker3 commands
///
/// Supports AT3 tracker compilation for Amstrad CPC music.
///
/// Example:
/// ```python
/// from cpclib_python.bndbuild import ArkosTracker3Builder
///
/// builder = ArkosTracker3Builder()
/// builder.input("song.aks")
/// builder.output("song.bin")
/// task = builder.build()
/// ```
#[pyclass(name = "ArkosTracker3Builder")]
#[derive(Default, Clone)]
pub struct PyArkosTracker3Builder {
    input: Option<String>,
    output: Option<String>,
    extra_args: Vec<String>
}

#[pymethods]
impl PyArkosTracker3Builder {
    #[new]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the input AKS file (fluent API)
    pub fn input(&mut self, path: &str) -> Self {
        self.input = Some(path.to_string());
        self.clone()
    }

    /// Set the output binary file (fluent API)
    pub fn output(&mut self, path: &str) -> Self {
        self.output = Some(path.to_string());
        self.clone()
    }

    /// Add a raw argument (fluent API)
    pub fn arg(&mut self, arg: &str) -> Self {
        self.extra_args.push(arg.to_string());
        self.clone()
    }

    /// Build the task using proper Tracker struct
    pub fn build(&self, py: Python) -> PyResult<Py<PyBndTask>> {
        let mut args = Vec::new();

        if let Some(ref input) = self.input {
            args.push(shell_escape(input));
        }

        if let Some(ref output) = self.output {
            args.push(shell_escape(output));
        }

        for arg in &self.extra_args {
            args.push(shell_escape(arg));
        }

        let args_string = args.join(" ");
        let std_args = StandardTaskArguments::new(args_string);
        let tracker = Tracker::new_at3_default();
        let inner = InnerTask::with_tracker(tracker, std_args);
        let task = Task::from(inner);

        PyBndTask::new_from_task(py, task)
    }

    fn __repr__(&self) -> String {
        format!(
            "ArkosTracker3Builder(input={:?}, output={:?})",
            self.input, self.output
        )
    }
}

/// Builder for Chipnsfx tracker commands
///
/// Supports Chipnsfx sound effects compilation.
///
/// Example:
/// ```python
/// builder = ChipnsfxBuilder()
/// builder.input("effects.txt")
/// builder.output("effects.bin")
/// task = builder.build()
/// ```
#[pyclass(name = "ChipnsfxBuilder")]
#[derive(Default, Clone)]
pub struct PyChipnsfxBuilder {
    input: Option<String>,
    output: Option<String>,
    extra_args: Vec<String>
}

#[pymethods]
impl PyChipnsfxBuilder {
    #[new]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the input file (fluent API)
    pub fn input(&mut self, path: &str) -> Self {
        self.input = Some(path.to_string());
        self.clone()
    }

    /// Set the output file (fluent API)
    pub fn output(&mut self, path: &str) -> Self {
        self.output = Some(path.to_string());
        self.clone()
    }

    /// Add a raw argument (fluent API)
    pub fn arg(&mut self, arg: &str) -> Self {
        self.extra_args.push(arg.to_string());
        self.clone()
    }

    /// Build the task using proper Tracker struct
    pub fn build(&self, py: Python) -> PyResult<Py<PyBndTask>> {
        let mut args = Vec::new();

        if let Some(ref input) = self.input {
            args.push(shell_escape(input));
        }

        if let Some(ref output) = self.output {
            args.push(shell_escape(output));
        }

        for arg in &self.extra_args {
            args.push(shell_escape(arg));
        }

        let args_string = args.join(" ");
        let std_args = StandardTaskArguments::new(args_string);
        let tracker = Tracker::new_chipnsfx_default();
        let inner = InnerTask::with_tracker(tracker, std_args);
        let task = Task::from(inner);

        PyBndTask::new_from_task(py, task)
    }

    fn __repr__(&self) -> String {
        format!(
            "ChipnsfxBuilder(input={:?}, output={:?})",
            self.input, self.output
        )
    }
}

/// Builder for YM file minimizer (miny) commands
///
/// Compresses YM music files.
///
/// Example:
/// ```python
/// builder = MinyBuilder()
/// builder.input("song.ym")
/// builder.output("song.min.ym")
/// task = builder.build()
/// ```
#[pyclass(name = "MinyBuilder")]
#[derive(Default, Clone)]
pub struct PyMinyBuilder {
    input: Option<String>,
    output: Option<String>
}

#[pymethods]
impl PyMinyBuilder {
    #[new]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the input YM file (fluent API)
    pub fn input(&mut self, path: &str) -> Self {
        self.input = Some(path.to_string());
        self.clone()
    }

    /// Set the output minimized YM file (fluent API)
    pub fn output(&mut self, path: &str) -> Self {
        self.output = Some(path.to_string());
        self.clone()
    }

    /// Build the task using proper YmCruncher struct
    pub fn build(&self, py: Python) -> PyResult<Py<PyBndTask>> {
        let mut args = Vec::new();

        if let Some(ref input) = self.input {
            args.push(shell_escape(input));
        }

        if let Some(ref output) = self.output {
            args.push(shell_escape(output));
        }

        let args_string = args.join(" ");
        let std_args = StandardTaskArguments::new(args_string);
        let inner = InnerTask::with_ym_cruncher(YmCruncher::Miny, std_args);
        let task = Task::from(inner);

        PyBndTask::new_from_task(py, task)
    }

    fn __repr__(&self) -> String {
        format!(
            "MinyBuilder(input={:?}, output={:?})",
            self.input, self.output
        )
    }
}

/// Builder for AYT (AY to Text) commands
///
/// Converts AY music files to text format.
///
/// Example:
/// ```python
/// builder = AytBuilder()
/// builder.input("song.ym")
/// builder.output("song.txt")
/// task = builder.build()
/// ```
#[pyclass(name = "AytBuilder")]
#[derive(Default, Clone)]
pub struct PyAytBuilder {
    input: Option<String>,
    output: Option<String>
}

#[pymethods]
impl PyAytBuilder {
    #[new]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the input AY/YM file (fluent API)
    pub fn input(&mut self, path: &str) -> Self {
        self.input = Some(path.to_string());
        self.clone()
    }

    /// Set the output text file (fluent API)
    pub fn output(&mut self, path: &str) -> Self {
        self.output = Some(path.to_string());
        self.clone()
    }

    /// Build the task using proper YmCruncher struct
    pub fn build(&self, py: Python) -> PyResult<Py<PyBndTask>> {
        let mut args = Vec::new();

        if let Some(ref input) = self.input {
            args.push(shell_escape(input));
        }

        if let Some(ref output) = self.output {
            args.push(shell_escape(output));
        }

        let args_string = args.join(" ");
        let std_args = StandardTaskArguments::new(args_string);
        let inner = InnerTask::with_ym_cruncher(YmCruncher::Ayt, std_args);
        let task = Task::from(inner);

        PyBndTask::new_from_task(py, task)
    }

    fn __repr__(&self) -> String {
        format!(
            "AytBuilder(input={:?}, output={:?})",
            self.input, self.output
        )
    }
}

/// Builder for song converter commands
///
/// Converts ArkosTracker songs to various formats (AKM, AKY, AKG, YM, WAV, VGM, etc.)
///
/// Example:
/// ```python
/// # Convert to YM format
/// builder = SongConverterBuilder("ym")
/// builder.input("song.aks")
/// builder.output("song.ym")
/// task = builder.build()
///
/// # Convert to WAV
/// builder = SongConverterBuilder("wav")
/// builder.input("song.aks")
/// builder.output("song.wav")
/// task = builder.build()
/// ```
#[pyclass(name = "SongConverterBuilder")]
#[derive(Clone)]
pub struct PySongConverterBuilder {
    converter_type: String,
    input: Option<String>,
    output: Option<String>,
    extra_args: Vec<String>
}

#[pymethods]
impl PySongConverterBuilder {
    #[new]
    pub fn new(converter_type: &str) -> Self {
        Self {
            converter_type: converter_type.to_lowercase(),
            input: None,
            output: None,
            extra_args: Vec::new()
        }
    }

    /// Set the input song file (fluent API)
    pub fn input(&mut self, path: &str) -> Self {
        self.input = Some(path.to_string());
        self.clone()
    }

    /// Set the output file (fluent API)
    pub fn output(&mut self, path: &str) -> Self {
        self.output = Some(path.to_string());
        self.clone()
    }

    /// Add a raw argument (fluent API)
    pub fn arg(&mut self, arg: &str) -> Self {
        self.extra_args.push(arg.to_string());
        self.clone()
    }

    /// Build the task using proper SongConverter struct
    pub fn build(&self, py: Python) -> PyResult<Py<PyBndTask>> {
        let mut args = Vec::new();

        if let Some(ref input) = self.input {
            args.push(shell_escape(input));
        }

        if let Some(ref output) = self.output {
            args.push(shell_escape(output));
        }

        for arg in &self.extra_args {
            args.push(shell_escape(arg));
        }

        let args_string = args.join(" ");
        let std_args = StandardTaskArguments::new(args_string);

        // Map converter type to the correct SongConverter variant
        let converter = match self.converter_type.as_str() {
            "akg" => SongConverter::new_song_to_akg_default(),
            "akm" => SongConverter::new_song_to_akm_default(),
            "aky" => SongConverter::new_song_to_aky_default(),
            "events" => SongConverter::new_song_to_events_default(),
            "raw" => SongConverter::new_song_to_raw_default(),
            "soundeffects" | "effects" => SongConverter::new_song_to_sound_effects_default(),
            "vgm" => SongConverter::new_song_to_vgm_default(),
            "wav" => SongConverter::new_song_to_wav_default(),
            "ym" => SongConverter::new_song_to_ym_default(),
            "z80profiler" | "profiler" => SongConverter::new_z80profiler_default(),
            _ => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Unknown converter type: {}. Valid types: akg, akm, aky, events, raw, soundeffects, vgm, wav, ym, z80profiler",
                    self.converter_type
                )));
            },
        };

        let inner = InnerTask::with_songconverter(converter, std_args);
        let task = Task::from(inner);

        PyBndTask::new_from_task(py, task)
    }

    fn __repr__(&self) -> String {
        format!(
            "SongConverterBuilder(type={:?}, input={:?}, output={:?})",
            self.converter_type, self.input, self.output
        )
    }
}

/// Simple shell escaping for arguments with spaces
fn shell_escape(s: &str) -> String {
    if s.contains(' ') || s.contains('"') {
        format!("\"{}\"", s.replace('"', "\\\""))
    }
    else {
        s.to_string()
    }
}

/// Helper that builds a `SnapshotCli` instance from the `PySnapshotBuilder`.
///
/// This enforces using the real CLI data structure before converting to
/// the command-args string, preserving compile-time checks.
struct PySnapshot;

impl PySnapshot {
    pub fn build_parser(b: &PySnapshotBuilder) -> SnapshotCli {
        // flatten load files and put_data into the vec-of-strings format
        let mut load_vec: Vec<String> = Vec::new();
        for (f, a) in &b.load_files {
            load_vec.push(f.clone());
            load_vec.push(a.clone());
        }

        let mut set_vec: Vec<String> = Vec::new();
        for (t, v) in &b.set_tokens {
            set_vec.push(t.clone());
            set_vec.push(v.clone());
        }

        let mut put_vec: Vec<String> = Vec::new();
        for (addr, byte) in &b.put_data {
            put_vec.push(format!("0x{:X}", addr));
            put_vec.push(format!("0x{:X}", byte));
        }

        SnapshotCli {
            info: false,
            debug: false,
            output: b.output.clone(),
            in_snapshot: b.in_snapshot.clone(),
            load: load_vec,
            get_token: Vec::new(),
            set_token: set_vec,
            put_data: put_vec,
            sna_version: b.sna_version.clone(),
            flags: false,
            cli: false
        }
    }

    // argv construction moved to `SnapshotCli::build_argv` in cpclib-sna
}
