use std::{
    collections::{HashMap, HashSet},
    io::stdout,
    net::SocketAddr,
    panic::{set_hook, take_hook},
    sync::Arc,
};

use append_only_vec::AppendOnlyVec;
use crossterm::event::EventStream;
use futures::StreamExt as _;
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
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_512};
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    net::{TcpListener, TcpStream},
    select, spawn,
    sync::{
        broadcast,
        mpsc::{self, UnboundedReceiver, UnboundedSender, unbounded_channel},
    },
};

pub fn generate_uuid() -> String {
    // get some random data:
    let mut data = [0u8; 64];
    rand::rng().fill_bytes(&mut data);
    base16ct::lower::encode_string(&data)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AST {
    uuid: String,
    changed_by: String,
    value: ASTInner,
}

pub enum MySpan {
    Cursor,
    Text(String, bool),
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

    pub fn render(&self, selected: &HashMap<String, Option<usize>>) -> Vec<MySpan> {
        let se = selected.get(&self.uuid);
        let style = se.is_some();
        match &self.value {
            ASTInner::Add { items } => [
                MySpan::Text("(".to_owned(), style),
                MySpan::Text("+".to_owned(), false),
            ]
            .into_iter()
            .chain(items.iter().enumerate().flat_map(|(idx, a)| {
                let mut arr = a.render(selected);
                arr.insert(0, MySpan::Text(" ".to_owned(), false));
                arr
            }))
            .chain(std::iter::once(MySpan::Text(")".to_owned(), style)))
            .collect(),
            ASTInner::Integer { value } => {
                if let Some(se) = se {
                    if let Some(se) = se {
                        let before = value.to_string()[..*se].to_owned();
                        let after = value.to_string()[*se..].to_owned();
                        vec![
                            MySpan::Text(before, false),
                            MySpan::Cursor,
                            MySpan::Text(after, false),
                        ]
                    } else {
                        vec![MySpan::Text(value.to_string(), true)]
                    }
                } else {
                    vec![MySpan::Text(value.to_string(), false)]
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum ASTInner {
    Add {
        // two users should be allowed to add elements concurrently without conflict?
        // or maybe a light conflict that you can easily resolve?
        items: Vec<AST>,
    },
    Integer {
        // e.g. if one user updates this, then this should be fine.
        // but two users updating it should create a conflict
        value: i64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ASTHistoryEntry {
    previous: Vec<String>,
    peer: String, // TODO sign with this peer id
    // we could also store which commit changed the value last here and
    // if it doesn't match, it's a conflict
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        wrapping_uuid: String,
    },
    /// As we're a programming language inserting at index probably makes most
    /// sense
    InsertAtIndex {
        uuid: String,
        index: usize,
        ast: AST,
    },
    Delete {
        uuid: String,
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
    selected: HashMap<String, Option<usize>>,
    receive: mpsc::UnboundedReceiver<(ASTHistoryEntry, SocketAddr)>,
    send: broadcast::Sender<ASTHistoryEntry>,
}

impl App {
    pub fn apply(&mut self, history: &ASTHistoryEntry) {
        match &history.value {
            ASTHistoryEntryInner::Initial { ast } => panic!("initial can not be set twice"),
            ASTHistoryEntryInner::SetInteger {
                uuid,
                value: new_value,
            } => {
                let ast = self.ast.get_by_uuid_mut(uuid).unwrap();
                let ASTInner::Integer { value } = &mut ast.value else {
                    panic!()
                };
                *value = *new_value;
            }
            ASTHistoryEntryInner::InsertAtIndex { uuid, index, ast } => {
                let Some(list_ast) = self.ast.get_by_uuid_mut(uuid) else {
                    panic!("failed to get {uuid} from {:?}", self.ast)
                };
                let ASTInner::Add { items } = &mut list_ast.value else {
                    panic!()
                };
                items.insert(*index, ast.clone());
            }
            ASTHistoryEntryInner::WrapIntegerInAdd {
                uuid,
                wrapping_uuid,
            } => {
                let ast = self.ast.get_by_uuid_mut(uuid).unwrap();

                // this looks like an issue
                let new = AST {
                    uuid: wrapping_uuid.clone(),
                    changed_by: history.peer.clone(),
                    value: ASTInner::Add { items: vec![] },
                };

                let inner = std::mem::replace(ast, new);

                let ASTInner::Add { items } = &mut ast.value else {
                    panic!()
                };
                items.push(inner);
            }
            ASTHistoryEntryInner::Delete { uuid } => {
                let ASTInner::Add { items } = &mut self.ast.parent_of_uuid_mut(uuid).unwrap().value
                else {
                    panic!();
                };

                items.retain(|elem| elem.uuid != *uuid);
            }
        }
        #[cfg(debug_assertions)]
        self.ast.validate();
    }

    fn handle_event(&mut self, event: Option<Result<Event, std::io::Error>>) -> bool {
        if let Some(Ok(Event::Key(key))) = event {
            if key.kind == event::KeyEventKind::Release {
                // Skip events that are not KeyEventKind::Press
                return false;
            }
            self.status = "".to_owned();
            match key.code {
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    return true;
                }
                KeyCode::Char('+') => {
                    let operations = self
                        .selected
                        .iter()
                        .filter_map(|(elem, offset)| {
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
                                        wrapping_uuid: generate_uuid(),
                                    },
                                }),
                            }
                        })
                        .collect::<Vec<_>>();

                    operations.iter().for_each(|history| {
                        self.ast.apply(history);
                        self.send.send(history.clone()).unwrap();
                    });
                }
                KeyCode::Insert => {
                    // insert to the right in list

                    let operations = self
                        .selected
                        .iter()
                        .filter_map(|(elem, offset)| {
                            let parent = self.ast.parent_of_uuid_mut(elem)?;

                            match &parent.value {
                                ASTInner::Add { items } => {
                                    let uuid = generate_uuid();
                                    Some(ASTHistoryEntry {
                                        previous: vec![],
                                        peer: "todo".to_owned(),
                                        value: ASTHistoryEntryInner::InsertAtIndex {
                                            uuid: parent.uuid.clone(),
                                            index: items
                                                .iter()
                                                .position(|item| item.uuid == *elem)
                                                .unwrap()
                                                + 1,
                                            ast: AST {
                                                uuid,
                                                changed_by: "".to_owned(),
                                                value: ASTInner::Integer { value: 1 },
                                            },
                                        },
                                    })
                                }
                                ASTInner::Integer { value } => None,
                            }
                        })
                        .collect::<Vec<_>>();

                    operations.iter().for_each(|history| {
                        let ASTHistoryEntryInner::InsertAtIndex {
                            ref uuid,
                            ref index,
                            ref ast,
                        } = history.value
                        else {
                            panic!();
                        };
                        let ASTInner::Add { ref items } =
                            self.ast.get_by_uuid_mut(uuid).unwrap().value
                        else {
                            panic!();
                        };
                        self.selected.remove(&items[*index].uuid);
                        self.selected.insert(ast.uuid.clone(), None);
                    });

                    operations.iter().for_each(|history| {
                        self.ast.apply(history);
                        self.send.send(history.clone()).unwrap();
                    });
                }
                KeyCode::Char(' ') => {
                    // insert in list (maybe first simply to the right?)

                    let operations = self
                        .selected
                        .iter()
                        .filter_map(|(elem, offset)| {
                            let parent = self.ast.parent_of_uuid_mut(elem)?;

                            match &parent.value {
                                ASTInner::Add { items } => Some(ASTHistoryEntry {
                                    previous: vec![],
                                    peer: "todo".to_owned(),
                                    value: ASTHistoryEntryInner::InsertAtIndex {
                                        uuid: parent.uuid.clone(),
                                        index: items
                                            .iter()
                                            .position(|item| item.uuid == *elem)
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

                    operations.iter().for_each(|history| {
                        self.ast.apply(history);
                        self.send.send(history.clone()).unwrap();
                    });
                }
                KeyCode::Down => {
                    self.selected = self
                        .selected
                        .iter()
                        .map(|(elem, offset)| {
                            let node = self.ast.get_by_uuid_mut(elem).unwrap();
                            match &node.value {
                                ASTInner::Add { items } => {
                                    (items.first().unwrap().uuid.clone(), None)
                                }
                                ASTInner::Integer { value } => (node.uuid.clone(), Some(0)),
                            }
                        })
                        .collect();
                }
                KeyCode::Up => {
                    self.selected = self
                        .selected
                        .iter()
                        .map(|(elem, offset)| {
                            if offset.is_some() {
                                return (elem.clone(), None);
                            }

                            self.ast
                                .parent_of_uuid_mut(elem)
                                .map(|item| (item.uuid.clone(), None))
                                .unwrap_or((elem.clone(), None))
                        })
                        .collect();
                }
                KeyCode::Left => {
                    self.selected = self
                        .selected
                        .iter()
                        .map(|(elem, offset)| {
                            if let Some(offset) = offset {
                                let node = self.ast.get_by_uuid_mut(elem).unwrap();

                                match &node.value {
                                    ASTInner::Add { items } => {}
                                    ASTInner::Integer { value } => {
                                        return (elem.clone(), Some(offset.saturating_sub(1)));
                                    }
                                }
                            }

                            let Some(parent) = self.ast.parent_of_uuid_mut(elem) else {
                                return (elem.clone(), *offset);
                            };

                            match &parent.value {
                                ASTInner::Add { items } => {
                                    let index =
                                        items.iter().position(|item| item.uuid == *elem).unwrap();
                                    (items[index.saturating_sub(1)].uuid.clone(), None)
                                }
                                ASTInner::Integer { value } => unreachable!(),
                            }
                        })
                        .collect();
                }
                KeyCode::Right => {
                    self.selected = self
                        .selected
                        .iter()
                        .map(|(elem, offset)| {
                            if let Some(offset) = offset {
                                let node = self.ast.get_by_uuid_mut(elem).unwrap();

                                match &node.value {
                                    ASTInner::Add { items } => {}
                                    ASTInner::Integer { value } => {
                                        return (
                                            elem.clone(),
                                            Some(std::cmp::min(
                                                value.to_string().len(),
                                                offset + 1,
                                            )),
                                        );
                                    }
                                }
                            }

                            let Some(parent) = self.ast.parent_of_uuid_mut(elem) else {
                                return (elem.clone(), *offset);
                            };

                            match &parent.value {
                                ASTInner::Add { items } => {
                                    let index =
                                        items.iter().position(|item| item.uuid == *elem).unwrap();
                                    (
                                        items[std::cmp::min(items.len() - 1, index + 1)]
                                            .uuid
                                            .clone(),
                                        None,
                                    )
                                }
                                ASTInner::Integer { value } => unreachable!(),
                            }
                        })
                        .collect();
                }
                KeyCode::Char(char @ '0'..='9') => {
                    let operations = self
                        .selected
                        .iter()
                        .filter_map(|(elem, offset)| {
                            let node = self.ast.get_by_uuid_mut(elem).unwrap();

                            match &node.value {
                                ASTInner::Add { items } => None,
                                ASTInner::Integer { value } => {
                                    let mut new_value = value.to_string();
                                    new_value.insert((*offset)?, char);
                                    Some(ASTHistoryEntry {
                                        previous: vec![],
                                        peer: "todo".to_owned(),
                                        value: ASTHistoryEntryInner::SetInteger {
                                            uuid: elem.clone(),
                                            value: new_value.parse().unwrap(),
                                        },
                                    })
                                }
                            }
                        })
                        .collect::<Vec<_>>();

                    self.selected.iter_mut().for_each(|(elem, offset)| {
                        let node = self.ast.get_by_uuid_mut(&elem).unwrap();

                        match &node.value {
                            ASTInner::Add { items } => {}
                            ASTInner::Integer { value } => {
                                *offset = offset.map(|offset| offset + 1);
                            }
                        }
                    });

                    // First apply all operations to ensure bugs in the apply logic don't propagate
                    operations.iter().for_each(|history| {
                        self.ast.apply(history);
                    });
                    operations.iter().for_each(|history| {
                        self.send.send(history.clone()).unwrap();
                    });
                }
                KeyCode::Backspace => {
                    let operations = self
                        .selected
                        .iter()
                        .filter_map(|(elem, offset)| {
                            let node = self.ast.get_by_uuid_mut(elem).unwrap();

                            match &node.value {
                                ASTInner::Add { items } => None,
                                ASTInner::Integer { value } => {
                                    if let Some(offset) = offset {
                                        let mut new_value = value.to_string();
                                        new_value.remove(*offset);
                                        Some(ASTHistoryEntry {
                                            previous: vec![],
                                            peer: "todo".to_owned(),
                                            value: ASTHistoryEntryInner::SetInteger {
                                                uuid: elem.clone(),
                                                value: new_value.parse().unwrap(),
                                            },
                                        })
                                    } else {
                                        Some(ASTHistoryEntry {
                                            previous: vec![],
                                            peer: "todo".to_owned(),
                                            value: ASTHistoryEntryInner::Delete {
                                                uuid: elem.clone(),
                                            },
                                        })
                                    }
                                }
                            }
                        })
                        .collect::<Vec<_>>();

                    self.selected.iter_mut().for_each(|(elem, offset)| {
                        let node = self.ast.get_by_uuid_mut(&elem).unwrap();

                        match &node.value {
                            ASTInner::Add { items } => {}
                            ASTInner::Integer { value } => {
                                *offset = offset.map(|offset| offset - 1);
                            }
                        }
                    });

                    operations.iter().for_each(|history| {
                        self.ast.apply(history);
                        self.send.send(history.clone()).unwrap();
                    });
                }
                KeyCode::Delete => {
                    let operations = self
                        .selected
                        .iter()
                        .filter_map(|(elem, offset)| {
                            let node = self.ast.get_by_uuid_mut(elem).unwrap();

                            match &node.value {
                                ASTInner::Add { items } => None,
                                ASTInner::Integer { value } => {
                                    let mut new_value = value.to_string();
                                    new_value.remove(offset.unwrap());
                                    Some(ASTHistoryEntry {
                                        previous: vec![],
                                        peer: "todo".to_owned(),
                                        value: ASTHistoryEntryInner::SetInteger {
                                            uuid: elem.clone(),
                                            value: new_value.parse().unwrap(),
                                        },
                                    })
                                }
                            }
                        })
                        .collect::<Vec<_>>();

                    self.selected.iter_mut().for_each(|(elem, offset)| {
                        let node = self.ast.get_by_uuid_mut(&elem).unwrap();

                        match &node.value {
                            ASTInner::Add { items } => {}
                            ASTInner::Integer { value } => {
                                *offset = offset.map(|offset| offset - 1);
                            }
                        }
                    });

                    operations.iter().for_each(|history| {
                        self.ast.apply(history);
                        self.send.send(history.clone()).unwrap();
                    });
                }
                _ => {}
            }
        }
        false
    }

    async fn run_app<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> std::io::Result<()> {
        let mut event_stream = EventStream::new();
        loop {
            terminal.draw(|f| self.ui(f))?;

            select! {
                event = event_stream.next() => {
                    if self.handle_event(event) {
                        break Ok(());
                    }
                }
                rec = self.receive.recv() => {
                    match rec {
                        Some(rec) => self.ast.apply(&rec.0),
                        None => panic!("empty")
                    }
                }
            }
        }
    }

    pub fn ui(&mut self, frame: &mut Frame) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Percentage(100), Constraint::Length(1)])
            .split(frame.area());

        let highlighted = Style::new().fg(Color::Black).bg(Color::White);
        let not_highlighted = Style::new().fg(Color::White);

        self.selected = self
            .selected
            .iter()
            .filter_map(|(elem, offset)| {
                let ast = self.ast.get_by_uuid_mut(elem)?;

                Some((elem.clone(), offset.clone()))
            })
            .collect();

        if self.selected.is_empty() {
            self.selected.insert(self.ast.uuid.clone(), None);
        }

        let binding = self.ast.render(&self.selected);
        let mut content = binding
            .iter()
            .filter(|i| match i {
                MySpan::Text(text, _) => !text.is_empty(),
                _ => true,
            })
            .peekable();
        let mut result = Vec::new();
        while let Some(myspan) = content.next() {
            match myspan {
                MySpan::Cursor => match content.peek() {
                    Some(MySpan::Text(text, is_highlighted)) => {
                        content.next();
                        let (text, text_last) = text.split_at(1);
                        result.push(Span::styled(text, highlighted));
                        result.push(Span::styled(
                            text_last,
                            if *is_highlighted {
                                highlighted
                            } else {
                                not_highlighted
                            },
                        ))
                    }
                    Some(MySpan::Cursor) => unreachable!(),
                    None => result.push(Span::styled(" ".to_owned(), highlighted)),
                },
                MySpan::Text(text, is_highlighted) => result.push(Span::styled(
                    text,
                    if *is_highlighted {
                        highlighted
                    } else {
                        not_highlighted
                    },
                )),
            }
        }

        frame.render_widget(Line::from(result), layout[0]);
        frame.render_widget(Line::raw(self.status.clone()), layout[1]);
        //frame.set_cursor_position((5, 0));
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // maybe having a dedicated server and all tuis are clients would be simpler?

    let history: Arc<AppendOnlyVec<ASTHistoryEntry>> = Arc::new(AppendOnlyVec::new());

    let (mut receive_sender, mut receive_receiver) =
        mpsc::unbounded_channel::<(ASTHistoryEntry, SocketAddr)>();
    let (mut send_sender, mut send_receiver) = broadcast::channel::<ASTHistoryEntry>(16);

    let mut stream = TcpStream::connect("127.0.0.1:1234").await.unwrap();
    let local_addr = stream.local_addr().unwrap();

    let (mut read, mut write) = stream.into_split();
    spawn(async move {
        // println!("new thread");

        while let Ok(rec) = send_receiver.recv().await {
            let serialized = serde_json::to_string(&rec).unwrap();
            let len: u64 = serialized.as_bytes().len().try_into().unwrap();
            // println!("send stuff");
            write.write(&len.to_be_bytes()).await.unwrap();
            write.write(serialized.as_bytes()).await.unwrap();
        }
    });
    {
        let receive_sender = receive_sender.clone();
        spawn(async move {
            // println!("new thread");
            let mut buf: [u8; 8] = [0; 8];
            loop {
                read.read_exact(&mut buf).await.unwrap();
                let size = u64::from_be_bytes(buf);
                let mut buf = vec![0; size.try_into().unwrap()];
                read.read_exact(&mut buf).await.unwrap();
                let deserialized: ASTHistoryEntry = serde_json::from_slice(&buf).unwrap();

                // println!("got new packet {:?}", deserialized);

                receive_sender.send((deserialized, local_addr)).unwrap();
            }
        });
    }

    // println!("connected to server");

    if args.len() > 1 {
        let initial_uuid = generate_uuid();
        let first = ASTHistoryEntry {
            peer: "1".to_string(),
            previous: vec![],
            value: ASTHistoryEntryInner::Initial {
                ast: AST {
                    uuid: initial_uuid.clone(),
                    changed_by: generate_uuid(),
                    value: ASTInner::Integer { value: 42 },
                },
            },
        };
        let first_hash = first.hash();
        history.push(first.clone());
        send_sender.send(first.clone()).unwrap();
        receive_sender.send((first, local_addr)).unwrap();
        let second = ASTHistoryEntry {
            peer: "2".to_string(),
            previous: vec![first_hash],
            value: ASTHistoryEntryInner::SetInteger {
                uuid: initial_uuid,
                value: 43,
            },
        };
        history.push(second.clone());
        send_sender.send(second.clone()).unwrap();
        receive_sender.send((second, local_addr)).unwrap();
    }

    println!(
        "waiting for initial? if this doesn't work add a second argument to trigger initialization"
    );
    let ref entry @ ASTHistoryEntry {
        ref previous,
        ref peer,
        value: ASTHistoryEntryInner::Initial { ref ast },
    } = receive_receiver.recv().await.unwrap().0
    else {
        panic!()
    };
    let mut ast = ast.clone();
    // println!("{ast:?}");

    // peer to peer is cool

    // first step is just apply updates in dag traversal order

    // maybe for every element store who updated it last (kind of like blame
    // information?) and create conflict if it is a parallel edit?

    let original_hook = take_hook();
    set_hook(Box::new(move |panic_info| {
        let _ = restore_tui();
        original_hook(panic_info);
    }));
    let mut tui = init_tui()?;
    let mut app = App {
        status: "Hello world".to_owned(),
        selected: HashMap::from([(ast.uuid.clone(), None)]),
        ast,
        receive: receive_receiver,
        send: send_sender,
    };
    app.run_app(&mut tui).await?;
    restore_tui()?;

    // println!("{:?}", app.ast);

    Ok(())
}
