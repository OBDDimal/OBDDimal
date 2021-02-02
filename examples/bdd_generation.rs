use obbdimal::bdd_manager::{BDDManager, InputFormat};

fn main() {
    // Read data from a dimacs file.
    let data = std::fs::read_to_string("examples/assets/easy1.dimacs").unwrap();
    // Create a BDD from input data (interpreted as dimacs cnf).
    let mgr = BDDManager::from_format(&data, InputFormat::CNF);
    // Calculate the number of variable assignments that evaluate the created BDD to true.
    let _sat_count = mgr.satcount();
}
