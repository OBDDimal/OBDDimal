use obbdimal::input::parser::ParserSettings;
use obbdimal::{
    bdd::{bdd_ds::InputFormat, bdd_manager::BddManager},
    input::static_ordering::StaticOrdering,
};

fn main() {
    // Read data from a dimacs file.
    let data = std::fs::read_to_string("examples/assets/easy1.dimacs").unwrap();
    // Create a BDD from input data (interpreted as dimacs cnf).
    let mut mgr = BddManager::new_from_format(
        &data,
        InputFormat::CNF,
        ParserSettings::default(),
        StaticOrdering::NONE,
    )
    .unwrap();
    // Calculate the number of variable assignments that evaluate the created BDD to true.
    let sat_count = mgr.sat_count();

    println!("sat_count of easy1.dimacs: {:?}", sat_count);
}
