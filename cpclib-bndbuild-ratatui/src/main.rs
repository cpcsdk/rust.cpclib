use std::io::{self, Stdout, stdout};
use std::thread::Thread;

use anstyle::{AnsiColor, Style};
use cpclib_bndbuild::app::{BndBuilderApp, BndBuilderCommand};
use cpclib_bndbuild::cpclib_common::event::EventObserver;
use cpclib_bndbuild::event::{BndBuilderObserved, BndBuilderObserver, BndBuilderObserverRc};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{self, KeyCode, KeyEventKind};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode
};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::prelude::{Backend, CrosstermBackend};
use ratatui::style::Stylize;
use ratatui::widgets::{Paragraph, Widget};
use ratatui::{Frame, Terminal};

use crate::ratatui_event::{RatatuiEvent, RatatuiState};

mod ratatui_event;

struct BndBuilderRatatui {
    current_command: Option<BndBuilderCommand>,
    current_thread: Option<Thread>,
    rx: std::sync::mpsc::Receiver<RatatuiMessage>,
    exit: bool
}

#[derive(Debug)]
pub enum RatatuiMessage {
    NewEvent(RatatuiEvent),
    Stdout(String),
    Stderr(String)
}

#[derive(Debug)]
struct BndBuilderRatatuiObserver {
    tx: std::sync::mpsc::Sender<RatatuiMessage>
}

impl BndBuilderRatatuiObserver {
    pub fn new(tx: std::sync::mpsc::Sender<RatatuiMessage>) -> Self {
        Self { tx }
    }
}

impl EventObserver for BndBuilderRatatuiObserver {
    fn emit_stdout(&self, s: &str) {
        let _ = self.tx.send(RatatuiMessage::Stdout(s.to_string()));
    }

    fn emit_stderr(&self, s: &str) {
        let _ = self.tx.send(RatatuiMessage::Stderr(s.to_string()));
    }
}

impl BndBuilderObserver for BndBuilderRatatuiObserver {
    fn update(&mut self, event: cpclib_bndbuild::event::BndBuilderEvent) {
        todo!();
        let _ = self.tx.send(RatatuiMessage::NewEvent(event.into()));
    }
}

impl BndBuilderRatatui {
    fn run<T>(&mut self, mut terminal: Terminal<T>) -> io::Result<()>
    where T: Backend {
        if let Some(command) = self.current_command.take() {
            assert!(command.is_build());

            let thread_handle = std::thread::spawn(move || command.execute());

            while !self.exit {
                // draw the application
                terminal
                    .draw(|frame| self.render_frame(frame))
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{:?}", e)))?;

                // handles the event.
                // ATM only q allow so eagerly exit the app
                self.handle_events()?;
            }
            let thread_result = thread_handle.join();
            match thread_result {
                Ok(exec_result) => {
                    match exec_result {
                        Ok(_) => {},
                        Err(e) => {
                            let red_bold =
                                Style::new().fg_color(Some(AnsiColor::Red.into())).bold();
                            let red = Style::new().fg_color(Some(AnsiColor::Red.into()));
                            eprintln!(
                                "{}Error executing command:{}{} {}{}",
                                red_bold.render(),
                                red_bold.render_reset(),
                                red.render(),
                                e,
                                red.render_reset()
                            );
                            return Err(io::Error::new(
                                io::ErrorKind::Other,
                                "Error executing command"
                            ));
                        }
                    }
                },
                Err(_) => {
                    let red_bold = Style::new().fg_color(Some(AnsiColor::Red.into())).bold();
                    let red = Style::new().fg_color(Some(AnsiColor::Red.into()));
                    eprintln!(
                        "{}Error executing command:{}{} Command thread panicked{}",
                        red_bold.render(),
                        red_bold.render_reset(),
                        red.render(),
                        red.render_reset()
                    );
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        "Error executing command"
                    ));
                }
            }
        }
        else {
            unimplemented!()
        }

        Ok(())
    }

    fn render_frame(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self) -> io::Result<()> {
        // keyboard events
        match event::read()? {
            event::Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                let (kind, code) = (key_event.kind, key_event.code);

                if kind == KeyEventKind::Press && code == KeyCode::Char('q') {
                    self.exit = true;
                }
            },
            _ => {}
        };

        // bndbuild events
        if let Ok(message) = self.rx.try_recv() {
            dbg!(&message);
            todo!();
            match message {
                RatatuiMessage::NewEvent(event) => {
                    match event {
                        RatatuiEvent::Stdout(_) => {
                            // process event stdout
                        },
                        RatatuiEvent::Stderr(_) => {
                            // process event stderr
                        },
                        RatatuiEvent::ChangeState(state) => {
                            match state {
                                RatatuiState::ComputeDependencies(path) => {
                                    let _ = path; // silence unused warning
                                },
                                RatatuiState::RunTasks => {
                                    // process run tasks
                                },
                                RatatuiState::Finish => {
                                    self.exit = true;
                                }
                            }
                        },
                        RatatuiEvent::StartRule { rule, nb, out_of } => {
                            let _ = (rule, nb, out_of); // silence unused warnings
                        },
                        RatatuiEvent::StopRule(rule) => {
                            let _ = rule; // silence unused warning
                        },
                        RatatuiEvent::FailedRule(rule) => {
                            let _ = rule; // silence unused warning
                        },
                        RatatuiEvent::StartTask { rule, task } => {
                            let _ = (rule, task); // silence unused warnings
                        },
                        RatatuiEvent::StopTask {
                            rule,
                            task,
                            duration
                        } => {
                            let _ = (rule, task, duration); // silence unused warnings
                        },
                        RatatuiEvent::TaskStdout { path, task, output } => {
                            let _ = (path, task, output); // silence unused warnings
                        },
                        RatatuiEvent::TaskStderr { path, task, output } => {
                            let _ = (path, task, output); // silence unused warnings
                        }
                    }
                },
                RatatuiMessage::Stdout(s) => {
                    let _ = s; // process stdout or silence unused warning
                },
                RatatuiMessage::Stderr(s) => {
                    let _ = s; // process stderr or silence unused warning
                }
            }
        }

        Ok(())
    }
}

impl Widget for &BndBuilderRatatui {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let layout = Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        let (left, right) = (layout[0], layout[1]);

        Paragraph::new("Task list ".bold()).render(left, buf);

        Paragraph::new("Output list").render(right, buf);
    }
}

/// Initialize the terminal
pub fn init() -> io::Result<Terminal<CrosstermBackend<Stdout>>> {
    execute!(stdout(), EnterAlternateScreen)?;
    enable_raw_mode()?;
    Terminal::new(CrosstermBackend::new(stdout()))
}

/// Restore the terminal to its original state
pub fn restore() -> io::Result<()> {
    execute!(stdout(), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}

fn main() {
    // check first arguments before building the app

    let app = BndBuilderApp::new().unwrap().unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    let _observer = BndBuilderObserverRc::new(BndBuilderRatatuiObserver::new(tx));

    let mut cmd = match app.command() {
        Ok(c) => c,
        Err(e) => {
            let red_bold = Style::new().fg_color(Some(AnsiColor::Red.into())).bold();
            let red = Style::new().fg_color(Some(AnsiColor::Red.into()));
            eprintln!(
                "{}Error building command:{}{} {}{}",
                red_bold.render(),
                red_bold.render_reset(),
                red.render(),
                e,
                red.render_reset()
            );
            std::process::exit(1);
        }
    };

    // If the command is help, execute it and exit
    if !cmd.is_build() {
        cmd.clear_observers();
        cmd.add_observer(BndBuilderObserverRc::new_default());
        match cmd.execute() {
            Ok(_) => {
                std::process::exit(0);
            },
            Err(e) => {
                let red_bold = Style::new().fg_color(Some(AnsiColor::Red.into())).bold();
                let red = Style::new().fg_color(Some(AnsiColor::Red.into()));
                eprintln!(
                    "{}Error executing command:{}{} {}{}",
                    red_bold.render(),
                    red_bold.render_reset(),
                    red.render(),
                    e,
                    red.render_reset()
                );
                std::process::exit(1);
            }
        }
    }

    let terminal = init().unwrap();

    let mut app = BndBuilderRatatui {
        current_command: Some(cmd),
        current_thread: None,
        exit: false,
        rx
    };

    let app_result = app.run(terminal);

    restore().unwrap();
    app_result.unwrap()
}
