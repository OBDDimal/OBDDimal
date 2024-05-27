//! An operation allowing to modify all nodes belonging to specific variables/layers.

use crate::{
    core::{
        bdd_manager::DDManager,
        bdd_node::{DDNode, NodeID, VarID, ONE, ZERO},
        order::var2level_to_ordered_varids,
    },
    misc::hash_select::{HashMap, HashSet},
};

pub type NodeModFunction = dyn FnMut(&DDNode, &mut DDManager, &HashMap<NodeID, NodeID>) -> NodeID;

impl DDManager {
    /// Modifies all nodes in the given BDD(s) corresponding to the given variables using the given
    /// function. Also corrects the references in nodes above the modified nodes so the resulting
    /// BDD(s) are still valid. The "modification" thereby creates new nodes, so the original BDDs
    /// can still be used later.
    ///
    /// * `relevant_nodes` - A set containing all nodes of the BDDs for which this operation is
    /// supposed to be run.
    /// * `vars` - The variables, for which the nodes should be "modified".
    /// * `func` - The function which should be applied to the nodes corresponding to the given
    /// variables.
    ///
    /// # Returns
    /// A HashMap which can be used to translate the root nodes of the original BDDs to the root
    /// nodes of the new ("modified") BDDs.
    ///
    /// # Notes
    /// May create temporary nodes, consider using [`purge_retain_multi`] or [`purge_retain`]
    /// afterwards.
    ///
    /// [`purge_retain_multi`]: crate::core::bdd_manager::DDManager::purge_retain_multi
    /// [`purge_retain`]: crate::core::bdd_manager::DDManager::purge_retain
    pub fn modify_var_nodes(
        &mut self,
        relevant_nodes: &HashSet<NodeID>,
        vars: &HashSet<VarID>,
        func: &mut NodeModFunction,
    ) -> HashMap<NodeID, NodeID> {
        let mut bottom_up_vars = var2level_to_ordered_varids(&self.var2level);
        bottom_up_vars.reverse();

        let mut new_ids: HashMap<NodeID, NodeID> = HashMap::default();
        new_ids.insert(ZERO.id, ZERO.id);
        new_ids.insert(ONE.id, ONE.id);

        let mut changed = false;
        bottom_up_vars.iter().for_each(|var_id| {
            if vars.contains(var_id) {
                // Apply function to all relevant nodes in this layer
                self.level2nodes[self.var2level[var_id.0]]
                    .clone()
                    .iter()
                    .filter(|DDNode { id, .. }| relevant_nodes.contains(id))
                    .for_each(|node| {
                        new_ids.insert(node.id, func(node, self, &new_ids));
                    });
                changed = true;
            } else if changed {
                // Update Nodes in case of new children
                self.level2nodes[self.var2level[var_id.0]]
                    .clone()
                    .iter()
                    .filter(|DDNode { id, .. }| relevant_nodes.contains(id))
                    .for_each(|DDNode { id, var, low, high }| {
                        new_ids.insert(
                            *id,
                            self.node_get_or_create(&DDNode {
                                id: NodeID(0),
                                var: *var,
                                low: *new_ids.get(low).unwrap(),
                                high: *new_ids.get(high).unwrap(),
                            }),
                        );
                    });
            } else {
                // Just add to new_ids without changing anything
                self.level2nodes[self.var2level[var_id.0]]
                    .iter()
                    .map(|node| node.id)
                    .filter(|node_id| relevant_nodes.contains(node_id))
                    .for_each(|node_id| {
                        new_ids.insert(node_id, node_id);
                    });
            }
        });

        new_ids
    }
}
