use std::fs;

use serde::{Deserialize, Serialize};

use crate::{
    core::{
        bdd_manager::DDManager,
        bdd_node::{NodeID, VarID},
    },
    misc::hash_select::HashMap,
};

#[derive(Serialize, Deserialize)]
struct BddFile {
    statistics: Statistics,
    bdd: Bdd,
}

#[derive(Serialize, Deserialize)]
struct Statistics {
    void: bool,
    count: usize,
}

#[derive(Serialize, Deserialize)]
struct Bdd {
    order: Vec<VarID>,
    roots: Vec<NodeID>,
    // node format:
    // id_node id_var id_high id_low
    nodes: String,
}

impl DDManager {
    /// Reads a (multi-rooted) BDD from a .bdd file.
    ///
    /// * `filename` - Name of the .bdd file.
    ///
    /// ```
    /// # use obddimal::core::bdd_manager::DDManager;
    /// //let (man, bdds) = DDManager::load_from_bdd_file("testbdd.bdd".to_string()).unwrap();
    /// ```
    pub fn load_from_bdd_file(filename: String) -> Result<(DDManager, Vec<NodeID>), String> {
        let bdd_file: BddFile =
            toml::from_str(&fs::read_to_string(filename).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        let varorder: Vec<VarID> = bdd_file.bdd.order;
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

        let terminals = match terminals {
            (Some(t_high), Some(t_low)) => Ok((t_high, t_low)),
            _ => Err("Terminal nodes missing!".to_string()),
        }?;

        let mut bdd = DDManager::default();
        Ok(bdd.load_bdd_from_nodetable(&nodes, &varorder, &roots, &terminals))
    }

    /// Writes a (multi-rooted) BDD to a .bdd file.
    ///
    /// * `filename` - Name of the .bdd file.
    /// * `roots` - The roots of the BDD.
    ///
    pub fn write_to_bdd_file(filename: String, roots: Vec<NodeID>) -> Result<(), String> {
        let void = true; // TODO
        let count = 42; // TODO
        let order = Vec::new(); // TODO
        let nodes = "".to_string(); // TODO
        let bdd_file = BddFile {
            statistics: Statistics { void, count },
            bdd: Bdd {
                order,
                roots,
                nodes,
            },
        };
        fs::write(
            filename,
            toml::to_string_pretty(&bdd_file).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())
    }
}
