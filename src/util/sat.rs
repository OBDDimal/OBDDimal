//! Satisfyability count, active nodes count

use num_bigint::BigUint;
use num_traits::{One, Zero};

use crate::{
    core::{bdd_manager::DDManager, bdd_node::NodeID},
    misc::hash_select::HashMap,
};

impl DDManager {
    pub fn is_sat(&self, node: NodeID) -> bool {
        node.0 != 0
    }

    pub fn sat_count(&self, f: NodeID) -> BigUint {
        self.sat_count_with_cache(f, &mut HashMap::default())
    }

    pub(crate) fn sat_count_with_cache(
        &self,
        f: NodeID,
        cache: &mut HashMap<NodeID, BigUint>,
    ) -> BigUint {
        let node_sat = self.sat_count_rec(f, cache);

        let jump = self.var2level[self.nodes.get(&f).unwrap().var.0] - 1;
        let fac = BigUint::parse_bytes(b"2", 10).unwrap().pow(jump as u32);

        node_sat * fac
    }

    fn sat_count_rec(&self, node_id: NodeID, cache: &mut HashMap<NodeID, BigUint>) -> BigUint {
        if node_id == NodeID(0) {
            Zero::zero()
        } else if node_id == NodeID(1) {
            One::one()
        } else if cache.contains_key(&node_id) {
            cache.get(&node_id).unwrap().clone()
        } else {
            let mut total: BigUint = Zero::zero();

            let node = &self.nodes.get(&node_id).unwrap();

            let low_var = &self.nodes.get(&node.low).unwrap().var;
            let low_jump = self.var2level[low_var.0] - self.var2level[node.var.0] - 1;
            let low_fac = BigUint::parse_bytes(b"2", 10).unwrap().pow(low_jump as u32);
            total += self.sat_count_rec(node.low, cache) * low_fac;

            let high_var = &self.nodes.get(&node.high).unwrap().var;
            let high_jump = self.var2level[high_var.0] - self.var2level[node.var.0] - 1;
            let high_fac = BigUint::parse_bytes(b"2", 10)
                .unwrap()
                .pow(high_jump as u32);
            total += self.sat_count_rec(node.high, cache) * high_fac;

            cache.insert(node_id, total.clone());
            total
        }
    }

    pub fn count_active(&self, f: NodeID) -> usize {
        self.get_reachable(&[f]).len()
    }
}
