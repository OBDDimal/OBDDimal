#![allow(non_snake_case, dead_code)] // Suppress warning because of crate name "OBBDimal" and dead_code for debugging purposes.
mod bdd;

use crate::bdd::bdd_manager::*;

fn main() {
    let mgr = BDDManager::new();
    println!("{:?}", mgr);
}
