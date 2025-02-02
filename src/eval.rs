use std::collections::HashMap;

use crate::ast::{Node, NodeInner};

#[derive(Debug)]
pub enum Type<'a> {
    And(Vec<&'a str>),
    Or(Vec<&'a str>),
    Primitive(&'a str),
    Function {
        /// (name type)
        params: Vec<(&'a str, &'a str)>,
        /// (name type)
        returns: Vec<(&'a str, &'a str)>,
        body: &'a Node<'a>,
    },
}

pub fn eval(input: Vec<Node>) {
    let mut types = HashMap::new();
    for command in &input {
        match &command.inner {
            crate::ast::NodeInner::List(nodes) => match nodes.first() {
                Some(Node {
                    inner: NodeInner::Symbol("define-primitive"),
                    ..
                }) => {
                    let primitive = match &nodes[1].inner {
                        NodeInner::List(nodes) => todo!(),
                        NodeInner::Symbol(symbol) => *symbol,
                    };
                    types.insert(primitive, Type::Primitive(primitive));
                }
                Some(Node {
                    inner: NodeInner::Symbol("define-type"),
                    ..
                }) => {
                    let NodeInner::Symbol(name) = nodes[1].inner else {
                        todo!()
                    };
                    let NodeInner::List(definition) = &nodes[2].inner else {
                        todo!()
                    };
                    match definition[0].inner {
                        NodeInner::Symbol("and") => {
                            let and_types: Vec<&str> = definition[1..]
                                .iter()
                                .map(|elem| match &elem.inner {
                                    NodeInner::List(nodes) => todo!(),
                                    NodeInner::Symbol(name) => *name,
                                })
                                .collect();
                            types.insert(name, Type::And(and_types));
                        }
                        NodeInner::Symbol("or") => {
                            let and_types: Vec<&str> = definition[1..]
                                .iter()
                                .map(|elem| match &elem.inner {
                                    NodeInner::List(nodes) => todo!(),
                                    NodeInner::Symbol(name) => *name,
                                })
                                .collect();
                            types.insert(name, Type::Or(and_types));
                        }
                        _ => todo!(),
                    }
                }
                Some(Node {
                    inner: NodeInner::Symbol("define-function"),
                    ..
                }) => {
                    let NodeInner::Symbol(name) = nodes[1].inner else {
                        todo!()
                    };
                    let NodeInner::List(returns) = &nodes[2].inner else {
                        todo!()
                    };
                    let returns: Vec<(&str, &str)> = returns
                        .iter()
                        .map(|elem| match &elem.inner {
                            NodeInner::List(list) => (
                                (&list[0]).try_into().unwrap(),
                                (&list[1]).try_into().unwrap(),
                            ),
                            NodeInner::Symbol(_) => todo!(),
                        })
                        .collect();
                    let NodeInner::List(params) = &nodes[3].inner else {
                        todo!()
                    };
                    let params: Vec<(&str, &str)> = params
                        .iter()
                        .map(|elem| match &elem.inner {
                            NodeInner::List(list) => (
                                (&list[0]).try_into().unwrap(),
                                (&list[1]).try_into().unwrap(),
                            ),
                            NodeInner::Symbol(_) => todo!(),
                        })
                        .collect();
                    let body = &nodes[4];
                    types.insert(name, Type::Function {
                        params,
                        returns,
                        body,
                    });
                }
                Some(Node {
                    inner: NodeInner::Symbol(command),
                    ..
                }) => {
                    if let Some(typ) = types.get(command) {
                        match typ {
                            Type::And(items) => todo!("construct type"),
                            Type::Or(items) => todo!("construct type"),
                            Type::Primitive(_) => eprintln!("primitive is not callable"),
                            Type::Function {
                                params,
                                returns,
                                body,
                            } => todo!("call function"),
                        }
                    } else {
                        eprintln!("unknown command {command}")
                    }
                }
                _ => todo!(),
            },
            crate::ast::NodeInner::Symbol(symbol) => {
                if let Some(typ) = types.get(symbol) {
                    println!("{symbol} -> {typ:?}")
                } else {
                    eprintln!("unknown symbol {symbol}");
                }
            }
        }
    }
}
