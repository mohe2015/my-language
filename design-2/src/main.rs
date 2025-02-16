use std::{error::Error, io};

use crossterm::event::KeyModifiers;
use ratatui::{
    Frame, Terminal,
    backend::{Backend, CrosstermBackend},
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    style::{Color, Style},
    text::{Line, Span},
};

pub enum ASTInner<T> {
    Integer(u64),
    Double(f64),
    Add(Vec<AST<T>>),
    Multiply(Vec<AST<T>>),
}

pub struct AST<T> {
    auxiliary: T,
    inner: ASTInner<T>,
}

impl AST<bool> {
    pub fn render(&self, highlight: bool) -> Vec<Span> {
        let style = if self.auxiliary || highlight {
            Style::new().fg(Color::Black).bg(Color::White)
        } else {
            Style::new().fg(Color::White)
        };
        match &self.inner {
            ASTInner::Integer(value) => vec![Span::styled(value.to_string(), style)],
            ASTInner::Double(value) => vec![Span::styled(value.to_string(), style)],
            ASTInner::Add(asts) => std::iter::once(Span::styled("(+", style))
                .chain(
                    asts.iter()
                        .flat_map(|a| std::iter::once(Span::styled(" ", style)).chain(a.render(self.auxiliary || highlight))),
                )
                .chain(std::iter::once(Span::styled(")", style)))
                .collect(),
            ASTInner::Multiply(asts) => std::iter::once(Span::styled("(*", style))
                .chain(
                    asts.iter()
                        .flat_map(|a| std::iter::once(Span::styled(" ", style)).chain(a.render(self.auxiliary || highlight))),
                )
                .chain(std::iter::once(Span::styled(")", style)))
                .collect(),
        }
    }

    pub fn left(&mut self) {
        match &mut self.inner {
            ASTInner::Integer(_) => {}
            ASTInner::Double(_) => {}
            ASTInner::Add(asts) | ASTInner::Multiply(asts) => {
                for i in 1..asts.len() {
                    if asts[i].auxiliary {
                        asts[i].auxiliary = false;
                        asts[i - 1].auxiliary = true;
                    }
                    asts[i].left();
                }
            }
        }
    }

    pub fn right(&mut self) {
        match &mut self.inner {
            ASTInner::Integer(_) => {}
            ASTInner::Double(_) => {}
            ASTInner::Add(asts) | ASTInner::Multiply(asts) => {
                for i in (0..asts.len() - 1).rev() {
                    if asts[i].auxiliary {
                        asts[i].auxiliary = false;
                        asts[i + 1].auxiliary = true;
                    }
                    asts[i].right();
                }
            }
        }
    }

    pub fn up(&mut self) {
        match &mut self.inner {
            ASTInner::Integer(_) => {}
            ASTInner::Double(_) => {}
            ASTInner::Add(asts) | ASTInner::Multiply(asts) => {
                for i in 0..asts.len() {
                    if asts[i].auxiliary {
                        asts[i].auxiliary = false;
                        self.auxiliary = true;
                    }
                    asts[i].up();
                }
            }
        }
    }

    pub fn down(&mut self) {
        match &mut self.inner {
            ASTInner::Integer(_) => {}
            ASTInner::Double(_) => {}
            ASTInner::Add(asts) | ASTInner::Multiply(asts) => {
                for i in 0..asts.len() {
                    asts[i].down();
                }
                if self.auxiliary {
                    if asts.len() > 0 {
                        self.auxiliary = false;
                        asts[0].auxiliary = true;
                    }
                }
            }
        }
    }

    pub fn delete_left(&mut self) {
        match &mut self.inner {
            ASTInner::Integer(_) => {}
            ASTInner::Double(_) => {}
            ASTInner::Multiply(asts) | ASTInner::Add(asts) => {
                let mut i = asts.len()-1;
                loop {
                    if asts[i].auxiliary {
                        asts.remove(i);
                        if i > 0 && asts.len() > 0 {
                            if !asts[i-1].auxiliary {
                                asts[i - 1].auxiliary = true;
                                i -= 1;
                            }
                        } else if i == 0 && asts.len() > 0 {
                            asts[0].auxiliary = true;
                        }
                    }
                    asts[i].delete_left();
                    if i == 0 {
                        break;
                    }
                    i -= 1;
                }
            }
        }
    }

    pub fn delete_right(&mut self) {
        match &mut self.inner {
            ASTInner::Integer(_) => {}
            ASTInner::Double(_) => {}
            ASTInner::Add(asts) | ASTInner::Multiply(asts) => {
                let mut i = asts.len()-1;
                loop {
                    if asts[i].auxiliary {
                        asts.remove(i);
                        if i < asts.len() {
                            asts[i].auxiliary = true;
                        } else if asts.len() > 0 {
                            let idx = asts.len() - 1;
                            if !asts[idx].auxiliary {
                                asts[idx].auxiliary = true;
                                i -= 1; // skip
                            }
                        }
                    }
                    asts[i].delete_right();
                    if i == 0 {
                        break;
                    }
                    i -= 1;
                }
            }
        }
    }

    pub fn insert(&mut self) {
        match &mut self.inner {
            ASTInner::Integer(_) => {}
            ASTInner::Double(_) => {}
            ASTInner::Add(asts) => {
                for i in (0..asts.len() - 1).rev() {
                    if asts[i].auxiliary {
                        asts[i].auxiliary = false;
                        asts[i + 1].auxiliary = true;
                    }
                    asts[i].insert();
                }
            }
            ASTInner::Multiply(asts) => {
                for i in (0..asts.len() - 1).rev() {
                    if asts[i].auxiliary {
                        asts[i].auxiliary = false;
                        asts[i + 1].auxiliary = true;
                    }
                    asts[i].insert();
                }
            }
        }
    }
}

pub struct App {
    ast: AST<bool>,
}

impl App {
    fn run_app<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Release {
                    // Skip events that are not KeyEventKind::Press
                    continue;
                }
                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(());
                    }
                    KeyCode::Left => {
                        self.ast.left();
                    }
                    KeyCode::Right => {
                        self.ast.right();
                    }
                    KeyCode::Up => {
                        self.ast.up();
                    }
                    KeyCode::Down => {
                        self.ast.down();
                    }
                    KeyCode::Backspace => {
                        self.ast.delete_left();
                    }
                    KeyCode::Delete => {
                        self.ast.delete_right();
                    }
                    KeyCode::Char('i') => {
                        self.ast.insert();
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn ui(&self, frame: &mut Frame) {
        frame.render_widget(Line::from(self.ast.render(false)), frame.area());
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
        ast: AST {
            auxiliary: false,
            inner: ASTInner::Add(vec![
                AST {
                    auxiliary: false,
                    inner: ASTInner::Integer(5),
                },
                AST {
                    auxiliary: false,
                    inner: ASTInner::Multiply(vec![
                        AST {
                            auxiliary: false,
                            inner: ASTInner::Integer(5),
                        },
                        AST {
                            auxiliary: true,
                            inner: ASTInner::Integer(7),
                        },
                    ])
                },
                AST {
                    auxiliary: false,
                    inner: ASTInner::Integer(7),
                },
            ]),
        },
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
