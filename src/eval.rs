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

#[derive(Debug, Clone)]
pub enum Value<'a> {
    PrimitiveType(),
    AndType(Vec<&'a str>),
    OrType(Vec<&'a str>),
    Function {
        /// (name type)
        params: Vec<(&'a str, &'a str)>,
        /// (name type)
        returns: Vec<(&'a str, &'a str)>,
        body: &'a Node<'a>,
    },
    Unit, // TODO special primitivetype?
}

pub fn eval<'a>(input: &'a Node<'a>, env: &mut HashMap<&'a str, Value<'a>>) -> Value<'a> {
    match &input.inner {
        crate::ast::NodeInner::List(nodes) => match nodes.first() {
            Some(Node {
                inner: NodeInner::Symbol("define-primitive"),
                ..
            }) => {
                assert_eq!(nodes.len(), 1);
                Value::PrimitiveType()
            }
            Some(Node {
                inner: NodeInner::Symbol("define-type"),
                ..
            }) => {
                assert_eq!(nodes.len(), 2);
                let definition: &'a Vec<Node<'a>> = (&nodes[1]).try_into().unwrap();
                match definition[0].inner {
                    NodeInner::Symbol("and") => {
                        let and_types: Vec<&str> = definition[1..]
                            .iter()
                            .map(|elem| match &elem.inner {
                                NodeInner::List(nodes) => todo!(),
                                NodeInner::Symbol(name) => *name,
                            })
                            .collect();
                        Value::AndType(and_types)
                    }
                    NodeInner::Symbol("or") => {
                        let and_types: Vec<&str> = definition[1..]
                            .iter()
                            .map(|elem| match &elem.inner {
                                NodeInner::List(nodes) => todo!(),
                                NodeInner::Symbol(name) => *name,
                            })
                            .collect();
                        Value::OrType(and_types)
                    }
                    _ => todo!(),
                }
            }
            Some(Node {
                inner: NodeInner::Symbol("define-function"),
                ..
            }) => {
                assert_eq!(nodes.len(), 4);
                let returns: &'a Vec<Node<'a>> = (&nodes[1]).try_into().unwrap();
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
                let params: &'a Vec<Node<'a>> = (&nodes[2]).try_into().unwrap();
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
                let body = &nodes[3];
                Value::Function {
                    params,
                    returns,
                    body,
                }
            }
            Some(Node {
                inner: NodeInner::Symbol("set"),
                ..
            }) => {
                assert_eq!(nodes.len(), 3);
                let name: &str = (&nodes[1]).try_into().unwrap();
                let value = &nodes[2];
                let value = eval(value, &mut env.clone());
                env.insert(name, value);
                Value::Unit
            }
            Some(Node {
                inner: NodeInner::Symbol(command),
                ..
            }) => {
                if let Some(typ) = env.get(command) {
                    match typ {
                        Value::AndType(items) => todo!("construct type"),
                        Value::OrType(items) => {
                            let instantiated_type: &str = (&nodes[1]).try_into().unwrap();
                            if items.contains(&instantiated_type) {
                                todo!("actually construct type")
                            } else {
                                panic!(
                                    "{instantiated_type} can't be constructed for type containing or of {items:?}"
                                )
                            }
                        }
                        Value::PrimitiveType() => panic!("primitive is not callable"),
                        Value::Function {
                            params,
                            returns,
                            body,
                        } => todo!("call function"),
                        Value::Unit => todo!("unit is not callable"),
                    }
                } else {
                    panic!("unknown command {command}")
                }
            }
            _ => todo!(),
        },
        crate::ast::NodeInner::Symbol(symbol) => {
            if let Some(value) = env.get(symbol) {
                value.to_owned()
            } else {
                panic!("unknown symbol {symbol}");
            }
        }
    }
}
