#![allow(non_snake_case, dead_code)] // Suppress warning because of crate name "OBBDimal" and dead_code for debugging purposes.
mod bdd;

use crate::bdd::bdd_manager::*;
use crate::bdd::bdd_graph::*;

// x1 * (x2 + x3)

// x1 * !x1

fn main() {
    let mut mgr = BDDManager::new();
    
    let bdd = NodeType::COMPLEX(
        Node {
            top_var: 1,
            low: Box::new(NodeType::ZERO),
            high: Box::new(NodeType::COMPLEX(
                Node {
                    top_var: 2,
                    low: Box::new(NodeType::COMPLEX(
                        Node {
                            top_var: 3,
                            low: Box::new(NodeType::ZERO),
                            high: Box::new(NodeType::ONE),
                        })),
                    high: Box::new(NodeType::ONE),
                }))
        });

    println!("{:?}", mgr.satisfiable(bdd));
}
