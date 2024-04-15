use std::fs;

use crate::{
    core::{bdd_manager::DDManager, bdd_node::NodeID},
    store::common::{BddFile, Statistics},
};

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
        Self::load_from_bdd_file_object_with_statistics(bdd_file)
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
    /// [`NodeStatistics`](NodeStatistics).
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
        toml::to_string(&self.generate_bdd_file_object(roots, statistics))
            .map_err(|e| e.to_string())
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
        store::common::NodeStatistics,
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
