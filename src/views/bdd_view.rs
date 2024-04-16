//! View to access a BDD.

use std::{
    collections::BTreeSet,
    fmt,
    hash::{Hash, Hasher},
    ops,
    sync::{Arc, RwLock},
};

use num_bigint::BigUint;

use crate::{
    core::{
        bdd_manager::DDManager,
        bdd_node::{NodeID, VarID},
        options::Options,
    },
    misc::hash_select::HashSet,
};

//#[derive(Clone)]
pub struct BddView {
    man: Arc<RwLock<DDManager>>,
    root: NodeID,
    sliced_vars: HashSet<VarID>,
}

impl Drop for BddView {
    fn drop(&mut self) {
        self.man.clone().write().unwrap().remove_view(self);
    }
}

impl Hash for BddView {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.man.read().unwrap().hash(state);
        self.root.hash(state);
        self.sliced_vars.iter().collect::<BTreeSet<_>>().hash(state);
    }
}

impl PartialEq for BddView {
    fn eq(&self, other: &Self) -> bool {
        #[inline]
        fn calc_hash(view: &BddView) -> u64 {
            let mut s = std::hash::DefaultHasher::new();
            view.hash(&mut s);
            s.finish()
        }

        calc_hash(self) == calc_hash(other)
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

    //TODO bdd, json and xml im-/export

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
