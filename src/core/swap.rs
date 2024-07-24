//! Implementation of BDD layer swap

use std::{
    collections::HashSet,
    sync::{Arc, RwLock},
};

use crate::core::{
    bdd_manager::DDManager,
    bdd_node::{DDNode, NodeID, VarID},
    order::var2level_to_ordered_varids,
};

// Stores temporary nodes during swap, which are then inserted into the unique-table
#[derive(Debug, PartialEq, Eq)]
struct TempNode {
    id: NodeID,
    var: VarID,
    low: ChildEnum,
    high: ChildEnum,
}

// Stores temporary children during swap, which are then inserted into the unique-table
#[derive(Debug, PartialEq, Eq)]
struct TempChild {
    id: NodeID,
    var: VarID,
    low: NodeID,
    high: NodeID,
}

// Enum to store either a new child or an existing child
#[derive(Debug, PartialEq, Eq)]
enum ChildEnum {
    NewChild(TempChild),
    OldChild(NodeID),
}

impl DDManager {
    pub async fn async_swap(manager: Arc<RwLock<Self>>, a: VarID, b: VarID) -> i32 {
        // If b above a, switch a and b

        let mut additional_v2l_upper = HashSet::<DDNode>::new();
        let mut new_v2l_lower = HashSet::<DDNode>::new();

        let mut new_nodes: Vec<TempNode> = vec![];

        let (upper_level, lower_level, size_before) = {
            let manager = manager.read().unwrap();
            let (a, b) = if manager.var2level[b.0] < manager.var2level[a.0] {
                (b, a)
            } else {
                (a, b)
            };
            log::info!(
                "Swapping variables {:?} and {:?} (layers {}({}) and {}({}))",
                a,
                b,
                manager.var2level[a.0],
                manager.level2nodes[manager.var2level[a.0]].len(),
                manager.var2level[b.0],
                manager.level2nodes[manager.var2level[b.0]].len()
            );
            println!(
                "Swapping variables {:?} and {:?} (layers {}({}) and {}({}))",
                a,
                b,
                manager.var2level[a.0],
                manager.level2nodes[manager.var2level[a.0]].len(),
                manager.var2level[b.0],
                manager.level2nodes[manager.var2level[b.0]].len()
            );
            assert!(a.0 != 0 && b.0 != 0);
            let upper_level = manager.var2level[a.0];
            let lower_level = manager.var2level[b.0];
            assert_eq!(
                lower_level,
                upper_level + 1,
                "Variables not on adjacent layers!"
            );

            let size_before =
                manager.level2nodes[upper_level].len() + manager.level2nodes[lower_level].len();

            let ids = manager.level2nodes[upper_level]
                .iter()
                .map(|n| n.id)
                .collect::<Vec<NodeID>>();

            for id in ids {
                log::debug!("Replacing node {:?}.", id);
                let f_id = id;

                let old_f_node = manager.nodes[&f_id];

                let f_1_id = old_f_node.high;
                let f_0_id = old_f_node.low;

                let f_0_node = manager.nodes[&f_0_id];
                let f_1_node = manager.nodes[&f_1_id];

                if f_0_node.var != b && f_1_node.var != b {
                    // f does not have connections to level directly below, we leave it as it is.
                    log::debug!(
                        "Children of node {:?} more than one level below, leaving as is.",
                        f_id
                    );

                    new_v2l_lower.insert(old_f_node);
                    continue;
                }
                log::debug!("Replacing node {:?} old_f_node={:?}", f_id, old_f_node);

                let (f_01_id, f_00_id) = if f_0_node.var == b {
                    (f_0_node.high, f_0_node.low)
                } else {
                    (f_0_id, f_0_id)
                };
                let (f_11_id, f_10_id) = if f_1_node.var == b {
                    (f_1_node.high, f_1_node.low)
                } else {
                    (f_1_id, f_1_id)
                };

                {
                    let new_then_child = if f_01_id == f_11_id {
                        ChildEnum::OldChild(f_01_id)
                    } else {
                        // self.level2nodes[self.var2level[node.var.0]].get(node)
                        let maybe_node = manager.level2nodes[upper_level].get(&DDNode {
                            id: NodeID(0),
                            var: a,
                            high: f_11_id,
                            low: f_01_id,
                        });
                        if let Some(node) = maybe_node {
                            new_v2l_lower.insert(node.clone());
                            ChildEnum::OldChild(node.id)
                        } else {
                            ChildEnum::NewChild(TempChild {
                                id: NodeID(0),
                                var: a,
                                high: f_11_id,
                                low: f_01_id,
                            })
                        }
                    };

                    let new_else_child = if f_00_id == f_10_id {
                        ChildEnum::OldChild(f_00_id)
                    } else {
                        let maybe_node = manager.level2nodes[upper_level].get(&DDNode {
                            id: NodeID(0),
                            var: a,
                            high: f_10_id,
                            low: f_00_id,
                        });
                        if let Some(node) = maybe_node {
                            new_v2l_lower.insert(node.clone());
                            ChildEnum::OldChild(node.id)
                        } else {
                            ChildEnum::NewChild(TempChild {
                                id: NodeID(0),
                                var: a,
                                high: f_10_id,
                                low: f_00_id,
                            })
                        }
                    };

                    assert_ne!(new_then_child, new_else_child);

                    let new_f_node = TempNode {
                        id: f_id,
                        var: b,
                        high: new_then_child,
                        low: new_else_child,
                    };

                    log::debug!("new_f_node={:?}", new_f_node);

                    new_nodes.push(new_f_node);
                }
            }

            // This swap generates some dangling nodes, which are stored in nodes and are referenced by other nodes, but not stored in the level2nodes.
            // This causes the swap to be incorrect in certain cases without the reduce function.
            // To fix this we search for nodes that have references to the lower_level nodes and add those referenced lower_level nodes to the new upper level.
            let lower_level_ids = manager.level2nodes[lower_level]
                .iter()
                .map(|n| n.id)
                .collect::<Vec<NodeID>>();

            let mut lower_level_root = vec![true; lower_level_ids.len()];

            for (level, level_nodes) in manager.level2nodes[0..=upper_level].iter().enumerate() {
                level_nodes.iter().for_each(|node| {
                    if lower_level_ids.contains(&node.high) {
                        lower_level_root[lower_level_ids
                            .iter()
                            .position(|&x| x == node.high)
                            .unwrap()] = false;
                    }
                    if lower_level_ids.contains(&node.low) {
                        lower_level_root
                            [lower_level_ids.iter().position(|&x| x == node.low).unwrap()] = false;
                    }
                    if level < upper_level {
                        if lower_level_ids.contains(&node.high) {
                            additional_v2l_upper.insert(manager.nodes[&node.high]);
                        }
                        if lower_level_ids.contains(&node.low) {
                            additional_v2l_upper.insert(manager.nodes[&node.low]);
                        }
                    }
                });
            }

            lower_level_ids
                .iter()
                .enumerate()
                .filter(|(i, _)| lower_level_root[*i])
                .for_each(|(_, id)| {
                    additional_v2l_upper.insert(manager.nodes[id]);
                });

            (upper_level, lower_level, size_before)
        };

        let mut manager = manager.write().unwrap();

        // Clear ITE cache
        manager.clear_c_table();

        // Clear levels
        manager.var2level.swap(a.0, b.0);
        manager.level2nodes[upper_level].clear();
        assert!(manager.level2nodes[upper_level].is_empty());
        manager.level2nodes[lower_level].clear();
        assert!(manager.level2nodes[lower_level].is_empty());

        manager.level2nodes[lower_level].extend(new_v2l_lower.iter());
        manager.level2nodes[upper_level].extend(additional_v2l_upper.iter());
        println!(
            "new_v2l_lower: {:?}, additional_v2l_upper: {:?}",
            new_v2l_lower.len(),
            additional_v2l_upper.len()
        );

        // Add new nodes
        for node in new_nodes {
            let new_then_id = match node.high {
                ChildEnum::OldChild(id) => id,
                ChildEnum::NewChild(temp) => manager.node_get_or_create(&DDNode {
                    id: NodeID(0),
                    var: temp.var,
                    high: temp.high,
                    low: temp.low,
                }),
            };

            let new_else_id = match node.low {
                ChildEnum::OldChild(id) => id,
                ChildEnum::NewChild(temp) => manager.node_get_or_create(&DDNode {
                    id: NodeID(0),
                    var: temp.var,
                    high: temp.high,
                    low: temp.low,
                }),
            };

            assert_ne!(new_then_id, new_else_id);

            let new_f_node = DDNode {
                id: node.id,
                var: node.var,
                high: new_then_id,
                low: new_else_id,
            };

            // Replace node in nodes list
            *manager.nodes.get_mut(&node.id).unwrap() = new_f_node;

            // Insert new node in unique-table
            let inserted = manager.level2nodes[upper_level].insert(new_f_node);
            assert!(inserted);

            log::debug!(
                "Replaced node {:?} with {:?}",
                node.id,
                manager.nodes[&node.id]
            );
        }

        log::debug!(
            "Order is now: {:?} (layers: {:?})",
            manager.var2level,
            var2level_to_ordered_varids(&manager.var2level)
        );
        let size_after_up = manager.level2nodes[upper_level].len();
        let size_after_low = manager.level2nodes[lower_level].len();
        let size_after = size_after_up + size_after_low;
        println!(
            "finished Swapping variables {:?} and {:?} - before: {:?}, after: {:?}({:?}/{:?}) => {:?}",
            a, b, size_before, size_after, size_after_up, size_after_low, (size_before as i32 - size_after as i32) as i32
        );
        // TODO still reduce?
        (size_before as i32 - size_after as i32) as i32
    }

    /// Swaps graph layers of variables a and b. Requires a to be directly above b or vice versa.
    /// Performs reduction which may change NodeIDs. Returns new NodeID of f.
    #[allow(unused)]
    #[must_use]
    pub fn swap(&mut self, a: VarID, b: VarID, f: NodeID) -> NodeID {
        // If b above a, switch a and b
        let (a, b) = if (self.var2level[b.0] < self.var2level[a.0]) {
            (b, a)
        } else {
            (a, b)
        };
        log::info!(
            "Swapping variables {:?} and {:?} (layers {} and {})",
            a,
            b,
            self.var2level[a.0],
            self.var2level[b.0]
        );
        assert!(a.0 != 0 && b.0 != 0);
        let upperlevel = self.var2level[a.0];
        let lowerlevel = self.var2level[b.0];
        assert_eq!(
            lowerlevel,
            upperlevel + 1,
            "Variables not on adjacent layers!"
        );

        let ids = self.level2nodes[upperlevel]
            .iter()
            .map(|n| n.id)
            .collect::<Vec<NodeID>>();

        self.var2level.swap(a.0, b.0);
        self.level2nodes[upperlevel].clear();
        assert!(self.level2nodes[upperlevel].is_empty());
        self.level2nodes[lowerlevel].clear();
        assert!(self.level2nodes[lowerlevel].is_empty());

        for id in ids {
            log::debug!("Replacing node {:?}.", id);
            let f_id = id;

            let old_f_node = self.nodes[&f_id];

            let f_1_id = old_f_node.high;
            let f_0_id = old_f_node.low;

            let f_0_node = self.nodes[&f_0_id];
            let f_1_node = self.nodes[&f_1_id];

            if f_0_node.var != b && f_1_node.var != b {
                // f does not have connections to level directly below, we leave it as it is.
                log::debug!(
                    "Children of node {:?} more than one level below, leaving as is.",
                    f_id
                );
                self.level2nodes[lowerlevel].insert(old_f_node);
                continue;
            }

            log::debug!("Replacing node {:?} old_f_node={:?}", f_id, old_f_node);

            let (f_01_id, f_00_id) = if f_0_node.var == b {
                (f_0_node.high, f_0_node.low)
            } else {
                (f_0_id, f_0_id)
            };
            let (f_11_id, f_10_id) = if f_1_node.var == b {
                (f_1_node.high, f_1_node.low)
            } else {
                (f_1_id, f_1_id)
            };

            let new_then_id = if f_01_id == f_11_id {
                f_01_id
            } else {
                self.node_get_or_create(&DDNode {
                    id: NodeID(0),
                    var: a,
                    low: f_01_id,
                    high: f_11_id,
                })
            };
            let new_else_id = if f_00_id == f_10_id {
                f_00_id
            } else {
                self.node_get_or_create(&DDNode {
                    id: NodeID(0),
                    var: a,
                    low: f_00_id,
                    high: f_10_id,
                })
            };

            assert_ne!(new_then_id, new_else_id);

            // Replace F node
            let new_f_node = DDNode {
                id: f_id,
                var: b,
                low: new_else_id,
                high: new_then_id,
            };

            log::debug!("new_f_node={:?}", new_f_node);

            // Replace node in nodes list
            *self.nodes.get_mut(&f_id).unwrap() = new_f_node;

            // Insert new node in unique-table
            let inserted = self.level2nodes[upperlevel].insert(new_f_node);
            assert!(inserted);

            log::debug!("Replaced node {:?} with {:?}", f_id, self.nodes[&f_id]);
        }

        // Clear ITE cache
        self.clear_c_table();

        log::debug!(
            "Order is now: {:?} (layers: {:?})",
            self.var2level,
            var2level_to_ordered_varids(&self.var2level)
        );

        self.reduce(f)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use num_bigint::BigUint;

    use crate::core::{
        bdd_manager::DDManager, bdd_node::VarID, order::var2level_to_ordered_varids,
        test::tests::TestCase,
    };

    /// Swap each variable pair from initial order
    #[test]
    fn swap_sandwich() {
        let testcase = TestCase::test_trivial();

        for i in 1..testcase.nr_variables {
            let mut man = testcase.man.clone();

            let bdd = man.swap(VarID(i), VarID(i + 1), testcase.f);
            assert!(testcase.verify_against(&man, bdd));
        }
    }

    /// Test sifting each variables to the bottom
    #[test]
    fn swap_sandwich_top_to_bottom() {
        let _ = env_logger::builder().is_test(true).try_init();

        let expected = BigUint::parse_bytes(b"2808", 10).unwrap();

        let mut instance = dimacs::parse_dimacs(
            &fs::read_to_string("examples/sandwich.dimacs").expect("Failed to read dimacs file."),
        )
        .expect("Failed to parse dimacs file.");
        let (man, bdd) = DDManager::from_instance(&mut instance, None, Default::default()).unwrap();
        let num_vars = match instance {
            dimacs::Instance::Cnf { num_vars, .. } => num_vars as usize,
            _ => panic!("Unsupported dimacs format!"),
        };

        assert_eq!(man.sat_count(bdd), expected);

        for v in 1..num_vars {
            let mut man = man.clone();
            let mut bdd = bdd;
            for i in v..num_vars {
                bdd = man.swap(VarID(v), VarID(i + 1), bdd);
                // Use sat_count as sanity check that the BDD isnt completely broken
                assert_eq!(man.sat_count(bdd), expected);
            }
        }
    }

    #[test]
    #[should_panic(expected = "Variables not on adjacent layers!")]
    fn swap_failure_non_adjacent() {
        let mut instance = dimacs::parse_dimacs(
            &fs::read_to_string("examples/sandwich.dimacs").expect("Failed to read dimacs file."),
        )
        .expect("Failed to parse dimacs file.");
        let (mut man, bdd) =
            DDManager::from_instance(&mut instance, None, Default::default()).unwrap();
        let _ = man.swap(VarID(1), VarID(3), bdd);
    }

    // Test that reverting a swap results in same node count as before
    fn swap_invert_nodecount(testcase: TestCase) {
        let _ = env_logger::builder().is_test(true).try_init();

        for i in 1..testcase.nr_variables {
            let mut man = testcase.man.clone();
            let var_a = VarID(i);
            let var_b = VarID(i + 1);

            println!("Swapping variables {:?} and {:?}", var_a, var_b);

            let count_before = man.count_active(testcase.f);

            let bdd = man.swap(var_a, var_b, testcase.f);
            assert!(testcase.verify_against(&man, bdd));

            let bdd = man.swap(var_b, var_a, bdd);
            assert!(testcase.verify_against(&man, bdd));

            let count_after = man.count_active(bdd);

            assert_eq!(count_before, count_after);
        }
    }

    #[test]
    fn swap_invert_nodecount_trivial() {
        swap_invert_nodecount(TestCase::test_trivial());
    }

    #[test]
    fn swap_invert_nodecount_random1() {
        swap_invert_nodecount(TestCase::random_1());
    }

    #[test]
    fn swap_last_vars() {
        let _ = env_logger::builder().is_test(true).try_init();

        let testcase = TestCase::test_trivial();
        fs::write("before.dot", testcase.man.graphviz(testcase.f)).unwrap();

        let mut man = testcase.man.clone();

        let bdd = man.swap(VarID(2), VarID(3), testcase.f);
        fs::write("after.dot", man.graphviz(bdd)).unwrap();

        assert!(testcase.verify_against(&man, bdd));
    }

    fn swap_multiple_noop(testcase: TestCase) {
        let _ = env_logger::builder().is_test(true).try_init();

        let mut man = testcase.man.clone();
        let mut bdd = testcase.f;

        let mut counts = vec![man.count_active(bdd)];

        let var = VarID(1);

        // Sift down, record BDD sizes
        for i in var.0 + 1..testcase.nr_variables + 1 {
            bdd = man.swap(var, VarID(i), bdd);
            man.purge_retain(bdd);

            println!("Swapped, count is now {:?}", man.count_active(bdd));
            println!(
                "Order is now {:?}",
                var2level_to_ordered_varids(&man.var2level)
            );

            assert!(testcase.verify_against(&man, bdd));

            counts.push(man.count_active(bdd));
        }

        let mut counts_up = vec![man.count_active(bdd)];

        // Sift up
        for i in (var.0 + 1..testcase.nr_variables + 1).rev() {
            bdd = man.swap(VarID(i), var, bdd);
            man.purge_retain(bdd);

            println!("Swapped, count is now {:?}", man.count_active(bdd));
            println!(
                "Order is now {:?}",
                var2level_to_ordered_varids(&man.var2level)
            );
            assert!(testcase.verify_against(&man, bdd));

            counts_up.push(man.count_active(bdd));
        }
        counts_up.reverse();

        println!("{:?}\n{:?}", counts, counts_up);

        assert_eq!(counts, counts_up);
    }

    #[test]
    fn swap_multiple_noop_trivial() {
        swap_multiple_noop(TestCase::test_trivial());
    }

    #[test]
    fn swap_multiple_noop_random1() {
        swap_multiple_noop(TestCase::random_1());
    }
}

#[cfg(test)]
mod tests_async {
    use std::{
        fs,
        sync::{Arc, RwLock},
    };

    use ::futures::future;

    use crate::core::{
        bdd_manager::DDManager, bdd_node::VarID, order::var2level_to_ordered_varids,
        test::tests::TestCase,
    };

    #[tokio::test]
    async fn swap_simultaneously_test() {
        let testcase = TestCase::random_2();
        assert!(testcase.nr_variables == 16);

        let root = testcase.f;
        let expected = testcase.man.sat_count(root);

        let manager = Arc::new(RwLock::new(testcase.man.clone()));

        let swap1 = tokio::spawn(DDManager::async_swap(manager.clone(), VarID(1), VarID(2)));
        let swap2 = tokio::spawn(DDManager::async_swap(manager.clone(), VarID(5), VarID(6)));
        let swap3 = tokio::spawn(DDManager::async_swap(manager.clone(), VarID(10), VarID(11)));

        future::join_all([swap1, swap2, swap3]).await;

        assert!(testcase.verify_against(&manager.read().unwrap(), root));
        assert_eq!(manager.read().unwrap().sat_count(root), expected);
    }

    #[tokio::test]
    async fn simple_async_swap_test() {
        let testcase = TestCase::random_2();
        assert!(testcase.nr_variables == 16);

        let before_active_count = testcase.man.count_active(testcase.f);

        let manager = Arc::new(RwLock::new(testcase.man.clone()));

        DDManager::async_swap(manager.clone(), VarID(2), VarID(3)).await;
        assert!(testcase.verify_against(&manager.read().unwrap(), testcase.f));

        DDManager::async_swap(manager.clone(), VarID(2), VarID(3)).await;
        assert!(testcase.verify_against(&manager.read().unwrap(), testcase.f));
        let after_active_count: usize = manager.read().unwrap().count_active(testcase.f);

        assert!(before_active_count == after_active_count);
    }

    #[tokio::test]
    async fn swap_back_and_forth_sandwich_async() {
        let _ = env_logger::builder().is_test(true).try_init();

        // Build BDD
        let mut instance = dimacs::parse_dimacs(
            &fs::read_to_string("examples/berkeleydb.dimacs").expect("Failed to read dimacs file."),
        )
        .expect("Failed to parse dimacs file.");
        let (mut man, mut bdd) =
            DDManager::from_instance(&mut instance, None, Default::default()).unwrap();
        let expected = man.sat_count(bdd);
        bdd = man.reduce(bdd);

        let levels = man.level2nodes.len();
        let manager = Arc::new(RwLock::new(man));

        for i in 1..(levels - 2) {
            // for i in 13..(14) {
            let before = { manager.read().unwrap().count_active(bdd) };
            let forth = DDManager::async_swap(manager.clone(), VarID(i), VarID(i + 1)).await;
            let back = DDManager::async_swap(manager.clone(), VarID(i), VarID(i + 1)).await;
            let after = { manager.read().unwrap().count_active(bdd) };

            bdd = manager.write().unwrap().reduce(bdd);

            assert_eq!(i32::abs(forth) - i32::abs(back), 0);
            assert_eq!(before, after);
            assert_eq!(manager.read().unwrap().sat_count(bdd), expected);
        }
    }

    /// Swap each variable pair from initial order
    #[tokio::test]
    async fn swap_sandwich_par() {
        let testcase = TestCase::test_trivial();
        for i in 1..testcase.nr_variables {
            let manager = Arc::new(RwLock::new(testcase.man.clone()));

            DDManager::async_swap(manager.clone(), VarID(i), VarID(i + 1)).await;
            assert!(testcase.verify_against(&manager.read().unwrap(), testcase.f));
        }
    }

    #[tokio::test]
    #[should_panic(expected = "Variables not on adjacent layers!")]
    async fn swap_failure_non_adjacent_par() {
        let mut instance = dimacs::parse_dimacs(
            &fs::read_to_string("examples/sandwich.dimacs").expect("Failed to read dimacs file."),
        )
        .expect("Failed to parse dimacs file.");
        let (man, _) = DDManager::from_instance(&mut instance, None, Default::default()).unwrap();
        let manager = Arc::new(RwLock::new(man.clone()));
        let _ = DDManager::async_swap(manager.clone(), VarID(1), VarID(3)).await;
        // let _ = man.swap(VarID(1), VarID(3), bdd);
    }

    // Test that reverting a swap results in same node count as before
    async fn swap_invert_nodecount_par(testcase: TestCase) {
        let _ = env_logger::builder().is_test(true).try_init();

        for i in 1..testcase.nr_variables {
            let man = testcase.man.clone();
            let var_a = VarID(i);
            let var_b = VarID(i + 1);

            let count_before = man.count_active(testcase.f);

            let manager = Arc::new(RwLock::new(man.clone()));

            let mut bdd;
            DDManager::async_swap(manager.clone(), var_a, var_b).await;
            // let bdd = man.swap(var_a, var_b, testcase.f);
            {
                bdd = manager.write().unwrap().reduce(testcase.f);
            }
            assert!(testcase.verify_against(&manager.read().unwrap(), bdd));

            DDManager::async_swap(manager.clone(), var_b, var_a).await;
            {
                bdd = manager.write().unwrap().reduce(bdd);
            }
            // let bdd = man.swap(var_b, var_a, bdd);
            assert!(testcase.verify_against(&manager.read().unwrap(), bdd));

            let count_after = manager.read().unwrap().count_active(bdd);

            assert_eq!(count_before, count_after);
        }
    }

    #[tokio::test]
    async fn swap_invert_nodecount_trivial_par() {
        swap_invert_nodecount_par(TestCase::test_trivial()).await;
    }

    #[tokio::test]
    async fn swap_invert_nodecount_random1_par() {
        swap_invert_nodecount_par(TestCase::random_1()).await;
    }

    #[tokio::test]
    async fn swap_last_vars_par() {
        let _ = env_logger::builder().is_test(true).try_init();

        let testcase = TestCase::test_trivial();
        fs::write("before.dot", testcase.man.graphviz(testcase.f)).unwrap();

        let manager: Arc<RwLock<DDManager>> = Arc::new(RwLock::new(testcase.man.clone()));

        DDManager::async_swap(manager.clone(), VarID(2), VarID(3)).await;
        // man.swap(VarID(2), VarID(3), testcase.f);
        let man = manager.read().unwrap();
        fs::write("after.dot", man.graphviz(testcase.f)).unwrap();

        assert!(testcase.verify_against(&man, testcase.f));
    }

    async fn swap_multiple_noop_par(testcase: TestCase) {
        let _ = env_logger::builder().is_test(true).try_init();

        let man = testcase.man.clone();
        let mut bdd = testcase.f;

        let mut counts = vec![man.count_active(bdd)];

        let var = VarID(1);

        let manager: Arc<RwLock<DDManager>> = Arc::new(RwLock::new(man.clone()));

        // let bdd = DDManager::par_swap(manager.clone(), var_a, var_b, testcase.f);

        // Sift down, record BDD sizes
        for i in var.0 + 1..testcase.nr_variables + 1 {
            DDManager::async_swap(manager.clone(), var, VarID(i)).await;
            {
                bdd = manager.write().unwrap().reduce(bdd);
            }
            let mut man = manager.write().unwrap();
            //bdd = man.swap(var, VarID(i), bdd);
            man.purge_retain(bdd);

            println!("Swapped, count is now {:?}", man.count_active(bdd));
            println!(
                "Order is now {:?}",
                var2level_to_ordered_varids(&man.var2level)
            );

            assert!(testcase.verify_against(&man, bdd));

            counts.push(man.count_active(bdd));
        }

        let mut counts_up = vec![manager.read().unwrap().count_active(bdd)];

        // Sift up
        for i in (var.0 + 1..testcase.nr_variables + 1).rev() {
            DDManager::async_swap(manager.clone(), VarID(i), var).await;
            {
                bdd = manager.write().unwrap().reduce(bdd);
            }
            let mut man = manager.write().unwrap();
            // bdd = man.swap(VarID(i), var, bdd);
            man.purge_retain(bdd);

            println!("Swapped, count is now {:?}", man.count_active(bdd));
            println!(
                "Order is now {:?}",
                var2level_to_ordered_varids(&man.var2level)
            );
            assert!(testcase.verify_against(&man, bdd));

            counts_up.push(man.count_active(bdd));
        }
        counts_up.reverse();

        println!("{:?}\n{:?}", counts, counts_up);

        assert_eq!(counts, counts_up);
    }

    #[tokio::test]
    async fn swap_multiple_noop_trivial_par() {
        swap_multiple_noop_par(TestCase::test_trivial()).await;
    }

    #[tokio::test]
    async fn swap_multiple_noop_random1_par() {
        swap_multiple_noop_par(TestCase::random_1()).await;
    }
}
