mod ast;
mod eval;
use std::{collections::HashMap, fs};

use ast::{parse, parse_toplevel};
use eval::{Value, eval};

pub fn main() {
    let code = fs::read_to_string("main.myl").unwrap();
    //println!("{code}");
    let parsed = parse_toplevel(&code);
    //println!("{parsed:#?}");
    let mut env = HashMap::new();
    env.insert("set", Value::SetBuiltin);
    env.insert("define-primitive", Value::DefinePrimitiveBuiltin);
    env.insert("define-type", Value::DefineTypeBuiltin);
    env.insert("define-function", Value::DefineFunctionBuiltin);
    for command in &parsed {
        println!("{:?}", eval(&command, &mut env));
    }
}
