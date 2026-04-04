use std::collections::HashMap;
use std::io;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use cpclib_bndbuild::app::{BndBuilderApp, BndBuilderCommand};
use cpclib_bndbuild::event::{BndBuilderObserved, BndBuilderObserverRc};
use cpclib_runner::kill_all_children;
use ratatui::crossterm::event::{
    self, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind
};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::prelude::Backend;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::{Frame, Terminal};

use crate::model::{BuildPhase, RuleEntry, RuleStatus, TaskEntry, TaskStatus};
use crate::observer::{BndBuilderRatatuiObserver, RatatuiMessage};
use crate::ratatui_event::{RatatuiEvent, RatatuiState};
use crate::timing::TimingCache;
use crate::widgets::{RulesView, fmt_duration, strip_ansi_codes};

// ─── Constants ────────────────────────────────────────────────────────────────

const MAX_LINES_PER_TASK: usize = crate::model::MAX_LINES_PER_TASK;

// ─── App state ────────────────────────────────────────────────────────────────

pub(crate) struct BndBuilderRatatui {
    pub(crate) command: Option<BndBuilderCommand>,
    pub(crate) rx: mpsc::Receiver<RatatuiMessage>,
    pub(crate) rules: Vec<RuleEntry>,
    /// Tasks fired without a parent rule.
    pub(crate) orphans: Vec<TaskEntry>,
    pub(crate) phase: BuildPhase,
    /// Entry-based scroll (rules + orphan-tasks). `None` = auto-follow.
    pub(crate) scroll: Option<usize>,
    pub(crate) exit: bool,
    /// True while the "confirm quit" modal is shown.
    pub(crate) confirm_quit: bool,
    /// Aliases waiting to be attached to their representative's RuleEntry.
    /// Key = representative path string, value = list of alias path strings.
    pub(crate) pending_aliases: HashMap<String, Vec<String>>,
    /// Index into `rules` of the currently selected rule (for manual task scrolling).
    pub(crate) selected_rule: Option<usize>,
    /// Error message from the build thread, set when the build fails.
    pub(crate) build_error: Option<String>,
    /// When the first RunTasks event arrives (build actually starts executing).
    pub(crate) build_started: Option<Instant>,
    /// Total duration snapped when the build finishes (so it stops growing).
    pub(crate) build_duration: Option<Duration>,
    /// Active build file for nested bndbuild invocations (tagged onto new rules).
    pub(crate) current_build_file: Option<String>,
    /// Nesting depth of RunTasks/Finish pairs. The outermost build has depth 1
    /// while running; depth returns to 0 only when the outermost Finish arrives.
    /// Nested bndbuild tasks increment this when their RunTasks fires and
    /// decrement it on their Finish, so we can ignore premature inner Finishes.
    pub(crate) build_nesting_depth: usize,
    /// Build-time prediction cache (`.bndbuild_timings` in the working directory).
    pub(crate) timing_cache: TimingCache,
    /// Absolute time at which the build is estimated to finish.
    /// Precomputed on each event so `draw()` never does a full rules scan.
    pub(crate) estimated_finish: Option<std::time::Instant>,
    /// When true, UpToDate rules are collapsed into a single summary line.
    pub(crate) collapse_uptodate: bool,
    /// When true, the `?` help overlay is shown.
    pub(crate) show_help: bool,
    /// Set to a status string after the user presses `p` to save a profile.
    /// Stays visible in the status bar for the remainder of the session.
    pub(crate) profile_msg: Option<String>,
    /// `--profile FILE` path from CLI args: if set, auto-saves on build completion.
    pub(crate) profile_output: Option<String>
}

impl BndBuilderRatatui {
    // ── Event routing ─────────────────────────────────────────────────────────

    pub(crate) fn running_rule_mut(&mut self, name: &str) -> Option<&mut RuleEntry> {
        self.rules
            .iter_mut()
            .rev()
            .find(|r| r.is_running() && r.name == name)
    }

    pub(crate) fn running_task_mut(&mut self, task_name: &str) -> Option<&mut TaskEntry> {
        for rule in self.rules.iter_mut().rev() {
            if let Some(t) = rule
                .tasks
                .iter_mut()
                .rev()
                .find(|t| t.is_running() && t.task == task_name)
            {
                return Some(t);
            }
        }
        self.orphans
            .iter_mut()
            .rev()
            .find(|t| t.is_running() && t.task == task_name)
    }

    /// Like running_task_mut but falls back to any (most-recent) task with the same name.
    /// Used for stdout/stderr routing so output is never silently dropped if StopTask
    /// has already been processed (can happen in parallel builds).
    pub(crate) fn any_task_mut(&mut self, task_name: &str) -> Option<&mut TaskEntry> {
        // Two-phase (find index, then take mutable ref) to satisfy the borrow checker.

        // Phase 1: running tasks in rules.
        let idx = self.rules.iter().enumerate().rev().find_map(|(ri, rule)| {
            rule.tasks
                .iter()
                .enumerate()
                .rev()
                .find(|(_, t)| t.is_running() && t.task == task_name)
                .map(|(ti, _)| (ri, ti))
        });
        if let Some((ri, ti)) = idx {
            return Some(&mut self.rules[ri].tasks[ti]);
        }

        // Phase 2: running orphans.
        let idx = self
            .orphans
            .iter()
            .enumerate()
            .rev()
            .find(|(_, t)| t.is_running() && t.task == task_name)
            .map(|(i, _)| i);
        if let Some(i) = idx {
            return Some(&mut self.orphans[i]);
        }

        // Phase 3: fallback — any task in rules (most recent).
        let idx = self.rules.iter().enumerate().rev().find_map(|(ri, rule)| {
            rule.tasks
                .iter()
                .enumerate()
                .rev()
                .find(|(_, t)| t.task == task_name)
                .map(|(ti, _)| (ri, ti))
        });
        if let Some((ri, ti)) = idx {
            return Some(&mut self.rules[ri].tasks[ti]);
        }

        // Phase 4: fallback — any orphan.
        let idx = self
            .orphans
            .iter()
            .enumerate()
            .rev()
            .find(|(_, t)| t.task == task_name)
            .map(|(i, _)| i);
        idx.map(|i| &mut self.orphans[i])
    }

    // ── Event application ─────────────────────────────────────────────────────

    /// Route output lines from a task into its stdout/stderr buffer and stamp
    /// `last_output` on the parent rule to trigger a brief border flash.
    fn push_task_output(&mut self, task_name: &str, output: &str, is_stderr: bool) {
        let parent_rule = if let Some(t) = self.any_task_mut(task_name) {
            for line in output.lines() {
                let clean = strip_ansi_codes(line);
                let clean = clean.trim_end_matches('\r');
                if !clean.is_empty() {
                    let buf = if is_stderr {
                        &mut t.stderr
                    }
                    else {
                        &mut t.stdout
                    };
                    if buf.len() >= MAX_LINES_PER_TASK {
                        buf.pop_front();
                    }
                    buf.push_back(clean.to_owned());
                }
            }
            t.parent_rule.clone()
        }
        else {
            None
        };
        if let Some(rn) = parent_rule {
            if let Some(r) = self.rules.iter_mut().rev().find(|r| r.name == rn) {
                r.last_output = Some(Instant::now());
            }
        }
    }

    pub(crate) fn apply_event(&mut self, event: RatatuiEvent) {
        match event {
            RatatuiEvent::ChangeState(state) => {
                match state {
                    RatatuiState::ComputeDependencies(p) => {
                        self.phase = BuildPhase::ComputingDeps(p)
                    },
                    RatatuiState::RunTasks => {
                        self.build_started.get_or_insert_with(Instant::now);
                        self.build_nesting_depth += 1;
                        self.phase = BuildPhase::Running {
                            current: 0,
                            total: 0
                        }
                    },
                    RatatuiState::Finish => {
                        if self.build_nesting_depth > 0 {
                            self.build_nesting_depth -= 1;
                        }
                        // Only snap duration and transition to Finished for the
                        // outermost build. Nested bndbuild tasks also fire Finish;
                        // those must be ignored so the header keeps counting up.
                        if self.build_nesting_depth == 0 {
                            if let Some(t) = self.build_started {
                                self.build_duration.get_or_insert_with(|| t.elapsed());
                            }
                            self.phase = BuildPhase::Finished;
                            // Auto-jump to first failed rule so cause of failure is immediately visible.
                            self.selected_rule = self
                                .rules
                                .iter()
                                .position(|r| matches!(r.status, RuleStatus::Failed(_)));
                            // Persist timing data immediately so that a terminal
                            // close (Ctrl+C, window X) does not lose the samples.
                            self.timing_cache.save().ok();
                            // Auto-save profile immediately on build completion so that
                            // Ctrl+C or window-close does not lose the report.
                            if let (Some(path_str), Some(started), Some(dur)) = (
                                self.profile_output.as_deref(),
                                self.build_started,
                                self.build_duration
                            ) {
                                let path = std::path::PathBuf::from(path_str);
                                crate::profile::save_profile(&self.rules, started, dur, &path).ok();
                            }
                        }
                    }
                }
            },

            RatatuiEvent::StartRuleAlias {
                alias,
                representative,
                ..
            } => {
                self.pending_aliases
                    .entry(representative)
                    .or_default()
                    .push(alias);
            },

            RatatuiEvent::StartRule { rule, nb, out_of } => {
                let aliases = self.pending_aliases.remove(&rule).unwrap_or_default();
                let mut entry = RuleEntry::new(rule.clone(), nb, out_of);
                entry.aliases = aliases;
                entry.source = self.current_build_file.clone();
                // Look up the historical average duration for this rule so the
                // widget can show an ETA while it is running.
                entry.estimated_duration = self.timing_cache.estimate(
                    self.current_build_file.as_deref().unwrap_or(""),
                    &rule,
                    ""
                );
                self.rules.push(entry);
                self.phase = BuildPhase::Running {
                    current: nb,
                    total: out_of
                };
                self.scroll = None; // auto-follow
                self.recompute_eta();
            },

            RatatuiEvent::StopRule(rule) => {
                // Phase 1: update status and capture what we need for the cache,
                // ending the mutable borrow before we touch self.timing_cache.
                let timing = if let Some(r) = self.running_rule_mut(&rule) {
                    let d = r.started.elapsed();
                    let source = r.source.clone();
                    r.status = RuleStatus::Success(d);
                    Some((source, d))
                }
                else {
                    None
                };
                // Phase 2: persist the new sample (only for successful completions).
                if let Some((source, d)) = timing {
                    self.timing_cache
                        .record(source.as_deref().unwrap_or(""), &rule, "", d);
                }
                self.recompute_eta();
            },

            RatatuiEvent::SkippedRule(rule) => {
                if let Some(r) = self.running_rule_mut(&rule) {
                    r.status = RuleStatus::UpToDate;
                    // UpToDate rules were never built — do NOT record a timing sample.
                }
                self.recompute_eta();
            },

            RatatuiEvent::BuildFileContext(ctx) => {
                self.current_build_file = ctx;
            },

            RatatuiEvent::FailedRule(rule) => {
                let timing = if let Some(r) = self.running_rule_mut(&rule) {
                    let d = r.started.elapsed();
                    for t in r.tasks.iter_mut() {
                        if t.is_running() {
                            t.status = TaskStatus::Failed(t.started.elapsed());
                        }
                    }
                    let source = r.source.clone();
                    r.status = RuleStatus::Failed(d);
                    Some((source, d))
                }
                else {
                    None
                };
                // Record partial timing even for failures so ETA can learn from
                // builds that fail after significant work on this rule.
                if let Some((source, d)) = timing {
                    self.timing_cache
                        .record(source.as_deref().unwrap_or(""), &rule, "", d);
                }
                self.recompute_eta();
            },

            RatatuiEvent::StartTask {
                rule: Some(rule_name),
                task
            } => {
                // Phase 1: collect the rule's build-file context and look up the
                // cached estimate (read-only borrows end here).
                let build_file = self
                    .rules
                    .iter()
                    .rev()
                    .find(|r| r.is_running() && r.name == rule_name)
                    .and_then(|r| r.source.clone());
                let est = self.timing_cache.estimate(
                    build_file.as_deref().unwrap_or(""),
                    &rule_name,
                    &task
                );
                // Phase 2: push the task (mutable borrow acceptable now).
                if let Some(r) = self.running_rule_mut(&rule_name) {
                    let mut t = TaskEntry::new(task);
                    t.estimated_duration = est;
                    t.parent_rule = Some(rule_name);
                    t.parent_build_file = build_file;
                    r.tasks.push(t);
                }
                else {
                    self.orphans.push(TaskEntry::new(task));
                }
                // Note: starting a task does not meaningfully change the global ETA.
            },

            RatatuiEvent::StartTask { rule: None, task } => {
                self.orphans.push(TaskEntry::new(task));
            },

            RatatuiEvent::StopTask {
                rule,
                task,
                duration
            } => {
                // Use the build-file path already stored on the task entry to avoid
                // a second rules scan.  Fall back to searching the rules vec only when
                // the task was already stopped (rare race in parallel builds).
                let build_file = if let Some(t) = self.running_task_mut(&task) {
                    let bf = t.parent_build_file.clone();
                    t.status = TaskStatus::Success(duration);
                    bf
                }
                else {
                    rule.as_ref().and_then(|rn| {
                        self.rules
                            .iter()
                            .rev()
                            .find(|r| r.name == *rn)
                            .and_then(|r| r.source.clone())
                    })
                };
                if let Some(rule_name) = &rule {
                    self.timing_cache.record(
                        build_file.as_deref().unwrap_or(""),
                        rule_name,
                        &task,
                        duration
                    );
                }
            },

            RatatuiEvent::TaskStdout { task, output, .. } => {
                self.push_task_output(&task, &output, false);
            },

            RatatuiEvent::TaskStderr { task, output, .. } => {
                self.push_task_output(&task, &output, true);
            },

            RatatuiEvent::Stdout(_) | RatatuiEvent::Stderr(_) => {}
        }
    }

    pub(crate) fn handle_message(&mut self, msg: RatatuiMessage) {
        if let RatatuiMessage::NewEvent(ev) = msg {
            self.apply_event(ev);
        }
    }

    // ── Scroll helpers ────────────────────────────────────────────────────────

    pub(crate) fn total_entries(&self) -> usize {
        self.rules.len() + self.orphans.len()
    }

    /// How many entries to skip so that content from that entry onward fills
    /// `visible_rows` rows (or fills as much as possible).
    pub(crate) fn bottom_skip(&self, visible_rows: u16) -> usize {
        let total = self.total_entries();
        if total == 0 {
            return 0;
        }
        // Iterate from the bottom (orphans first, then rules) without allocating.
        // The first entry is always included even if it is taller than the screen.
        let rule_heights = self.rules.iter().rev().map(|r| {
            if self.collapse_uptodate && matches!(r.status, RuleStatus::UpToDate) {
                0
            }
            else {
                r.height()
            }
        });
        let heights_from_bottom = self
            .orphans
            .iter()
            .rev()
            .map(|t| t.inline_height())
            .chain(rule_heights);
        let mut remaining = visible_rows;
        let mut visible = 0usize;
        for h in heights_from_bottom {
            if visible == 0 || h <= remaining {
                visible += 1;
                remaining = remaining.saturating_sub(h);
            }
            else {
                break;
            }
        }
        total.saturating_sub(visible)
    }

    pub(crate) fn effective_skip(&self, list_h: u16) -> usize {
        match self.scroll {
            None => self.bottom_skip(list_h),
            Some(n) => n.min(self.total_entries())
        }
    }

    // ── Selection & scroll helpers ────────────────────────────────────────────

    /// Change the selected rule, resetting its scroll state when a new rule is chosen.
    fn select_rule(&mut self, new_sel: Option<usize>) {
        if new_sel != self.selected_rule {
            if let Some(idx) = new_sel {
                if let Some(r) = self.rules.get_mut(idx) {
                    r.task_scroll = 0;
                    r.h_scroll = 0;
                }
            }
            self.selected_rule = new_sel;
        }
    }

    /// Scroll toward newer content (Down / ScrollDown).
    fn scroll_list_down(&mut self, skip: usize) {
        if let Some(idx) = self.selected_rule {
            if let Some(rule) = self.rules.get_mut(idx) {
                rule.task_scroll = rule.task_scroll.saturating_sub(1);
            }
        }
        else {
            self.scroll = Some(skip.saturating_add(1).min(self.total_entries()));
        }
    }

    /// Scroll toward older content (Up / ScrollUp).
    fn scroll_list_up(&mut self, skip: usize) {
        if let Some(idx) = self.selected_rule {
            if let Some(rule) = self.rules.get_mut(idx) {
                rule.task_scroll = rule.task_scroll.saturating_add(1);
            }
        }
        else {
            self.scroll = Some(skip.saturating_sub(1));
        }
    }

    // ── ETA ───────────────────────────────────────────────────────────────────

    /// Recompute `estimated_finish` from the current rule states.
    ///
    /// Called on every relevant event (rule start/stop/fail/skip) so that
    /// `draw()` can read a pre-computed value without iterating rules per frame.
    pub(crate) fn recompute_eta(&mut self) {
        let total_expected = self.rules.iter().map(|r| r.out_of).max().unwrap_or(0);
        let not_started = total_expected.saturating_sub(self.rules.len());

        // Remaining time for currently running rules with a cached estimate.
        let mut any_data = false;
        let running_rem: Duration = self
            .rules
            .iter()
            .filter(|r| r.is_running())
            .filter_map(|r| {
                let est = r.estimated_duration?;
                any_data = true;
                let spent = r.started.elapsed();
                Some(est.saturating_sub(spent))
            })
            // Parallel rules run concurrently; the critical path is the
            // longest remaining lane, not the sum of all lanes.
            .max()
            .unwrap_or(Duration::ZERO);

        // Average estimate across all rules that have cache data.
        let (known_sum, known_count) = self
            .rules
            .iter()
            .filter_map(|r| r.estimated_duration)
            .fold((0u128, 0usize), |(s, c), d| (s + d.as_nanos(), c + 1));
        let avg_rule: Option<Duration> = if known_count == 0 {
            None
        }
        else {
            any_data = true;
            Some(Duration::from_nanos(
                (known_sum / known_count as u128) as u64
            ))
        };

        if !any_data {
            self.estimated_finish = None;
            return;
        }

        let future_rem = avg_rule
            .map(|avg| avg * not_started as u32)
            .unwrap_or(Duration::ZERO);

        self.estimated_finish = Some(std::time::Instant::now() + running_rem + future_rem);
    }

    // ── Run loop ──────────────────────────────────────────────────────────────

    pub(crate) fn run<T: Backend>(&mut self, mut terminal: Terminal<T>) -> io::Result<()> {
        let cmd = self.command.take().expect("command is required");
        assert!(cmd.is_build());

        let mut thread = Some(std::thread::spawn(move || cmd.execute()));

        // Ensure the screen starts clean — ratatui diffs against an empty
        // previous buffer, so without this the first frame may be invisible.
        terminal
            .clear()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        let mut thread_result: Option<Result<(), cpclib_bndbuild::BndBuilderError>> = None;

        'main: loop {
            // Check whether the build thread has finished (success or failure).
            // If so, drain all pending messages and force the phase to Finished
            // so the user can see the final state and press 'q' to exit.
            if thread_result.is_none() {
                if thread.as_ref().is_some_and(|t| t.is_finished()) {
                    let handle = thread.take().unwrap();
                    match handle.join() {
                        Ok(res) => thread_result = Some(res),
                        Err(_) => {
                            thread_result = Some(Err(cpclib_bndbuild::BndBuilderError::AnyError(
                                "build thread panicked".into()
                            )))
                        },
                    }
                    // Drain any messages the thread sent before finishing.
                    while let Ok(msg) = self.rx.try_recv() {
                        self.handle_message(msg);
                    }
                    // Ensure phase reflects completion even if do_finish() was
                    // never called (e.g. build failed before reaching it).
                    if !matches!(self.phase, BuildPhase::Finished) {
                        self.phase = BuildPhase::Finished;
                    }
                    // Auto-jump to first failed rule for immediate failure visibility.
                    self.selected_rule = self
                        .rules
                        .iter()
                        .position(|r| matches!(r.status, RuleStatus::Failed(_)));
                    // Snap the elapsed duration if not already recorded.
                    if let Some(t) = self.build_started {
                        self.build_duration.get_or_insert_with(|| t.elapsed());
                    }
                    // Auto-save profile for builds that finish without a Finish
                    // message (e.g. the build failed and do_finish was skipped).
                    // The Finish handler already covers the success case, but this
                    // ensures the file is also written on error paths.
                    if let (Some(path_str), Some(started), Some(dur)) = (
                        self.profile_output.as_deref(),
                        self.build_started,
                        self.build_duration
                    ) {
                        let path = std::path::PathBuf::from(path_str);
                        crate::profile::save_profile(&self.rules, started, dur, &path).ok();
                    }
                    // If the build ended with an error, mark any still-running
                    // rules and tasks as Failed so the TUI reflects the failure.
                    if matches!(thread_result, Some(Err(_))) {
                        let err_msg = thread_result
                            .as_ref()
                            .and_then(|r| r.as_ref().err())
                            .map(|e| e.to_string())
                            .unwrap_or_default();
                        self.build_error = Some(err_msg);
                        for rule in &mut self.rules {
                            if rule.is_running() {
                                let elapsed = rule.started.elapsed();
                                for task in &mut rule.tasks {
                                    if task.is_running() {
                                        task.status = TaskStatus::Failed(task.started.elapsed());
                                    }
                                }
                                rule.status = RuleStatus::Failed(elapsed);
                            }
                        }
                        for task in &mut self.orphans {
                            if task.is_running() {
                                task.status = TaskStatus::Failed(task.started.elapsed());
                            }
                        }
                    }
                }
            }

            let list_h = terminal
                .size()
                .map(|s| s.height.saturating_sub(2))
                .unwrap_or(20);
            let skip = self.effective_skip(list_h);

            terminal
                .draw(|f| self.draw(f, skip))
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

            if event::poll(Duration::from_millis(16))? {
                match event::read()? {
                    event::Event::Key(k) => {
                        if k.kind == KeyEventKind::Press {
                            // Ctrl+C: restore terminal and kill immediately.
                            if k.code == KeyCode::Char('c')
                                && k.modifiers.contains(KeyModifiers::CONTROL)
                            {
                                kill_all_children();
                                drop(terminal);
                                crate::terminal::restore_terminal().ok();
                                std::process::exit(130);
                            }
                            match k.code {
                                KeyCode::Esc => {
                                    self.confirm_quit = false;
                                    self.selected_rule = None;
                                },
                                KeyCode::Tab => {
                                    self.confirm_quit = false;
                                    let n = self.rules.len();
                                    let new_sel = match self.selected_rule {
                                        None => {
                                            if n > 0 {
                                                Some(0)
                                            }
                                            else {
                                                None
                                            }
                                        },
                                        Some(i) => {
                                            if i + 1 < n {
                                                Some(i + 1)
                                            }
                                            else {
                                                None
                                            }
                                        },
                                    };
                                    self.select_rule(new_sel);
                                },
                                KeyCode::BackTab => {
                                    self.confirm_quit = false;
                                    let n = self.rules.len();
                                    let new_sel = match self.selected_rule {
                                        None => {
                                            if n > 0 {
                                                Some(n - 1)
                                            }
                                            else {
                                                None
                                            }
                                        },
                                        Some(0) => None,
                                        Some(i) => Some(i - 1)
                                    };
                                    self.select_rule(new_sel);
                                },
                                KeyCode::Char('q') | KeyCode::Char('Q') => {
                                    let build_done = matches!(self.phase, BuildPhase::Finished);
                                    if build_done {
                                        self.exit = true;
                                        break 'main;
                                    }
                                    else if self.confirm_quit {
                                        self.timing_cache.save().ok();
                                        kill_all_children();
                                        drop(terminal);
                                        crate::terminal::restore_terminal().ok();
                                        std::process::exit(1);
                                    }
                                    else {
                                        self.confirm_quit = true;
                                    }
                                },
                                KeyCode::Down | KeyCode::Char('j') => {
                                    self.confirm_quit = false;
                                    self.scroll_list_down(skip);
                                },
                                KeyCode::Up | KeyCode::Char('k') => {
                                    self.confirm_quit = false;
                                    self.scroll_list_up(skip);
                                },
                                KeyCode::Left => {
                                    if let Some(idx) = self.selected_rule {
                                        if let Some(rule) = self.rules.get_mut(idx) {
                                            rule.h_scroll = rule.h_scroll.saturating_sub(4);
                                        }
                                    }
                                },
                                KeyCode::Right => {
                                    if let Some(idx) = self.selected_rule {
                                        if let Some(rule) = self.rules.get_mut(idx) {
                                            rule.h_scroll = rule.h_scroll.saturating_add(4);
                                        }
                                    }
                                },
                                KeyCode::Home | KeyCode::Char('g') => {
                                    self.scroll = Some(0);
                                },
                                KeyCode::End | KeyCode::Char('G') => {
                                    self.scroll = None; // auto-follow = bottom
                                },
                                KeyCode::PageUp => {
                                    let page = (list_h as usize).saturating_sub(1);
                                    self.scroll = Some(skip.saturating_sub(page));
                                },
                                KeyCode::PageDown => {
                                    let page = (list_h as usize).saturating_sub(1);
                                    let next = skip.saturating_add(page);
                                    self.scroll = Some(next.min(self.total_entries()));
                                },
                                KeyCode::Char('r') | KeyCode::Char('R') => {
                                    self.confirm_quit = false;
                                    self.show_help = false;
                                    let build_done = matches!(self.phase, BuildPhase::Finished);
                                    if build_done && thread.is_none() {
                                        // Re-parse CLI args for a fresh command — avoids
                                        // needing Clone on BndBuilderCommand/BndBuilderCommandInner.
                                        if let Ok(Some(new_app)) = BndBuilderApp::new() {
                                            let new_build_file =
                                                new_app.build_file().map(|s| s.to_owned());
                                            if let Ok(mut retry_cmd) = new_app.command() {
                                                let (tx2, rx2) = mpsc::channel::<RatatuiMessage>();
                                                retry_cmd.clear_observers();
                                                retry_cmd.add_observer(BndBuilderObserverRc::new(
                                                    BndBuilderRatatuiObserver::new(tx2)
                                                ));
                                                // Reset all state for a fresh build run.
                                                self.rules.clear();
                                                self.orphans.clear();
                                                self.phase = BuildPhase::default();
                                                self.scroll = None;
                                                self.selected_rule = None;
                                                self.build_error = None;
                                                self.build_started = None;
                                                self.build_duration = None;
                                                self.pending_aliases.clear();
                                                self.current_build_file = new_build_file;
                                                self.build_nesting_depth = 0;
                                                self.estimated_finish = None;
                                                self.rx = rx2;
                                                thread_result = None;
                                                thread = Some(std::thread::spawn(move || {
                                                    retry_cmd.execute()
                                                }));
                                            }
                                        }
                                    }
                                },
                                KeyCode::Char('u') | KeyCode::Char('U') => {
                                    self.confirm_quit = false;
                                    self.collapse_uptodate = !self.collapse_uptodate;
                                    // Deselect if the selected rule is now being collapsed.
                                    if self.collapse_uptodate {
                                        if let Some(idx) = self.selected_rule {
                                            if let Some(r) = self.rules.get(idx) {
                                                if matches!(r.status, RuleStatus::UpToDate) {
                                                    self.selected_rule = None;
                                                }
                                            }
                                        }
                                    }
                                },
                                KeyCode::Char('?') => {
                                    self.confirm_quit = false;
                                    self.show_help = !self.show_help;
                                },
                                KeyCode::Char('p') | KeyCode::Char('P') => {
                                    self.confirm_quit = false;
                                    let build_done = matches!(self.phase, BuildPhase::Finished);
                                    if build_done {
                                        if let (Some(started), Some(dur)) =
                                            (self.build_started, self.build_duration)
                                        {
                                            let path_str = self
                                                .profile_output
                                                .as_deref()
                                                .unwrap_or("bndbuild-profile.html");
                                            let path = std::path::Path::new(path_str);
                                            match crate::profile::save_profile(
                                                &self.rules,
                                                started,
                                                dur,
                                                path
                                            ) {
                                                Ok(()) => {
                                                    self.profile_msg = Some(format!(
                                                        "Profile saved \u{2192} {path_str}"
                                                    ));
                                                },
                                                Err(e) => {
                                                    self.profile_msg =
                                                        Some(format!("Profile error: {e}"));
                                                }
                                            }
                                        }
                                    }
                                },
                                _ => {}
                            }
                        }
                    },
                    event::Event::Mouse(mouse_ev) => {
                        match mouse_ev.kind {
                            MouseEventKind::Down(MouseButton::Left) => {
                                let row = mouse_ev.row;
                                // Row 0 = header, last row = status bar; list starts at row 1.
                                if row >= 1 {
                                    let list_row = row - 1;
                                    let mut cur_y = 0u16;
                                    let mut clicked: Option<usize> = None;
                                    for (idx, rule) in self.rules.iter().enumerate() {
                                        if idx < skip {
                                            continue;
                                        }
                                        let h = rule.height();
                                        if list_row >= cur_y && list_row < cur_y + h {
                                            clicked = Some(idx);
                                            break;
                                        }
                                        cur_y += h;
                                    }
                                    if let Some(idx) = clicked {
                                        // Clicking the already-selected rule deselects it.
                                        self.selected_rule = if self.selected_rule == Some(idx) {
                                            None
                                        }
                                        else {
                                            Some(idx)
                                        };
                                    }
                                }
                            },
                            MouseEventKind::ScrollUp => {
                                self.scroll_list_up(skip);
                            },
                            MouseEventKind::ScrollDown => {
                                self.scroll_list_down(skip);
                            },
                            _ => {}
                        }
                    },
                    _ => {}
                }
            }

            while let Ok(msg) = self.rx.try_recv() {
                self.handle_message(msg);
            }
        }

        self.timing_cache.save().ok();
        // Auto-save profile if --profile was specified on the command line.
        if let (Some(path_str), Some(started), Some(dur)) = (
            &self.profile_output,
            self.build_started,
            self.build_duration
        ) {
            if let Err(e) = crate::profile::save_profile(
                &self.rules,
                started,
                dur,
                std::path::Path::new(path_str)
            ) {
                eprintln!("Warning: could not save profile: {e}");
            }
        }
        match thread_result {
            Some(Ok(())) => Ok(()),
            Some(Err(e)) => Err(io::Error::new(io::ErrorKind::Other, e.to_string())),
            // Thread was still running when the user quit (q during build).
            None => Ok(())
        }
    }

    // ── Drawing ───────────────────────────────────────────────────────────────

    pub(crate) fn draw(&self, frame: &mut Frame, skip: usize) {
        let area = frame.area();
        let chunks = Layout::vertical([
            Constraint::Length(1), // header
            Constraint::Fill(1),   // rule list
            Constraint::Length(1)  // status bar
        ])
        .split(area);

        // Header
        let elapsed_str = self
            .build_duration
            .or_else(|| self.build_started.map(|t| t.elapsed()))
            .map(|d| format!("  {}", fmt_duration(d)))
            .unwrap_or_default();
        let eta_str: String = if matches!(&self.phase, BuildPhase::Running { .. }) {
            self.estimated_finish
                .map(|finish| {
                    let remaining = finish.saturating_duration_since(std::time::Instant::now());
                    format!("  ETA ~{}", fmt_duration(remaining))
                })
                .unwrap_or_default()
        }
        else {
            String::new()
        };
        match &self.phase {
            BuildPhase::Idle => {
                frame.render_widget(
                    Paragraph::new("bndbuild").style(Style::default().add_modifier(Modifier::BOLD)),
                    chunks[0]
                );
            },
            BuildPhase::ComputingDeps(p) => {
                let text = format!("Computing dependencies: {p}");
                frame.render_widget(
                    Paragraph::new(text).style(Style::default().add_modifier(Modifier::BOLD)),
                    chunks[0]
                );
            },
            BuildPhase::Running { .. } => {
                let global_current = self.rules.len();
                let global_total: usize = self
                    .rules
                    .iter()
                    .filter(|r| r.nb == 1)
                    .map(|r| r.out_of)
                    .sum();
                let mut spans = vec![Span::styled(
                    "Building ",
                    Style::default().add_modifier(Modifier::BOLD)
                )];
                if global_total > 0 {
                    const BAR_W: usize = 20;
                    let done = (global_current * BAR_W / global_total).min(BAR_W);
                    let filled = "\u{2588}".repeat(done);
                    let empty = "\u{2591}".repeat(BAR_W - done);
                    spans.push(Span::styled("[", Style::default().fg(Color::DarkGray)));
                    spans.push(Span::styled(filled, Style::default().fg(Color::Green)));
                    spans.push(Span::styled(empty, Style::default().fg(Color::DarkGray)));
                    spans.push(Span::styled("]", Style::default().fg(Color::DarkGray)));
                    spans.push(Span::styled(
                        format!(" {global_current}/{global_total}"),
                        Style::default().fg(Color::DarkGray)
                    ));
                }
                else {
                    spans.push(Span::raw("\u{2026}"));
                }
                if !elapsed_str.is_empty() {
                    spans.push(Span::styled(
                        elapsed_str.clone(),
                        Style::default().fg(Color::DarkGray)
                    ));
                }
                if !eta_str.is_empty() {
                    spans.push(Span::styled(
                        eta_str.clone(),
                        Style::default().fg(Color::Cyan)
                    ));
                }
                frame.render_widget(Paragraph::new(Line::from(spans)), chunks[0]);
            },
            BuildPhase::Finished => {
                let (text, style) = if let Some(err) = &self.build_error {
                    let short = if err.len() > 100 {
                        format!("\u{2717} Build FAILED{elapsed_str}: {}\u{2026}", &err[..97])
                    }
                    else {
                        format!("\u{2717} Build FAILED{elapsed_str}: {err}")
                    };
                    (
                        short,
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
                    )
                }
                else {
                    (
                        format!("\u{2713} Build complete{elapsed_str}"),
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD)
                    )
                };
                frame.render_widget(Paragraph::new(text).style(style), chunks[0]);
            }
        }

        // Rule list
        frame.render_widget(
            RulesView {
                rules: &self.rules,
                orphans: &self.orphans,
                skip,
                selected_rule: self.selected_rule,
                collapse_uptodate: self.collapse_uptodate
            },
            chunks[1]
        );

        // Status bar — single pass over rules for all counts.
        let (running_rules, done_rules, skipped_rules, failed_rules) =
            self.rules
                .iter()
                .fold((0usize, 0, 0, 0), |(run, done, skip, fail), r| {
                    match &r.status {
                        RuleStatus::Running => (run + 1, done, skip, fail),
                        RuleStatus::Success(_) => (run, done + 1, skip, fail),
                        RuleStatus::UpToDate => (run, done, skip + 1, fail),
                        RuleStatus::Failed(_) => (run, done, skip, fail + 1)
                    }
                });
        let running_tasks: usize = self
            .rules
            .iter()
            .flat_map(|r| r.tasks.iter())
            .filter(|t| t.is_running())
            .count()
            + self.orphans.iter().filter(|t| t.is_running()).count();

        let rn = |n: usize| {
            if n == 1 {
                "rule".to_owned()
            }
            else {
                "rules".to_owned()
            }
        };
        let tn = |n: usize| {
            if n == 1 {
                "task".to_owned()
            }
            else {
                "tasks".to_owned()
            }
        };

        let (status_text, status_style) = match &self.phase {
            BuildPhase::Finished => {
                if let Some(msg) = &self.profile_msg {
                    (
                        msg.clone(),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    )
                }
                else if failed_rules > 0 {
                    (
                        format!(
                            "\u{2717} Build failed  \u{b7}  {done_rules} {} done  {skipped_rules} skipped  {failed_rules} {} failed  \u{b7}  q quit  r retry  p:profile  tab select  \u{2191}\u{2193}/\u{2190}\u{2192}  ?:help",
                            rn(done_rules),
                            rn(failed_rules)
                        ),
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
                    )
                }
                else {
                    (
                        format!(
                            "\u{2713} Build complete  \u{b7}  {done_rules} {} done  {skipped_rules} skipped  \u{b7}  q quit  r retry  p:profile  tab select  \u{2191}\u{2193}/\u{2190}\u{2192}  ?:help",
                            rn(done_rules)
                        ),
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD)
                    )
                }
            },
            _ => {
                (
                    format!(
                        "Rules: {running_rules} running  {done_rules} done  {skipped_rules} skipped  {failed_rules} failed \
                     \u{b7}  {running_tasks} {} active  \u{b7}  q quit  ^C force-quit  \u{2191}\u{2193}/jk scroll  tab select  u:collapse  ?:help",
                        tn(running_tasks)
                    ),
                    Style::default().fg(Color::DarkGray)
                )
            },
        };
        frame.render_widget(Paragraph::new(status_text).style(status_style), chunks[2]);

        // Confirm-quit modal overlay
        if self.confirm_quit {
            self.draw_confirm_modal(frame);
        }
        // Help overlay (drawn on top of confirm-quit if both are active)
        if self.show_help {
            self.draw_help_modal(frame);
        }
    }

    /// Return a rectangle centered within `area` capped to at most `w`×`h`.
    fn centered_rect(area: Rect, w: u16, h: u16) -> Rect {
        let modal_w = w.min(area.width);
        let modal_h = h.min(area.height);
        Rect {
            x: area.x + area.width.saturating_sub(modal_w) / 2,
            y: area.y + area.height.saturating_sub(modal_h) / 2,
            width: modal_w,
            height: modal_h
        }
    }

    fn draw_confirm_modal(&self, frame: &mut Frame) {
        let modal_rect = Self::centered_rect(frame.area(), 54, 5);
        frame.render_widget(Clear, modal_rect);
        frame.render_widget(
            Paragraph::new(
                "Build is still running.\n\nPress Q again to quit  \u{b7}  Esc to continue"
            )
            .block(
                Block::default()
                    .title(" Confirm Quit ")
                    .borders(Borders::ALL)
                    .border_style(
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    )
            )
            .style(Style::default().fg(Color::White)),
            modal_rect
        );
    }

    fn draw_help_modal(&self, frame: &mut Frame) {
        let modal_rect = Self::centered_rect(frame.area(), 60, 16);
        frame.render_widget(Clear, modal_rect);
        let key = |k: &'static str| Span::styled(k, Style::default().fg(Color::Yellow));
        let desc = |d: &'static str| Span::raw(d);
        let lines = vec![
            Line::from(vec![
                key("  \u{2191}/\u{2193}  j/k        "),
                desc("Scroll rules"),
            ]),
            Line::from(vec![key("  Tab/Shift+Tab   "), desc("Select/expand rule")]),
            Line::from(vec![
                key("  \u{2190}/\u{2192}             "),
                desc("Horizontal scroll (in selection)"),
            ]),
            Line::from(vec![key("  PgUp/PgDn       "), desc("Page up/down")]),
            Line::from(vec![key("  g/Home          "), desc("Go to top")]),
            Line::from(vec![key("  G/End           "), desc("Go to bottom")]),
            Line::from(vec![
                key("  u               "),
                desc("Toggle up-to-date rule collapse"),
            ]),
            Line::from(vec![
                key("  r               "),
                desc("Retry build (after build finishes)"),
            ]),
            Line::from(vec![
                key("  p               "),
                desc("Save build profile to bndbuild-profile.html"),
            ]),
            Line::from(vec![
                key("  q               "),
                desc("Quit (confirm during build)"),
            ]),
            Line::from(vec![
                key("  Ctrl+C          "),
                desc("Force quit immediately"),
            ]),
            Line::from(vec![key("  ?               "), desc("Close this help")]),
        ];
        frame.render_widget(
            Paragraph::new(lines)
                .block(
                    Block::default()
                        .title(" Key Bindings ")
                        .borders(Borders::ALL)
                        .border_style(
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD)
                        )
                )
                .style(Style::default().fg(Color::White)),
            modal_rect
        );
    }
}
