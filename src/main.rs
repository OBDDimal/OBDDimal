use std::time::Instant;

use clap::{load_yaml, App};
use obbdimal::bdd::bdd_manager::{BddManager};
use obbdimal::bdd::bdd_para_manager::BddParaManager;
use obbdimal::input::parser::ParserSettings;
use obbdimal::{bdd::bdd_ds::InputFormat, input::static_ordering::StaticOrdering};

fn main() {
    let data = std::fs::read_to_string("./examples/assets/sandwich.dimacs").unwrap();

    let mut mgr = BddParaManager::new_parallelized(
        &data,
        InputFormat::CNF,
        ParserSettings::default(),
        StaticOrdering::FORCE,
    );

    println!("Parallelized calculated #SAT: {}", mgr.sat_count().unwrap());
    
    todo!();
    let yaml = load_yaml!("clap_config.yaml");
    let matches = App::from(yaml).get_matches();
    
    match matches.value_of("load") {
        Some(i) => {
            let data = std::fs::read_to_string(i).unwrap();
            let mgr = BddManager::new();
            let mut mgr = mgr.deserialize_bdd(&data);
            println!("Loaded BDD got {} solutions.", mgr.sat_count().unwrap());
            return;
        }
        None => {}
    }

    let path = match matches.value_of("input") {
        Some(i) => i,
        None => {
            println!("No input file specified.");
            panic!("No input file specified!");
        }
    };

    let mut selected_output_path = "NONE";

    let output_path = match matches.value_of("output") {
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

    let static_ordering = match matches.value_of("preorder") {
        Some("FORCE") => {
            selected_static_ordering = "FORCE";
            StaticOrdering::FORCE
        }
        _ => StaticOrdering::NONE,
    };

    if matches.is_present("verbose") {
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
    println!("Node Count = {:?}!", mgr.node_count());

    match sat_count {
        Ok(num) => {
            println!("Number of solutions for the BDD: {:?}", num);
            if matches.is_present("timer") {
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
