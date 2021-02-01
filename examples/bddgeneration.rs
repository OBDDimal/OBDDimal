use obbdimal::parser;
use obbdimal::bdd_manager::BDDManager;
use obbdimal::boolean_function;

fn main() {
    // should look like: let mgr: BDDManager = BDDManager::from_format(data, format);

    let input_string = std::fs::read_to_string("assets/easy1.dimacs").unwrap(); // Read dimacs cnf from file.
    let cnf_vec = parser::parse_string(&input_string).unwrap();                 // Parse string to vector representation.
    let cnf = boolean_function::BooleanFunction::new_cnf_formula(cnf_vec);      // Create symbols representation from vector.
    let mgr = BDDManager::from_cnf(cnf);                                        // Create BDDManager holding the generated BDD.
}