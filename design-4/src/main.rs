use std::{
    io::stdout,
    panic::{set_hook, take_hook},
};

use ratatui::{
    Frame, Terminal,
    crossterm::{
        event::{self, Event, KeyCode, KeyModifiers},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    layout::{Constraint, Direction, Layout},
    prelude::{Backend, CrosstermBackend},
    text::{Line, Span},
};
use sha3::{Digest, Sha3_512};

#[derive(Debug, Clone)]
pub struct AST {
    uuid: String,
    changed_by: String,
    value: ASTInner,
}

impl AST {
    pub fn get_by_uuid_mut(&mut self, uuid: &str) -> Option<&mut AST> {
        if self.uuid == uuid {
            return Some(self);
        }
        match &mut self.value {
            ASTInner::Add { items } => items.iter_mut().find_map(|item| item.get_by_uuid_mut(uuid)),
            ASTInner::Integer { value } => panic!(),
        }
    }
}

#[derive(Debug, Clone)]
enum ASTInner {
    Add {
        items: Vec<AST>, // two users should be allowed to add elements concurrently without conflict? or maybe a light conflict that you can easily resolve?
    },
    Integer {
        value: i64, // e.g. if one user updates this, then this should be fine. but two users updating it should create a conflict
    },
}

#[derive(Debug, Clone)]
pub struct ASTHistoryEntry {
    previous: Vec<String>,
    peer: String, // TODO sign with this peer id
    // we could also store which commit changed the value last here and if it doesn't match, it's a conflict
    value: ASTHistoryEntryInner,
}

impl ASTHistoryEntry {
    fn hash(&self) -> String {
        let hasher = Sha3_512::new();
        let hasher = self
            .previous
            .iter()
            .fold(hasher, |hasher, val| hasher.chain_update(val));
        let hasher = hasher
            .chain_update(&self.peer)
            .chain_update(format!("{:?}", self.value));
        let hash = hasher.finalize();
        let hex_hash = base16ct::lower::encode_string(&hash);
        hex_hash
    }
}

#[derive(Debug, Clone)]
enum ASTHistoryEntryInner {
    Initial { ast: AST },
    SetInteger { uuid: String, value: i64 },
    InsertToAdd { uuid: String, ast: AST },
}

pub fn init_tui() -> std::io::Result<Terminal<impl Backend>> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    Terminal::new(CrosstermBackend::new(stdout()))
}

pub fn restore_tui() -> std::io::Result<()> {
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}

pub struct App {
    ast: AST,
    status: String,
}

impl App {
    fn run_app<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> std::io::Result<()> {
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
                    KeyCode::Left => {}
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

        //frame.render_widget(Line::from(self.ast.render(false)), layout[0]);
        frame.render_widget(Line::raw(self.status.clone()), layout[1]);
    }
}

fn main() -> std::io::Result<()> {
    let ast_peer_1 = vec![ASTHistoryEntry {
        peer: "1".to_string(),
        previous: vec![],
        value: ASTHistoryEntryInner::Initial {
            ast: AST {
                uuid: "test".to_owned(),
                changed_by: "".to_owned(),
                value: ASTInner::Integer { value: 42 },
            },
        },
    }];

    let mut ast_peer_2 = ast_peer_1.clone();
    ast_peer_2.push(ASTHistoryEntry {
        peer: "2".to_string(),
        previous: vec![ast_peer_2[0].hash()],
        value: ASTHistoryEntryInner::SetInteger {
            uuid: "test".to_owned(),
            value: 43,
        },
    });

    let mut ast_peer_2_iter = ast_peer_2.iter();
    let Some(ASTHistoryEntry {
        previous,
        peer,
        value: ASTHistoryEntryInner::Initial { ast },
    }) = ast_peer_2_iter.next()
    else {
        panic!()
    };
    let mut ast = ast.clone();
    println!("{ast:?}");

    for history in ast_peer_2_iter {
        match &history.value {
            ASTHistoryEntryInner::Initial { ast } => panic!("initial can not be set twice"),
            ASTHistoryEntryInner::SetInteger {
                uuid,
                value: new_value,
            } => {
                println!("modifying ast integer");
                let ast = ast.get_by_uuid_mut(uuid).unwrap();
                let ASTInner::Integer { value } = &mut ast.value else {
                    panic!()
                };
                *value = *new_value;
            }
            ASTHistoryEntryInner::InsertToAdd { uuid, ast } => todo!(),
        }
    }
    println!("{ast:?}");

    // peer to peer is cool

    // first step is just apply updates in dag traversal order

    // maybe for every element store who updated it last (kind of like blame information?) and create conflict if it is a parallel edit?

    let original_hook = take_hook();
    set_hook(Box::new(move |panic_info| {
        let _ = restore_tui();
        original_hook(panic_info);
    }));
    let mut tui = init_tui()?;
    tui.draw(|frame| frame.render_widget(Span::from("Hello, world!"), frame.area()))?;
    let mut app = App {
        status: "Hello world".to_owned(),
        ast,
    };
    app.run_app(&mut tui)?;
    restore_tui()?;
    Ok(())
}
