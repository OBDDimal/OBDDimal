//! All BDD building and manipulation functionality

use std::{
    fmt,
    hash::{Hash, Hasher},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Weak,
    },
};

use dimacs::Clause;
use rand::Rng;
use weak_table::WeakHashSet;

use crate::{
    core::{
        apply::ApplyOperation,
        bdd_node::{DDNode, NodeID, VarID, ONE, ZERO},
        order::var2level_to_ordered_varids,
    },
    misc::hash_select::{HashMap, HashSet},
    views::bdd_view::BddView,
};

static DDMANAGER_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Container combining the nodes list, unique tables, information about current
/// variable ordering and computed table.
pub struct DDManager {
    /// ID to identify the DDManager
    id: usize,
    /// Node List
    pub nodes: HashMap<NodeID, DDNode>,
    /// Variable ordering: var2level[v.0] is the depth of variable v in the tree
    pub(crate) var2level: Vec<usize>,
    /// Unique Table for each variable. Note that the indices into this table are the depths of the
    /// variables, not their IDs. The depth of a variable can be determined through
    /// [var2level](`DDManager::var2level`)!
    pub(crate) level2nodes: Vec<HashSet<DDNode>>,
    /// Computed Table for ITE: maps (f,g,h) to ite(f,g,h)
    pub(super) ite_c_table: HashMap<(NodeID, NodeID, NodeID), NodeID>,
    /// Computed Table for Apply: maps (op,u,v) to apply(op,u,v)
    pub(super) apply_c_table: HashMap<(ApplyOperation, NodeID, NodeID), NodeID>,
    /// Set of Views for BDDs in this manager
    views: WeakHashSet<Weak<BddView>>,
}

impl Default for DDManager {
    fn default() -> Self {
        let mut man = DDManager {
            id: DDMANAGER_COUNTER.fetch_add(1, Ordering::SeqCst),
            nodes: Default::default(),
            var2level: Vec::new(),
            level2nodes: Vec::new(),
            ite_c_table: Default::default(),
            apply_c_table: Default::default(),
            views: Default::default(),
        };

        man.bootstrap();
        man
    }
}

impl PartialEq for DDManager {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for DDManager {}

impl Hash for DDManager {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Clone for DDManager {
    fn clone(&self) -> Self {
        Self {
            id: DDMANAGER_COUNTER.fetch_add(1, Ordering::SeqCst),
            nodes: self.nodes.clone(),
            var2level: self.var2level.clone(),
            level2nodes: self.level2nodes.clone(),
            ite_c_table: self.ite_c_table.clone(),
            apply_c_table: self.apply_c_table.clone(),
            views: self.views.clone(),
        }
    }
}

/// Determine order in which clauses should be added to BDD
pub(crate) fn align_clauses(clauses: &[Clause]) -> Vec<usize> {
    let mut shuffle: Vec<(usize, f32)> = Vec::default();

    for (i, clause) in clauses.iter().enumerate() {
        let min = clause
            .lits()
            .iter()
            .map(|x| x.var().to_u64())
            .min()
            .unwrap();
        let max = clause
            .lits()
            .iter()
            .map(|x| x.var().to_u64())
            .max()
            .unwrap();

        shuffle.push((i, (clause.len() as f32 * (max - min) as f32)));
    }

    shuffle.sort_by(|x, y| x.1.partial_cmp(&y.1).unwrap());
    shuffle.iter().map(|(x, _)| *x).collect::<Vec<usize>>()
}

impl DDManager {
    /// Initialize the BDD with zero and one constant nodes
    fn bootstrap(&mut self) {
        self.add_node(ZERO);
        self.add_node(ONE);
    }

    /// Add a view to the BDD Manager
    pub(crate) fn get_or_add_view(&mut self, view: BddView) -> Arc<BddView> {
        if let Some(view) = self.views.get(&view) {
            view
        } else {
            let view: Arc<BddView> = view.into();
            self.views.insert(view.clone());
            view
        }
    }

    /// Get all views for this BDD Manager
    pub fn get_views(&self) -> Vec<Arc<BddView>> {
        self.views.iter().collect()
    }

    /// Get all root nodes for this BDD Manager
    pub fn get_roots(&self) -> Vec<NodeID> {
        self.views.iter().map(|v| v.get_root()).collect()
    }

    /// Get the id of the BDD Manager (mostly relevant for checking for equality of two BDD
    /// Managers).
    pub(crate) fn get_id(&self) -> usize {
        self.id
    }

    /// Initializes the BDD for a specific variable ordering.
    ///
    /// # Panics
    /// Only allowed on empty DDManagers. If called on a non-empty DDManager, this function will
    /// panic!
    pub(crate) fn prepare_varorder(&mut self, varorder: Vec<usize>) {
        assert!(
            self.nodes.len() == 2, // The terminal nodes already exist in a new DDManager
            "prepare_varorder is only allowed on empty DDManagers."
        );

        self.var2level.clone_from(&varorder);
        self.level2nodes = vec![HashSet::default(); self.var2level[0] + 1];
        self.bootstrap();
    }

    /// Ensure var2level vec is valid up to specified variable
    fn ensure_order(&mut self, target: VarID) {
        let old_size = self.var2level.len();

        if target.0 < old_size {
            // var2level[target] exists an contains tree depth of target
            return;
        }

        // Ensure there is space for var2level[target]
        self.var2level.resize(target.0 + 1, 0);

        // Fill newly created space:
        let mut y = old_size;
        for x in old_size..self.var2level.len() {
            // var2level[x] = x
            self.var2level[x] = y;
            y += 1;
            // Add newly created level to level2nodes
            while self.level2nodes.len() <= y {
                self.level2nodes.push(HashSet::default());
            }
        }

        // VarID 0 (terminal nodes) at the very bottom of the tree
        self.var2level[0] = y;
    }

    /// Insert Node. ID is assigned for nonterminal nodes (var != 0).
    /// This does not check the unique table, you should do so before using!
    fn add_node(&mut self, mut node: DDNode) -> NodeID {
        if node.id.0 != 0 && node.id.0 != 1 {
            panic!("Adding node With ID > 1: {:?}", node);
        }

        if node.var.0 != 0 && node.id.0 != 0 {
            panic!("Trying to add node with predefined ID that is not a terminal node");
        }

        if node.var != VarID(0) {
            assert_ne!(node.high, node.low);

            // Assign new node ID
            let mut id = NodeID(rand::thread_rng().gen::<usize>());

            while self.nodes.contains_key(&id) {
                id = NodeID(rand::thread_rng().gen::<usize>());
            }

            node.id = id;
        }

        let id = node.id;
        let var = node.var;

        self.nodes.insert(id, node);

        self.ensure_order(var);

        let was_inserted = self.level2nodes[self.var2level[var.0]].insert(node);
        if !was_inserted {
            panic!("Node is already in unique table!");
        }

        id
    }

    /// Search for Node, create if it doesnt exist
    pub(crate) fn node_get_or_create(&mut self, node: &DDNode) -> NodeID {
        // If both child nodes are the same, this node must be replaced by its child node to get a
        // reduced BDD.
        if node.low == node.high {
            return node.low;
        }

        if self.var2level.len() <= node.var.0 {
            // Unique table does not contain any entries for this variable. Create new Node.
            return self.add_node(*node);
        }

        // Lookup in variable-specific unique-table
        let res = self.level2nodes[self.var2level[node.var.0]].get(node);

        match res {
            Some(stuff) => stuff.id,      // An existing node was found
            None => self.add_node(*node), // No existing node found -> create new
        }
    }

    //------------------------------------------------------------------------//
    // Constants

    pub(crate) fn zero(&self) -> NodeID {
        ZERO.id
    }

    pub(crate) fn one(&self) -> NodeID {
        ONE.id
    }

    //------------------------------------------------------------------------//
    // Variables

    pub fn ith_var(&mut self, var: VarID) -> NodeID {
        let v = DDNode {
            id: NodeID(0),
            var,
            low: NodeID(0),
            high: NodeID(1),
        };

        self.node_get_or_create(&v)
    }

    pub fn nith_var(&mut self, var: VarID) -> NodeID {
        let v = DDNode {
            id: NodeID(0),
            var,
            low: NodeID(1),
            high: NodeID(0),
        };

        self.node_get_or_create(&v)
    }

    //------------------------------------------------------------------------//
    // Unitary Operations

    pub fn not(&mut self, f: NodeID) -> NodeID {
        self.ite(f, NodeID(0), NodeID(1))
    }

    //------------------------------------------------------------------------//
    // Quantification

    pub fn exists(&mut self, f: NodeID, vars: &HashSet<VarID>) -> NodeID {
        let reachable = self.get_reachable(&[f]);
        let mut func = |DDNode { high, low, .. }: &DDNode,
                        man: &mut DDManager,
                        new_ids: &HashMap<NodeID, NodeID>| {
            man.or(*new_ids.get(high).unwrap(), *new_ids.get(low).unwrap())
        };

        *self
            .modify_var_nodes(&reachable, vars, &mut func)
            .get(&f)
            .unwrap()
    }

    pub fn forall(&mut self, f: NodeID, vars: &HashSet<VarID>) -> NodeID {
        let reachable = self.get_reachable(&[f]);
        let mut func = |DDNode { high, low, .. }: &DDNode,
                        man: &mut DDManager,
                        new_ids: &HashMap<NodeID, NodeID>| {
            man.and(*new_ids.get(high).unwrap(), *new_ids.get(low).unwrap())
        };

        *self
            .modify_var_nodes(&reachable, vars, &mut func)
            .get(&f)
            .unwrap()
    }

    /// Existential quantification, but on multiple BDDs at the same time. Returns a HashMap, which
    /// can be used to translate the root nodes of the old BDDs to the root nodes of the resulting
    /// BDDs.
    pub fn exists_multiple(
        &mut self,
        fs: &[NodeID],
        vars: &HashSet<VarID>,
    ) -> HashMap<NodeID, NodeID> {
        let reachable = self.get_reachable(fs);
        let mut func = |DDNode { high, low, .. }: &DDNode,
                        man: &mut DDManager,
                        new_ids: &HashMap<NodeID, NodeID>| {
            man.or(*new_ids.get(high).unwrap(), *new_ids.get(low).unwrap())
        };

        self.modify_var_nodes(&reachable, vars, &mut func)
    }

    /// Universal quantification, but on multiple BDDs at the same time. Returns a HashMap, which
    /// can be used to translate the root nodes of the old BDDs to the root nodes of the resulting
    /// BDDs.
    pub fn forall_multiple(
        &mut self,
        fs: &[NodeID],
        vars: &HashSet<VarID>,
    ) -> HashMap<NodeID, NodeID> {
        let reachable = self.get_reachable(fs);
        let mut func = |DDNode { high, low, .. }: &DDNode,
                        man: &mut DDManager,
                        new_ids: &HashMap<NodeID, NodeID>| {
            man.and(*new_ids.get(high).unwrap(), *new_ids.get(low).unwrap())
        };

        self.modify_var_nodes(&reachable, vars, &mut func)
    }

    //------------------------------------------------------------------------//
    // Binary Operations

    pub fn and(&mut self, f: NodeID, g: NodeID) -> NodeID {
        self.apply(ApplyOperation::AND, f, g)
    }

    pub fn or(&mut self, f: NodeID, g: NodeID) -> NodeID {
        self.apply(ApplyOperation::OR, f, g)
    }

    pub fn xor(&mut self, f: NodeID, g: NodeID) -> NodeID {
        self.apply(ApplyOperation::XOR, f, g)
    }

    //------------------------------------------------------------------------//
    // N-ary Operations

    /// Find top variable: Highest in tree according to order
    pub(super) fn min_by_order(&self, fvar: VarID, gvar: VarID, hvar: VarID) -> VarID {
        let list = [fvar, gvar, hvar];

        // Tree depths
        let tlist = [
            self.var2level[fvar.0],
            self.var2level[gvar.0],
            self.var2level[hvar.0],
        ];

        // Minimum tree depth
        let min = *tlist.iter().min().unwrap();
        // Index of Var with minimum tree depth
        let index = tlist.iter().position(|&x| x == min).unwrap();

        list[index]
    }

    //------------------------------------------------------------------------//
    // Builders

    /// Creates an XOR "ladder"
    pub fn xor_prim(&mut self, without_vars: &HashSet<VarID>) -> NodeID {
        let mut vars = var2level_to_ordered_varids(&self.var2level);
        vars.reverse();

        let mut one_side = self.one();
        let mut zero_side = self.zero();

        vars.iter()
            .filter(|v| !without_vars.contains(v))
            .for_each(|var| {
                let one_side_new = self.node_get_or_create(&DDNode {
                    id: NodeID(0),
                    var: *var,
                    low: one_side,
                    high: zero_side,
                });
                let zero_side_new = self.node_get_or_create(&DDNode {
                    id: NodeID(0),
                    var: *var,
                    low: zero_side,
                    high: one_side,
                });

                (one_side, zero_side) = (one_side_new, zero_side_new);
            });

        zero_side
    }

    pub fn verify(&self, f: NodeID, trues: &[usize]) -> bool {
        let mut values: Vec<bool> = vec![false; self.level2nodes.len() + 1];

        for x in trues {
            let x: usize = *x;

            values[x] = x < values.len();
        }

        let mut node_id = f;

        while node_id.0 >= 2 {
            let node = &self.nodes.get(&node_id).unwrap();

            if values[node.var.0] {
                node_id = node.high;
            } else {
                node_id = node.low;
            }
        }

        node_id.0 == 1
    }

    /// Returns a set containing all nodes reachable from the given root nodes.
    pub fn get_reachable(&self, roots: &[NodeID]) -> HashSet<NodeID> {
        let mut keep = HashSet::default();

        let mut stack = roots.to_vec();

        while let Some(x) = stack.pop() {
            if keep.contains(&x) {
                continue;
            }

            let node = self.nodes.get(&x).unwrap();

            stack.push(node.low);
            stack.push(node.high);
            keep.insert(x);
        }

        keep
    }

    /// Removes nodes which do not belong to the BDD with the specified root node from the
    /// [DDManager].
    ///
    /// * `root` - The root node of the BDD which should remain in the [DDManager].
    pub fn purge_retain(&mut self, root: NodeID) {
        self.purge_retain_multi(&[root])
    }

    /// Removes nodes which do not belong to any of the BDDs with the specified root nodes from the
    /// [DDManager].
    ///
    /// * `roots` - The root nodes of the BDDs which should remain in the [DDManager].
    pub fn purge_retain_multi(&mut self, roots: &[NodeID]) {
        let keep = self.get_reachable(roots);

        let mut garbage = self.nodes.clone();

        garbage.retain(|&x, _| !keep.contains(&x) && x.0 > 1);

        for x in &garbage {
            self.level2nodes[self.var2level[x.1.var.0]].remove(x.1);
            self.nodes.remove(x.0);
        }

        self.ite_c_table.retain(|_, x| keep.contains(x));
    }

    /// Removes nodes which do not belong to any of the BDDs for which views exist from the
    /// [DDManager].
    pub fn clean(&mut self) {
        self.views.remove_expired();
        self.purge_retain_multi(&self.get_roots());
    }

    pub fn clear_c_table(&mut self) {
        self.ite_c_table.clear();
        self.apply_c_table.clear();
    }
}

impl fmt::Debug for DDManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DDManager [{} nodes, unique table size {}, cache sizes: {} (ite), {} (apply)]",
            self.nodes.len(),
            self.level2nodes.iter().map(|s| s.len()).sum::<usize>(),
            self.ite_c_table.len(),
            self.apply_c_table.len(),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        core::{
            bdd_manager::DDManager,
            bdd_node::{VarID, ONE, ZERO},
            order::var2level_to_ordered_varids,
        },
        misc::hash_select::HashSet,
    };

    #[test]
    fn exists_all() {
        let (mut man, root) =
            DDManager::load_from_dddmp_file("examples/sandwich.dimacs.dddmp".to_string()).unwrap();
        let root = root[0];
        let vars = var2level_to_ordered_varids(&man.var2level)
            .into_iter()
            .collect::<HashSet<VarID>>();

        assert_eq!(man.exists(root, &vars), ONE.id);
    }

    #[test]
    fn forall_all() {
        let (mut man, root) =
            DDManager::load_from_dddmp_file("examples/sandwich.dimacs.dddmp".to_string()).unwrap();
        let root = root[0];
        let vars = var2level_to_ordered_varids(&man.var2level)
            .into_iter()
            .collect::<HashSet<VarID>>();

        assert_eq!(man.forall(root, &vars), ZERO.id);
    }
}
