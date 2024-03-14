//! BDD slicing

use crate::{
    core::{
        bdd_manager::DDManager,
        bdd_node::{DDNode, NodeID, VarID},
        order::order_to_layernames,
    },
    misc::hash_select::{HashMap, HashSet},
};

impl DDManager {
    /// Creates a slice of a BDD containing only the given variables.
    ///
    /// * `keep` - List of the variables to keep
    /// * `node_id` - The root node of the BDD that is supposed to be sliced
    ///
    /// # Returns
    /// The id of the root node of the slice.
    #[inline]
    pub fn create_slice(&mut self, node_id: NodeID, keep: &[VarID]) -> NodeID {
        *self.create_slices(&[node_id], keep).get(&node_id).unwrap()
    }

    /// Creates slices of BDDs containing only the given variables.
    ///
    /// * `keep` - List of the variables to keep
    /// * `node_ids` - The root nodes of the BDDs that are supposed to be sliced
    ///
    /// # Returns
    /// A HashMap giving the id of the root node of the created slice for each BDD (the keys to the
    /// HashMap are the ids of the root nodes of the original BDDs, as given in the node_ids
    /// parameter).
    pub fn create_slices(
        &mut self,
        node_ids: &[NodeID],
        keep: &[VarID],
    ) -> HashMap<NodeID, NodeID> {
        todo!();
    }
}

#[cfg(test)]
mod tests {}
