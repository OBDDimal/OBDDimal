//! Strategies for when and how to run DVO during BDD generation

#![allow(rustdoc::private_intra_doc_links)]

use std::time::{Duration, Instant};

use enum_dispatch::enum_dispatch;
use indicatif::ProgressBar;

use crate::{
    core::{bdd_manager::DDManager, bdd_node::NodeID},
    if_some,
};

/// Dummy DVO implementation that does nothing
#[derive(Default)]
pub struct NoDVOSchedule {}

impl DVOSchedule for NoDVOSchedule {
    fn run_dvo(
        &mut self,
        _num_clause: usize,
        _man: &mut DDManager,
        f: NodeID,
        _bar: &Option<ProgressBar>,
    ) -> NodeID {
        f
    }
}

/// Always perform sifting of all variables until the number of
/// nodes does not change anymore.
#[derive(Default)]
pub struct AlwaysUntilConvergence {}

impl DVOSchedule for AlwaysUntilConvergence {
    fn run_dvo(
        &mut self,
        _num_clause: usize,
        man: &mut DDManager,
        mut f: NodeID,
        bar: &Option<ProgressBar>,
    ) -> NodeID {
        log::info!("DVO... ");
        let mut last_size = man.count_active(f);
        loop {
            f = man.sift_all_vars(f, bar.is_some(), None);
            let new_size = man.count_active(f);

            if_some!(bar, set_message(format!("{} nodes", new_size)));

            if new_size == last_size {
                break;
            }
            last_size = new_size;
        }
        f
    }
}

/// Run one iteration of sifting for all variables, every time it's called.
/// See [DDManager::sift_single_var()] for `max_increase` parameter.
#[derive(Default)]
pub struct AlwaysOnce {
    pub max_increase: Option<u32>,
}

impl DVOSchedule for AlwaysOnce {
    fn run_dvo(
        &mut self,
        _num_clause: usize,
        man: &mut DDManager,
        mut f: NodeID,
        bar: &Option<ProgressBar>,
    ) -> NodeID {
        log::info!("DVO... ");
        f = man.sift_all_vars(f, bar.is_some(), self.max_increase);
        let new_size = man.count_active(f);
        if_some!(bar, set_message(format!("{} nodes", new_size)));
        f
    }
}

/// Call the underlying strategy if the node count exceeds the specified limit
pub struct AtThreshold {
    pub active_nodes_threshold: u32,
    pub underlying_schedule: Box<DVOScheduleEnum>,
}

impl DVOSchedule for AtThreshold {
    fn run_dvo(
        &mut self,
        num_clause: usize,
        man: &mut DDManager,
        f: NodeID,
        bar: &Option<ProgressBar>,
    ) -> NodeID {
        if man.count_active(f) <= self.active_nodes_threshold {
            f
        } else {
            self.underlying_schedule.run_dvo(num_clause, man, f, bar)
        }
    }
}

/// Performs sifting until the number of nodes does not change anymore,
/// but only if the initial number of nodes exceeds a configurable threshold.
pub struct SiftingAtThreshold {
    underlying: AtThreshold,
}

impl DVOSchedule for SiftingAtThreshold {
    fn run_dvo(
        &mut self,
        num_clause: usize,
        man: &mut DDManager,
        f: NodeID,
        bar: &Option<ProgressBar>,
    ) -> NodeID {
        self.underlying.run_dvo(num_clause, man, f, bar)
    }
}

impl SiftingAtThreshold {
    pub fn new(active_nodes_threshold: u32) -> SiftingAtThreshold {
        SiftingAtThreshold {
            underlying: AtThreshold {
                active_nodes_threshold,
                underlying_schedule: Box::new(AlwaysUntilConvergence::default().into()),
            },
        }
    }
}

/// Calls the underlying DVO mode if the specified duration has passed since the last
/// invocation, or the nodes table exceeds the size specified in `limit`.
pub struct TimeSizeLimit {
    pub interval: Duration,
    pub limit: usize,
    last_dvo: Instant,
    pub underlying_schedule: Box<DVOScheduleEnum>,
}

impl TimeSizeLimit {
    pub fn new(
        interval: Duration,
        limit: usize,
        underlying_schedule: Box<DVOScheduleEnum>,
    ) -> TimeSizeLimit {
        TimeSizeLimit {
            interval,
            limit,
            last_dvo: Instant::now(),
            underlying_schedule,
        }
    }
}

impl DVOSchedule for TimeSizeLimit {
    fn run_dvo(
        &mut self,
        num_clause: usize,
        man: &mut DDManager,
        f: NodeID,
        bar: &Option<ProgressBar>,
    ) -> NodeID {
        if (Instant::now() - self.last_dvo) > self.interval || man.nodes.len() > self.limit {
            let r = self.underlying_schedule.run_dvo(num_clause, man, f, bar);
            self.last_dvo = Instant::now();
            r
        } else {
            f
        }
    }
}

/// This contains all available DVO implementations
#[enum_dispatch]
pub enum DVOScheduleEnum {
    NoDVOSchedule,
    AlwaysUntilConvergence,
    AtThreshold,
    SiftingAtThreshold,
    TimeSizeLimit,
    AlwaysOnce,
}

impl Default for DVOScheduleEnum {
    fn default() -> Self {
        NoDVOSchedule::default().into()
    }
}

/// Implements run_dvo()
#[enum_dispatch(DVOScheduleEnum)]
pub(crate) trait DVOSchedule {
    /// This gets called after a CNF clause has been integrated.
    /// The current root node is f, the implementation must return the
    /// new root node ID, even if it does not change.
    /// * `num_clause`: The index of the current clause, in integration order
    ///   (which may differ from the order defined in the input CNF).
    fn run_dvo(
        &mut self,
        num_clause: usize,
        man: &mut DDManager,
        f: NodeID,
        bar: &Option<ProgressBar>,
    ) -> NodeID;
}
