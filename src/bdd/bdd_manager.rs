use crate::bdd::bdd_graph::*;
use crate::input::boolean_function::*;
use std::collections::HashMap;

#[derive(Debug)]
pub struct BDDManager {
    unique_table: HashMap<(i64, NodeType, NodeType), NodeType>,
    computed_table: HashMap<(NodeType, NodeType, NodeType), NodeType>,
}

impl BDDManager {
    /// Creates a new instance of a BDD manager.
    pub fn new() -> Self {
        BDDManager {
            unique_table: HashMap::new(),
            computed_table: HashMap::new(),
        }
    }

    fn add_node_to_unique(&mut self, var: i64, low: NodeType, high: NodeType) -> NodeType {
        let low_c = low.clone(); // Performance not so good because of cloning.
        let high_c = high.clone(); // Performance not so good because of cloning.

        self.unique_table
            .entry((var, low, high))
            .or_insert(NodeType::COMPLEX(Node {
                top_var: var,
                low: Box::new(low_c),
                high: Box::new(high_c),
            }))
            .clone() // Performance not so good because of cloning.
    }

    fn restrict(&mut self, subtree: NodeType, var: i64, val: bool) -> NodeType {
        let st = subtree.clone();
        match subtree {
            NodeType::ZERO => subtree,
            NodeType::ONE => subtree,
            NodeType::COMPLEX(node) => {
                if node.top_var > var {
                    return st;
                }
                if node.top_var < var {
                    let srh = self.restrict(*node.high, var, val);
                    let srl = self.restrict(*node.low, var, val);
                    self.add_node_to_unique(node.top_var, srl, srh)
                } else {
                    if val {
                        self.restrict(*node.high, var, val)
                    } else {
                        self.restrict(*node.low, var, val)
                    }
                }
            }
        }
    }

    // Broken
    pub fn satcount(&mut self, subtree: NodeType) -> i64 {
        let st = subtree.clone();
        match subtree {
            NodeType::ZERO => 0,
            NodeType::ONE => 1,
            NodeType::COMPLEX(n) => {
                2_i64.pow((n.top_var - 1) as u32 * self.satcount_rec(st) as u32)
            }
        }
    }

    fn satcount_rec(&mut self, subtree: NodeType) -> i64 {
        match subtree {
            NodeType::ZERO => 0,
            NodeType::ONE => 1,
            NodeType::COMPLEX(n) => {
                let sub_low = *n.low;
                let sub_low2 = sub_low.clone();
                let sub_high = *n.high;
                let sub_high2 = sub_high.clone();
                let mut size = 0;

                let s = match sub_low {
                    NodeType::ZERO => 0,
                    NodeType::ONE => 1,
                    NodeType::COMPLEX(ln) => 2_i64.pow((ln.top_var - n.top_var - 1) as u32),
                };

                size += s * self.satcount_rec(sub_low2);

                let s = match sub_high {
                    NodeType::ZERO => 0,
                    NodeType::ONE => 1,
                    NodeType::COMPLEX(hn) => 2_i64.pow((hn.top_var - n.top_var - 1) as u32),
                };

                size += s * self.satcount_rec(sub_high2);

                size
            }
        }
    }

    /// Returns true if there is a variable assignment which evaluates the given formula to `true`.
    pub fn satisfiable(&mut self, subtree: NodeType) -> bool {
        match subtree {
            NodeType::ZERO => false,
            NodeType::ONE => true,
            NodeType::COMPLEX(n) => {
                let s_left = self.satisfiable(*n.low);
                let s_right = self.satisfiable(*n.high);
                s_left | s_right
            }
        }
    }

    fn ite(&mut self, f: NodeType, g: NodeType, h: NodeType) -> NodeType {
        match (f, g, h) {
            (NodeType::ZERO, _, e) => e,
            (NodeType::ONE, t, _) => t,
            (f, NodeType::ONE, NodeType::ZERO) => f,
            (i, t, e) => {
                // This is dumb! Do not try this at home! Should be replaced by references.
                let i_c = i.clone(); // Performance not so good because of cloning.
                let t_c = t.clone(); // Performance not so good because of cloning.
                let e_c = e.clone(); // Performance not so good because of cloning.
                let i_c2 = i.clone(); // Performance not so good because of cloning.
                let t_c2 = t.clone(); // Performance not so good because of cloning.
                let e_c2 = e.clone(); // Performance not so good because of cloning.
                let i_c3 = i.clone(); // Performance not so good because of cloning.
                let t_c3 = t.clone(); // Performance not so good because of cloning.
                let e_c3 = e.clone(); // Performance not so good because of cloning.
                let i_c4 = i.clone(); // Performance not so good because of cloning.
                let t_c4 = t.clone(); // Performance not so good because of cloning.
                let e_c4 = e.clone(); // Performance not so good because of cloning.

                if self.computed_table.contains_key(&(i, t, e)) {
                    self.computed_table[&(i_c, t_c, e_c)].clone() // Performance not so good because of cloning.
                } else {
                    let v = match (i_c2, t_c2, e_c2) {
                        (
                            NodeType::COMPLEX(i_n),
                            NodeType::COMPLEX(t_n),
                            NodeType::COMPLEX(e_n),
                        ) => i_n.top_var.min(t_n.top_var).min(e_n.top_var),
                        (NodeType::COMPLEX(i_n), _, NodeType::COMPLEX(e_n)) => {
                            i_n.top_var.min(e_n.top_var)
                        }
                        (NodeType::COMPLEX(i_n), NodeType::COMPLEX(t_n), _) => {
                            i_n.top_var.min(t_n.top_var)
                        }
                        (_, NodeType::COMPLEX(t_n), NodeType::COMPLEX(e_n)) => {
                            e_n.top_var.min(t_n.top_var)
                        }
                        (NodeType::COMPLEX(i_n), _, _) => i_n.top_var,
                        (_, NodeType::COMPLEX(t_n), _) => t_n.top_var,
                        (_, _, NodeType::COMPLEX(e_n)) => e_n.top_var,
                        (_, _, _) => panic!("There was no assignment for v."),
                    };

                    let ixt = self.restrict(i_c3, v, true);
                    let txt = self.restrict(t_c3, v, true);
                    let ext = self.restrict(e_c3, v, true);

                    let tv = self.ite(ixt, txt, ext);

                    let ixf = self.restrict(i_c4, v, false);
                    let txf = self.restrict(t_c4, v, false);
                    let exf = self.restrict(e_c4, v, false);

                    let ev = self.ite(ixf, txf, exf);

                    if tv == ev {
                        return tv;
                    }

                    self.add_node_to_unique(v, ev, tv)
                }
            }
        }
    }

    pub fn and(&mut self, lhs: NodeType, rhs: NodeType) -> NodeType {
        self.ite(lhs, rhs, NodeType::ZERO)
    }

    pub fn or(&mut self, lhs: NodeType, rhs: NodeType) -> NodeType {
        self.ite(lhs, NodeType::ONE, rhs)
    }

    pub fn not(&mut self, val: NodeType) -> NodeType {
        self.ite(val, NodeType::ZERO, NodeType::ONE)
    }
}
