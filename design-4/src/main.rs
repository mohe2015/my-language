use std::{
    collections::HashSet,
    io::stdout,
    panic::{set_hook, take_hook},
};

use rand::RngCore as _;
use ratatui::{
    Frame, Terminal,
    crossterm::{
        event::{self, Event, KeyCode, KeyModifiers},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    layout::{Constraint, Direction, Layout},
    prelude::{Backend, CrosstermBackend},
    style::{Color, Style},
    text::{Line, Span},
};
use sha3::{Digest, Sha3_512};

pub fn generate_uuid() -> String {
    // get some random data:
    let mut data = [0u8; 64];
    rand::rng().fill_bytes(&mut data);
    base16ct::lower::encode_string(&data)
}

#[derive(Debug, Clone)]
pub struct AST {
    uuid: String,
    changed_by: String,
    value: ASTInner,
}

impl AST {
    /// Check no uuid is duplicated
    pub fn validate(&self) {
        self.validate_inner(&mut HashSet::new());
    }

    fn validate_inner(&self, known: &mut HashSet<String>) {
        assert!(known.insert(self.uuid.clone()));
        match &self.value {
            ASTInner::Add { items } => items.iter().for_each(|item| item.validate_inner(known)),
            ASTInner::Integer { value } => {}
        }
    }

    pub fn get_by_uuid_mut(&mut self, uuid: &str) -> Option<&mut AST> {
        #[cfg(debug_assertions)]
        self.validate();
        if self.uuid == uuid {
            return Some(self);
        }
        match &mut self.value {
            ASTInner::Add { items } => items.iter_mut().find_map(|item| item.get_by_uuid_mut(uuid)),
            ASTInner::Integer { value } => None,
        }
    }

    pub fn parent_of_uuid_mut<'a, 'b>(&'a mut self, uuid: &'b str) -> Option<&'a mut AST> {
        #[cfg(debug_assertions)]
        self.validate();
        match &mut self.value {
            ASTInner::Add { items } => {
                if items.iter_mut().any(|item| item.uuid == uuid) {
                    return Some(self);
                }
            }
            ASTInner::Integer { value } => {}
        }
        match &mut self.value {
            ASTInner::Add { items } => {
                return items
                    .iter_mut()
                    .find_map(|item| item.parent_of_uuid_mut(uuid));
            }
            ASTInner::Integer { value } => {}
        }
        None
    }

    pub fn apply(&mut self, history: &ASTHistoryEntry) {
        match &history.value {
            ASTHistoryEntryInner::Initial { ast } => panic!("initial can not be set twice"),
            ASTHistoryEntryInner::SetInteger {
                uuid,
                value: new_value,
            } => {
                let ast = self.get_by_uuid_mut(uuid).unwrap();
                let ASTInner::Integer { value } = &mut ast.value else {
                    panic!()
                };
                *value = *new_value;
            }
            ASTHistoryEntryInner::InsertAtIndex { uuid, index, ast } => {
                let list_ast = self.get_by_uuid_mut(uuid).unwrap();
                let ASTInner::Add { items } = &mut list_ast.value else {
                    panic!()
                };
                items.insert(*index, ast.clone());
            }
            ASTHistoryEntryInner::WrapIntegerInAdd { uuid } => {
                let ast = self.get_by_uuid_mut(uuid).unwrap();

                let new = AST {
                    uuid: generate_uuid(),
                    changed_by: history.peer.clone(),
                    value: ASTInner::Add { items: vec![] },
                };

                let inner = std::mem::replace(ast, new);

                let ASTInner::Add { items } = &mut ast.value else {
                    panic!()
                };
                items.push(inner);
            }
        }
        #[cfg(debug_assertions)]
        self.validate();
    }

    pub fn render(&self, selected: &HashSet<String>) -> Vec<Span> {
        let highlighted = Style::new().fg(Color::Black).bg(Color::White);
        let not_highlighted = Style::new().fg(Color::White);
        let style = if selected.contains(&self.uuid) {
            highlighted
        } else {
            not_highlighted
        };
        match &self.value {
            ASTInner::Add { items } => {
                [Span::styled("(", style), Span::styled("+", not_highlighted)]
                    .into_iter()
                    .chain(items.iter().flat_map(|a| {
                        std::iter::once(Span::styled(" ", not_highlighted))
                            .chain(a.render(selected))
                    }))
                    .chain(std::iter::once(Span::styled(")", style)))
                    .collect()
            }
            ASTInner::Integer { value } => vec![Span::styled(value.to_string(), style)],
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
    Initial {
        ast: AST,
    },
    SetInteger {
        uuid: String,
        value: i64,
    },
    WrapIntegerInAdd {
        uuid: String,
    },
    /// As we're a programming language inserting at index probably makes most sense
    InsertAtIndex {
        uuid: String,
        index: usize,
        ast: AST,
    },
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
    /// UUIDs of selected nodes
    selected: HashSet<String>,
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
                self.status = "".to_owned();
                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(());
                    }
                    KeyCode::Char('+') => {
                        let operations = self
                            .selected
                            .iter()
                            .filter_map(|elem| {
                                let node = self.ast.get_by_uuid_mut(elem).unwrap();
                                match &node.value {
                                    ASTInner::Add { items } => {
                                        self.status = "can't wrap + into +".to_owned();
                                        None
                                    }
                                    ASTInner::Integer { value } => Some(ASTHistoryEntry {
                                        previous: vec![],
                                        peer: "todo".to_owned(),
                                        value: ASTHistoryEntryInner::WrapIntegerInAdd {
                                            uuid: elem.clone(),
                                        },
                                    }),
                                }
                            })
                            .collect::<Vec<_>>();

                        operations
                            .iter()
                            .for_each(|history| self.ast.apply(history));
                    }
                    KeyCode::Char(' ') => {
                        // insert in list (maybe first simply to the right?)

                        let operations = self
                            .selected
                            .iter()
                            .filter_map(|elem| {
                                let child = self.ast.get_by_uuid_mut(elem).unwrap().uuid.clone();
                                let node = self.ast.parent_of_uuid_mut(elem)?;

                                match &node.value {
                                    ASTInner::Add { items } => Some(ASTHistoryEntry {
                                        previous: vec![],
                                        peer: "todo".to_owned(),
                                        value: ASTHistoryEntryInner::InsertAtIndex {
                                            uuid: node.uuid.clone(),
                                            index: items
                                                .iter()
                                                .position(|item| item.uuid == child)
                                                .unwrap(),
                                            ast: AST {
                                                uuid: generate_uuid(),
                                                changed_by: "".to_owned(),
                                                value: ASTInner::Integer { value: 1 },
                                            },
                                        },
                                    }),
                                    ASTInner::Integer { value } => None,
                                }
                            })
                            .collect::<Vec<_>>();

                        operations
                            .iter()
                            .for_each(|history| self.ast.apply(history));
                    }
                    KeyCode::Down => {
                        self.selected = self
                            .selected
                            .iter()
                            .map(|elem| {
                                let node = self.ast.get_by_uuid_mut(elem).unwrap();
                                match &node.value {
                                    ASTInner::Add { items } => items.first().unwrap().uuid.clone(),
                                    ASTInner::Integer { value } => node.uuid.clone(),
                                }
                            })
                            .collect();
                    }
                    KeyCode::Up => {
                        self.selected = self
                            .selected
                            .iter()
                            .map(|elem| {
                                self.ast
                                    .parent_of_uuid_mut(elem)
                                    .map(|item| item.uuid.clone())
                                    .unwrap_or(elem.clone())
                            })
                            .collect();
                    }
                    KeyCode::Left => {
                        self.selected = self
                            .selected
                            .iter()
                            .filter_map(|elem| {
                                let child = self.ast.get_by_uuid_mut(elem).unwrap().uuid.clone();
                                let node = self.ast.parent_of_uuid_mut(elem)?;

                                match &node.value {
                                    ASTInner::Add { items } => {
                                        let index = items
                                            .iter()
                                            .position(|item| item.uuid == child)
                                            .unwrap();
                                        items.get(index.saturating_sub(1)).map(|i| i.uuid.clone())
                                    }
                                    ASTInner::Integer { value } => Some(elem.clone()),
                                }
                            })
                            .collect();
                    }
                    KeyCode::Right => {
                        self.selected = self
                            .selected
                            .iter()
                            .filter_map(|elem| {
                                let child = self.ast.get_by_uuid_mut(elem).unwrap().uuid.clone();
                                let node = self.ast.parent_of_uuid_mut(elem)?;

                                match &node.value {
                                    ASTInner::Add { items } => {
                                        let index = items
                                            .iter()
                                            .position(|item| item.uuid == child)
                                            .unwrap();
                                        items
                                            .get(std::cmp::min(items.len() - 1, index + 1))
                                            .map(|i| i.uuid.clone())
                                    }
                                    ASTInner::Integer { value } => Some(elem.clone()),
                                }
                            })
                            .collect();
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

        frame.render_widget(Line::from(self.ast.render(&self.selected)), layout[0]);
        frame.render_widget(Line::raw(self.status.clone()), layout[1]);
    }
}

fn main() -> std::io::Result<()> {
    let initial_uuid = generate_uuid();
    let ast_peer_1 = vec![ASTHistoryEntry {
        peer: "1".to_string(),
        previous: vec![],
        value: ASTHistoryEntryInner::Initial {
            ast: AST {
                uuid: initial_uuid.clone(),
                changed_by: generate_uuid(),
                value: ASTInner::Integer { value: 42 },
            },
        },
    }];

    let mut ast_peer_2 = ast_peer_1.clone();
    ast_peer_2.push(ASTHistoryEntry {
        peer: "2".to_string(),
        previous: vec![ast_peer_2[0].hash()],
        value: ASTHistoryEntryInner::SetInteger {
            uuid: initial_uuid,
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
        ast.apply(history);
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
        selected: HashSet::from([ast.uuid.clone()]),
        ast,
    };
    app.run_app(&mut tui)?;
    restore_tui()?;

    println!("{:?}", app.ast);

    Ok(())
}
