use std::io::{self, stdout, Stdout};
use std::thread::Thread;

use cpclib_bndbuild::app::{BndBuilderApp, BndBuilderCommand};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{self, KeyCode, KeyEventKind};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen
};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::prelude::{Backend, CrosstermBackend};
use ratatui::style::Stylize;
use ratatui::widgets::{Paragraph, Widget};
use ratatui::{Frame, Terminal};

struct BndBuilderRatatui {
    current_command: Option<BndBuilderCommand>,
    current_thread: Option<Thread>,
    exit: bool
}

impl BndBuilderRatatui {
    fn run<T>(&mut self, mut terminal: Terminal<T>) -> io::Result<()>
    where T: Backend {
        if let Some(command) = self.current_command.take() {
            assert!(command.is_build());

            while !self.exit {
                // draw the application
                terminal.draw(|frame| self.render_frame(frame))?;

                // handles the event.
                // ATM only q allow so eagerly exit the app
                self.handle_events()?;
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
        match event::read()? {
            event::Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                let (kind, code) = (key_event.kind, key_event.code);

                if kind == KeyEventKind::Press && code == KeyCode::Char('q') {
                    self.exit = true;
                }
            },
            _ => {}
        };
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
    let cmd = app.command().unwrap();
    let terminal = init().unwrap();

    let mut app = BndBuilderRatatui {
        current_command: Some(cmd),
        current_thread: None,
        exit: false
    };
    let app_result = app.run(terminal);

    restore().unwrap();
    app_result.unwrap()
}
