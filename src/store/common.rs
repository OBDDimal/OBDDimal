use serde::{Deserialize, Serialize};

use crate::{
    core::{
        bdd_manager::DDManager,
        bdd_node::{DDNode, NodeID, VarID},
    },
    misc::hash_select::{HashMap, HashSet},
};

pub(super) type Statistics = Option<HashMap<NodeID, NodeStatistics>>;

#[derive(Serialize, Deserialize)]
pub(super) struct BddFile {
    statistics: Option<HashMap<String, NodeStatistics>>,
    bdd: Bdd,
}

#[derive(Serialize, Deserialize)]
pub struct NodeStatistics {
    pub void: Option<bool>,
    pub count: Option<usize>,
}

#[derive(Serialize, Deserialize)]
pub(super) struct Bdd {
    order: Vec<usize>,
    roots: Vec<NodeID>,
    // node format:
    // id_node id_var id_high id_low
    nodes: String,
}

impl DDManager {
    /// Reads a (multi-rooted) BDD and corresponding statistics from a bdd file object.
    ///
    /// * `bdd_file` - The BddFile object.
    pub(super) fn load_from_bdd_file_object_with_statistics(
        bdd_file: BddFile,
    ) -> Result<(DDManager, Vec<NodeID>, Statistics), String> {
        let varorder: Vec<usize> = bdd_file.bdd.order;
        let roots: Vec<NodeID> = bdd_file.bdd.roots;

        let mut terminals: (Option<NodeID>, Option<NodeID>) = (None, None);
        let nodes = bdd_file
            .bdd
            .nodes
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                let line: Vec<_> = line.split_whitespace().map(str::to_string).collect();
                if line.len() != 4 {
                    Err("Node list contains unexpected line!".to_string())
                } else {
                    let id = NodeID(line[0].parse::<usize>().map_err(|e| e.to_string())?);
                    let varid = VarID(line[1].parse::<usize>().map_err(|e| e.to_string())?);
                    let high = NodeID(line[2].parse::<usize>().map_err(|e| e.to_string())?);
                    let low = NodeID(line[3].parse::<usize>().map_err(|e| e.to_string())?);
                    if high == low {
                        if high == NodeID(1) {
                            terminals.0 = Some(id);
                        } else if high == NodeID(0) {
                            terminals.1 = Some(id);
                        } else {
                            return Err("Invalid terminal node!".to_string());
                        }
                    }
                    Ok((id, (varid, high, low)))
                }
            })
            .try_collect::<HashMap<NodeID, (VarID, NodeID, NodeID)>>()?;

        // Check if root nodes are valid:
        for r in roots.iter() {
            if !nodes.contains_key(r) {
                return Err("Root node not existant in BDD!".to_string());
            }
        }

        // Check if terminal nodes exist:
        let terminals = match terminals {
            (Some(t_high), Some(t_low)) => Ok((t_high, t_low)),
            _ => Err("Terminal nodes missing!".to_string()),
        }?;

        let (ddmanager, roots, id_translator) = DDManager::default()
            .load_bdd_from_nodelist_with_translation(nodes, varorder, roots, terminals);

        // Change node ids in statistic:
        let statistics = bdd_file
            .statistics
            .map(|statistics| {
                statistics
                    .into_iter()
                    .map(|(node_id, stats)| {
                        let node_id =
                            NodeID(node_id.parse::<usize>().map_err(|err| err.to_string())?);
                        Ok::<_, String>((*id_translator.get(&node_id).unwrap(), stats))
                    })
                    .try_collect()
            })
            .transpose()?;

        Ok((ddmanager, roots, statistics))
    }

    pub(super) fn generate_bdd_file_object(
        &self,
        roots: Vec<NodeID>,
        statistics: Statistics,
    ) -> BddFile {
        let order = self.var2level.clone();
        let nodes = self
            .nodes
            .values()
            .map(
                |DDNode {
                     id: NodeID(n),
                     var: VarID(v),
                     low: NodeID(l),
                     high: NodeID(h),
                     ..
                 }| format!("{n} {v} {h} {l}"),
            )
            .fold("".to_string(), |mut s, line| {
                s.push('\n');
                s.push_str(&line);
                s
            });
        BddFile {
            statistics: statistics.map(|statistics| {
                statistics
                    .into_iter()
                    .map(|(node_id, stats)| (node_id.0.to_string(), stats))
                    .collect()
            }),
            bdd: Bdd {
                order,
                roots,
                nodes,
            },
        }
    }

    /// Loads a BDD from a Nodelist (containing all nodes from a BDD) into the DDManager.
    ///
    /// # Panics
    /// Only allowed on empty DDManagers. If called on a non-empty DDManager, this function will
    /// panic!
    #[inline]
    pub(super) fn load_bdd_from_nodelist(
        self,
        nodes: HashMap<NodeID, (VarID, NodeID, NodeID)>,
        varorder: Vec<usize>,
        roots: Vec<NodeID>,
        terminals: (NodeID, NodeID),
    ) -> (DDManager, Vec<NodeID>) {
        let (man, roots, _) =
            self.load_bdd_from_nodelist_with_translation(nodes, varorder, roots, terminals);
        (man, roots)
    }

    /// Loads a BDD from a Nodelist (containing all nodes from a BDD) into the DDManager.
    ///
    /// # Panics
    /// Only allowed on empty DDManagers. If called on a non-empty DDManager, this function will
    /// panic!
    pub(super) fn load_bdd_from_nodelist_with_translation(
        mut self,
        nodes: HashMap<NodeID, (VarID, NodeID, NodeID)>,
        varorder: Vec<usize>,
        roots: Vec<NodeID>,
        terminals: (NodeID, NodeID),
    ) -> (DDManager, Vec<NodeID>, HashMap<NodeID, NodeID>) {
        assert!(
            self.nodes.len() == 2, // The terminal nodes already exist in a new DDManager
            "load_bdd_from_nodelist and load_bdd_from_nodelist_with_translation are only allowed on empty DDManagers."
        );

        // Prepare DDManager
        let mut new_ids: HashMap<NodeID, NodeID> = HashMap::default();
        new_ids.insert(terminals.0, self.one());
        new_ids.insert(terminals.1, self.zero());

        let layer_to_nodes: HashMap<usize, HashSet<NodeID>> = nodes
            .iter()
            .map(|(n, (v, _, _))| (varorder[v.0], n))
            .fold(HashMap::default(), |mut layer_to_nodes, (l, n)| {
                if let Some(nodes) = layer_to_nodes.get_mut(&l) {
                    nodes.insert(*n);
                } else {
                    let mut nodes = HashSet::default();
                    nodes.insert(*n);
                    layer_to_nodes.insert(l, nodes);
                }
                layer_to_nodes
            });

        self.prepare_varorder(varorder.clone());

        // Create nodes in DDManager (bottom up)
        let mut layers = varorder;
        layers.sort_unstable();
        layers.reverse();
        layers
            .iter()
            .filter(|layer| layer_to_nodes.contains_key(layer))
            .flat_map(|layer| layer_to_nodes.get(layer).unwrap())
            .filter(|node_id| **node_id != terminals.0 && **node_id != terminals.1)
            .for_each(|node_id| {
                let (var, high, low) = nodes.get(node_id).unwrap();
                let new_id = self.node_get_or_create(&DDNode {
                    id: NodeID(0),
                    var: *var,
                    low: *new_ids.get(low).unwrap(),
                    high: *new_ids.get(high).unwrap(),
                });
                new_ids.insert(*node_id, new_id);
            });

        // Convert root ids
        let roots: Vec<NodeID> = roots.iter().map(|r| *new_ids.get(r).unwrap()).collect();

        (self, roots, new_ids)
    }
}
