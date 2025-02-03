use std::{
    collections::HashMap,
    sync::atomic::{AtomicU64, Ordering},
};

use crate::ast::{Node, NodeInner};

#[derive(Debug)]
pub enum Type<'a> {
    And(Vec<&'a str>),
    Or(Vec<&'a str>),
    Primitive(),
}

#[derive(Debug, Clone)]
pub enum Value<'a> {
    PrimitiveType(u64), // global id
    AndType(Vec<&'a str>),
    OrType(Vec<&'a str>),
    Function {
        /// (name type)
        params: Vec<(&'a str, &'a str)>,
        body: &'a Node<'a>,
    },
    Unit, // TODO special primitivetype?
    OrInstance {
        // TODO FIXME
        typ: Box<Value<'a>>,
        value: Box<Value<'a>>,
    },
    AndInstance {
        typ: Box<Value<'a>>,
        value: Vec<Value<'a>>,
    },
    PrimitiveInstance(u64),
}

static PRIMITIVE_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn eval<'a>(input: &'a Node<'a>, env: &mut HashMap<&'a str, Value<'a>>) -> Value<'a> {
    match &input.inner {
        crate::ast::NodeInner::List(nodes) => match nodes.first() {
            Some(Node {
                inner: NodeInner::Symbol("define-primitive"),
                ..
            }) => {
                assert_eq!(nodes.len(), 1);
                Value::PrimitiveType(PRIMITIVE_COUNTER.fetch_add(1, Ordering::Relaxed))
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
                assert_eq!(nodes.len(), 3);
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
                Value::Function { params, body }
            }
            Some(Node {
                inner: NodeInner::Symbol("set"),
                ..
            }) => {
                assert_eq!(nodes.len(), 3);
                let name: &str = (&nodes[1]).try_into().unwrap();
                let value = &nodes[2];
                let value = eval(value, &mut env.clone());
                println!("set {name} {value:?}");
                env.insert(name, value);
                Value::Unit
            }
            Some(Node {
                inner: NodeInner::Symbol(command),
                ..
            }) => {
                if let Some(typ) = env.get(command) {
                    match typ {
                        Value::AndType(items) => {
                            let and_instances: Vec<Value<'a>> = nodes[1..]
                                .iter()
                                .map(|elem| eval(&elem, &mut env.clone()))
                                .collect();
                            Value::AndInstance {
                                typ: Box::new(typ.to_owned()),
                                value: and_instances,
                            }
                        }
                        Value::OrType(items) => {
                            let instantiated_type = eval(&nodes[1], &mut env.clone());
                            Value::OrInstance {
                                typ: Box::new(typ.to_owned()),
                                value: Box::new(instantiated_type),
                            }
                        }
                        Value::PrimitiveType(_) => panic!("primitive is not callable"),
                        Value::Function { params, body } => {
                            let actual_params: Vec<Value<'a>> = nodes[1..]
                                .iter()
                                .map(|elem| eval(&elem, &mut env.clone()))
                                .collect();
                            assert_eq!(actual_params.len(), params.len());
                            let mut env = env.clone();
                            params.iter().zip(actual_params).for_each(|(elem, value)| {
                                env.insert(elem.0, value);
                            });
                            eval(body, &mut env)
                        }
                        Value::Unit => todo!("unit is not callable"),
                        Value::OrInstance { typ, value } => todo!(),
                        Value::PrimitiveInstance(_) => todo!(),
                        Value::AndInstance { typ, value } => todo!(),
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
