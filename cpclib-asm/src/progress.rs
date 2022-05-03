use cpclib_common::itertools::Itertools;
use indicatif::{ProgressBar, MultiProgress, ProgressStyle};
use core::time::Duration;
use std::{sync::{Mutex, Arc}};

lazy_static::lazy_static! {
	static ref PROGRESS: Progress = Progress::new();
}

const REFRESH_RATE: Duration = Duration::from_millis(250);

pub struct Progress {
	multi: MultiProgress,
	parse: Arc<Mutex<CountedProgress>>,
	load: Arc<Mutex<CountedProgress>>,
}

struct CountedProgress {
	bar: ProgressBar,
	current_items: hashbag::HashBag<String>,
	visible: bool
}

impl CountedProgress {
	pub fn new(kind: &'static str) -> Self {

		let bar = ProgressBar::new(0);
		bar.set_style(
			ProgressStyle::with_template("{prefix:.bold<6}  [{bar}] {pos:>3}/{len:3} {wide_msg}")
				.unwrap()
				.progress_chars("=> ")
		);

		let cp = CountedProgress { 
			bar, 
			current_items: hashbag::HashBag::new(),
			visible: false
		};
		cp.bar.set_prefix(kind);
		cp
	}

	fn add_item(&mut self, item: &str, multi: &MultiProgress) {
		self.bar.inc_length(1);
		self.current_items.insert(item.into());
		self.update_visual(multi);
	}

	fn remove_item(&mut self, item: &str, multi: &MultiProgress) {
		self.bar.inc(1);
		self.current_items.remove(item);
		self.update_visual(multi);
	}

	fn update_visual(&mut self, multi: &MultiProgress) {
		if self.bar.length() == Some(self.bar.position()) {
			if self.visible {
				self.bar.finish_and_clear();
				self.bar.tick();

				multi.remove(&self.bar);
				self.visible = false;
				panic!();
			}
		} else {
			let content = self.current_items.iter().join(", ");
			self.bar.set_message(content);

			if !self.visible{
				let other_bar = self.bar.clone();
				self.bar = multi.add(other_bar);
			}
			self.bar.tick();

		}

	}
}


fn new_spinner() -> ProgressBar {
	let bar = ProgressBar::new_spinner();

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
	bar.enable_steady_tick(REFRESH_RATE);
	bar
}

impl Progress {
	pub fn progress() -> &'static Self {
		& PROGRESS
	}



	pub fn new() -> Self {
		let multi = MultiProgress::new();
		let load = ProgressBar::new_spinner();
		let parse = ProgressBar::new_spinner();

		multi.add(load);
		multi.add(parse);

		Progress {
			multi,
			parse: Arc::new(Mutex::new(CountedProgress::new("Parse"))),
			load: Arc::new(Mutex::new(CountedProgress::new("Load")))
		}
	}


	pub fn add_parse(&self, ident: &str) {
		self.parse
			.lock()
			.unwrap()
			.add_item(ident, &self.multi)
	}

	pub fn remove_parse(&self, ident: &str) {
		self.parse
			.lock()
			.unwrap()
			.remove_item(ident, &self.multi)
	}


	pub fn add_load(&self, ident: &str) {
		self.load
			.lock()
			.unwrap()
			.add_item(ident, &self.multi)
	}

	pub fn remove_load(&self, ident: &str) {
		self.load
			.lock()
			.unwrap()
			.remove_item(ident, &self.multi)
	}


	/// Add the progress bar for a file to read
	pub fn add_bar(&self, msg: &str) -> ProgressBar {
		let bar = new_spinner();
		let bar = self.multi.add(bar);
		bar.set_message(msg.to_owned());
		bar

	}

	/// Remove the progress bar of the current file
	pub fn remove_bar_ok(&self, bar: &ProgressBar) {
		bar.disable_steady_tick();
        bar.finish_and_clear();
		bar.tick();
		self.multi.remove(bar);
	}

	pub fn remove_bar_err(&self, bar: &ProgressBar, msg: &str) {
		bar.disable_steady_tick();
		bar.abandon_with_message(msg.to_owned());
		bar.tick();
		self.multi.remove(bar);
	}
}