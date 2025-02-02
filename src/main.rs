mod ast;
mod eval;
use std::{collections::HashMap, fs};

use ast::{parse, parse_toplevel};
use eval::eval;

pub fn main() {
    let code = fs::read_to_string("main.myl").unwrap();
    //println!("{code}");
    let parsed = parse_toplevel(&code);
    //println!("{parsed:#?}");
    let mut env = HashMap::new();
    for command in &parsed {
        println!("{:?}", eval(&command, &mut env));
    }
}
