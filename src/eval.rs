use crate::ast::{Node, NodeInner};


pub fn eval(input: Vec<Node>) {
    let mut primitives = Vec::new();
    for command in &input {
        match &command.inner {
            crate::ast::NodeInner::List(nodes) => {
                match nodes.first() {
                    Some(Node { inner: NodeInner::Symbol("define-primitive"), .. }) => {
                        println!("defining primitive");
                        primitives.push(match &nodes[1].inner {
                            NodeInner::List(nodes) => todo!(),
                            NodeInner::Symbol(symbol) => *symbol,
                        })
                    }
                    Some(Node { inner: NodeInner::Symbol("define-type"), .. }) => {
                        todo!()
                    }
                    Some(Node { inner: NodeInner::Symbol(command), .. }) => {
                        todo!("unknown command {command}")
                    }
                    _ => todo!()
                }
            },
            crate::ast::NodeInner::Symbol(_) => todo!(),
        }
    }
}