use super::dvo_schedules::DVOScheduleEnum;

#[derive(Default)]
pub struct Options {
    pub progressbars: bool,
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
