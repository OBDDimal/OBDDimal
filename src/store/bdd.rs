use std::{fs, sync::Arc};

use crate::{
    core::{bdd_manager::DDManager, bdd_node::NodeID},
    store::common::{BddFile, LoadResult, LoadResultWithStatistics, Statistics},
    views::bdd_view::BddView,
};

impl DDManager {
    /// Reads a (multi-rooted) BDD and corresponding statistics from a .bdd file.
    ///
    /// * `filename` - Name of the .bdd file.
    ///
    /// ```
    /// # use obddimal::core::bdd_manager::DDManager;
    /// let (man, bdds, views, statistics) = DDManager::load_from_bdd_file_with_statistics("examples/simple.bdd".to_string()).unwrap();
    /// ```
    pub fn load_from_bdd_file_with_statistics(filename: String) -> LoadResultWithStatistics {
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
    /// let (man, bdds, views) = DDManager::load_from_bdd_file("examples/simple.bdd".to_string()).unwrap();
    /// ```
    #[inline]
    pub fn load_from_bdd_file(filename: String) -> LoadResult {
        let (ddmanager, roots, views, _) = Self::load_from_bdd_file_with_statistics(filename)?;
        Ok((ddmanager, roots, views))
    }

    /// Writes a (multi-rooted) BDD and all its views to a .bdd file.
    ///
    /// * `self` - The DDManager containing the BDD.
    /// * `filename` - Name of the .bdd file.
    /// * `statistics` - Optional HashMap containing statistics about individual nodes, see
    ///   [`NodeStatistics`](NodeStatistics).
    pub fn write_all_views_to_bdd_file(
        &self,
        filename: String,
        statistics: Statistics,
    ) -> Result<(), String> {
        self.write_to_bdd_file(
            filename,
            self.get_roots(),
            Some(self.get_views()),
            statistics,
        )
    }

    /// Writes a (multi-rooted) BDD to a .bdd file.
    ///
    /// * `self` - The DDManager containing the BDD.
    /// * `filename` - Name of the .bdd file.
    /// * `roots` - The roots of the BDD.
    /// * `views` - Optionally the views to store for the BDDs.
    /// * `statistics` - Optional HashMap containing statistics about individual nodes, see
    ///   [`NodeStatistics`](NodeStatistics).
    pub fn write_to_bdd_file(
        &self,
        filename: String,
        roots: Vec<NodeID>,
        views: Option<Vec<Arc<BddView>>>,
        statistics: Statistics,
    ) -> Result<(), String> {
        fs::write(
            filename,
            self.generate_bdd_file_string(roots, views, statistics)?,
        )
        .map_err(|e| e.to_string())
    }

    fn generate_bdd_file_string(
        &self,
        roots: Vec<NodeID>,
        views: Option<Vec<Arc<BddView>>>,
        statistics: Statistics,
    ) -> Result<String, String> {
        toml::to_string(&self.generate_bdd_file_object(roots, views, statistics))
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod test {
    use std::{
        fs,
        sync::{Arc, RwLock},
    };

    use crate::{
        core::{
            bdd_manager::DDManager,
            bdd_node::{DDNode, NodeID, VarID},
        },
        misc::hash_select::HashMap,
        store::common::NodeStatistics,
        views::bdd_view::BddView,
    };

    #[test]
    fn bdd_file_read_simple() {
        let (man, bdds, views, statistics) =
            DDManager::load_from_bdd_file_with_statistics("examples/simple.bdd".to_string())
                .unwrap();
        let statistics_4 = statistics.as_ref().unwrap().get(&bdds[0]).unwrap();
        let statistics_2 = statistics.as_ref().unwrap().get(&bdds[1]).unwrap();

        assert!(!statistics_4.void.unwrap());
        assert_eq!(statistics_4.count.unwrap(), 10usize);
        assert!(!statistics_2.void.unwrap());

        assert_eq!(man.read().unwrap().sat_count(bdds[0]), 10usize.into());
        assert_eq!(man.read().unwrap().sat_count(bdds[1]), 2usize.into());

        assert_eq!(views.as_ref().unwrap()[0].sat_count(), 2usize.into());
        assert_eq!(views.as_ref().unwrap()[1].sat_count(), 10usize.into());
        assert_eq!(views.unwrap()[2].sat_count(), 5usize.into());
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

        let man: Arc<RwLock<DDManager>> = RwLock::new(man).into();

        let views: Vec<Arc<BddView>> = vec![
            BddView::new(NodeID(2), man.clone()),
            BddView::new(NodeID(4), man.clone()),
            BddView::new_with_sliced(NodeID(4), man.clone(), vec![VarID(4)].into_iter().collect()),
        ];

        assert_eq!(
            collapse::collapse(
                &man.read()
                    .unwrap()
                    .generate_bdd_file_string(bdds, Some(views), Some(statistics))
                    .unwrap()
            ),
            collapse::collapse(&fs::read_to_string("examples/simple.bdd").unwrap())
        );
    }
}
