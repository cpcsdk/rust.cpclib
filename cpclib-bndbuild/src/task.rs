use std::fmt::Display;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, LazyLock};

use camino::Utf8Path;
use cpclib_common::itertools::Itertools;
use cpclib_runner::emucontrol::EMUCTRL_CMD;
use cpclib_runner::runner::assembler::{RasmVersion, RASM_CMD, SJASMPLUS_CMD, VASM_CMD};
use cpclib_runner::runner::convgeneric::CONVGENERIC_CMD;
use cpclib_runner::runner::disassembler::disark::{DisarkVersion, DISARK_CMD};
use cpclib_runner::runner::disassembler::ExternDisassembler;
use cpclib_runner::runner::emulator::{
    ACE_CMD, AMSPIRIT_CMD, CPCEC_CMD, SUGARBOX_V2_CMD, WINAPE_CMD
};
use cpclib_runner::runner::fap::FAP_CMD;
use cpclib_runner::runner::hspcompiler::HSPC_CMD;
use cpclib_runner::runner::impdisc::IMPDISC_CMD;
use cpclib_runner::runner::martine::MARTINE_CMD;
use cpclib_runner::runner::tracker::at3::AT_CMD;
use fancy_regex::Regex;
use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer};

use crate::event::BndBuilderObserver;
use crate::execute;
use crate::runners::assembler::Assembler;
use crate::runners::disassembler::Disassembler;
use crate::runners::emulator::Emulator;
use crate::runners::hideur::HIDEUR_CMD;
use crate::runners::tracker::Tracker;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum InnerTask {
    Assembler(Assembler, StandardTaskArguments),
    BndBuild(StandardTaskArguments),
    Convgeneric(StandardTaskArguments),
    Cp(StandardTaskArguments),
    Disassembler(Disassembler, StandardTaskArguments),
    Disc(StandardTaskArguments),
    Echo(StandardTaskArguments),
    Emulator(Emulator, StandardTaskArguments),
    Extern(StandardTaskArguments),
    Fap(StandardTaskArguments),
    Hideur(StandardTaskArguments),
    HspCompiler(StandardTaskArguments),
    ImgConverter(StandardTaskArguments),
    ImpDsk(StandardTaskArguments),
    Martine(StandardTaskArguments),
    Mkdir(StandardTaskArguments),
    Rm(StandardTaskArguments),
    Snapshot(StandardTaskArguments),
    Tracker(Tracker, StandardTaskArguments),
    Xfer(StandardTaskArguments)
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Task {
    inner: InnerTask,
    id: usize
}

impl<'de> Deserialize<'de> for Task {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        InnerTask::deserialize(deserializer).map(|t| t.into())
    }
}
impl Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.inner)
    }
}

impl From<InnerTask> for Task {
    fn from(value: InnerTask) -> Self {
        Self {
            inner: value,
            id: Self::next_id()
        }
    }
}

impl Task {
    pub fn execute<E: BndBuilderObserver + 'static>(
        &self,
        observer: &Arc<E>
    ) -> Result<(), String> {
        execute(self, observer)
    }

    fn next_id() -> usize {
        static mut COUNTER: AtomicUsize = AtomicUsize::new(1);
        unsafe { COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed) }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn new_basm(args: &str) -> Self {
        InnerTask::new_basm(args).into()
    }

    pub fn new_bndbuild(args: &str) -> Self {
        InnerTask::new_bndbuild(args).into()
    }

    pub fn new_echo(args: &str) -> Self {
        InnerTask::new_echo(args).into()
    }

    pub fn new_imgconverter(args: &str) -> Self {
        InnerTask::new_imgconverter(args).into()
    }

    pub fn new_rm(args: &str) -> Self {
        InnerTask::new_rm(args).into()
    }

    pub fn set_ignore_errors(mut self, flag: bool) -> Self {
        let new = self.inner.clone().set_ignore_errors(flag);
        self.inner = new;
        self
    }
}

impl Deref for Task {
    type Target = InnerTask;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Task {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

// list of keywords; do not forget to add them to bndbuild/lib.rs
pub const EMUCTRL_CMDS: &[&str] = &[EMUCTRL_CMD, "emu", "emuctrl", "emucontrol"];
pub const ACE_CMDS: &[&str] = &[ACE_CMD, "acedl"];
pub const WINAPE_CMDS: &[&str] = &[WINAPE_CMD];
pub const CPCEC_CMDS: &[&str] = &[CPCEC_CMD];
pub const AMSPIRIT_CMDS: &[&str] = &[AMSPIRIT_CMD];
pub const SUGARBOX_CMDS: &[&str] = &[SUGARBOX_V2_CMD];

pub const BASM_CMDS: &[&str] = &["basm", "assemble"];
pub const ORGAMS_CMDS: &[&str] = &["orgams"];
pub const RASM_CMDS: &[&str] = &[RASM_CMD];
pub const SJASMPLUS_CMDS: &[&str] = &[SJASMPLUS_CMD];
pub const VASM_CMDS: &[&str] = &[VASM_CMD];

pub const BDASM_CMDS: &[&str] = &["bdasm", "dz80"];
pub const DISARK_CMDS: &[&str] = &[DISARK_CMD];

pub const AT_CMDS: &[&str] = &[AT_CMD, "ArkosTracker3"];

pub const HSPC_CMDS: &[&str] =&[HSPC_CMD, "hspc"];

pub const CP_CMDS: &[&str] = &["cp", "copy"];
pub const MKDIR_CMDS: &[&str] = &["mkdir"];
pub const RM_CMDS: &[&str] = &["rm", "del"];

pub const BNDBUILD_CMDS: &[&str] = &["bndbuild", "build"];
pub const CONVGENERIC_CMDS: &[&str] = &[CONVGENERIC_CMD];
pub const DISC_CMDS: &[&str] = &["dsk", "disc"];
pub const ECHO_CMDS: &[&str] = &["echo", "print"];
pub const EXTERN_CMDS: &[&str] = &["extern"];
pub const FAP_CMDS: &[&str] = &[FAP_CMD];
pub const IMG2CPC_CMDS: &[&str] = &["img2cpc", "imgconverter"];
pub const HIDEUR_CMDS: &[&str] = &[HIDEUR_CMD];
pub const IMPDISC_CMDS: &[&str] = &[IMPDISC_CMD, "impdisc"];
pub const MARTINE_CMDS: &[&str] = &[MARTINE_CMD];
pub const SNA_CMDS: &[&str] = &["sna", "snpashot"];
pub const XFER_CMDS: &[&str] = &["xfer", "cpcwifi", "m4"];

impl Display for InnerTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (cmd, s) = match self {
            Self::Assembler(a, s) => (a.get_command(), s),
            Self::BndBuild(s) => (BNDBUILD_CMDS[0], s),
            Self::Convgeneric(s) => (CONVGENERIC_CMDS[0], s),
            Self::Cp(s) => (CP_CMDS[0], s),
            Self::Disassembler(d, s) => (d.get_command(), s),
            Self::Disc(s) => (DISC_CMDS[0], s),
            Self::Echo(s) => (ECHO_CMDS[0], s),
            Self::Emulator(e, s) => (e.get_command(), s),
            Self::Extern(s) => (EXTERN_CMDS[0], s),
            Self::Fap(s) => (FAP_CMDS[0], s),
            Self::Hideur(s) => (HIDEUR_CMDS[0], s),
            Self::HspCompiler(s) => (HSPC_CMDS[0], s),
            Self::ImgConverter(s) => (IMG2CPC_CMDS[0], s),
            Self::ImpDsk(s) => (IMPDISC_CMDS[0], s),
            Self::Martine(s) => (MARTINE_CMDS[0], s),
            Self::Mkdir(s) => (MKDIR_CMDS[0], s),
            Self::Rm(s) => (RM_CMDS[0], s),
            Self::Snapshot(s) => (SNA_CMDS[0], s),
            Self::Tracker(t, s) => (t.get_command(), s),
            Self::Xfer(s) => (XFER_CMDS[0], s),
            Self::Fap(s) => (FAP_CMDS[0], s),
            Self::Snapshot(s) => (SNA_CMDS[0], s)
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

#[rustfmt::skip]
is_some_cmd!(
    ace, amspirit, at,
    basm, bdasm, bndbuild,
    convgeneric, cp, cpcec,
    disark, disc,
    echo, emuctrl, r#extern,
    fap,
    hideur,hspc,
    img2cpc, impdisc,
    martine, mkdir,
    orgams,
    rasm, rm,
    sjasmplus, sna, sugarbox,
    vasm,
    winape,
    xfer
);

impl<'de> Deserialize<'de> for InnerTask {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        struct Line;
        impl Visitor<'_> for Line {
            type Value = InnerTask;

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where E: serde::de::Error {
                let (code, next) = v.split_once(" ").unwrap_or((v, ""));
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
                    Ok(InnerTask::Emulator(Emulator::new_ace_default(), std))
                }
                else if is_at_cmd(code) {
                    Ok(InnerTask::Tracker(Tracker::new_at3_default(), std))
                }
                else if is_convgeneric_cmd(code) {
                    Ok(InnerTask::Convgeneric(std))
                }
                else if is_cpcec_cmd(code) {
                    Ok(InnerTask::Emulator(Emulator::new_cpcec_default(), std))
                }
                else if is_amspirit_cmd(code) {
                    Ok(InnerTask::Emulator(Emulator::new_amspirit_default(), std))
                }
                else if is_sugarbox_cmd(code) {
                    Ok(InnerTask::Emulator(Emulator::new_sugarbox_default(), std))
                }
                else if is_winape_cmd(code) {
                    Ok(InnerTask::Emulator(Emulator::new_winape_default(), std))
                }
                else if is_emuctrl_cmd(code) {
                    Ok(InnerTask::Emulator(Emulator::new_facade(), std))
                }
                else if is_basm_cmd(code) {
                    Ok(InnerTask::Assembler(Assembler::Basm, std))
                }
                else if is_bdasm_cmd(code) {
                    Ok(InnerTask::Disassembler(Disassembler::Bdasm, std))
                }
                else if is_disark_cmd(code) {
                    Ok(InnerTask::Disassembler(
                        Disassembler::Extern(ExternDisassembler::Disark(DisarkVersion::default())),
                        std
                    ))
                }
                else if is_fap_cmd(code) {
                    Ok(InnerTask::Fap(std))
                }
                else if is_orgams_cmd(code) {
                    Ok(InnerTask::Assembler(Assembler::Orgams, std))
                }
                else if is_rasm_cmd(code) {
                    Ok(InnerTask::Assembler(
                        Assembler::Extern(cpclib_runner::runner::assembler::ExternAssembler::Rasm(
                            RasmVersion::default()
                        )),
                        std
                    ))
                }
                else if is_sjasmplus_cmd(code) {
                    Ok(InnerTask::Assembler(
                        Assembler::Extern(
                            cpclib_runner::runner::assembler::ExternAssembler::Sjasmplus(
                                Default::default()
                            )
                        ),
                        std
                    ))
                }
                else if is_vasm_cmd(code) {
                    Ok(InnerTask::Assembler(
                        Assembler::Extern(cpclib_runner::runner::assembler::ExternAssembler::Vasm(
                            Default::default()
                        )),
                        std
                    ))
                }
                else if is_sna_cmd(code) {
                    Ok(InnerTask::Snapshot(std))
                }
                else if is_bndbuild_cmd(code) {
                    Ok(InnerTask::BndBuild(std))
                }
                else if is_disc_cmd(code) {
                    Ok(InnerTask::Disc(std))
                }
                else if is_echo_cmd(code) {
                    Ok(InnerTask::Echo(std))
                }
                else if is_extern_cmd(code) {
                    Ok(InnerTask::Extern(std))
                }
                else if is_hideur_cmd(code) {
                    Ok(InnerTask::Hideur(std))
                }
                else if is_img2cpc_cmd(code) {
                    Ok(InnerTask::ImgConverter(std))
                }
                else if is_impdisc_cmd(code) {
                    Ok(InnerTask::ImpDsk(std))
                }
                else if is_hspc_cmd(code) {
                    Ok(InnerTask::HspCompiler(std))
                }
                else if is_martine_cmd(code) {
                    Ok(InnerTask::Martine(std))
                }
                else if is_xfer_cmd(code) {
                    Ok(InnerTask::Xfer(std))
                }
                else if is_cp_cmd(code) {
                    Ok(InnerTask::Cp(std))
                }
                else if is_mkdir_cmd(code) {
                    Ok(InnerTask::Mkdir(std))
                }
                else if is_rm_cmd(code) {
                    Ok(InnerTask::Rm(std))
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

impl FromStr for Task {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        InnerTask::from_str(s).map(|t| t.into())
    }
}
impl FromStr for InnerTask {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_yaml::from_str(s).map_err(|e| e.to_string())
    }
}

impl InnerTask {
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


    pub fn replace_automatic_variables(&mut self, first_dep: Option<&Utf8Path>, first_tgt: Option<&Utf8Path>) -> Result<(), String > {
        self.standard_task_arguments_mut()
            .replace_automatic_variables(first_dep, first_tgt)

    } 

    fn standard_task_arguments(&self) -> &StandardTaskArguments {
        match self {
            InnerTask::Assembler(_, t)
            | InnerTask::BndBuild(t)
            | InnerTask::Convgeneric(t)
            | InnerTask::Cp(t)
            | InnerTask::Disassembler(_, t)
            | InnerTask::Disc(t)
            | InnerTask::ImpDsk(t)
            | InnerTask::Echo(t)
            | InnerTask::Extern(t)
            | InnerTask::Hideur(t)
            | InnerTask::HspCompiler(t)
            | InnerTask::ImgConverter(t)
            | InnerTask::Martine(t)
            | InnerTask::Mkdir(t)
            | InnerTask::Rm(t)
            | InnerTask::Xfer(t)
            | InnerTask::Emulator(_, t)
            | InnerTask::Snapshot(t)
            | InnerTask::Tracker(_, t)
            | InnerTask::Fap(t) => t
        }
    }

    fn standard_task_arguments_mut(&mut self) -> &mut StandardTaskArguments {
        match self {
            InnerTask::Assembler(_, t)
            | InnerTask::BndBuild(t)
            | InnerTask::Convgeneric(t)
            | InnerTask::Cp(t)
            | InnerTask::Disassembler(_, t)
            | InnerTask::Disc(t)
            | InnerTask::Echo(t)
            | InnerTask::Emulator(_, t)
            | InnerTask::Extern(t)
            | InnerTask::Fap(t)
            | InnerTask::Hideur(t)
            | InnerTask::HspCompiler(t)
            | InnerTask::ImgConverter(t)
            | InnerTask::ImpDsk(t)
            | InnerTask::BndBuild(t)
            | InnerTask::Martine(t)
            | InnerTask::Mkdir(t)
            | InnerTask::Rm(t)
            | InnerTask::Snapshot(t)
            | InnerTask::Tracker(_, t)
            | InnerTask::Xfer(t) => t
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
            InnerTask::Assembler(..) => false, // wrong when displaying stuff
            InnerTask::BndBuild(_) => false,
            InnerTask::Convgeneric(_) => false,
            InnerTask::Cp(_) => false,
            InnerTask::Disassembler(..) => false,
            InnerTask::Disc(_) => false,
            InnerTask::Echo(_) => true,
            InnerTask::Emulator(..) => true,
            InnerTask::Extern(_) => false,
            InnerTask::Fap(..) => true,
            InnerTask::Hideur(_) => false,
            InnerTask::HspCompiler(_) => false,
            InnerTask::ImgConverter(_) => false,
            InnerTask::ImpDsk(_) => false,
            InnerTask::Martine(t) => false,
            InnerTask::Mkdir(_) => false,
            InnerTask::Rm(_s) => false, // wrong when downloading files
            InnerTask::Snapshot(_) => false,
            InnerTask::Tracker(_, t) => true, // XXX think if false is better
            InnerTask::Xfer(_) => true
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

    /// This method modify the args to replace automatic variables by the expected values
    /// TODO keep the original argument for display and error purposes ?
    fn replace_automatic_variables(
        &mut self,
        first_dep: Option<&Utf8Path>,
        first_tgt: Option<&Utf8Path>
    ) -> Result<(), String> {
        static RE_FIRST_DEP: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"\${1}(?!\$)<").unwrap()); // 1 repetition does not seem to work :(
        static RE_FIRST_TGT: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"\${1}(?!\$)@").unwrap());

        let initial = self.args.clone();

        if let Some(first_dep) = first_dep {
            self.args = RE_FIRST_DEP
                .replace_all(&self.args, first_dep.as_str())
                .into_owned();
        }
        else {
            if RE_FIRST_DEP.is_match(&self.args).unwrap() {
                self.args = initial;
                return Err(format!(
                    "{} contains $<, but there are no available dependencies.",
                    self.args
                ));
            }
        }

        if let Some(first_tgt) = first_tgt {
            self.args = RE_FIRST_TGT
                .replace_all(&self.args, first_tgt.as_str())
                .into_owned();
        }
        else {
            if RE_FIRST_TGT.is_match(&self.args).unwrap() {
                self.args = initial;
                return Err(format!(
                    "{} contains $@, but there are no available targets.",
                    self.args
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::InnerTask;
    use crate::task::StandardTaskArguments;

    #[test]
    fn test_automatic_arguments() {
        // no replacement expected
        let mut no_args = StandardTaskArguments::new("a b");
        assert!(dbg!(no_args.replace_automatic_variables(None, None).is_ok()));
        assert_eq!(no_args.args, "a b");

        let mut no_args = StandardTaskArguments::new("a b");
        assert!(dbg!(no_args.replace_automatic_variables(Some("a".into()), None).is_ok()));
        assert_eq!(no_args.args, "a b");

        let mut no_args = StandardTaskArguments::new("a b");
        assert!(dbg!(no_args.replace_automatic_variables(None, Some("b".into())).is_ok()));
        assert_eq!(no_args.args, "a b");

        let mut no_args = StandardTaskArguments::new("a b");
        assert!(dbg!(no_args.replace_automatic_variables(Some("a".into()), Some("b".into())).is_ok()));
        assert_eq!(no_args.args, "a b");


        // tgt replacement expected
        let mut no_args = StandardTaskArguments::new("$@ b");
        assert!(dbg!(no_args.replace_automatic_variables(None, None).is_err()));
        assert_eq!(no_args.args, "$@ b");

        let mut no_args = StandardTaskArguments::new("$@ b");
        assert!(dbg!(no_args.replace_automatic_variables(Some("a".into()), None).is_err()));
        assert_eq!(no_args.args, "$@ b");

        let mut no_args = StandardTaskArguments::new("$@ b");
        assert!(dbg!(no_args.replace_automatic_variables(None, Some("b".into())).is_ok()));
        assert_eq!(no_args.args, "b b");


        let mut no_args = StandardTaskArguments::new("$@ b");
        assert!(dbg!(no_args.replace_automatic_variables(Some("a".into()), Some("b".into())).is_ok()));
        assert_eq!(no_args.args, "b b");


        // tgt and dep replacements expected
        let mut no_args = StandardTaskArguments::new("$@ $<");
        assert!(dbg!(no_args.replace_automatic_variables(None, None).is_err()));
        assert_eq!(no_args.args, "$@ $<");

        let mut no_args = StandardTaskArguments::new("$@ $<");
        assert!(dbg!(no_args.replace_automatic_variables(Some("a".into()), None).is_err()));
        assert_eq!(no_args.args, "$@ $<");

        let mut no_args = StandardTaskArguments::new("$@ $<");
        assert!(dbg!(no_args.replace_automatic_variables(None, Some("b".into())).is_err()));
        assert_eq!(no_args.args, "$@ $<");


        let mut no_args = StandardTaskArguments::new("$@ $<");
        assert!(dbg!(no_args.replace_automatic_variables(Some("a".into()), Some("b".into())).is_ok()));
        assert_eq!(no_args.args, "b a");

/*
        // duplicated $ change nothing
//        this onefails but i do not understand why
        let mut no_args = StandardTaskArguments::new("$$@ $$<");
        assert!(dbg!(no_args.replace_automatic_variables(Some("a".into()), Some("b".into())).is_ok()));
        assert_eq!(no_args.args, "$$@ $$<");

*/
    }

    #[test]
    fn test_deserialize_task() {
        let yaml = "basm toto.asm -o toto.o";
        let task: InnerTask = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            task,
            InnerTask::Assembler(
                crate::runners::assembler::Assembler::Basm,
                StandardTaskArguments {
                    args: "toto.asm -o toto.o".to_owned(),
                    ignore_error: false
                }
            )
        );

        let yaml = "-basm toto.asm -o toto.o";
        let task: InnerTask = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            task,
            InnerTask::Assembler(
                crate::runners::assembler::Assembler::Basm,
                StandardTaskArguments {
                    args: "toto.asm -o toto.o".to_owned(),
                    ignore_error: true
                }
            )
        );
    }
}
