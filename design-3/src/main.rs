use std::{
    error::Error,
    io::{self, stdout},
    iter::{Product, Sum},
    ops::RangeInclusive,
    panic::{set_hook, take_hook},
};

use crossterm::event::KeyModifiers;
use ratatui::{
    Frame, Terminal,
    backend::{Backend, CrosstermBackend},
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    text::{Line, Span},
};

// TODO cursor that addresses single characters so you can properly insert numbers text etc.

pub enum AST {
    Integer(u64),
    Double(f64),
    Add(Vec<AST>),
    Multiply(Vec<AST>),
    Placeholder,
}
#[derive(Debug)]
pub enum Value {
    Integer(u64),
    Double(f64),
}

impl Sum for Value {
    fn sum<I: Iterator<Item = Self>>(mut iter: I) -> Self {
        let first = iter.next().unwrap();
        match first {
            Value::Integer(int) => Value::Integer(
                std::iter::once(int)
                    .chain(iter.map(|elem| {
                        let Value::Integer(int) = elem else { panic!() };
                        int
                    }))
                    .sum(),
            ),
            Value::Double(double) => Value::Double(
                std::iter::once(double)
                    .chain(iter.map(|elem| {
                        let Value::Double(double) = elem else {
                            panic!()
                        };
                        double
                    }))
                    .sum(),
            ),
        }
    }
}

impl Product for Value {
    fn product<I: Iterator<Item = Self>>(mut iter: I) -> Self {
        let first = iter.next().unwrap();
        match first {
            Value::Integer(int) => Value::Integer(
                std::iter::once(int)
                    .chain(iter.map(|elem| {
                        let Value::Integer(int) = elem else { panic!() };
                        int
                    }))
                    .product(),
            ),
            Value::Double(double) => Value::Double(
                std::iter::once(double)
                    .chain(iter.map(|elem| {
                        let Value::Double(double) = elem else {
                            panic!()
                        };
                        double
                    }))
                    .product(),
            ),
        }
    }
}

impl AST {
    pub fn render(&self, ranges: &Vec<RangeInclusive<usize>>) -> Vec<Span> {
        let style = if false {
            Style::new().fg(Color::Black).bg(Color::White)
        } else {
            Style::new().fg(Color::White)
        };
        match &self {
            AST::Placeholder => vec![Span::styled(" ", style.underlined())],
            AST::Integer(value) => vec![Span::styled(value.to_string(), style)],
            AST::Double(value) => vec![Span::styled(value.to_string(), style)],
            AST::Add(asts) => std::iter::once(Span::styled("(+", style))
                .chain(asts.iter().flat_map(|a| {
                    std::iter::once(Span::styled(" ", style)).chain(a.render(ranges))
                }))
                .chain(std::iter::once(Span::styled(")", style)))
                .collect(),
            AST::Multiply(asts) => std::iter::once(Span::styled("(*", style))
                .chain(asts.iter().flat_map(|a| {
                    std::iter::once(Span::styled(" ", style)).chain(a.render(ranges))
                }))
                .chain(std::iter::once(Span::styled(")", style)))
                .collect(),
        }
    }

    pub fn left(&mut self) {
        match &mut self {
            AST::Placeholder => {}
            AST::Integer(_) => {}
            AST::Double(_) => {}
            AST::Add(asts) | AST::Multiply(asts) => {
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
        match &mut self {
            AST::Placeholder => {}
            AST::Integer(_) => {}
            AST::Double(_) => {}
            AST::Add(asts) | AST::Multiply(asts) => {
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
        match &mut self {
            AST::Placeholder => {}
            AST::Integer(_) => {}
            AST::Double(_) => {}
            AST::Add(asts) | AST::Multiply(asts) => {
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
        match &mut self {
            AST::Placeholder => {}
            AST::Integer(_) => {}
            AST::Double(_) => {}
            AST::Add(asts) | AST::Multiply(asts) => {
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
        match &mut self {
            AST::Placeholder => {}
            AST::Integer(_) => {}
            AST::Double(_) => {}
            AST::Multiply(asts) | AST::Add(asts) => {
                let mut i = asts.len() - 1;
                loop {
                    if asts[i].auxiliary {
                        asts.remove(i);
                        if i > 0 && asts.len() > 0 {
                            if !asts[i - 1].auxiliary {
                                asts[i - 1].auxiliary = true;
                                i -= 1;
                            }
                        } else if i == 0 && asts.len() > 0 {
                            asts[0].auxiliary = true;
                        }
                    } else {
                        asts[i].delete_left();
                    }
                    if i == 0 {
                        break;
                    }
                    i -= 1;
                }
            }
        }
    }

    pub fn delete_right(&mut self) {
        match &mut self {
            AST::Placeholder => {}
            AST::Integer(_) => {}
            AST::Double(_) => {}
            AST::Add(asts) | AST::Multiply(asts) => {
                let mut i = asts.len() - 1;
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
                    } else {
                        asts[i].delete_right();
                    }
                    if i == 0 {
                        break;
                    }
                    i -= 1;
                }
            }
        }
    }

    pub fn insert(&mut self) {
        match &mut self {
            AST::Placeholder => {}
            AST::Integer(_) => {}
            AST::Double(_) => {}
            AST::Add(asts) | AST::Multiply(asts) => {
                for i in (0..asts.len()).rev() {
                    if asts[i].auxiliary {
                        asts[i].auxiliary = false;
                        asts.insert(
                            i + 1,
                            AST {
                                auxiliary: true,
                                inner: AST::Placeholder,
                            },
                        );
                    }
                    asts[i].insert();
                }
            }
        }
    }

    pub fn map_selected(&mut self, mapper: &impl Fn(&mut AST<bool>)) {
        if self.auxiliary {
            mapper(&mut self);
        }
        match &mut self {
            AST::Integer(_) => {}
            AST::Double(_) => {}
            AST::Placeholder => {}
            AST::Add(asts) => {
                asts.iter_mut().for_each(|val| val.map_selected(mapper));
            }
            AST::Multiply(asts) => {
                asts.iter_mut().for_each(|val| val.map_selected(mapper));
            }
        }
    }

    pub fn eval(&mut self) -> Value {
        match &mut self {
            AST::Placeholder => panic!("placeholder can't be evaluated"),
            AST::Integer(value) => Value::Integer(*value),
            AST::Double(value) => Value::Double(*value),
            AST::Add(asts) => asts.iter_mut().map(|e| e.eval()).sum(),
            AST::Multiply(asts) => asts.iter_mut().map(|e| e.eval()).product(),
        }
    }
}

pub struct App {
    ast: AST<bool>,
    status: String,
    selections: Vec<RangeInclusive<usize>>,
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
                    KeyCode::Char(' ') => {
                        self.ast.insert();
                    }
                    KeyCode::Char(char @ '0'..='9') => {
                        self.ast.map_selected(&|input| match input {
                            AST::Integer(value) => {
                                *value = *value * 10 + (char as u64 - '0' as u64)
                            }
                            AST::Double(_) => {}
                            AST::Add(asts) => {}
                            AST::Multiply(asts) => {}
                            AST::Placeholder => *input = AST::Integer(char as u64 - '0' as u64),
                        });
                    }
                    KeyCode::Char('e') => {
                        // TODO edit
                    }
                    KeyCode::Enter => {
                        self.status = format!("evaluated to {:?}", self.ast.eval());
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn ui(&self, frame: &mut Frame) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Percentage(100), Constraint::Length(1)])
            .split(frame.area());

        frame.render_widget(Line::from(self.ast.render(false)), layout[0]);
        frame.render_widget(Line::raw(self.status.clone()), layout[1]);
    }
}

// store the code as structured data in a file and have a custom editor to edit this code
// https://docs.rs/ratatui-core/0.1.0-alpha.2/ratatui_core/text/struct.Span.html

pub fn main() -> io::Result<()> {
    init_panic_hook();
    let mut tui = init_tui()?;
    tui.draw(|frame| frame.render_widget(Span::from("Hello, world!"), frame.area()))?;
    let mut app = App {
        status: "Hello world".to_owned(),
        selections: vec![(0..=1)],
        ast: AST {
            auxiliary: false,
            inner: AST::Add(vec![
                AST {
                    auxiliary: false,
                    inner: AST::Integer(5),
                },
                AST {
                    auxiliary: false,
                    inner: AST::Multiply(vec![
                        AST {
                            auxiliary: false,
                            inner: AST::Integer(5),
                        },
                        AST {
                            auxiliary: true,
                            inner: AST::Integer(7),
                        },
                    ]),
                },
                AST {
                    auxiliary: false,
                    inner: AST::Integer(7),
                },
            ]),
        },
    };
    app.run_app(&mut tui)?;
    restore_tui()?;
    Ok(())
}

pub fn init_panic_hook() {
    let original_hook = take_hook();
    set_hook(Box::new(move |panic_info| {
        // intentionally ignore errors here since we're already in a panic
        let _ = restore_tui();
        original_hook(panic_info);
    }));
}

pub fn init_tui() -> io::Result<Terminal<impl Backend>> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    Terminal::new(CrosstermBackend::new(stdout()))
}

pub fn restore_tui() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}
