//! BDD slicing

use crate::{
    core::{
        bdd_manager::DDManager,
        bdd_node::{NodeID, VarID},
    },
    misc::hash_select::{HashMap, HashSet},
};

impl DDManager {
    /// Creates a slice of a BDD containing only the given variables.
    ///
    /// * `keep` - The variables to keep
    /// * `root` - The root node of the BDD that is supposed to be sliced
    ///
    /// # Returns
    /// The id of the root node of the slice.
    ///
    /// # Notes
    /// May create temporary nodes, consider using [`purge_retain_multi`] or [`purge_retain`]
    /// afterwards.
    ///
    /// [`purge_retain_multi`]: crate::core::bdd_manager::DDManager::purge_retain_multi
    /// [`purge_retain`]: crate::core::bdd_manager::DDManager::purge_retain
    #[inline]
    pub fn create_slice(&mut self, root: NodeID, keep: &HashSet<VarID>) -> NodeID {
        *self.create_slices(&[root], keep).get(&root).unwrap()
    }

    /// Creates a slice of a BDD containing all except the given variables.
    ///
    /// * `remove` - The variables to remove
    /// * `root` - The root node of the BDD that is supposed to be sliced
    ///
    /// # Returns
    /// The id of the root node of the slice.
    ///
    /// # Notes
    /// May create temporary nodes, consider using [`purge_retain_multi`] or [`purge_retain`]
    /// afterwards.
    ///
    /// [`purge_retain_multi`]: crate::core::bdd_manager::DDManager::purge_retain_multi
    /// [`purge_retain`]: crate::core::bdd_manager::DDManager::purge_retain
    #[inline]
    pub fn create_slice_without_vars(&mut self, root: NodeID, remove: &HashSet<VarID>) -> NodeID {
        self.exists(root, remove)
    }

    /// Creates slices of BDDs containing only the given variables.
    ///
    /// * `keep` - The variables to keep
    /// * `roots` - The root nodes of the BDDs that are supposed to be sliced
    ///
    /// # Returns
    /// A HashMap giving the id of the root node of the created slice for each BDD (the keys to the
    /// HashMap are the ids of the root nodes of the original BDDs, as given in the node_ids
    /// parameter).
    ///
    /// # Notes
    /// May create temporary nodes, consider using [`purge_retain_multi`] or [`purge_retain`]
    /// afterwards.
    ///
    /// [`purge_retain_multi`]: crate::core::bdd_manager::DDManager::purge_retain_multi
    /// [`purge_retain`]: crate::core::bdd_manager::DDManager::purge_retain
    #[inline]
    pub fn create_slices(
        &mut self,
        roots: &[NodeID],
        keep: &HashSet<VarID>,
    ) -> HashMap<NodeID, NodeID> {
        let remove = (1..(self.var2level.len() - 1))
            .map(VarID)
            .filter(|var_id| !keep.contains(var_id))
            .collect::<HashSet<_>>();

        self.create_slices_without_vars(roots, &remove)
    }

    /// Creates slices of BDDs containing all except the given variables.
    ///
    /// * `remove` - The variables to remove
    /// * `roots` - The root nodes of the BDDs that are supposed to be sliced
    ///
    /// # Returns
    /// A HashMap giving the id of the root node of the created slice for each BDD (the keys to the
    /// HashMap are the ids of the root nodes of the original BDDs, as given in the node_ids
    /// parameter).
    ///
    /// # Notes
    /// May create temporary nodes, consider using [`purge_retain_multi`] or [`purge_retain`]
    /// afterwards.
    ///
    /// [`purge_retain_multi`]: crate::core::bdd_manager::DDManager::purge_retain_multi
    /// [`purge_retain`]: crate::core::bdd_manager::DDManager::purge_retain
    #[inline]
    pub fn create_slices_without_vars(
        &mut self,
        roots: &[NodeID],
        remove: &HashSet<VarID>,
    ) -> HashMap<NodeID, NodeID> {
        self.exists_multiple(roots, remove)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        core::{
            bdd_manager::DDManager,
            bdd_node::{NodeID, VarID},
        },
        misc::hash_select::HashSet,
    };

    #[test]
    fn slice_ab_ba_eq_1() {
        let (man, root) =
            DDManager::load_from_dddmp_file("examples/sandwich.dimacs.dddmp".to_string()).unwrap();
        let root = root[0];

        slice_ab_ba_eq(man, root, VarID(5), VarID(15));
    }

    #[test]
    fn slice_ab_ba_eq_2() {
        let (man, root) =
            DDManager::load_from_dddmp_file("examples/sandwich.dimacs.dddmp".to_string()).unwrap();
        let root = root[0];

        slice_ab_ba_eq(man, root, VarID(1), VarID(19));
    }

    fn slice_ab_ba_eq(mut man: DDManager, root: NodeID, a: VarID, b: VarID) {
        let a = [a].into_iter().collect::<HashSet<_>>();
        let b = [b].into_iter().collect::<HashSet<_>>();

        let root_bdd_without_a = man.create_slice_without_vars(root, &a);
        let root_bdd_without_b = man.create_slice_without_vars(root, &b);

        assert_eq!(
            man.create_slice_without_vars(root_bdd_without_a, &b),
            man.create_slice_without_vars(root_bdd_without_b, &a),
        );
    }
}
