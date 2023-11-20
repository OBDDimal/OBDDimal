//! Module containing type definitions for the elements of the BDD:
//! Nodes ([DDNode], [NodeID]) and Variables ([VarID])

use std::hash::{Hash, Hasher};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeID(pub usize);

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct VarID(pub usize);

/// Element of a BDD.
/// Note that the Node contains its own ID. This may be set to zero until it has been assigned,
/// and most importantly is not considered in hashing and equality testing.
#[derive(Debug, Copy, Clone)]
pub struct DDNode {
    /// Node ID. Special values: 0 and 1 for terminal nodes
    pub id: NodeID,
    /// Variable number. Special variable 0 == terminal nodes
    pub var: VarID,
    pub low: NodeID,
    pub high: NodeID,
}

/// Test equality of two nodes, not considering the ID!
impl PartialEq for DDNode {
    fn eq(&self, that: &Self) -> bool {
        self.var == that.var && self.low == that.low && self.high == that.high
    }
}

impl Eq for DDNode {}

impl DDNode {
    /// Returns the function resulting when setting the specified variable to the specified value.
    /// Note that this only implements the case of the node being at the exact level of the specified
    /// variable.
    pub fn restrict(&self, top: VarID, var2level: &[usize], val: bool) -> NodeID {
        if self.var == VarID(0) {
            return self.id;
        }

        if var2level[top.0] < var2level[self.var.0] {
            // Variable does not occur in current function
            return self.id;
        }

        if top == self.var {
            if val {
                return self.high;
            } else {
                return self.low;
            }
        }

        // Variable occurs further down in the function. This is not supported in this restrict().
        panic!("Restrict called with variable below current node");
    }
}

/// Hash a node, not considering the ID!
impl Hash for DDNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.var.hash(state);
        self.low.hash(state);
        self.high.hash(state);
    }
}
