use std::{
    collections::HashMap,
    sync::atomic::{AtomicU64, Ordering},
};

use crate::ast::{Node, NodeInner};

#[derive(Debug)]
pub enum Type {
    And(Vec<String>),
    Or(Vec<String>),
    Primitive(),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    PrimitiveType(u64), // global id
    AndType(Vec<String>),
    OrType(Vec<String>),
    Function {
        /// (name type)
        params: Vec<(String, String)>,
        body: Node,
    },
    Unit,
    OrInstance {
        typ: Box<Value>,
        value: Box<Value>,
    },
    AndInstance {
        typ: Box<Value>,
        value: Vec<Value>,
    },
    DefineFunctionBuiltin,
    DefinePrimitiveBuiltin,
    DefineTypeBuiltin,
    SetBuiltin,
    NthBuiltin,
    IfEqBuiltin,
    LetBuiltin,
}

impl Value {
    pub fn into_value(&self) -> &Value {
        match self {
            Value::OrInstance { typ, value } => value,
            other => other,
        }
    }
}

// TODO implement let in some other way

static PRIMITIVE_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn eval(input: &Node, env: &mut HashMap<String, Value>) -> Value {
    match &input.inner {
        crate::ast::NodeInner::List(nodes) => {
            let first = eval(nodes.first().unwrap(), env);
            match &first {
                Value::AndType(items) => {
                    let and_instances: Vec<Value> = nodes[1..]
                        .iter()
                        .map(|elem| eval(&elem, &mut env.clone()))
                        .collect();
                    Value::AndInstance {
                        typ: Box::new(first.to_owned()),
                        value: and_instances,
                    }
                }
                Value::OrType(items) => {
                    let instantiated_type = eval(&nodes[1], &mut env.clone());
                    Value::OrInstance {
                        typ: Box::new(first.to_owned()),
                        value: Box::new(instantiated_type),
                    }
                }
                Value::PrimitiveType(_) => panic!("primitive is not callable"),
                Value::Function { params, body } => {
                    let actual_params: Vec<Value> = nodes[1..]
                        .iter()
                        .map(|elem| eval(&elem, &mut env.clone()))
                        .collect();
                    assert_eq!(
                        actual_params.len(),
                        params.len(),
                        "{actual_params:?} does not match {params:?}"
                    );
                    let mut env = env.clone();
                    params.iter().zip(actual_params).for_each(|(elem, value)| {
                        env.insert(elem.0.clone(), value);
                    });
                    eval(body, &mut env)
                }
                Value::Unit => todo!("unit is not callable"),
                Value::OrInstance { typ, value } => todo!(),
                Value::AndInstance { typ, value } => todo!(),
                Value::DefineFunctionBuiltin => {
                    assert_eq!(nodes.len(), 3);
                    let params: &Vec<Node> = (&nodes[1]).try_into().unwrap();
                    let params: Vec<(String, String)> = params
                        .iter()
                        .map(|elem| match &elem.inner {
                            NodeInner::List(list) => (
                                <&str>::try_from(&list[0]).unwrap().to_owned(),
                                <&str>::try_from(&list[1]).unwrap().to_owned(),
                            ),
                            NodeInner::Symbol(_) => todo!(),
                        })
                        .collect();
                    let body = nodes[2].clone();
                    Value::Function { params, body }
                }
                Value::DefinePrimitiveBuiltin => {
                    assert_eq!(nodes.len(), 1);
                    Value::PrimitiveType(PRIMITIVE_COUNTER.fetch_add(1, Ordering::Relaxed))
                }
                Value::DefineTypeBuiltin => {
                    assert_eq!(nodes.len(), 2);
                    let definition: &Vec<Node> = (&nodes[1]).try_into().unwrap();
                    match &definition[0].inner {
                        NodeInner::Symbol(s) if s == "and" => {
                            let and_types: Vec<String> = definition[1..]
                                .iter()
                                .map(|elem| match &elem.inner {
                                    NodeInner::List(nodes) => todo!(),
                                    NodeInner::Symbol(name) => name.clone(),
                                })
                                .collect();
                            Value::AndType(and_types)
                        }
                        NodeInner::Symbol(s) if s == "or" => {
                            let and_types: Vec<String> = definition[1..]
                                .iter()
                                .map(|elem| match &elem.inner {
                                    NodeInner::List(nodes) => todo!(),
                                    NodeInner::Symbol(name) => name.clone(),
                                })
                                .collect();
                            Value::OrType(and_types)
                        }
                        _ => todo!(),
                    }
                }
                Value::SetBuiltin => {
                    assert_eq!(nodes.len(), 3);
                    let name: &str = (&nodes[1]).try_into().unwrap();
                    let value = &nodes[2];
                    let value = eval(value, &mut env.clone());
                    println!("set {name} {value:?}");
                    env.insert(name.to_owned(), value);
                    Value::Unit
                }
                Value::NthBuiltin => {
                    assert_eq!(nodes.len(), 3);
                    let value = eval(&nodes[1], &mut env.clone());
                    if let Value::AndInstance { typ, value } = value {
                        let index: &str = (&nodes[2]).try_into().unwrap();
                        value[index.parse::<usize>().unwrap()].clone()
                    } else {
                        panic!("nth can only be called on instances of and types")
                    }
                }
                Value::IfEqBuiltin => {
                    assert_eq!(nodes.len(), 5, "{:?}", nodes);
                    let lhs = eval(&nodes[1], &mut env.clone());
                    let rhs = eval(&nodes[2], &mut env.clone());
                    let true_body = &nodes[3];
                    let false_body = &nodes[4];
                    //println!("{lhs:?} == {rhs:?}");
                    if lhs.into_value() == rhs.into_value() {
                        //println!("true");
                        eval(true_body, env)
                    } else {
                        //println!("false");
                        eval(false_body, env)
                    }
                }
                Value::LetBuiltin => {
                    assert_eq!(nodes.len(), 4, "{:?}", nodes);
                    let binding: &Vec<Node> = (&nodes[1]).try_into().unwrap();
                    let name: &str = (&binding[0]).try_into().unwrap();
                    let bound_value = eval(&nodes[2], &mut env.clone());
                    let mut env = env.clone();
                    env.insert(name.to_owned(), bound_value);
                    eval(&nodes[3], &mut env)
                }
            }
        }
        crate::ast::NodeInner::Symbol(symbol) => {
            if let Some(value) = env.get(symbol) {
                value.to_owned()
            } else {
                panic!("unknown symbol {symbol}");
            }
        }
    }
}
