use std::fs;

use crate::{
    core::{bdd_manager::DDManager, bdd_node::NodeID},
    store::common::{BddFile, Statistics},
};

impl DDManager {
    /// Reads a (multi-rooted) BDD and corresponding statistics from a .bdd.xml file.
    ///
    /// * `filename` - Name of the .bdd.xml file.
    pub fn load_from_bdd_file_xml_with_statistics(
        filename: String,
    ) -> Result<(DDManager, Vec<NodeID>, Statistics), String> {
        let bdd_file: BddFile =
            serde_xml_rs::from_str(&fs::read_to_string(filename).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        Self::load_from_bdd_file_object_with_statistics(bdd_file)
    }

    /// Reads a (multi-rooted) BDD from a .bdd.xml file.
    ///
    /// * `filename` - Name of the .bdd.xml file.
    #[inline]
    pub fn load_from_bdd_file_xml(filename: String) -> Result<(DDManager, Vec<NodeID>), String> {
        let (ddmanager, roots, _) = Self::load_from_bdd_file_with_statistics(filename)?;
        Ok((ddmanager, roots))
    }

    /// Writes a (multi-rooted) BDD to a .bdd.xml file.
    ///
    /// * `self` - The DDManager containing the BDD.
    /// * `filename` - Name of the .bdd.xml file.
    /// * `roots` - The roots of the BDD.
    /// * `statistics` - Optional HashMap containing statistics about individual nodes, see
    /// [`NodeStatistics`](NodeStatistics).
    pub fn write_to_bdd_file_xml(
        &self,
        filename: String,
        roots: Vec<NodeID>,
        statistics: Statistics,
    ) -> Result<(), String> {
        fs::write(
            filename,
            self.generate_bdd_file_xml_string(roots, statistics)?,
        )
        .map_err(|e| e.to_string())
    }

    fn generate_bdd_file_xml_string(
        &self,
        roots: Vec<NodeID>,
        statistics: Statistics,
    ) -> Result<String, String> {
        serde_xml_rs::to_string(&self.generate_bdd_file_object(roots, statistics))
            .map_err(|e| e.to_string())
    }
}
