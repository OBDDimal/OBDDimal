use crate::bdd::bdd_graph::*;
use crate::input::boolean_function::*;
use crate::input::parser::{Cnf, DataFormatError, ParserSettings};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

/// Used as key for the unique_table.
#[derive(Debug, Clone, Eq, PartialEq)]
struct UniqueKey {
    tv: i64,
    low: Rc<NodeType>,
    high: Rc<NodeType>,
}

impl Hash for UniqueKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.tv.hash(state);
        std::ptr::hash(&self.low, state);
        std::ptr::hash(&self.high, state);
    }
}

impl UniqueKey {
    fn new(tv: i64, low: Rc<NodeType>, high: Rc<NodeType>) -> Self {
        Self { tv, low, high }
    }
}

/// Used as the key for the computed_table.
#[derive(Debug, Clone, Eq, PartialEq)]
struct ComputedKey {
    f: Rc<NodeType>,
    g: Rc<NodeType>,
    h: Rc<NodeType>,
}

impl Hash for ComputedKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(&self.f, state);
        std::ptr::hash(&self.g, state);
        std::ptr::hash(&self.h, state);
    }
}

impl ComputedKey {
    fn new(f: Rc<NodeType>, g: Rc<NodeType>, h: Rc<NodeType>) -> Self {
        Self { f, g, h }
    }
}

/// All the data formats that are currently supported to create a BDD from.
pub enum InputFormat {
    CNF,
}

/// Represents a wrapper struct for a BDD, allowing us to query methods on it.
#[derive(Debug)]
pub struct Bdd {
    unique_table: HashMap<UniqueKey, Rc<NodeType>>,
    computed_table: HashMap<ComputedKey, Rc<NodeType>>,
    cnf: Cnf,
    pub bdd: Rc<NodeType>,
}

impl Bdd {
    /// Creates a new instance of a BDD manager out of a given input format.
    /// Currently there is only `InputFormat::CNF` supported, which represents Dimacs CNF.
    pub fn from_format(
        data: &str,
        format: InputFormat,
        settings: ParserSettings,
    ) -> Result<Self, DataFormatError> {
        let cnf = crate::input::parser::parse_string(data, settings)?;

        let symbolic_rep = match format {
            InputFormat::CNF => {
                crate::boolean_function::BooleanFunction::new_from_cnf_formula(cnf.terms.clone())
            }
        };
        Ok(Bdd::from_cnf(symbolic_rep, cnf))
    }

    /// Creates a new instance of a BDD manager from a given CNF.
    fn from_cnf(symbols: Symbol, cnf: Cnf) -> Self {
        let mut mgr = Self {
            unique_table: HashMap::new(),
            computed_table: HashMap::new(),
            bdd: Rc::new(NodeType::Zero),
            cnf,
        };
        mgr.bdd = mgr.from_cnf_rec(symbols);
        mgr
    }

    /// Helper method for `from_cnf`.
    fn from_cnf_rec(&mut self, cnf: Symbol) -> Rc<NodeType> {
        match cnf {
            Symbol::Posterminal(i) => Rc::new(Node::new_node_type(
                i as i64,
                Rc::new(NodeType::Zero),
                Rc::new(NodeType::One),
            )),
            Symbol::Negterminal(i) => Rc::new(Node::new_node_type(
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

    /// Adds a `NodeType` to the unique_table, if it is not already there.
    fn add_node_to_unique(
        &mut self,
        var: i64,
        low: Rc<NodeType>,
        high: Rc<NodeType>,
    ) -> Rc<NodeType> {
        Rc::clone(
            self.unique_table
                .entry(UniqueKey::new(var, low.clone(), high.clone()))
                .or_insert_with(|| Rc::new(Node::new_node_type(var, low, high))),
        )
    }

    pub fn nodecount(&self) -> u64 {
        if self.bdd.as_ref() == &NodeType::Zero {
            1
        } else {
            2 + Self::nodecount_rec(Rc::clone(&self.bdd))
        }
    }

    fn nodecount_rec(subtree: Rc<NodeType>) -> u64 {
        let root = subtree.as_ref();

        match root {
            NodeType::Zero => 0,
            NodeType::One => 0,
            NodeType::Complex(n) => {
                1 + Self::nodecount_rec(Rc::clone(&n.low)) + Self::nodecount_rec(Rc::clone(&n.high))
            }
        }
    }

    /// Returns the number of variable assignments that evaluate the represented BDD to true.
    pub fn satcount(&self) -> u64 {
        let mut count: u64 = 0;
        let mut stack = vec![];

        stack.push((Rc::clone(&self.bdd), 0));

        while !stack.is_empty() {
            let tuple = stack.pop().unwrap(); // unwarp is okay, because stack can't be empty there.
            let node = tuple.0.as_ref();
            let depth = tuple.1;

            match node {
                NodeType::Zero | NodeType::One => {
                    if node == &NodeType::One {
                        count += 2_u64.pow(self.cnf.varibale_count - depth);
                    }
                }
                NodeType::Complex(n) => {
                    stack.push((Rc::clone(&n.low), depth + 1));
                    stack.push((Rc::clone(&n.high), depth + 1));
                }
            }
        }

        count
    }

    /// Returns true if there is a variable assignment which evaluates the represented formula to `true`.
    pub fn satisfiable(&self) -> bool {
        self.bdd.as_ref() != &NodeType::Zero
    }

    fn restrict_alternative(&mut self, node: Rc<NodeType>, v: i64, val: bool) -> Rc<NodeType> {
        match node.as_ref() {
            NodeType::Complex(n) => {
                if val {
                    // we assume v is less or equal to n.top_var (w)
                    if v < n.top_var {
                        node
                    } else if v == n.top_var {
                        Rc::clone(&n.high)
                    } else {
                        panic!("v is bigger than w!");
                    }
                } else {
                    if v < n.top_var {
                        node
                    } else if v == n.top_var {
                        Rc::clone(&n.low)
                    } else {
                        panic!("v is bigger than w!");
                    }
                }
            }
            NodeType::Zero => node,
            NodeType::One => node,
        }
    }

    /// If-then-else, if `f` ite returns `g`, else `h`.
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

                        let ixt = self.restrict_alternative(Rc::clone(&f), v, true);
                        let txt = self.restrict_alternative(Rc::clone(&g), v, true);
                        let ext = self.restrict_alternative(Rc::clone(&h), v, true);

                        let tv = self.ite(ixt, txt, ext);

                        let ixf = self.restrict_alternative(Rc::clone(&f), v, false);
                        let txf = self.restrict_alternative(Rc::clone(&g), v, false);
                        let exf = self.restrict_alternative(Rc::clone(&h), v, false);

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

    /// Calculates the Boolean AND with the given left hand side `lhs` and the given right hand side `rhs`.
    pub fn and(&mut self, lhs: Rc<NodeType>, rhs: Rc<NodeType>) -> Rc<NodeType> {
        self.ite(lhs, rhs, Rc::new(NodeType::Zero))
    }

    /// Calculates the Boolean OR with the given left hand side `lhs` and the given right hand side `rhs`.
    pub fn or(&mut self, lhs: Rc<NodeType>, rhs: Rc<NodeType>) -> Rc<NodeType> {
        self.ite(lhs, Rc::new(NodeType::One), rhs)
    }

    /// Calculates the Boolean NOT with the given value `val`.
    pub fn not(&mut self, val: Rc<NodeType>) -> Rc<NodeType> {
        self.ite(val, Rc::new(NodeType::Zero), Rc::new(NodeType::One))
    }

    pub fn serialize(&self) -> String {
        let root = Rc::clone(&self.bdd);
        let result = Self::serialize_rec(root);
        let mut buffer = String::new();

        for l in result.split_whitespace() {
            buffer.push_str(l);
            buffer.push_str("\n");
        }

        buffer
    }

    fn serialize_rec(subtree: Rc<NodeType>) -> String {
        let node = subtree.as_ref();

        match node {
            NodeType::Zero => String::from(""),
            NodeType::One => String::from(""),
            NodeType::Complex(n) => {
                let low_id = match n.low.as_ref() {
                    NodeType::Zero => String::from("ZERO"),
                    NodeType::One => String::from("ONE"),
                    NodeType::Complex(low_n) => low_n.id.to_string(),
                };
                let high_id = match n.high.as_ref() {
                    NodeType::Zero => String::from("ZERO"),
                    NodeType::One => String::from("ONE"),
                    NodeType::Complex(high_n) => high_n.id.to_string(),
                };
                let id = n.id;

                let low = Self::serialize_rec(Rc::clone(&n.low));
                let high = Self::serialize_rec(Rc::clone(&n.high));
                format!("{},{},{}\n{}\n{}", id, low_id, high_id, low, high)
            }
        }
    }

    pub fn deserialize(_input: String) -> Bdd {
        todo!();
    }

    fn deserialize_rec(input: String) -> Bdd {
        let mut lines = input.lines();
        let current = lines.next();

        let mut symbols = current.unwrap().split_terminator(',');
        let id = symbols.next();
        let low_id = symbols.next();
        let high_id = symbols.next();

        todo!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bdd::bdd_graph::NodeType::*;

    fn build_bdd(path: &str) -> Bdd {
        let input = crate::input::parser::parse_string(
            &std::fs::read_to_string(path).unwrap(),
            ParserSettings::default(),
        )
        .unwrap();
        let input_symbols = BooleanFunction::new_from_cnf_formula(input.terms.clone());
        Bdd::from_cnf(input_symbols, input)
    }

    #[test]
    #[ignore = "Only works with --test-threads=1 because parallelism changes the global counter for the node ID."]
    fn easy1_structural() {
        let mgr = build_bdd("examples/assets/easy1.dimacs");

        assert_eq!(
            mgr.bdd.as_ref(),
            &Complex(Node {
                id: 29,
                top_var: 1,
                low: Rc::new(Complex(Node {
                    id: 21,
                    top_var: 3,
                    low: Rc::new(One),
                    high: Rc::new(Zero),
                })),
                high: Rc::new(Complex(Node {
                    id: 27,
                    top_var: 2,
                    low: Rc::new(Complex(Node {
                        id: 24,
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
    fn easy1_sat() {
        let mgr = build_bdd("examples/assets/easy1.dimacs");
        assert!(mgr.satisfiable());
        assert_eq!(mgr.satcount(), 5);
    }

    #[test]
    fn easy1_serialize_deserialize() {
        let bdd = build_bdd("examples/assets/easy1.dimacs");
        let ser = bdd.serialize();
        let bdd = Bdd::deserialize(ser);
        assert!(bdd.satisfiable());
        assert_eq!(bdd.satcount(), 5);
    }

    #[test]
    fn easyns_nodecount() {
        let mgr = build_bdd("examples/assets/easyns.dimacs");
        assert_eq!(mgr.nodecount(), 1);
    }

    #[test]
    fn easy1_nodecount() {
        let mgr = build_bdd("examples/assets/easy1.dimacs");
        assert_eq!(mgr.nodecount(), 6);
    }

    #[test]
    fn sandwich_nodecount() {
        let mgr = build_bdd("examples/assets/sandwich.dimacs");
        assert_eq!(mgr.nodecount(), 353); //Should be around 20-50
    }

    #[test]
    fn easyns_satcount() {
        let mgr = build_bdd("examples/assets/easyns.dimacs");
        assert_eq!(mgr.satcount(), 0);
    }

    #[test]
    fn easyns_structural() {
        let mgr = build_bdd("examples/assets/easyns.dimacs");
        assert_eq!(mgr.bdd.as_ref(), &NodeType::Zero);
    }

    #[test]
    fn sandwich_sat() {
        let mgr = build_bdd("examples/assets/sandwich.dimacs");
        assert!(mgr.satisfiable());
        assert_eq!(mgr.satcount(), 2808);
    }

    #[test]
    #[ignore = "takes a long time"]
    fn berkeleydb_sat() {
        let mgr = build_bdd("examples/assets/berkeleydb.dimacs");
        assert!(mgr.satisfiable());
        assert_eq!(mgr.nodecount(), 356704); //Should be around 1000-5000
        assert_eq!(mgr.satcount(), 4080389785);
    }
}
