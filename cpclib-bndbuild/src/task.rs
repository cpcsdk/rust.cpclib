use std::fmt::Display;

use cpclib_common::itertools::Itertools;
use cpclib_runner::emucontrol::EMUCTRL_CMD;
use cpclib_runner::runner::assembler::{RasmVersion, RASM_CMD};
use cpclib_runner::runner::emulator::{ACE_CMD, CPCEC_CMD, WINAPE_CMD};
use cpclib_runner::runner::fap::FAP_CMD;
use cpclib_runner::runner::impdisc::IMPDISC_CMD;
use cpclib_runner::runner::martine::MARTINE_CMD;
use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer};

use crate::runners::assembler::Assembler;
use crate::runners::emulator::Emulator;
use crate::runners::hideur::HIDEUR_CMD;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Task {
    Assembler(Assembler, StandardTaskArguments),
    BndBuild(StandardTaskArguments),
    Cp(StandardTaskArguments),
    Disc(StandardTaskArguments),
    Echo(StandardTaskArguments),
    Emulator(Emulator, StandardTaskArguments),
    Extern(StandardTaskArguments),
    Fap(StandardTaskArguments),
    Hideur(StandardTaskArguments),
    ImgConverter(StandardTaskArguments),
    ImpDsk(StandardTaskArguments),
    Martine(StandardTaskArguments),
    Rm(StandardTaskArguments),
    Xfer(StandardTaskArguments)
}

pub const EMUCTRL_CMDS: &[&str] = &[EMUCTRL_CMD, "emu", "emuctrl", "emucontrol"];
pub const ACE_CMDS: &[&str] = &[ACE_CMD, "acedl"];
pub const WINAPE_CMDS: &[&str] = &[WINAPE_CMD];
pub const CPCEC_CMDS: &[&str] = &[CPCEC_CMD];

pub const BASM_CMDS: &[&str] = &["basm", "assemble"];
pub const BNDBUILD_CMDS: &[&str] = &["bndbuild", "build"];
pub const CP_CMDS: &[&str] = &["cp", "copy"];
pub const DISC_CMDS: &[&str] = &["dsk", "disc"];
pub const ECHO_CMDS: &[&str] = &["echo", "print"];
pub const EXTERN_CMDS: &[&str] = &["extern"];
pub const FAP_CMDS: &[&str] = &[FAP_CMD];
pub const IMG2CPC_CMDS: &[&str] = &["img2cpc", "imgconverter"];
pub const HIDEUR_CMDS: &[&str] = &[HIDEUR_CMD];
pub const IMPDISC_CMDS: &[&str] = &[IMPDISC_CMD, "impdisc"];
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
            Task::Emulator(e, s) => (e.get_command(), s),
            Task::Extern(s) => (EXTERN_CMDS[0], s),
            Task::Hideur(s) => (HIDEUR_CMDS[0], s),
            Task::ImgConverter(s) => (IMG2CPC_CMDS[0], s),
            Task::ImpDsk(s) => (IMPDISC_CMDS[0], s),
            Task::Martine(s) => (MARTINE_CMDS[0], s),
            Task::Rm(s) => (RM_CMDS[0], s),
            Task::Xfer(s) => (XFER_CMDS[0], s),
            Task::Fap(s) =>  (FAP_CMDS[0], s),
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

macro_rules! is_some_cmd {
    ($($code:ident), *) => {
        $(
            paste::paste! {
                #[inline]
                pub fn [<is_ $code:lower _cmd>](code: &str) -> bool {
                    [< $code:upper _CMDS>] .iter().contains(&code)
                }
            }
        )*

    };
}

is_some_cmd!(
    ace, cpcec, winape, emuctrl, basm, rasm, orgams, bndbuild, cp, rm, echo, disc, impdisc, hideur,
    img2cpc, martine, r#extern, xfer, fap
);

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
                let std = StandardTaskArguments {
                    args: next.to_owned(),
                    ignore_error: ignore
                };

                if is_ace_cmd(code) {
                    Ok(Task::Emulator(Emulator::new_ace_default(), std))
                }
                else if is_cpcec_cmd(code) {
                    Ok(Task::Emulator(Emulator::new_cpcec_default(), std))
                }
                else if is_winape_cmd(code) {
                    Ok(Task::Emulator(Emulator::new_winape_default(), std))
                }
                else if is_emuctrl_cmd(code) {
                    Ok(Task::Emulator(Emulator::new_controlled_access(), std))
                }
                else if is_basm_cmd(code) {
                    Ok(Task::Assembler(Assembler::Basm, std))
                } else if is_fap_cmd(code) {
                    Ok(Task::Fap(std))
                }
                else if is_orgams_cmd(code) {
                    Ok(Task::Assembler(Assembler::Orgams, std))
                }
                else if is_rasm_cmd(code) {
                    Ok(Task::Assembler(
                        Assembler::Extern(cpclib_runner::runner::assembler::ExternAssembler::Rasm(
                            RasmVersion::default()
                        )),
                        std
                    ))
                }
                else if is_bndbuild_cmd(code) {
                    Ok(Task::BndBuild(std))
                }
                else if is_cp_cmd(code) {
                    Ok(Task::Cp(std))
                }
                else if is_disc_cmd(code) {
                    Ok(Task::Disc(std))
                }
                else if is_echo_cmd(code) {
                    Ok(Task::Echo(std))
                }
                else if is_extern_cmd(code) {
                    Ok(Task::Extern(std))
                }
                else if is_hideur_cmd(code) {
                    Ok(Task::Hideur(std))
                }
                else if is_img2cpc_cmd(code) {
                    Ok(Task::ImgConverter(std))
                }
                else if is_impdisc_cmd(code) {
                    Ok(Task::ImpDsk(std))
                }
                else if is_martine_cmd(code) {
                    Ok(Task::Martine(std))
                }
                else if is_rm_cmd(code) {
                    Ok(Task::Rm(std))
                }
                else if is_xfer_cmd(code) {
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
        Self::Assembler(Assembler::Basm, StandardTaskArguments::new(args))
    }

    pub fn new_bndbuild(args: &str) -> Self {
        Self::BndBuild(StandardTaskArguments::new(args))
    }

    pub fn new_dsk(args: &str) -> Self {
        Self::Disc(StandardTaskArguments::new(args))
    }

    pub fn new_rm(args: &str) -> Self {
        Self::Rm(StandardTaskArguments::new(args))
    }

    pub fn new_echo(args: &str) -> Self {
        Self::Echo(StandardTaskArguments::new(args))
    }

    pub fn new_imgconverter(args: &str) -> Self {
        Self::ImgConverter(StandardTaskArguments::new(args))
    }

    fn standard_task_arguments(&self) -> &StandardTaskArguments {
        match self {
            Task::Assembler(_, t)
            | Task::BndBuild(t)
            | Task::Cp(t)
            | Task::Disc(t)
            | Task::ImpDsk(t)
            | Task::Echo(t)
            | Task::Extern(t)
            | Task::Hideur(t)
            | Task::ImgConverter(t)
            | Task::Martine(t)
            | Task::Rm(t)
            | Task::Xfer(t)
            | Task::Emulator(_, t)
            | Task::Fap(t)
             => t
        }
    }

    fn standard_task_arguments_mut(&mut self) -> &mut StandardTaskArguments {
        match self {
            Task::Assembler(_, t)
            | Task::Rm(t)
            | Task::Echo(t)
            | Task::ImgConverter(t)
            | Task::Xfer(t)
            | Task::Extern(t)
            | Task::Disc(t)
            | Task::Hideur(t)
            | Task::ImpDsk(t)
            | Task::BndBuild(t)
            | Task::Martine(t)
            | Task::Cp(t)
            | Task::Emulator(_, t)
            | Task::Fap(t)
             => t
        }
    }

    pub fn args(&self) -> &str {
        &self.standard_task_arguments().args
    }

    pub fn ignore_errors(&self) -> bool {
        self.standard_task_arguments().ignore_error
    }

    pub fn set_ignore_errors(mut self, ignore: bool) -> Self {
        self.standard_task_arguments_mut().ignore_error = ignore;
        self
    }

    // TODO deeply check the arguments of the commands because here we may be wrong ...
    pub fn is_phony(&self) -> bool {
        match self {
            Task::Assembler(..) => false, // wrong when displaying stuff
            Task::Rm(_) => false,
            Task::Echo(_) => true,
            Task::Emulator(..) => true,
            Task::Fap(..) => true,
            Task::Xfer(_) => true, // wrong when downloading files
            Task::Martine(t) => false,
            Task::Hideur(_) => false,
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
pub struct StandardTaskArguments {
    args: String,
    ignore_error: bool
}

impl StandardTaskArguments {
    pub fn new<S: Into<String>>(args: S) -> Self {
        Self {
            args: args.into(),
            ignore_error: false
        }
    }
}

#[cfg(test)]
mod test {
    use super::Task;
    use crate::task::StandardTaskArguments;

    #[test]
    fn test_deserialize_task() {
        let yaml = "basm toto.asm -o toto.o";
        let task: Task = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            task,
            Task::Assembler(
                crate::runners::assembler::Assembler::Basm,
                StandardTaskArguments {
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
                StandardTaskArguments {
                    args: "toto.asm -o toto.o".to_owned(),
                    ignore_error: true
                }
            )
        );
    }
}
