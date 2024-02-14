//! BDD building from CNF

//pub mod dimacs;

use std::cmp;

use dimacs::{Instance, Sign};
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};

use crate::{
    core::{
        bdd_manager::{align_clauses, DDManager},
        bdd_node::{NodeID, VarID},
        dvo::dvo_schedules::DVOSchedule,
        options::Options,
        order::check_order,
    },
    if_some,
    misc::hash_select::HashSet,
};

impl DDManager {
    /// Builds a BDD from a CNF read from DIMACS.
    ///
    /// * `instance` - Input CNF
    /// * `order` - Optional initial variable ordering, see [crate::misc::static_ordering] for implementations
    /// * `options` - DVO and progress bar settings
    ///
    /// ```
    /// # use obddimal::{
    /// #           core::{
    /// #               dvo::dvo_schedules,
    /// #               options::Options,
    /// #               bdd_manager::DDManager
    /// #           },
    /// #           misc::static_ordering,
    /// # };
    /// # use std::fs;
    /// let mut instance = dimacs::parse_dimacs(
    ///     &fs::read_to_string("examples/trivial.dimacs")
    ///         .expect("Failed to read dimacs file."),
    /// )
    /// .expect("Failed to parse dimacs file.");
    /// let order = Some(static_ordering::force(&instance));
    /// let dvo = dvo_schedules::SiftingAtThreshold::new(5);
    /// let (man, bdd) = DDManager::from_instance(
    ///     &mut instance,
    ///     order,
    ///     Options::default().with_progressbars().with_dvo(dvo.into()),
    /// ).unwrap();
    /// ```
    pub fn from_instance(
        instance: &mut Instance,
        order: Option<Vec<usize>>,
        mut options: Options,
    ) -> Result<(DDManager, NodeID), String> {
        let clauses = match instance {
            Instance::Cnf { ref clauses, .. } => clauses,
            _ => panic!("Unsupported dimacs format!"),
        };

        let mut man = DDManager::default();

        let clause_order = align_clauses(clauses);
        if let Some(o) = order {
            check_order(instance, &o)?;
            man.level2nodes.resize(
                cmp::max(man.level2nodes.len(), *o.iter().max().unwrap()),
                HashSet::default(),
            );
            man.var2level = o;
        }

        let mut bdd = man.one();

        let bar = if options.progressbars {
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
            let clause = &clauses[*clause_nr];

            log::info!("Integrating clause: {:?}", clause);

            let mut cbdd = man.zero();
            for x in clause.lits().iter() {
                let var = x.var().to_u64();
                let node = match x.sign() {
                    Sign::Pos => man.ith_var(VarID(var as usize)),
                    Sign::Neg => man.nith_var(VarID(var as usize)),
                };

                cbdd = man.or(node, cbdd);
            }

            bdd = man.and(cbdd, bdd);

            log::info!(
                "Nr. Nodes: {:?} ({:?}/{:?} clauses integrated)",
                &man.nodes.len(),
                n,
                clauses.len()
            );

            man.purge_retain(bdd);
            if_some!(bar, set_message(format!("{} nodes", man.nodes.len())));

            bdd = options.dvo.run_dvo(n, &mut man, bdd, &bar);

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
