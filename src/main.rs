#![allow(non_snake_case, dead_code)] // Suppress warning because of crate name "OBBDimal" and dead_code for debugging purposes.
mod bdd;

use crate::bdd::bdd_manager::*;
use crate::bdd::bdd_graph::*;

fn main() {
    let mut mgr = BDDManager::new();

    let x1 = Node {top_var: 3, high: Some(Box::new(Node {top_var: 4, high: None, low: None})), low: None};
    let x2 = Node {top_var: 3, high: Some(Box::new(Node {top_var: 5, high: None, low: None})), low: Some(Box::new(Node {top_var: 5, high: None, low: None}))};

    mgr.and(x1, x2);
    
    println!("{:?}", &mgr);
}
