use cpclib_bndbuild::event::{BndBuilderEvent, BndBuilderState};

impl<'a> From<BndBuilderEvent<'a>> for RatatuiEvent {
    fn from(event: BndBuilderEvent<'a>) -> Self {
        match event {
            BndBuilderEvent::ChangeState(state) => {
                RatatuiEvent::ChangeState(match state {
                    BndBuilderState::ComputeDependencies(path) => {
                        RatatuiState::ComputeDependencies(path.to_string())
                    },
                    BndBuilderState::RunTasks => RatatuiState::RunTasks,
                    BndBuilderState::Finish => RatatuiState::Finish
                })
            },
            BndBuilderEvent::StartRule { rule, nb, out_of } => {
                RatatuiEvent::StartRule {
                    rule: rule.to_string(),
                    nb,
                    out_of
                }
            },
            BndBuilderEvent::StopRule(rule) => RatatuiEvent::StopRule(rule.to_string()),
            BndBuilderEvent::FailedRule(rule) => RatatuiEvent::FailedRule(rule.to_string()),
            BndBuilderEvent::StartTask(rule, task) => {
                RatatuiEvent::StartTask {
                    rule: rule.map(|r| r.to_string()),
                    task: format!("{}", task)
                }
            },
            BndBuilderEvent::StopTask(rule, task, duration) => {
                RatatuiEvent::StopTask {
                    rule: rule.map(|r| r.to_string()),
                    task: format!("{}", task),
                    duration
                }
            },
            BndBuilderEvent::TaskStdout(path, task, output) => {
                RatatuiEvent::TaskStdout {
                    path: path.to_string(),
                    task: format!("{}", task),
                    output: output.to_string()
                }
            },
            BndBuilderEvent::TaskStderr(path, task, output) => {
                RatatuiEvent::TaskStderr {
                    path: path.to_string(),
                    task: format!("{}", task),
                    output: output.to_string()
                }
            },
            BndBuilderEvent::Stdout(s) => RatatuiEvent::Stdout(s.to_string()),
            BndBuilderEvent::Stderr(s) => RatatuiEvent::Stderr(s.to_string())
        }
    }
}
#[derive(Clone, Debug)]
pub enum RatatuiEvent {
    ChangeState(RatatuiState),
    StartRule {
        rule: String,
        nb: usize,
        out_of: usize
    },
    StopRule(String),
    FailedRule(String),
    StartTask {
        rule: Option<String>,
        task: String
    },
    StopTask {
        rule: Option<String>,
        task: String,
        duration: std::time::Duration
    },
    TaskStdout {
        path: String,
        task: String,
        output: String
    },
    TaskStderr {
        path: String,
        task: String,
        output: String
    },
    Stdout(String),
    Stderr(String)
}

#[derive(Clone, Debug)]
pub enum RatatuiState {
    ComputeDependencies(String),
    RunTasks,
    Finish
}
