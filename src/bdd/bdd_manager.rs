use crate::bdd::bdd_graph::*;
use crate::input::boolean_function::*;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Hash, Debug, Clone, Eq, PartialEq)]
struct UniqueKey {
    tv: i64,
    low: Rc<NodeType>,
    high: Rc<NodeType>,
}

impl UniqueKey {
    fn new(tv: i64, low: Rc<NodeType>, high: Rc<NodeType>) -> Self {
        Self { tv, low, high }
    }
}

#[derive(Hash, Debug, Clone, Eq, PartialEq)]
struct ComputedKey {
    f: Rc<NodeType>,
    g: Rc<NodeType>,
    h: Rc<NodeType>,
}

impl ComputedKey {
    fn new(f: Rc<NodeType>, g: Rc<NodeType>, h: Rc<NodeType>) -> Self {
        Self { f, g, h }
    }
}

#[derive(Debug)]
pub struct BDDManager {
    unique_table: HashMap<UniqueKey, Rc<NodeType>>,
    computed_table: HashMap<ComputedKey, Rc<NodeType>>,
    pub bdd: Rc<NodeType>,
}

impl BDDManager {
    /// Creates a new instance of a BDD manager.
    pub fn new() -> Self {
        Self {
            unique_table: HashMap::new(),
            computed_table: HashMap::new(),
            bdd: Rc::new(NodeType::Zero),
        }
    }

    /// Creates a new instance of a BDD manager from a given CNF.
    pub fn from_cnf(cnf: Symbol) -> Self {
        let mut mgr = Self::new();
        mgr.bdd = mgr.from_cnf_rec(cnf);
        mgr
    }

    fn from_cnf_rec(&mut self, cnf: Symbol) -> Rc<NodeType> {
        match cnf {
            Symbol::Posterminal(i) => Rc::new(Node::new(
                i as i64,
                Rc::new(NodeType::Zero),
                Rc::new(NodeType::One),
            )),
            Symbol::Negterminal(i) => Rc::new(Node::new(
                i as i64,
                Rc::new(NodeType::One),
                Rc::new(NodeType::Zero),
            )),
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

    fn add_node_to_unique(
        &mut self,
        var: i64,
        low: Rc<NodeType>,
        high: Rc<NodeType>,
    ) -> Rc<NodeType> {
        Rc::clone(
            self.unique_table
                .entry(UniqueKey::new(var, low.clone(), high.clone()))
                .or_insert(Rc::new(Node::new(var, low, high))),
        )
    }

    fn restrict(&mut self, subtree: Rc<NodeType>, var: i64, val: bool) -> Rc<NodeType> {
        match subtree.as_ref() {
            NodeType::Zero => subtree,
            NodeType::One => subtree,
            NodeType::Complex(node) => {
                if node.top_var > var {
                    return subtree;
                }
                if node.top_var < var {
                    let srh = self.restrict(Rc::clone(&node.high), var, val);
                    let srl = self.restrict(Rc::clone(&node.low), var, val);
                    self.add_node_to_unique(node.top_var, srl, srh)
                } else {
                    if val {
                        self.restrict(Rc::clone(&node.high), var, val)
                    } else {
                        self.restrict(Rc::clone(&node.low), var, val)
                    }
                }
            }
        }
    }

    // Broken
    /*
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
    }*/
    /*
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
    */

    /// Returns true if there is a variable assignment which evaluates the represented formula to `true`.
    pub fn satisfiable(&self) -> bool {
        self.bdd.as_ref() != &NodeType::Zero
    }

    fn ite(&mut self, f: Rc<NodeType>, g: Rc<NodeType>, h: Rc<NodeType>) -> Rc<NodeType> {
        match (f.as_ref(), g.as_ref(), h.as_ref()) {
            (NodeType::Zero, _, _) => h,
            (NodeType::One, _, _) => g,
            (_, NodeType::One, NodeType::Zero) => f,
            (i, t, e) => {
                match self.computed_table.get(&ComputedKey::new(
                    Rc::clone(&f),
                    Rc::clone(&g),
                    Rc::clone(&h),
                )) {
                    Some(entry) => Rc::clone(entry),
                    None => {
                        let v = [i, t, e]
                            .iter()
                            .filter_map(|x| match x {
                                NodeType::Complex(Node { top_var, .. }) => Some(*top_var),
                                _ => None,
                            })
                            .min()
                            .unwrap(); // Unwrap can't fail, because the match ensures that at least one NodeType::Complex(n) is present.

                        let ixt = self.restrict(Rc::clone(&f), v, true);
                        let txt = self.restrict(Rc::clone(&g), v, true);
                        let ext = self.restrict(Rc::clone(&h), v, true);

                        let tv = self.ite(ixt, txt, ext);

                        let ixf = self.restrict(Rc::clone(&f), v, false);
                        let txf = self.restrict(Rc::clone(&g), v, false);
                        let exf = self.restrict(Rc::clone(&h), v, false);

                        let ev = self.ite(ixf, txf, exf);

                        if tv == ev {
                            return tv;
                        }

                        let r = self.add_node_to_unique(v, ev, tv);

                        self.computed_table
                            .insert(ComputedKey::new(f, g, h), Rc::clone(&r));

                        r
                    }
                }
            }
        }
    }

    pub fn and(&mut self, lhs: Rc<NodeType>, rhs: Rc<NodeType>) -> Rc<NodeType> {
        self.ite(lhs, rhs, Rc::new(NodeType::Zero))
    }

    pub fn or(&mut self, lhs: Rc<NodeType>, rhs: Rc<NodeType>) -> Rc<NodeType> {
        self.ite(lhs, Rc::new(NodeType::One), rhs)
    }

    pub fn not(&mut self, val: Rc<NodeType>) -> Rc<NodeType> {
        self.ite(val, Rc::new(NodeType::Zero), Rc::new(NodeType::One))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bdd::bdd_graph::NodeType::*;

    fn build_bdd(path: &str) -> BDDManager {
        let input =
            crate::input::parser::parse_string(&std::fs::read_to_string(path).unwrap()).unwrap();
        let input_symbols = BooleanFunction::new_cnf_formula(input);
        BDDManager::from_cnf(input_symbols)
    }

    #[test]
    fn easy1_structural() {
        let mgr = build_bdd("examples/assets/easy1.dimacs");

        assert_eq!(
            mgr.bdd.as_ref(),
            &Complex(Node {
                top_var: 1,
                low: Rc::new(Complex(Node {
                    top_var: 3,
                    low: Rc::new(One),
                    high: Rc::new(Zero),
                })),
                high: Rc::new(Complex(Node {
                    top_var: 2,
                    low: Rc::new(Complex(Node {
                        top_var: 3,
                        low: Rc::new(Zero),
                        high: Rc::new(One),
                    })),
                    high: Rc::new(One),
                })),
            })
        );
    }

    #[test]
    fn easyns_structural() {
        let mgr = build_bdd("examples/assets/easyns.dimacs");
        assert_eq!(mgr.bdd.as_ref(), &NodeType::Zero);
    }

    #[test]
    #[ignore]
    fn ns_structural() {
        let mgr = build_bdd("examples/assets/ns.dimacs");
        assert_eq!(mgr.bdd.as_ref(), &NodeType::Zero);
    }

    #[test]
    fn sandwich_satisfiable() {
        let mgr = build_bdd("examples/assets/sandwich.dimacs");
        assert!(mgr.satisfiable());
    }

    #[test]
    #[ignore]
    fn berkeleydb_satisfiable() {
        let mgr = build_bdd("examples/assets/berkeleydb.dimacs");
        assert!(mgr.satisfiable());
    }
}
