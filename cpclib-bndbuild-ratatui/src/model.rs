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
    pub(crate) stdout:  Vec<String>,
    pub(crate) stderr:  Vec<String>,
    pub(crate) status:  TaskStatus,
}

impl TaskEntry {
    pub(crate) fn new(task: String) -> Self {
        Self {
            task,
            started: Instant::now(),
            stdout:  Vec::new(),
            stderr:  Vec::new(),
            status:  TaskStatus::Running,
        }
    }

    pub(crate) fn is_running(&self) -> bool {
        matches!(self.status, TaskStatus::Running)
    }

    /// Rows this task takes when rendered inline inside a rule widget (non-selected).
    pub(crate) fn inline_height(&self) -> u16 {
        match self.status {
            TaskStatus::Running => 1 + (self.stdout.len() + self.stderr.len()).min(8) as u16,
            // Failed tasks show up to 8 stderr lines inline so the user can see why.
            TaskStatus::Failed(_) => 1 + self.stderr.len().min(8) as u16,
            _ => 1,
        }
    }

    /// Rows this task needs to display ALL output (used in selected/expanded running view).
    pub(crate) fn full_height(&self) -> u16 {
        match self.status {
            TaskStatus::Running => 1 + (self.stdout.len() + self.stderr.len()) as u16,
            TaskStatus::Failed(_) => 1 + self.stderr.len() as u16,
            _ => 1,
        }
    }
}

// ─── Rule ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub(crate) enum RuleStatus {
    Running,
    Success(Duration),
    Failed(Duration),
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
}

impl RuleEntry {
    pub(crate) fn new(name: String, nb: usize, out_of: usize) -> Self {
        Self {
            name,
            aliases:     Vec::new(),
            nb,
            out_of,
            started:     Instant::now(),
            tasks:       Vec::new(),
            status:      RuleStatus::Running,
            task_scroll: 0,
            h_scroll:    0,
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
