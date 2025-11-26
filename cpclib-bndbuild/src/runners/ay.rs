#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum YmCruncher {
    #[cfg(feature = "fap")]
    Fap,
    Ayt,
    Miny
}

impl YmCruncher {
    pub fn get_command(&self) -> &'static str {
        match self {
            #[cfg(feature = "fap")]
            YmCruncher::Fap => cpclib_runner::runner::ay::fap::FAP_CMD,
            YmCruncher::Ayt => cpclib_runner::runner::ay::ayt::AYT_CMD,
            YmCruncher::Miny => cpclib_runner::runner::ay::minimiser::MINIMISER_CMD
        }
    }
}
