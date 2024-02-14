use std::fs;

use serde::{Deserialize, Serialize};

use crate::core::{bdd_manager::DDManager, bdd_node::NodeID};

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
    order: Vec<usize>,
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
        let _bdd_file: BddFile =
            toml::from_str(&fs::read_to_string(filename).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        todo!();
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
