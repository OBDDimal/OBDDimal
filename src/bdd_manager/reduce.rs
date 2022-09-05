use crate::{
    bdd_manager::{order::order_to_layernames, ZERO},
    bdd_node::{DDNode, NodeID, VarID},
};

use super::DDManager;

use rustc_hash::FxHashMap as HashMap;

impl DDManager {
    /// Reduces the BDD. This changes Node IDs, the new Node ID of the function passed to the function is returned.
    /// All other known Node IDs are invalid after reduce.
    /// Note: Reduction does not imply removing nodes not part of the specified function.
    /// Other functions may be present in the tree (multiple root nodes, or root above function).
    ///
    /// See also "Graph-Based Algorithms for Boolean Function Manipulation" by Bryant (10.1109/TC.1986.1676819)
    #[must_use]
    #[allow(unused)]
    pub(crate) fn reduce(&mut self, v: NodeID) -> NodeID {
        log::debug!("reducing");

        let mut vlist: Vec<Vec<NodeID>> = vec![Vec::new(); self.order[0] as usize];

        for (id, node) in self.nodes.iter() {
            vlist[node.var.0 as usize].push(*id);
        }

        let mut nextid = 0;

        #[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
        enum Key {
            Terminal(bool),
            LowHigh(NodeID, NodeID),
            Unmatchable,
        }

        // This will be modified, such that old_nodes[id].id != id may be true, if a node is redundant.
        let mut old_nodes = self.nodes.clone();
        let mut new_nodes: HashMap<NodeID, DDNode> = HashMap::default();

        // Graph layers, bottom to top
        for i in order_to_layernames(&self.order).iter().rev() {
            log::debug!("Handling var {:?}", i);

            #[allow(non_snake_case)]
            let mut Q: Vec<(Key, NodeID)> = Vec::new();

            for u in vlist[i.0 as usize].iter() {
                let node_var = old_nodes.get(u).unwrap().var;
                let node_id = old_nodes.get(u).unwrap().id;
                let low_ptr = old_nodes.get(u).unwrap().low;
                let high_ptr = old_nodes.get(u).unwrap().high;
                let low_real_id = old_nodes.get(&low_ptr).unwrap().id;
                let high_real_id = old_nodes.get(&high_ptr).unwrap().id;
                if node_var == VarID(0) {
                    let key = if node_id == ZERO.id {
                        Key::Terminal(false)
                    } else {
                        Key::Terminal(true)
                    };

                    Q.push((key, *u));
                } else if low_real_id == high_real_id {
                    // Redundant to only child node.low
                    old_nodes.get_mut(u).unwrap().id = low_real_id;
                } else {
                    // Normal node, adding to Q
                    Q.push((Key::LowHigh(low_real_id, high_real_id), node_id));
                }
            }

            Q.sort_by_key(|k| k.0);

            log::debug!(" Iterating over Q...");
            let mut oldkey = Key::Unmatchable;
            for (key, u) in Q {
                log::debug!("  <{:?}, {:?}>", key, u);
                if key == oldkey {
                    log::debug!(
                        "  Repeated key -> Duplicate node. Assigning same ID as last ({:?})",
                        NodeID(nextid)
                    );
                    let node = old_nodes.get_mut(&u).unwrap();
                    node.id = NodeID(nextid);
                } else {
                    log::debug!("  New node");
                    nextid = {
                        match key {
                            Key::Terminal(true) => 1,
                            Key::Terminal(false) => 0,
                            _ => nextid + 1,
                        }
                    };
                    {
                        let node = old_nodes.get_mut(&u).unwrap();
                        log::debug!("  Assigning ID {:?}", nextid);
                        node.id = NodeID(nextid);
                    }

                    let (low_ptr, high_ptr) = {
                        let node = old_nodes.get(&u).unwrap();
                        (node.low, node.high)
                    };

                    log::debug!("  Visiting low and high child to see if ID changed");
                    let lownode_id = old_nodes
                        .get(&low_ptr)
                        .unwrap_or_else(|| {
                            panic!("Low child at {:?} not found in old nodes list!", low_ptr)
                        })
                        .id;
                    let highnode_id = old_nodes
                        .get(&high_ptr)
                        .unwrap_or_else(|| {
                            panic!("High child at {:?} not found in old nodes list!", high_ptr)
                        })
                        .id;

                    log::debug!(
                        "  Low, High were ({:?},{:?}), are now ({:?},{:?})",
                        low_ptr,
                        high_ptr,
                        lownode_id,
                        highnode_id
                    );

                    let node = old_nodes.get_mut(&u).unwrap();
                    node.low = lownode_id;
                    node.high = highnode_id;

                    new_nodes.insert(node.id, *node);

                    oldkey = key;
                }
            }
        }

        self.nodes = new_nodes;

        // Rebuild unique-table
        for v in self.var2nodes.iter_mut() {
            v.clear();
        }

        for (_id, node) in self.nodes.iter() {
            self.var2nodes[node.var.0 as usize].insert(DDNode {
                var: node.var,
                low: node.low,
                high: node.high,
                id: node.id,
            });
        }

        // Return updated ID of function (Changes due to renumbering, but this
        // is unavoidable since v may have been redundant or duplicate)
        old_nodes.get(&v).unwrap().id
    }
}

#[cfg(test)]
mod tests {
    use rustc_hash::FxHashSet as HashSet;

    use crate::bdd_node::{DDNode, NodeID, VarID};
    use crate::dimacs;

    use super::DDManager;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    /// Test if chain of nodes with low==high is reduced to the last node
    #[test]
    fn reduce_redundant() {
        init();

        let mut man = DDManager::default();
        #[allow(clippy::field_reassign_with_default)]
        {
            man.order = vec![4, 1, 2, 3];
        }
        man.var2nodes.resize(4, HashSet::default());

        // Node 2: Low:0, High: 1
        man.nodes.insert(
            NodeID(2),
            DDNode {
                id: NodeID(2),
                var: VarID(3),
                low: NodeID(0),
                high: NodeID(1),
            },
        );
        man.var2nodes[3].insert(DDNode {
            id: NodeID(2),
            var: VarID(3),
            low: NodeID(0),
            high: NodeID(1),
        });

        // Node 3: Low=High=2
        man.nodes.insert(
            NodeID(3),
            DDNode {
                id: NodeID(3),
                var: VarID(2),
                low: NodeID(2),
                high: NodeID(2),
            },
        );
        man.var2nodes[2].insert(DDNode {
            var: VarID(2),
            low: NodeID(2),
            high: NodeID(2),
            id: NodeID(3),
        });

        // Node 4 (f): Low=High=3
        man.nodes.insert(
            NodeID(4),
            DDNode {
                id: NodeID(4),
                var: VarID(1),
                low: NodeID(3),
                high: NodeID(3),
            },
        );
        man.var2nodes[1].insert(DDNode {
            var: VarID(1),
            low: NodeID(3),
            high: NodeID(3),
            id: NodeID(4),
        });

        let f = NodeID(4);
        //assert!(man.nr_nodes(f) == 5);

        let f = man.reduce(f);
        let f_node = man.nodes.get(&f).unwrap();
        assert!(f_node.low == NodeID(0));
        assert!(f_node.high == NodeID(1));
        //assert!(man.nr_nodes(f) == 3);
    }

    /// Test if two identical nodes get merged
    #[test]
    fn reduce_duplicate() {
        init();

        let mut man = DDManager::default();
        #[allow(clippy::field_reassign_with_default)]
        {
            man.order = vec![4, 1, 2, 3];
        }
        man.var2nodes.resize(4, HashSet::default());

        // Node 2: Low=0, High=1
        man.nodes.insert(
            NodeID(2),
            DDNode {
                id: NodeID(2),
                var: VarID(3),
                low: NodeID(0),
                high: NodeID(1),
            },
        );
        man.var2nodes[3].insert(DDNode {
            id: NodeID(2),
            var: VarID(3),
            low: NodeID(0),
            high: NodeID(1),
        });

        // Duplicate node 3: Low=0, High=1
        man.nodes.insert(
            NodeID(3),
            DDNode {
                id: NodeID(3),
                var: VarID(3),
                low: NodeID(0),
                high: NodeID(1),
            },
        );
        // (no unique-table entry)

        // Node 4: Low=1, High=2
        man.nodes.insert(
            NodeID(4),
            DDNode {
                id: NodeID(4),
                var: VarID(2),
                low: NodeID(1),
                high: NodeID(2),
            },
        );
        man.var2nodes[2].insert(DDNode {
            id: NodeID(4),
            var: VarID(2),
            low: NodeID(1),
            high: NodeID(2),
        });

        // Node 5 (f): Low=3, High=4
        man.nodes.insert(
            NodeID(5),
            DDNode {
                id: NodeID(5),
                var: VarID(1),
                low: NodeID(3),
                high: NodeID(4),
            },
        );
        man.var2nodes[1].insert(DDNode {
            id: NodeID(5),
            var: VarID(1),
            low: NodeID(3),
            high: NodeID(4),
        });

        let f = NodeID(5);
        let f = man.reduce(f);
        let f_node = man.nodes.get(&f).unwrap();

        let t_node_id = f_node.high;
        let t_node = man.nodes.get(&t_node_id).unwrap();
        assert!(f_node.low == t_node.high);
    }

    /// This builds a BDD from 5 nodes which can be entirely reduced to f=Var(3)
    #[test]
    fn reduce_combined() {
        init();

        let mut man = DDManager::default();
        #[allow(clippy::field_reassign_with_default)]
        {
            man.order = vec![4, 1, 2, 3];
        }
        man.var2nodes.resize(5, HashSet::default());

        // Node 2: Low=0, High=1
        man.nodes.insert(
            NodeID(2),
            DDNode {
                id: NodeID(2),
                var: VarID(3),
                low: NodeID(0),
                high: NodeID(1),
            },
        );
        man.var2nodes[3].insert(DDNode {
            id: NodeID(2),
            var: VarID(3),
            low: NodeID(0),
            high: NodeID(1),
        });

        // Duplicate node 3: Low=0, High=1
        man.nodes.insert(
            NodeID(3),
            DDNode {
                id: NodeID(3),
                var: VarID(3),
                low: NodeID(0),
                high: NodeID(1),
            },
        );
        // (no unique-table entry)

        // Node 4: Low=2, High=2
        let node_4 = DDNode {
            id: NodeID(4),
            var: VarID(2),
            low: NodeID(2),
            high: NodeID(2),
        };
        man.nodes.insert(NodeID(4), node_4);
        man.var2nodes[2].insert(node_4);

        // Node 5: Low=3, High=2
        let node_5 = DDNode {
            id: NodeID(5),
            var: VarID(2),
            low: NodeID(3),
            high: NodeID(2),
        };
        man.nodes.insert(NodeID(5), node_5);
        man.var2nodes[2].insert(node_5);

        // Node 6 (f): Low=5, High=4
        let node_6 = DDNode {
            id: NodeID(6),
            var: VarID(1),
            low: NodeID(5),
            high: NodeID(4),
        };
        man.nodes.insert(NodeID(6), node_6);
        man.var2nodes[1].insert(node_6);

        let f = NodeID(6);
        let f = man.reduce(f);
        let f_node = man.nodes.get(&f).unwrap();
        assert_eq!(f_node.high, NodeID(1));
        assert_eq!(f_node.low, NodeID(0));
    }

    /// This builds two disjunct functions, and tests if both are still present
    #[test]
    fn reduce_multirooted() {
        init();

        let mut man = DDManager::default();
        #[allow(clippy::field_reassign_with_default)]
        {
            man.order = vec![4, 1, 2, 3];
        }
        man.var2nodes.resize(4, HashSet::default());

        // Node 2: Low=0, High=1
        let node_2 = DDNode {
            id: NodeID(2),
            var: VarID(3),
            low: NodeID(0),
            high: NodeID(1),
        };
        man.nodes.insert(NodeID(2), node_2);
        man.var2nodes[3].insert(node_2);

        // Node 3: Low=1, High=0
        let node_3 = DDNode {
            id: NodeID(3),
            var: VarID(3),
            low: NodeID(1),
            high: NodeID(0),
        };
        man.nodes.insert(NodeID(3), node_3);
        man.var2nodes[3].insert(node_3);

        let f = NodeID(2);
        let f = man.reduce(f);
        let f_node = man.nodes.get(&f).unwrap();
        assert_eq!(f_node.high, NodeID(1));
        assert_eq!(f_node.low, NodeID(0));
        // Both nodes should still be present, but the IDs may have changed.
        assert_eq!(man.nodes.len(), 2 + 2);
        assert_eq!(man.var2nodes[3].len(), 2);
    }

    use num_bigint::BigUint;

    /// This tests that reducing the "sandwich" bdd does not fail and does not break it
    #[test]
    fn reduce_sandwich() {
        let expected = BigUint::parse_bytes(b"2808", 10).unwrap();

        let mut instance = dimacs::parse_dimacs("examples/sandwich.dimacs");
        let (mut man, bdd) = DDManager::from_instance(&mut instance, None, false).unwrap();

        assert_eq!(man.sat_count(bdd), expected);

        let bdd = man.reduce(bdd);
        assert_eq!(man.sat_count(bdd), expected);
    }
}
