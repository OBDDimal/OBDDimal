//! Implementation of dynamic variable ordering techniques
#![allow(rustdoc::private_intra_doc_links)]

pub mod dvo_schedules;

use core::panic;
use std::{
    io::stdout,
    ops::RangeBounds,
    sync::Arc,
    time::{Instant, SystemTime},
};

use crossterm::{cursor, execute};
use indicatif::ProgressBar;
use itertools::Itertools;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use tokio::{
    runtime::Runtime,
    task::{spawn_blocking, JoinHandle},
};

use super::{
    bdd_manager::DDManager,
    bdd_node::{NodeID, VarID, ZERO},
    order::var2level_to_ordered_varids,
    swap::SwapContext,
};
use crate::if_some;

impl DDManager {
    /// Find the variable at specified level
    fn var_at_level(&self, level: usize) -> Option<VarID> {
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
        for level in (1..starting_pos).rev() {
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

        for v in (1..self.var2level.len()) {
            if_some!(bar, inc(1));

            if self.level2nodes[self.var2level[v]].is_empty() {
                continue;
            }

            let var = VarID(v);
            f = self.sift_single_var(var, max_increase, f);
            self.purge_retain(f);
        }
        if_some!(bar, finish_and_clear());

        if bar.is_some() {
            // Move cursor up to continue updating the top-level progress bar
            execute!(stdout(), cursor::MoveToPreviousLine(1));
        }
        f
    }

    /// Swap layer containing specified variable first to the bottom of the BDD, then to the top,
    /// and then to the position which resulted in smallest BDD size.
    /// Optional parameter `max_increase` stops swapping in either direction once the difference
    /// between BDD size and current optimum exceeds threshold.
    // #[allow(unused)]
    fn sift_single_var_in_range<R: RangeBounds<usize> + IntoIterator<Item = usize>>(
        &self,
        var: &VarID,
        max_increase: Option<usize>,
        level_range: R,
        prev_swap: SwapContext,
    ) -> SwapContext {
        let starting_pos = prev_swap.var2level(&self.var2level, var.0);
        assert!(level_range.contains(&starting_pos));

        let mut best_position = starting_pos;
        let mut best_graphsize = 0;
        let mut current_size = 0;

        assert!(level_range.contains(&starting_pos));

        log::info!(
            "Sifting variable {:?}, starting from level {} (graph size {}).",
            var,
            starting_pos,
            best_graphsize
        );
        // println!(
        //     "Sifting variable {:?}, starting from level {} (graph size {}).",
        //     var, starting_pos, best_graphsize
        // );

        // Move variable to the bottom
        let end_level = match level_range.end_bound() {
            std::ops::Bound::Included(&x) => x,
            std::ops::Bound::Excluded(&x) => x - 1,
            std::ops::Bound::Unbounded => panic!("Unbounded range."),
        };

        let start_level = match level_range.start_bound() {
            std::ops::Bound::Included(&x) => x,
            std::ops::Bound::Excluded(&x) => x + 1,
            std::ops::Bound::Unbounded => panic!("Unbounded range."),
        };

        let mut current_level = starting_pos;

        log::info!("Moving down...");

        let mut result = (0, prev_swap);

        for level in starting_pos + 1..end_level {
            log::info!("Trying level {}", level);
            // Swap var at level-1 (our variable) with var at level
            result = self.partial_swap(
                result.1.var_at_level(level - 1, &self.var2level).unwrap(),
                result.1.var_at_level(level, &self.var2level).unwrap(),
                result.1,
            );
            current_level += 1;

            current_size += result.0;
            // println!(
            //     "Swapping level {} and {} => {}",
            //     level - 1,
            //     level,
            //     current_size
            // );
            log::info!(" Size is {}", current_size);

            if current_size < best_graphsize {
                log::info!(
                    " New optimum found with order {:?}",
                    var2level_to_ordered_varids(&result.1.permute_swaps(&self.var2level))
                );
                // self.purge_retain(root);
                best_graphsize = current_size;
                best_position = level;
            } else if let Some(max) = max_increase {
                if current_size > best_graphsize + (max as isize) {
                    // Do not continue moving downwards, because the graph has grown too much
                    break;
                }
            }
        }

        // Level is now bottom (terminal-1).
        log::info!("Moving up...");
        // println!("Moving up...");

        // Swap back to initial position
        for level in (starting_pos..current_level).rev() {
            result = self.partial_swap(
                result.1.var_at_level(level, &self.var2level).unwrap(),
                result.1.var_at_level(level + 1, &self.var2level).unwrap(),
                result.1,
            );
            current_size += result.0;
            current_level -= 1;
        }

        assert_eq!(current_level, starting_pos);

        // Move variable to the top
        for level in (start_level..starting_pos).rev() {
            log::info!("Trying level {}", level);
            // Swap var at level+1 (our variable) with var at level
            result = self.partial_swap(
                result.1.var_at_level(level, &self.var2level).unwrap(),
                result.1.var_at_level(level + 1, &self.var2level).unwrap(),
                result.1,
            );
            current_level -= 1;

            current_size += result.0;
            log::info!(" Size is {}", current_size);

            // println!(
            //     "Swapping level {} and {} => {}",
            //     level - 1,
            //     level,
            //     current_size
            // );

            if current_size < best_graphsize {
                log::info!(
                    " New optimum found with order {:?}",
                    var2level_to_ordered_varids(&result.1.permute_swaps(&self.var2level))
                );
                // self.purge_retain(root);
                best_graphsize = current_size;
                best_position = level;
            } else if let Some(max) = max_increase {
                if current_size > best_graphsize + (max as isize) {
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
        // println!(
        //     "The best result was graph size of {} at level {} for {:?}. Moving there...",
        //     best_graphsize, best_position, var
        // );

        for level in current_level + 1..best_position + 1 {
            // Swap var at level-1 (our variable) with var at level
            result = self.partial_swap(
                result.1.var_at_level(level - 1, &self.var2level).unwrap(),
                result.1.var_at_level(level, &self.var2level).unwrap(),
                result.1,
            );
            current_level += 1;
        }

        assert_eq!(current_level, best_position);

        // log::info!("Size is now  {}", self.count_active(root));

        result.1
    }

    /// Perform sifting for every layer containing at least one variable.
    /// If `progressbar` is `true`, display a progress bar in the terminal
    /// which shows the number of layers already processed.
    /// See [Self::sift_single_var()] for `max_increase` parameter.
    #[must_use]
    #[allow(unused)]
    pub async fn sift_all_vars_in_range<
        R: RangeBounds<usize> + Clone + IntoIterator<Item = usize>,
    >(
        man: Arc<DDManager>,
        progress_bar: bool,
        max_increase: Option<usize>,
        level_range: R,
    ) -> SwapContext {
        let man = man.clone();
        let bar = if progress_bar {
            Some(ProgressBar::new(
                level_range.clone().into_iter().count() as u64
            ))
        } else {
            None
        };

        let vars: Vec<VarID> = level_range
            .clone()
            .into_iter()
            .map(|level| man.var_at_level(level).unwrap())
            .collect();

        let mut result = SwapContext::default();

        for var in vars {
            if_some!(bar, inc(1));

            if var.0 == 0 {
                panic!("Variable 0 is not allowed in sifting.");
            }

            if man.level2nodes[man.var2level[var.0]].is_empty() {
                continue;
            }
            // println!("Sifting variable {:?}", var);
            result = man.sift_single_var_in_range(&var, max_increase, level_range.clone(), result);
            // self.purge_retain(f);
        }
        if_some!(bar, finish_and_clear());

        if bar.is_some() {
            // Move cursor up to continue updating the top-level progress bar
            execute!(stdout(), cursor::MoveToPreviousLine(1));
        }
        println!(
            "Finished siftin for range {:?} - {:?}",
            level_range.start_bound(),
            level_range.end_bound()
        );
        result
    }

    #[allow(unused)]
    fn sift_hotspot(&mut self, max_increase: Option<usize>, max_hotspot_size: usize) {
        let range = 1;
        let global_start_level = 2;
        let global_end_level = self.level2nodes.len() - 1;

        let l2n: Vec<usize> = self
            .level2nodes
            .clone()
            .into_iter()
            .map(|level| level.len())
            .collect();

        let threshold = (l2n.clone().into_iter().sum::<usize>() / self.var2level.len()) as isize;
        println!("Threshold: {}", threshold);

        // let mut diffs: Vec<isize> = vec![];
        // for i in 1..l2n.len() {
        //     log::info!("Level {}: {}", i, l2n[i]);
        //     diffs.push(l2n[i] as isize - l2n[i - 1] as isize);
        // }
        // println!("Differences: {:?}", diffs);

        // let avg = diffs.clone().into_iter().map(|x| abs(x)).sum::<isize>() / diffs.len() as isize;
        let avg = l2n.clone().into_iter().sum::<usize>() / l2n.len();
        println!("Average: {}", avg);

        let diff_levels: Vec<(usize, usize)> = l2n
            .clone()
            .into_iter()
            .enumerate()
            .filter(|(_, val)| val > &avg)
            .sorted_by(|(_, a), (_, b)| b.cmp(a))
            .collect();
        println!("Hotspots: {:?}", diff_levels);

        let mut ranges: Vec<(usize, usize)> = vec![];
        for (i, _) in diff_levels {
            // First check if there is a range already
            if ranges
                .iter()
                .any(|(start, end)| (i - range) >= *start && (i + range) <= *end)
            {
                println!("ALready in ranges");
                continue;
            }

            // Now check if there are ranges that can be extended or fused;
            let options: Vec<(usize, (usize, usize))> = ranges
                .iter()
                .enumerate()
                .filter(|(_, (start, end))| {
                    ((i - range) <= *end && i >= *start)
                        || ((i + range) >= *start && i <= *end)
                        || (i <= *end && i >= *start)
                })
                .map(|(index, range)| (index, *range))
                .collect();

            println!("{} - Options: {:?}", i, options);

            // If no ranges can be extended or fused, create a new one
            match options.len() {
                0 => ranges.push((i - range, i + range)),
                1 => {
                    // Extend range
                    let (index, (start, end)) = options[0];

                    let new_start = global_start_level.max(start.min(i - range));
                    let new_end = global_end_level.min(end.max(i + range));

                    if new_end - new_start > max_hotspot_size && end >= i && start <= i {
                        // Is already inside range
                        println!("WARNING: Already inside range");
                        continue;
                    }
                    if new_end - new_start > max_hotspot_size {
                        // make new Hotspot next to it

                        // new hotspot to the left
                        if (i) < start {
                            let new_start = i - range;
                            let new_end = start - 1;
                            // ranges.remove(index);
                            assert!(new_start < new_end);
                            ranges.push((new_start, new_end));
                            continue;
                        }

                        // new hotspot to the right
                        if (i) > end {
                            let new_end = i + range;
                            let new_start = end + 1;
                            // ranges.remove(index);
                            assert!(new_start < new_end);
                            ranges.push((new_start, new_end));
                            continue;
                        }
                        assert!(new_start < new_end);

                        ranges.push((new_start, new_end));
                        continue;
                    }
                    ranges.remove(index);
                    assert!(new_start < new_end);
                    ranges.push((new_start, new_end));
                }
                2 => {
                    // Fuse ranges
                    let (index1, (start1, end1)) = options[0];
                    let (index2, (start2, end2)) = options[1];

                    let new_start = global_start_level.max(start1.min(start2));
                    let new_end = global_end_level.min(end1.max(end2));

                    if new_end - new_start > max_hotspot_size && end1 >= i && start1 <= i {
                        // New range is to big -> first Range (containing index) moves directly to second range
                        if start1 < start2 {
                            let new_start = start1;
                            let new_end = start2 - 1;
                            ranges.remove(index1);
                            assert!(new_start < new_end);
                            ranges.push((new_start, new_end));
                        } else {
                            let new_end = end1;
                            let new_start = end2 + 1;
                            ranges.remove(index1);
                            assert!(new_start < new_end);
                            ranges.push((new_start, new_end));
                        }
                        continue;
                    }

                    if new_end - new_start > max_hotspot_size && end2 >= i && start2 <= i {
                        // New range is to big -> second Range (containing index) moves directly to first range
                        if start2 < start1 {
                            let new_start = start2;
                            let new_end = start1 - 1;
                            ranges.remove(index2);
                            assert!(new_start < new_end);
                            ranges.push((new_start, new_end));
                        } else {
                            let new_end = end2;
                            let new_start = end1 + 1;
                            ranges.remove(index2);
                            assert!(new_start < new_end);
                            ranges.push((new_start, new_end));
                        }
                        continue;
                    }

                    if new_end - new_start > max_hotspot_size {
                        // New range is to big -> Smaller range moves directly to bigger range
                        let (index_smaller, index_bigger) = if end1 - start1 < end2 - start2 {
                            (0, 1)
                        } else {
                            (1, 0)
                        };
                        if options[index_smaller].1 .0 < options[index_bigger].1 .0 {
                            // smaller is in front of bigger
                            let new_start = options[index_smaller].1 .0;
                            let new_end = options[index_bigger].1 .0 - 1;
                            ranges.remove(index_smaller);
                            assert!(new_start < new_end);
                            ranges.push((new_start, new_end));
                        } else {
                            // smaller is behind bigger
                            let new_end = options[index_smaller].1 .1;
                            let new_start = options[index_bigger].1 .1 + 1;
                            ranges.remove(index_smaller);
                            assert!(new_start < new_end);
                            ranges.push((new_start, new_end));
                        }
                        continue;
                    }
                    // Fuse ranges
                    ranges.remove(index2);
                    ranges.remove(index1);
                    assert!(new_start < new_end);
                    ranges.push((new_start, new_end));
                    println!(
                        "FUSED RANGES ({}, {}) + ({}, {}) -> ({}, {})",
                        start1, end1, start2, end2, new_start, new_end
                    );
                }
                _ => panic!("More than 2 ranges to fuse or extend."),
            }
        }
        println!("Ranges: {:?}", ranges);

        let arc_self = Arc::new(self.clone());
        let runtime = Runtime::new().unwrap();

        // Create the runtime
        // let runtime = Runtime::new().unwrap();

        // let mut futures = vec![];
        // for (start, stop) in ranges {
        //     let arc_self = arc_self.clone();
        //     futures.push(spawn_blocking(move || {
        //         DDManager::sift_all_vars_in_range(arc_self, false, max_increase, start..=stop)
        //     }));
        // }

        // println!("working on futures");

        // let results = futures::future::join_all(futures).await;

        // results.into_iter().for_each(|result| {
        //     let result = result.unwrap();
        //     self.resolve_swap(result);
        // });

        // without tokio::main

        // #############################
        // normal tokio

        // let runtime = tokio::runtime::Builder::new_multi_thread()
        //     .disable_lifo_slot()
        //     .max_blocking_threads(10)
        //     .build()
        //     .unwrap();

        // let handle = runtime.handle();

        // let mut futures: Vec<JoinHandle<SwapContext>> = vec![];
        // for (start, stop) in ranges {
        //     let arc_self = arc_self.clone();

        //     println!("spawning task at: {:?} ", SystemTime::now());

        //     futures.push(handle.spawn(DDManager::sift_all_vars_in_range(
        //         arc_self,
        //         false,
        //         max_increase,
        //         start..=stop,
        //     )));
        // }

        // println!("working on futures");

        // let results = handle.block_on(async move { futures::future::join_all(futures).await });

        // results.into_iter().for_each(|result| {
        //     let result = result.unwrap();
        //     self.resolve_swap(result);
        // });
    }
}

#[cfg(test)]
mod tests_async {
    use std::{fs, sync::Arc, time::Instant};

    use futures::future;
    use num_bigint::BigUint;

    use crate::{
        core::{
            bdd_manager::DDManager, bdd_node::VarID, order::var2level_to_ordered_varids,
            swap::SwapContext, test::tests::TestCase,
        },
        store::bdd,
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
        let result = man.sift_single_var_in_range(&VarID(5), None, 1..18, SwapContext::default());
        man.resolve_swap(result);
        let size_after = man.count_active(bdd);
        println!("Size after sifting: {}", size_after);
        println!(
            "Order after sifting: {:?}",
            var2level_to_ordered_varids(&man.var2level)
        );

        assert_eq!(man.sat_count(bdd), expected);
        assert!(size_after <= size_before);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sift_sandwich_all_other_testcase() {
        let _ = env_logger::builder().is_test(true).try_init();

        let testcase = TestCase::random_1();
        let bdd = testcase.f;
        let mut man = testcase.man.clone();
        let bdd = man.reduce(bdd);
        man.purge_retain(bdd);
        let root_node = man.nodes.get(&bdd).unwrap();
        println!("Root node: {:?}", root_node);
        let expected = man.sat_count(bdd);

        assert_eq!(man.sat_count(bdd), expected);

        let size_before = man.count_active(bdd);
        let nodes_before: usize = man.level2nodes.iter().map(|x| x.len()).sum();
        println!("Size before sifting: {}", size_before);

        let start = Instant::now();

        let manager = Arc::new(man.clone());
        let m1 = manager.clone();
        let m2 = manager.clone();
        let m3 = manager.clone();

        let sift1 = tokio::spawn(async move {
            DDManager::sift_all_vars_in_range(m1, false, None, 8..m3.level2nodes.len() - 1).await
        });
        let sift2 =
            tokio::spawn(
                async move { DDManager::sift_all_vars_in_range(m2, false, None, 2..9).await },
            );
        let result = future::join_all([sift1, sift2]).await;

        println!("Sifting took {:?}", start.elapsed());

        for result in result {
            let result = result.unwrap();
            man.resolve_swap(result);
        }

        println!("resolve_swap took {:?}", start.elapsed());

        // man.purge_retain(bdd);
        let size_after = man.count_active(bdd);
        let nodes_after: usize = man.level2nodes.iter().map(|x| x.len()).sum();
        println!("Size after sifting: {}", size_after);
        println!(
            "Order after sifting: {:?}",
            var2level_to_ordered_varids(&man.var2level)
        );
        println!("v2l {:?}", man.var2level);
        fs::write("after.dot", man.graphviz(bdd)).unwrap();

        assert!(testcase.verify_against(&man, bdd));

        assert_eq!(man.sat_count(bdd), expected);
        assert!(nodes_after <= nodes_before);
        println!(
            "Nodes before: {}, Nodes after: {}",
            nodes_before, nodes_after
        );
        // assert!(size_after <= size_before);

        // TODO: Check if function is actually the same
    }
    #[tokio::test]
    async fn sift_sandwich_all() {
        let _ = env_logger::builder().is_test(true).try_init();

        // let expected = BigUint::parse_bytes(b"2808", 10).unwrap();

        // Build BDD
        let mut instance = dimacs::parse_dimacs(
            &fs::read_to_string("examples/JHipster.dimacs").expect("Failed to read dimacs file."),
        )
        .expect("Failed to parse dimacs file.");
        let (mut man, bdd) =
            DDManager::from_instance(&mut instance, None, Default::default()).unwrap();
        man.purge_retain(bdd);
        // assert_eq!(man.sat_count(bdd), expected);
        let expected = man.sat_count(bdd);
        let root_node = man.nodes.get(&bdd).unwrap();
        println!("Root node: {:?}", root_node);

        let size_before = man.count_active(bdd);
        println!("Size before sifting: {}", size_before);
        let result = DDManager::sift_all_vars_in_range(
            Arc::new(man.clone()),
            false,
            None,
            2..man.level2nodes.len() - 1,
        )
        .await;
        man.resolve_swap(result);
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
    fn sift_hotspot_test() {
        let _ = env_logger::builder().is_test(true).try_init();

        // Build BDD
        let mut instance = dimacs::parse_dimacs(
            &fs::read_to_string("examples/financialservices01.dimacs")
                .expect("Failed to read dimacs file."),
        )
        .expect("Failed to parse dimacs file.");

        let (mut man, bdd) =
            DDManager::from_instance(&mut instance, None, Default::default()).unwrap();
        man.purge_retain(bdd);
        let expected = man.sat_count(bdd);

        let before = man
            .clone()
            .level2nodes
            .into_iter()
            .map(|x| x.len())
            .sum::<usize>();

        let start_time = Instant::now();

        man.sift_hotspot(None, 10);

        println!("Sifting took {:?}", start_time.elapsed());

        let after = man
            .clone()
            .level2nodes
            .into_iter()
            .map(|x| x.len())
            .sum::<usize>();

        println!("Nodes before: {}, Nodes after: {}", before, after);

        assert!(after <= before);

        assert_eq!(expected, man.sat_count(bdd));
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use num_bigint::BigUint;

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
        let bdd = man.sift_single_var(VarID(2), None, bdd);
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
}
