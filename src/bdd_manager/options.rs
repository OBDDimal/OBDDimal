//! Options for BDD building

use super::dvo_schedules::DVOScheduleEnum;

#[derive(Default)]
pub struct Options {
    /// Display progress bars for BDD building and DVO progress
    pub progressbars: bool,
    /// DVO strategy: When and how to run DVO
    pub dvo: DVOScheduleEnum,
}

impl Options {
    pub fn with_progressbars(mut self) -> Options {
        self.progressbars = true;
        self
    }

    pub fn with_dvo(mut self, schedule: DVOScheduleEnum) -> Options {
        self.dvo = schedule;
        self
    }
}
