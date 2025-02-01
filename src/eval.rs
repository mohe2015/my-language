use std::collections::HashMap;

use crate::ast::{Node, NodeInner};

pub enum Type<'a> {
    And(Vec<&'a str>),
    Or(Vec<&'a str>),
    Primitive(&'a str),
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
                    println!("defining primitive");
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
                            println!("and type");
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
                            println!("or type");
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
                    todo!()
                }
                Some(Node {
                    inner: NodeInner::Symbol(command),
                    ..
                }) => {
                    todo!("unknown command {command}")
                }
                _ => todo!(),
            },
            crate::ast::NodeInner::Symbol(_) => todo!(),
        }
    }
}
