//! BDD slicing

use crate::{
    core::{
        bdd_manager::DDManager,
        bdd_node::{DDNode, NodeID, VarID, ZERO},
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
    pub fn create_slice_structural(&mut self, root: NodeID, keep: &HashSet<VarID>) -> NodeID {
        *self
            .create_slices_structural(&[root], keep)
            .get(&root)
            .unwrap()
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
                    self.or(
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
        let mut func = if force_to {
            |DDNode { var, high, .. }: &DDNode,
             man: &mut DDManager,
             new_ids: &HashMap<NodeID, NodeID>| {
                man.node_get_or_create(&DDNode {
                    id: NodeID(0),
                    var: *var,
                    low: ZERO.id,
                    high: *new_ids.get(high).unwrap(),
                })
            }
        } else {
            |DDNode { var, low, .. }: &DDNode,
             man: &mut DDManager,
             new_ids: &HashMap<NodeID, NodeID>| {
                man.node_get_or_create(&DDNode {
                    id: NodeID(0),
                    var: *var,
                    low: *new_ids.get(low).unwrap(),
                    high: ZERO.id,
                })
            }
        };

        self.modify_var_nodes(&relevant_nodes, vars, &mut func)
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
    pub fn create_slice_exists(&mut self, root: NodeID, keep: &HashSet<VarID>) -> NodeID {
        *self.create_slices_exists(&[root], keep).get(&root).unwrap()
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
    pub fn create_slice_without_vars_exists(
        &mut self,
        root: NodeID,
        remove: &HashSet<VarID>,
    ) -> NodeID {
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
    pub fn create_slices_exists(
        &mut self,
        roots: &[NodeID],
        keep: &HashSet<VarID>,
    ) -> HashMap<NodeID, NodeID> {
        let remove = (1..(self.var2level.len() - 1))
            .map(VarID)
            .filter(|var_id| !keep.contains(var_id))
            .collect::<HashSet<_>>();

        self.create_slices_without_vars_exists(roots, &remove)
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
    pub fn create_slices_without_vars_exists(
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
    fn slice_structural_ab_ba_eq_1() {
        let (man, root) =
            DDManager::load_from_dddmp_file("examples/sandwich.dimacs.dddmp".to_string()).unwrap();
        let root = root[0];

        slice_structural_ab_ba_eq(man, root, VarID(5), VarID(15));
    }

    #[test]
    fn slice_structural_ab_ba_eq_2() {
        let (man, root) =
            DDManager::load_from_dddmp_file("examples/sandwich.dimacs.dddmp".to_string()).unwrap();
        let root = root[0];

        slice_structural_ab_ba_eq(man, root, VarID(1), VarID(19));
    }

    fn slice_structural_ab_ba_eq(mut man: DDManager, root: NodeID, a: VarID, b: VarID) {
        let a = [a].into_iter().collect::<HashSet<_>>();
        let b = [b].into_iter().collect::<HashSet<_>>();

        let root_bdd_without_a = man.create_slice_without_vars_structural(root, &a);
        let root_bdd_without_b = man.create_slice_without_vars_structural(root, &b);

        assert_eq!(
            man.create_slice_without_vars_structural(root_bdd_without_a, &b),
            man.create_slice_without_vars_structural(root_bdd_without_b, &a),
        );
    }

    #[test]
    fn slice_exists_ab_ba_eq_1() {
        let (man, root) =
            DDManager::load_from_dddmp_file("examples/sandwich.dimacs.dddmp".to_string()).unwrap();
        let root = root[0];

        slice_exists_ab_ba_eq(man, root, VarID(5), VarID(15));
    }

    #[test]
    fn slice_exists_ab_ba_eq_2() {
        let (man, root) =
            DDManager::load_from_dddmp_file("examples/sandwich.dimacs.dddmp".to_string()).unwrap();
        let root = root[0];

        slice_exists_ab_ba_eq(man, root, VarID(1), VarID(19));
    }

    fn slice_exists_ab_ba_eq(mut man: DDManager, root: NodeID, a: VarID, b: VarID) {
        let a = [a].into_iter().collect::<HashSet<_>>();
        let b = [b].into_iter().collect::<HashSet<_>>();

        let root_bdd_without_a = man.create_slice_without_vars_exists(root, &a);
        let root_bdd_without_b = man.create_slice_without_vars_exists(root, &b);

        assert_eq!(
            man.create_slice_without_vars_exists(root_bdd_without_a, &b),
            man.create_slice_without_vars_exists(root_bdd_without_b, &a),
        );
    }

    #[ignore]
    #[test]
    fn slice_eq_exists_structural_1() {
        let (man, root) =
            DDManager::load_from_dddmp_file("examples/sandwich.dimacs.dddmp".to_string()).unwrap();
        let root = root[0];

        slice_eq_exists_structural(man, root, VarID(5), VarID(15));
    }

    #[ignore]
    #[test]
    fn slice_eq_exists_structural_2() {
        let (man, root) =
            DDManager::load_from_dddmp_file("examples/sandwich.dimacs.dddmp".to_string()).unwrap();
        let root = root[0];

        slice_eq_exists_structural(man, root, VarID(1), VarID(19));
    }

    fn slice_eq_exists_structural(mut man: DDManager, root: NodeID, a: VarID, b: VarID) {
        let vars = [a, b].into_iter().collect::<HashSet<_>>();

        assert_eq!(
            man.create_slice_without_vars_structural(root, &vars),
            man.create_slice_without_vars_exists(root, &vars),
        );
    }
}
