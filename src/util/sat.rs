//! Satisfyability count, active nodes count

use num_bigint::BigUint;
use num_traits::{One, Zero};

use crate::{
    core::{
        bdd_manager::DDManager,
        bdd_node::{self, NodeID},
    },
    misc::hash_select::HashMap,
};

impl DDManager {
    pub fn is_sat(&self, node: NodeID) -> bool {
        node.0 != 0
    }

    pub fn sat_count(&self, f: NodeID) -> BigUint {
        self.sat_count_from(f, &mut HashMap::default())
    }

    #[inline]
    fn sat_count_from(&self, from_node: NodeID, cache: &mut HashMap<NodeID, BigUint>) -> BigUint {
        let reachable = self.get_reachable(&[from_node]);

        cache.insert(NodeID(0), Zero::zero());
        cache.insert(NodeID(1), One::one());

        (self.var2level[self.nodes.get(&from_node).unwrap().var.0]
            ..self.var2level[bdd_node::ZERO.var.0])
            .rev()
            .flat_map(|level| &self.level2nodes[level])
            .filter(|node| reachable.contains(&node.id))
            .for_each(|node| {
                let mut total: BigUint = Zero::zero();

                let low_var = &self.nodes.get(&node.low).unwrap().var;
                let low_jump = self.var2level[low_var.0] - self.var2level[node.var.0] - 1;
                let low_fac = BigUint::parse_bytes(b"2", 10).unwrap().pow(low_jump as u32);
                total += cache.get(&node.low).unwrap() * low_fac;

                let high_var = &self.nodes.get(&node.high).unwrap().var;
                let high_jump = self.var2level[high_var.0] - self.var2level[node.var.0] - 1;
                let high_fac = BigUint::parse_bytes(b"2", 10)
                    .unwrap()
                    .pow(high_jump as u32);
                total += cache.get(&node.high).unwrap() * high_fac;

                cache.insert(node.id, total);
            });

        cache.get(&from_node).unwrap().clone()
    }

    pub fn count_active(&self, f: NodeID) -> usize {
        self.get_reachable(&[f]).len()
    }
}
