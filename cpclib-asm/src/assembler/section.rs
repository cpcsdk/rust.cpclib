use crate::{Env, ExprResult, AssemblerError};

#[derive(Clone, Debug)]
pub struct Section {
    /// Name of the section
    pub(crate) name: String,
    /// Start address of the section
    pub(crate) start: u16,
    /// Last (included) address of the section
    pub(crate) stop: u16,
    /// Expected mmr configuration
    pub(crate) mmr: u8,

    pub(crate) output_adr: u16,
    pub(crate) max_output_adr: u16,
    pub(crate) code_adr: u16
}

impl Section {
    pub(crate) fn new(name: &str, start: u16, stop: u16, mmr: u8) -> Self {
        Section {
            mmr,
            name: name.to_owned(),
            start,
            stop,

            output_adr: start,
            code_adr: start,

            max_output_adr: start
        }
    }

    pub fn contains(&self, addr: u16) -> bool {
        addr >= self.start && addr <= self.stop
    }

    pub(crate) fn new_pass(&mut self) {
        self.output_adr = self.start;
        self.code_adr = self.start;
    }

    pub fn length(&self) -> u16 {
        self.stop - self.start + 1
    }

    pub fn used(&self) -> u16 {
        self.max_output_adr - self.start
    }
}

/// Returns the address of the beginning of the section
pub fn section_start(section_name: &str, env: &Env) -> Result<ExprResult, Box<AssemblerError>> {
    Ok(env.get_section_description(section_name)?.start.into())
}

pub fn section_stop(section_name: &str, env: &Env) -> Result<ExprResult, Box<AssemblerError>> {
    Ok(env.get_section_description(section_name)?.stop.into())
}

/// Returns the number of bytes available  in the section
pub fn section_length(section_name: &str, env: &Env) -> Result<ExprResult, Box<AssemblerError>> {
    Ok(env.get_section_description(section_name)?.length().into())
}

/// Returns the number of bytes consumed in the section
pub fn section_used(section_name: &str, env: &Env) -> Result<ExprResult, Box<AssemblerError>> {
    Ok(env.get_section_description(section_name)?.used().into())
}

pub fn section_mmr(section_name: &str, env: &Env) -> Result<ExprResult, Box<AssemblerError>> {
    Ok(env.get_section_description(section_name)?.mmr.into())
}
