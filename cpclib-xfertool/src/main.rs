#![deny(
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_qualifications,
    nonstandard_style,
    rust_2018_idioms,
    unused,
    warnings
)]
#![deny(clippy::pedantic)]
#![allow(unused)]

use std::env;
use std::path::Path;
use std::time::Duration;

use cpclib_common::clap::{self, ArgAction, Command};
use cpclib_xfertool::{build_args_parser, built_info, process};
#[cfg(feature = "watch")]
use crossbeam_channel::unbounded;
#[cfg(feature = "watch")]
use hotwatch::{Event, Hotwatch};
#[cfg(feature = "watch")]
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use {anyhow, cpclib_disc as disc, cpclib_sna as sna, cpclib_xfer as xfer};

fn main() -> anyhow::Result<()> {
    let desc_before = format!(
        "Profile {} compiled: {}",
        built_info::PROFILE,
        built_info::BUILT_TIME_UTC
    );

    let matches = build_args_parser().before_help(desc_before).get_matches();

    process(&matches)
}
