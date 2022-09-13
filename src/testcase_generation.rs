use bitvec::prelude::*;
use rand::Rng;
use obddimal::bdd_manager::hash_select::HashMap;

/// This prints the one-columns of a random truth table with 8 variables,
/// for testcase generation. The result is to be used in src/bdd_manager/test.rs.
fn main() {
    let mut truthtable: HashMap<u8, bool> = Default::default();

    let mut rng = rand::thread_rng();

    for i in 0u8..=255u8 {
        let f: bool = rng.gen();
        truthtable.insert(i, f);
    }

    for (k, v) in truthtable {
        if v {
            println!("{},", k.view_bits::<Lsb0>())
        }
    }
}
