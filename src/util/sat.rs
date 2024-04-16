//! Satisfyability count, active nodes count

use num_bigint::BigUint;
use num_traits::{One, Zero};

use crate::{
    core::{
        bdd_manager::DDManager,
        bdd_node::{NodeID, VarID},
    },
    misc::hash_select::HashMap,
};

impl DDManager {
    pub fn is_sat(&self, node: NodeID) -> bool {
        node.0 != 0
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
                self.var2level.len() - self.var2level[node.var.0] - 1
            } else {
                self.var2level[low.var.0] - self.var2level[node.var.0] - 1
            };

            let high_jump = if high.var == VarID(0) {
                self.var2level.len() - self.var2level[node.var.0] - 1
            } else {
                self.var2level[high.var.0] - self.var2level[node.var.0] - 1
            };

            let low_fac = BigUint::parse_bytes(b"2", 10).unwrap().pow(low_jump as u32);
            let high_fac = BigUint::parse_bytes(b"2", 10)
                .unwrap()
                .pow(high_jump as u32);

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

    pub fn count_active(&self, f: NodeID) -> usize {
        self.get_reachable(&[f]).len()
    }
}
