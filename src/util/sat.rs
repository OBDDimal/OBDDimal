//! Satisfyability count, active nodes count

use malachite::{num::arithmetic::traits::Pow, Natural};

use crate::{
    core::{bdd_manager::DDManager, bdd_node, bdd_node::NodeID},
    misc::hash_select::HashMap,
};

impl DDManager {
    pub fn is_sat(&self, node: NodeID) -> bool {
        node.0 != 0
    }

    pub fn sat_count(&self, f: NodeID) -> Natural {
        self.sat_count_with_cache(f, &mut HashMap::default())
    }

    pub(crate) fn sat_count_with_cache(
        &self,
        f: NodeID,
        cache: &mut HashMap<NodeID, Natural>,
    ) -> Natural {
        let node_sat = self.sat_count_from(f, cache);

        let jump = self.var2level[self.nodes.get(&f).unwrap().var.0] - 1;
        let fac = Natural::from(2usize).pow(jump as u64);

        node_sat * fac
    }

    #[inline]
    fn sat_count_from(&self, from_node: NodeID, cache: &mut HashMap<NodeID, Natural>) -> Natural {
        let reachable = self.get_reachable(&[from_node]);

        cache.insert(NodeID(0), Natural::from(0usize));
        cache.insert(NodeID(1), Natural::from(1usize));

        (self.var2level[self.nodes.get(&from_node).unwrap().var.0]
            ..self.var2level[bdd_node::ZERO.var.0])
            .rev()
            .flat_map(|level| &self.level2nodes[level])
            .filter(|node| reachable.contains(&node.id))
            .for_each(|node| {
                let mut total = Natural::from(0usize);

                let low_var = &self.nodes.get(&node.low).unwrap().var;
                let low_jump = self.var2level[low_var.0] - self.var2level[node.var.0] - 1;
                let low_fac = Natural::from(2usize).pow(low_jump as u64);
                total += cache.get(&node.low).unwrap() * low_fac;

                let high_var = &self.nodes.get(&node.high).unwrap().var;
                let high_jump = self.var2level[high_var.0] - self.var2level[node.var.0] - 1;
                let high_fac = Natural::from(2usize).pow(high_jump as u64);
                total += cache.get(&node.high).unwrap() * high_fac;

                cache.insert(node.id, total);
            });

        cache.get(&from_node).unwrap().clone()
    }

    pub fn count_active(&self, f: NodeID) -> usize {
        self.get_reachable(&[f]).len()
    }
}
