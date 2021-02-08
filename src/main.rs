use clap::{App, Arg};
use obbdimal::bdd::bdd_ds::InputFormat;
use obbdimal::input::parser::ParserSettings;
use obbdimal::bdd::bdd_manager::BddManager;

fn main() {
    let matches = App::new("OBDDimal")
        .version("0.1")
        .author("Timo Netzer <timo.netzer@uni-ulm.de>")
        .about("A experimental, parallelized BDD library written in Rust.")
        .arg(
            Arg::new("INPUT")
                .short('i')
                .long("input")
                .value_name("FILE")
                .about("Sets the path of the input file to use")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::new("OUTPUT")
                .short('o')
                .long("output")
                .value_name("FILE")
                .about("Sets the path where the output is saved"),
        )
        .get_matches();

    let path = if let Some(i) = matches.value_of("INPUT") {
        i
    } else {
        println!("No input file specified.");
        panic!("No input file specified!");
    };

    // Read data from specified dimacs file.
    let data = std::fs::read_to_string(path).unwrap();
    // Create a BDD from input data (interpreted as dimacs cnf).
    let mut mgr = BddManager::new_from_format(&data, InputFormat::CNF, ParserSettings::default()).unwrap();
    // Calculate the number of variable assignments that evaluate the created BDD to true.
    let sat_count = mgr.sat_count();

    println!("Number of solutions for the BDD: {:?}", sat_count);
}
