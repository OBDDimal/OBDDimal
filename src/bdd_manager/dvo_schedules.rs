use enum_dispatch::enum_dispatch;
use indicatif::ProgressBar;

use super::DDManager;
use crate::{bdd_node::NodeID, if_some};

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
            f = man.sift_all_vars(f, bar.is_some());
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

pub struct SiftingAtThreshold {
    pub active_nodes_threshold: u32,
}

impl DVOSchedule for SiftingAtThreshold {
    fn run_dvo(
        &mut self,
        _num_clause: usize,
        man: &mut DDManager,
        mut f: NodeID,
        bar: &Option<ProgressBar>,
    ) -> NodeID {
        if man.count_active(f) < self.active_nodes_threshold {
            return f;
        }

        let mut last_size = man.count_active(f);
        loop {
            f = man.sift_all_vars(f, bar.is_some());
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

#[enum_dispatch]
pub enum DVOScheduleEnum {
    NoDVOSchedule,
    AlwaysUntilConvergence,
    SiftingAtThreshold,
}

impl Default for DVOScheduleEnum {
    fn default() -> Self {
        NoDVOSchedule::default().into()
    }
}

#[enum_dispatch(DVOScheduleEnum)]
pub(crate) trait DVOSchedule {
    fn run_dvo(
        &mut self,
        num_clause: usize,
        man: &mut DDManager,
        f: NodeID,
        bar: &Option<ProgressBar>,
    ) -> NodeID;
}
