use std::hash::{Hash, Hasher};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeID(pub u32);

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct VarID(pub u32);

#[derive(Debug, Copy, Clone)]
pub struct DDNode {
    /// Node ID. Special values: 0 and 1 for terminal nodes
    pub id: NodeID,
    /// Variable number. Special variable 0 == terminal nodes
    pub var: VarID,
    pub low: NodeID,
    pub high: NodeID,
}

impl PartialEq for DDNode {
    fn eq(&self, that: &Self) -> bool {
        self.var == that.var && self.low == that.low && self.high == that.high
    }
}

impl Eq for DDNode {}

impl DDNode {
    pub fn restrict(&self, top: VarID, order: &[VarID], val: bool) -> NodeID {
        if self.var == VarID(0) {
            return self.id;
        }

        if order[top.0 as usize] < order[self.var.0 as usize] {
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

impl Hash for DDNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.var.hash(state);
        self.low.hash(state);
        self.high.hash(state);
    }
}
