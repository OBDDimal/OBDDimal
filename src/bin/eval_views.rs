//! Some Benchmarks to evaluate some usecases of Views.
use std::{
    env, fs,
    path::Path,
    process,
    time::{Instant, SystemTime},
};

use csv::Writer;
use obddimal::{
    core::{bdd_node::VarID, order::var2level_to_ordered_varids},
    misc::hash_select::HashSet,
    views::bdd_view::BddView,
};
use rand::{seq::SliceRandom, thread_rng};

pub fn main() {
    // Create place to store the results:
    let folder_path = format!(
        "eval_views/results-{}",
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );
    if Path::new(&folder_path).exists() {
        println!("Results folder already exists?!");
        process::exit(1);
    }
    fs::create_dir_all(&folder_path).unwrap();

    // Run selected tests:
    let cmd_args: HashSet<String> = env::args().collect();
    let all = cmd_args.contains("--all") || cmd_args.len() == 1;

    if all || cmd_args.contains("--slicing") {
        evaluate_slicing(&folder_path);
    }

    if all || cmd_args.contains("--atomic-sets") {
        evaluate_atomic_sets(&folder_path);
    }
}

#[derive(serde::Serialize)]
struct SlicingMeasurement {
    sliced_variable: VarID,
    time_in_seconds: f64,
    size_before: usize,
    size_after: usize,
    nodes_in_manager_before: usize,
    nodes_in_manager_after: usize,
}

fn evaluate_slicing(folder_path: &str) {
    for example in [
        "automotive01",
        "automotive02_v4",
        //"sandwich",
    ]
    .iter()
    {
        const ITERATION_COUNT: usize = 1000;
        for n in 0..ITERATION_COUNT {
            println!(
                "Slicing {} (iteration {}/{}).",
                example,
                n + 1,
                ITERATION_COUNT
            );
            // Prepare
            let mut bdds =
                vec![
                    BddView::load_from_dddmp_file(format!("examples/{}.dimacs.dddmp", example))
                        .unwrap()[0]
                        .clone(),
                ];
            let mut varids =
                var2level_to_ordered_varids(&bdds[0].get_manager().read().unwrap().var2level);
            varids.shuffle(&mut thread_rng());
            let mut result_writer =
                Writer::from_path(format!("{}/slicing-{}-{:03}.csv", folder_path, example, n))
                    .unwrap();
            // Measure
            for var_id in varids.iter() {
                let size_before = bdds[bdds.len() - 1].count_nodes();
                let nodes_in_manager_before = bdds[0].get_manager().read().unwrap().nodes.len();
                let remove_vars = [*var_id].into_iter().collect::<HashSet<_>>();
                let time = Instant::now();
                bdds.push(bdds[bdds.len() - 1].create_slice_without_vars(&remove_vars));
                let elapsed = time.elapsed();
                let size_after = bdds[bdds.len() - 1].count_nodes();
                let nodes_in_manager_after = bdds[0].get_manager().read().unwrap().nodes.len();
                // Store result
                result_writer
                    .serialize(SlicingMeasurement {
                        sliced_variable: *var_id,
                        time_in_seconds: elapsed.as_secs_f64(),
                        size_before,
                        size_after,
                        nodes_in_manager_before,
                        nodes_in_manager_after,
                    })
                    .unwrap();
            }
        }
    }
}

#[derive(serde::Serialize)]
struct AtomicSetsMeasurement {
    bdd: String,
    time_in_seconds: f64,
    size_before: usize,
    size_after: usize,
}

fn evaluate_atomic_sets(folder_path: &str) {
    let mut result_writer = Writer::from_path(format!("{}/atomic_sets.csv", folder_path)).unwrap();
    for example in [
        "sandwich",
        "berkeleydb",
        "embtoolkit",
        "busybox_1.18.0",
        "financialservices01",
        "automotive01",
        "automotive02_v1",
        "automotive02_v2",
        "automotive02_v3",
        "automotive02_v4",
    ]
    .iter()
    {
        println!("Optimizing {}.", example);
        let bdds =
            BddView::load_from_dddmp_file(format!("examples/{}.dimacs.dddmp", example)).unwrap();
        for i in 0..bdds.len() {
            let bdd = bdds[i].clone();
            // Measure
            let size_before = bdd.count_nodes();
            let time = Instant::now();
            let bdd = bdd.optimize_through_atomic_sets().unwrap();
            let elapsed = time.elapsed();
            let size_after = bdd.count_nodes();
            // Store result
            let bdd_name = if bdds.len() == 1 {
                example.to_string()
            } else {
                format!("{}[{}]", example, i)
            };
            result_writer
                .serialize(AtomicSetsMeasurement {
                    bdd: bdd_name,
                    time_in_seconds: elapsed.as_secs_f64(),
                    size_before,
                    size_after,
                })
                .unwrap();
        }
    }
}
