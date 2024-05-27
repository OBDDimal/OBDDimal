use std::{fs, sync::Arc};

use crate::{
    core::{bdd_manager::DDManager, bdd_node::NodeID},
    store::common::{BddFile, LoadResult, LoadResultWithStatistics, Statistics},
    views::bdd_view::BddView,
};

impl DDManager {
    /// Reads a (multi-rooted) BDD and corresponding statistics from a .bdd.xml file.
    ///
    /// * `filename` - Name of the .bdd.xml file.
    pub fn load_from_bdd_file_xml_with_statistics(filename: String) -> LoadResultWithStatistics {
        let bdd_file: BddFile =
            serde_xml_rs::from_str(&fs::read_to_string(filename).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        Self::load_from_bdd_file_object_with_statistics(bdd_file)
    }

    /// Reads a (multi-rooted) BDD from a .bdd.xml file.
    ///
    /// * `filename` - Name of the .bdd.xml file.
    #[inline]
    pub fn load_from_bdd_file_xml(filename: String) -> LoadResult {
        let (ddmanager, roots, views, _) = Self::load_from_bdd_file_with_statistics(filename)?;
        Ok((ddmanager, roots, views))
    }

    /// Writes a (multi-rooted) BDD and all its views to a .bdd.xml file.
    ///
    /// * `self` - The DDManager containing the BDD.
    /// * `filename` - Name of the .bdd.xml file.
    /// * `statistics` - Optional HashMap containing statistics about individual nodes, see
    ///   [`NodeStatistics`](NodeStatistics).
    pub fn write_all_views_to_bdd_file_xml(
        &self,
        filename: String,
        statistics: Statistics,
    ) -> Result<(), String> {
        self.write_to_bdd_file_xml(
            filename,
            self.get_roots(),
            Some(self.get_views()),
            statistics,
        )
    }

    /// Writes a (multi-rooted) BDD to a .bdd.xml file.
    ///
    /// * `self` - The DDManager containing the BDD.
    /// * `filename` - Name of the .bdd.xml file.
    /// * `roots` - The roots of the BDD.
    /// * `views` - Optionally the views to store for the BDDs.
    /// * `statistics` - Optional HashMap containing statistics about individual nodes, see
    ///   [`NodeStatistics`](NodeStatistics).
    pub fn write_to_bdd_file_xml(
        &self,
        filename: String,
        roots: Vec<NodeID>,
        views: Option<Vec<Arc<BddView>>>,
        statistics: Statistics,
    ) -> Result<(), String> {
        fs::write(
            filename,
            self.generate_bdd_file_xml_string(roots, views, statistics)?,
        )
        .map_err(|e| e.to_string())
    }

    fn generate_bdd_file_xml_string(
        &self,
        roots: Vec<NodeID>,
        views: Option<Vec<Arc<BddView>>>,
        statistics: Statistics,
    ) -> Result<String, String> {
        serde_xml_rs::to_string(&self.generate_bdd_file_object(roots, views, statistics))
            .map_err(|e| e.to_string())
    }
}
