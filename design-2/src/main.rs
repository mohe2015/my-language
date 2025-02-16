use std::{error::Error, io};

use ratatui::{
    backend::{Backend, CrosstermBackend}, crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    }, style::{Color, Style}, text::{Line, Span}, Frame, Terminal
};

pub enum AST {
    Integer(u64),
    Double(f64),
    Add(Vec<AST>),
    Multiply(Vec<AST>)
}

pub enum CursorASTInner {
    Integer(u64),
    Double(f64),
    Add(Vec<CursorAST>),
    Multiply(Vec<CursorAST>)
}

pub struct CursorAST {
    cursor: bool,
    inner: CursorASTInner
}

impl CursorAST {
    pub fn render(&self) -> Vec<Span> {
        let style = if self.cursor {
            Style::new().fg(Color::Black).bg(Color::White)
        } else {
            Style::new().fg(Color::White).bg(Color::Black)
        };
        match &self.inner {
            CursorASTInner::Integer(value) => vec![Span::styled(value.to_string(), style)],
            CursorASTInner::Double(value) => vec![Span::styled(value.to_string(), style)],
            CursorASTInner::Add(asts) => std::iter::once(Span::styled("(+ ", style)).chain(asts.iter().flat_map(|a| a.render())).chain(std::iter::once(Span::styled(")", style))).collect(),
            CursorASTInner::Multiply(asts) => std::iter::once(Span::styled("(* ", style)).chain(asts.iter().flat_map(|a| a.render())).chain(std::iter::once(Span::styled(")", style))).collect(),
        }
    }
}

pub struct App {
    ast: CursorAST
}

impl App {

    fn run_app<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> io::Result<bool> {
        loop {
            terminal.draw(|f| ui(f))?;
    
            if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Release {
                    // Skip events that are not KeyEventKind::Press
                    continue;
                }

                if key.code == KeyCode::Left {

                }
            }
        }
    }
    
    pub fn ui(&self, frame: &mut Frame) {
        frame.render_widget(Line::from(self.ast.render()), frame.area());
    }
}

// store the code as structured data in a file and have a custom editor to edit this code
// https://docs.rs/ratatui-core/0.1.0-alpha.2/ratatui_core/text/struct.Span.html

fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stderr = io::stderr();
    execute!(stderr, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stderr);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App {
        ast: CursorAST {
            cursor: false,
            inner: CursorASTInner::Add(vec![CursorAST {
                cursor: true,
                inner: CursorASTInner::Integer(5)
            }])
        }
    };
    let res = app.run_app(&mut terminal);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
