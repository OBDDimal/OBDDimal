//! View to access a BDD.

use std::{
    collections::BTreeSet,
    fmt,
    hash::{Hash, Hasher},
    ops,
    sync::{Arc, RwLock},
};

use crate::{
    core::{
        bdd_manager::DDManager,
        bdd_node::{NodeID, VarID},
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
    #[allow(dead_code)]
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

    pub fn get_manager(&self) -> Arc<RwLock<DDManager>> {
        self.man.clone()
    }

    pub fn get_root(&self) -> NodeID {
        self.root
    }

    //------------------------------------------------------------------------//
    // Unitary Operations

    pub fn not(&self) -> Arc<Self> {
        Self::new_with_sliced(
            self.man.write().unwrap().not(self.root),
            self.man.clone(),
            self.sliced_vars.clone(),
        )
    }

    //------------------------------------------------------------------------//
    // Binary Operations

    pub fn and(&self, other: &Self) -> Arc<Self> {
        assert_eq!(self.sliced_vars, other.sliced_vars);
        assert!(self.man.read().unwrap().eq(&other.man.read().unwrap()));

        Self::new_with_sliced(
            self.man.write().unwrap().and(self.root, other.root),
            self.man.clone(),
            self.sliced_vars.clone(),
        )
    }

    pub fn or(&self, other: &Self) -> Arc<Self> {
        assert_eq!(self.sliced_vars, other.sliced_vars);
        assert!(self.man.read().unwrap().eq(&other.man.read().unwrap()));

        Self::new_with_sliced(
            self.man.write().unwrap().or(self.root, other.root),
            self.man.clone(),
            self.sliced_vars.clone(),
        )
    }

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
