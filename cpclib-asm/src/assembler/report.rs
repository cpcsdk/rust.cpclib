use std::time::{Instant};
use std::fmt::Display;
use super::Env;

pub struct Report {
	nb_passes: usize,
	duration: std::time::Duration
}


impl From<(&Env, &Instant)> for Report {
	fn from((env, start): (&Env, &Instant)) -> Self {
		Report {
			nb_passes: env.real_nb_passes,
			duration: Instant::now().duration_since(*start)
		}
	}
}


impl Display for Report {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let duration = self.duration.as_secs_f64();
		write!(
			f,
			"Assembled in {} passes and {}.",
			self.nb_passes,
			if duration >= 60. {
				format!("{:.2}min", duration/60.)
			} else {
				format!("{:.2}s", duration)
			}
		)

	}
}
