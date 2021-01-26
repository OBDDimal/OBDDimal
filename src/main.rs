#![allow(non_snake_case, dead_code)] // Suppress warning because of crate name "OBBDimal" and dead_code for debugging purposes.
mod bdd;

use crate::bdd::bdd_manager::*;
use crate::bdd::bdd_graph::*;

fn main() {
    let mut mgr = BDDManager::new();
    mgr.make_node(0, mgr.bdd_true(), mgr.bdd_false());
    mgr.make_node(0, mgr.bdd_true(), mgr.bdd_false());
    println!("{:?}", &mgr);
}
