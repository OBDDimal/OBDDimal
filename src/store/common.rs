use crate::{
    core::{
        bdd_manager::DDManager,
        bdd_node::{NodeID, VarID},
    },
    misc::hash_select::HashMap,
};

impl DDManager {
    pub fn load_bdd_from_nodetable(
        &mut self,
        _nodes: &HashMap<NodeID, (VarID, NodeID, NodeID)>,
        _varorder: &[VarID],
    ) {
        // TODO prepare DDManager (ordering, …)

        // TODO create nodes in DDManager (bottom up)
        todo!();
    }
}
