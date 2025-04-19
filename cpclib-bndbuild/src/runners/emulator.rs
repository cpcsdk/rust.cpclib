use crate::task::EMUCTRL_CMDS;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Emulator {
    /// The user directly access to the emulator and can only use the options as coded by the emulator author
    EmulatorProxy(cpclib_runner::runner::emulator::Emulator),
    /// The user use the cpclib abstraction over emulator and can automatize zome tasks or harmonize options
    EmulatorFacade
}

impl Emulator {
    pub fn get_command(&self) -> &str {
        match self {
            Emulator::EmulatorProxy(e) => e.get_command(),
            Emulator::EmulatorFacade => EMUCTRL_CMDS[0]
        }
    }

    pub fn new_cpcemupower_default() -> Self {
        Self::EmulatorProxy(cpclib_runner::runner::emulator::Emulator::CpcEmuPower(Default::default()))
    }

    pub fn new_capriceforever_default() -> Self {
        Self::EmulatorProxy(cpclib_runner::runner::emulator::Emulator::CapriceForever(Default::default()))
    }

    pub fn new_winape_default() -> Self {
        Self::EmulatorProxy(cpclib_runner::runner::emulator::Emulator::Winape(
            Default::default()
        ))
    }

    pub fn new_ace_default() -> Self {
        Self::EmulatorProxy(cpclib_runner::runner::emulator::Emulator::Ace(
            Default::default()
        ))
    }

    pub fn new_cpcec_default() -> Self {
        Self::EmulatorProxy(cpclib_runner::runner::emulator::Emulator::Cpcec(
            Default::default()
        ))
    }

    pub fn new_amspirit_default() -> Self {
        Self::EmulatorProxy(cpclib_runner::runner::emulator::Emulator::Amspirit(
            Default::default()
        ))
    }

    pub fn new_sugarbox_default() -> Self {
        Self::EmulatorProxy(cpclib_runner::runner::emulator::Emulator::SugarBoxV2(
            Default::default()
        ))
    }

    pub fn new_facade() -> Self {
        Self::EmulatorFacade
    }
}
