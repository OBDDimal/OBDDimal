//! Implementation of dynamic variable ordering techniques
#![allow(rustdoc::private_intra_doc_links)]

pub mod dvo_schedules;

use std::io::stdout;

use crossterm::{cursor, execute};
use indicatif::ProgressBar;

use super::{
    bdd_manager::{DDManager, ZERO},
    bdd_node::{NodeID, VarID},
    order::order_to_layernames,
};
use crate::if_some;

impl DDManager {
    /// Find the variable at specified level
    fn var_at_level(&self, level: usize) -> Option<VarID> {
        self.var2level
            .iter()
            .enumerate()
            .find(|(_, &l)| l == level)
            .map(|(v, _)| VarID(v))
    }

    /// Swap layer containing specified variable first to the bottom of the BDD, then to the top,
    /// and then to the position which resulted in smallest BDD size.
    /// Optional parameter `max_increase` stops swapping in either direction once the difference
    /// between BDD size and current optimum exceeds threshold.
    #[allow(unused)]
    fn sift_single_var(
        &mut self,
        var: VarID,
        max_increase: Option<usize>,
        mut f: NodeID,
    ) -> NodeID {
        let starting_pos = self.var2level[var.0];

        let mut best_position = starting_pos;
        let mut best_graphsize = self.count_active(f);

        log::info!(
            "Sifting variable {:?}, starting from level {} (graph size {}).",
            var,
            starting_pos,
            best_graphsize
        );

        // Move variable to the bottom
        let terminal_node_level = self.var2level[ZERO.var.0];

        let mut current_level = starting_pos;

        log::info!("Moving down...");

        for level in starting_pos + 1..terminal_node_level {
            log::info!("Trying level {}", level);
            // Swap var at level-1 (our variable) with var at level
            f = self.swap(
                self.var_at_level(level - 1).unwrap(),
                self.var_at_level(level).unwrap(),
                f,
            );
            current_level += 1;

            let new_size = self.count_active(f);
            log::info!(" Size is {}", new_size);

            if new_size < best_graphsize {
                log::info!(
                    " New optimum found with order {:?}",
                    order_to_layernames(&self.var2level)
                );
                self.purge_retain(f);
                best_graphsize = new_size;
                best_position = level;
            } else if let Some(max) = max_increase {
                if new_size > best_graphsize + max {
                    // Do not continue moving downwards, because the graph has grown too much
                    break;
                }
            }
        }

        // Level is now bottom (terminal-1).
        log::info!("Moving up...");

        // Swap back to initial position, without calculating size
        for level in (starting_pos..current_level).rev() {
            f = self.swap(
                self.var_at_level(level).unwrap(),
                self.var_at_level(level + 1).unwrap(),
                f,
            );
            current_level -= 1;
        }

        assert_eq!(current_level, starting_pos);

        // Move variable to the top
        for level in (1..starting_pos).rev() {
            log::info!("Trying level {}", level);
            // Swap var at level+1 (our variable) with var at level
            f = self.swap(
                self.var_at_level(level).unwrap(),
                self.var_at_level(level + 1).unwrap(),
                f,
            );
            current_level -= 1;

            let new_size = self.count_active(f);
            log::info!(" Size is {}", new_size);

            if new_size < best_graphsize {
                log::info!(
                    " New optimum found with order {:?}",
                    order_to_layernames(&self.var2level)
                );
                self.purge_retain(f);
                best_graphsize = new_size;
                best_position = level;
            } else if let Some(max) = max_increase {
                if new_size > best_graphsize + max {
                    // Do not continue moving upwards, because the graph has grown too much
                    break;
                }
            }
        }

        // Level is now top (1). Move variable down to best location

        log::info!(
            "The best result was graph size of {} at level {}. Moving there...",
            best_graphsize,
            best_position
        );

        for level in current_level + 1..best_position + 1 {
            // Swap var at level-1 (our variable) with var at level
            f = self.swap(
                self.var_at_level(level - 1).unwrap(),
                self.var_at_level(level).unwrap(),
                f,
            );
            current_level += 1;
        }

        assert_eq!(current_level, best_position);

        log::info!("Size is now  {}", self.count_active(f));

        f
    }

    /// Perform sifting for every layer containing at least one variable.
    /// If `progressbar` is `true`, display a progress bar in the terminal
    /// which shows the number of layers already processed.
    /// See [Self::sift_single_var()] for `max_increase` parameter.
    #[must_use]
    #[allow(unused)]
    pub fn sift_all_vars(
        &mut self,
        mut f: NodeID,
        progressbar: bool,
        max_increase: Option<usize>,
    ) -> NodeID {
        let bar = if progressbar {
            Some(ProgressBar::new(self.level2nodes.len() as u64 - 1))
        } else {
            None
        };

        for v in (1..self.var2level.len()) {
            if_some!(bar, inc(1));

            if self.level2nodes[self.var2level[v]].is_empty() {
                continue;
            }

            let var = VarID(v);
            f = self.sift_single_var(var, max_increase, f);
            self.purge_retain(f);
        }
        if_some!(bar, finish_and_clear());

        if bar.is_some() {
            // Move cursor up to continue updating the top-level progress bar
            execute!(stdout(), cursor::MoveToPreviousLine(1));
        }
        f
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use num_bigint::BigUint;

    use crate::core::{bdd_manager::DDManager, bdd_node::VarID, order::order_to_layernames};

    #[test]
    fn sift_sandwich_single() {
        let _ = env_logger::builder().is_test(true).try_init();

        let expected = BigUint::parse_bytes(b"2808", 10).unwrap();

        // Build BDD
        let mut instance = dimacs::parse_dimacs(
            &fs::read_to_string("examples/sandwich.dimacs").expect("Failed to read dimacs file."),
        )
        .expect("Failed to parse dimacs file.");
        let (mut man, bdd) =
            DDManager::from_instance(&mut instance, None, Default::default()).unwrap();
        assert_eq!(man.sat_count(bdd), expected);

        let size_before = man.count_active(bdd);
        println!("Size before sifting: {}", size_before);
        let bdd = man.sift_single_var(VarID(2), None, bdd);
        let size_after = man.count_active(bdd);
        println!("Size after sifting: {}", size_after);
        println!(
            "Order after sifting: {:?}",
            order_to_layernames(&man.var2level)
        );

        assert_eq!(man.sat_count(bdd), expected);
        assert!(size_after <= size_before);
    }

    #[test]
    fn sift_sandwich_all() {
        let _ = env_logger::builder().is_test(true).try_init();

        let expected = BigUint::parse_bytes(b"2808", 10).unwrap();

        // Build BDD
        let mut instance = dimacs::parse_dimacs(
            &fs::read_to_string("examples/sandwich.dimacs").expect("Failed to read dimacs file."),
        )
        .expect("Failed to parse dimacs file.");
        let (mut man, bdd) =
            DDManager::from_instance(&mut instance, None, Default::default()).unwrap();
        assert_eq!(man.sat_count(bdd), expected);

        let size_before = man.count_active(bdd);
        println!("Size before sifting: {}", size_before);
        let bdd = man.sift_all_vars(bdd, false, None);
        let size_after = man.count_active(bdd);
        println!("Size after sifting: {}", size_after);
        println!(
            "Order after sifting: {:?}",
            order_to_layernames(&man.var2level)
        );
        fs::write("after.dot", man.graphviz(bdd)).unwrap();

        assert_eq!(man.sat_count(bdd), expected);
        assert!(size_after <= size_before);
        // TODO: Check if function is actually the same
    }
}
