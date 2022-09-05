#[derive(Clone, Default)]
pub struct Options {
    pub progressbars: bool,
    pub enable_dvo: bool,
}

impl Options {
    #[allow(unused)]
    pub fn with_progressbars(mut self) -> Options {
        self.progressbars = true;
        self
    }

    #[allow(unused)]
    pub fn with_dvo(mut self) -> Options {
        self.enable_dvo = true;
        self
    }
}
