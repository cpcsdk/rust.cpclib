use std::fmt::Display;
use std::path::PathBuf;
use std::time::Instant;

use cpclib_common::itertools::Itertools;

use super::Env;

pub struct Report<'env> {
    nb_passes: usize,
    duration: std::time::Duration,
    saved_files: Vec<&'env SavedFile>
}

#[derive(Clone, Debug)]
pub struct SavedFile {
    pub(crate) name: PathBuf,
    pub(crate) size: usize
}

impl<'env> From<(&'env Env, &Instant)> for Report<'env> {
    fn from((env, start): (&'env Env, &Instant)) -> Self {
        Report {
            nb_passes: env.real_nb_passes,
            duration: Instant::now().duration_since(*start),
            saved_files: env
                .saved_files
                .as_ref()
                .map(|v| v.iter().collect_vec())
                .unwrap_or(Vec::new())
        }
    }
}

impl<'env> Display for Report<'env> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let duration = self.duration.as_secs_f64();

        if false {
            let saved = self
                .saved_files
                .iter()
                .map(|s| format!("Saved \"{}\" for {} bytes.\n", s.name.display(), s.size))
                .join("");
            write!(f, "{}", saved)?;
        }
        write!(
            f,
            "Assembled in {} pass{} and {}.",
            self.nb_passes,
            if self.nb_passes > 1 { "es" } else { "" },
            if duration >= 60. {
                format!("{:.2}min", duration / 60.)
            }
            else {
                format!("{:.2}s", duration)
            }
        )
    }
}
