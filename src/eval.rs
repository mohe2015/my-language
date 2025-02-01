use crate::ast::{Node, NodeInner};

pub enum Type<'a> {
    And(Vec<Type<'a>>),
    Or(Vec<Type<'a>>),
    Primitive(&'a str),
}

pub fn eval(input: Vec<Node>) {
    let mut primitives = Vec::new();
    for command in &input {
        match &command.inner {
            crate::ast::NodeInner::List(nodes) => match nodes.first() {
                Some(Node {
                    inner: NodeInner::Symbol("define-primitive"),
                    ..
                }) => {
                    println!("defining primitive");
                    primitives.push(match &nodes[1].inner {
                        NodeInner::List(nodes) => todo!(),
                        NodeInner::Symbol(symbol) => *symbol,
                    })
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
                            println!("and type")
                        }
                        NodeInner::Symbol("or") => {
                            println!("or type")
                        }
                        _ => todo!(),
                    }
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
