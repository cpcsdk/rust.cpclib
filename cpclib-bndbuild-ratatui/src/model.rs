use std::collections::VecDeque;
use std::time::{Duration, Instant};

// ─── Constants ────────────────────────────────────────────────────────────────

/// Per-task stdout/stderr line cap. Oldest lines are dropped when exceeded.
pub(crate) const MAX_LINES_PER_TASK: usize = 2000;

// ─── Task ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub(crate) enum TaskStatus {
    Running,
    Success(Duration),
    Failed(Duration),
}

#[derive(Debug)]
pub(crate) struct TaskEntry {
    pub(crate) task:    String,
    pub(crate) started: Instant,
    pub(crate) stdout:  VecDeque<String>,
    pub(crate) stderr:  VecDeque<String>,
    pub(crate) status:  TaskStatus,
    /// Historical average duration for this task, if cache data exists.
    pub(crate) estimated_duration: Option<Duration>,
    /// Rule name this task belongs to (needed to record timing in StopTask).
    pub(crate) parent_rule: Option<String>,
    /// Build file this task's rule came from (needed to key into the cache).
    pub(crate) parent_build_file: Option<String>,
}

impl TaskEntry {
    pub(crate) fn new(task: String) -> Self {
        Self {
            task,
            started:            Instant::now(),
            stdout:             VecDeque::new(),
            stderr:             VecDeque::new(),
            status:             TaskStatus::Running,
            estimated_duration: None,
            parent_rule:        None,
            parent_build_file:  None,
        }
    }

    pub(crate) fn is_running(&self) -> bool {
        matches!(self.status, TaskStatus::Running)
    }

    /// Rows this task takes when rendered inline inside a rule widget (non-selected).
    pub(crate) fn inline_height(&self) -> u16 {
        match self.status {
            TaskStatus::Running => 1 + (self.stdout.len() + self.stderr.len()).min(8) as u16,
            // Failed tasks show up to 8 stderr/stdout lines inline so the user can see why.
            // stdout is included because PTY-spawned processes route all output there.
            TaskStatus::Failed(_) => 1 + (self.stderr.len() + self.stdout.len()).min(8) as u16,
            // Success tasks show stdout if they produced any (e.g. emulator output).
            TaskStatus::Success(_) => {
                if self.stdout.is_empty() { 1 } else { 1 + self.stdout.len().min(8) as u16 }
            },
        }
    }

    /// Rows this task needs to display ALL output (used in selected/expanded running view).
    pub(crate) fn full_height(&self) -> u16 {
        match self.status {
            TaskStatus::Running => 1 + (self.stdout.len() + self.stderr.len()) as u16,
            TaskStatus::Failed(_) => 1 + (self.stderr.len() + self.stdout.len()) as u16,
            TaskStatus::Success(_) => 1 + self.stdout.len() as u16,
        }
    }
}

// ─── Rule ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub(crate) enum RuleStatus {
    Running,
    Success(Duration),
    Failed(Duration),
    /// Rule was skipped because all targets are already up to date.
    UpToDate,
}

#[derive(Debug)]
pub(crate) struct RuleEntry {
    pub(crate) name:    String,
    /// Co-target names that share the same rule (populated before this entry is created).
    pub(crate) aliases: Vec<String>,
    pub(crate) nb:      usize,
    pub(crate) out_of:  usize,
    pub(crate) started: Instant,
    pub(crate) tasks:   Vec<TaskEntry>,
    pub(crate) status:  RuleStatus,
    /// Lines to skip from the auto-follow bottom (Up key increases, Down key decreases).
    pub(crate) task_scroll: usize,
    /// Chars to skip from the left edge in the expanded output view (Left/Right keys).
    pub(crate) h_scroll:    usize,
    /// Source build file for rules coming from a nested bndbuild invocation.
    pub(crate) source:      Option<String>,
    /// Historical average duration for this rule, if cache data exists.
    pub(crate) estimated_duration: Option<Duration>,
    /// Instant of the last stdout/stderr output received from any task in this
    /// rule.  Used to briefly flash the border so the user's eye is drawn to
    /// active rules even when not selected.
    pub(crate) last_output: Option<Instant>,
}

impl RuleEntry {
    pub(crate) fn new(name: String, nb: usize, out_of: usize) -> Self {
        Self {
            name,
            aliases:            Vec::new(),
            nb,
            out_of,
            started:            Instant::now(),
            tasks:              Vec::new(),
            status:             RuleStatus::Running,
            task_scroll:        0,
            h_scroll:           0,
            source:             None,
            estimated_duration: None,
            last_output:        None,
        }
    }

    pub(crate) fn is_running(&self) -> bool {
        matches!(self.status, RuleStatus::Running)
    }

    /// Total rows this rule occupies when rendered.
    pub(crate) fn height(&self) -> u16 {
        match self.status {
            // Border (2) + one row per inline task, minimum 3.
            RuleStatus::Running => {
                let inner: u16 = self.tasks.iter().map(|t| t.inline_height()).sum();
                (2 + inner).max(3)
            },
            // Failed rules expand to show their tasks (with stderr) unless there are none.
            RuleStatus::Failed(_) => {
                let inner: u16 = self.tasks.iter().map(|t| t.inline_height()).sum();
                if inner > 0 { (2 + inner).max(3) } else { 1 }
            },
            // Success rules always show as compact 1-line. Any task stdout is
            // still visible when the user TABs to select (expand) the rule.
            RuleStatus::Success(_) => 1,
            // UpToDate: compact 1-line view.
            _ => 1,
        }
    }
}

// ─── Build phase ──────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub(crate) enum BuildPhase {
    #[default]
    Idle,
    ComputingDeps(String),
    Running { current: usize, total: usize },
    Finished,
}
