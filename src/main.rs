#![allow(dead_code)] // Suppress warning because of crate name "OBBDimal" and dead_code for debugging purposes.
mod bdd;
mod input;

use crate::bdd::bdd_manager::*;
use crate::bdd::bdd_graph::*;
use crate::input::parser::*;
use crate::input::boolean_function::*;

fn main() {   
    let input = parse_file("debug_input/easy1.dimacs").unwrap();
    println!("{:?}", BooleanFunction::new_cnf_formula(input));
}
