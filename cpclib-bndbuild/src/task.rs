use std::fmt::Display;

use cpclib_common::itertools::Itertools;
use cpclib_runner::emucontrol::EMUCTRL_CMD;
use cpclib_runner::runner::assembler::{RasmVersion, RASM_CMD};
use cpclib_runner::runner::emulator::{
    AceVersion, CpcecVersion, WinapeVersion, ACE_CMD, CPCEC_CMD, WINAPE_CMD
};
use cpclib_runner::runner::impdisc::IMPDISC_CMD;
use cpclib_runner::runner::martine::MARTINE_CMD;
use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer};

use crate::runners::assembler::Assembler;
use crate::runners::emulator::Emulator;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Task {
    Cp(StandardTask),
    Assembler(Assembler, StandardTask),
    BndBuild(StandardTask),
    Disc(StandardTask),
    ImpDsk(StandardTask),
    Echo(StandardTask),
    Emulator(Emulator, StandardTask),
    Extern(StandardTask),
    ImgConverter(StandardTask),
    Martine(StandardTask),
    Rm(StandardTask),
    Xfer(StandardTask)
}

pub const EMUCTRL_CMDS: &[&str] = &[EMUCTRL_CMD, "emu", "emuctrl", "emucontrol"];
pub const ACE_CMDS: &[&str] = &[ACE_CMD, "acedl"];
pub const WINAPE_CMDS: &[&str] = &[WINAPE_CMD];
pub const CPCEC_CMDS: &[&str] = &[CPCEC_CMD];

pub const BASM_CMDS: &[&str] = &["basm", "assemble"];
pub const BNDBUILD_CMDS: &[&str] = &["bndbuild", "build"];
pub const CP_CMDS: &[&str] = &["cp", "copy"];
pub const DISC_CMDS: &[&str] = &["dsk", "disc"];
pub const IMPDISC_CMDS: &[&str] = &[IMPDISC_CMD, "impdisc"];
pub const ECHO_CMDS: &[&str] = &["echo", "print"];
pub const EXTERN_CMDS: &[&str] = &["extern"];
pub const IMG2CPC_CMDS: &[&str] = &["img2cpc", "imgconverter"];
pub const MARTINE_CMDS: &[&str] = &[MARTINE_CMD];
pub const ORGAMS_CMDS: &[&str] = &["orgams"];
pub const RASM_CMDS: &[&str] = &[RASM_CMD];
pub const RM_CMDS: &[&str] = &["rm", "del"];
pub const XFER_CMDS: &[&str] = &["xfer", "cpcwifi", "m4"];

impl Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (cmd, s) = match self {
            Task::Assembler(a, s) => (a.get_command(), s),
            Task::BndBuild(s) => (BNDBUILD_CMDS[0], s),
            Task::Cp(s) => (CP_CMDS[0], s),
            Task::Disc(s) => (DISC_CMDS[0], s),
            Task::Echo(s) => (ECHO_CMDS[0], s),
            Task::Extern(s) => (EXTERN_CMDS[0], s),
            Task::ImgConverter(s) => (IMG2CPC_CMDS[0], s),
            Task::Rm(s) => (RM_CMDS[0], s),
            Task::Xfer(s) => (XFER_CMDS[0], s),
            Task::Emulator(e, s) => (e.get_command(), s),
            Task::Martine(s) => (MARTINE_CMDS[0], s),
            Task::ImpDsk(s) => (IMPDISC_CMDS[0], s)
        };

        write!(
            f,
            "{}{} {}",
            if s.ignore_error { "-" } else { "" },
            cmd,
            s.args
        )
    }
}

impl<'de> Deserialize<'de> for Task {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        struct Line;
        impl<'de> Visitor<'de> for Line {
            type Value = Task;

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where E: serde::de::Error {
                let (code, next) = v.split_once(" ").ok_or(Error::custom("Wrong format"))?;
                let (code, ignore) = if code.starts_with("-") {
                    (&code[1..], true)
                }
                else {
                    (code, false)
                };
                let std = StandardTask {
                    args: next.to_owned(),
                    ignore_error: ignore
                };

                if ACE_CMDS.iter().contains(&code) {
                    Ok(Task::Emulator(Emulator::new_ace_default(), std))
                }
                else if CPCEC_CMDS.iter().contains(&code) {
                    Ok(Task::Emulator(
                        Emulator::new_cpcec_default(),
                        std
                    ))
                }
                else if WINAPE_CMDS.iter().contains(&code) {
                    Ok(Task::Emulator(
                        Emulator::new_winape_default(),
                        std
                    ))
                }
                else if EMUCTRL_CMDS.iter().contains(&code) {
                    Ok(Task::Emulator(Emulator::new_controlled_access(), std))
                }
                else if BASM_CMDS.iter().contains(&code) {
                    Ok(Task::Assembler(Assembler::Basm, std))
                }
                else if ORGAMS_CMDS.iter().contains(&code) {
                    Ok(Task::Assembler(Assembler::Orgams, std))
                }
                else if RASM_CMDS.iter().contains(&code) {
                    Ok(Task::Assembler(
                        Assembler::Extern(cpclib_runner::runner::assembler::ExternAssembler::Rasm(
                            RasmVersion::default()
                        )),
                        std
                    ))
                }
                else if BNDBUILD_CMDS.iter().contains(&code) {
                    Ok(Task::BndBuild(std))
                }
                else if CP_CMDS.iter().contains(&code) {
                    Ok(Task::Cp(std))
                }
                else if DISC_CMDS.iter().contains(&code) {
                    Ok(Task::Disc(std))
                }
                else if ECHO_CMDS.iter().contains(&code) {
                    Ok(Task::Echo(std))
                }
                else if EXTERN_CMDS.iter().contains(&code) {
                    Ok(Task::Extern(std))
                }
                else if IMG2CPC_CMDS.iter().contains(&code) {
                    Ok(Task::ImgConverter(std))
                }
                else if IMPDISC_CMDS.iter().contains(&code) {
                    Ok(Task::ImpDsk(std))
                }
                else if MARTINE_CMDS.iter().contains(&code) {
                    Ok(Task::Martine(std))
                }
                else if RM_CMDS.iter().contains(&code) {
                    Ok(Task::Rm(std))
                }
                else if XFER_CMDS.iter().contains(&code) {
                    Ok(Task::Xfer(std))
                }
                else {
                    Err(Error::custom(format!("{code} is an invalid command")))
                }
            }

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("Expecting a command")
            }
        }

        deserializer.deserialize_str(Line)
    }
}

impl Task {
    pub fn new_basm(args: &str) -> Self {
        Self::Assembler(Assembler::Basm, StandardTask::new(args))
    }

    pub fn new_bndbuild(args: &str) -> Self {
        Self::BndBuild(StandardTask::new(args))
    }

    pub fn new_dsk(args: &str) -> Self {
        Self::Disc(StandardTask::new(args))
    }

    pub fn new_rm(args: &str) -> Self {
        Self::Rm(StandardTask::new(args))
    }

    pub fn new_echo(args: &str) -> Self {
        Self::Echo(StandardTask::new(args))
    }

    pub fn new_imgconverter(args: &str) -> Self {
        Self::ImgConverter(StandardTask::new(args))
    }

    fn standard_task(&self) -> &StandardTask {
        match self {
            Task::Assembler(_, t)
            | Task::BndBuild(t)
            | Task::Cp(t)
            | Task::Disc(t)
            | Task::ImpDsk(t)
            | Task::Echo(t)
            | Task::Extern(t)
            | Task::ImgConverter(t)
            | Task::Martine(t)
            | Task::Rm(t)
            | Task::Xfer(t)
            | Task::Emulator(_, t) => t
        }
    }

    fn standard_task_mut(&mut self) -> &mut StandardTask {
        match self {
            Task::Assembler(_, t)
            | Task::Rm(t)
            | Task::Echo(t)
            | Task::ImgConverter(t)
            | Task::Xfer(t)
            | Task::Extern(t)
            | Task::Disc(t)
            | Task::ImpDsk(t)
            | Task::BndBuild(t)
            | Task::Martine(t)
            | Task::Cp(t)
            | Task::Emulator(_, t) => t
        }
    }

    pub fn args(&self) -> &str {
        &self.standard_task().args
    }

    pub fn ignore_errors(&self) -> bool {
        self.standard_task().ignore_error
    }

    pub fn set_ignore_errors(mut self, ignore: bool) -> Self {
        self.standard_task_mut().ignore_error = ignore;
        self
    }

    // TODO deeply check the arguments of the commands because here we may be wrong ...
    pub fn is_phony(&self) -> bool {
        match self {
            Task::Assembler(..) => false, // wrong when displaying stuff
            Task::Rm(_) => false,
            Task::Echo(_) => true,
            Task::Emulator(..) => true,
            Task::Xfer(_) => true, // wrong when downloading files
            Task::Martine(t) => false,
            Task::ImgConverter(_) => false,
            Task::Extern(_) => false,
            Task::BndBuild(_) => false,
            Task::Disc(_) => false,
            Task::ImpDsk(_) => false,
            Task::Cp(_) => false
        }
    }
}

#[derive(Deserialize, Clone, PartialEq, Debug, Eq, Hash)]
pub struct StandardTask {
    args: String,
    ignore_error: bool
}

impl StandardTask {
    pub fn new(args: &str) -> Self {
        Self {
            args: args.to_string(),
            ignore_error: false
        }
    }
}

#[cfg(test)]
mod test {
    use super::Task;
    use crate::task::StandardTask;

    #[test]
    fn test_deserialize_task() {
        let yaml = "basm toto.asm -o toto.o";
        let task: Task = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            task,
            Task::Assembler(
                crate::runners::assembler::Assembler::Basm,
                StandardTask {
                    args: "toto.asm -o toto.o".to_owned(),
                    ignore_error: false
                }
            )
        );

        let yaml = "-basm toto.asm -o toto.o";
        let task: Task = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            task,
            Task::Assembler(
                crate::runners::assembler::Assembler::Basm,
                StandardTask {
                    args: "toto.asm -o toto.o".to_owned(),
                    ignore_error: true
                }
            )
        );
    }
}
