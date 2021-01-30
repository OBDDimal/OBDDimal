#![allow(dead_code, unused_imports)] // Suppress warning dead_code and unused_imports for debugging purposes.
mod bdd;
mod input;

use crate::bdd::bdd_graph::*;
use crate::bdd::bdd_manager::*;
use crate::input::boolean_function::*;
use crate::input::parser::*;

fn main() {
    //build easy1.dimacs from hand
    /*let mut mgr = BDDManager::new();
        let x1 = Node::new(1, NodeType::Zero, NodeType::One);
        let x2 = Node::new(2, NodeType::Zero, NodeType::One);
        let x3 = Node::new(3, NodeType::Zero, NodeType::One);
        let x1n = Node::new(1, NodeType::One, NodeType::Zero);
        let x3n = Node::new(3, NodeType::One, NodeType::Zero);

        let x1orx3n = mgr.or(x3n, x1.clone());
        let x2orx3 = mgr.or(x2, x3);
        let x2orx3orx1n = mgr.or(x2orx3, x1n);
        let bdd = mgr.and(x1orx3n, x2orx3orx1n);
    */

    let input = parse_string(&std::fs::read_to_string("examples/berkeleydb.dimacs").unwrap()).unwrap();
    let input_symbols = BooleanFunction::new_cnf_formula(input);
    let mgr = BDDManager::from_cnf(input_symbols);

    println!("{:#?}", mgr.bdd);
}
