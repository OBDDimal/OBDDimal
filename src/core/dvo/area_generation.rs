use enum_dispatch::enum_dispatch;

use crate::core::bdd_manager::DDManager;

// impl Default for AreaSelectionEnum {
//     fn default() -> Self {
//         Self::ThresholdMethod
//     }
// }

pub struct ThresholdMethod {
    pub offset: Option<usize>,
}

impl Default for ThresholdMethod {
    fn default() -> Self {
        ThresholdMethod { offset: Some(0) }
    }
}

impl ThresholdMethod {
    pub fn new(offset: Option<usize>) -> Self {
        ThresholdMethod { offset }
    }
}

impl AreaSelection for ThresholdMethod {
    fn generate_area(
        &self,
        distribution: Vec<usize>,
        area_size: Option<usize>,
        threshold: Option<usize>,
        start_level: Option<usize>,
    ) -> Vec<(usize, usize)> {
        let distribution_len = distribution.len();
        let threshold = threshold.unwrap_or(0);
        let mut ranges: Vec<(usize, usize)> = vec![];
        let mut current_range = (0, 0);
        let offset = self.offset.unwrap_or(0);
        let start_level = start_level.unwrap_or(0) + offset;

        for (i, _) in distribution
            .into_iter()
            .enumerate()
            .filter(|(i, a)| a > &threshold && *i > start_level && *i < distribution_len - 1)
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
                        ranges.push(current_range);
                        current_range = (i, i);
                    }
                }
            }
        }
        ranges.push(current_range);
        split_ranges(ranges, area_size)
    }
}

#[derive(Default)]
pub struct EqualSplitMethod {}

impl AreaSelection for EqualSplitMethod {
    #[allow(unused_variables)]
    fn generate_area(
        &self,
        distribution: Vec<usize>,
        area_size: Option<usize>,
        threshold: Option<usize>,
        start_level: Option<usize>,
    ) -> Vec<(usize, usize)> {
        let distribution_len = distribution.len();
        let start_level = start_level.unwrap_or(0);

        split_ranges(vec![(start_level + 1, distribution_len - 2)], area_size)
    }
}

#[derive(Default)]
pub struct NSplitMethod {}

impl AreaSelection for NSplitMethod {
    #[allow(unused_variables)]
    fn generate_area(
        &self,
        distribution: Vec<usize>,
        area_size: Option<usize>,
        threshold: Option<usize>,
        start_level: Option<usize>,
    ) -> Vec<(usize, usize)> {
        let distribution_len = distribution.len();
        let start_level = start_level.unwrap_or(0);
        let area_size = area_size.unwrap_or(1);

        let size = (distribution_len - 1) - start_level;

        let range_size = (size as f64 / area_size as f64).ceil() as usize;
        split_ranges(
            vec![(start_level + 1, distribution_len - 2)],
            Some(range_size),
        )
    }
}

#[derive(Default)]
pub struct HotspotMethod {}

impl AreaSelection for HotspotMethod {
    fn generate_area(
        &self,
        distribution: Vec<usize>,
        area_size: Option<usize>,
        threshold: Option<usize>,
        start_level: Option<usize>,
    ) -> Vec<(usize, usize)> {
        let distribution_len = distribution.len();
        let threshold = threshold.unwrap_or(2);
        let start_level = start_level.unwrap_or(0);
        let start_level = start_level + threshold + 1;
        let mut ranges: Vec<(usize, usize)> = vec![];

        // find all maxima in distribution
        for i in start_level..(distribution_len - threshold) - 1 {
            if distribution[i] > distribution[i - 1] && distribution[i] > distribution[i + 1] {
                ranges.push((i - threshold, i + threshold));
            }
        }
        let ranges = merge_ranges(&ranges);
        split_ranges(ranges, area_size)
    }
}

pub(crate) fn merge_ranges(ranges: &Vec<(usize, usize)>) -> Vec<(usize, usize)> {
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

fn split_ranges(ranges: Vec<(usize, usize)>, max_size: Option<usize>) -> Vec<(usize, usize)> {
    if let None = max_size {
        return ranges;
    }
    let area_size = max_size.unwrap();
    let mut result = vec![];
    for (start, end) in ranges {
        if end - start <= 1 {
            continue; // area to small for dvo
        }

        // else we add area as range or split it into smaller ranges
        if (end - start) + 1 < area_size {
            result.push((start, end));
        } else {
            // split into equal ranges
            let current_len = (end - start) + 1;
            let range_count = (current_len as f64 / area_size as f64).ceil() as usize;
            let step = current_len / range_count;
            let orphan = current_len - (range_count * step);
            // println!("orphans: {}", orphan);
            // println!("range_count: {}", range_count);
            // println!("step: {}", step);
            // println!("current_len: {}", current_len);
            assert!(orphan < range_count, "orphan >= range_count");
            if range_count * step > current_len {
                assert!(false, "range_count * step > current_len");
            }
            for i in 0..range_count {
                if i < orphan {
                    result.push((i + start + i * step, i + start + ((i + 1) * step)));
                } else {
                    result.push((
                        orphan + start + i * step,
                        orphan + start + ((i + 1) * step) - 1,
                    ));
                }
                // ranges.push((start + i * step, start + ((i + 1) * step) - 1));
            }
            if result.last().unwrap().1 != end {
                panic!("last range != end");
                // let (a, b) = result.pop().unwrap();
                // result.push((a, end));
            }
            // println!(
            //     "Split range {} - {} into {} result | {}",
            //     start,
            //     end,
            //     range_count,
            //     result.last().unwrap().1
            // );
            assert!(result.last().unwrap().1 == end, "last range != end");
        }
    }
    result
}

impl DDManager {
    pub(crate) fn calculate_node_count(&self) -> Vec<usize> {
        self.level2nodes
            .clone()
            .into_iter()
            .map(|level| level.len())
            .collect::<Vec<usize>>()
    }
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

#[allow(unused)]
fn area_valley_method(
    distribution: Vec<usize>,
    area_size: Option<usize>,
    threshold: Option<usize>,
) -> Vec<(usize, usize)> {
    let threshold = threshold.unwrap_or(0);

    let mut valleys: Vec<(usize, usize, f64)> = vec![];

    // find valleys in distribution
    for i in 2..distribution.len() - 2 {
        let temp = ((distribution[i - 2]
            + distribution[i - 1]
            + distribution[i]
            + distribution[i + 1]
            + distribution[i + 2]) as f64
            / 5.0)
            - (distribution[i] as f64);
        if temp > 0.0 {
            valleys.push((i, distribution[i], temp));
        }
    }

    // let med = median(&valleys.iter().map(|(_, _, a)| *a).collect::<Vec<f64>>());
    // // println!("Median: {}", med);
    // valleys.retain(|(_, _, a)| *a > med);

    // let mut max_valleys: Vec<(usize, usize, usize)> = vec![];
    // let mut last_biggest: usize = usize::MAX;
    // // get max values in ranges
    // for i in 0..valleys.len() - 1 {
    //     let mut max = valleys[i].1;
    //     for j in valleys[i].0..valleys[i + 1].0 {
    //         if distribution[j] > max {
    //             max = distribution[j];
    //         }
    //     }
    //     max_valleys.push((
    //         valleys[i].0,
    //         valleys[i].1,
    //         last_biggest.min(max) - valleys[i].1,
    //     ));
    //     last_biggest = max;
    // }

    // max_valleys.push((
    //     valleys.last().unwrap().0,
    //     valleys.last().unwrap().1,
    //     usize::MAX,
    // ));

    // let valleys = max_valleys
    //     .into_iter()
    //     .filter(|(_, _, m)| m > &(0 as usize))
    //     .collect::<Vec<(usize, usize, usize)>>();

    // remove valleys that are not the lowest in their range or on a decline
    // let valleys_temp = valleys
    //     .clone()
    //     .into_iter()
    //     .enumerate()
    //     .filter(|(i, (x, y, a))| {
    //         if *i == 0 as usize || *i == valleys.len() - 1 {
    //             return true;
    //         }
    //         if &valleys[i - 1].1 < y && &valleys[i + 1].1 < y {
    //             return false;
    //         }
    //         if &valleys[i - 1].1 <= y && &valleys[i + 1].1 < y {
    //             return false;
    //         }
    //         if &valleys[i - 1].1 < y && &valleys[i + 1].1 <= y {
    //             return false;
    //         }
    //         return true;
    //     })
    //     .map(|(i, (x, y, a))| (x, y, a))
    //     .collect::<Vec<(usize, usize, f64)>>();
    // valleys = valleys_temp;

    println!(
        "Valleys: {:?}",
        valleys
            .into_iter()
            .map(|(i, y, _)| (i, y))
            .collect::<Vec<(usize, usize)>>()
    );

    let mut ranges: Vec<(usize, usize)> = vec![];
    split_ranges(ranges, area_size)
}

/// This contains all available DVO implementations
#[enum_dispatch(AreaSelection)]
pub enum AreaSelectionEnum {
    EqualSplitMethod,
    NSplitMethod,
    ThresholdMethod,
    HotspotMethod,
}
/// Implements generate_area()
#[enum_dispatch(AreaSelection)]
pub trait AreaSelection {
    /// Generates a list of ranges that should be used for concurrent DVO
    fn generate_area(
        &self,
        distribution: Vec<usize>,
        area_size: Option<usize>,
        threshold: Option<usize>,
        start_level: Option<usize>,
    ) -> Vec<(usize, usize)>;
}

impl Default for AreaSelectionEnum {
    fn default() -> Self {
        EqualSplitMethod::default().into()
    }
}
#[cfg(test)]
mod test {
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
}

#[cfg(test)]
mod evaluation {
    use std::{fs, io::Write};

    use crate::core::{
        bdd_manager::DDManager,
        dvo::{
            area_generation::{
                merge_ranges, AreaSelection, HotspotMethod, NSplitMethod, ThresholdMethod,
            },
            dvo_strategies::{median, nth_percentile},
        },
    };

    static PATH: &str = "examples/berkeleydb.dimacs.dddmp";
    // static PATH: &str = "examples/financialServices01.dimacs.dddmp";
    // static PATH: &str = "examples/automotive02v4.dimacs.dddmp";
    // static PATH: &str = "examples/automotive01.dimacs.dddmp";

    static NAME: &str = "automotive01";

    #[test]
    fn info() {
        let (mut man, nodes) = DDManager::load_from_dddmp_file(PATH.to_string()).unwrap();
        let bdd = nodes[0];
        man.purge_retain(bdd);

        let nodes = man.calculate_node_count();
        let connection_distance = man.calculate_connection_distance();

        println!("Model: {}", PATH);
        println!("median nodes: {}", median(&nodes));
        println!(
            "median connection distance: {}",
            median(&connection_distance)
        );
        println!("mean nodes: {}", nodes.iter().sum::<usize>() / nodes.len());
        println!(
            "mean connection distance: {}",
            connection_distance.iter().sum::<usize>() / connection_distance.len()
        );
        println!("Level: {}", nodes.len());
        println!("Nodes sum: {}", nodes.iter().sum::<usize>());
        println!(
            "Connection distance sum: {}",
            connection_distance.iter().sum::<usize>()
        );

        println!("nodes max: {}", nodes.iter().max().unwrap());
        println!(
            "connection dist max: {}",
            connection_distance.iter().max().unwrap()
        );
        let len = nodes.len();
        let nodes_median = median(&nodes);
        let connection_median = median(&connection_distance);
        println!("\\addplot[mark=none, black] coordinates {{(0,{nodes_median}) ({len},{nodes_median})}};");
        println!("\\addplot[mark=none, black] coordinates {{(0,{connection_median}) ({len},{connection_median})}};");

        let mut file = fs::File::create(format!("evaluation-area-selection-{}.csv", NAME)).unwrap();
        let mut wtr = csv::Writer::from_writer(vec![]);
        wtr.write_record(vec!["layer", "nodes", "connection-distance"])
            .unwrap();
        for i in 0..nodes.len() {
            wtr.write_record(vec![
                i.to_string(),
                nodes[i].to_string(),
                connection_distance[i].to_string(),
            ])
            .unwrap();
        }
        let data = String::from_utf8(wtr.into_inner().unwrap()).unwrap();
        file.write_all(data.as_bytes()).unwrap();
    }

    #[test]
    fn node_distribution() {
        let (mut man, nodes) = DDManager::load_from_dddmp_file(PATH.to_string()).unwrap();
        let bdd = nodes[0];
        man.purge_retain(bdd);

        let nodes = man.calculate_node_count();

        println!("75th percentile: {}", nth_percentile(&nodes, 75));
        println!("70th percentile: {}", nth_percentile(&nodes, 70));
        println!("60th percentile: {}", nth_percentile(&nodes, 60));
        println!("50th percentile: {}", nth_percentile(&nodes, 50));
        println!("40th percentile: {}", nth_percentile(&nodes, 40));
        println!("30th percentile: {}", nth_percentile(&nodes, 30));
    }
    #[test]
    fn node_connection_distance() {
        let (mut man, nodes) = DDManager::load_from_dddmp_file(PATH.to_string()).unwrap();
        let bdd = nodes[0];
        man.purge_retain(bdd);

        let nodes = man.calculate_connection_distance();

        println!("median: {}", median(&nodes));
        println!("75th percentile: {}", nth_percentile(&nodes, 75));
        println!("70th percentile: {}", nth_percentile(&nodes, 70));
        println!("60th percentile: {}", nth_percentile(&nodes, 60));
        println!("50th percentile: {}", nth_percentile(&nodes, 50));
        println!("40th percentile: {}", nth_percentile(&nodes, 40));
        println!("30th percentile: {}", nth_percentile(&nodes, 30));
    }

    fn print_highlighted_ranges((from, to): (usize, usize), i: usize, max: Option<&str>) {
        let max = max.unwrap_or("MAX");
        if i % 2 == 0 {
            println!("\\addplot[fill=blue, opacity=0.15, draw=none, forget plot] coordinates {{({},{max}) ({},{max})}} \\closedcycle;", from, to);
        } else {
            println!("\\addplot[fill=lightgray, opacity=0.35, draw=none, forget plot] coordinates {{({},{max}) ({},{max})}} \\closedcycle;", from, to);
        }
    }

    #[test]
    fn equal_split() {
        let (mut man, nodes) = DDManager::load_from_dddmp_file(PATH.to_string()).unwrap();
        let bdd = nodes[0];
        man.purge_retain(bdd);
        let start_level = man.var2level[man.nodes.get(&bdd).unwrap().var.0];
        let nodes = man.calculate_node_count();

        // let ranges =
        //     EqualSplitMethod::default().generate_area(nodes, Some(11), None, Some(start_level));
        let ranges =
            NSplitMethod::default().generate_area(nodes.clone(), Some(11), None, Some(start_level));
        println!("{:?}", ranges);

        for (i, (from, to)) in ranges.iter().enumerate() {
            print_highlighted_ranges(
                (*from, *to),
                i,
                Some(nodes.iter().max().unwrap().to_string().as_str()),
            );
        }
    }

    #[test]
    fn threshold_method() {
        let (mut man, nodes) = DDManager::load_from_dddmp_file(PATH.to_string()).unwrap();
        let bdd = nodes[0];
        man.purge_retain(bdd);

        let start_level = man.var2level[man.nodes.get(&bdd).unwrap().var.0];

        let nodes = man.calculate_node_count();
        let connection_dist = man.calculate_connection_distance();

        let max = nodes
            .iter()
            .max()
            .unwrap()
            .max(connection_dist.iter().max().unwrap());

        let mut ranges = ThresholdMethod::default().generate_area(
            nodes.clone(),
            None,
            Some(median(&nodes)),
            Some(start_level),
        );
        println!("nodes: {:?}", ranges);
        println!("Nodes ranges:");
        // for (from, to) in ranges.iter() {
        //     print_highlighted_ranges((*from, *to), "lightgray", None);
        // }

        let mut connection_dist_ranges = ThresholdMethod::default().generate_area(
            connection_dist.clone(),
            None,
            Some(median(&connection_dist)),
            Some(start_level),
        );
        println!("connection ranges");
        println!("connection: {:?}", connection_dist_ranges);
        // for (from, to) in connection_dist_ranges.iter() {
        //     print_highlighted_ranges((*from, *to), "blue", None);
        // }

        ranges.append(&mut connection_dist_ranges);
        let ranges = merge_ranges(&ranges);
        println!("Merged ranges");
        println!("{:?}", ranges);
        for (i, (from, to)) in ranges.iter().enumerate() {
            print_highlighted_ranges((*from, *to), i, Some(max.to_string().as_str()));
        }
    }

    #[test]
    fn hotspot_method() {
        let (mut man, nodes) = DDManager::load_from_dddmp_file(PATH.to_string()).unwrap();
        let bdd = nodes[0];
        man.purge_retain(bdd);

        let start_level = man.var2level[man.nodes.get(&bdd).unwrap().var.0];

        let nodes = man.calculate_node_count();
        let connection_dist = man.calculate_connection_distance();

        let max = nodes
            .iter()
            .max()
            .unwrap()
            .max(connection_dist.iter().max().unwrap());

        let n_nodes = 3.max(nodes.len() / 100);

        let mut ranges_node = HotspotMethod::default().generate_area(
            nodes.clone(),
            None,
            Some(n_nodes),
            Some(start_level),
        );
        let mut ranges_dist = HotspotMethod::default().generate_area(
            connection_dist.clone(),
            None,
            Some(n_nodes / 3),
            Some(start_level),
        );
        // println!("{:?}", ranges_node);
        // println!("{:?}", ranges_dist);
        ranges_node.append(&mut ranges_dist);
        let ranges = merge_ranges(&ranges_node);
        println!("{:?}", ranges);
        for (i, (from, to)) in ranges.iter().enumerate() {
            print_highlighted_ranges((*from, *to), i, Some(max.to_string().as_str()));
        }
    }
}

#[cfg(test)]
mod evaluation_dvo {
    use std::{fs, io::Write, sync::Arc, time::Instant};

    use futures::future;
    use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
    use tokio::{runtime::Runtime, task::JoinHandle};

    use crate::core::{
        bdd_manager::DDManager,
        dvo::{
            area_generation::{
                merge_ranges, split_ranges, AreaSelection, HotspotMethod, NSplitMethod,
                ThresholdMethod,
            },
            dvo_strategies::{
                median, nth_percentile, ConcurrentDVO, ConcurrentDVOStrategie, Sifting, SiftingTwo,
            },
        },
        swap::SwapContext,
    };

    // static PATH: &str = "examples/berkeleydb.dimacs.dddmp";
    // static PATH: &str = "examples/financialServices01.dimacs.dddmp";
    // static PATH: &str = "examples/automotive02v4.dimacs.dddmp";
    // static PATH: &str = "examples/automotive01.dimacs.dddmp";

    static MODELS: [&str; 1] = [
        // "examples/berkeleydb.dimacs.dddmp",
        // "examples/financialServices01.dimacs.dddmp",
        // "examples/automotive02v4.dimacs.dddmp",
        "examples/automotive01.dimacs.dddmp",
    ];
    // static MODELS: [&str; 4] = [
    //     "examples/berkeleydb.dimacs.dddmp",
    //     "examples/financialServices01.dimacs.dddmp",
    //     "examples/automotive02v4.dimacs.dddmp",
    //     "examples/automotive01.dimacs.dddmp",
    // ];

    static N: usize = 1;

    fn run_dvo_on(ranges: Vec<(usize, usize)>, manager: &mut DDManager) {
        let man = Arc::new(manager.clone());
        let runtime = Runtime::new().unwrap();

        let max_increase = manager
            .level2nodes
            .iter()
            .map(|level| level.len())
            .max()
            .unwrap()
            / 2;
        println!("Max increase: {}", max_increase);
        // let ranges = vec![ranges[9]];

        let futures = ranges
            .iter()
            .map(|(start, end)| {
                let start = *start;
                let end = *end;
                let man = man.clone();
                let mut swap_context = SwapContext::new();
                swap_context.precalc_references(&man.clone(), start, end);
                runtime.spawn_blocking(move || {
                    SiftingTwo::default().compute_concurrent_dvo(
                        man,
                        Some(max_increase),
                        start..=end,
                        swap_context,
                    )
                })
            })
            .collect::<Vec<JoinHandle<SwapContext>>>();

        let results = runtime.block_on(future::join_all(futures));
        for result in results {
            let result = result.unwrap();
            manager.persist_swap(result);
        }

        // let manager = Arc::new(man.clone());

        // let results = ranges
        //     .par_iter()
        //     .map(|(from, to)| {
        //         Sifting::default().compute_concurrent_dvo(
        //             manager.clone(),
        //             None,
        //             *from..=*to,
        //             SwapContext::new(),
        //         )
        //     })
        //     .collect::<Vec<SwapContext>>();

        // for result in results {
        //     man.persist_swap(result);
        // }
    }

    #[test]
    fn equal_split() {
        println!("Equal split");
        for model in MODELS.iter() {
            println!("Model: {}", model);
            let (mut man, nodes) = DDManager::load_from_dddmp_file(model.to_string()).unwrap();
            let bdd = nodes[0];
            man.purge_retain(bdd);
            let start_level = man.var2level[man.nodes.get(&bdd).unwrap().var.0];
            let nodes = man.calculate_node_count();

            // let ranges =
            //     EqualSplitMethod::default().generate_area(nodes, Some(11), None, Some(start_level));
            let ranges = NSplitMethod::default().generate_area(
                nodes.clone(),
                Some(11),
                None,
                Some(start_level),
            );

            // let max_size = nodes.len() / 10;
            // let ranges = split_ranges(ranges, Some(max_size));
            let before = man.count_active(bdd);
            println!("before: {}", before);
            let start = Instant::now();
            for _ in 0..N {
                let mut man = man.clone();
                run_dvo_on(ranges.clone(), &mut man);
                let after = man.count_active(bdd);
                println!(
                    "after: {} = {}%",
                    after,
                    (before - after) as f64 / before as f64
                );
            }
            println!("Time: {:?}", start.elapsed() / N as u32);
        }
    }

    #[test]
    fn threshold_method() {
        println!("Threshold method");
        for model in MODELS.iter() {
            println!("Model: {}", model);
            let (mut man, nodes) = DDManager::load_from_dddmp_file(model.to_string()).unwrap();
            let bdd = nodes[0];
            man.purge_retain(bdd);

            let start_level = man.var2level[man.nodes.get(&bdd).unwrap().var.0];

            let nodes = man.calculate_node_count();
            let connection_dist = man.calculate_connection_distance();

            let mut ranges = ThresholdMethod::default().generate_area(
                nodes.clone(),
                None,
                Some(median(&nodes)),
                Some(start_level), 
            );

            let mut connection_dist_ranges = ThresholdMethod::default().generate_area(
                connection_dist.clone(),
                None,
                Some(median(&connection_dist)),
                Some(start_level),
            );

            ranges.append(&mut connection_dist_ranges);
            let ranges = merge_ranges(&ranges);

            let max_size = nodes.len() / 11;
            let ranges = split_ranges(ranges, Some(max_size));

            let before = man.count_active(bdd);
            println!("before: {}", before);
            let start = Instant::now();
            for _ in 0..N {
                let mut man = man.clone();
                run_dvo_on(ranges.clone(), &mut man);
                let after = man.count_active(bdd);
                println!(
                    "after: {} = {}%",
                    after,
                    (before - after) as f64 / before as f64
                );
            }
            println!("Time: {:?}", start.elapsed() / N as u32);
        }
    }

    #[test]
    fn hotspot_method() {
        println!("Hotspot method");
        for model in MODELS.iter() {
            println!("Model: {}", model);
            let (mut man, nodes) = DDManager::load_from_dddmp_file(model.to_string()).unwrap();
            let bdd = nodes[0];
            man.purge_retain(bdd);

            let start_level = man.var2level[man.nodes.get(&bdd).unwrap().var.0];

            let nodes = man.calculate_node_count();
            let connection_dist = man.calculate_connection_distance();

            let n_nodes = 3.max(nodes.len() / 100);

            let mut ranges_node = HotspotMethod::default().generate_area(
                nodes.clone(),
                None,
                Some(n_nodes),
                Some(start_level),
            );
            let mut ranges_dist = HotspotMethod::default().generate_area(
                connection_dist.clone(),
                None,
                Some(n_nodes / 3),
                Some(start_level),
            );

            ranges_node.append(&mut ranges_dist);
            let ranges = merge_ranges(&ranges_node);

            let max_size = nodes.len() / 11;
            let ranges = split_ranges(ranges, Some(max_size));

            let before = man.count_active(bdd);
            println!("before: {}", before);
            let start = Instant::now();
            for _ in 0..N {
                let mut man = man.clone();
                run_dvo_on(ranges.clone(), &mut man);
                let after = man.count_active(bdd);
                println!(
                    "after: {} = {}%",
                    after,
                    (before - after) as f64 / before as f64
                );
            }
            println!("Time: {:?}", start.elapsed() / N as u32);
        }
    }

    #[test]
    fn run_all() {
        // equal_split();
        threshold_method();
        // hotspot_method();
    }
}
