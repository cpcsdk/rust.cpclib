use std::collections::HashSet;
use std::env::current_dir;
use std::io::{BufReader, Read};
use std::path::{self, Path};

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

pub fn init_project(path: Option<&Path>) -> Result<(), BndBuilderError> {
    let path = path
        .map(|p| p.to_owned())
        .unwrap_or_else(|| current_dir().unwrap());

    if !path.is_dir() {
        return Err(BndBuilderError::AnyError(format!(
            "{} is not a valid directory",
            path.display()
        )));
    }

    let bndbuild_yml = path.join("bndbuild.yml");
    if bndbuild_yml.exists() {
        return Err(BndBuilderError::AnyError(format!(
            "{} already exists",
            bndbuild_yml.display()
        )));
    }

    let main_asm = path.join("main.asm");
    if main_asm.exists() {
        return Err(BndBuilderError::AnyError(format!(
            "{} already exists",
            main_asm.display()
        )));
    }

    let data_asm = path.join("data.asm");
    if main_asm.exists() {
        return Err(BndBuilderError::AnyError(format!(
            "{} already exists",
            data_asm.display()
        )));
    }

    std::fs::write(&bndbuild_yml, include_bytes!("default_bndbuild.yml"))
        .map_err(|e| BndBuilderError::AnyError(e.to_string()))?;

    std::fs::write(&main_asm, include_bytes!("default_main.asm"))
        .map_err(|e| BndBuilderError::AnyError(e.to_string()))?;

    std::fs::write(&data_asm, include_bytes!("default_data.asm"))
        .map_err(|e| BndBuilderError::AnyError(e.to_string()))?;

    Ok(())
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
                                },
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
    UnknownTarget(String),
    #[error("{0}")]
    AnyError(String)
}
