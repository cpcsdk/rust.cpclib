use std::io::{self, Stdout, stdout};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use cpclib_bndbuild::app::{BndBuilderApp, BndBuilderCommand};
use cpclib_bndbuild::cpclib_common::event::EventObserver;
use cpclib_runner::kill_all_children;
use cpclib_bndbuild::event::{BndBuilderEvent, BndBuilderObserved, BndBuilderObserver, BndBuilderObserverRc};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{self, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode
};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::prelude::{Backend, CrosstermBackend};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget};
use ratatui::{Frame, Terminal};

use crate::ratatui_event::{RatatuiEvent, RatatuiState};

mod ratatui_event;

// ─── Task ─────────────────────────────────────────────────────────────────────

/// Per-task stdout/stderr line cap.  Oldest lines are dropped when exceeded.
const MAX_LINES_PER_TASK: usize = 2000;

#[derive(Debug)]
enum TaskStatus {
    Running,
    Success(Duration),
    Failed(Duration),
}

#[derive(Debug)]
struct TaskEntry {
    task:    String,
    started: Instant,
    stdout:  Vec<String>,
    stderr:  Vec<String>,
    status:  TaskStatus,
}

impl TaskEntry {
    fn new(task: String) -> Self {
        Self {
            task,
            started: Instant::now(),
            stdout:  Vec::new(),
            stderr:  Vec::new(),
            status:  TaskStatus::Running,
        }
    }

    fn is_running(&self) -> bool {
        matches!(self.status, TaskStatus::Running)
    }

    /// Rows this task takes when rendered inline inside a rule widget (non-selected).
    fn inline_height(&self) -> u16 {
        match self.status {
            TaskStatus::Running => 1 + (self.stdout.len() + self.stderr.len()).min(8) as u16,
            _ => 1,
        }
    }

    /// Rows this task needs to display ALL output (used in selected/expanded running view).
    fn full_height(&self) -> u16 {
        match self.status {
            TaskStatus::Running => 1 + (self.stdout.len() + self.stderr.len()) as u16,
            _ => 1,
        }
    }
}

// ─── Rule ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
enum RuleStatus {
    Running,
    Success(Duration),
    Failed(Duration),
}

#[derive(Debug)]
struct RuleEntry {
    name:    String,
    /// Co-target names that share the same rule (populated before this entry is created).
    aliases: Vec<String>,
    nb:      usize,
    out_of:  usize,
    started: Instant,
    tasks:   Vec<TaskEntry>,
    status:  RuleStatus,
    /// Lines to skip from the auto-follow bottom (Up key increases, Down key decreases).
    task_scroll: usize,
    /// Chars to skip from the left edge in the expanded output view (Left/Right keys).
    h_scroll:    usize,
}

impl RuleEntry {
    fn new(name: String, nb: usize, out_of: usize) -> Self {
        Self {
            name,
            aliases: Vec::new(),
            nb,
            out_of,
            started: Instant::now(),
            tasks:       Vec::new(),
            status:      RuleStatus::Running,
            task_scroll: 0,
            h_scroll:    0,
        }
    }

    fn is_running(&self) -> bool {
        matches!(self.status, RuleStatus::Running)
    }

    /// Total rows this rule occupies when rendered.
    fn height(&self) -> u16 {
        match self.status {
            // Border (2) + one row per inline task, minimum 3.
            RuleStatus::Running => {
                let inner: u16 = self.tasks.iter().map(|t| t.inline_height()).sum();
                (2 + inner).max(3)
            },
            _ => 1,
        }
    }
}

// ─── Build phase ──────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
enum BuildPhase {
    #[default]
    Idle,
    ComputingDeps(String),
    Running { current: usize, total: usize },
    Finished,
}

// ─── App state ────────────────────────────────────────────────────────────────

struct BndBuilderRatatui {
    command: Option<BndBuilderCommand>,
    rx:      mpsc::Receiver<RatatuiMessage>,
    rules:   Vec<RuleEntry>,
    /// Tasks fired without a parent rule.
    orphans: Vec<TaskEntry>,
    phase:   BuildPhase,
    /// Entry-based scroll (rules + orphan-tasks). `None` = auto-follow.
    scroll:          Option<usize>,
    exit:            bool,
    /// True while the "confirm quit" modal is shown.
    confirm_quit:    bool,
    /// Aliases waiting to be attached to their representative's RuleEntry.
    /// Key = representative path string, value = list of alias path strings.
    pending_aliases: std::collections::HashMap<String, Vec<String>>,
    /// Index into `rules` of the currently selected rule (for manual task scrolling).
    selected_rule: Option<usize>,
}

// ─── Messages ─────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum RatatuiMessage {
    NewEvent(RatatuiEvent),
    Stdout(String),
    Stderr(String),
}

// ─── Observer ─────────────────────────────────────────────────────────────────

#[derive(Debug)]
struct BndBuilderRatatuiObserver {
    tx: mpsc::Sender<RatatuiMessage>,
}

impl BndBuilderRatatuiObserver {
    fn new(tx: mpsc::Sender<RatatuiMessage>) -> Self {
        Self { tx }
    }
}

impl EventObserver for BndBuilderRatatuiObserver {
    fn emit_stdout(&self, s: &str) {
        let _ = self.tx.send(RatatuiMessage::Stdout(s.to_owned()));
    }

    fn emit_stderr(&self, s: &str) {
        let _ = self.tx.send(RatatuiMessage::Stderr(s.to_owned()));
    }
}

impl BndBuilderObserver for BndBuilderRatatuiObserver {
    fn update(&mut self, event: BndBuilderEvent) {
        let _ = self.tx.send(RatatuiMessage::NewEvent(event.into()));
    }
}

// ─── App event logic ──────────────────────────────────────────────────────────

impl BndBuilderRatatui {
    fn running_rule_mut(&mut self, name: &str) -> Option<&mut RuleEntry> {
        self.rules.iter_mut().rev().find(|r| r.is_running() && r.name == name)
    }

    fn running_task_mut(&mut self, task_name: &str) -> Option<&mut TaskEntry> {
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
    fn any_task_mut(&mut self, task_name: &str) -> Option<&mut TaskEntry> {
        // Two-phase (find index, then take mutable ref) to satisfy the borrow checker.

        // Phase 1: running tasks in rules.
        let idx = self.rules.iter().enumerate().rev().find_map(|(ri, rule)| {
            rule.tasks.iter().enumerate().rev()
                .find(|(_, t)| t.is_running() && t.task == task_name)
                .map(|(ti, _)| (ri, ti))
        });
        if let Some((ri, ti)) = idx {
            return Some(&mut self.rules[ri].tasks[ti]);
        }

        // Phase 2: running orphans.
        let idx = self.orphans.iter().enumerate().rev()
            .find(|(_, t)| t.is_running() && t.task == task_name)
            .map(|(i, _)| i);
        if let Some(i) = idx {
            return Some(&mut self.orphans[i]);
        }

        // Phase 3: fallback — any task in rules (most recent).
        let idx = self.rules.iter().enumerate().rev().find_map(|(ri, rule)| {
            rule.tasks.iter().enumerate().rev()
                .find(|(_, t)| t.task == task_name)
                .map(|(ti, _)| (ri, ti))
        });
        if let Some((ri, ti)) = idx {
            return Some(&mut self.rules[ri].tasks[ti]);
        }

        // Phase 4: fallback — any orphan.
        let idx = self.orphans.iter().enumerate().rev()
            .find(|(_, t)| t.task == task_name)
            .map(|(i, _)| i);
        idx.map(|i| &mut self.orphans[i])
    }

    fn apply_event(&mut self, event: RatatuiEvent) {
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
                // Buffer this alias; when the representative's StartRule fires
                // it will pick these up and store them in RuleEntry::aliases.
                self.pending_aliases
                    .entry(representative)
                    .or_default()
                    .push(alias);
            },

            RatatuiEvent::StartRule { rule, nb, out_of } => {
                let aliases = self.pending_aliases.remove(&rule).unwrap_or_default();
                let mut entry = RuleEntry::new(rule, nb, out_of);
                entry.aliases = aliases;
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

            RatatuiEvent::FailedRule(rule) => {
                if let Some(r) = self
                    .rules
                    .iter_mut()
                    .rev()
                    .find(|r| r.is_running() && r.name == rule)
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

    fn handle_message(&mut self, msg: RatatuiMessage) {
        if let RatatuiMessage::NewEvent(ev) = msg {
            self.apply_event(ev);
        }
    }

    // ── Scroll helpers ────────────────────────────────────────────────────────

    fn total_entries(&self) -> usize {
        self.rules.len() + self.orphans.len()
    }

    /// How many entries to skip so that content from that entry onward
    /// fills `visible_rows` rows (or fills as much as possible).
    ///
    /// The bottom-most entry is always included even when it exceeds
    /// `visible_rows` (it gets clipped by the widget renderer).  Earlier
    /// entries are included only while they still fit.
    fn bottom_skip(&self, visible_rows: u16) -> usize {
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
                // Always include the last (bottom-most) entry, even if it is
                // taller than the visible area — it will be clipped.
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

    fn effective_skip(&self, list_h: u16) -> usize {
        match self.scroll {
            None => self.bottom_skip(list_h),
            Some(n) => n.min(self.total_entries()),
        }
    }

    // ── Run loop ──────────────────────────────────────────────────────────────

    fn run<T: Backend>(&mut self, mut terminal: Terminal<T>) -> io::Result<()> {
        let cmd = self.command.take().expect("command is required");
        assert!(cmd.is_build());

        let thread = std::thread::spawn(move || cmd.execute());

        // Ensure the screen starts clean — ratatui diffs against an empty
        // previous buffer, so without this the first frame may be invisible.
        terminal
            .clear()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        'main: loop {
            let list_h = terminal
                .size()
                .map(|s| s.height.saturating_sub(2))
                .unwrap_or(20);
            let skip = self.effective_skip(list_h);

            terminal
                .draw(|f| self.draw(f, skip))
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

            if event::poll(Duration::from_millis(16))? {
                if let event::Event::Key(k) = event::read()? {
                    if k.kind == KeyEventKind::Press {
                        // Ctrl+C: restore terminal and kill immediately.
                        if k.code == KeyCode::Char('c')
                            && k.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            kill_all_children();
                            drop(terminal);
                            restore_terminal().ok();
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
                                    None => if n > 0 { Some(0) } else { None },
                                    Some(i) => if i + 1 < n { Some(i + 1) } else { None },
                                };
                            },
                            KeyCode::BackTab => {
                                self.confirm_quit = false;
                                let n = self.rules.len();
                                self.selected_rule = match self.selected_rule {
                                    None => if n > 0 { Some(n - 1) } else { None },
                                    Some(0) => None,
                                    Some(i) => Some(i - 1),
                                };
                            },
                            KeyCode::Char('q') | KeyCode::Char('Q') => {
                                let build_done = matches!(self.phase, BuildPhase::Finished);
                                if build_done {
                                    // Build is done; thread should already have finished.
                                    self.exit = true;
                                    break 'main;
                                } else if self.confirm_quit {
                                    // User confirmed: force-quit while build is running.
                                    kill_all_children();
                                    drop(terminal);
                                    restore_terminal().ok();
                                    std::process::exit(1);
                                } else {
                                    // First Q while running: show the confirmation modal.
                                    self.confirm_quit = true;
                                }
                            },
                            KeyCode::Down | KeyCode::Char('j') => {
                                self.confirm_quit = false;
                                if let Some(idx) = self.selected_rule {
                                    if let Some(rule) = self.rules.get_mut(idx) {
                                        rule.task_scroll = rule.task_scroll.saturating_sub(1);
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
                                        rule.task_scroll = rule.task_scroll.saturating_add(1);
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
                }
            }

            while let Ok(msg) = self.rx.try_recv() {
                self.handle_message(msg);
            }
        }

        match thread.join() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(io::Error::new(io::ErrorKind::Other, e.to_string())),
            Err(_) => Err(io::Error::new(io::ErrorKind::Other, "build thread panicked")),
        }
    }

    // ── Drawing ──────────────────────────────────────────────────────────────

    fn draw(&self, frame: &mut Frame, skip: usize) {
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
            BuildPhase::Running { current, total } if *total > 0 => {
                format!("Building [{current}/{total}]")
            },
            BuildPhase::Running { .. } => "Building\u{2026}".to_owned(),
            BuildPhase::Finished => "Build finished".to_owned(),
        };
        frame.render_widget(
            Paragraph::new(header).style(Style::default().add_modifier(Modifier::BOLD)),
            chunks[0],
        );

        // Rule list
        frame.render_widget(
            RulesView { rules: &self.rules, orphans: &self.orphans, skip, selected_rule: self.selected_rule },
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

        let rn = |n: usize| if n == 1 { "rule".to_owned() } else { format!("rules") };
        let tn = |n: usize| if n == 1 { "task".to_owned() } else { format!("tasks") };

        let (status_text, status_style) = match &self.phase {
            BuildPhase::Finished => {
                if failed_rules > 0 {
                    (
                        format!(
                            "\u{2717} Build failed  \u{b7}  {done_rules} {} done  {failed_rules} {} failed  \u{b7}  q quit  tab select  \u{2191}\u{2193}/\u{2190}\u{2192}",
                            rn(done_rules), rn(failed_rules)
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

        frame.render_widget(
            Paragraph::new(status_text).style(status_style),
            chunks[2],
        );

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
                "Build is still running.\n\nPress Q again to quit  \u{b7}  Esc to continue"
            )
            .block(
                Block::default()
                    .title(" Confirm Quit ")
                    .borders(Borders::ALL)
                    .border_style(
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    ),
            )
            .style(Style::default().fg(Color::White)),
            modal_rect,
        );
    }
}

// ─── Rules list widget ──────────────────────────────────────────────────────

struct RulesView<'a> {
    rules:         &'a [RuleEntry],
    orphans:       &'a [TaskEntry],
    skip:          usize,
    selected_rule: Option<usize>,
}

impl<'a> Widget for RulesView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut y = area.y;
        let bottom = area.y + area.height;
        let mut remaining_skip = self.skip;

        for (idx, rule) in self.rules.iter().enumerate() {
            if remaining_skip > 0 {
                remaining_skip -= 1;
                continue;
            }
            if y >= bottom {
                break;
            }
            let is_selected = self.selected_rule == Some(idx);
            // A selected rule expands to fill all remaining space so the
            // user can read its full output.  Nothing renders below it.
            let h = if is_selected {
                bottom - y
            } else {
                rule.height().min(bottom - y)
            };
            RuleWidget { rule, selected: is_selected }.render(
                Rect { x: area.x, y, width: area.width, height: h },
                buf,
            );
            y += h;
        }
        for task in self.orphans {
            if remaining_skip > 0 {
                remaining_skip -= 1;
                continue;
            }
            if y >= bottom {
                break;
            }
            let h = task.inline_height().min(bottom - y);
            InlineTaskWidget::new(task).render(
                Rect { x: area.x, y, width: area.width, height: h },
                buf,
            );
            y += h;
        }
    }
}

// ─── Inline task widget ───────────────────────────────────────────────────────

struct InlineTaskWidget<'a> {
    task:     &'a TaskEntry,
    /// Horizontal character offset applied to all output lines.
    h_scroll: usize,
}

impl<'a> InlineTaskWidget<'a> {
    fn new(task: &'a TaskEntry) -> Self {
        Self { task, h_scroll: 0 }
    }
    fn with_h_scroll(task: &'a TaskEntry, h_scroll: usize) -> Self {
        Self { task, h_scroll }
    }
}

impl<'a> Widget for InlineTaskWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 {
            return;
        }
        let entry = self.task;
        let h_scroll = self.h_scroll;
        let (prefix, style) = match &entry.status {
            TaskStatus::Running => ("● ", Style::default().fg(Color::Yellow)),
            TaskStatus::Success(_) => ("✓ ", Style::default().fg(Color::Green)),
            TaskStatus::Failed(_) => (
                "✗ ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
        };
        let header = match &entry.status {
            TaskStatus::Running => {
                const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
                let frame = FRAMES[(entry.started.elapsed().as_millis() as usize / 100) % FRAMES.len()];
                format!("{frame} {}", entry.task)
            },
            TaskStatus::Success(d) | TaskStatus::Failed(d) => {
                format!("{prefix} {:6.2}s  {}", d.as_secs_f64(), entry.task)
            },
        };
        // Apply h_scroll to header line too.
        let header_chars: Vec<char> = header.chars().collect();
        let header_display: String = header_chars[h_scroll.min(header_chars.len())..].iter().collect();
        Paragraph::new(header_display)
            .style(style)
            .render(Rect { height: 1, ..area }, buf);

        if matches!(entry.status, TaskStatus::Running) && area.height > 1 {
            let out_area = Rect { y: area.y + 1, height: area.height - 1, ..area };
            let all_lines: Vec<(&str, Style)> = entry
                .stderr
                .iter()
                .map(|s| (s.as_str(), Style::default().fg(Color::Red)))
                .chain(
                    entry
                        .stdout
                        .iter()
                        .map(|s| (s.as_str(), Style::default())),
                )
                .collect();
            let start = all_lines.len().saturating_sub(out_area.height as usize);
            let w = out_area.width as usize;
            for (i, (text, sty)) in all_lines[start..].iter().enumerate() {
                if i as u16 >= out_area.height {
                    break;
                }
                let line = format!("  {text}");
                let chars: Vec<char> = line.chars().collect();
                let from = h_scroll.min(chars.len());
                let visible: String = chars[from..].iter().take(w).collect();
                Paragraph::new(visible)
                    .style(*sty)
                    .render(Rect { y: out_area.y + i as u16, height: 1, ..out_area }, buf);
            }
        }
    }
}

// ─── Rule widget ──────────────────────────────────────────────────────────────

struct RuleWidget<'a> {
    rule:     &'a RuleEntry,
    selected: bool,
}

impl<'a> Widget for RuleWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 {
            return;
        }
        let rule = self.rule;
        let selected = self.selected;
        // Build a full display name that includes any co-target aliases.
        let full_name: std::borrow::Cow<str> = if rule.aliases.is_empty() {
            std::borrow::Cow::Borrowed(rule.name.as_str())
        } else {
            let parts = std::iter::once(rule.name.as_str())
                .chain(rule.aliases.iter().map(|s| s.as_str()))
                .collect::<Vec<_>>()
                .join(" ");
            std::borrow::Cow::Owned(parts)
        };
        let elapsed_ms = rule.started.elapsed().as_millis() as u64;
        match &rule.status {
            RuleStatus::Running => {
                let counter = if rule.out_of > 0 {
                    format!("  [{}/{}]", rule.nb, rule.out_of)
                } else {
                    String::new()
                };

                // Title bar has area.width columns; border chars take 1 each side.
                let title_bar_w = area.width.saturating_sub(2) as usize;
                let prefix_w = 2usize; // "⟳ "
                let counter_w = counter.chars().count();
                let names_w = full_name.chars().count();

                let title = if prefix_w + names_w + counter_w <= title_bar_w
                    || area.width < 8
                {
                    // Fits without scrolling.
                    if counter.is_empty() {
                        Line::from(vec![
                            Span::styled("⟳ ", Style::default().fg(Color::Yellow)),
                            Span::raw(full_name.as_ref().to_owned()),
                        ])
                    } else {
                        Line::from(vec![
                            Span::styled("⟳ ", Style::default().fg(Color::Yellow)),
                            Span::raw(full_name.as_ref().to_owned()),
                            Span::styled(counter.clone(), Style::default().fg(Color::DarkGray)),
                        ])
                    }
                } else {
                    // Too long: marquee-scroll the names from right to left.
                    // Cap names area at 2/3 of title bar width.
                    let names_avail = (title_bar_w * 2 / 3)
                        .min(title_bar_w.saturating_sub(prefix_w + counter_w));
                    let scrolled = marquee_window(&full_name, elapsed_ms, names_avail);
                    Line::from(vec![
                        Span::styled("⟳ ", Style::default().fg(Color::Yellow)),
                        Span::raw(scrolled),
                        Span::styled(counter.clone(), Style::default().fg(Color::DarkGray)),
                    ])
                };
                let border_color = if selected { Color::Cyan } else { Color::Yellow };
                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color))
                    .title(title);
                let inner = block.inner(area);
                block.render(area, buf);

                // Auto-scroll: show the most-recent (last) tasks when they
                // don't all fit.  Walk backwards to count how many fit, then
                // render from auto_start onward.
                // When selected, use full (uncapped) per-task height so all
                // output lines are reachable via task_scroll.
                let height_fn = |t: &TaskEntry| -> u16 {
                    if selected { t.full_height() } else { t.inline_height() }
                };
                let auto_start = {
                    let mut rem = inner.height;
                    let mut fit = 0usize;
                    for t in rule.tasks.iter().rev() {
                        let h = height_fn(t);
                        if fit == 0 {
                            fit += 1;
                            rem = rem.saturating_sub(h);
                        } else if h <= rem {
                            fit += 1;
                            rem -= h;
                        } else {
                            break;
                        }
                    }
                    rule.tasks.len().saturating_sub(fit)
                };
                // When selected, allow scrolling back from the auto-follow position.
                let task_start = if selected {
                    auto_start.saturating_sub(rule.task_scroll)
                } else {
                    auto_start
                };

                let mut y = inner.y;
                for task in &rule.tasks[task_start..] {
                    if y >= inner.y + inner.height {
                        break;
                    }
                    let avail = inner.y + inner.height - y;
                    let h = height_fn(task).min(avail);
                    let widget = if selected {
                        InlineTaskWidget::with_h_scroll(task, rule.h_scroll)
                    } else {
                        InlineTaskWidget::new(task)
                    };
                    widget.render(Rect { y, height: h, ..inner }, buf);
                    y += h;
                }
            },

            RuleStatus::Success(dur) | RuleStatus::Failed(dur) => {
                let is_success = matches!(&rule.status, RuleStatus::Success(_));
                if selected {
                    // ── Expanded detail view ─────────────────────────────────────────
                    let (icon, title_style) = if is_success {
                        ("✓", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
                    } else {
                        ("✗", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
                    };
                    let title_str = format!("{icon} {:6.2}s  {}  ", dur.as_secs_f64(), full_name.as_ref());
                    let block = Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Cyan))
                        .title(Line::from(vec![
                            Span::styled(title_str, title_style),
                            Span::styled(
                                "esc/tab · ↑↓ scroll · ←→ h-scroll",
                                Style::default().fg(Color::DarkGray),
                            ),
                        ]));
                    let inner = block.inner(area);
                    block.render(area, buf);

                    // Collect all task output lines: (text, Style)
                    let mut all_lines: Vec<(String, Style)> = Vec::new();
                    for task in &rule.tasks {
                        let (t_icon, t_style) = match &task.status {
                            TaskStatus::Running => ("⠿", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                            TaskStatus::Success(_) => ("✓", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                            TaskStatus::Failed(_) => ("✗", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                        };
                        let d_str = match &task.status {
                            TaskStatus::Running => "  ??.??s".to_owned(),
                            TaskStatus::Success(d) | TaskStatus::Failed(d) => {
                                format!("  {:6.2}s", d.as_secs_f64())
                            },
                        };
                        all_lines.push((format!("{t_icon}{d_str}  {}", task.task), t_style));
                        for s in &task.stderr {
                            all_lines.push((format!("  {s}"), Style::default().fg(Color::Red)));
                        }
                        for s in &task.stdout {
                            all_lines.push((format!("  {s}"), Style::default()));
                        }
                    }

                    // Vertical: task_scroll=0 → bottom (newest); larger → older lines
                    let total = all_lines.len();
                    let visible_h = inner.height as usize;
                    let auto_start = total.saturating_sub(visible_h);
                    let v_start = auto_start.saturating_sub(rule.task_scroll);
                    let end = (v_start + visible_h).min(total);

                    let h_off = rule.h_scroll;
                    let w = inner.width as usize;
                    for (row, (text, style)) in all_lines[v_start..end].iter().enumerate() {
                        if row >= visible_h {
                            break;
                        }
                        let chars: Vec<char> = text.chars().collect();
                        let from = h_off.min(chars.len());
                        let visible_str: String = chars[from..].iter().take(w).collect();
                        Paragraph::new(visible_str)
                            .style(*style)
                            .render(Rect { y: inner.y + row as u16, height: 1, ..inner }, buf);
                    }
                } else {
                    // ── Compact 1-line view ───────────────────────────────────────────
                    if is_success {
                        let prefix = format!("✓  {:6.2}s  ", dur.as_secs_f64());
                        let prefix_w = prefix.chars().count();
                        let names_avail = (area.width as usize * 2 / 3)
                            .min((area.width as usize).saturating_sub(prefix_w));
                        let name_text = marquee_window(&full_name, elapsed_ms, names_avail);
                        Paragraph::new(Line::from(vec![
                            Span::raw(prefix),
                            Span::raw(name_text),
                        ]))
                        .style(Style::default().fg(Color::Green))
                        .render(area, buf);
                    } else {
                        let prefix = format!("✗  {:6.2}s  ", dur.as_secs_f64());
                        let suffix = "  [FAILED]";
                        let prefix_w = prefix.chars().count();
                        let suffix_w = suffix.chars().count();
                        let names_avail = (area.width as usize * 2 / 3)
                            .min((area.width as usize).saturating_sub(prefix_w + suffix_w));
                        let name_text = marquee_window(&full_name, elapsed_ms, names_avail);
                        Paragraph::new(Line::from(vec![
                            Span::raw(prefix),
                            Span::raw(name_text),
                            Span::raw(suffix),
                        ]))
                        .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
                        .render(area, buf);
                    }
                }
            },
        }
    }
}

// ─── Marquee helper ─────────────────────────────────────────────────────────────

/// Return an `avail`-wide window of `names` scrolling left over time.
/// If `names` fits it is returned as-is, right-padded with spaces.
fn marquee_window(names: &str, elapsed_ms: u64, avail: usize) -> String {
    if avail == 0 {
        return String::new();
    }
    let chars: Vec<char> = names.chars().collect();
    if chars.len() <= avail {
        let mut s = names.to_owned();
        s.extend(std::iter::repeat(' ').take(avail - chars.len()));
        return s;
    }
    // Pad the cycle with a visible separator so wrap-around is clear.
    let sep: Vec<char> = "  ·  ".chars().collect();
    let padded: Vec<char> = chars.iter().chain(sep.iter()).copied().collect();
    let cycle = padded.len();
    let offset = (elapsed_ms as usize / 80) % cycle;
    (0..avail).map(|i| padded[(offset + i) % cycle]).collect()
}

// ─── ANSI helpers ─────────────────────────────────────────────────────────────

/// Strip CSI escape sequences (`ESC [ … final-byte`) from a string so that
/// raw assembler diagnostics don't corrupt ratatui's cell buffer.
fn strip_ansi_codes(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // CSI: ESC '[' <params> <final-byte in 0x40–0x7e>
            if chars.next() == Some('[') {
                for inner in chars.by_ref() {
                    if ('\x40'..='\x7e').contains(&inner) {
                        break;
                    }
                }
            }
            // Non-CSI ESC sequences: just drop the ESC itself; leave the rest.
        } else {
            out.push(c);
        }
    }
    out
}

// ─── Terminal helpers ─────────────────────────────────────────────────────────

fn init_terminal() -> io::Result<Terminal<CrosstermBackend<Stdout>>> {
    execute!(stdout(), EnterAlternateScreen)?;
    enable_raw_mode()?;
    Terminal::new(CrosstermBackend::new(stdout()))
}

fn restore_terminal() -> io::Result<()> {
    execute!(stdout(), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}

// ─── main ─────────────────────────────────────────────────────────────────────

fn main() {
    let app = match BndBuilderApp::new() {
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        },
        Ok(None) => return, // help / version already printed
        Ok(Some(a)) => a,
    };

    let (tx, rx) = mpsc::channel();

    let mut cmd = match app.command() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        },
    };
    // Drop app so its Arc<observers> clone is released, allowing add_observer to
    // get exclusive mutable access via Arc::get_mut.
    drop(app);

    if !cmd.is_build() {
        // For non-build commands (list, show, dot, …) bypass the TUI.
        cmd.clear_observers();
        cmd.add_observer(BndBuilderObserverRc::new_default());
        if let Err(e) = cmd.execute() {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
        return;
    }

    // Replace any existing observers with the ratatui channel observer.
    cmd.clear_observers();
    cmd.add_observer(BndBuilderObserverRc::new(BndBuilderRatatuiObserver::new(tx)));

    let terminal = match init_terminal() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to initialise terminal: {e}");
            std::process::exit(1);
        },
    };

    let mut state = BndBuilderRatatui {
        command:         Some(cmd),
        rx,
        rules:           Vec::new(),
        orphans:         Vec::new(),
        phase:           BuildPhase::default(),
        scroll:          None,
        exit:            false,
        confirm_quit:    false,
        pending_aliases: std::collections::HashMap::new(),
        selected_rule:   None,
    };

    let result = state.run(terminal);
    restore_terminal().unwrap();

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
