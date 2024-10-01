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
}

fn evaluate_slicing(folder_path: &str) {
    for example in ["automotive01", "automotive02v4"].iter() {
        for n in 0..1000 {
            // Prepare
            let mut bdd =
                BddView::load_from_dddmp_file(format!("examples/{}.dimacs.dddmp", example))
                    .unwrap()[0]
                    .clone();
            let mut varids =
                var2level_to_ordered_varids(&bdd.get_manager().read().unwrap().var2level);
            varids.shuffle(&mut thread_rng());
            let mut result_writer =
                Writer::from_path(format!("{}/slicing-{}-{:03}.csv", folder_path, example, n))
                    .unwrap();
            // Measure
            for var_id in varids.iter() {
                let size_before = bdd.count_nodes();
                let remove_vars = [*var_id].into_iter().collect::<HashSet<_>>();
                let time = Instant::now();
                bdd = bdd.create_slice_without_vars(&remove_vars);
                let elapsed = time.elapsed();
                let size_after = bdd.count_nodes();
                // Store result
                result_writer
                    .serialize(SlicingMeasurement {
                        sliced_variable: *var_id,
                        time_in_seconds: elapsed.as_secs_f64(),
                        size_before,
                        size_after,
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
        "automotive02v1",
        "automotive02v2",
        "automotive02v3",
        "automotive02v4",
    ]
    .iter()
    {
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
