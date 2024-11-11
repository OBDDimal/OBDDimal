use std::{ops::RangeBounds, sync::Arc, time::Instant};

use enum_dispatch::enum_dispatch;
use futures::future;
use itertools::Itertools;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use tokio::{runtime::Runtime, task::JoinHandle};

use super::area_generation::{AreaSelection, AreaSelectionEnum};
use crate::core::{
    bdd_manager::DDManager,
    bdd_node::{NodeID, VarID},
    dvo::area_generation::merge_ranges,
    order::var2level_to_ordered_varids,
    swap::SwapContext,
};

/// This finds the best position for a variable in a specific range
/// of levels. It returns the best position and the graph size at that
/// position.
///
/// # Arguments
/// * `man` - The DDManager
/// * `var` - The variable to find the best position for
/// * `max_increase` - The maximum increase in graph size allowed before terminating
/// * `level_range` - The range of levels to consider
/// * `prev_swap` - The current swap context
#[allow(unused)]
fn find_best_position_in_range<R: RangeBounds<usize> + IntoIterator<Item = usize>>(
    man: Arc<DDManager>,
    var: &VarID,
    max_increase: Option<usize>,
    level_range: R,
    prev_swap: SwapContext,
) -> (isize, SwapContext) {
    let starting_pos = prev_swap.var2level(&man.var2level, var.0);
    assert!(level_range.contains(&starting_pos));

    let mut best_context: SwapContext = prev_swap.clone();
    let mut best_swaps: Vec<(VarID, VarID)> = vec![];
    let mut current_swaps: Vec<(VarID, VarID)> = vec![];
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

    // check that the variable is at either side of the range
    assert!(end_level == starting_pos || start_level == starting_pos);

    let range = if start_level == starting_pos {
        (start_level..end_level).collect::<Vec<usize>>()
    } else {
        (start_level..end_level).rev().collect::<Vec<usize>>()
    };

    let mut result = (0, prev_swap);
    for level in range {
        let a = result.1.var_at_level(level, &man.var2level).unwrap();
        let b = result.1.var_at_level(level + 1, &man.var2level).unwrap();
        result = man.partial_swap(a, b, result.1);
        current_swaps.push((a, b));

        current_size += result.0;
        log::info!(" Size is {}", current_size);

        if current_size < best_graphsize {
            log::info!(
                " New optimum found with order {:?}",
                var2level_to_ordered_varids(&result.1.permute_swaps(&man.var2level))
            );
            // self.purge_retain(root);
            best_context = result.1.clone();
            best_graphsize = current_size;
        } else if let Some(max) = max_increase {
            if current_size > best_graphsize + (max as isize) {
                // println!("Max increase reached");
                // Do not continue moving downwards, because the graph has grown too much
                return (best_graphsize, best_context);
            }
        }
    }
    (best_graphsize, best_context)
}

/// Swap layer containing specified variable first to the bottom of the BDD, then to the top,
/// and then to the position which resulted in smallest BDD size.
/// Optional parameter `max_increase` stops swapping in either direction once the difference
/// between BDD size and current optimum exceeds threshold.
// #[allow(unused)]
fn sift_single_var_in_range_faster<R: RangeBounds<usize> + IntoIterator<Item = usize>>(
    man: Arc<DDManager>,
    var: &VarID,
    max_increase: Option<usize>,
    level_range: R,
    prev_swap: SwapContext,
) -> SwapContext {
    let starting_pos = prev_swap.var2level(&man.var2level, var.0);
    assert!(level_range.contains(&starting_pos));
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

    let par_result = vec![start_level..=starting_pos, starting_pos..=end_level]
        .into_par_iter()
        .map(|range| {
            find_best_position_in_range(man.clone(), var, max_increase, range, prev_swap.clone())
        })
        .collect::<Vec<(isize, SwapContext)>>();

    // go to best position
    return match par_result
        .into_iter()
        // .filter(|x| x.is_ok())
        // .map(|x| x.unwrap())
        .sorted_by(|(a, a_swaps), (b, b_swaps)| match a.cmp(b) {
            std::cmp::Ordering::Less => std::cmp::Ordering::Less,
            std::cmp::Ordering::Greater => std::cmp::Ordering::Greater,
            std::cmp::Ordering::Equal => a_swaps
                .get_swaps_in_result()
                .len()
                .cmp(&b_swaps.get_swaps_in_result().len()),
        })
        .next()
    {
        Some((evaluation, swaps)) => {
            if evaluation >= 0 {
                prev_swap
            } else {
                swaps
            }
        }
        None => prev_swap,
    };
}

/// Swap layer containing specified variable first to the bottom of the BDD, then to the top,
/// and then to the position which resulted in smallest BDD size.
/// Optional parameter `max_increase` stops swapping in either direction once the difference
/// between BDD size and current optimum exceeds threshold.
// #[allow(unused)]
fn sift_single_var_in_range<R: RangeBounds<usize> + IntoIterator<Item = usize>>(
    man: Arc<DDManager>,
    var: &VarID,
    max_increase: Option<usize>,
    level_range: R,
    prev_swap: SwapContext,
) -> SwapContext {
    let starting_pos = prev_swap.var2level(&man.var2level, var.0);
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

    for level in starting_pos + 1..=end_level {
        log::info!("Trying level {}", level);
        // Swap var at level-1 (our variable) with var at level
        result = man.partial_swap(
            result.1.var_at_level(level - 1, &man.var2level).unwrap(),
            result.1.var_at_level(level, &man.var2level).unwrap(),
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
                var2level_to_ordered_varids(&result.1.permute_swaps(&man.var2level))
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
        result = man.partial_swap(
            result.1.var_at_level(level, &man.var2level).unwrap(),
            result.1.var_at_level(level + 1, &man.var2level).unwrap(),
            result.1,
        );
        current_size += result.0;
        current_level -= 1;
    }

    assert_eq!(current_level, starting_pos);
    // println!("moving top");

    // Move variable to the top
    for level in (start_level..starting_pos).rev() {
        log::info!("Trying level {}", level);
        // Swap var at level+1 (our variable) with var at level
        result = man.partial_swap(
            result.1.var_at_level(level, &man.var2level).unwrap(),
            result.1.var_at_level(level + 1, &man.var2level).unwrap(),
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
                var2level_to_ordered_varids(&result.1.permute_swaps(&man.var2level))
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
        result = man.partial_swap(
            result.1.var_at_level(level - 1, &man.var2level).unwrap(),
            result.1.var_at_level(level, &man.var2level).unwrap(),
            result.1,
        );
        current_level += 1;
    }

    assert_eq!(current_level, best_position);

    // log::info!("Size is now  {}", self.count_active(root));

    result.1
}

#[derive(Default, Clone, Debug, PartialEq)]
pub struct Sifting {}

impl ConcurrentDVOStrategie for Sifting {
    /// Generates a list of ranges that should be used for concurrent DVO
    fn compute_concurrent_dvo<R: RangeBounds<usize> + Clone + IntoIterator<Item = usize>>(
        &self,
        man: Arc<DDManager>,
        max_increase: Option<usize>,
        level_range: R,
        swap_context: SwapContext,
    ) -> SwapContext {
        let man = man.clone();

        let vars: Vec<VarID> = level_range
            .clone()
            .into_iter()
            .filter_map(|level| man.var_at_level(level))
            .collect();

        let mut result = swap_context;

        for var in vars {
            if var.0 == 0 {
                panic!("Variable 0 is not allowed in sifting.");
            }

            if man.level2nodes[man.var2level[var.0]].is_empty() {
                continue;
            }
            result = sift_single_var_in_range(
                man.clone(),
                &var,
                max_increase,
                level_range.clone(),
                result,
            );
            // self.purge_retain(f);
        }
        result
    }
}

#[derive(Default, Clone, Debug, PartialEq)]
pub struct SiftingTwo {}

impl ConcurrentDVOStrategie for SiftingTwo {
    /// Generates a list of ranges that should be used for concurrent DVO
    fn compute_concurrent_dvo<R: RangeBounds<usize> + Clone + IntoIterator<Item = usize>>(
        &self,
        man: Arc<DDManager>,
        max_increase: Option<usize>,
        level_range: R,
        swap_context: SwapContext,
    ) -> SwapContext {
        let man = man.clone();

        let vars: Vec<VarID> = level_range
            .clone()
            .into_iter()
            .filter_map(|level| man.var_at_level(level))
            .collect();

        let _: Vec<()> = level_range
            .clone()
            .into_iter()
            .map(|level| match man.var_at_level(level) {
                Some(var) => (),
                None => {
                    println!("No variable at level {}", level);
                    println!("max level: {}", man.level2nodes.len());
                }
            })
            .collect();

        let mut result = swap_context;

        for var in vars {
            if var.0 == 0 {
                panic!("Variable 0 is not allowed in sifting.");
            }

            if man.level2nodes[man.var2level[var.0]].is_empty() {
                continue;
            }
            result = sift_single_var_in_range_faster(
                man.clone(),
                &var,
                max_increase,
                level_range.clone(),
                result,
            );
            // self.purge_retain(f);
        }
        result
    }
}

#[derive(Default, Clone, Debug, PartialEq)]
pub struct WindowPermutation {}

impl ConcurrentDVOStrategie for WindowPermutation {
    fn compute_concurrent_dvo<R: RangeBounds<usize> + Clone + IntoIterator<Item = usize>>(
        &self,
        man: Arc<DDManager>,
        max_increase: Option<usize>,
        level_range: R,
        swap_context: SwapContext,
    ) -> SwapContext {
        let man = man.clone();

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

        if end_level - start_level > 6 {
            return swap_context;
        }

        // println!("Start level: {}, End level: {}", start_level, end_level);

        let mut current_permutation: Vec<usize> = (start_level..=end_level).collect();
        let mut best_permutation: Vec<usize> = (start_level..=end_level).collect();
        let mut current_size = 0;
        let mut best_size = 0;
        let mut result = (0, swap_context.clone());

        for (from, to) in gen_permutation(start_level, end_level) {
            // println!(
            //     "Swapping {} ({:?}) and {}({:?})",
            //     from,
            //     result.1.var_at_level(from, &man.var2level).unwrap(),
            //     to,
            //     result.1.var_at_level(to, &man.var2level).unwrap()
            // );
            let a = result.1.var_at_level(from, &man.var2level).unwrap();
            let b = result.1.var_at_level(to, &man.var2level).unwrap();

            result = man.partial_swap(a, b, result.1);
            current_size += result.0;
            current_permutation.swap(from - start_level, to - start_level);
            // println!("current_permutation: {:?}", current_permutation);

            if current_size < best_size {
                log::info!(" New optimum found with order {:?}", current_permutation);
                // println!(" New optimum found with order {:?}", current_permutation);
                // self.purge_retain(root);
                best_size = current_size;
                best_permutation = current_permutation.clone();
            } else if let Some(max) = max_increase {
                if current_size > best_size + (max as isize) {
                    // Do not continue moving upwards, because the graph has grown too much
                    // return SwapContext::default();
                    break;
                }
            }
        }

        // way from start to best permutation
        // println!("swapping to best permutation: {:?}", best_permutation);
        let mut result = (0, swap_context.clone());
        for (from, to) in swaps_from_to((start_level..=end_level).collect(), best_permutation) {
            result = man.partial_swap(
                result
                    .1
                    .var_at_level(start_level + from, &man.var2level)
                    .unwrap(),
                result
                    .1
                    .var_at_level(start_level + to, &man.var2level)
                    .unwrap(),
                result.1,
            );
            // println!("Swapping {} and {}", from, to);
        }

        // println!("Best size: {}", best_size);

        result.1
    }
}

#[derive(Default, Clone, Debug, PartialEq)]
pub struct SecondWindowPermutation {}

impl ConcurrentDVOStrategie for SecondWindowPermutation {
    fn compute_concurrent_dvo<R: RangeBounds<usize> + Clone + IntoIterator<Item = usize>>(
        &self,
        man: Arc<DDManager>,
        max_increase: Option<usize>,
        level_range: R,
        swap_context: SwapContext,
    ) -> SwapContext {
        let man = man.clone();

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

        if end_level - start_level > 6 {
            return swap_context;
        }

        // println!("Start level: {}, End level: {}", start_level, end_level);

        let mut current_permutation: Vec<usize> = (start_level..=end_level).collect();
        let mut current_size = 0;
        let mut best_size = 0;
        let mut result = (0, swap_context.clone());
        let mut best_swap: SwapContext = swap_context.clone();

        for (from, to) in gen_permutation(start_level, end_level) {
            // println!(
            //     "Swapping {} ({:?}) and {}({:?})",
            //     from,
            //     result.1.var_at_level(from, &man.var2level).unwrap(),
            //     to,
            //     result.1.var_at_level(to, &man.var2level).unwrap()
            // );
            let a = result.1.var_at_level(from, &man.var2level).unwrap();
            let b = result.1.var_at_level(to, &man.var2level).unwrap();

            result = man.partial_swap(a, b, result.1);
            current_size += result.0;
            current_permutation.swap(from - start_level, to - start_level);
            // println!("current_permutation: {:?}", current_permutation);

            if current_size < best_size {
                log::info!(" New optimum found with order {:?}", current_permutation);
                // println!(" New optimum found with order {:?}", current_permutation);
                // self.purge_retain(root);
                best_size = current_size;
                best_swap = result.1.clone();
            } else if let Some(max) = max_increase {
                if current_size > best_size + (max as isize) {
                    // Do not continue moving upwards, because the graph has grown too much
                    return best_swap;
                }
            }
        }
        best_swap
    }
}

#[derive(Default)]
pub struct RegularSifting {}

impl DVOStrategie for RegularSifting {
    /// Generates a list of ranges that should be used for concurrent DVO
    #[allow(unused_variables)]
    fn run_dvo(&self, manager: &mut DDManager, f: NodeID, max_increase: Option<usize>) -> NodeID {
        manager.sift_all_vars(f, true, max_increase)
    }
}

#[derive(Default)]
pub struct ConcurrentDVO {
    pub area_size: Option<usize>,
    pub strategy: Box<ConcurrentDVOStrategieEnum>,
    pub area_selection: Box<AreaSelectionEnum>,
    pub threshold: Option<usize>,
}

impl ConcurrentDVO {
    pub fn new(
        area_size: Option<usize>,
        strategy: Box<ConcurrentDVOStrategieEnum>,
        area_selection: Box<AreaSelectionEnum>,
        threshold: Option<usize>,
    ) -> Self {
        Self {
            area_size: area_size,
            strategy: strategy,
            area_selection: area_selection,
            threshold: threshold,
        }
    }
}

impl DVOStrategie for ConcurrentDVO {
    /// Generates a list of ranges that should be used for concurrent DVO
    #[allow(unused_variables)]
    fn run_dvo(&self, manager: &mut DDManager, f: NodeID, max_increase: Option<usize>) -> NodeID {
        let start_time = Instant::now();
        let range = 2;
        let global_start_level = 2;
        let global_end_level = manager.level2nodes.len() - 1;

        let root_var = manager.nodes.get(&f).unwrap().var;
        let root_level = manager.var2level[root_var.0];

        let node_distribution: Vec<usize> = manager.calculate_node_count();
        let threshold_node_dist = self.threshold.unwrap_or(median(&node_distribution));

        let ranges_node_dist = self.area_selection.generate_area(
            node_distribution,
            self.area_size,
            Some(threshold_node_dist),
            Some(root_level),
        );

        // let connection_distance = manager.calculate_connection_distance();
        // let threshold_connection_dist = self.threshold.unwrap_or(median(&connection_distance));

        // let mut ranges_connection_dist = self.area_selection.generate_area(
        //     connection_distance,
        //     None,
        //     Some(threshold_connection_dist),
        //     Some(root_level),
        // );

        // ranges_node_dist.append(&mut ranges_connection_dist);
        // let ranges = merge_ranges(&ranges_node_dist);

        println!("Ranges: {:?}", ranges_node_dist);

        let man = Arc::new(manager.clone());

        let runtime = Runtime::new().unwrap();

        let futures = ranges_node_dist
            .iter()
            .map(|(start, end)| {
                let start = *start;
                let end = *end;
                let man = man.clone();
                let strategy = self.strategy.clone();
                runtime.spawn_blocking(move || {
                    strategy.compute_concurrent_dvo(
                        man,
                        max_increase,
                        start..=end,
                        SwapContext::new(),
                    )
                })
            })
            .collect::<Vec<JoinHandle<SwapContext>>>();

        let results = runtime.block_on(future::join_all(futures));
        for result in results {
            let result = result.unwrap();
            manager.persist_swap(result);
        }

        // let results = ranges_node_dist
        //     .par_iter()
        //     .map(|(start, end)| {
        //         self.strategy.compute_concurrent_dvo(
        //             man.clone(),
        //             max_increase,
        //             *start..=*end,
        //             SwapContext::new(),
        //         )
        //     })
        //     .collect::<Vec<SwapContext>>();

        // for result in results {
        //     manager.persist_swap(result);
        // }

        f
    }
}

impl DDManager {
    /// Perform sifting for every layer containing at least one variable.
    /// If `progressbar` is `true`, display a progress bar in the terminal
    /// which shows the number of layers already processed.
    /// See [Self::sift_single_var()] for `max_increase` parameter.
    #[must_use]
    #[allow(unused)]
    fn wp_in_range<R: RangeBounds<usize> + Clone + IntoIterator<Item = usize>>(
        man: Arc<DDManager>,
        max_increase: Option<usize>,
        level_range: R,
    ) -> SwapContext {
        let man = man.clone();

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

        println!("Start level: {}, End level: {}", start_level, end_level);

        let mut current_permutation: Vec<usize> = (start_level..=end_level).collect();
        let mut best_permutation: Vec<usize> = (start_level..=end_level).collect();
        let mut current_size = 0;
        let mut best_size = 0;
        let mut result = (0, SwapContext::default());

        for (from, to) in gen_permutation(start_level, end_level) {
            let a = result.1.var_at_level(from, &man.var2level).unwrap();
            let b = result.1.var_at_level(to, &man.var2level).unwrap();

            result = man.partial_swap(a, b, result.1);
            current_size += result.0;
            current_permutation.swap(from - start_level, to - start_level);
            // println!("current_permutation: {:?}", current_permutation);

            if current_size < best_size {
                log::info!(" New optimum found with order {:?}", current_permutation);
                // println!(" New optimum found with order {:?}", current_permutation);
                // self.purge_retain(root);
                best_size = current_size;
                best_permutation = current_permutation.clone();
            } else if let Some(max) = max_increase {
                if current_size > best_size + (max as isize) {
                    // Do not continue moving upwards, because the graph has grown too much
                    return SwapContext::default();
                }
            }
        }

        // way from start to best permutation
        // println!("swapping to best permutation: {:?}", best_permutation);
        let mut result = (0, SwapContext::default());
        for (from, to) in swaps_from_to((start_level..=end_level).collect(), best_permutation) {
            result = man.partial_swap(
                result
                    .1
                    .var_at_level(start_level + from, &man.var2level)
                    .unwrap(),
                result
                    .1
                    .var_at_level(start_level + to, &man.var2level)
                    .unwrap(),
                result.1,
            );
            // println!("Swapping {} and {}", from, to);
        }

        // println!("Best size: {}", best_size);

        result.1
    }
}

pub(crate) fn gen_permutation(from: usize, to: usize) -> Vec<(usize, usize)> {
    let size = to - from + 1;
    match size {
        0 => vec![],
        1 => vec![],
        2 => vec![(from, to), (to, from)],
        3 => vec![
            (from, from + 1),
            (from + 1, from + 2),
            (from, from + 1),
            (from + 1, from + 2),
            (from, from + 1),
            (from + 1, from + 2),
        ],
        _ => {
            let mut permutation: Vec<(usize, usize)> = vec![];
            for i in 0..size {
                for j in 0..i {
                    // move var j to top
                    permutation.push((from + i - j - 1, from + i - j));
                }

                permutation.append(&mut gen_permutation(from + 1, to));

                for j in 0..i {
                    // move var j to top
                    permutation.push((from + j, from + j + 1));
                }
            }
            permutation
        }
    }
}

pub(crate) fn swaps_from_to(from: Vec<usize>, to: Vec<usize>) -> Vec<(usize, usize)> {
    let mut from = from.clone();
    let mut swaps: Vec<(usize, usize)> = vec![];
    for (to_index, &var) in to.iter().enumerate() {
        let from_index = from.iter().position(|&x| x == var).unwrap();
        if from_index != to_index {
            for i in 0..(from_index - to_index) {
                swaps.push((from_index - i, from_index - i - 1));
                from.swap(from_index - i, from_index - i - 1);
            }
            assert_eq!(from[to_index], to[to_index]);
        }
    }
    swaps
}

pub(crate) fn median<T: PartialOrd + Clone + Copy>(list: &Vec<T>) -> T {
    let mut numbers = list.clone();
    numbers.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mid = numbers.len() / 2;
    numbers[mid]
}

pub(crate) fn nth_percentile<T: PartialOrd + Clone + Copy>(list: &Vec<T>, p: usize) -> T {
    assert!(p <= 100);
    let mut numbers = list.clone();
    numbers.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let i = (numbers.len() as f64 * (p as f64 / 100.0)).round() as usize;
    numbers[i]
}

/// This contains all available DVO implementations
#[enum_dispatch(DVOStrategie)]
pub enum DVOStrategieEnum {
    RegularSifting,
    ConcurrentDVO,
}

/// Implements generate_area()
#[enum_dispatch(DVOStrategie)]
pub trait DVOStrategie {
    /// Generates a list of ranges that should be used for concurrent DVO
    fn run_dvo(&self, manager: &mut DDManager, f: NodeID, max_increase: Option<usize>) -> NodeID;
}

/// Implements generate_area()
#[enum_dispatch(ConcurrentDVOStrategie)]
pub trait ConcurrentDVOStrategie {
    /// Generates a list of ranges that should be used for concurrent DVO
    fn compute_concurrent_dvo<R: RangeBounds<usize> + Clone + IntoIterator<Item = usize>>(
        &self,
        man: Arc<DDManager>,
        max_increase: Option<usize>,
        level_range: R,
        swap_context: SwapContext,
    ) -> SwapContext;
}

/// This contains all available DVO implementations
#[enum_dispatch(ConcurrentDVOStrategie)]
#[derive(Clone, Debug, PartialEq)]
pub enum ConcurrentDVOStrategieEnum {
    WindowPermutation,
    SecondWindowPermutation,
    Sifting,
    SiftingTwo,
}

impl Default for ConcurrentDVOStrategieEnum {
    fn default() -> Self {
        SiftingTwo::default().into()
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, sync::Arc, time::Instant};

    use crate::core::{
        bdd_manager::DDManager,
        bdd_node::VarID,
        dvo::{
            area_generation::{AreaSelection, EqualSplitMethod, ThresholdMethod},
            dvo_strategies::{sift_single_var_in_range, sift_single_var_in_range_faster},
        },
        swap::SwapContext,
    };

    #[test]
    fn sift_single_var_range() {
        let _ = env_logger::builder().is_test(true).try_init();
        let (mut man, nodes) =
            DDManager::load_from_dddmp_file("examples/sandwich.dimacs.dddmp".to_string()).unwrap();
        let bdd = nodes[0];
        man.purge_retain(bdd);
        let start_level = man.var2level[man.nodes.get(&bdd).unwrap().var.0] + 1;

        let l2n: Vec<usize> = man
            .level2nodes
            .clone()
            .into_iter()
            .map(|level| level.len())
            .collect();

        let ranges =
            ThresholdMethod::default().generate_area(l2n, Some(10), None, Some(start_level));

        let expected = man.sat_count(bdd);
        let size_before = man.count_active(bdd);
        for range in ranges {
            let vars: Vec<VarID> = (range.0..=range.1)
                .into_iter()
                .map(|level| man.var_at_level(level))
                .filter(|x| x.is_some())
                .map(|f| f.unwrap())
                .collect();

            for var in vars {
                let size_before = man.count_active(bdd);
                let result = sift_single_var_in_range(
                    Arc::new(man.clone()),
                    &var,
                    None,
                    range.0..=range.1,
                    SwapContext::default(),
                );
                man.persist_swap(result);
                assert_eq!(man.sat_count(bdd), expected);
                assert!(man.count_active(bdd) <= size_before);
            }
        }
        println!(
            "Size before: {}, Size after: {}",
            size_before,
            man.count_active(bdd)
        );
    }

    #[test]
    fn sift_single_var_range_faster() {
        let _ = env_logger::builder().is_test(true).try_init();
        let (mut man, nodes) =
            DDManager::load_from_dddmp_file("examples/sandwich.dimacs.dddmp".to_string()).unwrap();
        let bdd = nodes[0];
        man.purge_retain(bdd);
        let start_level = man.var2level[man.nodes.get(&bdd).unwrap().var.0] + 1;

        let l2n: Vec<usize> = man
            .level2nodes
            .clone()
            .into_iter()
            .map(|level| level.len())
            .collect();

        let ranges =
            ThresholdMethod::default().generate_area(l2n, Some(10), None, Some(start_level));

        let expected = man.sat_count(bdd);
        let size_before = man.count_active(bdd);
        for range in ranges {
            let vars: Vec<VarID> = (range.0..=range.1)
                .into_iter()
                .map(|level| man.var_at_level(level))
                .filter(|x| x.is_some())
                .map(|f| f.unwrap())
                .collect();

            for var in vars {
                let size_before = man.count_active(bdd);
                let result = sift_single_var_in_range_faster(
                    Arc::new(man.clone()),
                    &var,
                    None,
                    range.0..=range.1,
                    SwapContext::default(),
                );
                man.persist_swap(result);
                assert_eq!(man.sat_count(bdd), expected);
                assert!(man.count_active(bdd) <= size_before);
            }
        }
        println!(
            "Size before: {}, Size after: {}",
            size_before,
            man.count_active(bdd)
        );
    }
}

#[cfg(test)]
mod dvo_evaluation {
    use std::{fs, io::Write, thread::available_parallelism, time::Instant};

    use super::{
        ConcurrentDVO, ConcurrentDVOStrategieEnum, DVOStrategie, Sifting, SiftingTwo,
        WindowPermutation,
    };
    use crate::core::{
        bdd_manager::DDManager,
        bdd_node::VarID,
        dvo::{
            area_generation::{AreaSelection, EqualSplitMethod, HotspotMethod, ThresholdMethod},
            dvo_strategies::SecondWindowPermutation,
        },
    };

    static N: u32 = 1;
    // static PATH: &str = "examples/financialServices01.dimacs.dddmp";
    // static PATH: &str = "examples/berkeleydb.dimacs.dddmp";
    // static PATH: &str = "examples/automotive02v4.dimacs.dddmp";
    static PATH: &str = "examples/automotive01.dimacs.dddmp";

    #[test]
    fn info() {
        let (mut man, nodes) = DDManager::load_from_dddmp_file(PATH.to_string()).unwrap();
        let bdd = nodes[0];
        man.purge_retain(bdd);

        let nodes = man
            .level2nodes
            .clone()
            .into_iter()
            .map(|level| level.len())
            .collect::<Vec<usize>>();

        println!("Threads: {}", available_parallelism().unwrap().get());
        println!("Model: {}", PATH);
        println!("Level: {}", nodes.len());
        println!("Nodes: {}", nodes.into_iter().sum::<usize>());
        println!("N times: {}", N);
    }

    #[test]
    fn evaluate_area() {
        let _ = env_logger::builder().is_test(true).try_init();
        let (mut man, nodes) = DDManager::load_from_dddmp_file(PATH.to_string()).unwrap();
        let bdd = nodes[0];
        man.purge_retain(bdd);
        let start_level = man.var2level[man.nodes.get(&bdd).unwrap().var.0] + 1;

        let l2n: Vec<usize> = man
            .level2nodes
            .clone()
            .into_iter()
            .map(|level| level.len())
            .collect();

        let range_count = man.level2nodes.len() - start_level;
        let mut area_sizes: Vec<usize> = vec![];
        let mut last_area_count = usize::MAX;
        for area_size in 2..range_count {
            let ranges = ThresholdMethod::default().generate_area(
                l2n.clone(),
                Some(area_size),
                Some(0),
                Some(start_level),
            );
            if ranges.len() >= last_area_count {
                continue;
            }
            last_area_count = ranges.len();
            area_sizes.push(area_size);
        }

        area_sizes = vec![man.level2nodes.len()];

        println!("Area sizes: {:?}", area_sizes);
        // let area_sizes: Vec<usize> = area_sizes
        //     .into_iter()
        //     .rev()
        //     .take(12)
        //     .rev()
        //     .take(5)
        //     .collect();

        // let strategies: Vec<ConcurrentDVOStrategieEnum> = vec![SiftingTwo::default().into()];
        // let strategies: Vec<ConcurrentDVOStrategieEnum> = vec![
        //     WindowPermutation::default().into(),
        //     Sifting::default().into(),
        //     SiftingTwo::default().into(),
        // ];
        let strategies: Vec<ConcurrentDVOStrategieEnum> = vec![
            // WindowPermutation::default().into(),
            SiftingTwo::default().into(),
            // Sifting::default().into(),
        ];

        for strategy in strategies {
            let mut results: Vec<usize> = vec![];
            let mut times: Vec<Vec<usize>> = vec![vec![]; area_sizes.len()];
            for _ in 0..N {
                results = vec![];

                for (i, area_size) in area_sizes.clone().into_iter().enumerate() {
                    let mut man = man.clone();
                    let start = Instant::now();
                    let dvo = ConcurrentDVO::new(
                        Some(area_size),
                        Box::new(strategy.clone()),
                        Box::new(ThresholdMethod::default().into()),
                        Some(0),
                    );
                    dvo.run_dvo(&mut man, bdd, None);
                    times[i].push(start.elapsed().as_millis() as usize);
                    results.push(man.count_active(bdd));
                }
            }
            println!("strategy: {:?}", strategy);
            println!("starts as: {}", man.count_active(bdd));
            println!("results: {:?}", results);
            let time_calc = times
                .into_iter()
                .map(|time| time.iter().sum::<usize>() as f64 / time.len() as f64)
                .collect::<Vec<f64>>();
            println!("times: {:?}", time_calc);

            let mut file = fs::File::create(format!("{:?}.csv", strategy)).unwrap();
            let mut wtr = csv::Writer::from_writer(vec![]);
            wtr.write_record(vec!["Area size", "Result", "Time"])
                .unwrap();
            for i in 0..area_sizes.len() {
                wtr.write_record(vec![
                    area_sizes[i].to_string(),
                    results[i].to_string(),
                    time_calc[i].to_string(),
                ])
                .unwrap();
            }
            let data = String::from_utf8(wtr.into_inner().unwrap()).unwrap();
            file.write_all(data.as_bytes()).unwrap();
        }
    }

    #[test]
    fn repeated_sift() {
        let _ = env_logger::builder().is_test(true).try_init();
        let (mut man, nodes) = DDManager::load_from_dddmp_file(PATH.to_string()).unwrap();
        let bdd = nodes[0];
        man.purge_retain(bdd);
        let start_level = man.var2level[man.nodes.get(&bdd).unwrap().var.0] + 1;

        let l2n: Vec<usize> = man
            .level2nodes
            .clone()
            .into_iter()
            .map(|level| level.len())
            .collect();

        let range_count = man.level2nodes.len() - start_level;
        let mut area_sizes: Vec<usize> = vec![];
        let mut last_area_count = usize::MAX;
        for area_size in 2..range_count {
            let ranges = ThresholdMethod::default().generate_area(
                l2n.clone(),
                Some(area_size),
                Some(0),
                Some(start_level),
            );
            if ranges.len() >= last_area_count {
                continue;
            }
            last_area_count = ranges.len();
            area_sizes.push(area_size);
        }

        println!("Area sizes: {:?}", area_sizes);
        let area_sizes: Vec<usize> = area_sizes.into_iter().take(10).collect();

        // let strategies: Vec<ConcurrentDVOStrategieEnum> = vec![SiftingTwo::default().into()];
        // let strategies: Vec<ConcurrentDVOStrategieEnum> = vec![
        //     WindowPermutation::default().into(),
        //     Sifting::default().into(),
        //     SiftingTwo::default().into(),
        // ];

        let mut results: Vec<usize> = vec![];
        let mut times: Vec<Vec<usize>> = vec![vec![]; area_sizes.len()];
        for _ in 0..N {
            results = vec![];

            for (i, area_size) in area_sizes.clone().into_iter().enumerate() {
                let mut man = man.clone();
                let dvo = ConcurrentDVO::new(
                    Some(area_size),
                    Box::new(SiftingTwo::default().into()),
                    Box::new(ThresholdMethod::default().into()),
                    Some(0),
                );
                let start = Instant::now();
                let mut prev_count = usize::MAX;
                loop {
                    dvo.run_dvo(&mut man, bdd, None);
                    let count = man.count_active(bdd);
                    if count >= prev_count {
                        break;
                    }
                    prev_count = count;
                }
                results.push(man.count_active(bdd));
                times[i].push(start.elapsed().as_millis() as usize);
            }
        }
        println!("strategy: repeated Sifting");
        println!("starts as: {}", man.count_active(bdd));
        println!("results: {:?}", results);
        let time_calc = times
            .into_iter()
            .map(|time| time.iter().sum::<usize>() as f64 / time.len() as f64)
            .collect::<Vec<f64>>();
        println!("times: {:?}", time_calc);

        let mut file = fs::File::create(format!("repeatedSifting.csv")).unwrap();

        let mut wtr = csv::Writer::from_writer(vec![]);
        wtr.write_record(vec!["Area size", "Result", "Time"])
            .unwrap();
        for i in 0..area_sizes.len() {
            wtr.write_record(vec![
                area_sizes[i].to_string(),
                results[i].to_string(),
                time_calc[i].to_string(),
            ])
            .unwrap();
        }
        // wtr.write_record(area_sizes.iter().map(|x| x.to_string()))
        //     .unwrap();
        // wtr.write_record(results.iter().map(|x| x.to_string()))
        //     .unwrap();
        // wtr.write_record(time_calc.iter().map(|x| x.to_string()))
        //     .unwrap();

        let data = String::from_utf8(wtr.into_inner().unwrap()).unwrap();
        file.write_all(data.as_bytes()).unwrap();
    }

    #[test]
    fn accumulated_sift() {
        let _ = env_logger::builder().is_test(true).try_init();
        let (mut man, nodes) = DDManager::load_from_dddmp_file(PATH.to_string()).unwrap();
        let bdd = nodes[0];
        man.purge_retain(bdd);
        let start_level = man.var2level[man.nodes.get(&bdd).unwrap().var.0] + 1;

        let l2n: Vec<usize> = man
            .level2nodes
            .clone()
            .into_iter()
            .map(|level| level.len())
            .collect();

        let range_count = man.level2nodes.len() - start_level;
        let mut area_sizes: Vec<usize> = vec![];
        let mut last_area_count = usize::MAX;
        for area_size in 2..range_count {
            let ranges = ThresholdMethod::default().generate_area(
                l2n.clone(),
                Some(area_size),
                Some(0),
                Some(start_level),
            );
            if ranges.len() >= last_area_count {
                continue;
            }
            last_area_count = ranges.len();
            area_sizes.push(area_size);
        }

        println!("Area sizes: {:?}", area_sizes);
        let area_sizes: Vec<usize> = area_sizes.into_iter().take(10).collect();

        // let strategies: Vec<ConcurrentDVOStrategieEnum> = vec![SiftingTwo::default().into()];
        // let strategies: Vec<ConcurrentDVOStrategieEnum> = vec![
        //     WindowPermutation::default().into(),
        //     Sifting::default().into(),
        //     SiftingTwo::default().into(),
        // ];

        let mut results: Vec<usize> = vec![];
        let mut times: Vec<Vec<usize>> = vec![vec![]; area_sizes.len()];
        for _ in 0..N {
            results = vec![];
            let mut man = man.clone();
            let start = Instant::now();

            for (i, area_size) in area_sizes.clone().into_iter().enumerate() {
                let dvo = ConcurrentDVO::new(
                    Some(area_size),
                    Box::new(SiftingTwo::default().into()),
                    Box::new(ThresholdMethod::default().into()),
                    Some(0),
                );

                dvo.run_dvo(&mut man, bdd, None);
                times[i].push(start.elapsed().as_millis() as usize);
                results.push(man.count_active(bdd));
            }
        }
        println!("strategy: repeated Sifting");
        println!("starts as: {}", man.count_active(bdd));
        println!("results: {:?}", results);
        let time_calc = times
            .into_iter()
            .map(|time| time.iter().sum::<usize>() as f64 / time.len() as f64)
            .collect::<Vec<f64>>();
        println!("times: {:?}", time_calc);

        let mut file = fs::File::create(format!("accumulatedSift.csv")).unwrap();

        let mut wtr = csv::Writer::from_writer(vec![]);
        wtr.write_record(vec!["Area size", "Result", "Time"])
            .unwrap();
        for i in 0..area_sizes.len() {
            wtr.write_record(vec![
                area_sizes[i].to_string(),
                results[i].to_string(),
                time_calc[i].to_string(),
            ])
            .unwrap();
        }
        // wtr.write_record(area_sizes.iter().map(|x| x.to_string()))
        //     .unwrap();
        // wtr.write_record(results.iter().map(|x| x.to_string()))
        //     .unwrap();
        // wtr.write_record(time_calc.iter().map(|x| x.to_string()))
        //     .unwrap();

        let data = String::from_utf8(wtr.into_inner().unwrap()).unwrap();
        file.write_all(data.as_bytes()).unwrap();
    }

    #[test]
    fn three_times_sift() {
        let _ = env_logger::builder().is_test(true).try_init();
        let (mut man, nodes) = DDManager::load_from_dddmp_file(PATH.to_string()).unwrap();
        let bdd = nodes[0];
        man.purge_retain(bdd);
        let start_level = man.var2level[man.nodes.get(&bdd).unwrap().var.0] + 1;

        let l2n: Vec<usize> = man
            .level2nodes
            .clone()
            .into_iter()
            .map(|level| level.len())
            .collect();

        let range_count = man.level2nodes.len() - start_level;
        let mut area_sizes: Vec<usize> = vec![];
        let mut last_area_count = usize::MAX;
        for area_size in 2..range_count {
            let ranges = ThresholdMethod::default().generate_area(
                l2n.clone(),
                Some(area_size),
                Some(0),
                Some(start_level),
            );
            if ranges.len() >= last_area_count {
                continue;
            }
            last_area_count = ranges.len();
            area_sizes.push(area_size);
        }

        println!("Area sizes: {:?}", area_sizes);
        let area_sizes: Vec<usize> = area_sizes.into_iter().take(20).collect();

        // let strategies: Vec<ConcurrentDVOStrategieEnum> = vec![SiftingTwo::default().into()];
        // let strategies: Vec<ConcurrentDVOStrategieEnum> = vec![
        //     WindowPermutation::default().into(),
        //     Sifting::default().into(),
        //     SiftingTwo::default().into(),
        // ];

        let mut results: Vec<usize> = vec![];
        let mut times: Vec<Vec<usize>> = vec![vec![]; area_sizes.len()];
        for _ in 0..N {
            results = vec![];

            for (i, area_size) in area_sizes.clone().into_iter().enumerate() {
                let mut man = man.clone();
                let start = Instant::now();
                let dvo = ConcurrentDVO::new(
                    Some(area_size),
                    Box::new(SiftingTwo::default().into()),
                    Box::new(ThresholdMethod::default().into()),
                    Some(0),
                );

                let dvo_offset = ConcurrentDVO::new(
                    Some(area_size),
                    Box::new(SiftingTwo::default().into()),
                    Box::new(ThresholdMethod::new(Some((area_size / 2) as usize)).into()),
                    Some(0),
                );
                dvo.run_dvo(&mut man, bdd, None);
                dvo_offset.run_dvo(&mut man, bdd, None);
                dvo.run_dvo(&mut man, bdd, None);
                times[i].push(start.elapsed().as_millis() as usize);
                results.push(man.count_active(bdd));
            }
        }
        println!("strategy: repeated Sifting");
        println!("starts as: {}", man.count_active(bdd));
        println!("results: {:?}", results);
        let time_calc = times
            .into_iter()
            .map(|time| time.iter().sum::<usize>() as f64 / time.len() as f64)
            .collect::<Vec<f64>>();
        println!("times: {:?}", time_calc);

        let mut file = fs::File::create(format!("ThreeTimesSift.csv")).unwrap();

        let mut wtr = csv::Writer::from_writer(vec![]);
        wtr.write_record(vec!["Area size", "Result", "Time"])
            .unwrap();
        for i in 0..area_sizes.len() {
            wtr.write_record(vec![
                area_sizes[i].to_string(),
                results[i].to_string(),
                time_calc[i].to_string(),
            ])
            .unwrap();
        }
        // wtr.write_record(area_sizes.iter().map(|x| x.to_string()))
        //     .unwrap();
        // wtr.write_record(results.iter().map(|x| x.to_string()))
        //     .unwrap();
        // wtr.write_record(time_calc.iter().map(|x| x.to_string()))
        //     .unwrap();

        let data = String::from_utf8(wtr.into_inner().unwrap()).unwrap();
        file.write_all(data.as_bytes()).unwrap();
    }

    #[test]
    fn evaluate_regular_swap() {
        let _ = env_logger::builder().is_test(true).try_init();
        let (mut man, nodes) = DDManager::load_from_dddmp_file(PATH.to_string()).unwrap();
        let bdd = nodes[0];
        man.purge_retain(bdd);
        println!("regular swap");

        let start_v2l = man.var2level.clone();

        let start = Instant::now();
        let bdd = man.sift_all_vars(bdd, true, None);
        println!("Time: {}", start.elapsed().as_millis());
        println!("Nodes: {}", man.count_active(bdd));
        println!(
            "v2l distance: {:?}",
            var2level_distance(&start_v2l, &man.var2level)
        );
    }

    #[test]
    fn var2level_distance_evaluation() {
        let _ = env_logger::builder().is_test(true).try_init();
        let (mut man, nodes) = DDManager::load_from_dddmp_file(PATH.to_string()).unwrap();
        let bdd = nodes[0];
        man.purge_retain(bdd);
        println!("var2level_distance_evaluation");
        println!("level count: {}", man.var2level.len());
        let start_v2l = man.var2level.clone();
        let start_man = man.clone();

        let _ = man.sift_all_vars(bdd, true, None);
        let regular_distance = var2level_distance(&start_v2l, &man.var2level);

        let mut distances: Vec<Vec<usize>> = vec![];

        for i in vec![5, 7, 10] {
            let mut man = start_man.clone();
            ConcurrentDVO::new(
                Some(man.var2level.len() / i),
                Box::new(SiftingTwo::default().into()),
                Box::new(ThresholdMethod::default().into()),
                Some(0),
            )
            .run_dvo(&mut man, bdd, None);
            distances.push(var2level_distance(&start_v2l, &man.var2level));
        }

        let mut file = fs::File::create(format!("v2l_distance.csv")).unwrap();

        let mut wtr = csv::Writer::from_writer(vec![]);
        wtr.write_record(vec!["i", "regular", "5", "7", "10"])
            .unwrap();
        for i in 0..regular_distance.len() {
            wtr.write_record(vec![
                i.to_string(),
                regular_distance[i].to_string(),
                distances[0][i].to_string(),
                distances[1][i].to_string(),
                distances[2][i].to_string(),
            ])
            .unwrap();
        }

        let data = String::from_utf8(wtr.into_inner().unwrap()).unwrap();
        file.write_all(data.as_bytes()).unwrap();
    }

    fn var2level_distance(a: &Vec<usize>, b: &Vec<usize>) -> Vec<usize> {
        assert_eq!(a.len(), b.len());
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| x.abs_diff(*y))
            .collect()
    }

    #[test]
    fn diff_after_concurrent_sifting() {
        let _ = env_logger::builder().is_test(true).try_init();
        let (mut man, nodes) = DDManager::load_from_dddmp_file(PATH.to_string()).unwrap();
        let bdd = nodes[0];
        man.purge_retain(bdd);
        let start_order = man.var2level.clone();
        let mut current = man.count_active(bdd);
        let expected = man.sat_count(bdd);
        // 51695
        let nodes_before = man.calculate_node_count();

        let dvo_one = ConcurrentDVO::new(
            Some(50),
            Box::new(SiftingTwo::default().into()),
            Box::new(ThresholdMethod::new(None).into()),
            None,
        );
        let dvo_two = ConcurrentDVO::new(
            Some(5),
            Box::new(SecondWindowPermutation::default().into()),
            Box::new(EqualSplitMethod::default().into()),
            None,
        );
        let dvo_three = ConcurrentDVO::new(
            Some(50),
            Box::new(SiftingTwo::default().into()),
            Box::new(HotspotMethod::default().into()),
            Some(0),
        );
        let dvo_four = ConcurrentDVO::new(
            Some(70),
            Box::new(SiftingTwo::default().into()),
            Box::new(EqualSplitMethod::default().into()),
            None,
        );
        let start = Instant::now();
        loop {
            println!("Current: {}", current);
            dvo_two.run_dvo(&mut man, bdd, None);
            dvo_one.run_dvo(&mut man, bdd, None);
            dvo_three.run_dvo(&mut man, bdd, None);
            dvo_four.run_dvo(&mut man, bdd, None);
            let new_count = man.count_active(bdd);
            if current <= new_count {
                break;
            }
            current = new_count;
            break;
        }
        println!("Time: {}ms", start.elapsed().as_millis());
        //58691ms
        //59597ms

        println!("Current: {}", current);
        let distance = var2level_distance(&start_order, &man.var2level);
        // println!("Diff: {:?}", distance);
        assert_eq!(expected, man.sat_count(bdd));

        // println!("v2l before: {:?}", start_order);
        // println!("v2l after: {:?}", man.var2level);

        // println!(
        //     "root level {}",
        //     man.var2level[man.nodes.get(&bdd).unwrap().var.0]
        // );

        let nodes = man.calculate_node_count();
        // println!("Nodes: {:?}", nodes);

        let mut file = fs::File::create(format!("berkeleydb_after_dvo_sifting.csv")).unwrap();

        let mut wtr = csv::Writer::from_writer(vec![]);
        wtr.write_record(vec!["layer", "nodes", "nodes_after", "distance"])
            .unwrap();
        for i in 0..nodes.len() - 1 {
            wtr.write_record(vec![
                i.to_string(),
                nodes_before[i].to_string(),
                nodes[i].to_string(),
                distance[man.var_at_level(i).unwrap_or(VarID(0)).0].to_string(),
            ])
            .unwrap();
        }

        let data = String::from_utf8(wtr.into_inner().unwrap()).unwrap();
        file.write_all(data.as_bytes()).unwrap();
    }

    #[test]
    fn diff_after_sifting() {
        let _ = env_logger::builder().is_test(true).try_init();
        let (mut man, nodes) = DDManager::load_from_dddmp_file(PATH.to_string()).unwrap();
        let mut bdd = nodes[0];
        man.purge_retain(bdd);
        let start_order = man.var2level.clone();
        let mut current = man.count_active(bdd);
        let expected = man.sat_count(bdd);

        let nodes_before = man.calculate_node_count();
        let connection_distance_before = man.calculate_connection_distance();

        let start = Instant::now();
        loop {
            println!("Current: {}", current);
            bdd = man.sift_all_vars(bdd, true, None);
            let new_count = man.count_active(bdd);
            if current <= new_count {
                break;
            }
            current = new_count;
        }
        println!("Time: {}s", start.elapsed().as_secs());
        println!("Current: {}", current);
        let distance = var2level_distance(&start_order, &man.var2level);
        // println!("Diff: {:?}", distance);
        println!(
            "avg Diff: {}",
            distance.iter().sum::<usize>() / distance.len()
        );
        assert_eq!(expected, man.sat_count(bdd));

        // println!("v2l before: {:?}", start_order);
        // println!("v2l after: {:?}", man.var2level);

        println!(
            "root level {}",
            man.var2level[man.nodes.get(&bdd).unwrap().var.0]
        );

        let nodes = man.calculate_node_count();
        // println!("Nodes: {:?}", nodes);

        let connection_distance_after = man.calculate_connection_distance();

        println! {"#Nodes before: {:?}", nodes_before.iter().sum::<usize>()};
        println! {"#Nodes after: {:?}", nodes.iter().sum::<usize>()};

        let mut file = fs::File::create(format!("berkeleydb_after_sifting.csv")).unwrap();

        let mut wtr = csv::Writer::from_writer(vec![]);
        wtr.write_record(vec![
            "layer",
            "nodes",
            "connection_distance",
            "nodes_after",
            "connection_distance_after",
            "distance",
        ])
        .unwrap();
        for i in 0..nodes.len() - 1 {
            wtr.write_record(vec![
                i.to_string(),
                nodes_before[i].to_string(),
                connection_distance_before[i].to_string(),
                nodes[i].to_string(),
                connection_distance_after[i].to_string(),
                distance[man.var_at_level(i).unwrap_or(VarID(0)).0].to_string(),
            ])
            .unwrap();
        }

        let data = String::from_utf8(wtr.into_inner().unwrap()).unwrap();
        file.write_all(data.as_bytes()).unwrap();
    }
}

#[cfg(test)]
mod evaluation {

    use std::sync::Arc;

    use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};

    use super::{SecondWindowPermutation, SiftingTwo};
    use crate::core::{
        bdd_manager::DDManager,
        dvo::{
            area_generation::{AreaSelection, EqualSplitMethod},
            dvo_strategies::{
                ConcurrentDVOStrategie, ConcurrentDVOStrategieEnum, Sifting, WindowPermutation,
            },
        },
        swap::SwapContext,
    };

    static MODELS: [&str; 2] = [
        "examples/berkeleydb.dimacs.dddmp",
        "examples/financialServices01.dimacs.dddmp",
        // "examples/automotive02v4.dimacs.dddmp",
        // "examples/automotive01.dimacs.dddmp",
    ];

    #[test]
    fn concurrent_vs_sequential() {
        let strategies: Vec<ConcurrentDVOStrategieEnum> = vec![
            Sifting::default().into(),
            WindowPermutation::default().into(),
        ];

        for model in MODELS.iter() {
            println!("############ Model: {}", model);

            let (mut man, nodes) = DDManager::load_from_dddmp_file(model.to_string()).unwrap();
            let bdd = nodes[0];
            man.purge_retain(bdd);
            let start_level = man.var2level[man.nodes.get(&bdd).unwrap().var.0];
            let nodes = man.calculate_node_count();
            let max_increase = man
                .level2nodes
                .iter()
                .map(|level| level.len())
                .max()
                .unwrap();

            let before = man.count_active(bdd);

            let ranges =
                EqualSplitMethod::default().generate_area(nodes, Some(5), None, Some(start_level));
            // println!("{:?}", ranges);

            // // Sequential
            // for strategy in strategies.clone() {
            //     let start = std::time::Instant::now();

            //     let mut man = man.clone();
            //     let manager = Arc::new(man.clone());

            //     let results = ranges
            //         .clone()
            //         .into_iter()
            //         .map(|(from, to)| {
            //             let mut swap_context = SwapContext::new();
            //             swap_context.precalc_references(&manager.clone(), from, to);
            //             strategy.compute_concurrent_dvo(
            //                 manager.clone(),
            //                 None,
            //                 from..=to,
            //                 swap_context,
            //             )
            //         })
            //         .collect::<Vec<SwapContext>>();
            //     for result in results {
            //         man.persist_swap(result);
            //     }

            //     let after = man.count_active(bdd);

            //     println!(
            //         "Sciential Strategy {:?} took {:?} and improved {} to {} which is an improvement of {}%",
            //         strategy,
            //         start.elapsed(),
            //         before,
            //         after,
            //         ((before - after) as f64 / before as f64) * 100.0
            //     );
            // }
            // Asynchronous
            for strategy in strategies.clone() {
                let start = std::time::Instant::now();

                let mut man = man.clone();
                let manager = Arc::new(man.clone());

                let results = ranges
                    .clone()
                    .into_par_iter()
                    .map(|(from, to)| {
                        let mut swap_context = SwapContext::new();
                        swap_context.precalc_references(&manager.clone(), from, to);
                        strategy.compute_concurrent_dvo(
                            manager.clone(),
                            Some(max_increase),
                            from..=to,
                            swap_context,
                        )
                    })
                    .collect::<Vec<SwapContext>>();
                for result in results {
                    man.persist_swap(result);
                }

                let after = man.count_active(bdd);

                println!(
                    "Asynchronous Strategy {:?} took {:?} and improved {} to {} which is an improvement of {}%",
                    strategy,
                    start.elapsed(),
                    before,
                    after,
                    ((before - after) as f64 / before as f64) * 100.0
                );
            }
        }
    }

    #[test]
    fn utilize_context() {
        let strategies: Vec<ConcurrentDVOStrategieEnum> = vec![
            Sifting::default().into(),
            SiftingTwo::default().into(),
            // WindowPermutation::default().into(),
            // SecondWindowPermutation::default().into(),
        ];

        for model in MODELS.iter() {
            println!("############ Model: {}", model);

            let (mut man, nodes) = DDManager::load_from_dddmp_file(model.to_string()).unwrap();
            let bdd = nodes[0];
            man.purge_retain(bdd);
            let start_level = man.var2level[man.nodes.get(&bdd).unwrap().var.0];
            let nodes = man.calculate_node_count();
            let max_increase = man
                .level2nodes
                .iter()
                .map(|level| level.len())
                .max()
                .unwrap();

            let before = man.count_active(bdd);

            let ranges =
                EqualSplitMethod::default().generate_area(nodes, Some(10), None, Some(start_level));
            // println!("{:?}", ranges);

            for strategy in strategies.clone() {
                let start = std::time::Instant::now();

                let mut man = man.clone();
                let manager = Arc::new(man.clone());

                let results = ranges
                    .clone()
                    .into_iter()
                    .map(|(from, to)| {
                        let mut swap_context = SwapContext::new();
                        swap_context.precalc_references(&manager.clone(), from, to);
                        strategy.compute_concurrent_dvo(
                            manager.clone(),
                            Some(max_increase),
                            from..=to,
                            swap_context,
                        )
                    })
                    .collect::<Vec<SwapContext>>();
                for result in results {
                    man.persist_swap(result);
                }

                let after = man.count_active(bdd);

                println!(
                    "Asynchronous Strategy {:?} took {:?} and improved {} to {} which is an improvement of {}%",
                    strategy,
                    start.elapsed(),
                    before,
                    after,
                    ((before - after) as f64 / before as f64) * 100.0
                );
            }
        }
    }
}
