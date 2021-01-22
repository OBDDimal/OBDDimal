#![allow(non_snake_case)] // Suppress warning because of crate name "OBBDimal".

mod bdd;
use crate::bdd::bdd_manager::*;

fn main() {
    println!("{}", get_file_name());
}
