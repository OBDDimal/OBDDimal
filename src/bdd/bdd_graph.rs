/// Representing the possible states a child of a node in a Binary Decision Diagram can be.
#[derive(Debug)]
pub enum Child {
    ZERO,
    ONE,
    GRAPH(Box<BDDGraph>),
}

/// Representation of a Binary Decision Diagram, containing a boolean variable `var` and the children `hi` and `lo`).
#[derive(Debug)]
pub struct BDDGraph {
    pub var: bool,
    pub hi: Child,
    pub lo: Child,
}

use Child::*;

impl BDDGraph {   
    /// Creates a root node for a new BDD.
    pub fn new(var: bool) -> BDDGraph {
	BDDGraph {
	    var: var,
	     hi: ONE,
	     lo: ZERO,
	}
    }

    /// Returns the variable of the root node.
    pub fn var(self) -> bool {
	self.var
    }

    /// Returns the hi side of the root node.
    pub fn hi(self) -> Child {
	match self.hi {
	    ONE          => ONE,
	    ZERO         => ZERO,
	    GRAPH(graph) => GRAPH(graph),
	}
    }

    /// Returns the lo side of the root node.
    pub fn lo(self) -> Child {
	match self.lo {
	    ONE          => ONE,
	    ZERO         => ZERO,
	    GRAPH(graph) => GRAPH(graph),
	}
    }
}
