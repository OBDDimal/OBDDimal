use rustc_hash::FxHashMap as HashMap;
use rustc_hash::FxHashSet as HashSet;

use crate::bdd_node::{DDNode, NodeID, VarID};

use super::{DDManager, ZERO};

impl DDManager {
    /// Collect all nodes that are part of the specified function
    fn collect_nodes(&self, f: NodeID) -> HashSet<NodeID> {
        let mut h = HashSet::default();

        fn collect_impl(man: &DDManager, f: NodeID, h: &mut HashSet<NodeID>) {
            if h.contains(&f) {
                return;
            }

            let inserted = h.insert(f);
            assert!(inserted);

            let node = man.nodes.get(&f).unwrap();
            collect_impl(man, node.low, h);
            collect_impl(man, node.high, h);
        }

        collect_impl(self, f, &mut h);
        h
    }

    /// Generate graphviz for the provided function, not including any graph nodes not part of the function.
    /// TODO: Graphviz of all functions in DDManager
    pub fn graphviz(&self, f: NodeID) -> String {
        let nodes = self.collect_nodes(f);

        let mut by_var: HashMap<VarID, Vec<DDNode>> = HashMap::default();
        for id in nodes.iter() {
            let node = self.nodes.get(id).unwrap();
            by_var.entry(node.var).or_default().push(*node);
        }

        let mut graph = String::new();
        graph += "digraph G {\n";
        graph += "\"1\" [shape = \"box\"];\n";
        graph += "\"0\" [shape = \"box\"];\n";

        let mut edges = String::new();
        for (var, nodes) in by_var {
            if var == ZERO.var {
                continue;
            }
            graph += format!("subgraph cluster_{} {{\nrank=same;\n", var.0).as_str();
            for node in nodes {
                graph +=
                    format!("\"{}\" [label=\"{}\\n{}\"]\n", node.id.0, var.0, node.id.0).as_str();
                edges += format!(
                    "\"{}\" -> \"{}\" [style = \"dotted\"]\n",
                    node.id.0, node.low.0
                )
                .as_str();
                edges += format!("\"{}\" -> \"{}\"\n", node.id.0, node.high.0).as_str();
            }
            graph += "}\n\n";
        }

        graph += edges.as_str();
        graph += "}\n";
        graph
    }
}
