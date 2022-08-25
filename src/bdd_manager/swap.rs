use crate::{
    bdd_manager::order_to_layernames,
    bdd_node::{DDNode, NodeID, VarID},
};

use super::DDManager;

impl DDManager {
    /// Swaps graph layers of variables a and b. Requires a to be directly above b.
    /// Performs reduction which may change NodeIDs. Returns new NodeID of f.
    #[allow(unused)]
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
        let ids = self.var2nodes[a.0 as usize]
            .iter()
            .map(|n| n.id)
            .collect::<Vec<NodeID>>();

        self.var2nodes[a.0 as usize].clear();
        // Unique Table: Will not be valid. Directly modified node might be
        // degenerate. Reduction after swap will restore canonical form.

        for id in ids {
            let f_id = id;

            let old_f_node = self.nodes[&f_id];

            log::debug!("Replacing node {:?} old_f_node={:?}", f_id, old_f_node);

            let f_1_id = old_f_node.high;
            let f_0_id = old_f_node.low;

            let f_0_node = self.nodes[&f_0_id];
            let f_1_node = self.nodes[&f_1_id];

            if f_0_node.var != b && f_1_node.var != b {
                // "If neither child of the node for f is labeled b, then the
                // node is moved to the other subtable; otherwise swapping
                // proceeds as described above"
                let new_f_node = DDNode {
                    id: f_id,
                    var: b,
                    low: f_0_id,
                    high: f_1_id,
                };
                *self.nodes.get_mut(&f_id).unwrap() = new_f_node;
                continue;
            }

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

            let new_then_id = self.node_get_or_create(&DDNode {
                id: NodeID(0),
                var: a,
                low: f_01_id,
                high: f_11_id,
            });
            let new_else_id = self.node_get_or_create(&DDNode {
                id: NodeID(0),
                var: a,
                low: f_00_id,
                high: f_10_id,
            });

            /*
            log::debug!(
                "New children: high {} low {}",
                self.format_node(new_then_id),
                self.format_node(new_else_id)
            );
            */

            // Replace F node
            let new_f_node = DDNode {
                id: f_id,
                var: b,
                low: new_else_id,
                high: new_then_id,
            };

            log::debug!("new_f_node={:?}", new_f_node);

            *self.nodes.get_mut(&f_id).unwrap() = new_f_node;

            let inserted = self.var2nodes[b.0 as usize].insert(DDNode {
                var: b,
                low: new_else_id,
                high: new_then_id,
                id: f_id,
            });

            if !inserted {
                log::warn!(
                    "The unique table for {:?} seems to already contain a node ({:?} {:?} {:?})",
                    b,
                    b,
                    new_else_id,
                    new_then_id
                )
            }

            log::debug!("Replaced node {:?} with {:?}", f_id, self.nodes[&f_id]);
        }
        self.order.swap(a.0 as usize, b.0 as usize);
        self.c_table.clear();
        log::debug!(
            "Order is now: {:?} (layers: {:?})",
            self.order,
            order_to_layernames(&self.order)
        );

        // TODO: Use information which nodes have changed to avoid full reduce
        self.reduce(f)
    }
}

#[cfg(test)]
mod tests {
    use num_bigint::BigUint;

    use crate::{bdd_manager::DDManager, bdd_node::VarID, dimacs};

    /// Swap each variable pair from initial order
    #[test]
    fn swap_sandwich() {
        let expected = BigUint::parse_bytes(b"2808", 10).unwrap();

        let mut instance = dimacs::parse_dimacs("examples/sandwich.dimacs");
        let (man, bdd) = DDManager::from_instance(&mut instance, None).unwrap();

        assert_eq!(man.sat_count(bdd), expected);

        for i in 1..instance.no_variables {
            let mut man = man.clone();

            let bdd = man.swap(VarID(i), VarID(i + 1), bdd);
            assert_eq!(man.sat_count(bdd), expected);
        }
    }

    /// Test sifting each variables to the bottom
    #[test]
    fn swap_sandwich_top_to_bottom() {
        let _ = env_logger::builder().is_test(true).try_init();

        let expected = BigUint::parse_bytes(b"2808", 10).unwrap();

        let mut instance = dimacs::parse_dimacs("examples/sandwich.dimacs");
        let (man, bdd) = DDManager::from_instance(&mut instance, None).unwrap();

        assert_eq!(man.sat_count(bdd), expected);

        for v in 1..instance.no_variables {
            let mut man = man.clone();
            let mut bdd = bdd;
            for i in v..instance.no_variables {
                bdd = man.swap(VarID(v), VarID(i + 1), bdd);
                // Use sat_count as sanity check that the BDD isnt completely broken
                assert_eq!(man.sat_count(bdd), expected);
            }
        }
    }

    #[test]
    #[should_panic(expected = "Variables not on adjacent layers!")]
    fn swap_failure_non_adjacent() {
        let mut instance = dimacs::parse_dimacs("examples/sandwich.dimacs");
        let (mut man, bdd) = DDManager::from_instance(&mut instance, None).unwrap();
        man.swap(VarID(1), VarID(3), bdd);
    }

    // Test that reverting a swap results in same node count as before
    #[test]
    fn swap_invert_nodecount() {
        let _ = env_logger::builder().is_test(true).try_init();

        let expected = BigUint::parse_bytes(b"5", 10).unwrap();
        let mut instance = dimacs::parse_dimacs("examples/trivial.dimacs");
        let (mut man, bdd) = DDManager::from_instance(&mut instance, None).unwrap();
        assert_eq!(man.sat_count(bdd), expected);

        let bdd = man.reduce(bdd);
        let man = man;

        for i in 1..instance.no_variables {
            let mut man = man.clone();
            let var_a = VarID(i);
            let var_b = VarID(i + 1);

            println!("Swapping variables {:?} and {:?}", var_a, var_b);

            let count_before = man.count_active(bdd);

            let bdd = man.swap(var_a, var_b, bdd);
            assert_eq!(man.sat_count(bdd), expected);

            let bdd = man.swap(var_b, var_a, bdd);
            assert_eq!(man.sat_count(bdd), expected);
            let count_after = man.count_active(bdd);

            assert_eq!(count_before, count_after);
        }
    }
}
