mod ast;
mod eval;
use std::fs;

use ast::{parse, parse_toplevel};
use eval::eval;

pub fn main() {
    let code = fs::read_to_string("main.myl").unwrap();
    println!("{code}");
    let parsed = parse_toplevel(&code);
    println!("{parsed:#?}");
    eval(parsed);
}