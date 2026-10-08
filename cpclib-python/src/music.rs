//! Python access to bndbuild's music pipeline: turning a song (Arkos Tracker,
//! CHIPNSFX or YM) into a standalone CPC player program with any of the
//! players bndbuild knows - and measuring what each one weighs.
//!
//! ```python
//! from cpclib_python import music
//!
//! music.compatible_players("tune.aks")        # ['akg', 'akm', ..., 'miny']
//! build = music.build("tune.aks", "fap", snapshot="tune.sna")
//! build.song_bytes, build.player_bytes, build.buffer_bytes, build.play_nops
//! print(music.compare_table("tune.aks"))      # every compatible player
//! ```

#![allow(unsafe_op_in_unsafe_fn)]

use std::sync::Arc;

use cpclib_bndbuild::pipeline::music_run::{self, MusicOptions, MusicPlayer};
use cpclib_bndbuild::pipeline::{
    song_is_chp, song_is_ym, song_metadata, song_tracker, song_uses_sid
};
use cpclib_common::camino::{Utf8Path, Utf8PathBuf};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};

use crate::observer::PyObserver;

fn parse_player(player: Option<&str>) -> PyResult<MusicPlayer> {
    player
        .map_or(Ok(MusicPlayer::Auto), str::parse)
        .map_err(PyValueError::new_err)
}

fn options(player: Option<&str>, sid_wait_line_count: u16) -> PyResult<MusicOptions> {
    Ok(MusicOptions {
        sid_wait_line_count,
        player: parse_player(player)?
    })
}

/// The name the AMSDOS binary is given: `name`, or the song's file stem.
fn name_of(song: &Utf8Path, name: Option<&str>) -> String {
    name.map(str::to_string)
        .or_else(|| song.file_stem().map(str::to_string))
        .unwrap_or_else(|| "SONG".to_string())
}

/// What `build` made of a song.
#[pyclass(name = "MusicBuild", frozen)]
pub struct PyMusicBuild {
    /// The player the song ended up with: a name of `players()`, or `sid`
    /// for Arkos Tracker's SID player (what a SID song needs, not a choice).
    #[pyo3(get)]
    player: String,
    program: Vec<u8>,
    /// Where `program` loads, and is entered.
    #[pyo3(get)]
    load_address: u16,
    /// The song as the player reads it (None for CHIPNSFX, whose song is Z80
    /// source, and for the SID player).
    #[pyo3(get)]
    song_bytes: Option<u64>,
    /// The player's code alone (for AYT, its builder: the player it writes at
    /// run time, 250 to 340 bytes, comes on top). None for the SID player.
    #[pyo3(get)]
    player_bytes: Option<u64>,
    /// RAM the player needs beyond the program: FAP's decrunch buffer,
    /// MinYMiser's cache.
    #[pyo3(get)]
    buffer_bytes: Option<u64>,
    /// What FAP's packer says one frame costs, in NOPs.
    #[pyo3(get)]
    play_nops: Option<u64>,
    /// The snapshot written, if one was asked for.
    #[pyo3(get)]
    snapshot: Option<String>
}

#[pymethods]
impl PyMusicBuild {
    /// The player program, headerless: what is saved from `load_address`
    /// (song, player, and the song-info screen).
    #[getter]
    fn program<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.program)
    }

    /// `len(program)`.
    #[getter]
    fn program_bytes(&self) -> usize {
        self.program.len()
    }

    fn __repr__(&self) -> String {
        format!(
            "MusicBuild(player={:?}, program_bytes={}, song_bytes={:?}, player_bytes={:?}, \
             buffer_bytes={:?}, play_nops={:?})",
            self.player,
            self.program.len(),
            self.song_bytes,
            self.player_bytes,
            self.buffer_bytes,
            self.play_nops
        )
    }
}

impl PyMusicBuild {
    fn new(build: music_run::MusicBuild, snapshot: Option<&Utf8Path>) -> Self {
        Self {
            player: build.player.to_string(),
            program: build.program,
            load_address: build.load_address,
            song_bytes: build.song_bytes,
            player_bytes: build.player_bytes,
            buffer_bytes: build.buffer_bytes,
            play_nops: build.play_nops,
            snapshot: snapshot.map(|s| s.to_string())
        }
    }
}

/// What one player made of a song, in `compare`.
#[pyclass(name = "PlayerComparison", frozen)]
pub struct PyPlayerComparison {
    #[pyo3(get)]
    player: String,
    /// Why the player failed on this song, if it did.
    #[pyo3(get)]
    error: Option<String>,
    /// The result, unless it failed.
    #[pyo3(get)]
    build: Option<Py<PyMusicBuild>>
}

#[pymethods]
impl PyPlayerComparison {
    fn __repr__(&self, py: Python) -> String {
        match (&self.build, &self.error) {
            (Some(build), _) => format!("PlayerComparison({})", build.borrow(py).__repr__()),
            (None, Some(e)) => format!("PlayerComparison(player={:?}, error={:?})", self.player, e),
            _ => format!("PlayerComparison(player={:?})", self.player)
        }
    }
}

/// Every player name `build` accepts, `auto` (the one the song needs) first.
#[pyfunction]
fn players() -> Vec<&'static str> {
    MusicPlayer::ALL.map(MusicPlayer::name).to_vec()
}

/// The players that can play `song` (never `auto`): the Arkos Tracker ones for
/// an Arkos Tracker song, `chipnsfx` for a `.chp`, the YM-based ones (`fap`,
/// `ayt`, `miny`) for any song.
#[pyfunction]
fn compatible_players(song: &str) -> Vec<&'static str> {
    music_run::compatible_players(Utf8Path::new(song))
        .into_iter()
        .map(MusicPlayer::name)
        .collect()
}

/// What can be told about `song` without building it: `kind` (`arkos`,
/// `chipnsfx` or `ym`), `tracker` (when its format tells), `title`, `author`,
/// `composer`, `comment` (empty when unknown), and `uses_sid` (an Arkos Tracker
/// song using its SID feature, which needs a player of its own).
#[pyfunction]
fn song_info<'py>(py: Python<'py>, song: &str) -> PyResult<Bound<'py, PyDict>> {
    let path = Utf8Path::new(song);
    let meta = song_metadata(path);
    let kind = if song_is_chp(path) {
        "chipnsfx"
    }
    else if song_is_ym(path) {
        "ym"
    }
    else {
        "arkos"
    };

    let info = PyDict::new(py);
    info.set_item("kind", kind)?;
    info.set_item("tracker", song_tracker(path))?;
    info.set_item("title", meta.title)?;
    info.set_item("author", meta.author)?;
    info.set_item("composer", meta.composer)?;
    info.set_item("comment", meta.comment)?;
    info.set_item(
        "uses_sid",
        song_uses_sid(path).map_err(PyRuntimeError::new_err)?
    )?;
    Ok(info)
}

/// Converts `song` and assembles it into a standalone player program with
/// `player` (a name of `players()`; default `auto`, what the song needs). With
/// `snapshot`, a snapshot of the running player is written there too. `name`
/// defaults to the song's file name.
///
/// The tools' output goes to the console, or to `on_output(kind, text)` when
/// given (see `Task.execute`).
///
/// Raises `RuntimeError` when a conversion or the assembly fails, `ValueError`
/// for an unknown player.
#[pyfunction]
#[pyo3(signature = (song, player=None, snapshot=None, name=None, sid_wait_line_count=72, on_output=None))]
fn build(
    py: Python,
    song: &str,
    player: Option<&str>,
    snapshot: Option<&str>,
    name: Option<&str>,
    sid_wait_line_count: u16,
    on_output: Option<Py<PyAny>>
) -> PyResult<PyMusicBuild> {
    let options = options(player, sid_wait_line_count)?;
    let song = Utf8PathBuf::from(song);
    let snapshot = snapshot.map(Utf8PathBuf::from);
    let name = name_of(&song, name);

    let build = py
        .detach(|| {
            music_run::build_music(
                &song,
                &name,
                &options,
                snapshot.as_deref(),
                &Arc::new(PyObserver::new(on_output))
            )
        })
        .map_err(PyRuntimeError::new_err)?;
    Ok(PyMusicBuild::new(build, snapshot.as_deref()))
}

/// Same as `build`, but wraps the program as an AMSDOS binary on a fresh DSK,
/// saved at `dsk`. Returns `dsk`.
#[pyfunction]
#[pyo3(signature = (song, dsk, player=None, name=None, sid_wait_line_count=72, on_output=None))]
fn build_dsk(
    py: Python,
    song: &str,
    dsk: &str,
    player: Option<&str>,
    name: Option<&str>,
    sid_wait_line_count: u16,
    on_output: Option<Py<PyAny>>
) -> PyResult<String> {
    let options = options(player, sid_wait_line_count)?;
    let song = Utf8PathBuf::from(song);
    let name = name_of(&song, name);

    let built = py
        .detach(|| {
            music_run::build_music_dsk(
                &song,
                &name,
                &options,
                &Arc::new(PyObserver::new(on_output))
            )
        })
        .map_err(PyRuntimeError::new_err)?;
    std::fs::copy(&built, dsk)
        .map_err(|e| PyRuntimeError::new_err(format!("Could not write {dsk}: {e}")))?;
    Ok(dsk.to_string())
}

/// Builds the player and launches `emulator` on it (any name bndbuild's `emu`
/// accepts, `ace` by default). Returns what happened, as a message.
#[pyfunction]
#[pyo3(signature = (song, emulator="ace", player=None, name=None, sid_wait_line_count=72, on_output=None))]
fn play(
    py: Python,
    song: &str,
    emulator: &str,
    player: Option<&str>,
    name: Option<&str>,
    sid_wait_line_count: u16,
    on_output: Option<Py<PyAny>>
) -> PyResult<String> {
    let options = options(player, sid_wait_line_count)?;
    let song = Utf8PathBuf::from(song);
    let name = name_of(&song, name);

    let outcome = py.detach(|| {
        music_run::run_music_in_emulator(
            &song,
            &name,
            emulator,
            &options,
            &Arc::new(PyObserver::new(on_output))
        )
    });
    if outcome.success {
        Ok(outcome.message)
    }
    else {
        Err(PyRuntimeError::new_err(outcome.message))
    }
}

/// Builds `song` with every player that can play it, to compare them: one
/// `PlayerComparison` each, with its `build` or the `error` that stopped it.
#[pyfunction]
#[pyo3(signature = (song, name=None, on_output=None))]
fn compare(
    py: Python,
    song: &str,
    name: Option<&str>,
    on_output: Option<Py<PyAny>>
) -> PyResult<Vec<PyPlayerComparison>> {
    let song = Utf8PathBuf::from(song);
    let name = name_of(&song, name);

    let comparisons = py
        .detach(|| {
            music_run::compare_music_players(&song, &name, &Arc::new(PyObserver::new(on_output)))
        })
        .map_err(PyRuntimeError::new_err)?;

    comparisons
        .into_iter()
        .map(|c| {
            let player = c.player.name().to_string();
            Ok(match c.outcome {
                Ok(build) => {
                    PyPlayerComparison {
                        player,
                        error: None,
                        build: Some(Py::new(py, PyMusicBuild::new(build, None))?)
                    }
                },
                Err(error) => {
                    PyPlayerComparison {
                        player,
                        error: Some(error),
                        build: None
                    }
                },
            })
        })
        .collect()
}

/// `compare`, as the plain-text table bndbuild's VS Code extension shows.
#[pyfunction]
#[pyo3(signature = (song, name=None, on_output=None))]
fn compare_table(
    py: Python,
    song: &str,
    name: Option<&str>,
    on_output: Option<Py<PyAny>>
) -> PyResult<String> {
    let song = Utf8PathBuf::from(song);
    let name = name_of(&song, name);

    py.detach(|| {
        music_run::compare_music_players(&song, &name, &Arc::new(PyObserver::new(on_output)))
            .map(|c| music_run::format_player_comparison(&c))
    })
    .map_err(PyRuntimeError::new_err)
}

pub fn music(py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    let _ = py;
    m.add_class::<PyMusicBuild>()?;
    m.add_class::<PyPlayerComparison>()?;
    m.add_function(wrap_pyfunction!(players, m)?)?;
    m.add_function(wrap_pyfunction!(compatible_players, m)?)?;
    m.add_function(wrap_pyfunction!(song_info, m)?)?;
    m.add_function(wrap_pyfunction!(build, m)?)?;
    m.add_function(wrap_pyfunction!(build_dsk, m)?)?;
    m.add_function(wrap_pyfunction!(play, m)?)?;
    m.add_function(wrap_pyfunction!(compare, m)?)?;
    m.add_function(wrap_pyfunction!(compare_table, m)?)?;
    Ok(())
}
