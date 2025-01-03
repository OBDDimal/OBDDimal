//! Implementation of BDD layer swap

use std::{
    borrow::Borrow,
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
};

use crate::core::{
    bdd_manager::DDManager,
    bdd_node::{DDNode, NodeID, VarID},
};

/// Holds the context of multiple swap operations
/// This allows to calculate the effect of multiple swaps without actually executing them
///
/// # Fields
/// * `all_swaps_in_result` - All swap pairs that have been executed
/// * `new_nodes` - New nodes that have been created during the swaps
/// * `new_level2nodes` - New level2nodes that have been created during the swaps
/// * `node_id_counter` - Counter for new node IDs
///
#[derive(Debug, Clone)]
pub struct SwapContext {
    all_swaps_in_result: Vec<(VarID, VarID)>,
    new_nodes: HashSet<TempNode>,
    new_level2nodes: HashMap<VarID, HashSet<NodeEnum>>,
    node_id_counter: usize,
    referenced_above: Option<(HashSet<NodeID>, usize)>,
}

impl Default for SwapContext {
    fn default() -> Self {
        SwapContext {
            all_swaps_in_result: Vec::new(),
            new_nodes: HashSet::new(),
            new_level2nodes: HashMap::new(),
            node_id_counter: 2,
            referenced_above: None,
        }
    }
}

impl SwapContext {
    pub fn new(man: &DDManager, range: &(usize, usize)) -> Self {
        let mut context = SwapContext::default();
        context.precalc_references(man, range.0, range.1);
        context
    }
    pub fn precalc_references(&mut self, manager: &DDManager, from: usize, to: usize) {
        let mut referenced_above = HashSet::new();
        let mut in_range = HashSet::new();
        for level in from..=to {
            for node in manager.level2nodes[level].iter() {
                in_range.insert(node.id);
            }
        }
        for level in 0..from {
            for node in manager.level2nodes[level].iter() {
                if in_range.contains(&node.high) {
                    referenced_above.insert(node.high);
                }
                if in_range.contains(&node.low) {
                    referenced_above.insert(node.low);
                }
            }
        }
        self.referenced_above = Some((referenced_above, from));
    }

    /// get from original var2level to the var2level after all swaps
    ///
    /// # Arguments
    ///
    /// * `v2l` - The original var2level
    pub fn permute_swaps(&self, v2l: &Vec<usize>) -> Vec<usize> {
        let mut v2l = v2l.clone();
        self.all_swaps_in_result.iter().for_each(|(a, b)| {
            v2l.swap(a.0, b.0);
        });
        v2l
    }

    pub fn get_swaps_in_result(&self) -> Vec<(VarID, VarID)> {
        self.all_swaps_in_result.clone()
    }

    /// get var at level after all swaps
    ///
    /// # Arguments
    ///
    /// * `level` - The level
    /// * `v2l` - The original var2level
    pub fn var_at_level(&self, level: usize, v2l: &Vec<usize>) -> Option<VarID> {
        self.var_at_level_post_calc(level, &self.permute_swaps(v2l))
    }

    /// get var at level for current v2l
    ///
    /// # Arguments
    ///
    /// * `level` - The level
    /// * `v2l` - The current var2level
    pub fn var_at_level_post_calc(&self, level: usize, v2l: &Vec<usize>) -> Option<VarID> {
        v2l.iter()
            .enumerate()
            .find(|(_, &l)| l == level)
            .map(|(v, _)| VarID(v))
    }

    /// get level of variable after all swaps
    ///
    /// # Arguments
    ///
    /// * `var` - The variable
    /// * `v2l` - The original var2level
    pub fn var2level(&self, v2l: &Vec<usize>, var: usize) -> usize {
        self.permute_swaps(v2l)[var]
    }
}

/// Holds ids for either TempNodes or DDNodes
#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
enum IDEnum {
    NewID(usize),
    OldID(NodeID),
}

/// Holds a new temporary node
#[derive(Debug, Eq, Copy, Clone)]
pub struct TempNode {
    id: IDEnum,
    var: VarID,
    low: IDEnum,
    high: IDEnum,
}

/// Enum to store either a new node or an existing "old" node
#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
enum NodeEnum {
    NewNode(TempNode),
    OldNode(DDNode),
}

impl Borrow<IDEnum> for TempNode {
    fn borrow(&self) -> &IDEnum {
        &self.id
    }
}

impl std::fmt::Display for TempNode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "(id: {:?}, var: {:?}, high: {:?}, low: {:?})",
            self.id, self.var, self.high, self.low
        )
    }
}

impl std::fmt::Display for IDEnum {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            IDEnum::NewID(id) => write!(f, "NewID({})", id),
            IDEnum::OldID(id) => write!(f, "OldID({})", id.0),
        }
    }
}

/// Test equality of two nodes, not considering the ID!
impl PartialEq for TempNode {
    fn eq(&self, that: &Self) -> bool {
        self.var == that.var && self.low == that.low && self.high == that.high
    }
}

impl Hash for TempNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl From<&DDNode> for NodeEnum {
    fn from(val: &DDNode) -> Self {
        NodeEnum::OldNode(*val)
    }
}

impl Into<TempNode> for DDNode {
    fn into(self) -> TempNode {
        TempNode {
            id: IDEnum::OldID(self.id),
            var: self.var,
            low: IDEnum::OldID(self.low),
            high: IDEnum::OldID(self.high),
        }
    }
}

impl Into<TempNode> for &DDNode {
    fn into(self) -> TempNode {
        TempNode {
            id: IDEnum::OldID(self.id),
            var: self.var,
            low: IDEnum::OldID(self.low),
            high: IDEnum::OldID(self.high),
        }
    }
}

impl Into<TempNode> for NodeEnum {
    fn into(self) -> TempNode {
        match self {
            NodeEnum::NewNode(node) => node,
            NodeEnum::OldNode(node) => node.into(),
        }
    }
}

impl Into<TempNode> for &NodeEnum {
    fn into(self) -> TempNode {
        match self {
            NodeEnum::NewNode(node) => node.clone(),
            NodeEnum::OldNode(node) => node.into(),
        }
    }
}

impl Into<NodeEnum> for TempNode {
    fn into(self) -> NodeEnum {
        NodeEnum::NewNode(self)
    }
}

impl NodeEnum {
    fn get_var(&self) -> VarID {
        match self {
            NodeEnum::NewNode(node) => node.var,
            NodeEnum::OldNode(node) => node.var,
        }
    }
}

/// Saves a new node in new_nodes or replaces existing node
/// # Arguments
/// * `new_nodes` - HashSet of new nodes
/// * `new_node` - New node to save
/// * `counter` - Counter for new node IDs
fn save_node(
    new_nodes: &mut HashSet<TempNode>,
    new_node: TempNode,
    counter: &mut usize,
) -> TempNode {
    // check that children are different
    assert_ne!(new_node.low, new_node.high);

    // check if id already exists in new_nodes -> Replace Node
    if let Some(_) = new_nodes.get(&new_node.id) {
        assert!(new_nodes.remove(&new_node.id));

        // check that node does not exist any more
        assert!(new_nodes.get(&new_node.id).is_none());
        assert!(new_nodes.get(&new_node).is_none());
        assert!(!new_nodes.contains(&new_node));

        assert!(new_nodes.insert(new_node.clone()));
        return new_node;
    }

    // check if new_node already exists
    match new_nodes.iter().filter(|node| node == &&new_node).next() {
        Some(node) => node.clone(),
        None => {
            // check if new_node has a OldId -> Replace Node
            if let IDEnum::OldID(_) = new_node.id {
                new_nodes.insert(new_node.clone());
                return new_node;
            }

            // create new node
            *counter = *counter + 1;
            let id = IDEnum::NewID(*counter);

            assert!(new_nodes.insert(TempNode {
                id,
                var: new_node.var,
                low: new_node.low,
                high: new_node.high,
            }));

            new_nodes.get(&id).unwrap().clone()
        }
    }
}

impl DDManager {
    fn temp_node_get_or_create(
        &self,
        new_nodes: &mut HashSet<TempNode>,
        node: &TempNode,
        counter: &mut usize,
    ) -> (IDEnum, NodeEnum) {
        if node.low == node.high {
            match self.get_node(&node.low, new_nodes) {
                Some(NodeEnum::OldNode(node)) => {
                    return (IDEnum::OldID(node.id), NodeEnum::OldNode(node.clone()))
                }
                Some(NodeEnum::NewNode(node)) => return (node.id, NodeEnum::NewNode(node)),
                None => panic!("Node not found!"),
            }
        }

        // check again if node exists
        match new_nodes.iter().filter(|n| n == &node).next() {
            Some(n) => {
                return (n.id, NodeEnum::NewNode(n.clone()));
            }
            None => (),
        }

        // check if there is an old node representing the new node
        if let (IDEnum::OldID(low_id), IDEnum::OldID(high_id)) = (node.low, node.high) {
            match self.find_node(&DDNode {
                id: NodeID(0),
                var: node.var,
                high: high_id,
                low: low_id,
            }) {
                Some(id) => {
                    let old_node = self.nodes.get(&id).unwrap();
                    assert_eq!(
                        old_node,
                        &DDNode {
                            id: NodeID(0),
                            var: node.var,
                            high: high_id,
                            low: low_id,
                        }
                    );
                    return (IDEnum::OldID(id), NodeEnum::OldNode(old_node.clone()));
                }
                None => (),
            }
        }
        // No old node exists, create new node
        let new_node = save_node(new_nodes, node.clone(), counter);
        (new_node.id, NodeEnum::NewNode(new_node))
    }

    /// Get a node from new_nodes or nodes
    /// # Arguments
    /// * `id` - ID of the node
    /// * `new_nodes` - HashSet of new nodes
    /// # Returns
    /// Returns the node if it exists in new_nodes or nodes
    fn get_node(&self, id: &IDEnum, new_nodes: &HashSet<TempNode>) -> Option<NodeEnum> {
        match new_nodes.get(id) {
            Some(node) => Some(NodeEnum::NewNode(node.clone())),
            None => match id {
                IDEnum::OldID(id) => Some(NodeEnum::OldNode(self.nodes.get(id).unwrap().clone())),
                IDEnum::NewID(_) => None,
            },
        }
    }
}
impl DDManager {
    /// Swaps two levels of the BDD partially, in the sense, that it does not execute the swap, but returns the necessary changes to the BDD.
    /// This allows to calculate the effect of a swap without actually executing it, thus allowing to calculate the effect of multiple swaps.
    ///
    /// # Arguments
    /// * `a` - Variable a to swap
    /// * `b` - Variable b to swap
    /// * `prev_swap` - Previous swap result, which is used to calculate the effect of the previous swaps and continue from there.
    ///
    /// # Returns
    /// Returns a tuple of the difference in size of the BDD after the swap and the new swap context.
    /// So when the swap reduced the size of the BDD, the difference is negative. The new swap context contains the new nodes and level2nodes.
    pub fn partial_swap(&self, a: VarID, b: VarID, prev_swap: SwapContext) -> (isize, SwapContext) {
        // Reconstruct current var2level (with previous swaps)
        let mut v2l = self.var2level.clone();
        let mut node_id_counter = prev_swap.node_id_counter;
        prev_swap.all_swaps_in_result.iter().for_each(|(a, b)| {
            v2l.swap(a.0, b.0);
        });

        // Create new level2nodes entries
        let mut new_upper_level = HashSet::<NodeEnum>::new();
        let mut new_lower_level = HashSet::<NodeEnum>::new();

        // Create new nodes
        let mut new_nodes = prev_swap.new_nodes.clone();

        // get upper & lower level + their ids
        let (upper_level_var, lower_level_var) = if v2l[b.0] < v2l[a.0] { (b, a) } else { (a, b) };
        let upper_level = v2l[upper_level_var.0];
        let lower_level = v2l[lower_level_var.0];

        if upper_level_var == lower_level_var {
            // BIG PROBLEM
            panic!("Same variable!");
        }

        // current nodes on upper and lower level (either from previous swap or level2nodes)
        let current_upper_level_nodes = match prev_swap.new_level2nodes.get(&upper_level_var) {
            Some(set) => set,
            None => &self.level2nodes[self.var2level[upper_level_var.0]]
                .iter()
                .map(|n| n.into())
                .collect::<HashSet<NodeEnum>>(),
        };

        let current_lower_level_nodes = match prev_swap.new_level2nodes.get(&lower_level_var) {
            Some(set) => set,
            None => &self.level2nodes[self.var2level[lower_level_var.0]]
                .iter()
                .map(|n| n.into())
                .collect::<HashSet<NodeEnum>>(),
        };

        {
            log::info!(
                "Swapping variables {:?} and {:?} (layers {}({}) and {}({}))",
                upper_level_var,
                lower_level_var,
                upper_level,
                current_upper_level_nodes.len(),
                lower_level,
                current_lower_level_nodes.len()
            );

            assert!(upper_level_var.0 != 0 && lower_level_var.0 != 0);
            assert_eq!(
                lower_level,
                upper_level + 1,
                "Variables not on adjacent layers!"
            );
        }

        // Iterate upper level nodes
        // todo comment
        for current_node in current_upper_level_nodes {
            // Convert node to NewTempNode
            let old_node: TempNode = current_node.into();

            // Get Child nodes + IDs
            let child_1_id = old_node.high;
            let child_0_id = old_node.low;

            match self.get_node(&child_1_id, &new_nodes) {
                None => {
                    // child not found
                    continue;
                }
                _ => (),
            }
            match self.get_node(&child_0_id, &new_nodes) {
                None => {
                    // child not found
                    continue;
                }
                _ => (),
            }

            let child_1_node: TempNode = self.get_node(&child_1_id, &new_nodes).unwrap().into();
            let child_0_node: TempNode = self.get_node(&child_0_id, &new_nodes).unwrap().into();

            if child_0_node.var != lower_level_var && child_1_node.var != lower_level_var {
                // current_node does not have connections to level directly below, we leave it as it is.
                // current_node is note effected by swap -> just add to new lower level
                log::debug!(
                    "Children of node {:?} more than one level below, leaving as is.",
                    old_node.id,
                );

                assert_ne!(old_node.high, old_node.low);
                let (_, node) =
                    self.temp_node_get_or_create(&mut new_nodes, &old_node, &mut node_id_counter);

                assert_eq!(upper_level_var, old_node.var);
                new_lower_level.insert(node);
                continue;
            }

            // current_node is effected by swap
            log::debug!("Replacing node {:?} old_f_node={:?}", old_node.id, old_node);

            // Get Grandchildren IDs
            let (child_0_1_id, child_0_0_id) = if child_0_node.var == lower_level_var {
                (child_0_node.high, child_0_node.low)
            } else {
                (child_0_id, child_0_id)
            };
            let (child_1_1_id, child_1_0_id) = if child_1_node.var == lower_level_var {
                (child_1_node.high, child_1_node.low)
            } else {
                (child_1_id, child_1_id)
            };

            // Calculate new children IDs
            let (new_then_id, new_then_node) = self.temp_node_get_or_create(
                &mut new_nodes,
                &TempNode {
                    id: IDEnum::NewID(0),
                    var: upper_level_var,
                    low: child_0_1_id,
                    high: child_1_1_id,
                },
                &mut node_id_counter,
            );

            // when new node is right var, so it is in the new lower level, then add to new lower level
            if new_then_node.get_var() == upper_level_var {
                assert_eq!(upper_level_var, new_then_node.get_var());
                new_lower_level.insert(new_then_node);
            }

            let (new_else_id, new_else_node) = self.temp_node_get_or_create(
                &mut new_nodes,
                &TempNode {
                    id: IDEnum::NewID(0),
                    var: upper_level_var,
                    low: child_0_0_id,
                    high: child_1_0_id,
                },
                &mut node_id_counter,
            );

            // when new node is right var, so it is in the new lower level, then add to new lower level
            if new_else_node.get_var() == upper_level_var {
                assert_eq!(upper_level_var, new_else_node.get_var());
                new_lower_level.insert(new_else_node);
            }

            // Generate new node + replace it in new_nodes
            let new_node = save_node(
                &mut new_nodes,
                TempNode {
                    id: old_node.id,
                    var: lower_level_var,
                    low: new_else_id,
                    high: new_then_id,
                },
                &mut node_id_counter,
            );
            new_upper_level.insert(NodeEnum::NewNode(new_node));

            log::debug!("Replaced node {:?} with {:?}", old_node, new_node);
        }

        // Iterate lower level nodes

        // This swap generates some dangling nodes, which are stored in nodes and are referenced by other nodes, but not stored in the new level2nodes.
        // This causes the swap to be incorrect in certain cases without the reduce function.
        // To fix this we search for nodes that have references to the lower_level nodes and add those referenced lower_level nodes to the new upper level.
        let mut lower_level_ids = current_lower_level_nodes
            .iter()
            .map(|node| match node {
                NodeEnum::NewNode(temp_node) => temp_node.id,
                NodeEnum::OldNode(temp_node) => IDEnum::OldID(temp_node.id),
            })
            .collect::<Vec<IDEnum>>();

        // If a node has no reference above at all, it is a root node
        // let mut lower_level_root = vec![true; lower_level_ids.len()];

        // Pre calculate levels to new nodes
        let level2new_nodes: HashMap<usize, &HashSet<NodeEnum>> = prev_swap
            .new_level2nodes
            .iter()
            .map(|(var, set)| (v2l[var.0], set))
            .collect();

        let (referenced_above, from) = prev_swap
            .referenced_above
            .clone()
            .unwrap_or((HashSet::new(), 0));

        for lower_level_id in lower_level_ids.clone().iter().filter(|id| match id {
            IDEnum::OldID(id) => referenced_above.contains(id),
            IDEnum::NewID(_) => false,
        }) {
            let node = self.get_node(lower_level_id, &new_nodes).unwrap();
            match node {
                NodeEnum::OldNode(node) => {
                    assert_eq!(node.var, lower_level_var);
                }
                NodeEnum::NewNode(node) => {
                    assert_eq!(node.var, lower_level_var);
                }
            }
            lower_level_ids.retain(|&id| id != *lower_level_id);
            new_upper_level.insert(node);
        }

        for (level, level_nodes) in self.level2nodes[0..upper_level].iter().enumerate() {
            if level < from {
                continue;
            }
            // either get new nodes from previous swap or use current nodes
            let node_list = match level2new_nodes.get(&level) {
                None => level_nodes
                    .iter()
                    .map(|n| n.into())
                    .collect::<Vec<NodeEnum>>(),
                Some(set) => set.iter().cloned().collect::<Vec<NodeEnum>>(),
            };
            node_list.iter().for_each(|node| {
                // get high and low id
                let high_id = match node {
                    NodeEnum::OldNode(node) => IDEnum::OldID(node.high),
                    NodeEnum::NewNode(node) => node.high,
                };
                let low_id = match node {
                    NodeEnum::OldNode(node) => IDEnum::OldID(node.low),
                    NodeEnum::NewNode(node) => node.low,
                };

                // todo geht besser!!!
                // if node is referenced by a node above, add it to new upper level
                // if level < upper_level {
                if lower_level_ids.contains(&high_id) {
                    let node = self.get_node(&high_id, &new_nodes).unwrap();
                    match node {
                        NodeEnum::OldNode(node) => {
                            assert_eq!(node.var, lower_level_var);
                        }
                        NodeEnum::NewNode(node) => {
                            assert_eq!(node.var, lower_level_var);
                        }
                    }
                    lower_level_ids.retain(|&id| id != high_id);
                    new_upper_level.insert(node);
                }
                if lower_level_ids.contains(&low_id) {
                    let node = self.get_node(&low_id, &new_nodes).unwrap();
                    match node {
                        NodeEnum::OldNode(node) => {
                            assert_eq!(node.var, lower_level_var);
                        }
                        NodeEnum::NewNode(node) => {
                            assert_eq!(node.var, lower_level_var);
                        }
                    }

                    lower_level_ids.retain(|&id| id != low_id);
                    new_upper_level.insert(node);
                }
            });
        }

        // remove nodes without references from new_nodes
        for id in lower_level_ids {
            new_nodes.remove(&id);
        }

        // Prepare return type
        let mut all_swaps_in_result = prev_swap.all_swaps_in_result.clone();
        all_swaps_in_result.push((a, b));

        let difference: isize = (new_upper_level.len() + new_lower_level.len()) as isize
            - (current_upper_level_nodes.len() + current_lower_level_nodes.len()) as isize;

        let new_level2nodes = {
            let mut map = prev_swap.new_level2nodes.clone();
            map.insert(upper_level_var, new_lower_level);
            map.insert(lower_level_var, new_upper_level);
            map
        };

        return (
            difference,
            SwapContext {
                all_swaps_in_result: all_swaps_in_result,
                new_nodes,
                new_level2nodes,
                node_id_counter,
                referenced_above: prev_swap.referenced_above,
            },
        );
    }

    pub fn persist_swap(&mut self, par_swap: SwapContext) {
        self.clear_c_table();
        let mut new_levels: Vec<_> = par_swap.new_level2nodes.iter().collect();
        let swaps = par_swap.all_swaps_in_result.clone();
        swaps.iter().for_each(|(a, b)| {
            self.var2level.swap(a.0, b.0);
        });
        // sort levels from bottom to top, to reference new child nodes
        // todo check if from lower to higher level
        new_levels.sort_by(|(var_a, _), (var_b, _)| {
            self.var2level[var_b.0].cmp(&self.var2level[var_a.0])
        });

        let mut id2id = HashMap::<IDEnum, NodeID>::new();

        new_levels.iter().for_each(|(var, level_nodes)| {
            let level = self.var2level[var.0];
            // todo check if cloning works
            self.level2nodes[level].clear();
            level_nodes.into_iter().for_each(|node| match node {
                NodeEnum::OldNode(node) => {
                    self.level2nodes[level].insert(node.clone());
                }
                NodeEnum::NewNode(new_node) => {
                    let mut nothing_found = false;
                    let then_id = match id2id.get(&new_node.high) {
                        Some(id) => *id,
                        None => match new_node.high {
                            IDEnum::OldID(id) => id,
                            _ => {
                                nothing_found = true;
                                NodeID(0)
                            }
                        },
                    };

                    let else_id = match id2id.get(&new_node.low) {
                        Some(id) => *id,
                        None => match new_node.low {
                            IDEnum::OldID(id) => id,
                            _ => {
                                nothing_found = true;
                                NodeID(0)
                            }
                        },
                    };

                    assert!(!nothing_found);
                    assert_ne!(then_id, else_id);

                    match new_node.id {
                        IDEnum::OldID(id) => {
                            // replace node
                            let new_f_node = DDNode {
                                id,
                                var: new_node.var,
                                high: then_id,
                                low: else_id,
                            };
                            // Replace node in nodes list
                            *self.nodes.get_mut(&id).unwrap() = new_f_node;

                            // Insert new node in unique-table
                            let inserted = self.level2nodes[level].insert(new_f_node);
                            assert!(inserted);
                        }
                        IDEnum::NewID(_) => {
                            let id = self.node_get_or_create(&DDNode {
                                id: NodeID(0),
                                var: new_node.var,
                                high: then_id,
                                low: else_id,
                            });
                            id2id.insert(new_node.id, id);
                            self.level2nodes[level].insert(self.nodes.get(&id).unwrap().clone());
                        }
                    }
                }
            });
        });
    }

    /// Swaps graph layers of variables a and b. Requires a to be directly above b or vice versa.
    /// Performs reduction which may change NodeIDs. Returns new NodeID of f.
    #[allow(unused)]
    #[must_use]
    pub fn direct_swap(&mut self, a: VarID, b: VarID, f: NodeID) -> isize {
        let result = self.partial_swap(a, b, SwapContext::default());
        self.persist_swap(result.1);
        result.0
    }

    /// Swaps graph layers of variables a and b. Requires a to be directly above b or vice versa.
    /// Performs reduction which may change NodeIDs. Returns new NodeID of f.
    #[allow(unused)]
    #[must_use]
    pub fn swap(&mut self, a: VarID, b: VarID, f: NodeID) -> NodeID {
        // If b above a, switch a and b
        let (a, b) = if (self.var2level[b.0] < self.var2level[a.0]) {
            (b, a)
        } else {
            (a, b)
        };
        log::info!(
            "Swapping variables {:?} and {:?} (layers {} and {})",
            a,
            b,
            self.var2level[a.0],
            self.var2level[b.0]
        );
        assert!(a.0 != 0 && b.0 != 0);
        let upperlevel = self.var2level[a.0];
        let lowerlevel = self.var2level[b.0];
        assert_eq!(
            lowerlevel,
            upperlevel + 1,
            "Variables not on adjacent layers!"
        );

        let ids = self.level2nodes[upperlevel]
            .iter()
            .map(|n| n.id)
            .collect::<Vec<NodeID>>();

        self.var2level.swap(a.0, b.0);
        self.level2nodes[upperlevel].clear();
        assert!(self.level2nodes[upperlevel].is_empty());
        self.level2nodes[lowerlevel].clear();
        assert!(self.level2nodes[lowerlevel].is_empty());

        for id in ids {
            log::debug!("Replacing node {:?}.", id);
            let f_id = id;

            let old_f_node = self.nodes[&f_id];

            let f_1_id = old_f_node.high;
            let f_0_id = old_f_node.low;

            let f_0_node = self.nodes[&f_0_id];
            let f_1_node = self.nodes[&f_1_id];

            if f_0_node.var != b && f_1_node.var != b {
                // f does not have connections to level directly below, we leave it as it is.
                log::debug!(
                    "Children of node {:?} more than one level below, leaving as is.",
                    f_id
                );
                self.level2nodes[lowerlevel].insert(old_f_node);
                continue;
            }

            log::debug!("Replacing node {:?} old_f_node={:?}", f_id, old_f_node);

            let (f_01_id, f_00_id) = if f_0_node.var == b {
                (f_0_node.high, f_0_node.low)
            } else {
                (f_0_id, f_0_id)
            };
            let (f_11_id, f_10_id) = if f_1_node.var == b {
                (f_1_node.high, f_1_node.low)
            } else {
                (f_1_id, f_1_id)
            };

            let new_then_id = if f_01_id == f_11_id {
                f_01_id
            } else {
                self.node_get_or_create(&DDNode {
                    id: NodeID(0),
                    var: a,
                    low: f_01_id,
                    high: f_11_id,
                })
            };
            let new_else_id = if f_00_id == f_10_id {
                f_00_id
            } else {
                self.node_get_or_create(&DDNode {
                    id: NodeID(0),
                    var: a,
                    low: f_00_id,
                    high: f_10_id,
                })
            };

            assert_ne!(new_then_id, new_else_id);

            // Replace F node
            let new_f_node = DDNode {
                id: f_id,
                var: b,
                low: new_else_id,
                high: new_then_id,
            };

            log::debug!("new_f_node={:?}", new_f_node);

            // Replace node in nodes list
            *self.nodes.get_mut(&f_id).unwrap() = new_f_node;

            // Insert new node in unique-table
            let inserted = self.level2nodes[upperlevel].insert(new_f_node);
            assert!(inserted);

            log::debug!("Replaced node {:?} with {:?}", f_id, self.nodes[&f_id]);
        }

        // Clear ITE cache
        self.clear_c_table();

        self.reduce(f)
    }
}

#[cfg(test)]
mod test_par_swap {
    use std::fs;

    use num_bigint::BigUint;
    use num_traits::abs;

    use crate::core::{
        bdd_manager::DDManager, bdd_node::VarID, swap::SwapContext, test::tests::TestCase,
    };

    #[test]
    fn simple_one_swap() {
        let testcase = TestCase::test_trivial();
        let root = testcase.f;
        let mut man = testcase.man.clone();
        man.purge_retain(root);

        let expected = man.sat_count(root);
        let width_before = man.level2nodes[man.var2level[VarID(2).0]].len()
            + man.level2nodes[man.var2level[VarID(3).0]].len();

        // one way
        let result = man.partial_swap(VarID(2), VarID(3), SwapContext::default());
        man.persist_swap(result.1);
        assert_eq!(man.sat_count(root), expected);
        assert!(testcase.verify_against(&man, root));

        // other way
        let result2 = man.partial_swap(VarID(2), VarID(3), SwapContext::default());
        man.persist_swap(result2.1);

        let width_after = man.level2nodes[man.var2level[VarID(2).0]].len()
            + man.level2nodes[man.var2level[VarID(3).0]].len();

        assert_eq!(width_after, width_before);
        assert!(abs(result.0) == abs(result2.0));

        assert_eq!(man.sat_count(root), expected);
        assert!(testcase.verify_against(&man, root));
    }

    #[test]
    fn simple_double_swap() {
        let testcase = TestCase::random_2();
        // let testcase = TestCase::random_1();
        // assert!(testcase.nr_variables == 16);
        let root = testcase.f;
        let mut man = testcase.man.clone();
        man.purge_retain(root);
        let root = man.reduce(root);
        let expected = man.sat_count(root);
        let a = 1;
        let b = 2;

        // double swap
        let result = man.partial_swap(VarID(a), VarID(b), SwapContext::default());
        let result2 = man.partial_swap(VarID(a), VarID(b), result.1);
        man.persist_swap(result2.1);
        assert_eq!(man.sat_count(root), expected);
        assert!(testcase.verify_against(&man, root));
        assert!(abs(result.0) == abs(result2.0));
    }

    #[test]
    fn double_swap_test() {
        let testcase = TestCase::random_2();
        let bdd = testcase.f;
        let mut man = testcase.man.clone();
        man.purge_retain(bdd);
        let count = 5;

        let levels = man.level2nodes.len();
        let expected = man.sat_count(bdd);

        // partial swap with instant resolve
        man.level2nodes[1].iter().for_each(|node| {
            assert_eq!(node.var, VarID(1));
        });

        for _ in 0..count {
            let mut man = man.clone();
            for i in 1..(levels - 2) / 2 {
                let result1 = man.partial_swap(VarID(i), VarID(i + 1), SwapContext::default());
                man.persist_swap(result1.1);
                assert!(testcase.verify_against(&man, bdd));
                // assert_eq!(man.sat_count(bdd), expected);
                let result2 = man.partial_swap(VarID(i), VarID(i + 1), SwapContext::default());
                man.persist_swap(result2.1);
                assert_eq!(abs(result1.0), abs(result2.0), "i: {}", i);
                assert_eq!(man.sat_count(bdd), expected);
                assert!(testcase.verify_against(&man, bdd));
            }
            assert!(testcase.verify_against(&man, bdd));
        }

        assert_eq!(man.sat_count(bdd), expected);

        // partial swap with later resolve
        for _ in 0..count {
            let mut man = man.clone();
            for i in 1..(levels - 2) / 2 {
                let result1 = man.partial_swap(VarID(i), VarID(i + 1), SwapContext::default());
                // assert_eq!(man.sat_count(bdd), expected);
                let result2 = man.partial_swap(VarID(i), VarID(i + 1), result1.1);
                man.persist_swap(result2.1);
                assert_eq!(abs(result1.0), abs(result2.0));
                assert_eq!(man.sat_count(bdd), expected);
                assert!(testcase.verify_against(&man, bdd));
            }
            assert!(testcase.verify_against(&man, bdd));
        }

        assert_eq!(man.sat_count(bdd), expected);

        // assert_eq!(man.sat_count(bdd), expected);
    }

    #[test]
    fn double_swap_berkeleydb_test() {
        let _ = env_logger::builder().is_test(true).try_init();
        let count = 1;

        // Build BDD
        let mut instance = dimacs::parse_dimacs(
            &fs::read_to_string("examples/berkeleydb.dimacs").expect("Failed to read dimacs file."),
        )
        .expect("Failed to parse dimacs file.");
        let (mut man, bdd) =
            DDManager::from_instance(&mut instance, None, Default::default()).unwrap();
        man.purge_retain(bdd);
        let levels = man.level2nodes.len();
        let expected = man.sat_count(bdd);

        // partial swap with instant resolve
        for _ in 0..count {
            let mut man = man.clone();
            for i in 1..(levels - 2) / 2 {
                let result1 = man.partial_swap(VarID(i), VarID(i + 1), SwapContext::default());
                man.persist_swap(result1.1);
                // assert_eq!(man.sat_count(bdd), expected);
                let result2 = man.partial_swap(VarID(i), VarID(i + 1), SwapContext::default());
                man.persist_swap(result2.1);
                assert_eq!(abs(result1.0), abs(result2.0));
                assert_eq!(man.sat_count(bdd), expected);
            }
        }

        assert_eq!(man.sat_count(bdd), expected);

        // partial swap with later resolve
        for _ in 0..count {
            let mut man = man.clone();
            for i in 1..(levels - 2) / 2 {
                let result1 = man.partial_swap(VarID(i), VarID(i + 1), SwapContext::default());
                // assert_eq!(man.sat_count(bdd), expected);
                let result2 = man.partial_swap(VarID(i), VarID(i + 1), result1.1);
                man.persist_swap(result2.1);
                assert_eq!(abs(result1.0), abs(result2.0));
                assert_eq!(man.sat_count(bdd), expected);
            }
        }

        assert_eq!(man.sat_count(bdd), expected);
    }

    /// Test sifting each variables to the bottom
    #[test]
    fn swap_sandwich_top_to_bottom() {
        let _ = env_logger::builder().is_test(true).try_init();

        let expected = BigUint::parse_bytes(b"2808", 10).unwrap();

        let mut instance = dimacs::parse_dimacs(
            &fs::read_to_string("examples/sandwich.dimacs").expect("Failed to read dimacs file."),
        )
        .expect("Failed to parse dimacs file.");
        let (man, bdd) = DDManager::from_instance(&mut instance, None, Default::default()).unwrap();
        let num_vars = match instance {
            dimacs::Instance::Cnf { num_vars, .. } => num_vars as usize,
            _ => panic!("Unsupported dimacs format!"),
        };

        assert_eq!(man.sat_count(bdd), expected);

        for v in 1..num_vars {
            let mut man = man.clone();
            let mut result = (0, SwapContext::default());
            for i in v..num_vars {
                result = man.partial_swap(VarID(v), VarID(i + 1), result.1);
            }
            man.persist_swap(result.1);
            // Use sat_count as sanity check that the BDD isn't completely broken
            assert_eq!(man.sat_count(bdd), expected);
        }
    }

    #[test]
    fn swap_berkeleydb_top_to_bottom_benchmark() {
        let _ = env_logger::builder().is_test(true).try_init();

        // Build BDD
        let mut instance = dimacs::parse_dimacs(
            &fs::read_to_string("examples/berkeleydb.dimacs").expect("Failed to read dimacs file."),
        )
        .expect("Failed to parse dimacs file.");
        let (mut manager, bdd) =
            DDManager::from_instance(&mut instance, None, Default::default()).unwrap();
        manager.purge_retain(bdd);
        let num_vars = match instance {
            dimacs::Instance::Cnf { num_vars, .. } => num_vars as usize,
            _ => panic!("Unsupported dimacs format!"),
        };
        let expected = manager.sat_count(bdd);

        // partial swap with instant resolve
        let man = manager.clone();

        for v in 1..num_vars {
            let mut man = man.clone();
            for i in v..num_vars {
                let result = man.partial_swap(VarID(v), VarID(i + 1), SwapContext::default());
                man.persist_swap(result.1);
            }
        }

        assert_eq!(man.sat_count(bdd), expected);

        // partial swap with later resolve
        let man = manager.clone();

        for v in 1..num_vars {
            let mut man = man.clone();
            let mut result = (0, SwapContext::default());
            for i in v..num_vars {
                result = man.partial_swap(VarID(v), VarID(i + 1), result.1);
            }
            man.persist_swap(result.1);
        }

        assert_eq!(man.sat_count(bdd), expected);

        // assert_eq!(man.sat_count(bdd), expected);
    }
    #[test]
    fn double_swap_benchmark() {
        let _ = env_logger::builder().is_test(true).try_init();

        // Build BDD
        let mut instance = dimacs::parse_dimacs(
            &fs::read_to_string("examples/berkeleydb.dimacs").expect("Failed to read dimacs file."),
        )
        .expect("Failed to parse dimacs file.");
        let (mut manager, bdd) =
            DDManager::from_instance(&mut instance, None, Default::default()).unwrap();
        manager.purge_retain(bdd);

        let levels = manager.level2nodes.len();
        let expected = manager.sat_count(bdd);

        // partial swap with instant resolve
        let mut man = manager.clone();
        for i in 1..(levels - 2) / 2 {
            let result = man.partial_swap(VarID(i), VarID(i + 1), SwapContext::default());
            man.persist_swap(result.1);
            let result = man.partial_swap(VarID(i), VarID(i + 1), SwapContext::default());
            man.persist_swap(result.1);
        }
        assert_eq!(manager.sat_count(bdd), expected);

        // partial swap with later resolve
        let mut man = manager.clone();
        for i in 1..(levels - 2) / 2 {
            let result = man.partial_swap(VarID(i), VarID(i + 1), SwapContext::default());
            let result = man.partial_swap(VarID(i), VarID(i + 1), result.1);
            man.persist_swap(result.1);
        }
        assert_eq!(manager.sat_count(bdd), expected);
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use num_bigint::BigUint;

    use crate::core::{
        bdd_manager::DDManager, bdd_node::VarID, order::var2level_to_ordered_varids,
        test::tests::TestCase,
    };

    /// Swap each variable pair from initial order
    #[test]
    fn swap_sandwich() {
        let testcase = TestCase::test_trivial();

        for i in 1..testcase.nr_variables {
            let mut man = testcase.man.clone();

            let bdd = man.swap(VarID(i), VarID(i + 1), testcase.f);
            assert!(testcase.verify_against(&man, bdd));
        }
    }

    /// Test sifting each variables to the bottom
    #[test]
    fn swap_sandwich_top_to_bottom() {
        let _ = env_logger::builder().is_test(true).try_init();

        let expected = BigUint::parse_bytes(b"2808", 10).unwrap();

        let mut instance = dimacs::parse_dimacs(
            &fs::read_to_string("examples/sandwich.dimacs").expect("Failed to read dimacs file."),
        )
        .expect("Failed to parse dimacs file.");
        let (man, bdd) = DDManager::from_instance(&mut instance, None, Default::default()).unwrap();
        let num_vars = match instance {
            dimacs::Instance::Cnf { num_vars, .. } => num_vars as usize,
            _ => panic!("Unsupported dimacs format!"),
        };

        assert_eq!(man.sat_count(bdd), expected);

        for v in 1..num_vars {
            let mut man = man.clone();
            let mut bdd = bdd;
            for i in v..num_vars {
                bdd = man.swap(VarID(v), VarID(i + 1), bdd);
                // Use sat_count as sanity check that the BDD isnt completely broken
                assert_eq!(man.sat_count(bdd), expected);
            }
        }
    }

    #[test]
    #[should_panic(expected = "Variables not on adjacent layers!")]
    fn swap_failure_non_adjacent() {
        let mut instance = dimacs::parse_dimacs(
            &fs::read_to_string("examples/sandwich.dimacs").expect("Failed to read dimacs file."),
        )
        .expect("Failed to parse dimacs file.");
        let (mut man, bdd) =
            DDManager::from_instance(&mut instance, None, Default::default()).unwrap();
        let _ = man.swap(VarID(1), VarID(3), bdd);
    }

    // Test that reverting a swap results in same node count as before
    fn swap_invert_nodecount(testcase: TestCase) {
        let _ = env_logger::builder().is_test(true).try_init();

        for i in 1..testcase.nr_variables {
            let mut man = testcase.man.clone();
            let var_a = VarID(i);
            let var_b = VarID(i + 1);

            let count_before = man.count_active(testcase.f);

            let bdd = man.swap(var_a, var_b, testcase.f);
            assert!(testcase.verify_against(&man, bdd));

            let bdd = man.swap(var_b, var_a, bdd);
            assert!(testcase.verify_against(&man, bdd));

            let count_after = man.count_active(bdd);

            assert_eq!(count_before, count_after);
        }
    }

    #[test]
    fn swap_invert_nodecount_trivial() {
        swap_invert_nodecount(TestCase::test_trivial());
    }

    #[test]
    fn swap_invert_nodecount_random1() {
        swap_invert_nodecount(TestCase::random_1());
    }

    #[test]
    fn swap_last_vars() {
        let _ = env_logger::builder().is_test(true).try_init();

        let testcase = TestCase::test_trivial();
        fs::write("before.dot", testcase.man.graphviz(testcase.f)).unwrap();

        let mut man = testcase.man.clone();

        let bdd = man.swap(VarID(2), VarID(3), testcase.f);
        fs::write("after.dot", man.graphviz(bdd)).unwrap();

        assert!(testcase.verify_against(&man, bdd));
    }

    fn swap_multiple_noop(testcase: TestCase) {
        let _ = env_logger::builder().is_test(true).try_init();

        let mut man = testcase.man.clone();
        let mut bdd = testcase.f;

        let mut counts = vec![man.count_active(bdd)];

        let var = VarID(1);

        // Sift down, record BDD sizes
        for i in var.0 + 1..testcase.nr_variables + 1 {
            bdd = man.swap(var, VarID(i), bdd);
            man.purge_retain(bdd);

            println!("Swapped, count is now {:?}", man.count_active(bdd));
            println!(
                "Order is now {:?}",
                var2level_to_ordered_varids(&man.var2level)
            );

            assert!(testcase.verify_against(&man, bdd));

            counts.push(man.count_active(bdd));
        }

        let mut counts_up = vec![man.count_active(bdd)];

        // Sift up
        for i in (var.0 + 1..testcase.nr_variables + 1).rev() {
            bdd = man.swap(VarID(i), var, bdd);
            man.purge_retain(bdd);

            println!("Swapped, count is now {:?}", man.count_active(bdd));
            println!(
                "Order is now {:?}",
                var2level_to_ordered_varids(&man.var2level)
            );
            assert!(testcase.verify_against(&man, bdd));

            counts_up.push(man.count_active(bdd));
        }
        counts_up.reverse();

        println!("{:?}\n{:?}", counts, counts_up);

        assert_eq!(counts, counts_up);
    }

    #[test]
    fn swap_multiple_noop_trivial() {
        swap_multiple_noop(TestCase::test_trivial());
    }

    #[test]
    fn swap_multiple_noop_random1() {
        swap_multiple_noop(TestCase::random_1());
    }
}

#[cfg(test)]
mod test_comparison {
    use crate::core::bdd_manager::DDManager;

    #[test]
    fn comparison() {
        let (mut man, nodes) =
            DDManager::load_from_dddmp_file("examples/berkeleydb.dimacs.dddmp".to_string())
                .unwrap();
        let bdd = nodes[0];
        man.purge_retain(bdd);
        let expected = man.sat_count(bdd);
        assert_eq!(man.sat_count(bdd), expected);

        // partial swap
        let mut man = man.clone();
        let mut swap_man = man.clone();
        let mut swap_bdd = bdd.clone();
        for level in 3..man.level2nodes.len() - 2 {
            let a = man.var_at_level(level).unwrap();
            let b = man.var_at_level(level + 1).unwrap();

            let _ = man.direct_swap(a, b, bdd);
            swap_bdd = swap_man.swap(a, b, swap_bdd);
            swap_man.purge_retain(swap_bdd);

            assert_eq!(man.sat_count(bdd), expected);
            assert_eq!(swap_man.sat_count(swap_bdd), expected);

            assert_eq!(
                man.level2nodes[level].len(),
                swap_man.level2nodes[level].len()
            );
            assert_eq!(
                man.level2nodes[level + 1].len(),
                swap_man.level2nodes[level + 1].len()
            );
        }
    }
}
