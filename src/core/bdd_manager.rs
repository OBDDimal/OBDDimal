//! All BDD building and manipulation functionality

use std::fmt;

use dimacs::Clause;
use rand::Rng;

use crate::{
    core::bdd_node::{DDNode, NodeID, VarID},
    misc::hash_select::{HashMap, HashSet},
};

/// Terminal node "zero"
pub const ZERO: DDNode = DDNode {
    id: NodeID(0),
    var: VarID(0),
    low: NodeID(0),
    high: NodeID(0),
};

/// Terminal node "one"
pub const ONE: DDNode = DDNode {
    id: NodeID(1),
    var: VarID(0),
    low: NodeID(1),
    high: NodeID(1),
};

/// Bring ITE calls of the form
/// ite(f,f,h) = ite(f,1,h) = ite(h,1,f)
/// ite(f,g,f) = ite(f,g,0) = ite(g,f,0)
/// into canonical form
fn normalize_ite_args(mut f: NodeID, mut g: NodeID, mut h: NodeID) -> (NodeID, NodeID, NodeID) {
    if f == g {
        g = ONE.id;
    } else if f == h {
        h = ZERO.id
    }

    fn order(a: NodeID, b: NodeID) -> (NodeID, NodeID) {
        // TODO: "Efficient implementation of a BDD package" orders by top variable first, is this relevant?
        if a < b {
            (a, b)
        } else {
            (b, a)
        }
    }

    if g == ONE.id {
        (f, h) = order(f, h);
    }
    if h == ZERO.id {
        (f, g) = order(f, g);
    }

    (f, g, h)
}

/// Container combining the nodes list, unique tables, information about current
/// variable ordering and computed table.
#[derive(Clone)]
pub struct DDManager {
    /// Node List
    pub nodes: HashMap<NodeID, DDNode>,
    /// Variable ordering: var2level[v.0] is the depth of variable v in the tree
    pub(crate) var2level: Vec<usize>,
    /// Unique Table for each variable. Note that the indices into this table are the depths of the
    /// variables, not their IDs. The depth of a variable can be determined through
    /// [var2level](`DDManager::var2level`)!
    pub(crate) level2nodes: Vec<HashSet<DDNode>>,
    /// Computed Table: maps (f,g,h) to ite(f,g,h)
    pub(super) c_table: HashMap<(NodeID, NodeID, NodeID), NodeID>,
}

impl fmt::Debug for DDManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DDManager [{} nodes, unique table size {}, cache size {}]",
            self.nodes.len(),
            self.level2nodes.iter().map(|s| s.len()).sum::<usize>(),
            self.c_table.len()
        )
    }
}

impl Default for DDManager {
    fn default() -> Self {
        let mut man = DDManager {
            nodes: Default::default(),
            var2level: Vec::new(),
            level2nodes: Vec::new(),
            c_table: Default::default(),
        };

        man.bootstrap();
        man
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

    /// Ensure var2level vec is valid up to specified variable
    pub(crate) fn ensure_order(&mut self, target: VarID) {
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
        assert_ne!(node.low, node.high, "Creating a node with the same low and high edge creates a non-reduced BDD, which we don't want to do.");

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
        NodeID(0)
    }

    pub(crate) fn one(&self) -> NodeID {
        NodeID(1)
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

        if self.var2level.len() > var.0 {
            let x = self.level2nodes[self.var2level[var.0]].get(&v);

            if let Some(x) = x {
                return x.id;
            }
        }

        self.add_node(v)
    }

    pub fn nith_var(&mut self, var: VarID) -> NodeID {
        let v = DDNode {
            id: NodeID(0),
            var,
            low: NodeID(1),
            high: NodeID(0),
        };

        if self.var2level.len() > var.0 {
            let x = self.level2nodes[self.var2level[var.0]].get(&v);

            if let Some(x) = x {
                return x.id;
            }
        }

        self.add_node(v)
    }

    //------------------------------------------------------------------------//
    // Unitary Operations

    fn not(&mut self, f: NodeID) -> NodeID {
        self.ite(f, NodeID(0), NodeID(1))
    }

    //------------------------------------------------------------------------//
    // Binary Operations

    pub fn and(&mut self, f: NodeID, g: NodeID) -> NodeID {
        self.ite(f, g, NodeID(0))
    }

    pub fn or(&mut self, f: NodeID, g: NodeID) -> NodeID {
        self.ite(f, NodeID(1), g)
    }

    #[allow(dead_code)]
    fn xor(&mut self, f: NodeID, g: NodeID) -> NodeID {
        let ng = self.not(g);

        self.ite(f, ng, g)
    }

    //------------------------------------------------------------------------//
    // N-ary Operations

    /// Find top variable: Highest in tree according to order
    fn min_by_order(&self, fvar: VarID, gvar: VarID, hvar: VarID) -> VarID {
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

    fn ite(&mut self, f: NodeID, g: NodeID, h: NodeID) -> NodeID {
        let (f, g, h) = normalize_ite_args(f, g, h);
        match (f, g, h) {
            (_, NodeID(1), NodeID(0)) => f, // ite(f,1,0)
            (NodeID(1), _, _) => g,         // ite(1,g,h)
            (NodeID(0), _, _) => h,         // ite(0,g,h)
            (_, t, e) if t == e => t,       // ite(f,g,g)
            (_, _, _) => {
                let cache = self.c_table.get(&(f, g, h));

                if let Some(cached) = cache {
                    return *cached;
                }

                let fnode = self.nodes.get(&f).unwrap();
                let gnode = self.nodes.get(&g).unwrap();
                let hnode = self.nodes.get(&h).unwrap();

                let top = self.min_by_order(fnode.var, gnode.var, hnode.var);

                let fxt = fnode.restrict(top, &self.var2level, true);
                let gxt = gnode.restrict(top, &self.var2level, true);
                let hxt = hnode.restrict(top, &self.var2level, true);

                let fxf = fnode.restrict(top, &self.var2level, false);
                let gxf = gnode.restrict(top, &self.var2level, false);
                let hxf = hnode.restrict(top, &self.var2level, false);

                let high = self.ite(fxt, gxt, hxt);
                let low = self.ite(fxf, gxf, hxf);

                if low == high {
                    self.c_table.insert((f, g, h), low);
                    return low;
                }

                let node = DDNode {
                    id: NodeID(0),
                    var: top,
                    low,
                    high,
                };

                let out = self.node_get_or_create(&node);

                self.c_table.insert((f, g, h), out);

                out
            }
        }
    }

    //------------------------------------------------------------------------//
    // Builders

    /// Creates an XOR "ladder"
    ///
    ///
    #[allow(dead_code)]
    fn xor_prim(&mut self, _vars: Vec<usize>) -> usize {
        todo!();
    }

    #[allow(dead_code)]
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

    pub fn purge_retain(&mut self, f: NodeID) {
        let mut keep = HashSet::default();

        let mut stack = vec![f];

        while let Some(x) = stack.pop() {
            if keep.contains(&x) {
                continue;
            }

            let node = self.nodes.get(&x).unwrap();

            stack.push(node.low);
            stack.push(node.high);
            keep.insert(x);
        }

        let mut garbage = self.nodes.clone();

        garbage.retain(|&x, _| !keep.contains(&x) && x.0 > 1);

        for x in &garbage {
            self.level2nodes[self.var2level[x.1.var.0]].remove(x.1);
            self.nodes.remove(x.0);
        }

        self.c_table.retain(|_, x| keep.contains(x));
    }
}
