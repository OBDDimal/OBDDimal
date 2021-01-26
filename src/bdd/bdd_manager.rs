use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use crate::bdd::bdd_graph::*;

#[derive(Debug)]
pub struct BDDManager {
    numvars: u32,
    unique_table: HashMap<u64, Node>,
}

impl BDDManager {
    /// Creates a new instance of a BDD manager.
    pub fn new() -> Self    {
	let mut mgr = BDDManager {
	    numvars: 0,
	    unique_table: HashMap::new(),
	};

	mgr.unique_table.insert(0, Node {top_var: 0, high: None, low: None});
	mgr.unique_table.insert(1, Node {top_var: 1, high: None, low: None});
	
	mgr
    }

    fn node_hash(v: u32, h: &Node, l: &Node) -> u64 {
	let mut s = DefaultHasher::new();
	v.hash(&mut s);
	h.hash(&mut s);
	l.hash(&mut s);
	s.finish()
    }

    /// Creates a node entry in the unique_table. Currently ignores hash collisions!
    pub fn make_node(&mut self, var: u32, high: Node, low: Node) -> Node {
	println!("{:?}", &self);
	let node_hash = BDDManager::node_hash(var, &high, &low);
	if !self.unique_table.contains_key(&node_hash) {
	    self.unique_table.insert(node_hash, Node {top_var: var, high: Some(Box::new(high)), low: Some(Box::new(low))});
	    return self.unique_table[&node_hash].clone();
	}
	return self.unique_table[&node_hash].clone();
    }

    /// Returns the node representing the 1 sink node.
    pub fn bdd_true(&self) -> Node {
	self.unique_table[&1].clone()
    }

    /// Returns the node representing the 0 sink node.
    pub fn bdd_false(&self) -> Node {
	self.unique_table[&0].clone()
    }

    pub fn ithvar(&mut self, i: u32) -> Node {
	self.numvars += 1;
	self.unique_table.get_mut(&0).unwrap().top_var = self.numvars;
	self.unique_table.get_mut(&1).unwrap().top_var = self.numvars;
	self.make_node(i, self.bdd_true(), self.bdd_false())
    }

    pub fn restrict(&mut self, subtree: Node, var: u32, val: bool) -> Node {
	if subtree.top_var > var { return subtree.clone(); }
	else if subtree.top_var < var {
	    let h = self.restrict(*subtree.high.unwrap(), var, val);
	    let l =  self.restrict(*subtree.low.unwrap(), var, val);
	    return self.make_node(subtree.top_var, h, l);
	} else {
	    match val {
		true  => self.restrict(*subtree.high.unwrap(), var, val),
		false => self.restrict(*subtree.low.unwrap(), var, val),
	    }
	}
    }

    pub fn ite(&mut self, i: Node, t: Node, e: Node) -> Node {
	if i == self.bdd_true()                          { return t; }
	if i == self.bdd_false()                         { return e; }
	if t == e                                        { return t; }
	if t == self.bdd_true() && e == self.bdd_false() { return i; }

	let split = i.top_var.min(t.top_var).min(e.top_var);

	let ixt = self.restrict(i.clone(), split, true);
	let txt = self.restrict(t.clone(), split, true);
	let ext = self.restrict(e.clone(), split, true);

	let pos_ftor = self.ite(ixt, txt, ext);

	let ixf = self.restrict(i, split, false);
	let txf = self.restrict(t, split, false);
	let exf = self.restrict(e, split, false);

	let neg_ftor = self.ite(ixf, txf, exf);

	self.make_node(split, pos_ftor, neg_ftor)	
    }

    // Bryant API
    pub fn and(&mut self, lhs: Node, rhs: Node) -> Node {
	self.ite(lhs, rhs, self.bdd_false())
    }

    pub fn or(&mut self, lhs: Node, rhs: Node) -> Node {
	self.ite(lhs, self.bdd_true(), rhs)
    }

    pub fn not(&mut self, inp: Node) -> Node {
	self.ite(inp, self.bdd_false(), self.bdd_true())
    }
}
