use std::{fs, sync::Arc};

use crate::{
    core::{bdd_manager::DDManager, bdd_node::NodeID},
    store::common::{BddFile, LoadResult, LoadResultWithStatistics, Statistics},
    views::bdd_view::BddView,
};

impl DDManager {
    /// Reads a (multi-rooted) BDD and corresponding statistics from a .bdd.json file.
    ///
    /// * `filename` - Name of the .bdd.json file.
    pub fn load_from_bdd_file_json_with_statistics(filename: String) -> LoadResultWithStatistics {
        let bdd_file: BddFile =
            serde_json::from_str(&fs::read_to_string(filename).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        Self::load_from_bdd_file_object_with_statistics(bdd_file)
    }

    /// Reads a (multi-rooted) BDD from a .bdd.json file.
    ///
    /// * `filename` - Name of the .bdd.json file.
    #[inline]
    pub fn load_from_bdd_file_json(filename: String) -> LoadResult {
        let (ddmanager, roots, views, _) = Self::load_from_bdd_file_with_statistics(filename)?;
        Ok((ddmanager, roots, views))
    }

    /// Writes a (multi-rooted) BDD and all its views to a .bdd.json file.
    ///
    /// * `self` - The DDManager containing the BDD.
    /// * `filename` - Name of the .bdd.json file.
    /// * `statistics` - Optional HashMap containing statistics about individual nodes, see
    ///   [`NodeStatistics`](NodeStatistics).
    pub fn write_all_views_to_bdd_file_json(
        &self,
        filename: String,
        statistics: Statistics,
    ) -> Result<(), String> {
        self.write_to_bdd_file_json(
            filename,
            self.get_roots(),
            Some(self.get_views()),
            statistics,
        )
    }

    /// Writes a (multi-rooted) BDD to a .bdd.json file.
    ///
    /// * `self` - The DDManager containing the BDD.
    /// * `filename` - Name of the .bdd.json file.
    /// * `roots` - The roots of the BDD.
    /// * `views` - Optionally the views to store for the BDDs.
    /// * `statistics` - Optional HashMap containing statistics about individual nodes, see
    ///   [`NodeStatistics`](NodeStatistics).
    pub fn write_to_bdd_file_json(
        &self,
        filename: String,
        roots: Vec<NodeID>,
        views: Option<Vec<Arc<BddView>>>,
        statistics: Statistics,
    ) -> Result<(), String> {
        fs::write(
            filename,
            self.generate_bdd_file_json_string(roots, views, statistics)?,
        )
        .map_err(|e| e.to_string())
    }

    fn generate_bdd_file_json_string(
        &self,
        roots: Vec<NodeID>,
        views: Option<Vec<Arc<BddView>>>,
        statistics: Statistics,
    ) -> Result<String, String> {
        serde_json::to_string(&self.generate_bdd_file_object(roots, views, statistics))
            .map_err(|e| e.to_string())
    }
}
