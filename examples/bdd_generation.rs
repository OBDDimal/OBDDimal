use obbdimal::bdd_manager::{BDDManager, InputFormat};

fn main() {
    // Read data from a dimacs file.
    let data = std::fs::read_to_string("examples/assets/easy1.dimacs").unwrap();
    // Create a BDD from input data (interpreted as dimacs cnf).
    let _mgr = BDDManager::from_format(&data, InputFormat::CNF);
}
