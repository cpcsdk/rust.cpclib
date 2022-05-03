use indicatif::{ProgressBar, MultiProgress, ProgressStyle};
use core::time::Duration;

lazy_static::lazy_static! {
	static ref PROGRESS: Progress = Progress::new();
}

pub struct Progress {
	bar: MultiProgress
}

impl Progress {
	pub fn progress() -> &'static Self {
		& PROGRESS
	}

	pub fn new() -> Self {
		Progress {
			bar: MultiProgress::new(),
		}
	}

	/// Add the progress bar for a file to read
	pub fn add_bar(&self, msg: &str) -> ProgressBar {
		let bar = ProgressBar::new_spinner();
		let bar = self.bar.add(bar);

		bar.set_style(ProgressStyle::with_template("{spinner:.blue} {msg}")
			// For more spinners check out the cli-spinners project:
			// https://github.com/sindresorhus/cli-spinners/blob/master/spinners.json
			.unwrap()
			.tick_strings(&[
				"▹▹▹▹▹",
				"▸▹▹▹▹",
				"▹▸▹▹▹",
				"▹▹▸▹▹",
				"▹▹▹▸▹",
				"▹▹▹▹▸",
				"▪▪▪▪▪",
			])
		);
		bar.enable_steady_tick(Duration::from_millis(250));


		bar.set_message(msg.to_owned());
		bar

	}

	/// Remove the progress bar of the current file
	pub fn remove_bar_ok(&self, bar: &ProgressBar) {
		bar.disable_steady_tick();
        bar.finish_and_clear();
		bar.tick();
		self.bar.remove(bar);
	}

	pub fn remove_bar_err(&self, bar: &ProgressBar, msg: &str) {
		bar.disable_steady_tick();
		bar.abandon_with_message(msg.to_owned());
		bar.tick();
		self.bar.remove(bar);
	}
}