use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};

use super::{options::Options, DDManager};
use crate::{
    bdd_manager::{align_clauses, order::check_order},
    bdd_node::{NodeID, VarID},
    dimacs::Instance,
    if_some,
};

impl DDManager {
    pub fn from_instance(
        instance: &mut Instance,
        order: Option<Vec<u32>>,

        options: Options,
    ) -> Result<(DDManager, NodeID), String> {
        let mut man = DDManager {
            options,
            ..Default::default()
        };

        let clause_order = align_clauses(&instance.clauses);
        if let Some(o) = order {
            check_order(instance, &o)?;
            man.order = o;
        }

        let mut bdd = man.one();

        let bar = if man.options.progressbars {
            let bar = ProgressBar::new(clause_order.len() as u64);
            // Explicitly set stdout draw target to avoid the frame-rate limit, which
            // unfortunately sometimes prevents a message update when we want it, such
            // as before starting DVO which is rather long-runnning.
            bar.set_draw_target(ProgressDrawTarget::term_like(Box::new(
                console::Term::stdout(),
            )));

            bar.set_style(
                ProgressStyle::with_template("[{elapsed_precise}] {wide_bar} {pos}/{len} {msg}")
                    .unwrap(),
            );
            Some(bar)
        } else {
            None
        };

        for (n, clause_nr) in clause_order.iter().enumerate() {
            let clause = &instance.clauses[*clause_nr];

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

            if man.options.enable_dvo {
                log::info!("DVO... ");
                let mut last_size = man.count_active(bdd);
                loop {
                    bdd = man.sift_all_vars(bdd);
                    let new_size = man.count_active(bdd);

                    if_some!(bar, set_message(format!("{} nodes", new_size)));

                    if new_size == last_size {
                        break;
                    }
                    last_size = new_size;
                }
            }

            log::info!("Purge retain... ");
            man.purge_retain(bdd);

            log::info!("{} nodes remain", man.nodes.len());
            log::info!("{:?}", man);

            if_some!(bar, set_message(format!("{} nodes", man.nodes.len())));
            if_some!(bar, inc(1));
        }
        if_some!(bar, finish());

        Ok((man, bdd))
    }
}
