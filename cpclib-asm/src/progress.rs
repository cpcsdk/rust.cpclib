#[cfg(feature = "indicatif")]
use core::time::Duration;
use std::cell::RefCell;
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};

use cpclib_common::camino::Utf8Path;
#[cfg(feature = "indicatif")]
use cpclib_common::itertools::Itertools;
#[cfg(feature = "indicatif")]
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

static PROGRESS: LazyLock<Arc<Mutex<Progress>>> =
    LazyLock::new(|| Arc::new(Mutex::new(Progress::new())));

/// A structured assembly-progress event - the same phases `Progress` itself
/// tracks (parse/load/pass/save), but as data instead of a drawn terminal
/// bar. `done`/`total` describe that phase as a whole at the moment of the
/// event, not just the one item named (mirrors `CountedProgress`'s own
/// `nb_done`/`nb_expected`).
///
/// The number of *passes* basm will need is never known ahead of time (it
/// iterates until addresses stabilize), so `PassStarted`/`PassProgress`
/// carry a pass index but no "out of N passes" total - a consumer wanting a
/// single blended percentage across an unknown pass count has to make its
/// own judgment call (e.g. weighting each pass's own token-level progress,
/// and not trying to predict how many passes remain).
#[derive(Debug, Clone)]
pub enum AsmProgressEvent {
    Parse { item: String, done: u64, total: u64 },
    Load { item: String, done: u64, total: u64 },
    PassStarted { pass: usize },
    PassProgress { pass: usize, visited: u64, expected: u64 },
    Save { item: String, done: u64, total: u64 },
    SaveFinished
}

/// A destination for [`AsmProgressEvent`]s, installed per-thread (see
/// [`install_progress_sink`]) rather than globally, so multiple sinks can
/// coexist without cross-talk.
///
/// A plain thread-local is *not* enough on its own to see every event a
/// build produces: `build_processed_tokens_list` parses `INCLUDE`d files
/// via `tokens.par_iter()` (with the `rayon` feature), so parse events for
/// an included file can land on a rayon worker thread that never itself
/// called `install_progress_sink`. A caller that wants complete coverage
/// needs to install the sink on every thread that might run this work - see
/// `cpclib-lsp`'s `run_with_progress_sink` (in
/// `cpclib-lsp/src/bndbuild/command.rs`), which runs the whole build inside
/// a thread pool dedicated to that one call and broadcasts the install
/// across it, so no other concurrent rayon work (a second build, or
/// anything else in the process using the shared global pool) can ever
/// observe or contaminate this build's sink.
pub trait AsmProgressSink: Send + Sync {
    fn on_progress(&self, event: AsmProgressEvent);
}

thread_local! {
    static PROGRESS_SINK: RefCell<Option<Arc<dyn AsmProgressSink>>> = const { RefCell::new(None) };
}

/// Install (or, with `None`, clear) this thread's progress sink. Callers
/// must clear it (pass `None`) once the build they installed it for is
/// done - it is not scoped/dropped automatically, since a thread that runs
/// this code (a `spawn_blocking` thread, a rayon worker, ...) is typically
/// pooled and reused across unrelated work afterward. See [`AsmProgressSink`]'s
/// own doc for why installing this on a single thread isn't automatically
/// enough to see every event a build produces.
pub fn install_progress_sink(sink: Option<Arc<dyn AsmProgressSink>>) {
    PROGRESS_SINK.with(|s| *s.borrow_mut() = sink);
}

/// Whether this thread has a sink installed - an additional, independent
/// gate a caller can OR into its own `show_progress()` check so progress
/// events flow for a sink-consuming caller (the LSP) even when the
/// classic `--progress` CLI flag (which the sink has nothing to do with)
/// was never passed - see `AssemblingOptions`/`ParserOptions::show_progress`
/// call sites.
pub fn has_progress_sink() -> bool {
    PROGRESS_SINK.with(|s| s.borrow().is_some())
}

/// Suppresses this thread's progress sink for the guard's lifetime,
/// restoring whatever was installed before (which may itself be `None`)
/// when it drops - including on an early return/`?` from within the
/// suppressed scope, since restoration happens in `Drop`, not after a
/// fallible call.
///
/// For internal, throwaway sub-assemblies that are not a real build phase -
/// `Token::to_bytes`/`to_bytes_with_options` builds a disposable `Env` just
/// to measure one instruction's byte length (used for things like JR-range
/// checks), and does so per-instruction, potentially thousands of times
/// during a real build. `has_progress_sink()` is thread-local, with no way
/// to tell "the real build's `Env`" apart from one of these probes running
/// on the very same thread - without suppression, every such probe would
/// also report its own tiny `PassStarted`/`PassProgress` sequence into the
/// real build's progress stream, drowning genuine progress in noise that
/// has nothing to do with overall build progress (this was observed
/// directly: ~650 reported "passes" for a build that actually took 4).
pub fn suppress_progress_sink() -> SuppressedProgressSink {
    SuppressedProgressSink(PROGRESS_SINK.with(|s| s.borrow_mut().take()))
}

pub struct SuppressedProgressSink(Option<Arc<dyn AsmProgressSink>>);

impl Drop for SuppressedProgressSink {
    fn drop(&mut self) {
        PROGRESS_SINK.with(|s| *s.borrow_mut() = self.0.take());
    }
}

fn notify(event: AsmProgressEvent) {
    PROGRESS_SINK.with(|s| {
        if let Some(sink) = s.borrow().as_ref() {
            sink.on_progress(event);
        }
    });
}

#[cfg(feature = "indicatif")]
const REFRESH_RATE: Duration = Duration::from_millis(250);
#[cfg(feature = "indicatif")]
const PROGRESS_STYLE: &str = "{prefix:.bold.dim>8}  [{bar}] {pos:>3}/{len:3} {wide_msg}";
#[cfg(feature = "indicatif")]
const PASS_STYLE: &str = "{prefix:.bold.dim>8}  [{bar}] ";

#[cfg(feature = "indicatif")]
pub struct Progress {
    multi: MultiProgress,
    parse: CountedProgress,
    load: CountedProgress,
    save: Option<CountedProgress>,
    pass: Option<(usize, ProgressBar)>
}

#[cfg(not(feature = "indicatif"))]
pub struct Progress {
    parse: CountedProgress,
    load: CountedProgress,
    save: Option<CountedProgress>,
    pass: Option<(usize, usize, usize)> // pass, nb ivisited, nb to do
}

pub fn normalize(path: &Utf8Path) -> &str {
    path.file_name().unwrap()
}

#[cfg(feature = "indicatif")]
// TODO add the multiprogess bar as a field and never pass it as an argument
// it will allow to reduce duplicated code with indicatf/no indicatif versions
struct CountedProgress {
    bar: Option<ProgressBar>,
    current_items: hashbag::HashBag<String>,
    nb_expected: u64,
    nb_done: u64,
    prefix: &'static str,
    #[allow(unused)]
    index: usize,
    freeze_amount: bool
}

#[cfg(not(feature = "indicatif"))]
struct CountedProgress {
    current_items: hashbag::HashBag<String>,
    nb_expected: u64,
    nb_done: u64,
    #[allow(unused)]
    prefix: &'static str,
    #[allow(unused)]
    index: usize,
    freeze_amount: bool,
    last_tick: std::time::SystemTime
}

#[cfg(feature = "indicatif")]
impl CountedProgress {
    pub fn new(kind: &'static str, index: usize, freeze_amount: bool) -> Self {
        CountedProgress {
            bar: None,
            current_items: hashbag::HashBag::new(),
            nb_done: 0,
            nb_expected: 0,
            prefix: kind,
            index,
            freeze_amount
        }
    }

    fn add_item(&mut self, item: &str, multi: &MultiProgress) {
        if !self.freeze_amount {
            self.nb_expected += 1;
        }
        self.current_items.insert(item.into());
        self.update_visual(multi);
    }

    fn add_items<'a>(&mut self, items: impl Iterator<Item = &'a str>, multi: &MultiProgress) {
        let mut count = 0;
        for item in items {
            self.current_items.insert(String::from(item));
            count += 1;
        }

        if !self.freeze_amount {
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
        if let Some(bar) = self.bar.as_mut() {
            bar.finish()
        }
    }

    fn update_visual(&mut self, multi: &MultiProgress) {
        let visible = self.bar.is_some();

        if self.nb_done == self.nb_expected {
            if visible && let Some(bar) = self.bar.as_ref() {
                bar.set_message("");
                bar.set_position(self.nb_done);
                bar.set_length(self.nb_expected);
                bar.tick();
                // multi.remove(bar);
            }
            // self.bar = None;
        }
        else {
            let content = self.current_items.iter().join(", ");

            if !visible {
                self.bar = Some(multi.add(ProgressBar::new(self.nb_expected)));
                if let Some(bar) = self.bar.as_ref() {
                    bar.set_style(
                        ProgressStyle::with_template(PROGRESS_STYLE)
                            .unwrap()
                            .progress_chars("=> ")
                    );
                    bar.set_prefix(self.prefix);
                }
            }

            if let Some(bar) = self.bar.as_ref() {
                bar.set_message(content);
                bar.set_position(self.nb_done);
                bar.set_length(self.nb_expected);
                bar.tick();
            }
        }
    }
}

#[cfg(not(feature = "indicatif"))]
impl CountedProgress {
    pub fn new(kind: &'static str, index: usize, freeze_amount: bool) -> Self {
        CountedProgress {
            current_items: hashbag::HashBag::new(),
            nb_done: 0,
            nb_expected: 0,
            prefix: kind,
            index,
            freeze_amount,
            last_tick: std::time::SystemTime::now()
        }
    }

    fn add_item(&mut self, item: &str) {
        if !self.freeze_amount {
            self.nb_expected += 1;
        }
        self.current_items.insert(item.into());
        self.update_visual();
    }

    fn add_items<'a>(&mut self, items: impl Iterator<Item = &'a str>) {
        let mut count = 0;
        for item in items {
            self.current_items.insert(String::from(item));
            count += 1;
        }

        if !self.freeze_amount {
            self.nb_expected += count;
        }
        self.update_visual();
    }

    fn remove_item(&mut self, item: &str) {
        self.nb_done += 1;
        self.current_items.remove(item);
        self.update_visual();
    }

    fn finished(&mut self) {}

    fn update_visual(&mut self) {
        const HZ: u128 = 1000 / 15;

        if self.last_tick.elapsed().unwrap().as_millis() >= HZ {
            self.really_show();

            self.last_tick = std::time::SystemTime::now();
        }
    }

    fn really_show(&self) {
        // Suppressed: direct println! would bypass the observer/TUI channel.
    }
}

#[cfg(feature = "indicatif")]
fn new_spinner() -> ProgressBar {
    let bar = ProgressBar::new_spinner();

    bar.set_style(
        ProgressStyle::with_template("{spinner:.blue} {msg}")
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
                "▪▪▪▪▪"
            ])
    );
    bar.enable_steady_tick(REFRESH_RATE);
    bar
}

impl Default for Progress {
    fn default() -> Self {
        Self::new()
    }
}

impl Progress {
    pub fn instance() -> MutexGuard<'static, Progress> {
        PROGRESS.lock().unwrap()
    }

    #[cfg(feature = "indicatif")]
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

    #[cfg(not(feature = "indicatif"))]
    pub fn new() -> Self {
        Progress {
            load: CountedProgress::new("  Load", 0, false),
            parse: CountedProgress::new(" Parse", 1, false),
            save: None,
            pass: None
        }
    }

    pub fn add_parse(&mut self, ident: &str) {
        #[cfg(feature = "indicatif")]
        self.parse.add_item(ident, &self.multi);

        #[cfg(not(feature = "indicatif"))]
        self.parse.add_item(ident);

        notify(AsmProgressEvent::Parse {
            item: ident.to_string(),
            done: self.parse.nb_done,
            total: self.parse.nb_expected
        });
    }

    pub fn add_parses<'a>(&mut self, items: impl Iterator<Item = &'a str>) {
        #[cfg(feature = "indicatif")]
        self.parse.add_items(items, &self.multi);

        #[cfg(not(feature = "indicatif"))]
        self.parse.add_items(items);

        notify(AsmProgressEvent::Parse {
            item: String::new(),
            done: self.parse.nb_done,
            total: self.parse.nb_expected
        });
    }

    pub fn remove_parse(&mut self, ident: &str) {
        #[cfg(feature = "indicatif")]
        self.parse.remove_item(ident, &self.multi);

        #[cfg(not(feature = "indicatif"))]
        self.parse.remove_item(ident);

        notify(AsmProgressEvent::Parse {
            item: ident.to_string(),
            done: self.parse.nb_done,
            total: self.parse.nb_expected
        });
    }

    pub fn add_load(&mut self, ident: &str) {
        #[cfg(feature = "indicatif")]
        self.load.add_item(ident, &self.multi);

        #[cfg(not(feature = "indicatif"))]
        self.load.add_item(ident);

        notify(AsmProgressEvent::Load {
            item: ident.to_string(),
            done: self.load.nb_done,
            total: self.load.nb_expected
        });
    }

    pub fn add_loads<'a>(&mut self, items: impl Iterator<Item = &'a str>) {
        #[cfg(feature = "indicatif")]
        self.load.add_items(items, &self.multi);

        #[cfg(not(feature = "indicatif"))]
        self.load.add_items(items);

        notify(AsmProgressEvent::Load {
            item: String::new(),
            done: self.load.nb_done,
            total: self.load.nb_expected
        });
    }

    pub fn remove_load(&mut self, ident: &str) {
        #[cfg(feature = "indicatif")]
        self.load.remove_item(ident, &self.multi);

        #[cfg(not(feature = "indicatif"))]
        self.load.remove_item(ident);

        notify(AsmProgressEvent::Load {
            item: ident.to_string(),
            done: self.load.nb_done,
            total: self.load.nb_expected
        });
    }

    #[cfg(feature = "indicatif")]
    pub fn new_pass(&mut self) {
        if self.pass.is_none() {
            let bar = ProgressBar::new(0);
            bar.set_style(
                ProgressStyle::with_template(PASS_STYLE)
                    .unwrap()
                    .progress_chars("=> ")
            );
            self.pass = Some((0, self.multi.add(bar)));
        }
        else {
            // todo change pass numbering
        }

        if let Some((pass, bar)) = self.pass.as_mut() {
            *pass += 1;
            bar.set_prefix(format!("Pass {}", *pass));
            bar.set_position(0);
            bar.set_length(0);
        }

        notify(AsmProgressEvent::PassStarted {
            pass: self.pass.as_ref().unwrap().0
        });
    }

    #[cfg(not(feature = "indicatif"))]
    pub fn new_pass(&mut self) {
        if self.pass.is_none() {
            self.pass = Some((0, 0, 0));
        }

        if let Some(pass) = self.pass.as_mut() {
            *pass = (pass.0 + 1, 0, 0);
        }

        notify(AsmProgressEvent::PassStarted {
            pass: self.pass.as_ref().unwrap().0
        });
    }

    #[cfg(feature = "indicatif")]
    pub fn add_visited_to_pass(&mut self, amount: u64) {
        let (pass, bar) = self.pass.as_mut().unwrap();
        bar.inc(amount);

        notify(AsmProgressEvent::PassProgress {
            pass: *pass,
            visited: bar.position(),
            expected: bar.length().unwrap_or(0)
        });
    }

    #[cfg(not(feature = "indicatif"))]
    pub fn add_visited_to_pass(&mut self, amount: u64) {
        self.pass.as_mut().unwrap().1 += amount as usize;

        let (pass, visited, expected) = *self.pass.as_ref().unwrap();
        notify(AsmProgressEvent::PassProgress {
            pass,
            visited: visited as u64,
            expected: expected as u64
        });
    }

    #[cfg(feature = "indicatif")]
    pub fn add_expected_to_pass(&mut self, amount: u64) {
        let (pass, bar) = self.pass.as_mut().unwrap();
        bar.inc_length(amount);

        notify(AsmProgressEvent::PassProgress {
            pass: *pass,
            visited: bar.position(),
            expected: bar.length().unwrap_or(0)
        });
    }

    #[cfg(not(feature = "indicatif"))]
    pub fn add_expected_to_pass(&mut self, amount: u64) {
        self.pass.as_mut().unwrap().2 += amount as usize;

        let (pass, visited, expected) = *self.pass.as_ref().unwrap();
        notify(AsmProgressEvent::PassProgress {
            pass,
            visited: visited as u64,
            expected: expected as u64
        });
    }

    pub fn create_save_bar(&mut self, amount: u64) {
        let mut bar = CountedProgress::new("  Save", 2, true);
        bar.nb_expected = amount;
        self.save = Some(bar);

        notify(AsmProgressEvent::Save {
            item: String::new(),
            done: 0,
            total: amount
        });
    }

    pub fn add_save(&mut self, ident: &str) {
        #[cfg(feature = "indicatif")]
        self.save.as_mut().unwrap().add_item(ident, &self.multi);

        #[cfg(not(feature = "indicatif"))]
        self.save.as_mut().unwrap().add_item(ident);

        let save = self.save.as_ref().unwrap();
        notify(AsmProgressEvent::Save {
            item: ident.to_string(),
            done: save.nb_done,
            total: save.nb_expected
        });
    }

    pub fn remove_save(&mut self, ident: &str) {
        #[cfg(feature = "indicatif")]
        self.save.as_mut().unwrap().remove_item(ident, &self.multi);

        #[cfg(not(feature = "indicatif"))]
        self.save.as_mut().unwrap().remove_item(ident);

        let save = self.save.as_ref().unwrap();
        notify(AsmProgressEvent::Save {
            item: ident.to_string(),
            done: save.nb_done,
            total: save.nb_expected
        });
    }

    pub fn finish_save(&mut self) {
        self.save.as_mut().unwrap().finished();
        notify(AsmProgressEvent::SaveFinished);
    }

    /// Add the progress bar for a file to read
    #[cfg(feature = "indicatif")]
    pub fn add_bar(&self, msg: &str) -> ProgressBar {
        let bar = new_spinner();
        let bar = self.multi.add(bar);
        bar.set_message(msg.to_owned());
        bar
    }

    #[cfg(feature = "indicatif")]
    /// Remove the progress bar of the current file
    pub fn remove_bar_ok(&self, bar: &ProgressBar) {
        bar.disable_steady_tick();
        bar.finish_and_clear();
        bar.tick();
        self.multi.remove(bar);
    }

    #[cfg(feature = "indicatif")]
    pub fn remove_bar_err(&self, bar: &ProgressBar, msg: &str) {
        bar.disable_steady_tick();
        bar.abandon_with_message(msg.to_owned());
        bar.tick();
        self.multi.remove(bar);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    struct CountingSink(Arc<AtomicUsize>);

    impl AsmProgressSink for CountingSink {
        fn on_progress(&self, _event: AsmProgressEvent) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn a_fresh_thread_has_no_sink_installed() {
        std::thread::spawn(|| {
            assert!(!has_progress_sink());
        })
        .join()
        .unwrap();
    }

    #[test]
    fn installing_a_sink_delivers_events_until_cleared() {
        // Runs on its own thread: the sink is thread-local, so this can't
        // observe (or be observed by) any other test installing one.
        std::thread::spawn(|| {
            assert!(!has_progress_sink());

            let counter = Arc::new(AtomicUsize::new(0));
            install_progress_sink(Some(Arc::new(CountingSink(Arc::clone(&counter)))));
            assert!(has_progress_sink());

            notify(AsmProgressEvent::Parse {
                item: "x".to_string(),
                done: 1,
                total: 2
            });
            assert_eq!(counter.load(Ordering::SeqCst), 1);

            install_progress_sink(None);
            assert!(!has_progress_sink());

            // Clearing the sink stops delivery - not just future installs.
            notify(AsmProgressEvent::SaveFinished);
            assert_eq!(counter.load(Ordering::SeqCst), 1);
        })
        .join()
        .unwrap();
    }
}
