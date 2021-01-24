use std::collections::HashMap;
use crate::bdd::bdd_graph::BDDGraph;

#[derive(Debug)]
pub struct BDDManager {
    computed_table: HashMap<i32,bool>,
    graph: BDDGraph,
}

impl BDDManager {
    ///Creates a new instance of a BDD manager.
    pub fn new() -> BDDManager {
	BDDManager {
	    computed_table: HashMap::new(),
	    graph: BDDGraph::new(true),
	}
    }
}
