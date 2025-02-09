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
    env.insert("set".to_owned(), Value::SetBuiltin);
    env.insert("define-primitive".to_owned(), Value::DefinePrimitiveBuiltin);
    env.insert("define-type".to_owned(), Value::DefineTypeBuiltin);
    env.insert("define-function".to_owned(), Value::DefineFunctionBuiltin);
    env.insert("nth".to_owned(), Value::NthBuiltin);
    env.insert("if=".to_owned(), Value::IfEqBuiltin);
    env.insert("let".to_owned(), Value::LetBuiltin);
    for command in &parsed {
        println!("{:?}", eval(&command, &mut env));
    }
    for line in std::io::stdin().lines() {
        let line = line.unwrap();
        let commands = parse_toplevel(&line);
        for command in commands {
            println!("{:?}", eval(&command, &mut env));
        }
    }
}
