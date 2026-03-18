use std::collections::HashMap;
use std::io;
use std::sync::mpsc;
use std::time::Duration;

use cpclib_bndbuild::app::BndBuilderCommand;
use cpclib_runner::kill_all_children;
use ratatui::Frame;
use ratatui::crossterm::event::{self, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::prelude::Backend;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget};
use ratatui::Terminal;

use crate::model::{BuildPhase, RuleEntry, RuleStatus, TaskEntry, TaskStatus};
use crate::observer::RatatuiMessage;
use crate::ratatui_event::{RatatuiEvent, RatatuiState};
use crate::widgets::{strip_ansi_codes, RulesView};

// ─── Constants ────────────────────────────────────────────────────────────────

const MAX_LINES_PER_TASK: usize = crate::model::MAX_LINES_PER_TASK;

// ─── App state ────────────────────────────────────────────────────────────────

pub(crate) struct BndBuilderRatatui {
    pub(crate) command: Option<BndBuilderCommand>,
    pub(crate) rx:      mpsc::Receiver<RatatuiMessage>,
    pub(crate) rules:   Vec<RuleEntry>,
    /// Tasks fired without a parent rule.
    pub(crate) orphans: Vec<TaskEntry>,
    pub(crate) phase:   BuildPhase,
    /// Entry-based scroll (rules + orphan-tasks). `None` = auto-follow.
    pub(crate) scroll:          Option<usize>,
    pub(crate) exit:            bool,
    /// True while the "confirm quit" modal is shown.
    pub(crate) confirm_quit:    bool,
    /// Aliases waiting to be attached to their representative's RuleEntry.
    /// Key = representative path string, value = list of alias path strings.
    pub(crate) pending_aliases: HashMap<String, Vec<String>>,
    /// Index into `rules` of the currently selected rule (for manual task scrolling).
    pub(crate) selected_rule: Option<usize>,
    /// Error message from the build thread, set when the build fails.
    pub(crate) build_error: Option<String>,
    /// Active build file for nested bndbuild invocations (tagged onto new rules).
    pub(crate) current_build_file: Option<String>,
}

impl BndBuilderRatatui {
    // ── Event routing ─────────────────────────────────────────────────────────

    pub(crate) fn running_rule_mut(&mut self, name: &str) -> Option<&mut RuleEntry> {
        self.rules.iter_mut().rev().find(|r| r.is_running() && r.name == name)
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

    pub(crate) fn apply_event(&mut self, event: RatatuiEvent) {
        match event {
            RatatuiEvent::ChangeState(state) => match state {
                RatatuiState::ComputeDependencies(p) => {
                    self.phase = BuildPhase::ComputingDeps(p)
                },
                RatatuiState::RunTasks => {
                    self.phase = BuildPhase::Running { current: 0, total: 0 }
                },
                RatatuiState::Finish => self.phase = BuildPhase::Finished,
            },

            RatatuiEvent::StartRuleAlias { alias, representative, .. } => {
                self.pending_aliases.entry(representative).or_default().push(alias);
            },

            RatatuiEvent::StartRule { rule, nb, out_of } => {
                let aliases = self.pending_aliases.remove(&rule).unwrap_or_default();
                let mut entry = RuleEntry::new(rule, nb, out_of);
                entry.aliases = aliases;
                entry.source = self.current_build_file.clone();
                self.rules.push(entry);
                self.phase = BuildPhase::Running { current: nb, total: out_of };
                self.scroll = None; // auto-follow
            },

            RatatuiEvent::StopRule(rule) => {
                if let Some(r) = self.running_rule_mut(&rule) {
                    let d = r.started.elapsed();
                    r.status = RuleStatus::Success(d);
                }
            },

            RatatuiEvent::SkippedRule(rule) => {
                if let Some(r) = self.running_rule_mut(&rule) {
                    r.status = RuleStatus::UpToDate;
                }
            },

            RatatuiEvent::BuildFileContext(ctx) => {
                self.current_build_file = ctx;
            },

            RatatuiEvent::FailedRule(rule) => {
                if let Some(r) =
                    self.rules.iter_mut().rev().find(|r| r.is_running() && r.name == rule)
                {
                    let d = r.started.elapsed();
                    for t in r.tasks.iter_mut() {
                        if t.is_running() {
                            t.status = TaskStatus::Failed(t.started.elapsed());
                        }
                    }
                    r.status = RuleStatus::Failed(d);
                }
            },

            RatatuiEvent::StartTask { rule: Some(rule_name), task } => {
                if let Some(r) = self.running_rule_mut(&rule_name) {
                    r.tasks.push(TaskEntry::new(task));
                } else {
                    self.orphans.push(TaskEntry::new(task));
                }
            },

            RatatuiEvent::StartTask { rule: None, task } => {
                self.orphans.push(TaskEntry::new(task));
            },

            RatatuiEvent::StopTask { task, duration, .. } => {
                if let Some(t) = self.running_task_mut(&task) {
                    t.status = TaskStatus::Success(duration);
                }
            },

            RatatuiEvent::TaskStdout { task, output, .. } => {
                if let Some(t) = self.any_task_mut(&task) {
                    for line in output.lines() {
                        let clean = strip_ansi_codes(line);
                        let clean = clean.trim_end_matches('\r');
                        if !clean.is_empty() {
                            if t.stdout.len() >= MAX_LINES_PER_TASK {
                                t.stdout.remove(0);
                            }
                            t.stdout.push(clean.to_owned());
                        }
                    }
                }
            },

            RatatuiEvent::TaskStderr { task, output, .. } => {
                if let Some(t) = self.any_task_mut(&task) {
                    for line in output.lines() {
                        let clean = strip_ansi_codes(line);
                        let clean = clean.trim_end_matches('\r');
                        if !clean.is_empty() {
                            if t.stderr.len() >= MAX_LINES_PER_TASK {
                                t.stderr.remove(0);
                            }
                            t.stderr.push(clean.to_owned());
                        }
                    }
                }
            },

            RatatuiEvent::Stdout(_) | RatatuiEvent::Stderr(_) => {},
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
        let heights: Vec<u16> = self
            .rules
            .iter()
            .map(|r| r.height())
            .chain(self.orphans.iter().map(|t| t.inline_height()))
            .collect();

        if heights.is_empty() {
            return 0;
        }

        let mut remaining = visible_rows;
        let mut visible = 0usize;
        for &h in heights.iter().rev() {
            if visible == 0 {
                visible += 1;
                remaining = remaining.saturating_sub(h);
            } else if h <= remaining {
                visible += 1;
                remaining -= h;
            } else {
                break;
            }
        }
        heights.len().saturating_sub(visible)
    }

    pub(crate) fn effective_skip(&self, list_h: u16) -> usize {
        match self.scroll {
            None => self.bottom_skip(list_h),
            Some(n) => n.min(self.total_entries()),
        }
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
                if thread.as_ref().map_or(false, |t| t.is_finished()) {
                    let handle = thread.take().unwrap();
                    match handle.join() {
                        Ok(res) => thread_result = Some(res),
                        Err(_) => {
                            thread_result = Some(Err(cpclib_bndbuild::BndBuilderError::AnyError(
                                "build thread panicked".into(),
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
                                        task.status =
                                            TaskStatus::Failed(task.started.elapsed());
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
                                    self.selected_rule = match self.selected_rule {
                                        None => {
                                            if n > 0 { Some(0) } else { None }
                                        },
                                        Some(i) => {
                                            if i + 1 < n { Some(i + 1) } else { None }
                                        },
                                    };
                                },
                                KeyCode::BackTab => {
                                    self.confirm_quit = false;
                                    let n = self.rules.len();
                                    self.selected_rule = match self.selected_rule {
                                        None => {
                                            if n > 0 { Some(n - 1) } else { None }
                                        },
                                        Some(0) => None,
                                        Some(i) => Some(i - 1),
                                    };
                                },
                                KeyCode::Char('q') | KeyCode::Char('Q') => {
                                    let build_done = matches!(self.phase, BuildPhase::Finished);
                                    if build_done {
                                        self.exit = true;
                                        break 'main;
                                    } else if self.confirm_quit {
                                        kill_all_children();
                                        drop(terminal);
                                        crate::terminal::restore_terminal().ok();
                                        std::process::exit(1);
                                    } else {
                                        self.confirm_quit = true;
                                    }
                                },
                                KeyCode::Down | KeyCode::Char('j') => {
                                    self.confirm_quit = false;
                                    if let Some(idx) = self.selected_rule {
                                        if let Some(rule) = self.rules.get_mut(idx) {
                                            rule.task_scroll =
                                                rule.task_scroll.saturating_sub(1);
                                        }
                                    } else {
                                        let next = skip.saturating_add(1);
                                        self.scroll = Some(next.min(self.total_entries()));
                                    }
                                },
                                KeyCode::Up | KeyCode::Char('k') => {
                                    self.confirm_quit = false;
                                    if let Some(idx) = self.selected_rule {
                                        if let Some(rule) = self.rules.get_mut(idx) {
                                            rule.task_scroll =
                                                rule.task_scroll.saturating_add(1);
                                        }
                                    } else {
                                        self.scroll = Some(skip.saturating_sub(1));
                                    }
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
                                _ => {},
                            }
                        }
                    },
                    event::Event::Mouse(mouse_ev) => {
                        if mouse_ev.kind == MouseEventKind::Down(MouseButton::Left) {
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
                                    } else {
                                        Some(idx)
                                    };
                                }
                            }
                        }
                    },
                    _ => {},
                }
            }

            while let Ok(msg) = self.rx.try_recv() {
                self.handle_message(msg);
            }
        }

        match thread_result {
            Some(Ok(())) => Ok(()),
            Some(Err(e)) => Err(io::Error::new(io::ErrorKind::Other, e.to_string())),
            None => {
                // Thread was still running when the user quit (q during build).
                Ok(())
            },
        }
    }

    // ── Drawing ───────────────────────────────────────────────────────────────

    pub(crate) fn draw(&self, frame: &mut Frame, skip: usize) {
        let area = frame.area();
        let chunks = Layout::vertical([
            Constraint::Length(1), // header
            Constraint::Fill(1),   // rule list
            Constraint::Length(1), // status bar
        ])
        .split(area);

        // Header
        let header = match &self.phase {
            BuildPhase::Idle => "bndbuild".to_owned(),
            BuildPhase::ComputingDeps(p) => format!("Computing dependencies: {p}"),
            BuildPhase::Running { .. } => {
                let global_current = self.rules.len();
                let global_total: usize = self
                    .rules
                    .iter()
                    .filter(|r| r.nb == 1)
                    .map(|r| r.out_of)
                    .sum();
                if global_total > 0 {
                    format!("Building [{global_current}/{global_total}]")
                } else {
                    "Building\u{2026}".to_owned()
                }
            },
            BuildPhase::Finished => {
                if let Some(err) = &self.build_error {
                    let short = if err.len() > 100 {
                        format!("\u{2717} Build FAILED: {}…", &err[..97])
                    } else {
                        format!("\u{2717} Build FAILED: {err}")
                    };
                    short
                } else {
                    "Build finished".to_owned()
                }
            },
        };
        let header_style = if matches!(&self.phase, BuildPhase::Finished)
            && self.build_error.is_some()
        {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        } else {
            Style::default().add_modifier(Modifier::BOLD)
        };
        frame.render_widget(Paragraph::new(header).style(header_style), chunks[0]);

        // Rule list
        frame.render_widget(
            RulesView {
                rules:         &self.rules,
                orphans:       &self.orphans,
                skip,
                selected_rule: self.selected_rule,
            },
            chunks[1],
        );

        // Status bar
        let running_rules = self.rules.iter().filter(|r| r.is_running()).count();
        let done_rules = self
            .rules
            .iter()
            .filter(|r| matches!(r.status, RuleStatus::Success(_)))
            .count();
        let failed_rules = self
            .rules
            .iter()
            .filter(|r| matches!(r.status, RuleStatus::Failed(_)))
            .count();
        let running_tasks: usize = self
            .rules
            .iter()
            .map(|r| r.tasks.iter().filter(|t| t.is_running()).count())
            .sum::<usize>()
            + self.orphans.iter().filter(|t| t.is_running()).count();

        let rn = |n: usize| if n == 1 { "rule".to_owned() } else { "rules".to_owned() };
        let tn = |n: usize| if n == 1 { "task".to_owned() } else { "tasks".to_owned() };

        let (status_text, status_style) = match &self.phase {
            BuildPhase::Finished => {
                if failed_rules > 0 {
                    (
                        format!(
                            "\u{2717} Build failed  \u{b7}  {done_rules} {} done  {failed_rules} {} failed  \u{b7}  q quit  tab select  \u{2191}\u{2193}/\u{2190}\u{2192}",
                            rn(done_rules),
                            rn(failed_rules)
                        ),
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    )
                } else {
                    (
                        format!(
                            "\u{2713} Build complete  \u{b7}  {done_rules} {}  \u{b7}  q quit  tab select  \u{2191}\u{2193}/\u{2190}\u{2192}",
                            rn(done_rules)
                        ),
                        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                    )
                }
            },
            _ => (
                format!(
                    "Rules: {running_rules} running  {done_rules} done  {failed_rules} failed \
                     \u{b7}  {running_tasks} {} active  \u{b7}  q quit  ^C force-quit  \u{2191}\u{2193}/jk scroll  tab select",
                    tn(running_tasks)
                ),
                Style::default().fg(Color::DarkGray),
            ),
        };
        frame.render_widget(Paragraph::new(status_text).style(status_style), chunks[2]);

        // Confirm-quit modal overlay
        if self.confirm_quit {
            self.draw_confirm_modal(frame);
        }
    }

    fn draw_confirm_modal(&self, frame: &mut Frame) {
        let area = frame.area();
        let modal_w = 54u16.min(area.width);
        let modal_h = 5u16.min(area.height);
        let modal_rect = Rect {
            x:      area.x + area.width.saturating_sub(modal_w) / 2,
            y:      area.y + area.height.saturating_sub(modal_h) / 2,
            width:  modal_w,
            height: modal_h,
        };
        frame.render_widget(Clear, modal_rect);
        frame.render_widget(
            Paragraph::new(
                "Build is still running.\n\nPress Q again to quit  \u{b7}  Esc to continue",
            )
            .block(
                Block::default()
                    .title(" Confirm Quit ")
                    .borders(Borders::ALL)
                    .border_style(
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    ),
            )
            .style(Style::default().fg(Color::White)),
            modal_rect,
        );
    }
}
