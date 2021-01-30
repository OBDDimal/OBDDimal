#![allow(dead_code, unused_imports)] // Suppress warning dead_code and unused_imports for debugging purposes.
mod bdd;
mod input;

use crate::bdd::bdd_graph::*;
use crate::bdd::bdd_manager::*;
use crate::input::boolean_function::*;
use crate::input::parser::*;

fn main() {
/*
    //build easy1.dimacs from hand
    let mut mgr = BDDManager::new();
    let x1 = Node::new(1, NodeType::ZERO, NodeType::ONE);
    let x2 = Node::new(2, NodeType::ZERO, NodeType::ONE);
    let x3 = Node::new(3, NodeType::ZERO, NodeType::ONE);
    let x1n = Node::new(1, NodeType::ONE, NodeType::ZERO);
    let x3n = Node::new(3, NodeType::ONE, NodeType::ZERO);

    let x1orx3n = mgr.or(x3n, x1.clone());
    let x2orx3 = mgr.or(x2, x3);
    let x2orx3orx1n = mgr.or(x2orx3, x1n);
    let bdd = mgr.and(x1orx3n, x2orx3orx1n);
*/

    let input = parse_file("examples/quinn.dimacs").unwrap();
    let input_symbols = BooleanFunction::new_cnf_formula(input);
    let mgr = BDDManager::from_cnf(input_symbols);
  
    println!("{:?}", mgr.bdd);
}
