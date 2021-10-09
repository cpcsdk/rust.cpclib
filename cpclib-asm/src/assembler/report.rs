use super::Env;
use cpclib_common::itertools::Itertools;
use std::fmt::Display;
use std::time::Instant;

pub struct Report<'env> {
    nb_passes: usize,
    duration: std::time::Duration,
    saved_files: Vec<&'env SavedFile>,
}

#[derive(Clone, Debug)]
pub struct SavedFile {
    pub(crate) name: String,
    pub(crate) size: usize,
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
                .unwrap_or(Vec::new()),
        }
    }
}

impl<'env> Display for Report<'env> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let duration = self.duration.as_secs_f64();
        self.saved_files
            .iter()
            .map(|s| write!(f, "Saved \"{}\" for {} bytes.\n", s.name, s.size))
            .collect::<std::fmt::Result>()?;
        write!(
            f,
            "Assembled in {} passes and {}.",
            self.nb_passes,
            if duration >= 60. {
                format!("{:.2}min", duration / 60.)
            } else {
                format!("{:.2}s", duration)
            }
        )
    }
}
