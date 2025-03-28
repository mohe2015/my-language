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
    env.insert("nth", Value::NthBuiltin);
    env.insert("if=", Value::IfEqBuiltin);
    env.insert("let", Value::LetBuiltin);
    for command in &parsed {
        println!("{:?}", eval(&command, &mut env));
    }
    for line in std::io::stdin().lines() {
        // TODO FIXME
        let line: &'static mut str = line.unwrap().leak();
        let commands = parse_toplevel(line).leak();
        for command in commands {
            println!("{:?}", eval(command, &mut env));
        }
    }
}
