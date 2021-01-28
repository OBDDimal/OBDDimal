#![allow(non_snake_case, dead_code)] // Suppress warning because of crate name "OBBDimal" and dead_code for debugging purposes.
mod bdd;

use crate::bdd::bdd_manager::*;
use crate::bdd::bdd_graph::*;

// x1 * (x2 + x3)

fn main() {
    let mut mgr = BDDManager::new();

    let x1 = Node::new(1, NodeType::ZERO, NodeType::ONE);
    let x2 = Node::new(2, NodeType::ZERO, NodeType::ONE);
    let x3 = Node::new(3, NodeType::ZERO, NodeType::ONE);
    let x4 = Node::new(3, NodeType::ZERO, NodeType::ONE);
    let x5 = Node::new(3, NodeType::ZERO, NodeType::ONE);
    let x2orx3 = mgr.or(x2, x3);
    let x4andx5 = mgr.and(x4, x5);
    let test = mgr.or(x2orx3, x4andx5);
    let bdd = mgr.and(test, x1);
    
    println!("{:?}", mgr.satisfiable(bdd));
}
