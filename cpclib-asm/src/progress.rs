use cpclib_common::itertools::Itertools;
use indicatif::{ProgressBar, MultiProgress, ProgressStyle};
use core::time::Duration;
use std::{sync::{Mutex, Arc, MutexGuard}, path::PathBuf};

lazy_static::lazy_static! {
	static ref PROGRESS: Arc<Mutex<Progress>> = Arc::new(Mutex::new(Progress::new()));
}

const REFRESH_RATE: Duration = Duration::from_millis(250);
const PROGRESS_STYLE: &'static str = "{prefix:.bold.dim>8}  [{bar}] {pos:>3}/{len:3} {wide_msg}";
const PASS_STYLE: &'static str = "{prefix:.bold.dim>8}  [{bar}] ";

pub struct Progress {
	multi: MultiProgress,
	parse: CountedProgress,
	load: CountedProgress,
	save: Option<CountedProgress>,
	pass: Option<(usize, ProgressBar)>
}

struct CountedProgress {
	bar: Option<ProgressBar>,
	current_items: hashbag::HashBag<String>,
	nb_expected: u64,
	nb_done: u64,
	prefix: &'static str,
	index: usize,
	freeze_amount: bool
}


pub fn normalize(path: &PathBuf) -> &str {
	path.file_name().unwrap().to_str().unwrap()
}

impl CountedProgress {
	pub fn new(kind: &'static str, index: usize, freeze_amount: bool) -> Self {


		let cp = CountedProgress { 
			bar: None, 
			current_items: hashbag::HashBag::new(),
			nb_done: 0,
			nb_expected: 0,
			prefix: kind, 
			index,
			freeze_amount
		};
		cp
	}

	fn add_item(&mut self, item: &str, multi: &MultiProgress) {
		if ! self.freeze_amount {
			self.nb_expected += 1;
		}
		self.current_items.insert(item.into());
		self.update_visual(multi);
	}

	fn add_items<'a>(&mut self, items: impl Iterator<Item=&'a str>, multi: &MultiProgress) {
		let mut count = 0;
		for item in items {
			self.current_items.insert(String::from(item));
			count += 1;
		}

		if ! self.freeze_amount {
			self.nb_expected += count;
		}
		self.update_visual(multi);
	}

	fn remove_item(&mut self, item: &str, multi: &MultiProgress) {
		self.nb_done += 1;
		self.current_items.remove(item);
		self.update_visual(multi);
	}

	fn finished(&mut self) {
		self.bar.as_mut()
			.map(|bar| {
				bar.finish()
			});
	}

	fn update_visual(&mut self, multi: &MultiProgress) {
		let visible = self.bar.is_some();

		if self.nb_done == self.nb_expected {
			if visible {
				self.bar.as_ref().map(|bar| {
					bar.set_message("");
					bar.set_position(self.nb_done);
					bar.set_length(self.nb_expected);

					bar.tick();

					//multi.remove(bar);
				});
				//self.bar = None;
			}
		} else {
			let content = self.current_items.iter().join(", ");

			if !visible{
				self.bar = Some(multi.add(ProgressBar::new(self.nb_expected)));
				self.bar.as_ref().map(|bar|{
					bar.set_style(
						ProgressStyle::with_template(PROGRESS_STYLE)
							.unwrap()
							.progress_chars("=> ")
					);
					bar.set_prefix(self.prefix);
				});
		
			} 
				
			self.bar.as_ref().map(|bar| {
				bar.set_message(content);
				bar.set_position(self.nb_done);
				bar.set_length(self.nb_expected);
				bar.tick();
			});

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
	pub fn progress() -> MutexGuard<'static, Progress> {
		PROGRESS.lock().unwrap()
	}



	pub fn new() -> Self {
		let multi = MultiProgress::new();
		multi.set_move_cursor(true);


		Progress {
			multi,
			load: CountedProgress::new("  Load", 0, false),
			parse: CountedProgress::new(" Parse", 1, false),
			save: None,
			pass: None
		}
	}


	pub fn add_parse(&mut self, ident: &str) {
		self.parse
			.add_item(ident, &self.multi)
	}


	pub fn add_parses<'a>(&mut self, items: impl Iterator<Item=&'a str>) {
		self.parse
			.add_items(items, &self.multi)
	}
	
	pub fn remove_parse(&mut self, ident: &str) {
		self.parse
			.remove_item(ident, &self.multi)
	}


	pub fn add_load(&mut self, ident: &str) {
		self.load
			.add_item(ident, &self.multi)
	}

	pub fn add_loads<'a>(&mut self, items: impl Iterator<Item=&'a str>) {
		self.load
			.add_items(items, &self.multi)
	}


	pub fn remove_load(&mut self, ident: &str) {
		self.load
			.remove_item(ident, &self.multi)
	}


	pub fn new_pass(&mut self) {
		if self.pass.is_none() {
			let mut bar = ProgressBar::new(0);
			bar.set_style(
				ProgressStyle::with_template(PASS_STYLE)
					.unwrap()
					.progress_chars("=> ")
			);
			self.pass = Some((0, self.multi.add(bar)));
		} else {
			//todo change pass numbering
		}

		self.pass.as_mut().map(|(pass,bar)| {
			*pass += 1;
			bar.set_prefix(format!("Pass {}", *pass));
			bar.set_position(0);
			bar.set_length(0);
		});
	}

	pub fn add_visited_to_pass(&mut self, amount: u64 ) {
		self.pass.as_mut()
			.unwrap()
			.1.inc(amount)
	}

	pub fn add_expected_to_pass(&mut self, amount: u64 ) {
		self.pass.as_mut()
			.unwrap()
			.1.inc_length(amount)
	}


	pub fn create_save_bar(&mut self, amount: u64) {
		let mut bar = CountedProgress::new("  Save", 2, true);
		bar.nb_expected = amount;
		self.save = Some(bar);
	}

	pub fn add_save(&mut self, ident: &str) {
		self.save
			.as_mut().unwrap()
			.add_item(ident, &self.multi)
	}
	pub fn remove_save(&mut self, ident: &str) {
		self.save
			.as_mut().unwrap()
			.remove_item(ident, &self.multi)
	}
	pub fn finish_save(&mut self) {
		self.save
			.as_mut().unwrap()
			.finished()
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