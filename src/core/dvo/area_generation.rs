use enum_dispatch::enum_dispatch;

use super::dvo_strategies::median;
use crate::core::bdd_manager::DDManager;

#[enum_dispatch(AreaSelection)]
pub trait AreaSelection {
    /// Generates a list of ranges that should be used for concurrent DVO
    fn select_areas(&self, man: &DDManager, range: (usize, usize)) -> Vec<(usize, usize)>;
}

/// This contains all available DVO implementations
#[enum_dispatch(AreaSelection)]
pub enum AreaSelectionEnum {
    ThresholdMethod,
    EqualSplitMethod,
    HotspotMethod,
}

impl Default for AreaSelectionEnum {
    fn default() -> Self {
        ThresholdMethod::default().into()
    }
}

#[derive(Default)]
pub struct ThresholdMethod {}


impl ThresholdMethod {
    pub fn new() -> Self {
        ThresholdMethod {}
    }
}

impl ThresholdMethod {
    fn select_areas_for(
        &self,
        range: (usize, usize),
        distribution: Vec<usize>,
    ) -> Vec<(usize, usize)> {
        let mut result_ranges: Vec<(usize, usize)> = vec![];
        let mut current_range = (0, 0);
        let start_level = range.0;
        let end_level = distribution.len().min(range.1);

        let threshold = median(&distribution);

        for (i, _) in distribution
            .into_iter()
            .enumerate()
            .filter(|(i, a)| a > &threshold && *i > start_level && *i < end_level - 1)
        {
            match current_range {
                (0, 0) => {
                    current_range = (i, i);
                }
                (_, end) => {
                    if i - end <= 2 {
                        // if one variable is below threshold, we still want to include it
                        current_range.1 = i;
                    } else {
                        result_ranges.push(current_range);
                        current_range = (i, i);
                    }
                }
            }
        }
        result_ranges.push(current_range);
        result_ranges
    }
}

impl AreaSelection for ThresholdMethod {
    fn select_areas(&self, man: &DDManager, range: (usize, usize)) -> Vec<(usize, usize)> {
        let mut areas_node_count = self.select_areas_for(range, man.calculate_node_count());
        let mut areas_connection_distance =
            self.select_areas_for(range, man.calculate_connection_distance());

        areas_node_count.append(&mut areas_connection_distance);
        merge_ranges(&areas_node_count)
    }
}

pub struct EqualSplitMethod {
    pub n_splits: usize,
}

impl EqualSplitMethod {
    pub fn new(n_splits: usize) -> Self {
        EqualSplitMethod { n_splits }
    }
}

impl Default for EqualSplitMethod {
    fn default() -> Self {
        EqualSplitMethod::new(num_cpus::get())
    }
}

impl AreaSelection for EqualSplitMethod {
    fn select_areas(&self, man: &DDManager, range: (usize, usize)) -> Vec<(usize, usize)> {
        let distribution = man.calculate_node_count();
        let start_level = range.0;
        assert!(self.n_splits > 0);
        assert!(self.n_splits < distribution.len());

        let area_size = distribution.iter().sum::<usize>() / self.n_splits;

        split_ranges_by_node_count(
            &vec![(start_level + 1, distribution.len() - 2)],
            Some(area_size),
            &distribution,
        )
    }
}

#[derive(Default)]
pub struct HotspotMethod {
    /// The number of layers surrounding a hotspot
    pub surrounding_area: usize,
}

impl HotspotMethod {
    fn select_areas_for(
        &self,
        range: (usize, usize),
        distribution: Vec<usize>,
    ) -> Vec<(usize, usize)> {
        let distribution_len = distribution.len();
        let start_level = range.0;
        let start_level = start_level + self.surrounding_area + 1;
        let mut ranges: Vec<(usize, usize)> = vec![];

        // find all maxima in distribution
        for i in start_level..(distribution_len - self.surrounding_area) - 1 {
            if distribution[i] > distribution[i - 1] && distribution[i] > distribution[i + 1] {
                ranges.push((i - self.surrounding_area, i + self.surrounding_area));
            }
        }
        
        merge_ranges(&ranges)
    }
}

impl AreaSelection for HotspotMethod {
    fn select_areas(&self, man: &DDManager, range: (usize, usize)) -> Vec<(usize, usize)> {
        let mut areas_node_count = self.select_areas_for(range, man.calculate_node_count());
        let mut areas_connection_distance =
            self.select_areas_for(range, man.calculate_connection_distance());

        areas_node_count.append(&mut areas_connection_distance);
        merge_ranges(&areas_node_count)
    }
}

/// Merges overlapping areas
fn merge_ranges(ranges: &Vec<(usize, usize)>) -> Vec<(usize, usize)> {
    let mut result = vec![];
    let mut current_range = (0, 0);
    let mut ranges = ranges.clone();
    ranges.sort_by_key(|k| k.0);
    for (start, end) in ranges {
        match current_range {
            (0, 0) => {
                current_range = (start, end);
            }
            (_, current_end) => {
                if (start - 1).cmp(&current_end) == std::cmp::Ordering::Less
                    || (start - 1).cmp(&current_end) == std::cmp::Ordering::Equal
                {
                    if end > current_end {
                        current_range.1 = end;
                    }
                } else {
                    result.push(current_range);
                    current_range = (start, end);
                }
            }
        }
    }
    result.push(current_range);
    result
}

/// Splits ranges into smaller ranges with equal node count
pub(crate) fn split_ranges_by_node_count(
    ranges: &Vec<(usize, usize)>,
    max_size: Option<usize>,
    node_count: &Vec<usize>,
) -> Vec<(usize, usize)> {
    if max_size.is_none() {
        return ranges.clone();
    }
    let area_size = max_size.unwrap();
    let mut result = vec![];
    for (start, end) in ranges {
        if end - start <= 1 {
            continue; // area to small for dvo
        }

        let range_size = (*start..=*end).map(|i| node_count[i]).sum::<usize>();

        // else we add area as range or split it into smaller ranges
        if range_size <= area_size {
            result.push((*start, *end));
        } else {
            // split into ranges with equal node count
            let range_count = (range_size as f64 / area_size as f64).ceil() as usize;
            let range_count = range_size / range_count;

            let mut current_range = 0;
            let mut current_start = *start;

            for i in *start..*end {
                current_range += node_count[i];
                if current_range >= range_count {
                    result.push((current_start, i));
                    current_start = i + 1;
                    current_range = 0;
                }
            }
            result.push((current_start, *end));
            assert!(result.last().unwrap().1 == *end, "last range != end");
        }
    }
    result
}

impl DDManager {
    /// Calculate the number of nodes for each level
    pub(crate) fn calculate_node_count(&self) -> Vec<usize> {
        self.level2nodes
            .clone()
            .into_iter()
            .map(|level| level.len())
            .collect::<Vec<usize>>()
    }
    /// Calculate distance between nodes and their children for each level
    pub(crate) fn calculate_connection_distance(&self) -> Vec<usize> {
        self.level2nodes
            .clone()
            .into_iter()
            .map(|level| {
                let sum = level
                    .iter()
                    .map(|node| {
                        let mut result = 0;
                        let this = self.var2level[node.var.0];
                        let high = self.nodes.get(&node.high).unwrap();
                        result += match high.var.0 {
                            0 | 1 => 0,
                            _ => self.var2level[high.var.0] - this,
                        };
                        let low = self.nodes.get(&node.low).unwrap();
                        result += match low.var.0 {
                            0 | 1 => 0,
                            _ => self.var2level[low.var.0] - this,
                        };
                        result
                    })
                    .sum::<usize>();
                sum
            })
            .collect::<Vec<usize>>()
    }
}

#[cfg(test)]
mod test {
    use crate::core::{
        bdd_manager::DDManager,
        dvo::area_generation::{AreaSelection, EqualSplitMethod, HotspotMethod, ThresholdMethod},
    };

    #[test]
    fn merge_ranges_test() {
        let ranges = vec![(0, 1), (2, 3), (4, 5), (6, 7), (8, 9)];
        let result = super::merge_ranges(&ranges);
        assert_eq!(result, vec![(0, 9)]);

        let ranges = vec![(0, 1), (8, 9), (2, 3), (6, 7)];
        let result = super::merge_ranges(&ranges);
        assert_eq!(result, vec![(0, 3), (6, 9)]);

        let ranges = vec![(0, 3), (8, 9), (2, 3), (6, 10)];
        let result = super::merge_ranges(&ranges);
        assert_eq!(result, vec![(0, 3), (6, 10)]);
    }
    #[test]
    fn threshold_method_test() {
        let (mut man, nodes) =
            DDManager::load_from_dddmp_file("examples/berkeleydb.dimacs.dddmp".to_string())
                .unwrap();
        let bdd = nodes[0];

        man.purge_retain(bdd);

        let method = ThresholdMethod::default();
        let areas = method.select_areas(&man, (0, man.level2nodes.len()));

        assert_eq!(areas, vec![(7, 24), (31, 36), (41, 62), (65, 66)]);
    }

    #[test]
    fn equal_split_method_test() {
        let (mut man, nodes) =
            DDManager::load_from_dddmp_file("examples/berkeleydb.dimacs.dddmp".to_string())
                .unwrap();
        let bdd = nodes[0];

        man.purge_retain(bdd);

        let method = EqualSplitMethod { n_splits: 4 };
        let areas = method.select_areas(&man, (0, man.level2nodes.len()));

        assert_eq!(areas, vec![(1, 22), (23, 43), (44, 51), (52, 58), (59, 76)]);
    }
    #[test]
    fn hotspot_method_test() {
        let (mut man, nodes) =
            DDManager::load_from_dddmp_file("examples/berkeleydb.dimacs.dddmp".to_string())
                .unwrap();
        let bdd = nodes[0];

        man.purge_retain(bdd);

        let method = HotspotMethod {
            surrounding_area: 1,
        };
        let areas = method.select_areas(&man, (0, man.level2nodes.len()));

        assert_eq!(
            areas,
            vec![(3, 10), (13, 19), (21, 27), (30, 47), (52, 59), (61, 66)]
        );
    }
}
