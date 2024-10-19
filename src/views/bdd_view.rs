//! View to access a BDD.

use std::{
    collections::BTreeSet,
    fmt,
    hash::{Hash, Hasher},
    ops,
    sync::{Arc, RwLock},
};

use malachite::{num::arithmetic::traits::Pow, Natural};

use crate::{
    core::{
        bdd_manager::DDManager,
        bdd_node::{DDNode, NodeID, VarID},
        options::Options,
        order::var2level_to_ordered_varids,
    },
    misc::hash_select::{HashMap, HashSet},
};

//#[derive(Clone)]
pub struct BddView {
    man: Arc<RwLock<DDManager>>,
    man_id: usize,
    root: NodeID,
    removed_vars: HashSet<VarID>,
    atomic_sets: Option<HashMap<VarID, HashSet<VarID>>>,
}

impl Hash for BddView {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.man_id.hash(state);
        self.root.hash(state);
        self.removed_vars
            .iter()
            .collect::<BTreeSet<_>>()
            .hash(state);
        self.atomic_sets.is_some().hash(state);
    }
}

impl PartialEq for BddView {
    fn eq(&self, other: &Self) -> bool {
        self.man_id == other.man_id
            && self.root == other.root
            && self.removed_vars == other.removed_vars
            && self.atomic_sets.is_some() == other.atomic_sets.is_some()
    }
}

impl Eq for BddView {}

impl BddView {
    pub(crate) fn new(root: NodeID, manager: Arc<RwLock<DDManager>>) -> Arc<Self> {
        Self::new_with_removed_vars(root, manager, HashSet::<VarID>::default())
    }

    pub(crate) fn new_with_removed_vars(
        root: NodeID,
        manager: Arc<RwLock<DDManager>>,
        removed_vars: HashSet<VarID>,
    ) -> Arc<Self> {
        Self::new_with_atomic_sets(root, manager, removed_vars, None)
    }

    fn new_with_atomic_sets(
        root: NodeID,
        manager: Arc<RwLock<DDManager>>,
        removed_vars: HashSet<VarID>,
        atomic_sets: Option<HashMap<VarID, HashSet<VarID>>>,
    ) -> Arc<Self> {
        let view = Self {
            man: manager.clone(),
            man_id: manager.read().unwrap().get_id(),
            root,
            removed_vars,
            atomic_sets,
        };
        manager.write().unwrap().get_or_add_view(view)
    }

    /// Gives access to the manager which stores the BDD.
    pub fn get_manager(&self) -> Arc<RwLock<DDManager>> {
        self.man.clone()
    }

    /// Returns the [NodeID] of the root node of this BDD.
    pub fn get_root(&self) -> NodeID {
        self.root
    }

    /// Returns the [VarID]s of the removed variables of this BDD.
    pub fn get_removed_variables(&self) -> HashSet<VarID> {
        self.removed_vars.clone()
    }

    /// Returns a HashMap containing for each atomic set that was used for optimization the
    /// variable that represents the atomic set (as the key) and the set of variables that
    /// got removed during the optimization (as the Values) or None if no optimizations were
    /// applied.
    pub fn get_optimizations(&self) -> Option<HashMap<VarID, HashSet<VarID>>> {
        self.atomic_sets.clone()
    }

    //------------------------------------------------------------------------//
    // Building BDDs

    /// Returns a [BddView] for a BDD which represents the function which is constant 0.
    pub fn zero(manager: Arc<RwLock<DDManager>>) -> Arc<BddView> {
        let root = manager.clone().read().unwrap().zero();
        Self::new(root, manager)
    }

    /// Returns a [BddView] for a BDD which represents the function which is constant 1.
    pub fn one(manager: Arc<RwLock<DDManager>>) -> Arc<BddView> {
        let root = manager.clone().read().unwrap().one();
        Self::new(root, manager)
    }

    /// Build a bdd from dimacs. The BDD is stored in a new DDManager created by this function.
    pub fn from_dimacs(
        dimacs: String,
        order: Option<Vec<usize>>,
        options: Options,
    ) -> Result<Arc<BddView>, String> {
        let mut instance =
            dimacs::parse_dimacs(&dimacs).map_err(|_| "Failed to parse dimacs file.")?;
        let (man, root) = DDManager::from_instance(&mut instance, order, options)?;

        Ok(BddView::new(root, RwLock::new(man).into()))
    }

    /// Builds a xor ladder with all variables in the DDManager except the given ones and returns a
    /// [BddView] for the resulting BDD.
    pub fn xor_prim(
        manager: Arc<RwLock<DDManager>>,
        without_vars: &HashSet<VarID>,
    ) -> Arc<BddView> {
        let root = manager.clone().write().unwrap().xor_prim(without_vars);
        Self::new(root, manager)
    }

    //------------------------------------------------------------------------//
    // Unitary Operations

    /// Returns a (new) view on a BDD which represents the inverse of the function of the BDD.
    pub fn not(&self) -> Arc<Self> {
        let root = self.man.write().unwrap().not(self.root);
        Self::new_with_removed_vars(root, self.man.clone(), self.removed_vars.clone())
    }

    //------------------------------------------------------------------------//
    // Quantification

    /// Returns a (new) view on the BDD resulting from applying exists with the given variables to
    /// the BDD.
    pub fn exists(&self, vars: &HashSet<VarID>) -> Arc<Self> {
        assert!(self.atomic_sets.is_none());

        let root = self.man.write().unwrap().exists(self.root, vars);
        let result = Self::new_with_removed_vars(root, self.man.clone(), self.removed_vars.clone());
        self.man.write().unwrap().clean();

        result
    }

    /// Returns a (new) view on the BDD resulting from applying forall with the given variables to
    /// the BDD.
    pub fn forall(&self, vars: &HashSet<VarID>) -> Arc<Self> {
        assert!(self.atomic_sets.is_none());

        let root = self.man.write().unwrap().forall(self.root, vars);
        let result = Self::new_with_removed_vars(root, self.man.clone(), self.removed_vars.clone());
        self.man.write().unwrap().clean();

        result
    }

    //------------------------------------------------------------------------//
    // Binary Operations

    /// Returns a (new) view on the BDD resulting from connecting this views' BDD and another one
    /// given via the parameter with an `and`.
    pub fn and(&self, other: &Self) -> Arc<Self> {
        assert_eq!(self.removed_vars, other.removed_vars);
        assert!(self.man.read().unwrap().eq(&other.man.read().unwrap()));
        assert!(self.atomic_sets.is_none() && other.atomic_sets.is_none());

        let root = self.man.write().unwrap().and(self.root, other.root);
        Self::new_with_removed_vars(root, self.man.clone(), self.removed_vars.clone())
    }

    /// Returns a (new) view on the BDD resulting from connecting this views' BDD and another one
    /// given via the parameter with an `or`.
    pub fn or(&self, other: &Self) -> Arc<Self> {
        assert_eq!(self.removed_vars, other.removed_vars);
        assert!(self.man.read().unwrap().eq(&other.man.read().unwrap()));
        assert!(self.atomic_sets.is_none() && other.atomic_sets.is_none());

        let root = self.man.write().unwrap().or(self.root, other.root);
        Self::new_with_removed_vars(root, self.man.clone(), self.removed_vars.clone())
    }

    /// Returns a (new) view on the BDD resulting from connecting this views' BDD and another one
    /// given via the parameter with an `xor`.
    pub fn xor(&self, other: &Self) -> Arc<Self> {
        assert_eq!(self.removed_vars, other.removed_vars);
        assert!(self.man.read().unwrap().eq(&other.man.read().unwrap()));
        assert!(self.atomic_sets.is_none() && other.atomic_sets.is_none());

        let root = self.man.write().unwrap().xor(self.root, other.root);
        Self::new_with_removed_vars(root, self.man.clone(), self.removed_vars.clone())
    }

    //------------------------------------------------------------------------//
    // Slicing

    /// Creates a slice of the BDD containing only the given variables.
    ///
    /// * `keep` - The variables to keep
    pub fn create_slice(&self, keep: &HashSet<VarID>) -> Arc<Self> {
        let man = self.man.read().unwrap();
        let remove = (1..(man.var2level.len() - 1))
            .map(VarID)
            .filter(|var_id| !keep.contains(var_id))
            .collect::<HashSet<_>>();
        drop(man);

        self.create_slice_without_vars(&remove)
    }

    /// Creates a slice of the BDD containing all except the given variables.
    ///
    /// * `remove` - The variables to remove
    pub fn create_slice_without_vars(&self, remove: &HashSet<VarID>) -> Arc<Self> {
        assert!(self.atomic_sets.is_none());
        let root = self
            .man
            .write()
            .unwrap()
            .create_slice_without_vars(self.root, remove);
        let sliced = Self::new_with_removed_vars(
            root,
            self.man.clone(),
            remove
                .union(&self.removed_vars)
                .copied()
                .collect::<HashSet<_>>(),
        );
        self.man.write().unwrap().clean();
        sliced
    }

    //------------------------------------------------------------------------//
    // Import- / Export of BDDs

    fn nodelist_to_viewlist(nodes: Vec<NodeID>, man: DDManager) -> Vec<Arc<BddView>> {
        let man: Arc<RwLock<DDManager>> = RwLock::new(man).into();
        nodes.iter().map(|id| Self::new(*id, man.clone())).collect()
    }

    /// Loads the BDDs from a .dddmp file.
    ///
    /// * `filename` - Name of the .dddmp file.
    pub fn load_from_dddmp_file(filename: String) -> Result<Vec<Arc<BddView>>, String> {
        let (man, roots) = DDManager::load_from_dddmp_file(filename)?;
        Ok(Self::nodelist_to_viewlist(roots, man))
    }

    //------------------------------------------------------------------------//
    // SAT / #SAT

    /// Returns, whether the function represented by this BDD is satisfyable.
    pub fn is_sat(&self) -> bool {
        self.man.read().unwrap().is_sat(self.root)
    }

    /// Returns the #SAT result for the function represented by this BDD.
    pub fn sat_count(&self) -> Natural {
        self.man.read().unwrap().sat_count(self.root)
            >> (self.removed_vars.len()
                + if self.atomic_sets.is_some() {
                    self.atomic_sets
                        .as_ref()
                        .unwrap()
                        .values()
                        .flatten()
                        .count()
                } else {
                    0
                })
    }

    //------------------------------------------------------------------------//
    // Atomic Sets

    fn count_models_by_variable(&self) -> (Natural, HashMap<VarID, Natural>) {
        assert!(self.atomic_sets.is_none());

        let man = self.man.read().unwrap();

        let varids = var2level_to_ordered_varids(&man.var2level);
        let count_removed_vars_between = |first: &VarID, last: &VarID| -> usize {
            if self.removed_vars.is_empty() {
                0
            } else {
                ((man.var2level[first.0])..(man.var2level[last.0]))
                    .map(|level| varids[level])
                    .filter(|var_id| self.removed_vars.contains(var_id))
                    .count()
            }
        };

        let mut node_to_sat_count = HashMap::default();
        let sat_count = man.sat_count_with_cache(self.root, &mut node_to_sat_count);

        let reachable = node_to_sat_count.keys().cloned().collect::<HashSet<_>>();

        let mut paths_to_node: HashMap<NodeID, Natural> = reachable
            .iter()
            .map(|node| (*node, Natural::from(0usize)))
            .collect();

        let mut models_by_variable: HashMap<VarID, Natural> = varids
            .iter()
            .map(|var| -> (VarID, Natural) { (*var, Natural::from(0usize)) })
            .collect();

        let root_var = man.nodes.get(&self.root).unwrap().var;
        let top_jump =
            man.var2level[root_var.0] - 1 - count_removed_vars_between(&varids[0], &root_var);
        paths_to_node.insert(self.root, Natural::from(2usize).pow(top_jump as u64));

        for var_id in varids.iter() {
            if *var_id == VarID(0) {
                continue;
            }

            man.level2nodes[man.var2level[var_id.0]]
                .iter()
                .filter(|node| reachable.contains(&node.id))
                .for_each(
                    |DDNode {
                         id: node_id,
                         low: low_id,
                         high: high_id,
                         ..
                     }| {
                        let low_var = &man.nodes.get(low_id).unwrap().var;
                        let low_jump = man.var2level[low_var.0]
                            - man.var2level[var_id.0]
                            - 1
                            - count_removed_vars_between(var_id, low_var);

                        let high_var = &man.nodes.get(high_id).unwrap().var;
                        let high_jump = man.var2level[high_var.0]
                            - man.var2level[var_id.0]
                            - 1
                            - count_removed_vars_between(var_id, high_var);

                        // Calculate this node's model count for the current variable
                        let high_models = node_to_sat_count.get(high_id).unwrap()
                            * Natural::from(2usize).pow(high_jump as u64);
                        *models_by_variable.get_mut(var_id).unwrap() +=
                            paths_to_node.get(node_id).unwrap() * high_models;

                        // Calculate model counts for skipped variables (through jumps)
                        for (child_id, child_var, child_jump) in
                            [(low_id, low_var, low_jump), (high_id, high_var, high_jump)].iter()
                        {
                            let child_jump = *child_jump;

                            let paths_to_current_node = paths_to_node.get(node_id).unwrap().clone();
                            *paths_to_node.get_mut(child_id).unwrap() += paths_to_current_node
                                * Natural::from(2usize).pow(child_jump as u64);

                            if child_jump > 0 {
                                ((man.var2level[var_id.0] + 1)..(man.var2level[child_var.0] - 1))
                                    .map(|level| varids[level])
                                    .filter(|jumped_var_id| {
                                        !self.removed_vars.contains(jumped_var_id)
                                    })
                                    .for_each(|jumped_var_id| {
                                        *models_by_variable.get_mut(&jumped_var_id).unwrap() +=
                                            paths_to_node.get(node_id).unwrap()
                                                    // The nodes above that got also jumped add *2
                                                    // to the paths, the nodes below add *2 to the
                                                    // sat count below, the current node only uses
                                                    // its high edge… => * 2^(child_jump-1)
                                                * Natural::from(2usize).pow((child_jump - 1) as u64)
                                                * node_to_sat_count.get(child_id).unwrap();
                                    });
                            }
                        }
                    },
                );
        }

        // Remove removed vars
        self.removed_vars.iter().for_each(|var_id| {
            models_by_variable.remove(var_id);
        });

        (sat_count, models_by_variable)
    }

    pub fn identify_atomic_sets(&self) -> Vec<HashSet<VarID>> {
        if self.atomic_sets.is_some() {
            return self
                .atomic_sets
                .as_ref()
                .unwrap()
                .iter()
                .map(|(top_var, set)| {
                    let mut set = set.clone();
                    set.insert(*top_var);
                    set
                })
                .collect::<Vec<HashSet<VarID>>>();
        }

        // Identify candidates for atomic sets using commonalities:
        let mut candidates: HashMap<Natural, HashSet<VarID>> = HashMap::default();
        let (sat_count, models_by_variable) = self.count_models_by_variable();
        models_by_variable.iter().for_each(|(var, model_count)| {
            // Exclude free variables
            if *model_count != sat_count {
                if !candidates.contains_key(model_count) {
                    candidates.insert(model_count.clone(), HashSet::default());
                }
                candidates.get_mut(model_count).unwrap().insert(*var);
            }
        });

        // Sort out false postitives:
        candidates
            .values()
            .filter(|candidate_set| candidate_set.len() > 1)
            .flat_map(|candidate_set| {
                let mut result_sets: Vec<HashSet<VarID>> = Vec::default();
                candidate_set.iter().for_each(|candidate_var| {
                    let mut inserted = false;
                    for set in result_sets.iter_mut() {
                        let (a, b) = (candidate_var, set.iter().next().unwrap());
                        let without_vars =
                            var2level_to_ordered_varids(&self.man.read().unwrap().var2level)
                                .into_iter()
                                .filter(|var| var != a && var != b)
                                .collect();
                        let a_xor_b = Self::xor_prim(self.man.clone(), &without_vars);
                        let a_xor_b = a_xor_b.as_ref();

                        let a_xor_b_and_self = a_xor_b & self;
                        if !a_xor_b_and_self
                            .exists(&vec![*a, *b].into_iter().collect())
                            .is_sat()
                        {
                            set.insert(*candidate_var);
                            inserted = true;
                            break;
                        }
                    }
                    if !inserted {
                        result_sets.push(vec![*candidate_var].into_iter().collect());
                    }
                });
                result_sets
            })
            .filter(|candidate_set| candidate_set.len() > 1)
            .collect()
    }

    /// Optimizes the BDD by making use of Atomic Sets. Returns None if this view already uses an
    /// optimized BDD or otherwise a (new) View for the optimized BDD.
    pub fn optimize_through_atomic_sets(&self) -> Option<Arc<Self>> {
        if self.atomic_sets.is_some() {
            return None;
        }

        let var2level = &self.man.read().unwrap().var2level.clone();

        let atomic_sets = self
            .identify_atomic_sets()
            .into_iter()
            .map(|mut set| {
                let top = *set
                    .iter()
                    .min_by(|var_a, var_b| var2level[var_a.0].cmp(&var2level[var_b.0]))
                    .unwrap();
                set.remove(&top);
                (top, set)
            })
            .collect::<HashMap<VarID, HashSet<VarID>>>();

        let root = self.man.write().unwrap().exists(
            self.root,
            &atomic_sets
                .values()
                .flatten()
                .cloned()
                .collect::<HashSet<VarID>>(),
        );

        let optimized = Self::new_with_atomic_sets(
            root,
            self.man.clone(),
            self.removed_vars.clone(),
            Some(atomic_sets),
        );
        self.man.write().unwrap().clean();

        Some(optimized)
    }

    //------------------------------------------------------------------------//
    // Graphviz

    /// Generate graphviz for the BDD.
    pub fn graphviz(&self) -> String {
        assert!(self.atomic_sets.is_none());
        self.man.read().unwrap().graphviz(self.root)
    }

    //------------------------------------------------------------------------//
    // Other

    /// Evaluates the BDD, setting all given vars to 1 and all remaining vars to 0.
    ///
    /// * `trues` - The variables that should be set to true
    pub fn evaluate(&self, trues: &[VarID]) -> bool {
        // Check if all the variables in each atomic set have the same value before evaluation, if
        // atomic sets have been optimized
        let atomic_sets_check = match &self.atomic_sets {
            Some(atomic_sets) => {
                let trues_set = trues.iter().cloned().collect::<HashSet<VarID>>();
                atomic_sets.keys().all(|var_id| {
                    atomic_sets.get(var_id).unwrap().iter().all(|other_var| {
                        trues_set.contains(var_id) == trues_set.contains(other_var)
                    })
                })
            }
            None => true,
        };

        atomic_sets_check && self.man.read().unwrap().evaluate(self.root, trues)
    }

    /// Counts how many nodes the BDD consists of.
    pub fn count_nodes(&self) -> usize {
        self.man.read().unwrap().count_active(self.root)
    }
}

impl ops::Not for &BddView {
    type Output = Arc<BddView>;

    fn not(self) -> Self::Output {
        self.not()
    }
}

impl ops::BitAnd for &BddView {
    type Output = Arc<BddView>;

    fn bitand(self, rhs: Self) -> Self::Output {
        self.and(rhs)
    }
}

impl ops::BitOr for &BddView {
    type Output = Arc<BddView>;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.or(rhs)
    }
}

impl ops::BitXor for &BddView {
    type Output = Arc<BddView>;

    fn bitxor(self, rhs: Self) -> Self::Output {
        self.xor(rhs)
    }
}

impl fmt::Debug for BddView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BDD_View [Root Node: {:?}, Manager: {:?}, removed variables: {:?}, atomic set optimizations: {:?}]",
            self.root, self.man, self.removed_vars, self.atomic_sets,
        )
    }
}

#[cfg(test)]
mod test {
    use crate::{core::bdd_node::VarID, views::bdd_view::BddView};

    #[test]
    fn optimizations_sandwich() {
        comparison_with_without_optimization("examples/sandwich.dimacs.dddmp".to_string());
    }

    #[test]
    fn optimizations_berkeleydb() {
        comparison_with_without_optimization("examples/berkeleydb.dimacs-nce.dddmp".to_string());
    }

    #[ignore]
    #[test]
    fn optimizations_embtoolkit() {
        comparison_with_without_optimization("examples/embtoolkit.dimacs.dddmp".to_string());
    }

    #[ignore]
    #[test]
    fn optimizations_busybox() {
        comparison_with_without_optimization("examples/busybox_1.18.0.dimacs.dddmp".to_string());
    }

    #[ignore]
    #[test]
    fn optimizations_finanzialservices01() {
        comparison_with_without_optimization(
            "examples/financialservices01.dimacs.dddmp".to_string(),
        );
    }

    #[ignore]
    #[test]
    fn optimizations_automotive_02_v1() {
        comparison_with_without_optimization("examples/automotive02_v1.dimacs.dddmp".to_string());
    }

    #[ignore]
    #[test]
    fn optimizations_automotive_02_v2() {
        comparison_with_without_optimization("examples/automotive02_v2.dimacs.dddmp".to_string());
    }

    #[ignore]
    #[test]
    fn optimizations_automotive_02_v3() {
        comparison_with_without_optimization("examples/automotive02_v3.dimacs.dddmp".to_string());
    }

    #[ignore]
    #[test]
    fn optimizations_automotive_02_v4() {
        comparison_with_without_optimization("examples/automotive02_v4.dimacs.dddmp".to_string());
    }

    #[ignore]
    #[test]
    fn optimizations_automotive_01() {
        comparison_with_without_optimization("examples/automotive01.dimacs.dddmp".to_string());
    }

    #[inline]
    fn comparison_with_without_optimization(dddmp_file: String) {
        let bdd_views = BddView::load_from_dddmp_file(dddmp_file).unwrap();

        for bdd in bdd_views.iter() {
            let optimized_bdd = bdd.optimize_through_atomic_sets().unwrap();

            assert!(bdd.count_nodes() >= optimized_bdd.count_nodes());

            assert_eq!(bdd.sat_count(), optimized_bdd.sat_count());

            let var_count = bdd.man.read().unwrap().var2level.len();
            let all_ones = vec![VarID(1); var_count];
            let alternating: Vec<VarID> = all_ones.iter().copied().step_by(2).collect();
            assert_eq!(bdd.evaluate(&[]), optimized_bdd.evaluate(&[]));
            assert_eq!(bdd.evaluate(&all_ones), optimized_bdd.evaluate(&all_ones));
            assert_eq!(
                bdd.evaluate(&alternating),
                optimized_bdd.evaluate(&alternating)
            );
        }
    }
}
