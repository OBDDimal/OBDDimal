use std::hash::{Hash, Hasher};

#[derive(Debug, Copy, Clone)]
pub struct DDNode {
    pub id: u32,
    pub var: u32,
    pub low: u32,
    pub high: u32,
    pub misc: u32,
}

impl PartialEq for DDNode {
    fn eq(&self, that: &Self) -> bool {
        self.var == that.var && self.low == that.low && self.high == that.high
    }
}

impl Eq for DDNode {}

impl DDNode {
    pub fn restrict(&self, top: u32, order: &[u32], val: bool) -> u32 {
        if self.var == 0 {
            return self.id;
        }

        if order[top as usize] < order[self.var as usize] {
            return self.id;
        }

        if top == self.var {
            if val {
                return self.high;
            } else {
                return self.low;
            }
        }

        panic!("Should not be possible");
    }
}

impl Hash for DDNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.var.hash(state);
        self.low.hash(state);
        self.high.hash(state);
    }
}
