//! View to access a BDD.

use std::{
    collections::BTreeSet,
    fmt,
    hash::{Hash, Hasher},
    ops,
    sync::{Arc, RwLock},
};

use num_bigint::BigUint;
use num_traits::Zero;

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
    sliced_vars: HashSet<VarID>,
}

impl Hash for BddView {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.man_id.hash(state);
        self.root.hash(state);
        self.sliced_vars.iter().collect::<BTreeSet<_>>().hash(state);
    }
}

impl PartialEq for BddView {
    fn eq(&self, other: &Self) -> bool {
        self.man_id == other.man_id
            && self.root == other.root
            && self.sliced_vars == other.sliced_vars
    }
}

impl Eq for BddView {}

impl BddView {
    pub(crate) fn new(root: NodeID, manager: Arc<RwLock<DDManager>>) -> Arc<Self> {
        Self::new_with_sliced(root, manager, HashSet::<VarID>::default())
    }

    pub(crate) fn new_with_sliced(
        root: NodeID,
        manager: Arc<RwLock<DDManager>>,
        sliced_vars: HashSet<VarID>,
    ) -> Arc<Self> {
        let view = Self {
            man: manager.clone(),
            man_id: manager.read().unwrap().get_id(),
            root,
            sliced_vars,
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

    /// Returns the [VarID]s of the sliced variables of this BDD.
    pub fn get_sliced_variables(&self) -> HashSet<VarID> {
        self.sliced_vars.clone()
    }

    //------------------------------------------------------------------------//
    // Building BDDs

    /// Returns a [BddView] for a BDD which represents the function which is constant 0.
    pub fn zero(manager: Arc<RwLock<DDManager>>) -> Arc<BddView> {
        Self::new(manager.clone().read().unwrap().zero(), manager)
    }

    /// Returns a [BddView] for a BDD which represents the function which is constant 1.
    pub fn one(manager: Arc<RwLock<DDManager>>) -> Arc<BddView> {
        Self::new(manager.clone().read().unwrap().one(), manager)
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
        Self::new(
            manager.clone().write().unwrap().xor_prim(without_vars),
            manager,
        )
    }

    //------------------------------------------------------------------------//
    // Unitary Operations

    /// Returns a (new) view on a BDD which represents the inverse of the function of the BDD.
    pub fn not(&self) -> Arc<Self> {
        Self::new_with_sliced(
            self.man.write().unwrap().not(self.root),
            self.man.clone(),
            self.sliced_vars.clone(),
        )
    }

    //------------------------------------------------------------------------//
    // Quantification

    /// Returns a (new) view on the BDD resulting from applying exists with the given variables to
    /// the BDD.
    pub fn exists(&self, vars: &HashSet<VarID>) -> Arc<Self> {
        Self::new_with_sliced(
            self.man.write().unwrap().exists(self.root, vars),
            self.man.clone(),
            self.sliced_vars.clone(),
        )
    }

    /// Returns a (new) view on the BDD resulting from applying forall with the given variables to
    /// the BDD.
    pub fn forall(&self, vars: &HashSet<VarID>) -> Arc<Self> {
        Self::new_with_sliced(
            self.man.write().unwrap().forall(self.root, vars),
            self.man.clone(),
            self.sliced_vars.clone(),
        )
    }

    //------------------------------------------------------------------------//
    // Binary Operations

    /// Returns a (new) view on the BDD resulting from connecting this views' BDD and another one
    /// given via the parameter with an `and`.
    pub fn and(&self, other: &Self) -> Arc<Self> {
        assert_eq!(self.sliced_vars, other.sliced_vars);
        assert!(self.man.read().unwrap().eq(&other.man.read().unwrap()));

        Self::new_with_sliced(
            self.man.write().unwrap().and(self.root, other.root),
            self.man.clone(),
            self.sliced_vars.clone(),
        )
    }

    /// Returns a (new) view on the BDD resulting from connecting this views' BDD and another one
    /// given via the parameter with an `or`.
    pub fn or(&self, other: &Self) -> Arc<Self> {
        assert_eq!(self.sliced_vars, other.sliced_vars);
        assert!(self.man.read().unwrap().eq(&other.man.read().unwrap()));

        Self::new_with_sliced(
            self.man.write().unwrap().or(self.root, other.root),
            self.man.clone(),
            self.sliced_vars.clone(),
        )
    }

    /// Returns a (new) view on the BDD resulting from connecting this views' BDD and another one
    /// given via the parameter with an `xor`.
    pub fn xor(&self, other: &Self) -> Arc<Self> {
        assert_eq!(self.sliced_vars, other.sliced_vars);
        assert!(self.man.read().unwrap().eq(&other.man.read().unwrap()));

        Self::new_with_sliced(
            self.man.write().unwrap().xor(self.root, other.root),
            self.man.clone(),
            self.sliced_vars.clone(),
        )
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
        let mut man = self.man.write().unwrap();
        let sliced = Self::new_with_sliced(
            man.create_slice_without_vars(self.root, remove),
            self.man.clone(),
            remove
                .union(&self.sliced_vars)
                .copied()
                .collect::<HashSet<_>>(),
        );
        man.clean();
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
    pub fn sat_count(&self) -> BigUint {
        self.man.read().unwrap().sat_count(self.root) >> self.sliced_vars.len()
    }

    //------------------------------------------------------------------------//
    // Atomic Sets

    fn count_models_by_variable(&self) -> HashMap<VarID, BigUint> {
        let man = self.man.read().unwrap();

        let varids = var2level_to_ordered_varids(&man.var2level);
        let count_sliced_vars_between = |first: &VarID, last: &VarID| -> usize {
            ((man.var2level[first.0])..(man.var2level[last.0]))
                .map(|level| varids[level])
                .filter(|var_id| self.sliced_vars.contains(var_id))
                .count()
        };

        let mut node_to_sat_count = HashMap::default();
        man.sat_count_with_cache(self.root, &mut node_to_sat_count);

        let reachable = node_to_sat_count.keys().cloned().collect::<HashSet<_>>();

        let mut paths_to_node: HashMap<NodeID, usize> =
            reachable.iter().map(|node| (*node, 0usize)).collect();

        let mut models_by_variable: HashMap<VarID, BigUint> = varids
            .iter()
            .map(|var| -> (VarID, BigUint) { (*var, Zero::zero()) })
            .collect();

        let root_var = man.nodes.get(&self.root).unwrap().var;
        let top_jump =
            man.var2level[root_var.0] - 1 - count_sliced_vars_between(&varids[0], &root_var);
        paths_to_node.insert(self.root, 2usize.pow(top_jump as u32));

        for var_id in varids.iter() {
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
                            - count_sliced_vars_between(var_id, low_var);

                        let high_var = &man.nodes.get(high_id).unwrap().var;
                        let high_jump = man.var2level[high_var.0]
                            - man.var2level[var_id.0]
                            - 1
                            - count_sliced_vars_between(var_id, high_var);

                        // Calculate this node's model count for the current variable
                        let high_models = node_to_sat_count.get(high_id).unwrap()
                            * BigUint::parse_bytes(b"2", 10)
                                .unwrap()
                                .pow(high_jump as u32);
                        *models_by_variable.get_mut(var_id).unwrap() +=
                            paths_to_node.get(node_id).unwrap() * high_models;

                        // Calculate model counts for skipped variables (through jumps)
                        for (child_id, child_var, child_jump) in
                            [(low_id, low_var, low_jump), (high_id, high_var, high_jump)].iter()
                        {
                            let child_jump = *child_jump;

                            *paths_to_node.get_mut(child_id).unwrap() +=
                                paths_to_node.get(node_id).unwrap() * 2usize.pow(child_jump as u32);

                            if child_jump > 0 {
                                ((man.var2level[var_id.0] + 1)..(man.var2level[child_var.0] - 1))
                                    .map(|level| varids[level])
                                    .filter(|jumped_var_id| {
                                        !self.sliced_vars.contains(jumped_var_id)
                                    })
                                    .for_each(|jumped_var_id| {
                                        *models_by_variable.get_mut(&jumped_var_id).unwrap() +=
                                            paths_to_node.get(node_id).unwrap()
                                                    // The nodes above that got also jumped add *2
                                                    // to the paths, the nodes below add *2 to the
                                                    // sat count below, the current node only uses
                                                    // its high edge… => * 2^(child_jump-1)
                                                * 2usize.pow((child_jump - 1) as u32)
                                                * node_to_sat_count.get(child_id).unwrap();
                                    });
                            }
                        }
                    },
                );
        }

        models_by_variable
    }

    pub fn identify_atomic_sets(&self) -> Vec<HashSet<VarID>> {
        // Identify candidates for atomic sets using commonalities:
        let mut candidates: HashMap<BigUint, HashSet<VarID>> = HashMap::default();
        self.count_models_by_variable()
            .iter()
            .for_each(|(var, model_count)| {
                if !candidates.contains_key(model_count) {
                    candidates.insert(model_count.clone(), HashSet::default());
                }
                candidates.get_mut(model_count).unwrap().insert(*var);
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
                        let a_xor_b = Self::xor_prim(
                            self.man.clone(),
                            &var2level_to_ordered_varids(&self.man.read().unwrap().var2level)
                                .into_iter()
                                .filter(|var| var != a && var != b)
                                .collect(),
                        );
                        let a_xor_b = a_xor_b.as_ref();

                        if !(a_xor_b & self)
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

    //------------------------------------------------------------------------//
    // Graphviz

    /// Generate graphviz for the BDD.
    pub fn graphviz(&self) -> String {
        self.man.read().unwrap().graphviz(self.root)
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
            "BDD_View [Root Node: {:?}, Manager: {:?}, sliced variables: {:?}]",
            self.root, self.man, self.sliced_vars,
        )
    }
}
