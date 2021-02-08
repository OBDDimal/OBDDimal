use obbdimal::bdd::bdd_ds::{Bdd, InputFormat};
use obbdimal::input::parser::ParserSettings;

fn main() {
    // Read data from a dimacs file.
    let data = std::fs::read_to_string("examples/assets/easy1.dimacs").unwrap();
    // Create a BDD from input data (interpreted as dimacs cnf).
    let mgr = Bdd::from_format(&data, InputFormat::CNF, ParserSettings::default()).unwrap();
    // Calculate the number of variable assignments that evaluate the created BDD to true.
    let _sat_count = mgr.satcount();
}
