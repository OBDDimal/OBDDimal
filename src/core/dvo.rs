//! Implementation of dynamic variable ordering techniques
#![allow(rustdoc::private_intra_doc_links)]

pub mod area_generation;
pub mod dvo_schedules;
pub mod dvo_strategies;

use core::panic;
use std::{io::stdout, isize, ops::RangeBounds, sync::Arc, time::Instant, usize};

use area_generation::{AreaSelection, ThresholdMethod};
use crossterm::{cursor, execute};
use futures::future;
use indicatif::ProgressBar;
use itertools::Itertools;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use tokio::{runtime::Runtime, task::JoinHandle};

use super::{
    bdd_manager::DDManager,
    bdd_node::{NodeID, VarID, ZERO},
    order::var2level_to_ordered_varids,
    swap::SwapContext,
};
use crate::if_some;

impl DDManager {
    /// Find the variable at specified level
    pub fn var_at_level(&self, level: usize) -> Option<VarID> {
        self.var2level
            .iter()
            .enumerate()
            .find(|(_, &l)| l == level)
            .map(|(v, _)| VarID(v))
    }

    /// Swap layer containing specified variable first to the bottom of the BDD, then to the top,
    /// and then to the position which resulted in smallest BDD size.
    /// Optional parameter `max_increase` stops swapping in either direction once the difference
    /// between BDD size and current optimum exceeds threshold.
    #[allow(unused)]
    fn sift_single_var(
        &mut self,
        var: VarID,
        max_increase: Option<usize>,
        mut f: NodeID,
    ) -> NodeID {
        let starting_pos = self.var2level[var.0];

        let mut best_position = starting_pos;
        let mut best_graphsize = self.count_active(f);

        let root_level = self.var2level[self.nodes.get(&f).unwrap().var.0];

        log::info!(
            "Sifting variable {:?}, starting from level {} (graph size {}).",
            var,
            starting_pos,
            best_graphsize
        );

        // Move variable to the bottom
        let terminal_node_level = self.var2level[ZERO.var.0];

        let mut current_level = starting_pos;

        log::info!("Moving down...");

        for level in starting_pos + 1..terminal_node_level {
            log::info!("Trying level {}", level);
            // Swap var at level-1 (our variable) with var at level
            f = self.swap(
                self.var_at_level(level - 1).unwrap(),
                self.var_at_level(level).unwrap(),
                f,
            );
            current_level += 1;

            let new_size = self.count_active(f);
            log::info!(" Size is {}", new_size);

            if new_size < best_graphsize {
                log::info!(
                    " New optimum found with order {:?}",
                    var2level_to_ordered_varids(&self.var2level)
                );
                self.purge_retain(f);
                best_graphsize = new_size;
                best_position = level;
            } else if let Some(max) = max_increase {
                if new_size > best_graphsize + max {
                    // Do not continue moving downwards, because the graph has grown too much
                    break;
                }
            }
        }

        // Level is now bottom (terminal-1).
        log::info!("Moving up...");

        // Swap back to initial position, without calculating size
        for level in (starting_pos..current_level).rev() {
            f = self.swap(
                self.var_at_level(level).unwrap(),
                self.var_at_level(level + 1).unwrap(),
                f,
            );
            current_level -= 1;
        }

        assert_eq!(current_level, starting_pos);

        // Move variable to the top
        for level in ((root_level + 1)..starting_pos).rev() {
            log::info!("Trying level {}", level);
            // Swap var at level+1 (our variable) with var at level
            f = self.swap(
                self.var_at_level(level).unwrap(),
                self.var_at_level(level + 1).unwrap(),
                f,
            );
            current_level -= 1;

            let new_size = self.count_active(f);
            log::info!(" Size is {}", new_size);

            if new_size < best_graphsize {
                log::info!(
                    " New optimum found with order {:?}",
                    var2level_to_ordered_varids(&self.var2level)
                );
                self.purge_retain(f);
                best_graphsize = new_size;
                best_position = level;
            } else if let Some(max) = max_increase {
                if new_size > best_graphsize + max {
                    // Do not continue moving upwards, because the graph has grown too much
                    break;
                }
            }
        }

        // Level is now top (1). Move variable down to best location

        log::info!(
            "The best result was graph size of {} at level {}. Moving there...",
            best_graphsize,
            best_position
        );

        for level in current_level + 1..best_position + 1 {
            // Swap var at level-1 (our variable) with var at level
            f = self.swap(
                self.var_at_level(level - 1).unwrap(),
                self.var_at_level(level).unwrap(),
                f,
            );
            current_level += 1;
        }

        assert_eq!(current_level, best_position);

        log::info!("Size is now  {}", self.count_active(f));

        f
    }

    /// Swap layer containing specified variable first to the bottom of the BDD, then to the top,
    /// and then to the position which resulted in smallest BDD size.
    /// Optional parameter `max_increase` stops swapping in either direction once the difference
    /// between BDD size and current optimum exceeds threshold.
    #[allow(unused)]
    fn sift_single_var_faster(
        &mut self,
        var: VarID,
        max_increase: Option<usize>,
        mut f: NodeID,
    ) -> NodeID {
        let starting_pos = self.var2level[var.0];
        println!("Starting pos: {} for {}", starting_pos, var.0);

        let mut best_position = starting_pos;
        let mut best_graphsize: isize = 0;
        self.purge_retain(f);

        let mut evaluation: isize = 0;

        let root_level = self.var2level[self.nodes.get(&f).unwrap().var.0];

        log::info!(
            "Sifting variable {:?}, starting from level {} (graph size {}).",
            var,
            starting_pos,
            best_graphsize
        );

        // Move variable to the bottom
        let terminal_node_level = self.var2level[ZERO.var.0];

        let mut current_level = starting_pos;

        log::info!("Moving down...");

        for level in (starting_pos + 1)..terminal_node_level {
            log::info!("Trying level {}", level);
            // Swap var at level-1 (our variable) with var at level
            evaluation += self.direct_swap(
                self.var_at_level(level - 1).unwrap(),
                self.var_at_level(level).unwrap(),
                f,
            );
            current_level += 1;

            log::info!(" Size is {}", evaluation);

            if evaluation < best_graphsize {
                log::info!(
                    " New optimum found with order {:?}",
                    var2level_to_ordered_varids(&self.var2level)
                );
                best_graphsize = evaluation;
                best_position = level;
            } else if let Some(max) = max_increase {
                if evaluation > best_graphsize + max as isize {
                    // Do not continue moving downwards, because the graph has grown too much
                    break;
                }
            }
        }

        // Level is now bottom (terminal-1).
        log::info!("Moving up...");

        // Swap back to initial position, without calculating size
        for level in (starting_pos..current_level).rev() {
            evaluation += self.direct_swap(
                self.var_at_level(level).unwrap(),
                self.var_at_level(level + 1).unwrap(),
                f,
            );
            current_level -= 1;
        }

        assert_eq!(current_level, starting_pos);
        assert_eq!(evaluation, 0);

        // Move variable to the top
        for level in ((root_level + 1)..starting_pos).rev() {
            log::info!("Trying level {}", level);
            // Swap var at level+1 (our variable) with var at level
            evaluation += self.direct_swap(
                self.var_at_level(level).unwrap(),
                self.var_at_level(level + 1).unwrap(),
                f,
            );
            current_level -= 1;

            log::info!(" Size is {}", evaluation);

            if evaluation < best_graphsize {
                log::info!(
                    " New optimum found with order {:?}",
                    var2level_to_ordered_varids(&self.var2level)
                );
                best_graphsize = evaluation;
                best_position = level;
            } else if let Some(max) = max_increase {
                if evaluation > best_graphsize + max as isize {
                    // Do not continue moving upwards, because the graph has grown too much
                    break;
                }
            }
        }

        // Level is now top (1). Move variable down to best location

        log::info!(
            "The best result was graph size of {} at level {}. Moving there...",
            best_graphsize,
            best_position
        );
        println!(
            "The best result was graph size of {} at level {}. Moving there...",
            best_graphsize, best_position
        );

        for level in current_level + 1..=best_position {
            // Swap var at level-1 (our variable) with var at level
            evaluation += self.direct_swap(
                self.var_at_level(level - 1).unwrap(),
                self.var_at_level(level).unwrap(),
                f,
            );
            current_level += 1;
        }

        assert_eq!(evaluation, best_graphsize);
        assert_eq!(current_level, best_position);
        self.purge_retain(f);

        log::info!("Size is now  {}", self.count_active(f));

        f
    }

    /// Perform sifting for every layer containing at least one variable.
    /// If `progressbar` is `true`, display a progress bar in the terminal
    /// which shows the number of layers already processed.
    /// See [Self::sift_single_var()] for `max_increase` parameter.
    #[must_use]
    #[allow(unused)]
    pub fn sift_all_vars(
        &mut self,
        mut f: NodeID,
        progressbar: bool,
        max_increase: Option<usize>,
    ) -> NodeID {
        let bar = if progressbar {
            Some(ProgressBar::new(self.level2nodes.len() as u64 - 1))
        } else {
            None
        };

        let root_var = self.nodes.get(&f).unwrap().var;
        let root_level = self.var2level[root_var.0];

        println!("Root level: {} - varId: {:?} ", root_level, root_var);

        for v in (1..self.var2level.len() - 1) {
            if_some!(bar, inc(1));

            if v == root_var.0 {
                continue;
            }

            if self.level2nodes[self.var2level[v]].is_empty() {
                continue;
            }

            if self.var2level[v] <= root_level {
                continue;
            }

            let var = VarID(v);
            f = self.sift_single_var_faster(var, max_increase, f);
            self.purge_retain(f);
        }
        if_some!(bar, finish_and_clear());

        if bar.is_some() {
            // Move cursor up to continue updating the top-level progress bar
            execute!(stdout(), cursor::MoveToPreviousLine(1));
        }
        f
    }
}

#[cfg(test)]
mod area_test {
    use crate::core::{
        bdd_manager::DDManager,
        dvo::{
            area_generation::{
                AreaSelection, AreaSelectionEnum, EqualSplitMethod, ThresholdMethod,
            },
            dvo_strategies::median,
        },
    };

    #[test]
    fn threshold_method_test() {
        let (mut man, nodes) =
            DDManager::load_from_dddmp_file("examples/berkeleydb.dimacs.dddmp".to_string())
                .unwrap();
        let bdd = nodes[0];

        man.purge_retain(bdd);
        let _ = man.print_layer_to_csv("area_1.csv");

        let l2n: Vec<usize> = man
            .level2nodes
            .clone()
            .into_iter()
            .map(|level| level.len())
            .collect();

        // let threshold = l2n.iter().max().unwrap() / 3;
        let threshold = median(&l2n);

        println!("Threshold Method");

        let ranges = ThresholdMethod::default().generate_area(l2n, None, Some(threshold), None);
        println!("area_threshold_method: {:?}", ranges);

        println!("threshold: {}", threshold);
    }

    #[test]
    fn equal_slit_method_test() {
        let (mut man, nodes) =
            DDManager::load_from_dddmp_file("examples/berkeleydb.dimacs.dddmp".to_string())
                .unwrap();
        let bdd = nodes[0];

        man.purge_retain(bdd);
        let _ = man.print_layer_to_csv("area_1.csv");

        let l2n: Vec<usize> = man
            .level2nodes
            .clone()
            .into_iter()
            .map(|level| level.len())
            .collect();

        for i in 1..=10 {
            let ranges =
                EqualSplitMethod::default().generate_area(l2n.clone(), Some(i), None, None);
            assert_eq!(ranges.len(), i);
        }
    }
}

#[cfg(test)]
mod wp_test {
    use std::sync::Arc;

    use num_traits::abs;

    use crate::core::{
        bdd_manager::DDManager,
        dvo::dvo_strategies::{
            gen_permutation, swaps_from_to, ConcurrentDVOStrategie, WindowPermutation,
        },
        swap::SwapContext,
    };

    #[test]
    fn perm_test() {
        let _ = env_logger::builder().is_test(true).try_init();
        assert_eq!(0, gen_permutation(1, 1).len());
        assert_eq!(2, gen_permutation(1, 2).len());
        assert_eq!(6, gen_permutation(1, 3).len());
        assert_eq!(36, gen_permutation(1, 4).len());

        assert_eq!(gen_permutation(1, 1), vec![]);
        assert_eq!(gen_permutation(1, 2), vec![(1, 2), (2, 1)]);
        assert_eq!(
            gen_permutation(1, 3),
            vec![(1, 2), (2, 3), (1, 2), (2, 3), (1, 2), (2, 3)]
        );
        assert_eq!(
            gen_permutation(1, 4),
            vec![
                (2, 3),
                (3, 4),
                (2, 3),
                (3, 4),
                (2, 3),
                (3, 4),
                (1, 2),
                (2, 3),
                (3, 4),
                (2, 3),
                (3, 4),
                (2, 3),
                (3, 4),
                (1, 2),
                (2, 3),
                (1, 2),
                (2, 3),
                (3, 4),
                (2, 3),
                (3, 4),
                (2, 3),
                (3, 4),
                (1, 2),
                (2, 3),
                (3, 4),
                (2, 3),
                (1, 2),
                (2, 3),
                (3, 4),
                (2, 3),
                (3, 4),
                (2, 3),
                (3, 4),
                (1, 2),
                (2, 3),
                (3, 4)
            ]
        )
    }

    #[test]
    fn perm_swap_test() {
        let mut list = vec![1, 2, 3, 4, 5];
        for i in 0..5 {
            for (from, to) in gen_permutation(0, i) {
                assert!(abs(from as isize - to as isize) == 1);
                list.swap(from, to);
            }
            assert_eq!(list, vec![1, 2, 3, 4, 5]);
        }
    }

    #[test]
    fn perm_gen_count() {
        println!("4: {}", gen_permutation(1, 4).len());
        println!("5: {}", gen_permutation(1, 5).len());
        println!("6: {}", gen_permutation(1, 6).len());
        println!("7: {}", gen_permutation(1, 7).len());
    }

    #[test]
    fn swaps_from_to_test() {
        let from_vec = vec![
            vec![1, 3, 2, 4],
            vec![4, 3, 2, 1],
            vec![4, 2, 3, 1],
            vec![1, 2, 4, 3],
            vec![3, 2, 1, 4],
            vec![1, 2, 3, 4],
        ];

        for start in from_vec {
            let mut from = start.clone();
            for (swap_from, swap_to) in swaps_from_to(start, vec![1, 2, 3, 4]) {
                from.swap(swap_from, swap_to);
            }
            assert_eq!(from, vec![1, 2, 3, 4]);
        }
    }

    #[test]
    fn wp_single_test() {
        let _ = env_logger::builder().is_test(true).try_init();

        // let expected = BigUint::parse_bytes(b"2808", 10).unwrap();

        // Build BDD
        // let mut instance = dimacs::parse_dimacs(
        //     &fs::read_to_string("examples/sandwich.dimacs").expect("Failed to read dimacs file."),
        // )
        // .expect("Failed to parse dimacs file.");
        // let (mut man, bdd) =
        //     DDManager::from_instance(&mut instance, None, Default::default()).unwrap();
        // assert_eq!(man.sat_count(bdd), expected);

        let (mut man, nodes) =
            DDManager::load_from_dddmp_file("examples/berkeleydb.dimacs.dddmp".to_string())
                .unwrap();
        let bdd = nodes[0];

        let size_before = man.count_active(bdd);
        println!("Size before sifting: {}", size_before);
        let manager = Arc::new(man.clone());
        let result = WindowPermutation::default().compute_concurrent_dvo(
            manager.clone(),
            None,
            2..6,
            SwapContext::new(),
        );
        // let result = DDManager::wp_in_range(manager.clone(), None, 2..6);
        man.persist_swap(result);
        let size_after = man.count_active(bdd);
        println!("Size after sifting: {}", size_after);

        // assert_eq!(man.sat_count(bdd), expected);
        assert!(size_after <= size_before);
    }
}

#[cfg(test)]
mod tests_async {
    use std::{fs, sync::Arc, time::Instant};

    use futures::future;
    use num_bigint::BigUint;
    use num_traits::abs;
    use tokio::task::spawn_blocking;

    use crate::{
        core::{
            bdd_manager::DDManager, bdd_node::VarID, order::var2level_to_ordered_varids,
            swap::SwapContext, test::tests::TestCase,
        },
        store::bdd,
    };

    // #[test]
    // fn sift_sandwich_single() {
    //     let _ = env_logger::builder().is_test(true).try_init();

    //     let expected = BigUint::parse_bytes(b"2808", 10).unwrap();

    //     // Build BDD
    //     let mut instance = dimacs::parse_dimacs(
    //         &fs::read_to_string("examples/sandwich.dimacs").expect("Failed to read dimacs file."),
    //     )
    //     .expect("Failed to parse dimacs file.");
    //     let (mut man, bdd) =
    //         DDManager::from_instance(&mut instance, None, Default::default()).unwrap();
    //     assert_eq!(man.sat_count(bdd), expected);

    //     let size_before = man.count_active(bdd);
    //     println!("Size before sifting: {}", size_before);
    //     let manager = Arc::new(man.clone());
    //     let start = Instant::now();
    //     let result = DDManager::sift_single_var_in_range(
    //         manager.clone(),
    //         &VarID(13),
    //         None,
    //         1..18,
    //         SwapContext::default(),
    //     );
    //     println!("Sifting took {:?}", start.elapsed());
    //     let start = Instant::now();
    //     let _ = DDManager::sift_single_var_in_range_faster(
    //         manager.clone(),
    //         &VarID(13),
    //         None,
    //         1..18,
    //         SwapContext::default(),
    //     );
    //     println!("Sifting took {:?}", start.elapsed());

    //     man.persist_swap(result);
    //     let size_after = man.count_active(bdd);
    //     println!("Size after sifting: {}", size_after);
    //     println!(
    //         "Order after sifting: {:?}",
    //         var2level_to_ordered_varids(&man.var2level)
    //     );

    //     assert_eq!(man.sat_count(bdd), expected);
    //     assert!(size_after <= size_before);
    // }

    // #[test]
    // fn sift_single() {
    //     let _ = env_logger::builder().is_test(true).try_init();

    //     let (mut man, nodes) = DDManager::load_from_dddmp_file(
    //         "examples/financialServices01.dimacs.dddmp".to_string(),
    //     )
    //     .unwrap();
    //     let bdd = nodes[0];
    //     let start_level = man.var2level[man.nodes.get(&bdd).unwrap().var.0] + 1;
    //     let end_level = man.var2level.len() - 1;

    //     println!("order: {:?}", var2level_to_ordered_varids(&man.var2level));

    //     man.purge_retain(bdd);

    //     let size_before = man.count_active(bdd);
    //     println!("Size before sifting: {}", size_before);
    //     let manager = Arc::new(man.clone());
    //     let start = Instant::now();
    //     let _ = DDManager::sift_single_var_in_range(
    //         manager.clone(),
    //         &VarID(429),
    //         None,
    //         start_level..end_level,
    //         SwapContext::default(),
    //     );
    //     println!("Sifting took {:?}", start.elapsed());
    //     let start = Instant::now();
    //     let _ = DDManager::sift_single_var_in_range_faster(
    //         manager.clone(),
    //         &VarID(429),
    //         None,
    //         start_level..end_level,
    //         SwapContext::default(),
    //     );
    //     println!("Sifting took {:?}", start.elapsed());
    // }

    // #[tokio::test(flavor = "multi_thread")]
    // async fn sift_sandwich_all_other_testcase() {
    //     let _ = env_logger::builder().is_test(true).try_init();

    //     let testcase = TestCase::random_1();
    //     let bdd = testcase.f;
    //     let mut man = testcase.man.clone();
    //     let bdd = man.reduce(bdd);
    //     man.purge_retain(bdd);
    //     let root_node = man.nodes.get(&bdd).unwrap();
    //     println!("Root node: {:?}", root_node);
    //     let expected = man.sat_count(bdd);

    //     assert_eq!(man.sat_count(bdd), expected);

    //     let size_before = man.count_active(bdd);
    //     let nodes_before: usize = man.level2nodes.iter().map(|x| x.len()).sum();
    //     println!("Size before sifting: {}", size_before);

    //     let start = Instant::now();

    //     let manager = Arc::new(man.clone());
    //     let m1 = manager.clone();
    //     let m2 = manager.clone();
    //     let m3 = manager.clone();

    //     let s2 =
    //         spawn_blocking(move || DDManager::sift_all_vars_in_range(manager.clone(), None, 2..9));

    //     let sift1 = tokio::spawn(async move {
    //         DDManager::sift_all_vars_in_range(m1, None, 9..m3.level2nodes.len() - 1)
    //     });
    //     // let sift2 =
    //     //     tokio::spawn(async move { DDManager::sift_all_vars_in_range(m2, false, None, 2..9) });
    //     let result = future::join_all([sift1, s2]).await;

    //     println!("Sifting took {:?}", start.elapsed());

    //     for result in result {
    //         let result = result.unwrap();
    //         man.persist_swap(result);
    //     }

    //     println!("resolve_swap took {:?}", start.elapsed());

    //     // man.purge_retain(bdd);
    //     let size_after = man.count_active(bdd);
    //     let nodes_after: usize = man.level2nodes.iter().map(|x| x.len()).sum();
    //     println!("Size after sifting: {}", size_after);
    //     println!(
    //         "Order after sifting: {:?}",
    //         var2level_to_ordered_varids(&man.var2level)
    //     );
    //     println!("v2l {:?}", man.var2level);
    //     fs::write("after.dot", man.graphviz(bdd)).unwrap();

    //     assert!(testcase.verify_against(&man, bdd));

    //     assert_eq!(man.sat_count(bdd), expected);
    //     assert!(nodes_after <= nodes_before);
    //     println!(
    //         "Nodes before: {}, Nodes after: {}",
    //         nodes_before, nodes_after
    //     );
    //     // assert!(size_after <= size_before);

    //     // TODO: Check if function is actually the same
    // }
    // #[tokio::test]
    // async fn sift_sandwich_all() {
    //     let _ = env_logger::builder().is_test(true).try_init();

    //     // let expected = BigUint::parse_bytes(b"2808", 10).unwrap();

    //     // Build BDD
    //     let mut instance = dimacs::parse_dimacs(
    //         &fs::read_to_string("examples/JHipster.dimacs").expect("Failed to read dimacs file."),
    //     )
    //     .expect("Failed to parse dimacs file.");
    //     let (mut man, bdd) =
    //         DDManager::from_instance(&mut instance, None, Default::default()).unwrap();
    //     man.purge_retain(bdd);
    //     // assert_eq!(man.sat_count(bdd), expected);
    //     let expected = man.sat_count(bdd);
    //     let root_node = man.nodes.get(&bdd).unwrap();
    //     println!("Root node: {:?}", root_node);

    //     let size_before = man.count_active(bdd);
    //     println!("Size before sifting: {}", size_before);
    //     let result = DDManager::sift_all_vars_in_range(
    //         Arc::new(man.clone()),
    //         None,
    //         2..man.level2nodes.len() - 1,
    //     );
    //     man.persist_swap(result);
    //     let size_after = man.count_active(bdd);
    //     println!("Size after sifting: {}", size_after);
    //     println!(
    //         "Order after sifting: {:?}",
    //         var2level_to_ordered_varids(&man.var2level)
    //     );
    //     fs::write("after.dot", man.graphviz(bdd)).unwrap();

    //     assert_eq!(man.sat_count(bdd), expected);
    //     assert!(size_after <= size_before);

    //     // TODO: Check if function is actually the same
    // }

    // #[test]
    // fn sift_hotspot_test() {
    //     let _ = env_logger::builder().is_test(true).try_init();

    //     let start_start = Instant::now();

    //     // Build BDD
    //     // let mut instance = dimacs::parse_dimacs(
    //     //     &fs::read_to_string("examples/busybox.dimacs").expect("Failed to read dimacs file."),
    //     // )
    //     // .expect("Failed to parse dimacs file.");
    //     // let (mut man, bdd) =
    //     //     DDManager::from_instance(&mut instance, None, Default::default()).unwrap();
    //     // let expected = man.sat_count(bdd);

    //     let mut instance = dimacs::parse_dimacs(
    //         &fs::read_to_string("examples/berkeleydb.dimacs").expect("Failed to read dimacs file."),
    //     )
    //     .expect("Failed to parse dimacs file.");
    //     let (mut man, bdd) =
    //         DDManager::from_instance(&mut instance, None, Default::default()).unwrap();

    //     // let (mut man, nodes) =
    //     //     DDManager::load_from_dddmp_file("examples/berkeleydb.dimacs.dddmp".to_string())
    //     //         .unwrap();
    //     // let bdd = nodes[0];

    //     println!("v2l: {:?}", man.var2level);

    //     println!("Parsing took {:?}", start_start.elapsed());

    //     man.purge_retain(bdd);
    //     println!("purging took {:?}", start_start.elapsed());

    //     // let expected = man.sat_count(bdd);
    //     println!("expected took {:?}", start_start.elapsed());

    //     let before = man
    //         .clone()
    //         .level2nodes
    //         .into_iter()
    //         .map(|x| x.len())
    //         .sum::<usize>();

    //     println!("Nodes before: {}", before);
    //     println!("before took {:?}", start_start.elapsed());
    //     // assert!(false);

    //     let start_time = Instant::now();

    //     man.sift_hotspot(None, 3);

    //     println!("Sifting took {:?}", start_time.elapsed());

    //     let after = man
    //         .clone()
    //         .level2nodes
    //         .into_iter()
    //         .map(|x| x.len())
    //         .sum::<usize>();

    //     println!("Nodes before: {}, Nodes after: {}", before, after);

    //     assert!(after <= before);
    //     // assert_eq!(expected, man.sat_count(bdd));
    // }
}

#[cfg(test)]
mod tests {
    use std::{fs, time::Instant};

    use num_bigint::BigUint;
    use serde_xml_rs::expect;

    use crate::core::{
        bdd_manager::DDManager, bdd_node::VarID, order::var2level_to_ordered_varids,
    };

    #[test]
    fn sift_sandwich_single() {
        let _ = env_logger::builder().is_test(true).try_init();

        let expected = BigUint::parse_bytes(b"2808", 10).unwrap();

        // Build BDD
        let mut instance = dimacs::parse_dimacs(
            &fs::read_to_string("examples/sandwich.dimacs").expect("Failed to read dimacs file."),
        )
        .expect("Failed to parse dimacs file.");
        let (mut man, bdd) =
            DDManager::from_instance(&mut instance, None, Default::default()).unwrap();
        assert_eq!(man.sat_count(bdd), expected);

        let size_before = man.count_active(bdd);
        println!("Size before sifting: {}", size_before);
        let bdd = man.sift_single_var_faster(VarID(7), None, bdd);
        let size_after = man.count_active(bdd);
        println!("Size after sifting: {}", size_after);
        println!(
            "Order after sifting: {:?}",
            var2level_to_ordered_varids(&man.var2level)
        );

        assert_eq!(man.sat_count(bdd), expected);
        assert!(size_after <= size_before);
    }

    #[test]
    fn sift_sandwich_single_bug() {
        let _ = env_logger::builder().is_test(true).try_init();

        let expected = BigUint::parse_bytes(b"2808", 10).unwrap();

        // Build BDD
        let mut instance = dimacs::parse_dimacs(
            &fs::read_to_string("examples/sandwich.dimacs").expect("Failed to read dimacs file."),
        )
        .expect("Failed to parse dimacs file.");
        let (mut man, bdd) =
            DDManager::from_instance(&mut instance, None, Default::default()).unwrap();
        assert_eq!(man.sat_count(bdd), expected);

        let mut real_man = man.clone();
        let mut real_bdd = bdd.clone();

        let size_before = man.count_active(bdd);
        println!("Size before sifting: {}", size_before);
        let bdd = man.sift_single_var_faster(VarID(7), None, bdd);
        real_bdd = real_man.sift_single_var(VarID(7), None, real_bdd);
        real_man.purge_retain(real_bdd);
        let size_after = man.count_active(bdd);
        println!("Size after sifting: {}", size_after);
        println!(
            "Order after sifting: {:?}",
            var2level_to_ordered_varids(&man.var2level)
        );

        assert_eq!(man.sat_count(bdd), expected);
        assert!(size_after <= size_before);
    }

    #[test]
    fn sift_sandwich_all() {
        let _ = env_logger::builder().is_test(true).try_init();

        let expected = BigUint::parse_bytes(b"2808", 10).unwrap();

        // Build BDD
        let mut instance = dimacs::parse_dimacs(
            &fs::read_to_string("examples/sandwich.dimacs").expect("Failed to read dimacs file."),
        )
        .expect("Failed to parse dimacs file.");
        let (mut man, bdd) =
            DDManager::from_instance(&mut instance, None, Default::default()).unwrap();
        assert_eq!(man.sat_count(bdd), expected);

        man.purge_retain(bdd);

        let size_before = man.count_active(bdd);
        println!("Size before sifting: {}", size_before);
        let bdd = man.sift_all_vars(bdd, false, None);
        let size_after = man.count_active(bdd);
        println!("Size after sifting: {}", size_after);
        println!(
            "Order after sifting: {:?}",
            var2level_to_ordered_varids(&man.var2level)
        );
        fs::write("after.dot", man.graphviz(bdd)).unwrap();

        assert_eq!(man.sat_count(bdd), expected);
        assert!(size_after <= size_before);
        // TODO: Check if function is actually the same
    }

    #[test]
    fn sift_automotive_all() {
        let _ = env_logger::builder().is_test(true).try_init();

        let start_start = Instant::now();

        // // Build BDD
        // let mut instance = dimacs::parse_dimacs(
        //     &fs::read_to_string("examples/berkeleydb.dimacs").expect("Failed to read dimacs file."),
        // )
        // .expect("Failed to parse dimacs file.");
        // let (mut man, bdd) =
        //     DDManager::from_instance(&mut instance, None, Default::default()).unwrap();
        let (mut man, nodes) =
            DDManager::load_from_dddmp_file("examples/berkeleydb.dimacs.dddmp".to_string())
                .unwrap();
        let bdd = nodes[0];
        let expected = man.sat_count(bdd);
        assert_eq!(man.sat_count(bdd), expected);

        man.purge_retain(bdd);

        println!(
            "Order after sifting: {:?}",
            var2level_to_ordered_varids(&man.var2level)
        );

        println!("Parsing took {:?}", start_start.elapsed());

        let before = man
            .clone()
            .level2nodes
            .into_iter()
            .map(|x| x.len())
            .sum::<usize>();
        let start_time = Instant::now();
        let bdd = man.sift_all_vars(bdd, true, None);
        println!("Sifting took {:?}", start_time.elapsed());
        let after = man
            .clone()
            .level2nodes
            .into_iter()
            .map(|x| x.len())
            .sum::<usize>();
        println!("Nodes before: {}, Nodes after: {}", before, after);
        println!(
            "Order after sifting: {:?}",
            var2level_to_ordered_varids(&man.var2level)
        );
        // fs::write("after.dot", man.graphviz(bdd)).unwrap();

        assert_eq!(man.sat_count(bdd), expected);
        assert!(after <= before);

        // assert_eq!(man.sat_count(bdd), expected);
        // TODO: Check if function is actually the same
    }

    fn count_nodes(man: &DDManager) -> usize {
        man.clone()
            .level2nodes
            .into_iter()
            .map(|x| x.len())
            .sum::<usize>()
    }
}

#[cfg(test)]
mod evaluation_dvo {
    static N: u32 = 1;
    static PATH: &str = "examples/financialServices01.dimacs.dddmp";
}
