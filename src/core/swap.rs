//! Implementation of BDD layer swap

use core::fmt;
use std::{
    borrow::Borrow,
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    sync::{Arc, RwLock},
    thread::panicking,
};

use itertools::Itertools;

use crate::core::{
    bdd_manager::DDManager,
    bdd_node::{DDNode, NodeID, VarID},
    order::var2level_to_ordered_varids,
};

// Stores temporary nodes during swap, which are then inserted into the unique-table
#[derive(Debug, PartialEq, Eq)]
struct AsyncTempNode {
    id: NodeID,
    var: VarID,
    low: AsyncChildEnum,
    high: AsyncChildEnum,
}

// Stores temporary children during swap, which are then inserted into the unique-table
#[derive(Debug, PartialEq, Eq)]
struct AsyncTempChild {
    id: NodeID,
    var: VarID,
    low: NodeID,
    high: NodeID,
}

// Enum to store either a new child or an existing child
#[derive(Debug, PartialEq, Eq)]
enum AsyncChildEnum {
    NewChild(AsyncTempChild),
    OldChild(NodeID),
}
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

    /// get new SwapContext
    pub fn new() -> Self {
        Default::default()
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

impl Into<NodeEnum> for &DDNode {
    fn into(self) -> NodeEnum {
        NodeEnum::OldNode(self.clone())
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
        // println!("Created new node in save_node: {} -> {}", node, new_node);
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
            // println!(
            //     "Swapping variables {:?} and {:?} (layers {}({}) and {}({}))",
            //     upper_level_id,
            //     lower_level_id,
            //     upper_level,
            //     current_upper_level_nodes.len(),
            //     lower_level,
            //     current_lower_level_nodes.len()
            // );

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
        // println!("########## RESOLVE ##########");

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
                    // assert_ne!(then_id, NodeID(0));
                    // assert_ne!(else_id, NodeID(0));

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
                            if !inserted {
                                // todo maybe resolve
                                // println!("Node already exists: {:?}", new_f_node);
                            }
                            // assert!(inserted);
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

    pub async fn async_swap(manager: Arc<RwLock<Self>>, a: VarID, b: VarID) -> i32 {
        // If b above a, switch a and b

        let mut additional_v2l_upper = HashSet::<DDNode>::new();
        let mut new_v2l_lower = HashSet::<DDNode>::new();

        let mut new_nodes: Vec<AsyncTempNode> = vec![];

        let (upper_level, lower_level, size_before) = {
            let manager = manager.read().unwrap();
            let (a, b) = if manager.var2level[b.0] < manager.var2level[a.0] {
                (b, a)
            } else {
                (a, b)
            };
            log::info!(
                "Swapping variables {:?} and {:?} (layers {}({}) and {}({}))",
                a,
                b,
                manager.var2level[a.0],
                manager.level2nodes[manager.var2level[a.0]].len(),
                manager.var2level[b.0],
                manager.level2nodes[manager.var2level[b.0]].len()
            );
            // println!(
            //     "Swapping variables {:?} and {:?} (layers {}({}) and {}({}))",
            //     a,
            //     b,
            //     manager.var2level[a.0],
            //     manager.level2nodes[manager.var2level[a.0]].len(),
            //     manager.var2level[b.0],
            //     manager.level2nodes[manager.var2level[b.0]].len()
            // );
            assert!(a.0 != 0 && b.0 != 0);
            let upper_level = manager.var2level[a.0];
            let lower_level = manager.var2level[b.0];
            assert_eq!(
                lower_level,
                upper_level + 1,
                "Variables not on adjacent layers!"
            );

            let size_before =
                manager.level2nodes[upper_level].len() + manager.level2nodes[lower_level].len();

            let ids = manager.level2nodes[upper_level]
                .iter()
                .map(|n| n.id)
                .collect::<Vec<NodeID>>();

            for id in ids {
                log::debug!("Replacing node {:?}.", id);
                let f_id = id;

                let old_f_node = manager.nodes[&f_id];

                let f_1_id = old_f_node.high;
                let f_0_id = old_f_node.low;

                let f_0_node = manager.nodes[&f_0_id];
                let f_1_node = manager.nodes[&f_1_id];

                if f_0_node.var != b && f_1_node.var != b {
                    // f does not have connections to level directly below, we leave it as it is.
                    log::debug!(
                        "Children of node {:?} more than one level below, leaving as is.",
                        f_id
                    );

                    new_v2l_lower.insert(old_f_node);
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

                {
                    let new_then_child = if f_01_id == f_11_id {
                        AsyncChildEnum::OldChild(f_01_id)
                    } else {
                        // self.level2nodes[self.var2level[node.var.0]].get(node)
                        let maybe_node = manager.level2nodes[upper_level].get(&DDNode {
                            id: NodeID(0),
                            var: a,
                            high: f_11_id,
                            low: f_01_id,
                        });
                        if let Some(node) = maybe_node {
                            new_v2l_lower.insert(node.clone());
                            AsyncChildEnum::OldChild(node.id)
                        } else {
                            AsyncChildEnum::NewChild(AsyncTempChild {
                                id: NodeID(0),
                                var: a,
                                high: f_11_id,
                                low: f_01_id,
                            })
                        }
                    };

                    let new_else_child = if f_00_id == f_10_id {
                        AsyncChildEnum::OldChild(f_00_id)
                    } else {
                        let maybe_node = manager.level2nodes[upper_level].get(&DDNode {
                            id: NodeID(0),
                            var: a,
                            high: f_10_id,
                            low: f_00_id,
                        });
                        if let Some(node) = maybe_node {
                            new_v2l_lower.insert(node.clone());
                            AsyncChildEnum::OldChild(node.id)
                        } else {
                            AsyncChildEnum::NewChild(AsyncTempChild {
                                id: NodeID(0),
                                var: a,
                                high: f_10_id,
                                low: f_00_id,
                            })
                        }
                    };

                    assert_ne!(new_then_child, new_else_child);

                    let new_f_node = AsyncTempNode {
                        id: f_id,
                        var: b,
                        high: new_then_child,
                        low: new_else_child,
                    };

                    log::debug!("new_f_node={:?}", new_f_node);

                    new_nodes.push(new_f_node);
                }
            }

            // This swap generates some dangling nodes, which are stored in nodes and are referenced by other nodes, but not stored in the level2nodes.
            // This causes the swap to be incorrect in certain cases without the reduce function.
            // To fix this we search for nodes that have references to the lower_level nodes and add those referenced lower_level nodes to the new upper level.
            let lower_level_ids = manager.level2nodes[lower_level]
                .iter()
                .map(|n| n.id)
                .collect::<Vec<NodeID>>();

            let mut lower_level_root = vec![true; lower_level_ids.len()];

            for (level, level_nodes) in manager.level2nodes[0..=upper_level].iter().enumerate() {
                level_nodes.iter().for_each(|node| {
                    if lower_level_ids.contains(&node.high) {
                        lower_level_root[lower_level_ids
                            .iter()
                            .position(|&x| x == node.high)
                            .unwrap()] = false;
                    }
                    if lower_level_ids.contains(&node.low) {
                        lower_level_root
                            [lower_level_ids.iter().position(|&x| x == node.low).unwrap()] = false;
                    }
                    if level < upper_level {
                        if lower_level_ids.contains(&node.high) {
                            additional_v2l_upper.insert(manager.nodes[&node.high]);
                        }
                        if lower_level_ids.contains(&node.low) {
                            additional_v2l_upper.insert(manager.nodes[&node.low]);
                        }
                    }
                });
            }

            lower_level_ids
                .iter()
                .enumerate()
                .filter(|(i, _)| lower_level_root[*i])
                .for_each(|(_, id)| {
                    additional_v2l_upper.insert(manager.nodes[id]);
                });

            (upper_level, lower_level, size_before)
        };

        let mut manager = manager.write().unwrap();

        // Clear ITE cache
        manager.clear_c_table();

        // Clear levels
        manager.var2level.swap(a.0, b.0);
        manager.level2nodes[upper_level].clear();
        assert!(manager.level2nodes[upper_level].is_empty());
        manager.level2nodes[lower_level].clear();
        assert!(manager.level2nodes[lower_level].is_empty());

        manager.level2nodes[lower_level].extend(new_v2l_lower.iter());
        manager.level2nodes[upper_level].extend(additional_v2l_upper.iter());
        // println!(
        //     "new_v2l_lower: {:?}, additional_v2l_upper: {:?}",
        //     new_v2l_lower.len(),
        //     additional_v2l_upper.len()
        // );

        // Add new nodes
        for node in new_nodes {
            let new_then_id = match node.high {
                AsyncChildEnum::OldChild(id) => id,
                AsyncChildEnum::NewChild(temp) => manager.node_get_or_create(&DDNode {
                    id: NodeID(0),
                    var: temp.var,
                    high: temp.high,
                    low: temp.low,
                }),
            };

            let new_else_id = match node.low {
                AsyncChildEnum::OldChild(id) => id,
                AsyncChildEnum::NewChild(temp) => manager.node_get_or_create(&DDNode {
                    id: NodeID(0),
                    var: temp.var,
                    high: temp.high,
                    low: temp.low,
                }),
            };

            assert_ne!(new_then_id, new_else_id);

            let new_f_node = DDNode {
                id: node.id,
                var: node.var,
                high: new_then_id,
                low: new_else_id,
            };

            // Replace node in nodes list
            *manager.nodes.get_mut(&node.id).unwrap() = new_f_node;

            // Insert new node in unique-table
            let inserted = manager.level2nodes[upper_level].insert(new_f_node);
            assert!(inserted);

            log::debug!(
                "Replaced node {:?} with {:?}",
                node.id,
                manager.nodes[&node.id]
            );
        }

        log::debug!(
            "Order is now: {:?} (layers: {:?})",
            manager.var2level,
            var2level_to_ordered_varids(&manager.var2level)
        );
        let size_after_up = manager.level2nodes[upper_level].len();
        let size_after_low = manager.level2nodes[lower_level].len();
        let size_after = size_after_up + size_after_low;
        // println!(
        //     "finished Swapping variables {:?} and {:?} - before: {:?}, after: {:?}({:?}/{:?}) => {:?}",
        //     a, b, size_before, size_after, size_after_up, size_after_low, (size_before as i32 - size_after as i32) as i32
        // );
        (size_before as i32 - size_after as i32) as i32
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

        // log::debug!(
        //     "Order is now: {:?} (layers: {:?})",
        //     self.var2level,
        //     var2level_to_ordered_varids(&self.var2level)
        // );

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
        // let testcase = TestCase::random_1();

        // let mut instance = dimacs::parse_dimacs(
        //     &fs::read_to_string("examples/sandwich.dimacs").expect("Failed to read dimacs file."),
        // )
        // .expect("Failed to parse dimacs file.");
        // let (mut man, root) =
        //     DDManager::from_instance(&mut instance, None, Default::default()).unwrap();

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
        println!("result: {}", result.0);

        // other way
        let result2 = man.partial_swap(VarID(2), VarID(3), SwapContext::default());
        man.persist_swap(result2.1);
        println!("result: {}", result2.0);

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
        println!("result: {}", result.0);
        let result2 = man.partial_swap(VarID(a), VarID(b), result.1);
        man.persist_swap(result2.1);
        println!("result: {}", result2.0);
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

        println!("Start benchmark with {} variables", levels);

        // partial swap with instant resolve
        let start = std::time::Instant::now();

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

        println!("partial_swap - instant resolve: {:?}", start.elapsed());
        assert_eq!(man.sat_count(bdd), expected);

        // partial swap with later resolve
        let start = std::time::Instant::now();

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

        println!("partial_swap - later resolve: {:?}", start.elapsed());
        assert_eq!(man.sat_count(bdd), expected);

        // assert_eq!(man.sat_count(bdd), expected);
    }

    #[test]
    fn double_swap_result_test() {
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
        let num_vars = match instance {
            dimacs::Instance::Cnf { num_vars, .. } => num_vars as usize,
            _ => panic!("Unsupported dimacs format!"),
        };
        let levels = man.level2nodes.len();
        let expected = man.sat_count(bdd);

        println!("Start benchmark with {} variables", num_vars);

        // partial swap with instant resolve
        let start = std::time::Instant::now();

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

        println!("partial_swap - instant resolve: {:?}", start.elapsed());
        assert_eq!(man.sat_count(bdd), expected);

        // partial swap with later resolve
        let start = std::time::Instant::now();

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

        println!("partial_swap - later resolve: {:?}", start.elapsed());
        assert_eq!(man.sat_count(bdd), expected);

        // assert_eq!(man.sat_count(bdd), expected);
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
            println!("#################################################### {}", v);
            for i in v..num_vars {
                result = man.partial_swap(VarID(v), VarID(i + 1), result.1);
                println!("result: {}", result.0);
            }
            man.persist_swap(result.1);
            // Use sat_count as sanity check that the BDD isn't completely broken
            assert_eq!(man.sat_count(bdd), expected);
        }
    }

    #[test]
    fn swap_top_to_bottom_benchmark() {
        let _ = env_logger::builder().is_test(true).try_init();

        // Build BDD
        let mut instance = dimacs::parse_dimacs(
            &fs::read_to_string("examples/berkeleydb.dimacs").expect("Failed to read dimacs file."),
        )
        .expect("Failed to parse dimacs file.");
        let (mut man, bdd) =
            DDManager::from_instance(&mut instance, None, Default::default()).unwrap();
        man.purge_retain(bdd);
        let num_vars = match instance {
            dimacs::Instance::Cnf { num_vars, .. } => num_vars as usize,
            _ => panic!("Unsupported dimacs format!"),
        };
        let expected = man.sat_count(bdd);

        println!("Start benchmark with {} variables", num_vars);

        // partial swap with instant resolve
        let man = man.clone();
        let start = std::time::Instant::now();

        for v in 1..num_vars {
            let mut man = man.clone();
            for i in v..num_vars {
                let result = man.partial_swap(VarID(v), VarID(i + 1), SwapContext::default());
                man.persist_swap(result.1);
            }
        }

        println!("partial_swap - instant resolve: {:?}", start.elapsed());
        assert_eq!(man.sat_count(bdd), expected);

        // partial swap with later resolve
        let man = man.clone();
        let start = std::time::Instant::now();

        for v in 1..num_vars {
            let mut man = man.clone();
            let mut result = (0, SwapContext::default());
            for i in v..num_vars {
                result = man.partial_swap(VarID(v), VarID(i + 1), result.1);
            }
            man.persist_swap(result.1);
        }

        println!("partial_swap - later resolve: {:?}", start.elapsed());
        assert_eq!(man.sat_count(bdd), expected);

        // Normal swap
        let man = man.clone();
        let start = std::time::Instant::now();

        for v in 1..(num_vars - 2) {
            let mut man = man.clone();
            let mut bdd = bdd;
            for i in v..(num_vars - 1) {
                bdd = man.swap(VarID(v), VarID(i + 1), bdd);
            }
        }

        println!("Normal swap: {:?}", start.elapsed());
        assert_eq!(man.sat_count(bdd), expected);

        // assert_eq!(man.sat_count(bdd), expected);
    }
    #[test]
    fn double_swap_benchmark() {
        let _ = env_logger::builder().is_test(true).try_init();
        let count = 5;

        // Build BDD
        let mut instance = dimacs::parse_dimacs(
            &fs::read_to_string("examples/berkeleydb.dimacs").expect("Failed to read dimacs file."),
        )
        .expect("Failed to parse dimacs file.");
        let (mut man, bdd) =
            DDManager::from_instance(&mut instance, None, Default::default()).unwrap();
        man.purge_retain(bdd);
        let num_vars = match instance {
            dimacs::Instance::Cnf { num_vars, .. } => num_vars as usize,
            _ => panic!("Unsupported dimacs format!"),
        };
        let levels = man.level2nodes.len();
        let expected = man.sat_count(bdd);

        println!("Start benchmark with {} variables", num_vars);

        // partial swap with instant resolve
        let start = std::time::Instant::now();

        for _ in 0..count {
            let mut man = man.clone();
            for i in 1..(levels - 2) / 2 {
                let result = man.partial_swap(VarID(i), VarID(i + 1), SwapContext::default());
                man.persist_swap(result.1);
                // assert_eq!(man.sat_count(bdd), expected);
                let result = man.partial_swap(VarID(i), VarID(i + 1), SwapContext::default());
                man.persist_swap(result.1);
                // assert_eq!(man.sat_count(bdd), expected);
            }
        }

        println!("partial_swap - instant resolve: {:?}", start.elapsed());
        assert_eq!(man.sat_count(bdd), expected);

        // partial swap with later resolve
        let start = std::time::Instant::now();

        for _ in 0..count {
            let mut man = man.clone();
            for i in 1..(levels - 2) / 2 {
                let result = man.partial_swap(VarID(i), VarID(i + 1), SwapContext::default());
                // assert_eq!(man.sat_count(bdd), expected);
                let result = man.partial_swap(VarID(i), VarID(i + 1), result.1);
                man.persist_swap(result.1);
                // assert_eq!(man.sat_count(bdd), expected);
            }
        }

        println!("partial_swap - later resolve: {:?}", start.elapsed());
        assert_eq!(man.sat_count(bdd), expected);

        // Normal swap
        let start = std::time::Instant::now();

        for _ in 0..count {
            let mut man = man.clone();
            let mut bdd = bdd;
            for i in 1..(levels - 2) / 2 {
                bdd = man.swap(VarID(i), VarID(i + 1), bdd);
                // assert_eq!(man.sat_count(bdd), expected);
                bdd = man.swap(VarID(i), VarID(i + 1), bdd);
                // assert_eq!(man.sat_count(bdd), expected);
            }
        }

        println!("Normal swap: {:?}", start.elapsed());
        assert_eq!(man.sat_count(bdd), expected);

        // assert_eq!(man.sat_count(bdd), expected);
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

            println!("Swapping variables {:?} and {:?}", var_a, var_b);

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
mod tests_async {
    use std::{
        fs,
        sync::{Arc, RwLock},
    };

    use ::futures::future;

    use crate::core::{
        bdd_manager::DDManager, bdd_node::VarID, order::var2level_to_ordered_varids,
        test::tests::TestCase,
    };

    #[tokio::test]
    async fn swap_simultaneously_test() {
        let testcase = TestCase::random_2();
        assert!(testcase.nr_variables == 16);

        let root = testcase.f;
        let expected = testcase.man.sat_count(root);

        let manager = Arc::new(RwLock::new(testcase.man.clone()));

        let swap1 = tokio::spawn(DDManager::async_swap(manager.clone(), VarID(1), VarID(2)));
        let swap2 = tokio::spawn(DDManager::async_swap(manager.clone(), VarID(5), VarID(6)));
        let swap3 = tokio::spawn(DDManager::async_swap(manager.clone(), VarID(10), VarID(11)));

        future::join_all([swap1, swap2, swap3]).await;

        assert!(testcase.verify_against(&manager.read().unwrap(), root));
        assert_eq!(manager.read().unwrap().sat_count(root), expected);
    }

    #[tokio::test]
    async fn simple_async_swap_test() {
        let testcase = TestCase::random_2();
        assert!(testcase.nr_variables == 16);

        let before_active_count = testcase.man.count_active(testcase.f);

        let manager = Arc::new(RwLock::new(testcase.man.clone()));

        DDManager::async_swap(manager.clone(), VarID(2), VarID(3)).await;
        assert!(testcase.verify_against(&manager.read().unwrap(), testcase.f));

        DDManager::async_swap(manager.clone(), VarID(2), VarID(3)).await;
        assert!(testcase.verify_against(&manager.read().unwrap(), testcase.f));
        let after_active_count: usize = manager.read().unwrap().count_active(testcase.f);

        assert!(before_active_count == after_active_count);
    }

    #[tokio::test]
    async fn swap_back_and_forth_sandwich_async() {
        let _ = env_logger::builder().is_test(true).try_init();

        // Build BDD
        let mut instance = dimacs::parse_dimacs(
            &fs::read_to_string("examples/berkeleydb.dimacs").expect("Failed to read dimacs file."),
        )
        .expect("Failed to parse dimacs file.");
        let (mut man, mut bdd) =
            DDManager::from_instance(&mut instance, None, Default::default()).unwrap();
        let expected = man.sat_count(bdd);
        bdd = man.reduce(bdd);

        let levels = man.level2nodes.len();
        let manager = Arc::new(RwLock::new(man));

        for i in 1..(levels - 2) {
            // for i in 13..(14) {
            let before = { manager.read().unwrap().count_active(bdd) };
            let forth = DDManager::async_swap(manager.clone(), VarID(i), VarID(i + 1)).await;
            let back = DDManager::async_swap(manager.clone(), VarID(i), VarID(i + 1)).await;
            let after = { manager.read().unwrap().count_active(bdd) };

            bdd = manager.write().unwrap().reduce(bdd);

            assert_eq!(i32::abs(forth) - i32::abs(back), 0);
            assert_eq!(before, after);
            assert_eq!(manager.read().unwrap().sat_count(bdd), expected);
        }
    }

    /// Swap each variable pair from initial order
    #[tokio::test]
    async fn swap_sandwich_par() {
        let testcase = TestCase::test_trivial();
        for i in 1..testcase.nr_variables {
            let manager = Arc::new(RwLock::new(testcase.man.clone()));

            DDManager::async_swap(manager.clone(), VarID(i), VarID(i + 1)).await;
            assert!(testcase.verify_against(&manager.read().unwrap(), testcase.f));
        }
    }

    #[tokio::test]
    #[should_panic(expected = "Variables not on adjacent layers!")]
    async fn swap_failure_non_adjacent_par() {
        let mut instance = dimacs::parse_dimacs(
            &fs::read_to_string("examples/sandwich.dimacs").expect("Failed to read dimacs file."),
        )
        .expect("Failed to parse dimacs file.");
        let (man, _) = DDManager::from_instance(&mut instance, None, Default::default()).unwrap();
        let manager = Arc::new(RwLock::new(man.clone()));
        let _ = DDManager::async_swap(manager.clone(), VarID(1), VarID(3)).await;
        // let _ = man.swap(VarID(1), VarID(3), bdd);
    }

    // Test that reverting a swap results in same node count as before
    async fn swap_invert_nodecount_par(testcase: TestCase) {
        let _ = env_logger::builder().is_test(true).try_init();

        for i in 1..testcase.nr_variables {
            let man = testcase.man.clone();
            let var_a = VarID(i);
            let var_b = VarID(i + 1);

            let count_before = man.count_active(testcase.f);

            let manager = Arc::new(RwLock::new(man.clone()));

            let mut bdd;
            DDManager::async_swap(manager.clone(), var_a, var_b).await;
            // let bdd = man.swap(var_a, var_b, testcase.f);
            {
                bdd = manager.write().unwrap().reduce(testcase.f);
            }
            assert!(testcase.verify_against(&manager.read().unwrap(), bdd));

            DDManager::async_swap(manager.clone(), var_b, var_a).await;
            {
                bdd = manager.write().unwrap().reduce(bdd);
            }
            // let bdd = man.swap(var_b, var_a, bdd);
            assert!(testcase.verify_against(&manager.read().unwrap(), bdd));

            let count_after = manager.read().unwrap().count_active(bdd);

            assert_eq!(count_before, count_after);
        }
    }

    #[tokio::test]
    async fn swap_invert_nodecount_trivial_par() {
        swap_invert_nodecount_par(TestCase::test_trivial()).await;
    }

    #[tokio::test]
    async fn swap_invert_nodecount_random1_par() {
        swap_invert_nodecount_par(TestCase::random_1()).await;
    }

    #[tokio::test]
    async fn swap_last_vars_par() {
        let _ = env_logger::builder().is_test(true).try_init();

        let testcase = TestCase::test_trivial();
        fs::write("before.dot", testcase.man.graphviz(testcase.f)).unwrap();

        let manager: Arc<RwLock<DDManager>> = Arc::new(RwLock::new(testcase.man.clone()));

        DDManager::async_swap(manager.clone(), VarID(2), VarID(3)).await;
        // man.swap(VarID(2), VarID(3), testcase.f);
        let man = manager.read().unwrap();
        fs::write("after.dot", man.graphviz(testcase.f)).unwrap();

        assert!(testcase.verify_against(&man, testcase.f));
    }

    async fn swap_multiple_noop_par(testcase: TestCase) {
        let _ = env_logger::builder().is_test(true).try_init();

        let man = testcase.man.clone();
        let mut bdd = testcase.f;

        let mut counts = vec![man.count_active(bdd)];

        let var = VarID(1);

        let manager: Arc<RwLock<DDManager>> = Arc::new(RwLock::new(man.clone()));

        // let bdd = DDManager::par_swap(manager.clone(), var_a, var_b, testcase.f);

        // Sift down, record BDD sizes
        for i in var.0 + 1..testcase.nr_variables + 1 {
            DDManager::async_swap(manager.clone(), var, VarID(i)).await;
            {
                bdd = manager.write().unwrap().reduce(bdd);
            }
            let mut man = manager.write().unwrap();
            //bdd = man.swap(var, VarID(i), bdd);
            man.purge_retain(bdd);

            println!("Swapped, count is now {:?}", man.count_active(bdd));
            println!(
                "Order is now {:?}",
                var2level_to_ordered_varids(&man.var2level)
            );

            assert!(testcase.verify_against(&man, bdd));

            counts.push(man.count_active(bdd));
        }

        let mut counts_up = vec![manager.read().unwrap().count_active(bdd)];

        // Sift up
        for i in (var.0 + 1..testcase.nr_variables + 1).rev() {
            DDManager::async_swap(manager.clone(), VarID(i), var).await;
            {
                bdd = manager.write().unwrap().reduce(bdd);
            }
            let mut man = manager.write().unwrap();
            // bdd = man.swap(VarID(i), var, bdd);
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

    #[tokio::test]
    async fn swap_multiple_noop_trivial_par() {
        swap_multiple_noop_par(TestCase::test_trivial()).await;
    }

    #[tokio::test]
    async fn swap_multiple_noop_random1_par() {
        swap_multiple_noop_par(TestCase::random_1()).await;
    }
}

#[cfg(test)]
mod test_comparions {
    use crate::core::{
        bdd_manager::DDManager,
        swap::{self, SwapContext},
    };

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

#[cfg(test)]
mod evaluation_swap {

    use std::sync::Arc;

    use futures::future;
    use rayon::iter::{IntoParallelIterator, ParallelIterator};
    use tokio::{runtime::Runtime, task::JoinHandle};

    use crate::core::{
        bdd_manager::DDManager,
        dvo::{
            area_generation::{AreaSelection, ThresholdMethod},
            dvo_strategies::{gen_permutation, median},
        },
        swap::SwapContext,
    };

    static N: u32 = 10;
    static PATH: &str = "examples/financialServices01.dimacs.dddmp";
    // static PATH: &str = "examples/berkeleydb.dimacs.dddmp";

    #[test]
    fn info() {
        let (mut man, nodes) = DDManager::load_from_dddmp_file(PATH.to_string()).unwrap();
        let bdd = nodes[0];

        man.purge_retain(bdd);

        let nodes = man
            .level2nodes
            .clone()
            .into_iter()
            .map(|level| level.len())
            .collect::<Vec<usize>>();

        println!("Model: {}", PATH);
        println!("Level: {}", nodes.len());
        println!("Nodes: {}", nodes.into_iter().sum::<usize>());
        println!("N times: {}", N);
    }

    #[test]
    fn swap_eval_pairs() {
        // Build BDD
        let (mut man, nodes) = DDManager::load_from_dddmp_file(PATH.to_string()).unwrap();
        let bdd = nodes[0];

        man.purge_retain(bdd);
        let start_level = man.var2level[man.nodes.get(&bdd).unwrap().var.0] + 1;

        println!("Swap pairs");

        // swap count
        let mut count = 0;
        for _ in 0..N {
            for _ in start_level..man.level2nodes.len() - 2 {
                count += 2;
            }
        }
        println!("swap count: {}", count / N);

        // regular swap
        let start = std::time::Instant::now();
        for _ in 0..N {
            let mut man = man.clone();
            let mut bdd = bdd.clone();
            for level in start_level..man.level2nodes.len() - 2 {
                let a = man.var_at_level(level).unwrap();
                let b = man.var_at_level(level + 1).unwrap();
                bdd = man.swap(a, b, bdd);
                bdd = man.swap(a, b, bdd);
            }
        }
        println!("Regular swap took {:?}", start.elapsed() / N);

        // // async swap
        // let start = std::time::Instant::now();
        // for _ in 0..N {
        //     let manager = Arc::new(RwLock::new(man.clone()));
        //     for level in start_level..man.level2nodes.len() - 2 {
        //         let a = man.var_at_level(level).unwrap();
        //         let b = man.var_at_level(level + 1).unwrap();

        //         let _: Result<i32, tokio::task::JoinError> =
        //             tokio::spawn(DDManager::async_swap(manager.clone(), a, b)).await;
        //         let _ = tokio::spawn(DDManager::async_swap(manager.clone(), a, b)).await;
        //     }
        // }
        // println!("async swap took {:?}", start.elapsed() / N);

        // partial swap instant resolve
        let start = std::time::Instant::now();
        for _ in 0..N {
            let mut man = man.clone();
            for level in start_level..man.level2nodes.len() - 2 {
                let a = man.var_at_level(level).unwrap();
                let b = man.var_at_level(level + 1).unwrap();
                let swap_context = DDManager::partial_swap(&man, a, b, SwapContext::default());
                man.persist_swap(swap_context.1);
                let swap_context = DDManager::partial_swap(&man, a, b, SwapContext::default());
                man.persist_swap(swap_context.1);
            }
        }
        println!(
            "Partial swap instant resolve took {:?}",
            start.elapsed() / N
        );

        // partial swap
        let start = std::time::Instant::now();
        for _ in 0..N {
            let mut man = man.clone();
            for level in start_level..man.level2nodes.len() - 2 {
                let a = man.var_at_level(level).unwrap();
                let b = man.var_at_level(level + 1).unwrap();
                let swap_context = DDManager::partial_swap(&man, a, b, SwapContext::default());
                let swap_context = DDManager::partial_swap(&man, a, b, swap_context.1);
                man.persist_swap(swap_context.1);
            }
        }
        println!("Partial swap took {:?}", start.elapsed() / N);

        // partial swap
        let start = std::time::Instant::now();
        for _ in 0..N {
            let mut man = man.clone();

            let runtime = Runtime::new().unwrap();

            let manager = Arc::new(man.clone());
            let first_batch_futures = (start_level..man.level2nodes.len() - 2)
                .step_by(2)
                .into_iter()
                .map(|level| {
                    let man = manager.clone();
                    runtime.spawn_blocking(move || {
                        let a = man.var_at_level(level).unwrap();
                        let b = man.var_at_level(level + 1).unwrap();
                        let swap_context =
                            DDManager::partial_swap(&man, a, b, SwapContext::default());
                        let swap_context = DDManager::partial_swap(&man, a, b, swap_context.1);
                        swap_context.1
                    })
                })
                .collect::<Vec<JoinHandle<SwapContext>>>();

            let results = runtime.block_on(future::join_all(first_batch_futures));
            for result in results {
                let result = result.unwrap();
                man.persist_swap(result);
            }

            let manager = Arc::new(man.clone());
            let second_batch_futures = ((start_level + 1)..man.level2nodes.len() - 2)
                .step_by(2)
                .into_iter()
                .map(|level| {
                    let man = manager.clone();
                    runtime.spawn_blocking(move || {
                        let a = man.var_at_level(level).unwrap();
                        let b = man.var_at_level(level + 1).unwrap();
                        let swap_context =
                            DDManager::partial_swap(&man, a, b, SwapContext::default());
                        let swap_context = DDManager::partial_swap(&man, a, b, swap_context.1);
                        swap_context.1
                    })
                })
                .collect::<Vec<JoinHandle<SwapContext>>>();

            let results = runtime.block_on(future::join_all(second_batch_futures));
            for result in results {
                let result = result.unwrap();
                man.persist_swap(result);
            }
        }
        println!("Partial swap async took {:?}", start.elapsed() / N);

        // partial swap
        let start = std::time::Instant::now();
        for _ in 0..N {
            let mut man = man.clone();

            let manager = Arc::new(man.clone());
            let results = (start_level..man.level2nodes.len() - 2)
                .step_by(2)
                .collect::<Vec<usize>>()
                .into_par_iter()
                .map(|level| {
                    let man = manager.clone();

                    let a = man.var_at_level(level).unwrap();
                    let b = man.var_at_level(level + 1).unwrap();
                    let swap_context = DDManager::partial_swap(&man, a, b, SwapContext::default());
                    let swap_context = DDManager::partial_swap(&man, a, b, swap_context.1);
                    swap_context.1
                })
                .collect::<Vec<SwapContext>>();

            for result in results {
                man.persist_swap(result);
            }

            let manager = Arc::new(man.clone());
            let results = ((start_level + 1)..man.level2nodes.len() - 2)
                .step_by(2)
                .collect::<Vec<usize>>()
                .into_par_iter()
                .map(|level| {
                    let man = manager.clone();

                    let a = man.var_at_level(level).unwrap();
                    let b = man.var_at_level(level + 1).unwrap();
                    let swap_context = DDManager::partial_swap(&man, a, b, SwapContext::default());
                    let swap_context = DDManager::partial_swap(&man, a, b, swap_context.1);
                    swap_context.1
                })
                .collect::<Vec<SwapContext>>();

            for result in results {
                man.persist_swap(result);
            }
        }
        println!("Partial swap async 2 took {:?}", start.elapsed() / N);
    }

    #[tokio::test]
    async fn swap_eval_top_to_bottom() {
        // Build BDD
        let (mut man, nodes) = DDManager::load_from_dddmp_file(PATH.to_string()).unwrap();
        let bdd = nodes[0];
        let start_level = man.var2level[man.nodes.get(&bdd).unwrap().var.0] + 1;

        man.purge_retain(bdd);

        // // Build BDD
        // let mut instance = dimacs::parse_dimacs(
        //     &fs::read_to_string("examples/berkeleydb.dimacs").expect("Failed to read dimacs file."),
        // )
        // .expect("Failed to parse dimacs file.");
        // let (mut man, bdd) =
        //     DDManager::from_instance(&mut instance, None, Default::default()).unwrap();
        // man.purge_retain(bdd);

        println!("Swap top to bottom");

        // swap count
        let mut count = 0;
        for _ in 0..N {
            for _ in start_level..man.level2nodes.len() - 2 {
                count += 1;
            }
        }
        println!("swap count: {}", count / N);

        // regular swap
        let start = std::time::Instant::now();
        for _ in 0..N {
            let mut man = man.clone();
            let mut bdd = bdd.clone();

            for level in start_level..man.level2nodes.len() - 2 {
                let a = man.var_at_level(level);
                let b = man.var_at_level(level + 1);
                if let (Some(a), Some(b)) = (a, b) {
                    bdd = man.swap(a, b, bdd);
                }
            }
        }
        println!("Regular swap took {:?}", start.elapsed() / N);

        // partial swap
        let start = std::time::Instant::now();
        for _ in 0..N {
            let mut man = man.clone();
            for level in start_level..man.level2nodes.len() - 2 {
                let a = man.var_at_level(level);
                let b = man.var_at_level(level + 1);
                if let (Some(a), Some(b)) = (a, b) {
                    let swap_context = DDManager::partial_swap(&man, a, b, SwapContext::default());
                    man.persist_swap(swap_context.1);
                }
            }
        }
        println!(
            "Partial swap instant resolve took {:?}",
            start.elapsed() / N
        );

        // partial swap
        let start = std::time::Instant::now();
        for _ in 0..N {
            let mut man = man.clone();
            let mut swap_context = (0, SwapContext::default());
            for level in start_level..man.level2nodes.len() - 2 {
                let a = swap_context.1.var_at_level(level, &man.var2level);
                let b = swap_context.1.var_at_level(level + 1, &man.var2level);

                if let (Some(a), Some(b)) = (a, b) {
                    swap_context = DDManager::partial_swap(&man, a, b, swap_context.1);
                }
            }
            man.persist_swap(swap_context.1);
        }
        println!("Partial swap took {:?}", start.elapsed() / N);
    }

    #[test]
    fn swap_eval_window() {
        // Build BDD
        let (mut man, nodes) = DDManager::load_from_dddmp_file(PATH.to_string()).unwrap();
        let bdd = nodes[0];
        let start_level = man.var2level[man.nodes.get(&bdd).unwrap().var.0] + 1;

        man.purge_retain(bdd);

        let l2n: Vec<usize> = man
            .level2nodes
            .clone()
            .into_iter()
            .map(|level| level.len())
            .collect();

        // let threshold = l2n.iter().max().unwrap() / 3;
        let threshold = median(&l2n);
        let ranges = ThresholdMethod::default().generate_area(
            l2n,
            Some(4),
            Some(threshold),
            Some(start_level),
        );
        let window_swaps = ranges
            .into_iter()
            .map(|(a, b)| gen_permutation(a, b))
            .collect::<Vec<Vec<(usize, usize)>>>();

        println!("Swap window Permutation");

        let count: usize = window_swaps.clone().into_iter().map(|x| x.len()).sum();
        println!("swap count: {}", count);

        // regular swap
        let start = std::time::Instant::now();
        for _ in 0..N {
            let mut man = man.clone();
            let mut bdd = bdd.clone();

            for window in window_swaps.clone() {
                for (from, to) in window {
                    let a = man.var_at_level(from).unwrap();
                    let b = man.var_at_level(to).unwrap();
                    bdd = man.swap(a, b, bdd);
                }
            }
        }
        println!("Regular swap took {:?}", start.elapsed() / N);

        // partial swap
        let start = std::time::Instant::now();
        // Create the runtime
        // let runtime = Runtime::new().unwrap();
        for _ in 0..N {
            let manager = Arc::new(man.clone());

            let runtime = Runtime::new().unwrap();

            let futures = window_swaps
                .clone()
                .into_iter()
                .map(|window| {
                    let man = manager.clone();
                    runtime.spawn_blocking(move || {
                        let mut swap_context = (0, SwapContext::default());
                        for (from, to) in window {
                            let a = swap_context.1.var_at_level(from, &man.var2level).unwrap();
                            let b = swap_context.1.var_at_level(to, &man.var2level).unwrap();

                            swap_context =
                                DDManager::partial_swap(&man.clone(), a, b, swap_context.1);
                        }
                        swap_context.1
                    })
                })
                .collect::<Vec<JoinHandle<SwapContext>>>();

            let results = runtime.block_on(future::join_all(futures));
            for result in results {
                let result = result.unwrap();
                man.persist_swap(result);
            }
        }
        println!("Partial swap async took {:?}", start.elapsed() / N);

        // partial swap
        let start = std::time::Instant::now();
        // Create the runtime
        // let runtime = Runtime::new().unwrap();
        for _ in 0..N {
            let manager = Arc::new(man.clone());

            let results = window_swaps
                .clone()
                .into_par_iter()
                .map(|window| {
                    let mut swap_context = (0, SwapContext::default());
                    let man = manager.clone();
                    for (from, to) in window {
                        let a = swap_context.1.var_at_level(from, &man.var2level).unwrap();
                        let b = swap_context.1.var_at_level(to, &man.var2level).unwrap();

                        swap_context = DDManager::partial_swap(&man.clone(), a, b, swap_context.1);
                    }
                    swap_context.1
                })
                .collect::<Vec<SwapContext>>();

            for result in results {
                man.persist_swap(result);
            }
        }
        println!("Partial swap async 2 took {:?}", start.elapsed() / N);

        // partial swap
        let start = std::time::Instant::now();
        for _ in 0..N {
            let mut man = man.clone();

            for window in window_swaps.clone() {
                for (from, to) in window {
                    let a = man.var_at_level(from).unwrap();
                    let b = man.var_at_level(to).unwrap();

                    let swap_context = DDManager::partial_swap(&man, a, b, SwapContext::default());
                    man.persist_swap(swap_context.1);
                }
            }
        }
        println!(
            "Partial swap instant resolve took {:?}",
            start.elapsed() / N
        );

        // partial swap
        let start = std::time::Instant::now();
        for _ in 0..N {
            let mut man = man.clone();

            for window in window_swaps.clone() {
                let mut swap_context = (0, SwapContext::default());
                for (from, to) in window {
                    let a = swap_context.1.var_at_level(from, &man.var2level).unwrap();
                    let b = swap_context.1.var_at_level(to, &man.var2level).unwrap();

                    swap_context = DDManager::partial_swap(&man, a, b, swap_context.1);
                }
                man.persist_swap(swap_context.1);
            }
        }
        println!("Partial swap took {:?}", start.elapsed() / N);
    }
}

#[cfg(test)]
mod evaluation {
    use std::{fs, io::Write, sync::Arc, time::Instant};

    use futures::future;
    use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
    use tokio::{runtime::Runtime, task::JoinHandle};

    use crate::core::{
        bdd_manager::DDManager,
        dvo::{
            area_generation::{
                merge_ranges, AreaSelection, EqualSplitMethod, HotspotMethod, NSplitMethod,
                ThresholdMethod,
            },
            dvo_strategies::{
                gen_permutation, median, nth_percentile, ConcurrentDVO, ConcurrentDVOStrategie,
                Sifting, SiftingTwo,
            },
        },
        swap::SwapContext,
    };

    // static PATH: &str = "examples/berkeleydb.dimacs.dddmp";
    // static PATH: &str = "examples/financialServices01.dimacs.dddmp";
    // static PATH: &str = "examples/automotive02v4.dimacs.dddmp";
    // static PATH: &str = "examples/automotive01.dimacs.dddmp";

    static MODELS: [&str; 3] = [
        "examples/berkeleydb.dimacs.dddmp",
        "examples/financialServices01.dimacs.dddmp",
        // "examples/automotive02v4.dimacs.dddmp",
        "examples/automotive01.dimacs.dddmp",
    ];
    // static MODELS: [&str; 4] = [
    //     "examples/berkeleydb.dimacs.dddmp",
    //     "examples/financialServices01.dimacs.dddmp",
    //     "examples/automotive02v4.dimacs.dddmp",
    //     "examples/automotive01.dimacs.dddmp",
    // ];

    static N: usize = 1;

    fn run_dvo_on(ranges: Vec<(usize, usize)>, manager: &mut DDManager) {
        let man = Arc::new(manager.clone());
        let runtime = Runtime::new().unwrap();

        let max_increase = manager
            .level2nodes
            .iter()
            .map(|level| level.len())
            .max()
            .unwrap()
            / 2;
        println!("Max increase: {}", max_increase);
        // let ranges = vec![ranges[9]];

        let futures = ranges
            .iter()
            .map(|(start, end)| {
                let start = *start;
                let end = *end;
                let man = man.clone();
                let mut swap_context = SwapContext::new();
                swap_context.precalc_references(&man.clone(), start, end);
                runtime.spawn_blocking(move || {
                    SiftingTwo::default().compute_concurrent_dvo(
                        man,
                        Some(max_increase),
                        start..=end,
                        swap_context,
                    )
                })
            })
            .collect::<Vec<JoinHandle<SwapContext>>>();

        let results = runtime.block_on(future::join_all(futures));
        for result in results {
            let result = result.unwrap();
            manager.persist_swap(result);
        }
    }

    #[test]
    fn top_to_bottom() {
        println!("top_to_bottom. Concurrent vs. Regular");
        for model in MODELS.iter() {
            println!("Model: {}", model);
            let (mut man, nodes) = DDManager::load_from_dddmp_file(model.to_string()).unwrap();
            let bdd = nodes[0];
            man.purge_retain(bdd);

            let start_level = man.var2level[man.nodes.get(&bdd).unwrap().var.0] + 2;
            let end_level = man.level2nodes.len() - 2;

            let start = Instant::now();
            let mut counter: usize = 0;
            for _ in 0..N {
                let mut man = man.clone();
                let mut bdd = bdd.clone();
                for level in start_level..end_level {
                    let from = man.var_at_level(level).unwrap();
                    let to = man.var_at_level(level + 1).unwrap();
                    counter += 1;
                    bdd = man.swap(from, to, bdd);
                }
            }
            println!("Regular Time: {:?}", start.elapsed() / N as u32);
            println!(
                "Regular swap Time: {:?}",
                (start.elapsed() / N as u32) / counter as u32
            );

            let start = Instant::now();
            let mut counter: usize = 0;
            for _ in 0..N {
                let mut man = man.clone();
                let bdd = bdd.clone();
                for level in start_level..end_level {
                    let from = man.var_at_level(level).unwrap();
                    let to = man.var_at_level(level + 1).unwrap();
                    counter += 1;
                    let _ = man.direct_swap(from, to, bdd);
                }
            }
            println!("Direct Time: {:?}", start.elapsed() / N as u32);
            println!(
                "Direct swap Time: {:?}",
                (start.elapsed() / N as u32) / counter as u32
            );
        }
    }
    #[test]
    fn context() {
        println!("context. Concurrent vs. direct");
        let (mut man, nodes) = DDManager::load_from_dddmp_file(
            "examples/financialServices01.dimacs.dddmp".to_string(),
        )
        .unwrap();
        let bdd = nodes[0];
        man.purge_retain(bdd);
        let start_level = man.var2level[man.nodes.get(&bdd).unwrap().var.0];

        println!("Scenario 1:");
        let ranges = EqualSplitMethod::default().generate_area(
            man.calculate_node_count(),
            Some(4),
            None,
            Some(start_level),
        );

        let start = Instant::now();
        for _ in 0..N {
            let mut man = man.clone();

            let results = ranges
                .clone()
                .par_iter()
                .map(|(start, end)| {
                    let mut swap_context = SwapContext::new();
                    swap_context.precalc_references(&man, *start, *end);
                    let mut result = (0, swap_context);
                    for (from, to) in gen_permutation(*start, *end) {
                        let a = result.1.var_at_level(from, &man.var2level).unwrap();
                        let b = result.1.var_at_level(to, &man.var2level).unwrap();
                        result = man.partial_swap(a, b, result.1);
                    }
                    result.1
                })
                .collect::<Vec<SwapContext>>();

            for result in results {
                man.persist_swap(result);
            }
        }
        println!("Asynchron Time: {:?}", start.elapsed() / N as u32);

        // return ();

        let start = Instant::now();
        for _ in 0..N {
            let mut man = man.clone();
            let bdd = bdd.clone();
            for (start_level, end_level) in ranges.clone() {
                for (from, to) in gen_permutation(start_level, end_level) {
                    let a = man.var_at_level(from).unwrap();
                    let b = man.var_at_level(to).unwrap();
                    let _ = man.direct_swap(a, b, bdd);
                }
            }
        }
        println!("Direct Time: {:?}", start.elapsed() / N as u32);

        let start = Instant::now();
        for _ in 0..N {
            let mut man = man.clone();
            for (start_level, end_level) in ranges.clone() {
                let mut result = (0, SwapContext::new());

                for (from, to) in gen_permutation(start_level, end_level) {
                    let a = result.1.var_at_level(from, &man.var2level).unwrap();
                    let b = result.1.var_at_level(to, &man.var2level).unwrap();

                    result = man.partial_swap(a, b, result.1);
                }
                man.persist_swap(result.1);
            }
        }
        println!("Context Time: {:?}", start.elapsed() / N as u32);

        let start = Instant::now();
        for _ in 0..N {
            let mut man = man.clone();
            for (start_level, end_level) in ranges.clone() {
                let mut swap_context = SwapContext::new();
                swap_context.precalc_references(&man, start_level, end_level);
                let mut result = (0, swap_context);

                for (from, to) in gen_permutation(start_level, end_level) {
                    let a = result.1.var_at_level(from, &man.var2level).unwrap();
                    let b = result.1.var_at_level(to, &man.var2level).unwrap();

                    result = man.partial_swap(a, b, result.1);
                }
                man.persist_swap(result.1);
            }
        }
        println!(
            "Context with precalculation Time: {:?}",
            start.elapsed() / N as u32
        );

        println!("Scenario 2:");
        let start_level = man.var2level[man.nodes.get(&bdd).unwrap().var.0] + 2;
        let end_level = man.level2nodes.len() - 2;

        let start = Instant::now();
        // let mut counter: usize = 0;
        for _ in 0..N {
            let mut man = man.clone();
            let mut context = (0, SwapContext::new());
            for level in start_level..end_level {
                let from = context.1.var_at_level(level, &man.var2level).unwrap();
                let to = context.1.var_at_level(level + 1, &man.var2level).unwrap();
                // counter += 1;
                context = man.partial_swap(from, to, context.1);
            }
            man.persist_swap(context.1);
        }
        println!("Context Time: {:?}", start.elapsed() / N as u32);
        // println!(
        //     "Regular swap Time: {:?}",
        //     (start.elapsed() / N as u32) / counter as u32
        // );

        let start = Instant::now();
        // let mut counter: usize = 0;
        for _ in 0..N {
            let mut man = man.clone();
            let bdd = bdd.clone();
            for level in start_level..end_level {
                let from = man.var_at_level(level).unwrap();
                let to = man.var_at_level(level + 1).unwrap();
                // counter += 1;
                let _ = man.direct_swap(from, to, bdd);
            }
        }
        println!("Direct Time: {:?}", start.elapsed() / N as u32);
        // println!(
        //     "Direct swap Time: {:?}",
        //     (start.elapsed() / N as u32) / counter as u32
        // );
    }

    #[test]
    fn concurrent() {
        println!("context. Concurrent vs. direct");
        let (mut man, nodes) = DDManager::load_from_dddmp_file(
            "examples/financialServices01.dimacs.dddmp".to_string(),
        )
        .unwrap();
        let bdd = nodes[0];
        man.purge_retain(bdd);
        let start_level = man.var2level[man.nodes.get(&bdd).unwrap().var.0];

        println!("Scenario 1:");
        let ranges = EqualSplitMethod::default().generate_area(
            man.calculate_node_count(),
            Some(4),
            None,
            Some(start_level),
        );

        let start = Instant::now();
        for _ in 0..N {
            let mut man = man.clone();

            let results = ranges
                .clone()
                .iter()
                .map(|(start, end)| {
                    let swap_context = SwapContext::new();
                    // swap_context.precalc_references(&man, *start, *end);
                    let mut result = (0, swap_context);
                    for (from, to) in gen_permutation(*start, *end) {
                        let a = result.1.var_at_level(from, &man.var2level).unwrap();
                        let b = result.1.var_at_level(to, &man.var2level).unwrap();
                        result = man.partial_swap(a, b, result.1);
                    }
                    result.1
                })
                .collect::<Vec<SwapContext>>();

            for result in results {
                man.persist_swap(result);
            }
        }
        println!("Synchron Time: {:?}", start.elapsed() / N as u32);

        let start = Instant::now();
        for _ in 0..N {
            let mut man = man.clone();

            let results = ranges
                .clone()
                .iter()
                .map(|(start, end)| {
                    let mut swap_context = SwapContext::new();
                    swap_context.precalc_references(&man, *start, *end);
                    let mut result = (0, swap_context);
                    for (from, to) in gen_permutation(*start, *end) {
                        let a = result.1.var_at_level(from, &man.var2level).unwrap();
                        let b = result.1.var_at_level(to, &man.var2level).unwrap();
                        result = man.partial_swap(a, b, result.1);
                    }
                    result.1
                })
                .collect::<Vec<SwapContext>>();

            for result in results {
                man.persist_swap(result);
            }
        }
        println!(
            "Synchron precalculation Time: {:?}",
            start.elapsed() / N as u32
        );

        let start = Instant::now();
        for _ in 0..N {
            let mut man = man.clone();

            let results = ranges
                .clone()
                .par_iter()
                .map(|(start, end)| {
                    let swap_context = SwapContext::new();
                    // swap_context.precalc_references(&man, *start, *end);
                    let mut result = (0, swap_context);
                    for (from, to) in gen_permutation(*start, *end) {
                        let a = result.1.var_at_level(from, &man.var2level).unwrap();
                        let b = result.1.var_at_level(to, &man.var2level).unwrap();
                        result = man.partial_swap(a, b, result.1);
                    }
                    result.1
                })
                .collect::<Vec<SwapContext>>();

            for result in results {
                man.persist_swap(result);
            }
        }
        println!("Asynchron Time: {:?}", start.elapsed() / N as u32);

        let start = Instant::now();
        for _ in 0..N {
            let mut man = man.clone();

            let results = ranges
                .clone()
                .par_iter()
                .map(|(start, end)| {
                    let mut swap_context = SwapContext::new();
                    swap_context.precalc_references(&man, *start, *end);
                    let mut result = (0, swap_context);
                    for (from, to) in gen_permutation(*start, *end) {
                        let a = result.1.var_at_level(from, &man.var2level).unwrap();
                        let b = result.1.var_at_level(to, &man.var2level).unwrap();
                        result = man.partial_swap(a, b, result.1);
                    }
                    result.1
                })
                .collect::<Vec<SwapContext>>();

            for result in results {
                man.persist_swap(result);
            }
        }
        println!(
            "Asynchron precalculation Time: {:?}",
            start.elapsed() / N as u32
        );
    }
}
