use std::collections::hash_map::Entry::{Occupied, Vacant};

use num_bigint::BigUint;
use num_traits::{One, Zero};
use super::hash_select::HashMap;

use super::DDManager;
use crate::bdd_node::{NodeID, VarID};

impl DDManager {
    #[allow(dead_code)]
    fn is_sat(&self, node: u32) -> bool {
        node != 0
    }

    pub fn sat_count(&self, f: NodeID) -> BigUint {
        self.sat_count_rec(f, &mut HashMap::default())
    }

    fn sat_count_rec(&self, f: NodeID, cache: &mut HashMap<NodeID, BigUint>) -> BigUint {
        let mut total: BigUint = Zero::zero();
        let node_id = f;

        if node_id == NodeID(0) {
            return Zero::zero();
        } else if node_id == NodeID(1) {
            return One::one();
        } else {
            let node = &self.nodes.get(&node_id).unwrap();

            let low = &self.nodes.get(&node.low).unwrap();
            let high = &self.nodes.get(&node.high).unwrap();

            let low_jump = if low.var == VarID(0) {
                self.order.len() as u32 - self.order[node.var.0 as usize] - 1
            } else {
                self.order[low.var.0 as usize] - self.order[node.var.0 as usize] - 1
            };

            let high_jump = if high.var == VarID(0) {
                self.order.len() as u32 - self.order[node.var.0 as usize] - 1
            } else {
                self.order[high.var.0 as usize] - self.order[node.var.0 as usize] - 1
            };

            let low_fac = BigUint::parse_bytes(b"2", 10).unwrap().pow(low_jump);
            let high_fac = BigUint::parse_bytes(b"2", 10).unwrap().pow(high_jump);

            total += match cache.get(&node.low) {
                Some(x) => x * low_fac,
                None => self.sat_count_rec(node.low, cache) * low_fac,
            };

            total += match cache.get(&node.high) {
                Some(x) => x * high_fac,
                None => self.sat_count_rec(node.high, cache) * high_fac,
            };
        };

        cache.insert(f, total.clone());

        total
    }

    pub fn count_active(&self, f: NodeID) -> u32 {
        // We use HashMap<NodeID, ()> instead of HashSet<NodeID> to be able to use the .entry()
        // API below. This turns out to be faster, since it avoids the double lookup if the
        // ID is not yet known (!contains -> insert).
        let mut nodes = HashMap::<NodeID, ()>::default();
        nodes.reserve(self.nodes.len());

        let mut stack = vec![f];
        stack.reserve(self.nodes.len());

        while !stack.is_empty() {
            let x = stack.pop().unwrap();
            let entry = nodes.entry(x);

            match entry {
                Occupied(_) => continue, // Node already counted
                Vacant(vacant_entry) => {
                    // Store node, add children to stack
                    let node = self.nodes.get(&x).unwrap();
                    stack.push(node.low);
                    stack.push(node.high);
                    vacant_entry.insert(());
                }
            }
        }

        nodes.len() as u32
    }
}
