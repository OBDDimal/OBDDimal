use crate::bdd_manager::order_to_layernames;
use crate::bdd_manager::ZERO;
use crate::bdd_node::NodeID;
use crate::bdd_node::VarID;

use super::DDManager;

impl DDManager {
    /// Find the variable at specified level
    fn var_at_level(&self, level: u32) -> Option<VarID> {
        self.order
            .iter()
            .enumerate()
            .find(|(_, &l)| l == level)
            .map(|(v, _)| VarID(v as u32))
    }

    #[allow(unused)]
    fn sift_single_var(&mut self, var: VarID, mut f: NodeID) -> NodeID {
        let starting_pos = self.order[var.0 as usize];

        let mut best_position = starting_pos;
        let mut best_graphsize = self.count_active(f);

        log::info!(
            "Sifting variable {:?}, starting from level {} (graph size {}).",
            var,
            starting_pos,
            best_graphsize
        );

        // Move variable to the bottom
        let terminal_node_level = self.order[ZERO.var.0 as usize];

        log::info!("Moving down...");
        for level in starting_pos + 1..terminal_node_level {
            log::info!("Trying level {}", level);
            // Swap var at level-1 (our variable) with var at level
            f = self.swap(
                self.var_at_level(level - 1).unwrap(),
                self.var_at_level(level).unwrap(),
                f,
            );

            let new_size = self.count_active(f);
            log::info!(" Size is {}", new_size);

            if new_size < best_graphsize {
                log::info!(
                    " New optimum found with order {:?}",
                    order_to_layernames(&self.order)
                );
                best_graphsize = new_size;
                best_position = level;
            }
        }

        // Level is now bottom (terminal-1). Move variable to the top
        log::info!("Moving up...");

        for level in (1..terminal_node_level - 1).rev() {
            log::info!("Trying level {}", level);
            // Swap var at level+1 (our variable) with var at level
            f = self.swap(
                self.var_at_level(level).unwrap(),
                self.var_at_level(level + 1).unwrap(),
                f,
            );

            let new_size = self.count_active(f);
            log::info!(" Size is {}", new_size);

            if new_size < best_graphsize {
                log::info!(
                    " New optimum found with order {:?}",
                    order_to_layernames(&self.order)
                );
                best_graphsize = new_size;
                best_position = level;
            }
        }

        // Level is now top (1). Move variable down to best location

        log::info!(
            "The best result was graph size of {} at level {}. Moving there...",
            best_graphsize,
            best_position
        );

        for level in 2..best_position + 1 {
            // Swap var at level-1 (our variable) with var at level
            f = self.swap(
                self.var_at_level(level - 1).unwrap(),
                self.var_at_level(level).unwrap(),
                f,
            );
        }

        log::info!("Size is now  {}", self.count_active(f));

        f
    }

    #[must_use]
    #[allow(unused)]
    pub fn sift_all_vars(&mut self, mut f: NodeID) -> NodeID {
        for v in 1..self.var2nodes.len() {
            if self.var2nodes[v].is_empty() {
                continue;
            }

            let var = VarID(v as u32);
            f = self.sift_single_var(var, f);
            self.purge_retain(f);
        }
        f
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use num_bigint::BigUint;

    use crate::{
        bdd_manager::{order_to_layernames, DDManager},
        bdd_node::VarID,
        dimacs,
    };

    #[test]
    fn sift_sandwich_single() {
        let _ = env_logger::builder().is_test(true).try_init();

        let expected = BigUint::parse_bytes(b"2808", 10).unwrap();

        // Build BDD
        let mut instance = dimacs::parse_dimacs("examples/sandwich.dimacs");
        let (mut man, bdd) = DDManager::from_instance(&mut instance, None).unwrap();
        assert_eq!(man.sat_count(bdd), expected);

        let size_before = man.count_active(bdd);
        println!("Size before sifting: {}", size_before);
        let bdd = man.sift_single_var(VarID(2), bdd);
        let size_after = man.count_active(bdd);
        println!("Size after sifting: {}", size_after);
        println!("Order after sifting: {:?}", order_to_layernames(&man.order));

        assert_eq!(man.sat_count(bdd), expected);
        assert!(size_after <= size_before);
    }

    #[test]
    fn sift_sandwich_all() {
        let _ = env_logger::builder().is_test(true).try_init();

        let expected = BigUint::parse_bytes(b"2808", 10).unwrap();

        // Build BDD
        let mut instance = dimacs::parse_dimacs("examples/sandwich.dimacs");
        let (mut man, bdd) = DDManager::from_instance(&mut instance, None).unwrap();
        assert_eq!(man.sat_count(bdd), expected);

        let size_before = man.count_active(bdd);
        println!("Size before sifting: {}", size_before);
        let bdd = man.sift_all_vars(bdd);
        let size_after = man.count_active(bdd);
        println!("Size after sifting: {}", size_after);
        println!("Order after sifting: {:?}", order_to_layernames(&man.order));
        fs::write("after.dot", man.graphviz(bdd)).unwrap();

        assert_eq!(man.sat_count(bdd), expected);
        assert!(size_after <= size_before);
        // TODO: Check if function is actually the same
    }
}
