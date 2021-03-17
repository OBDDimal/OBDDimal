use fnv::FnvHashMap;

use crate::input::boolean_function::*;
use crate::input::parser::{Cnf, DataFormatError, ParserSettings};
use crate::{
    bdd::bdd_graph::*,
    input::static_ordering::{apply_heuristic, StaticOrdering},
};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use rayon::prelude::*;

type UniqueTable = Arc<Mutex<fnv::FnvHashMap<UniqueKey, Arc<NodeType>>>>;

/// Used as key for the unique_table.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UniqueKey {
    tv: i64,
    low: Arc<NodeType>,
    high: Arc<NodeType>,
}

impl Hash for UniqueKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.tv.hash(state);
        std::ptr::hash(&self.low, state);
        std::ptr::hash(&self.high, state);
    }
}

impl UniqueKey {
    fn new(tv: i64, low: Arc<NodeType>, high: Arc<NodeType>) -> Self {
        Self { tv, low, high }
    }
}

/// Used as the key for the computed_table.
#[derive(Debug, Clone, Eq, PartialEq)]
struct ComputedKey {
    f: Arc<NodeType>,
    g: Arc<NodeType>,
    h: Arc<NodeType>,
}

impl Hash for ComputedKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(&self.f, state);
        std::ptr::hash(&self.g, state);
        std::ptr::hash(&self.h, state);
    }
}

impl ComputedKey {
    fn new(f: Arc<NodeType>, g: Arc<NodeType>, h: Arc<NodeType>) -> Self {
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
    //FNV Hash bases HashMaps are faster for smaller keys, so should be a performance boost here.
    unique_table: fnv::FnvHashMap<UniqueKey, Arc<NodeType>>,
    computed_table: fnv::FnvHashMap<ComputedKey, Arc<NodeType>>,
    cnf: Cnf,
    pub bdd: Arc<NodeType>,
}

impl Bdd {
    /// Creates a new instance of a BDD manager out of a given input format.
    /// Currently there is only `InputFormat::CNF` supported, which represents Dimacs CNF.
    pub fn from_format(
        data: &str,
        format: InputFormat,
        settings: ParserSettings,
        static_ordering: StaticOrdering,
    ) -> Result<Self, DataFormatError> {
        let cnf = crate::input::parser::parse_string(data, settings)?;

        let cnf = match static_ordering {
            StaticOrdering::NONE => cnf,
            StaticOrdering::FORCE => apply_heuristic(cnf, StaticOrdering::FORCE),
        };

        let symbolic_rep = match format {
            InputFormat::CNF => {
                crate::boolean_function::BooleanFunction::new_from_cnf_formula(cnf.terms.clone())
            }
        };
        Ok(Bdd::from_cnf(symbolic_rep, cnf))
    }

    /// Creates a new instance of a BDD manager out of a given input format.
    /// Currently there is only `InputFormat::CNF` supported, which represents Dimacs CNF.
    pub fn from_format_para(
        data: &str,
        format: InputFormat,
        settings: ParserSettings,
        static_ordering: StaticOrdering,
	unique_table: UniqueTable
    ) -> Result<Self, DataFormatError> {
        let cnf = crate::input::parser::parse_string(data, settings)?;
	
        let cnf = match static_ordering {
            StaticOrdering::NONE => cnf,
            StaticOrdering::FORCE => apply_heuristic(cnf, StaticOrdering::FORCE),
        };

        let symbolic_rep = match format {
            InputFormat::CNF => {
                crate::boolean_function::BooleanFunction::new_from_cnf_formula(cnf.terms.clone())
            }
        };
        Ok(Bdd::from_cnf_para(symbolic_rep, cnf, unique_table))
    }

    /// Creates a new instance of a BDD manager from a given CNF.
    /// FIXME: Manager has to support the arc<mutex> type for unique_table
    fn from_cnf_para(symbols: Symbol, cnf: Cnf, unique_table: UniqueTable) -> Self {
	let cnf1 = cnf.clone();
        let mut mgr = Self {
            unique_table: fnv::FnvHashMap::with_capacity_and_hasher(10000000, Default::default()),
            computed_table: fnv::FnvHashMap::default(),
            bdd: Arc::new(NodeType::Zero),
            cnf,
        };
        mgr.bdd = Self::from_cnf_para_rec(symbols, cnf1, unique_table);
        mgr
    }

    /// Helper method for `from_cnf_para`.
    fn from_cnf_para_rec(symbols: Symbol, cnf: Cnf, unique_table: UniqueTable) -> Arc<NodeType> {
        match symbols {
            Symbol::Posterminal(i) => Arc::new(Node::new_node_type(
                i as i64,
                Arc::new(NodeType::Zero),
                Arc::new(NodeType::One),
            )),
            Symbol::Negterminal(i) => Arc::new(Node::new_node_type(
                i as i64,
                Arc::new(NodeType::One),
                Arc::new(NodeType::Zero),
            )),
            Symbol::Function(func) => match func.op {
                Operator::And => {
                    let l = Self::from_cnf_para_rec(*func.lhs, cnf.clone(), Arc::clone(&unique_table));
                    let r = Self::from_cnf_para_rec(*func.rhs, cnf.clone(), Arc::clone(&unique_table));
                    and_para(l, r, unique_table, cnf)
                }
                Operator::Or => {
                    let l = Self::from_cnf_para_rec(*func.lhs, cnf.clone(), Arc::clone(&unique_table));
                    let r = Self::from_cnf_para_rec(*func.rhs, cnf.clone(), Arc::clone(&unique_table));
                    or_para(l, r, unique_table, cnf)
                }
            },
        }
    }


    /// Creates a new instance of a BDD manager from a given CNF.
    fn from_cnf(symbols: Symbol, cnf: Cnf) -> Self {
        let mut mgr = Self {
            unique_table: fnv::FnvHashMap::with_capacity_and_hasher(10000000, Default::default()),
            computed_table: fnv::FnvHashMap::default(),
            bdd: Arc::new(NodeType::Zero),
            cnf,
        };
        mgr.bdd = mgr.from_cnf_rec(symbols);
        mgr
    }

    /// Helper method for `from_cnf`.
    fn from_cnf_rec(&mut self, cnf: Symbol) -> Arc<NodeType> {
        match cnf {
            Symbol::Posterminal(i) => Arc::new(Node::new_node_type(
                i as i64,
                Arc::new(NodeType::Zero),
                Arc::new(NodeType::One),
            )),
            Symbol::Negterminal(i) => Arc::new(Node::new_node_type(
                i as i64,
                Arc::new(NodeType::One),
                Arc::new(NodeType::Zero),
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
        low: Arc<NodeType>,
        high: Arc<NodeType>,
    ) -> Arc<NodeType> {
        Arc::clone(
            self.unique_table
                .entry(UniqueKey::new(var, low.clone(), high.clone()))
                .or_insert_with(|| Arc::new(Node::new_node_type(var, low, high))),
        )
    }

    pub fn nodecount(&self) -> u64 {
        if self.bdd.as_ref() == &NodeType::Zero {
            1
        } else {
            2 + Self::nodecount_rec(Arc::clone(&self.bdd))
        }
    }

    fn nodecount_rec(subtree: Arc<NodeType>) -> u64 {
        let root = subtree.as_ref();

        match root {
            NodeType::Zero => 0,
            NodeType::One => 0,
            NodeType::Complex(n) => {
                1 + Self::nodecount_rec(Arc::clone(&n.low)) + Self::nodecount_rec(Arc::clone(&n.high))
            }
        }
    }

    /// Returns the number of variable assignments that evaluate the represented BDD to true.
    pub fn satcount(&self) -> u64 {
        let mut count: u64 = 0;
        let mut stack = vec![];

        stack.push((Arc::clone(&self.bdd), 0));

        while !stack.is_empty() {
            let tuple = stack.pop().unwrap(); // unwrap is okay, because stack can't be empty here.
            let node = tuple.0.as_ref();
            let depth = tuple.1;

            match node {
                NodeType::Zero | NodeType::One => {
                    if node == &NodeType::One {
                        count += 2_u64.pow(self.cnf.varibale_count - depth);
                    }
                }
                NodeType::Complex(n) => {
                    stack.push((Arc::clone(&n.low), depth + 1));
                    stack.push((Arc::clone(&n.high), depth + 1));
                }
            }
        }

        count
    }

    /// Returns true if there is a variable assignment which evaluates the represented formula to `true`.
    pub fn satisfiable(&self) -> bool {
        self.bdd.as_ref() != &NodeType::Zero
    }

    fn restrict(
        &mut self,
        node: Arc<NodeType>,
        v: i64,
        order: &Vec<i32>,
        val: bool,
    ) -> Arc<NodeType> {
        match node.as_ref() {
            NodeType::Complex(n) => {
                let order_v = order.iter().position(|&x| x as i64 == v).unwrap();
                let order_top_var = order.iter().position(|&x| x as i64 == n.top_var).unwrap();
                if val {
                    if order_v < order_top_var {
                        node
                    } else if order_v == order_top_var {
                        Arc::clone(&n.high)
                    } else {
                        let low = self.restrict(Arc::clone(&n.low), v, order, val);
                        let high = self.restrict(Arc::clone(&n.high), v, order, val);
                        self.add_node_to_unique(n.top_var, low, high)
                    }
                } else {
                    if order_v < order_top_var {
                        node
                    } else if order_v == order_top_var {
                        Arc::clone(&n.low)
                    } else {
                        let low = self.restrict(Arc::clone(&n.low), v, order, val);
                        let high = self.restrict(Arc::clone(&n.high), v, order, val);
                        self.add_node_to_unique(n.top_var, low, high)
                    }
                }
            }
            NodeType::Zero => node,
            NodeType::One => node,
        }
    }

    /// If-then-else, if `f` ite returns `g`, else `h`.
    fn ite(&mut self, f: Arc<NodeType>, g: Arc<NodeType>, h: Arc<NodeType>) -> Arc<NodeType> {
        match (f.as_ref(), g.as_ref(), h.as_ref()) {
            (_, NodeType::One, NodeType::Zero) => f,
            (_, NodeType::Zero, NodeType::One) => self.not(f),
            (NodeType::One, _, _) => g,
            (NodeType::Zero, _, _) => h,
            (_, t, e) if t == e => g,
            (i, t, e) => {
                match self.computed_table.get(&ComputedKey::new(
                    Arc::clone(&f),
                    Arc::clone(&g),
                    Arc::clone(&h),
                )) {
                    Some(entry) => Arc::clone(entry),
                    None => {
                        let v = [i, t, e]
                            .iter()
                            .filter_map(|x| match x {
                                NodeType::Complex(Node { top_var, .. }) => Some(*top_var),
                                _ => None,
                            })
                            .min()
                            .unwrap(); // Unwrap can't fail, because the match ensures that at least one NodeType::Complex(n) is present.

                        let order = self.cnf.order.clone();

                        let ixt = self.restrict(Arc::clone(&f), v, &order, true);
                        let txt = self.restrict(Arc::clone(&g), v, &order, true);
                        let ext = self.restrict(Arc::clone(&h), v, &order, true);

                        let tv = self.ite(ixt, txt, ext);

                        let ixf = self.restrict(Arc::clone(&f), v, &order, false);
                        let txf = self.restrict(Arc::clone(&g), v, &order, false);
                        let exf = self.restrict(Arc::clone(&h), v, &order, false);

                        let ev = self.ite(ixf, txf, exf);

                        if tv == ev {
                            return tv;
                        }

                        let r = self.add_node_to_unique(v, ev, tv);

                        self.computed_table
                            .insert(ComputedKey::new(f, g, h), Arc::clone(&r));

                        r
                    }
                }
            }
        }
    }

    /// Calculates the Boolean AND with the given left hand side `lhs` and the given right hand side `rhs`.
    pub fn and(&mut self, lhs: Arc<NodeType>, rhs: Arc<NodeType>) -> Arc<NodeType> {
        self.ite(lhs, rhs, Arc::new(NodeType::Zero))
    }

    /// Calculates the Boolean OR with the given left hand side `lhs` and the given right hand side `rhs`.
    pub fn or(&mut self, lhs: Arc<NodeType>, rhs: Arc<NodeType>) -> Arc<NodeType> {
        self.ite(lhs, Arc::new(NodeType::One), rhs)
    }

    /// Calculates the Boolean NOT with the given value `val`.
    pub fn not(&mut self, val: Arc<NodeType>) -> Arc<NodeType> {
        self.ite(val, Arc::new(NodeType::Zero), Arc::new(NodeType::One))
    }

    /// Serializes `self` to a String representing the BDD.
    /// The serialization of the BDD obeys the following rules:
    /// 1. The first line of the string is the variable ordering of the BDD.
    /// 2. Every following line represents a node, where the first number is the internal ID of the node
    /// the second number is the `top_var` of the current node, the thrid number is the id of the node connected to the low edge
    /// the fourth number is the id of the node connected to the high edge of the current node.
    /// 3. Internal ID 0 and 1 are representations of the terminal ZERO and ONE node.
    /// The last two lines have to be appended as the sink nodes for the recursive deserialization to work.
    pub fn serialize(&self) -> String {
        let root = Arc::clone(&self.bdd);
        let result = Self::serialize_rec(root);
        let mut buffer = String::new();

        let variable_order = self
            .cnf
            .order
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<String>>()
            .join(" ");

        for l in result.split_whitespace() {
            buffer.push_str(l);
            buffer.push_str("\n");
        }

        let mut serialized_bdd = String::new();
        serialized_bdd.push_str(&variable_order);
        serialized_bdd.push_str("\n");
        serialized_bdd.push_str(&buffer);
        serialized_bdd.push_str("0,0,0,0\n");
        serialized_bdd.push_str("1,0,0,0\n");

        serialized_bdd
    }

    fn serialize_rec(subtree: Arc<NodeType>) -> String {
        let node = subtree.as_ref();

        match node {
            NodeType::Zero => String::from(""),
            NodeType::One => String::from(""),
            NodeType::Complex(n) => {
                let low_id = match n.low.as_ref() {
                    NodeType::Zero => String::from("0"),
                    NodeType::One => String::from("1"),
                    NodeType::Complex(low_n) => low_n.id.to_string(),
                };
                let high_id = match n.high.as_ref() {
                    NodeType::Zero => String::from("0"),
                    NodeType::One => String::from("1"),
                    NodeType::Complex(high_n) => high_n.id.to_string(),
                };
                let id = n.id;

                let low = Self::serialize_rec(Arc::clone(&n.low));
                let high = Self::serialize_rec(Arc::clone(&n.high));
                format!(
                    "{},{},{},{}\n{}\n{}",
                    id, n.top_var, low_id, high_id, low, high
                )
            }
        }
    }

    /// Deserializes the given string (which was previously serialized by `serialize`) into a `Bdd`.
    /// TODO: Error handling for wrong input formats.
    /// TODO: Enhance runtime by removing linear iteration of input data.
    /// WARNING: VERY EXPERIMENTAL AND NOT THOROUGHLY TESTED (Workd with easy1.dimacs and sandwich.dimacs)
    pub fn deserialize(input: String) -> Bdd {
        let mut line_iter = input.lines();
        // Get the variable order from the input, which is the first line of the given string.
        let var_order = line_iter
            .next()
            .unwrap()
            .split_whitespace()
            .map(|y| y.parse::<i32>().unwrap())
            .collect::<Vec<i32>>();

        // Create an empty Cnf for the Bdd
        let cnf = Cnf {
            varibale_count: var_order.len() as u32,
            term_count: 0,
            terms: vec![],
            order: var_order,
        };

        // Rebuild the input without the variable ordering
        let mut rest_input = String::new();

        for l in line_iter {
            rest_input.push_str(&(l.to_string() + "\n"));
        }

        // Get the first line of the input, which represents the root node.
        let current_line = String::from(rest_input.lines().next().unwrap());

        // Feed the input into the recursive build process. It returns the corresponding Bdd and HashMap.
        let (bdd, unique_table) =
            Self::deserialize_rec(rest_input, current_line, FnvHashMap::default());

        // Create the final Bdd.
        Bdd {
            unique_table: unique_table,
            computed_table: FnvHashMap::default(),
            cnf: cnf,
            bdd: bdd,
        }
    }

    fn deserialize_rec(
        complete_input: String,
        current: String,
        hash_map: FnvHashMap<UniqueKey, Arc<NodeType>>,
    ) -> (Arc<NodeType>, FnvHashMap<UniqueKey, Arc<NodeType>>) {
        // Extract the important features of a given line.
        let mut current_line = current.split(',');
        let internal_id = current_line.next().unwrap().parse::<i64>().unwrap();
        let top_var = current_line.next().unwrap().parse::<i64>().unwrap();
        let low_id = current_line.next().unwrap().parse::<u32>().unwrap();
        let high_id = current_line.next().unwrap().parse::<u32>().unwrap();

        // If we are internal_id == 0, we are the ZERO sink node.
        if internal_id == 0 {
            return (Arc::new(NodeType::Zero), hash_map);
        }

        // If we are internal_id == 1, we are the ONE sink node.
        if internal_id == 1 {
            return (Arc::new(NodeType::One), hash_map);
        }

        // Search for the line in the input representing the ID of the low edge.
        let low_line = match complete_input
            .lines()
            .filter(|x| x.split(',').next().unwrap().parse::<u32>().unwrap() == low_id)
            .next()
        {
            Some(i) => String::from(i), // If there is a corresponding line, get it.
            None => panic!("Can't be reached!"), // TODO: Otherwise do a better error handling job.
        };

        // Search for the line in the input representing the ID of the high edge.
        let high_line = match complete_input
            .lines()
            .filter(|x| x.split(',').next().unwrap().parse::<u32>().unwrap() == high_id)
            .next()
        {
            Some(i) => String::from(i), // If there is a corresponding line, get it.
            None => panic!("Can't be reached!"), // TODO: Otherwise do a better error handling job.
        };

        // Get the low edge of the Bdd.
        let (low, hash_map) = Self::deserialize_rec(complete_input.clone(), low_line, hash_map);
        // Get the high edge of the Bdd.
        let (high, mut hash_map) = Self::deserialize_rec(complete_input, high_line, hash_map);
        // Construct the node of the Bdd.
        let node = Arc::new(Node::new_node_type(
            top_var,
            Arc::clone(&low),
            Arc::clone(&high),
        ));
        // Add the node to the unique_table.
        hash_map.insert(UniqueKey::new(top_var, low, high), Arc::clone(&node));
        // Return the constructed node and the filled unique_table.
        (node, hash_map)
    }
}

/// Calculates the Boolean AND with the given left hand side `lhs` and the given right hand side `rhs`.
pub fn and_para(lhs: Arc<NodeType>, rhs: Arc<NodeType>, unique_table: UniqueTable, cnf: Cnf) -> Arc<NodeType> {
    para_ite(lhs, rhs, Arc::new(NodeType::Zero), unique_table, cnf)
}

/// Calculates the Boolean OR with the given left hand side `lhs` and the given right hand side `rhs`.
pub fn or_para(lhs: Arc<NodeType>, rhs: Arc<NodeType>, unique_table: UniqueTable, cnf: Cnf) -> Arc<NodeType> {
    para_ite(lhs, Arc::new(NodeType::One), rhs, unique_table, cnf)
}

fn para_ite(f: Arc<NodeType>, g: Arc<NodeType>, h: Arc<NodeType>, unique_table: UniqueTable, cnf: Cnf) -> Arc<NodeType> {
    let mut computed_table = FnvHashMap::default();
    
        match (f.as_ref(), g.as_ref(), h.as_ref()) {
            (_, NodeType::One, NodeType::Zero) => f,
            (NodeType::One, _, _) => g,
            (NodeType::Zero, _, _) => h,
            (_, t, e) if t == e => g,
            (i, t, e) => {
                match computed_table.get(&ComputedKey::new(
                    Arc::clone(&f),
                    Arc::clone(&g),
                    Arc::clone(&h),
                )) {
                    Some(entry) => Arc::clone(entry),
                    None => {
                        let v = [i, t, e]
                            .iter()
                            .filter_map(|x| match x {
                                NodeType::Complex(Node { top_var, .. }) => Some(*top_var),
                                _ => None,
                            })
                            .min()
                            .unwrap(); // Unwrap can't fail, because the match ensures that at least one NodeType::Complex(n) is present.

                        let order = cnf.order.clone();
			let cnf_right = cnf.clone();
			let cnf_left = cnf;
                        
                        let ixt = para_restrict(Arc::clone(&f), v, &order, true, Arc::clone(&unique_table));
                        let txt = para_restrict(Arc::clone(&g), v, &order, true, Arc::clone(&unique_table));
                        let ext = para_restrict(Arc::clone(&h), v, &order, true, Arc::clone(&unique_table));

                        let ixf = para_restrict(Arc::clone(&f), v, &order, false, Arc::clone(&unique_table));
                        let txf = para_restrict(Arc::clone(&g), v, &order, false, Arc::clone(&unique_table));
                        let exf = para_restrict(Arc::clone(&h), v, &order, false, Arc::clone(&unique_table));

                        let arc_to_ut_a = Arc::clone(&unique_table);
                        let arc_to_ut_b = Arc::clone(&unique_table);
			
                        //Obviously not working, but that should be the whole trick of parallelizing ITE.
                        let (tv, ev) = rayon::join(
                            || para_ite(ixt, txt, ext, arc_to_ut_a, cnf_left), 
                            || para_ite(ixf, txf, exf, arc_to_ut_b, cnf_right));

                        if tv == ev {
                            return tv;
                        }

                        let r = para_add_node_to_unique(unique_table, v, ev, tv);

                        computed_table
                            .insert(ComputedKey::new(f, g, h), Arc::clone(&r));

                        r
                    }
                }
            }
        }
}

fn para_restrict(
    node: Arc<NodeType>,
    v: i64,
    order: &Vec<i32>,
    val: bool,
    unique_table: UniqueTable
    ) -> Arc<NodeType> {
        match node.as_ref() {
            NodeType::Complex(n) => {
                let order_v = order.iter().position(|&x| x as i64 == v).unwrap();
                let order_top_var = order.iter().position(|&x| x as i64 == n.top_var).unwrap();
                if val {
                    if order_v < order_top_var {
                        node
                    } else if order_v == order_top_var {
                        Arc::clone(&n.high)
                    } else {
                        let low = para_restrict(Arc::clone(&n.low), v, order, val, Arc::clone(&unique_table));
                        let high = para_restrict(Arc::clone(&n.high), v, order, val, Arc::clone(&unique_table));
                        para_add_node_to_unique(unique_table, n.top_var, low, high)
                    }
                } else {
                    if order_v < order_top_var {
                        node
                    } else if order_v == order_top_var {
                        Arc::clone(&n.low)
                    } else {
                        let low = para_restrict(Arc::clone(&n.low), v, order, val, Arc::clone(&unique_table));
                        let high = para_restrict(Arc::clone(&n.high), v, order, val, Arc::clone(&unique_table));
                        para_add_node_to_unique(unique_table, n.top_var, low, high)
                    }
                }
            }
            NodeType::Zero => node,
            NodeType::One => node,
        }
}

    /// Adds a `NodeType` to the unique_table, if it is not already there.
fn para_add_node_to_unique(
    unique_table: UniqueTable,
    var: i64,
    low: Arc<NodeType>,
    high: Arc<NodeType>,
) -> Arc<NodeType> {
    Arc::clone(
        unique_table
	    .lock()
	    .unwrap()
            .entry(UniqueKey::new(var, low.clone(), high.clone()))
            .or_insert_with(|| Arc::new(Node::new_node_type(var, low, high))),
    )
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

    fn build_bdd_para(path: &str) -> Bdd {
	let input = crate::input::parser::parse_string(
            &std::fs::read_to_string(path).unwrap(),
            ParserSettings::default(),
        )
        .unwrap();
        let input_symbols = BooleanFunction::new_from_cnf_formula(input.terms.clone());
	let unique_table = Arc::new(Mutex::new(fnv::FnvHashMap::default()));
        Bdd::from_cnf_para(input_symbols, input, unique_table)
    }

    #[test]
    fn easy1_sat_para() {
        let mgr = build_bdd_para("examples/assets/easy1.dimacs");
        assert!(mgr.satisfiable());
        assert_eq!(mgr.satcount(), 5);
    }

    #[test]
    fn sandwich_sat_para() {
        let mgr = build_bdd_para("examples/assets/sandwich.dimacs");
        assert!(mgr.satisfiable());
        assert_eq!(mgr.satcount(), 2808);
    }

    #[test]
    #[ignore = "Takes a long time"]
    fn berkeleydb_sat_para() {
        let mgr = build_bdd_para("examples/assets/berkeleydb.dimacs");
        assert!(mgr.satisfiable());
        assert_eq!(mgr.nodecount(), 356704); //Should be around 1000-5000
        assert_eq!(mgr.satcount(), 4080389785);
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
                low: Arc::new(Complex(Node {
                    id: 21,
                    top_var: 3,
                    low: Arc::new(One),
                    high: Arc::new(Zero),
                })),
                high: Arc::new(Complex(Node {
                    id: 27,
                    top_var: 2,
                    low: Arc::new(Complex(Node {
                        id: 24,
                        top_var: 3,
                        low: Arc::new(Zero),
                        high: Arc::new(One),
                    })),
                    high: Arc::new(One),
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
    fn sandwich_serialize_deserialize() {
        let bdd = build_bdd("examples/assets/sandwich.dimacs");
        let ser = bdd.serialize();
        let bdd = Bdd::deserialize(ser);
        assert!(bdd.satisfiable());
        assert_eq!(bdd.satcount(), 2808);
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
    #[ignore = "Takes a long time"]
    fn berkeleydb_sat() {
        let mgr = build_bdd("examples/assets/berkeleydb.dimacs");
        assert!(mgr.satisfiable());
        assert_eq!(mgr.nodecount(), 356704); //Should be around 1000-5000
        assert_eq!(mgr.satcount(), 4080389785);
    }

    #[test]
    #[ignore = "Takes a long time."]
    fn berkeley_serialize_deserialize() {
        let bdd = build_bdd("examples/assets/berkeleydb.dimacs");
        let ser = bdd.serialize();
        let bdd = Bdd::deserialize(ser);
        assert!(bdd.satisfiable());
        assert_eq!(bdd.satcount(), 4080389785);
    }
}
