use std::fs;

use serde::{Deserialize, Serialize};

use crate::{
    core::{
        bdd_manager::DDManager,
        bdd_node::{DDNode, NodeID, VarID},
    },
    misc::hash_select::HashMap,
};

type Statistics = Option<HashMap<NodeID, NodeStatistics>>;

#[derive(Serialize, Deserialize)]
struct BddFile {
    statistics: Option<HashMap<String, NodeStatistics>>,
    bdd: Bdd,
}

#[derive(Serialize, Deserialize)]
pub struct NodeStatistics {
    void: Option<bool>,
    count: Option<usize>,
}

#[derive(Serialize, Deserialize)]
struct Bdd {
    order: Vec<usize>,
    roots: Vec<NodeID>,
    // node format:
    // id_node id_var id_high id_low
    nodes: String,
}

impl DDManager {
    /// Reads a (multi-rooted) BDD and corresponding statistics from a .bdd file.
    ///
    /// * `filename` - Name of the .bdd file.
    ///
    /// ```
    /// # use obddimal::core::bdd_manager::DDManager;
    /// let (man, bdds, statistics) = DDManager::load_from_bdd_file_with_statistics("examples/simple.bdd".to_string()).unwrap();
    /// ```
    pub fn load_from_bdd_file_with_statistics(
        filename: String,
    ) -> Result<(DDManager, Vec<NodeID>, Statistics), String> {
        let bdd_file: BddFile =
            toml::from_str(&fs::read_to_string(filename).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
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

    /// Reads a (multi-rooted) BDD from a .bdd file.
    ///
    /// * `filename` - Name of the .bdd file.
    ///
    /// ```
    /// # use obddimal::core::bdd_manager::DDManager;
    /// let (man, bdds) = DDManager::load_from_bdd_file("examples/simple.bdd".to_string()).unwrap();
    /// ```
    #[inline]
    pub fn load_from_bdd_file(filename: String) -> Result<(DDManager, Vec<NodeID>), String> {
        let (ddmanager, roots, _) = Self::load_from_bdd_file_with_statistics(filename)?;
        Ok((ddmanager, roots))
    }

    /// Writes a (multi-rooted) BDD to a .bdd file.
    ///
    /// * `self` - The DDManager containing the BDD.
    /// * `filename` - Name of the .bdd file.
    /// * `roots` - The roots of the BDD.
    /// * `statistics` - Optional HashMap containing statistics about individual nodes, see
    /// [`NodeStatistics`].
    pub fn write_to_bdd_file(
        &self,
        filename: String,
        roots: Vec<NodeID>,
        statistics: Statistics,
    ) -> Result<(), String> {
        fs::write(filename, self.generate_bdd_file_string(roots, statistics)?)
            .map_err(|e| e.to_string())
    }

    fn generate_bdd_file_string(
        &self,
        roots: Vec<NodeID>,
        statistics: Statistics,
    ) -> Result<String, String> {
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
        let bdd_file = BddFile {
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
        };
        toml::to_string(&bdd_file).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod test {
    use std::fs;

    use crate::{
        core::{
            bdd_manager::DDManager,
            bdd_node::{DDNode, NodeID, VarID},
        },
        misc::hash_select::HashMap,
        store::bdd::NodeStatistics,
    };

    #[test]
    fn bdd_file_read_simple() {
        let (man, bdds, statistics) =
            DDManager::load_from_bdd_file_with_statistics("examples/simple.bdd".to_string())
                .unwrap();
        let statistics_4 = statistics.as_ref().unwrap().get(&bdds[0]).unwrap();
        let statistics_2 = statistics.as_ref().unwrap().get(&bdds[1]).unwrap();
        assert!(!statistics_4.void.unwrap());
        assert_eq!(statistics_4.count.unwrap(), 10usize);
        assert!(!statistics_2.void.unwrap());
        assert_eq!(man.sat_count(bdds[0]), 10usize.into());
    }

    #[test]
    fn bdd_file_write_simple() {
        let bdds = vec![NodeID(4), NodeID(2)];
        let mut man = DDManager::default();
        man.prepare_varorder(vec![5, 1, 2, 3, 4]);
        [
            DDNode {
                id: NodeID(2),
                var: VarID(3),
                low: NodeID(0),
                high: NodeID(1),
            },
            DDNode {
                id: NodeID(3),
                var: VarID(2),
                low: NodeID(2),
                high: NodeID(1),
            },
            DDNode {
                id: NodeID(4),
                var: VarID(1),
                low: NodeID(2),
                high: NodeID(3),
            },
        ]
        .iter()
        .for_each(|node| {
            man.nodes.insert(node.id, *node);
            man.level2nodes[man.var2level[node.var.0]].insert(*node);
        });
        let statistics = HashMap::from(
            [
                (
                    NodeID(4),
                    NodeStatistics {
                        void: Some(false),
                        count: Some(10),
                    },
                ),
                (
                    NodeID(2),
                    NodeStatistics {
                        void: Some(false),
                        count: None,
                    },
                ),
            ]
            .into_iter()
            .collect(),
        );

        assert_eq!(
            collapse::collapse(
                &man.generate_bdd_file_string(bdds, Some(statistics))
                    .unwrap()
            ),
            collapse::collapse(&fs::read_to_string("examples/simple.bdd").unwrap())
        );
    }
}
