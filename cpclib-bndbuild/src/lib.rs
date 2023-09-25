use std::collections::HashSet;
use std::io::{BufReader, Read};
use std::path::Path;

use cpclib_common::itertools::Itertools;
use lazy_regex::regex_captures;
use rules::{Graph, Rule};
use thiserror::Error;

pub mod builder;
pub mod constraints;
pub mod executor;
pub mod rules;
pub mod runners;
pub mod task;

pub use builder::*;

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

/// Expand glob patterns
/// {a,b} expension is always done even if file does not exists
/// *.a is done only when file exists
fn expand_glob(p: &str) -> Vec<String> {
    let expended = if let Some((_, start, middle, end)) = regex_captures!(r"^(.*)\{(.*)\}(.*)$", p)
    {
        middle
            .split(",")
            .map(|component| format!("{start}{component}{end}"))
            .collect_vec()
    }
    else {
        vec![p.to_owned()]
    };

    expended
        .into_iter()
        .map(|p| {
            globmatch::Builder::new(p.as_str())
                .build("." /* std::env::current_dir().unwrap() */)
                .map(|builder| {
                    builder
                        .into_iter()
                        .map(|p2| {
                            match p2 {
                                Ok(p) => {
                                    let p = p.display().to_string();
                                    if p.starts_with(".\\") {
                                        p[2..].to_owned()
                                    }
                                    else {
                                        p
                                    }
                                }
                                Err(_e) => p.clone()
                            }
                        })
                        .collect_vec()
                })
                .map(|v| {
                    if v.is_empty() {
                        vec![p.clone()]
                    }
                    else {
                        v
                    }
                })
                .unwrap_or(vec![p])
        })
        .flatten()
        .collect_vec()
}

#[derive(Error, Debug)]
pub enum BndBuilderError {
    #[error("Unable to access file {fname}: {error}.")]
    InputFileError {
        fname: String,
        error: std::io::Error
    },
    #[error("Unable to setup working directory {fname}: {error}.")]
    WorkingDirectoryError {
        fname: String,
        error: std::io::Error
    },
    #[error("Unable to deserialize rules {0}.")]
    ParseError(serde_yaml::Error),
    #[error("Unable to build the dependency graph {0}.")]
    DependencyError(String),
    #[error("Unable to build {fname}: {msg}.")]
    ExecuteError { fname: String, msg: String },
    #[error("Unable to build default target.\n{source}")]
    DefaultTargetError { source: Box<BndBuilderError> },
    #[error("The file does not contain a target.")]
    NoTargets,
    #[error("The target {0} is disabled.")]
    DisabledTarget(String),
    #[error("Target {0} is not buildable.")]
    UnknownTarget(String)
}
