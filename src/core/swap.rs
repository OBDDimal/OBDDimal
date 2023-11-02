//! Implementation of BDD layer swap

use crate::{
    core::bdd_manager::DDManager,
    core::bdd_node::{DDNode, NodeID, VarID},
    core::order::order_to_layernames,
};

impl DDManager {
    /// Swaps graph layers of variables a and b. Requires a to be directly above b.
    /// Performs reduction which may change NodeIDs. Returns new NodeID of f.
    #[allow(unused)]
    #[must_use]
    pub fn swap(&mut self, a: VarID, b: VarID, f: NodeID) -> NodeID {
        log::info!(
            "Swapping variables {:?} and {:?} (layers {} and {})",
            a,
            b,
            self.order[a.0 as usize],
            self.order[b.0 as usize]
        );
        assert!(a.0 != 0 && b.0 != 0);
        assert_eq!(
            self.order[b.0 as usize],
            self.order[a.0 as usize] + 1,
            "Variables not on adjacent layers!"
        );

        while self.var2nodes.len() <= a.0 as usize {
            self.var2nodes.push(Default::default());
        }

        let ids = self.var2nodes[a.0 as usize]
            .iter()
            .map(|n| n.id)
            .collect::<Vec<NodeID>>();

        self.order.swap(a.0 as usize, b.0 as usize);

        for id in ids {
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

            // Insert node in new unique-table
            let inserted = self.var2nodes[b.0 as usize].insert(DDNode {
                var: b,
                low: new_else_id,
                high: new_then_id,
                id: f_id,
            });
            assert!(inserted);

            // Remove node from old unique-table

            let removed = self.var2nodes[a.0 as usize].remove(&DDNode {
                var: a,
                low: f_0_id,
                high: f_1_id,
                id: f_id,
            });
            assert!(removed);

            log::debug!("Replaced node {:?} with {:?}", f_id, self.nodes[&f_id]);
        }

        // Clear ITE cache
        self.c_table.clear();

        log::debug!(
            "Order is now: {:?} (layers: {:?})",
            self.order,
            order_to_layernames(&self.order)
        );

        f
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use num_bigint::BigUint;

    use crate::{
        core::bdd_manager::DDManager, core::bdd_node::VarID, core::order::order_to_layernames,
        core::test::tests::TestCase,
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
                bdd = man.swap(
                    VarID(v.try_into().unwrap()),
                    VarID((i + 1).try_into().unwrap()),
                    bdd,
                );
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
            println!("Order is now {:?}", order_to_layernames(&man.order));

            assert!(testcase.verify_against(&man, bdd));

            counts.push(man.count_active(bdd));
        }

        let mut counts_up = vec![man.count_active(bdd)];

        // Sift up
        for i in (var.0 + 1..testcase.nr_variables + 1).rev() {
            bdd = man.swap(VarID(i), var, bdd);
            man.purge_retain(bdd);

            println!("Swapped, count is now {:?}", man.count_active(bdd));
            println!("Order is now {:?}", order_to_layernames(&man.order));
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
