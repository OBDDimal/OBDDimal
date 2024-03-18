//! BDD slicing

use crate::{
    core::{
        bdd_manager::{DDManager, ZERO},
        bdd_node::{DDNode, NodeID, VarID},
        order::order_to_layernames,
    },
    misc::hash_select::{HashMap, HashSet},
};

impl DDManager {
    /// Creates a slice of a BDD containing only the given variables.
    ///
    /// * `keep` - List of the variables to keep
    /// * `root` - The root node of the BDD that is supposed to be sliced
    ///
    /// # Returns
    /// The id of the root node of the slice.
    #[inline]
    pub fn create_slice_structural(&mut self, root: NodeID, keep: &HashSet<VarID>) -> NodeID {
        *self
            .create_slices_structural(&[root], keep)
            .get(&root)
            .unwrap()
    }

    /// Creates a slice of a BDD containing all except the given variables.
    ///
    /// * `remove` - List of the variables to remove
    /// * `root` - The root node of the BDD that is supposed to be sliced
    ///
    /// # Returns
    /// The id of the root node of the slice.
    #[inline]
    pub fn create_slice_without_vars_structural(
        &mut self,
        root: NodeID,
        remove: &HashSet<VarID>,
    ) -> NodeID {
        *self
            .create_slices_without_vars_structural(&[root], remove)
            .get(&root)
            .unwrap()
    }

    /// Creates slices of BDDs containing only the given variables.
    ///
    /// * `keep` - List of the variables to keep
    /// * `roots` - The root nodes of the BDDs that are supposed to be sliced
    ///
    /// # Returns
    /// A HashMap giving the id of the root node of the created slice for each BDD (the keys to the
    /// HashMap are the ids of the root nodes of the original BDDs, as given in the node_ids
    /// parameter).
    pub fn create_slices_structural(
        &mut self,
        roots: &[NodeID],
        keep: &HashSet<VarID>,
    ) -> HashMap<NodeID, NodeID> {
        let remove = (1..(self.var2level.len() - 1))
            .map(VarID)
            .filter(|var_id| !keep.contains(var_id))
            .collect::<HashSet<_>>();

        self.create_slices_without_vars_structural(roots, &remove)
    }

    /// Creates slices of BDDs containing all except the given variables.
    ///
    /// * `remove` - List of the variables to remove
    /// * `roots` - The root nodes of the BDDs that are supposed to be sliced
    ///
    /// # Returns
    /// A HashMap giving the id of the root node of the created slice for each BDD (the keys to the
    /// HashMap are the ids of the root nodes of the original BDDs, as given in the node_ids
    /// parameter).
    pub fn create_slices_without_vars_structural(
        &mut self,
        roots: &[NodeID],
        remove: &HashSet<VarID>,
    ) -> HashMap<NodeID, NodeID> {
        let new_ids_forced_to_false = self.create_tmp_bdds_with_forced_vars(roots, remove, false);
        let new_ids_forced_to_true = self.create_tmp_bdds_with_forced_vars(roots, remove, true);

        roots
            .iter()
            .map(|node_id| {
                (
                    *node_id,
                    self.and(
                        *new_ids_forced_to_false.get(node_id).unwrap(),
                        *new_ids_forced_to_true.get(node_id).unwrap(),
                    ),
                )
            })
            .collect()
    }

    /// Creates a new BDD with the specified variables being forced to the specified value (by
    /// connecting the other edge of the variables nodes directly to 0). Returns a translation of
    /// the old nodes to the new nodes.
    fn create_tmp_bdds_with_forced_vars(
        &mut self,
        roots: &[NodeID],
        vars: &HashSet<VarID>,
        force_to: bool,
    ) -> HashMap<NodeID, NodeID> {
        let relevant_nodes = self.get_reachable_with_forced_vars(roots, vars, force_to);

        let mut bottom_up_vars = order_to_layernames(&self.var2level);
        bottom_up_vars.reverse();

        let mut new_ids: HashMap<NodeID, NodeID> = HashMap::default();

        let mut changed = false;
        bottom_up_vars.iter().for_each(|var_id| {
            if vars.contains(var_id) {
                // Force var to desired value
                self.level2nodes[self.var2level[var_id.0]]
                    .clone()
                    .iter()
                    .for_each(|DDNode { id, var, low, high }| {
                        let new_node = if force_to {
                            DDNode {
                                id: NodeID(0),
                                var: *var,
                                low: ZERO.id,
                                high: *new_ids.get(high).unwrap(),
                            }
                        } else {
                            DDNode {
                                id: NodeID(0),
                                var: *var,
                                low: *new_ids.get(low).unwrap(),
                                high: ZERO.id,
                            }
                        };
                        new_ids.insert(*id, self.node_get_or_create(&new_node));
                    });
                changed = true;
            } else if changed {
                // Update Nodes in case of new children
                self.level2nodes[self.var2level[var_id.0]]
                    .clone()
                    .iter()
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

    /// Creates a Set containing only the variables reachable from the specified root nodes with
    /// the specified variables being forced to the given value.
    fn get_reachable_with_forced_vars(
        &mut self,
        roots: &[NodeID],
        vars: &HashSet<VarID>,
        force_to: bool,
    ) -> HashSet<NodeID> {
        let mut reachable = HashSet::default();

        let mut stack = roots.to_vec();

        while let Some(x) = stack.pop() {
            if reachable.contains(&x) {
                continue;
            }

            let node = self.nodes.get(&x).unwrap();

            if !vars.contains(&node.var) {
                stack.push(node.low);
                stack.push(node.high);
            } else if force_to {
                stack.push(node.high);
            } else {
                stack.push(node.low);
            }
            reachable.insert(x);
        }

        reachable
    }
}

#[cfg(test)]
mod tests {}
