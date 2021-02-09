use clap::{App, Arg};
use obbdimal::bdd::bdd_ds::InputFormat;
use obbdimal::bdd::bdd_manager::BddManager;
use obbdimal::input::parser::ParserSettings;

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

    let output_path = if let Some(i) = matches.value_of("OUTPUT") {
        i
    } else {
        ""
    };

    // Read data from specified dimacs file.
    let data = std::fs::read_to_string(path).unwrap();
    // Create a BDD from input data (interpreted as dimacs cnf).
    let mut mgr =
        BddManager::new_from_format(&data, InputFormat::CNF, ParserSettings::default()).unwrap();
    // Calculate the number of variable assignments that evaluate the created BDD to true.
    let sat_count = mgr.sat_count();

    match sat_count {
        Ok(num) => println!("Number of solutions for the BDD: {:?}", num),
        Err(e) => {
            println!("{}", e)
        }
    }

    if output_path != "" {
        match std::fs::write(output_path, mgr.serialize_bdd().unwrap()) {
            Ok(_) => {
                println!("Wrote BDD to path: {}", output_path)
            }
            Err(e) => {
                println!("Couldn't write BDD to file: {}", e)
            }
        }
    }
}
