use std::time::Instant;

use clap::{App, Arg};
use obbdimal::bdd::bdd_manager::BddManager;
use obbdimal::input::parser::ParserSettings;
use obbdimal::{bdd::bdd_ds::InputFormat, input::static_ordering::StaticOrdering};

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
                .about("Path to the input file (currently only dimacs cnf files are supported)")
                .takes_value(true)
                .required_unless_present("LOAD"),
        )
        .arg(
            Arg::new("LOAD")
                .short('l')
                .long("load")
                .value_name("FILE")
                .about("Path to the input file of a previously saved BDD")
                .takes_value(true)
                .required_unless_present("INPUT"),
        )
        .arg(
            Arg::new("STATIC VARIABLE ORDERING")
                .short('s')
                .long("static")
                .about("Static variable heuristic which is applied before the BDD is generated. Currently supported: NONE, FORCE")
                .takes_value(true),
        )
        .arg(
            Arg::new("OUTPUT")
                .short('o')
                .long("output")
                .value_name("FILE")
                .takes_value(true)
                .about("Path where the serialized BDD should be saved"),
        )
        .arg(
            Arg::new("TIMER")
                .short('t')
                .long("timer")
                .about("Also prints how long the program was running (Not very exact for short time periods)"),
        )
        .arg(
            Arg::new("VERBOSE")
                .short('v')
                .long("verbose")
                .about("Prints the selected options before running the calculations")
        )
        .get_matches();

    match matches.value_of("LOAD") {
        Some(i) => {
            let data = std::fs::read_to_string(i).unwrap();
            let mgr = BddManager::new();
            let mut mgr = mgr.deserialize_bdd(&data);
            println!("Loaded BDD got {} solutions.", mgr.sat_count().unwrap());
            return;
        }
        None => {}
    }

    let path = match matches.value_of("INPUT") {
        Some(i) => i,
        None => {
            println!("No input file specified.");
            panic!("No input file specified!");
        }
    };

    let mut selected_output_path = "NONE";

    let output_path = match matches.value_of("OUTPUT") {
        Some(i) => {
            selected_output_path = i;
            selected_output_path
        }
        None => "",
    };

    // Read data from specified dimacs file.
    let data = std::fs::read_to_string(path).unwrap();
    // Create a BDD from input data (interpreted as dimacs cnf).

    let mut selected_static_ordering = "NONE";

    let static_ordering = match matches.value_of("STATIC VARIABLE ORDERING") {
        Some("FORCE") => {
            selected_static_ordering = "FORCE";
            StaticOrdering::FORCE
        }
        _ => StaticOrdering::NONE,
    };

    if matches.is_present("VERBOSE") {
        println!("Selected input path: {}\nSelected output path: {}\nSelected static variable ordering: {}\nSelected timer state: {}\n", path, selected_output_path, selected_static_ordering, matches.is_present("TIMER"));
    }

    let timer = Instant::now();

    let mut mgr = BddManager::new_from_format(
        &data,
        InputFormat::CNF,
        ParserSettings::default(),
        static_ordering,
    )
    .unwrap();
    // Calculate the number of variable assignments that evaluate the created BDD to true.
    let sat_count = mgr.sat_count();

    match sat_count {
        Ok(num) => {
            println!("Number of solutions for the BDD: {:?}", num);
            if matches.is_present("TIMER") {
                println!("It took {:?} to complete.", timer.elapsed());
            }
        }
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
