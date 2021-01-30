use crate::bdd::bdd_graph::*;
use crate::input::boolean_function::*;
use std::collections::HashMap;

#[derive(Debug)]
pub struct BDDManager {
    unique_table: HashMap<(i64, NodeType, NodeType), NodeType>,
    computed_table: HashMap<(NodeType, NodeType, NodeType), NodeType>,
    pub bdd: NodeType,
}

impl BDDManager {
    /// Creates a new instance of a BDD manager.
    pub fn new() -> Self {
        Self {
            unique_table: HashMap::new(),
            computed_table: HashMap::new(),
            bdd: NodeType::Zero,
        }
    }

    /// Creates a new instance of a BDD manager from a given CNF.
    pub fn from_cnf(cnf: Symbol) -> Self {
        let mut mgr = Self::new();
        mgr.bdd = mgr.from_cnf_rec(cnf);
        mgr
    }

    fn from_cnf_rec(&mut self, cnf: Symbol) -> NodeType {
        match cnf {
            Symbol::Posterminal(i) => Node::new(i as i64, NodeType::Zero, NodeType::One),
            Symbol::Negterminal(i) => Node::new(i as i64, NodeType::One, NodeType::Zero),
            Symbol::Function(func) => match func.op {
                Operator::And => {
                    let l = self.from_cnf_rec(*func.lhs);
                    let r = self.from_cnf_rec(*func.rhs);
                    self.and(l, r)
                }
                Operator::Or => {
                    let l = self.from_cnf_rec(*func.lhs);
                    let r = self.from_cnf_rec(*func.rhs);
                    self.or(l, r)
                }
            },
        }
    }

    fn add_node_to_unique(&mut self, var: i64, low: NodeType, high: NodeType) -> NodeType {
        let low_c = low.clone(); // Performance not so good because of cloning.
        let high_c = high.clone(); // Performance not so good because of cloning.

        self.unique_table
            .entry((var, low, high))
            .or_insert(Node::new(var, low_c, high_c))
            .clone() // Performance not so good because of cloning.
    }

    fn restrict(&mut self, subtree: NodeType, var: i64, val: bool) -> NodeType {
        let st = subtree.clone();
        match subtree {
            NodeType::Zero => subtree,
            NodeType::One => subtree,
            NodeType::Complex(node) => {
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
    pub fn satcount(&mut self) -> i64 {
        let st = self.bdd.clone();
        let st2 = self.bdd.clone();
        match st {
            NodeType::Zero => 0,
            NodeType::One => 1,
            NodeType::Complex(n) => {
                2_i64.pow((n.top_var - 1) as u32 * self.satcount_rec(st2) as u32)
            }
        }
    }

    fn satcount_rec(&mut self, subtree: NodeType) -> i64 {
        match subtree {
            NodeType::Zero => 0,
            NodeType::One => 1,
            NodeType::Complex(n) => {
                let sub_low = *n.low;
                let sub_low2 = sub_low.clone();
                let sub_high = *n.high;
                let sub_high2 = sub_high.clone();
                let mut size = 0;

                let s = match sub_low {
                    NodeType::Zero => 0,
                    NodeType::One => 1,
                    NodeType::Complex(ln) => 2_i64.pow((ln.top_var - n.top_var - 1) as u32),
                };

                size += s * self.satcount_rec(sub_low2);

                let s = match sub_high {
                    NodeType::Zero => 0,
                    NodeType::One => 1,
                    NodeType::Complex(hn) => 2_i64.pow((hn.top_var - n.top_var - 1) as u32),
                };

                size += s * self.satcount_rec(sub_high2);

                size
            }
        }
    }

    /// Returns true if there is a variable assignment which evaluates the given formula to `true`.
    pub fn satisfiable(&self) -> bool {
        self.bdd != NodeType::Zero
    }

    fn ite(&mut self, f: NodeType, g: NodeType, h: NodeType) -> NodeType {
        match (f, g, h) {
            (NodeType::Zero, _, e) => e,
            (NodeType::One, t, _) => t,
            (f, NodeType::One, NodeType::Zero) => f,
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
                            NodeType::Complex(i_n),
                            NodeType::Complex(t_n),
                            NodeType::Complex(e_n),
                        ) => i_n.top_var.min(t_n.top_var).min(e_n.top_var),
                        (NodeType::Complex(i_n), _, NodeType::Complex(e_n)) => {
                            i_n.top_var.min(e_n.top_var)
                        }
                        (NodeType::Complex(i_n), NodeType::Complex(t_n), _) => {
                            i_n.top_var.min(t_n.top_var)
                        }
                        (_, NodeType::Complex(t_n), NodeType::Complex(e_n)) => {
                            e_n.top_var.min(t_n.top_var)
                        }
                        (NodeType::Complex(i_n), _, _) => i_n.top_var,
                        (_, NodeType::Complex(t_n), _) => t_n.top_var,
                        (_, _, NodeType::Complex(e_n)) => e_n.top_var,
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
        self.ite(lhs, rhs, NodeType::Zero)
    }

    pub fn or(&mut self, lhs: NodeType, rhs: NodeType) -> NodeType {
        self.ite(lhs, NodeType::One, rhs)
    }

    pub fn not(&mut self, val: NodeType) -> NodeType {
        self.ite(val, NodeType::Zero, NodeType::One)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bdd::bdd_graph::NodeType::*;

    fn build_bdd(path: &str) -> BDDManager {
        let input = crate::input::parser::parse_string(&std::fs::read_to_string(path).unwrap()).unwrap();
        let input_symbols = BooleanFunction::new_cnf_formula(input);
        BDDManager::from_cnf(input_symbols)
    }

    #[test]
    fn easy1_structural() {
        let mgr = build_bdd("examples/easy1.dimacs");

        assert_eq!(
            mgr.bdd,
            Complex(Node {
                top_var: 1,
                low: Box::new(Complex(Node {
                    top_var: 3,
                    low: Box::new(One),
                    high: Box::new(Zero),
                })),
                high: Box::new(Complex(Node {
                    top_var: 2,
                    low: Box::new(Complex(Node {
                        top_var: 3,
                        low: Box::new(Zero),
                        high: Box::new(One),
                    })),
                    high: Box::new(One),
                })),
            })
        );
    }

    #[test]
    fn easyns_structural() {
        let mgr = build_bdd("examples/easyns.dimacs");
        assert_eq!(mgr.bdd, NodeType::Zero);
    }

    #[test]
    fn ns_structural() {
        let mgr = build_bdd("examples/ns.dimacs");
        assert_eq!(mgr.bdd, NodeType::Zero);
    }

    #[test]
    fn sandwich_satisfiable() {
        let mgr = build_bdd("examples/sandwich.dimacs");
        assert!(mgr.satisfiable());
    }

    //#[test]
    fn berkeleydb_satisfiable() {
        let mgr = build_bdd("examples/berkeleydb.dimacs");
        assert!(mgr.satisfiable());
    }
}
