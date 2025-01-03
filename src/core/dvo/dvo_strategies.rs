use std::sync::Arc;

use enum_dispatch::enum_dispatch;
use itertools::Itertools;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};

use super::area_generation::{AreaSelection, AreaSelectionEnum};
use crate::core::{
    bdd_manager::DDManager,
    bdd_node::{NodeID, VarID},
    order::var2level_to_ordered_varids,
    swap::SwapContext,
};

/// This finds the best position for a variable in a specific range
/// of levels. It returns the context for the best position and the graph size at that
/// position.
///
/// # Arguments
/// * `man` - The DDManager
/// * `var` - The variable to find the best position for
/// * `max_increase` - The maximum increase in graph size allowed before terminating
/// * `level_range` - The range of levels to consider
/// * `prev_swap` - The current swap context
#[allow(unused)]
fn find_best_position_in_range(
    man: Arc<DDManager>,
    var: &VarID,
    max_increase: Option<usize>,
    level_range: (usize, usize),
    prev_swap: SwapContext,
) -> (isize, SwapContext) {
    let starting_pos = prev_swap.var2level(&man.var2level, var.0);
    assert!(level_range.0 <= level_range.1);
    assert!(level_range.0 <= starting_pos);
    assert!(level_range.1 >= starting_pos);

    let mut best_context: SwapContext = prev_swap.clone();
    let mut best_swaps: Vec<(VarID, VarID)> = vec![];
    let mut current_swaps: Vec<(VarID, VarID)> = vec![];
    let mut best_graph_size = 0;
    let mut current_size = 0;

    log::info!(
        "Finding best position for variable {:?} in range {:?}",
        var,
        level_range
    );

    let start_level = level_range.0;
    let end_level = level_range.1;

    // check that the variable is at either side of the range
    assert!(end_level == starting_pos || start_level == starting_pos);

    let range = if start_level == starting_pos {
        (start_level..end_level).collect::<Vec<usize>>()
    } else {
        (start_level..end_level).rev().collect::<Vec<usize>>()
    };

    let mut result = (0, prev_swap);
    for level in range {
        match result.1.var_at_level(level, &man.var2level) {
            None => {
                continue;
            }
            Some(b) => (),
        }
        let a = result.1.var_at_level(level, &man.var2level).unwrap();
        match result.1.var_at_level(level + 1, &man.var2level) {
            None => {
                continue;
            }
            Some(b) => (),
        }
        let b = result.1.var_at_level(level + 1, &man.var2level).unwrap();
        result = man.partial_swap(a, b, result.1);
        current_swaps.push((a, b));

        current_size += result.0;

        if current_size < best_graph_size {
            log::info!(
                " New optimum found with order {:?}",
                var2level_to_ordered_varids(&result.1.permute_swaps(&man.var2level))
            );
            best_context = result.1.clone();
            best_graph_size = current_size;
        } else if let Some(max) = max_increase {
            if current_size > best_graph_size + (max as isize) {
                // Do not continue moving downwards, because the graph has grown too much
                return (best_graph_size, best_context);
            }
        }
    }
    (best_graph_size, best_context)
}

/// Swap layer containing specified variable first to the bottom of the BDD, then to the top,
/// and then to the position which resulted in smallest BDD size.
/// Optional parameter `max_increase` stops swapping in either direction once the difference
/// between BDD size and current optimum exceeds threshold.
// #[allow(unused)]
fn sift_single_var_in_range(
    man: Arc<DDManager>,
    var: &VarID,
    max_increase: Option<usize>,
    level_range: (usize, usize),
    prev_swap: SwapContext,
) -> (isize, SwapContext) {
    let starting_pos = prev_swap.var2level(&man.var2level, var.0);
    assert!(level_range.0 <= level_range.1);
    assert!(level_range.0 <= starting_pos);
    assert!(level_range.1 >= starting_pos);

    // Move variable to the bottom
    let start_level = level_range.0;
    let end_level = level_range.1;

    let par_result = vec![(start_level, starting_pos), (starting_pos, end_level)]
        .into_par_iter()
        .map(|range| {
            find_best_position_in_range(man.clone(), var, max_increase, range, prev_swap.clone())
        })
        .collect::<Vec<(isize, SwapContext)>>();

    // go to best position
    match par_result
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
                (0, prev_swap)
            } else {
                (evaluation, swaps)
            }
        }
        None => (0, prev_swap),
    }
}

#[derive(Default, Clone, Debug, PartialEq)]
pub struct Sifting {}

impl ConcurrentDVOStrategie for Sifting {
    /// Generates a list of ranges that should be used for concurrent DVO
    fn compute_concurrent_dvo(
        &self,
        man: Arc<DDManager>,
        max_increase: Option<usize>,
        level_range: &(usize, usize),
        swap_context: SwapContext,
    ) -> SwapContext {
        let man = man.clone();

        let vars: Vec<VarID> = (level_range.0..=level_range.1)
            .clone()
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
            (_, result) =
                sift_single_var_in_range(man.clone(), &var, max_increase, *level_range, result);
            // self.purge_retain(f);
        }
        result
    }
}

#[derive(Default, Clone, Debug, PartialEq)]
pub struct WindowPermutation {}

impl ConcurrentDVOStrategie for WindowPermutation {
    fn compute_concurrent_dvo(
        &self,
        man: Arc<DDManager>,
        max_increase: Option<usize>,
        level_range: &(usize, usize),
        swap_context: SwapContext,
    ) -> SwapContext {
        let man = man.clone();

        let start_level = level_range.0;
        let end_level = level_range.1;

        if end_level - start_level > 6 {
            return swap_context;
        }

        let mut current_permutation: Vec<usize> = (start_level..=end_level).collect();
        let mut current_size = 0;
        let mut best_size = 0;
        let mut result = (0, swap_context.clone());
        let mut best_swap: SwapContext = swap_context.clone();

        for (from, to) in gen_permutation(start_level, end_level) {
            let a = result.1.var_at_level(from, &man.var2level);
            let b = result.1.var_at_level(to, &man.var2level);
            let (a, b) = match (a, b) {
                (Some(a), Some(b)) => (a, b),
                _ => continue,
            };

            result = man.partial_swap(a, b, result.1);
            current_size += result.0;
            current_permutation.swap(from - start_level, to - start_level);

            if current_size < best_size {
                log::info!(" New optimum found with order {:?}", current_permutation);
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

pub struct ConcurrentDVO {
    pub strategy: ConcurrentDVOStrategieEnum,
    pub area_selection: AreaSelectionEnum,
}

impl ConcurrentDVO {
    pub fn new(strategy: ConcurrentDVOStrategieEnum, area_selection: AreaSelectionEnum) -> Self {
        Self {
            strategy,
            area_selection,
        }
    }
}

impl DVOStrategie for ConcurrentDVO {
    /// execute the DVO strategy concurrently
    fn run_dvo(&self, manager: &mut DDManager, f: NodeID, max_increase: Option<usize>) -> NodeID {
        let end_level = manager.level2nodes.len() - 1;

        let root_var = manager.nodes.get(&f).unwrap().var;
        let root_level = manager.var2level[root_var.0];

        let areas = &self
            .area_selection
            .select_areas(manager, (root_level + 1, end_level));

        let man = Arc::new(manager.clone());

        let results = areas
            .par_iter()
            .map(|range| {
                self.strategy.compute_concurrent_dvo(
                    man.clone(),
                    max_increase,
                    range,
                    SwapContext::new(&man, range),
                )
            })
            .collect::<Vec<SwapContext>>();

        for result in results {
            manager.persist_swap(result);
        }

        f
    }
}

/// Generate a list of permutations for a range of variables
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

/// Calculate the median of a list of numbers
pub(crate) fn median<T: PartialOrd + Clone + Copy>(list: &[T]) -> T {
    let mut numbers = list.to_vec();
    numbers.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mid = numbers.len() / 2;
    numbers[mid]
}

/// Calculate the nth percentile of a list of numbers
#[allow(unused)]
pub(crate) fn nth_percentile<T: PartialOrd + Clone + Copy>(list: &[T], p: usize) -> T {
    assert!(p <= 100);
    let mut numbers = list.to_vec();
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
    fn compute_concurrent_dvo(
        &self,
        man: Arc<DDManager>,
        max_increase: Option<usize>,
        level_range: &(usize, usize),
        swap_context: SwapContext,
    ) -> SwapContext;
}

/// This contains all available DVO implementations
#[enum_dispatch(ConcurrentDVOStrategie)]
#[derive(Clone, Debug, PartialEq)]
pub enum ConcurrentDVOStrategieEnum {
    WindowPermutation,
    Sifting,
}

impl Default for ConcurrentDVOStrategieEnum {
    fn default() -> Self {
        Sifting::default().into()
    }
}

#[cfg(test)]
mod window_permutation_test {
    use std::sync::Arc;

    use num_traits::abs;

    use crate::core::{
        bdd_manager::DDManager,
        dvo::dvo_strategies::{gen_permutation, ConcurrentDVOStrategie, WindowPermutation},
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
    fn wp_single_test() {
        let _ = env_logger::builder().is_test(true).try_init();

        let (mut man, nodes) =
            DDManager::load_from_dddmp_file("examples/berkeleydb.dimacs.dddmp".to_string())
                .unwrap();
        let bdd = nodes[0];

        let size_before = man.count_active(bdd);
        let manager = Arc::new(man.clone());
        let result = WindowPermutation::default().compute_concurrent_dvo(
            manager.clone(),
            None,
            &(2, 6),
            SwapContext::new(&man, &(2, 6)),
        );
        man.persist_swap(result);
        let size_after = man.count_active(bdd);

        assert!(size_after <= size_before);
    }
}

#[cfg(test)]
mod sifting_test {
    use std::sync::Arc;

    use crate::core::{
        bdd_manager::DDManager,
        bdd_node::VarID,
        dvo::{
            area_generation::{AreaSelection, EqualSplitMethod},
            dvo_strategies::sift_single_var_in_range,
        },
        swap::SwapContext,
    };

    #[test]
    fn sift_vars_in_range() {
        let _ = env_logger::builder().is_test(true).try_init();
        let (mut man, nodes) =
            DDManager::load_from_dddmp_file("examples/sandwich.dimacs.dddmp".to_string()).unwrap();
        let bdd = nodes[0];
        man.purge_retain(bdd);
        let start_level = man.var2level[man.nodes.get(&bdd).unwrap().var.0] + 1;

        let ranges =
            EqualSplitMethod::new(3).select_areas(&man, (start_level, man.level2nodes.len() - 1));

        let expected = man.sat_count(bdd);
        for range in ranges {
            let vars: Vec<VarID> = (range.0..=range.1)
                .filter_map(|level| man.var_at_level(level))
                .collect();

            for var in vars {
                let size_before = man.count_active(bdd);
                let (_, result) = sift_single_var_in_range(
                    Arc::new(man.clone()),
                    &var,
                    None,
                    range,
                    SwapContext::new(&man, &range),
                );
                man.persist_swap(result);
                assert_eq!(man.sat_count(bdd), expected);
                assert!(man.count_active(bdd) <= size_before);
            }
        }
    }
    #[test]
    fn sift_single_var_range() {
        let _ = env_logger::builder().is_test(true).try_init();
        let (mut man, nodes) =
            DDManager::load_from_dddmp_file("examples/sandwich.dimacs.dddmp".to_string()).unwrap();
        let bdd = nodes[0];
        man.purge_retain(bdd);
        let start_level = man.var2level[man.nodes.get(&bdd).unwrap().var.0] + 1;

        let ranges =
            EqualSplitMethod::new(3).select_areas(&man, (start_level, man.level2nodes.len() - 1));

        let range = ranges[1];

        let expected = man.sat_count(bdd);
        let vars: Vec<VarID> = (range.0..=range.1)
            .filter_map(|level| man.var_at_level(level))
            .collect();
        let var = vars[0];

        let size_before = man.count_active(bdd);
        let (_, result) = sift_single_var_in_range(
            Arc::new(man.clone()),
            &var,
            None,
            range,
            SwapContext::new(&man, &range),
        );
        man.persist_swap(result);
        assert_eq!(man.sat_count(bdd), expected);
        assert!(man.count_active(bdd) <= size_before);
        assert_eq!(
            man.var2level,
            vec![20, 1, 2, 3, 4, 5, 6, 7, 8, 9, 12, 10, 11, 13, 14, 15, 16, 17, 18, 19]
        );
    }
}

#[cfg(test)]
mod concurrent_dvo_test {
    use super::{ConcurrentDVO, DVOStrategie, DVOStrategieEnum, Sifting};
    use crate::core::{
        bdd_manager::DDManager,
        dvo::{area_generation::EqualSplitMethod, dvo_strategies::WindowPermutation},
    };

    #[test]
    fn sifting_test() {
        let _ = env_logger::builder().is_test(true).try_init();

        let (mut man, nodes) =
            DDManager::load_from_dddmp_file("examples/sandwich.dimacs.dddmp".to_string()).unwrap();
        let bdd = nodes[0];
        man.purge_retain(bdd);

        let before = man.count_active(bdd);
        let sat_count = man.sat_count(bdd);

        let dvo: DVOStrategieEnum =
            ConcurrentDVO::new(Sifting::default().into(), EqualSplitMethod::new(3).into()).into();

        dvo.run_dvo(&mut man, bdd, None);

        assert!(man.sat_count(bdd) == sat_count);
        assert!(man.count_active(bdd) <= before);
    }
    #[test]
    fn window_permutation_test() {
        let _ = env_logger::builder().is_test(true).try_init();

        let (mut man, nodes) =
            DDManager::load_from_dddmp_file("examples/sandwich.dimacs.dddmp".to_string()).unwrap();
        let bdd = nodes[0];
        man.purge_retain(bdd);

        let before = man.count_active(bdd);
        let sat_count = man.sat_count(bdd);

        let dvo: DVOStrategieEnum = ConcurrentDVO::new(
            WindowPermutation::default().into(),
            EqualSplitMethod::new(3).into(),
        )
        .into();

        dvo.run_dvo(&mut man, bdd, None);

        assert!(man.sat_count(bdd) == sat_count);
        assert!(man.count_active(bdd) <= before);
    }
}
