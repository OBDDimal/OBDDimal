use crate::bdd_node::{NodeID, VarID};
use crate::dimacs::Instance;

use super::order::check_order;
use super::{align_clauses, DDManager};

use std::io;
use std::io::Write;

impl DDManager {
    pub fn from_instance(
        instance: &mut Instance,
        order: Option<Vec<u32>>,
        enable_dvo: bool,
    ) -> Result<(DDManager, NodeID), String> {
        let mut man = DDManager::default();
        let clause_order = align_clauses(&instance.clauses);
        if let Some(o) = order {
            check_order(instance, &o)?;
            man.order = o;
        }

        let mut bdd = man.one();

        let mut n = 1;
        for i in clause_order.iter() {
            let clause = &instance.clauses[*i];

            log::info!("Integrating clause: {:?}", clause);

            let mut cbdd = man.zero();
            for x in clause {
                let node = if *x < 0_i32 {
                    man.nith_var(VarID(-x as u32))
                } else {
                    man.ith_var(VarID(*x as u32))
                };

                cbdd = man.or(node, cbdd);
            }

            bdd = man.and(cbdd, bdd);

            log::info!(
                "Nr. Nodes: {:?} ({:?}/{:?} clauses integrated)",
                &man.nodes.len(),
                n,
                &instance.clauses.len()
            );

            if enable_dvo {
                print!("DVO... ");
                io::stdout().flush().unwrap();

                let mut last_size = man.count_active(bdd);
                loop {
                    bdd = man.sift_all_vars(bdd);
                    let new_size = man.count_active(bdd);
                    if new_size == last_size {
                        break;
                    }
                    last_size = new_size;
                }
            }

            print!("Purge retain... ");
            io::stdout().flush().unwrap();
            man.purge_retain(bdd);
            println!("{} nodes remain", man.nodes.len());
            println!("{:?}", man);

            n += 1;
        }
        Ok((man, bdd))
    }
}
