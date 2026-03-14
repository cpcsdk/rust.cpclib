use std::fmt::Display;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, LazyLock};

use camino::Utf8Path;
use cpclib_common::clap::ArgMatches;
use cpclib_common::itertools::Itertools;
use cpclib_runner::emucontrol::EMUCTRL_CMD;
use cpclib_runner::runner::assembler::uz80::UZ80_CMD;
use cpclib_runner::runner::assembler::{RASM_CMD, RasmVersion, SJASMPLUS_CMD, VASM_CMD};
#[cfg(feature = "fap")]
use cpclib_runner::runner::ay::fap::FAP_CMD;
use cpclib_runner::runner::convgeneric::CONVGENERIC_CMD;
use cpclib_runner::runner::disassembler::ExternDisassembler;
use cpclib_runner::runner::disassembler::disark::{DISARK_CMD, DisarkVersion};
use cpclib_runner::runner::emulator::caprice_forever::CAPRICEFOREVER_CMD;
use cpclib_runner::runner::emulator::cpcemupower::CPCEMUPOWER_CMD;
use cpclib_runner::runner::emulator::{
    ACE_CMD, AMSPIRIT_CMD, CPCEC_CMD, SUGARBOX_V2_CMD, WINAPE_CMD
};
use cpclib_runner::runner::grafx2::GRAFX2_CMD;
use cpclib_runner::runner::hspcompiler::HSPC_CMD;
use cpclib_runner::runner::twocdt::TWO_CDT_CMD;
use cpclib_runner::runner::impdisc::IMPDISC_CMD;
use cpclib_runner::runner::martine::MARTINE_CMD;
use cpclib_runner::runner::tracker::at3::AT_CMD;
use cpclib_runner::runner::tracker::at3::extra::{
    SongToAkg, SongToAkm, SongToAky, SongToEvents, SongToRaw, SongToSoundEffects, SongToVgm,
    SongToWav, SongToYm, Z80Profiler
};
use cpclib_runner::runner::tracker::chipnsfx::CHIPNSFX_CMD;
use fancy_regex::Regex;
use serde::de::Visitor;
use serde::{Deserialize, Deserializer};

use crate::event::BndBuilderObserver;
use crate::execute;
use crate::runners::assembler::Assembler;
use crate::runners::ay::YmCruncher;
use crate::runners::basmdoc::BASMDOC_CMD;
use crate::runners::cdt::CdtManager;
use crate::runners::disassembler::Disassembler;
use crate::runners::emulator::Emulator;
use crate::runners::fade::FADE_CMD;
use crate::runners::hideur::HIDEUR_CMD;
use crate::runners::tracker::{SongConverter, Tracker};

/// Represents the kind of task based on how it's implemented
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TaskKind {
    /// Task implemented directly in bndbuild (e.g., via macros)
    Embedded,
    /// Task that delegates to an external program that must be installed
    Delegated,
    /// Task that runs through an emulator
    Emulated
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum InnerTask {
    Assembler(Assembler, StandardTaskArguments),
    BasmDoc(StandardTaskArguments),
    BndBuild(StandardTaskArguments),
    Catalog(StandardTaskArguments),
    Convgeneric(StandardTaskArguments),
    Locomotive(StandardTaskArguments),
    Cp(StandardTaskArguments),
    CpcToImg(StandardTaskArguments),
    Cpr(StandardTaskArguments),
    Csl(StandardTaskArguments),
    Crunch(StandardTaskArguments),
    Disassembler(Disassembler, StandardTaskArguments),
    Disc(StandardTaskArguments),
    Cdt(CdtManager, StandardTaskArguments),
    Echo(StandardTaskArguments),
    Emulator(Emulator, StandardTaskArguments),
    Extern(StandardTaskArguments),
    Fade(StandardTaskArguments),
    Grafx2(StandardTaskArguments),
    Hideur(StandardTaskArguments),
    HspCompiler(StandardTaskArguments),
    Hxcfe(StandardTaskArguments),
    ImgToCpc(StandardTaskArguments),
    ImpDsk(StandardTaskArguments),
    Martine(StandardTaskArguments),
    Mkdir(StandardTaskArguments),
    Mv(StandardTaskArguments),
    Rm(StandardTaskArguments),
    Snapshot(StandardTaskArguments),
    SongConverter(SongConverter, StandardTaskArguments),
    Tracker(Tracker, StandardTaskArguments),
    Xfer(StandardTaskArguments),
    YmCruncher(YmCruncher, StandardTaskArguments)
}

/// Represents a build task with a unique identifier.
///
/// Tasks encapsulate various build operations (assembly, compilation,
/// image conversion, etc.) and track dependencies between build steps.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Task {
    pub(crate) inner: InnerTask,
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
        static COUNTER: AtomicUsize = AtomicUsize::new(1);

        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
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
pub const CPCEMUPOWER_CMDS: &[&str] = &[CPCEMUPOWER_CMD];
pub const CAPRICEFOREVER_CMDS: &[&str] = &[CAPRICEFOREVER_CMD];

pub const BASM_CMDS: &[&str] = &["basm", "assemble"];
pub const ORGAMS_CMDS: &[&str] = &["orgams"];
pub const RASM_CMDS: &[&str] = &[RASM_CMD];
pub const SJASMPLUS_CMDS: &[&str] = &[SJASMPLUS_CMD];
pub const UZ80_CMDS: &[&str] = &[UZ80_CMD];
pub const VASM_CMDS: &[&str] = &[VASM_CMD];

pub const BASMDOC_CMDS: &[&str] = &[BASMDOC_CMD, "doc"];
pub const BDASM_CMDS: &[&str] = &["bdasm", "dz80"];
pub const DISARK_CMDS: &[&str] = &[DISARK_CMD];

pub const AT_CMDS: &[&str] = &[AT_CMD, "ArkosTracker3"];
pub const CHIPNSFX_CMDS: &[&str] = &[CHIPNSFX_CMD];

pub const HSPC_CMDS: &[&str] = &[HSPC_CMD, "hspc"];

pub const CP_CMDS: &[&str] = &["cp", "copy"];
pub const MV_CMDS: &[&str] = &["mv", "move", "rename"];
pub const MKDIR_CMDS: &[&str] = &["mkdir"];
pub const RM_CMDS: &[&str] = &["rm", "del"];

pub const BNDBUILD_CMDS: &[&str] = &["bndbuild", "build"];
pub const CONVGENERIC_CMDS: &[&str] = &[CONVGENERIC_CMD];
pub const DISC_CMDS: &[&str] = &["dsk", "disc"];
pub const CATALOG_CMDS: &[&str] = &["catalog", "cat"];
pub const LOCOMOTIVE_CMDS: &[&str] = &["locomotive", "basic"];
pub const ECHO_CMDS: &[&str] = &["echo", "print"];
pub const EXTERN_CMDS: &[&str] = &["extern"];

#[cfg(feature = "fap")]
pub const FAP_CMDS: &[&str] = &[FAP_CMD];
pub const AYT_CMDS: &[&str] = &[cpclib_runner::runner::ay::ayt::AYT_CMD];
pub const MINY_CMDS: &[&str] = &[cpclib_runner::runner::ay::minimiser::MINIMISER_CMD];

pub const FADE_CMDS: &[&str] = &[FADE_CMD];
pub const GRAFX2_CMDS: &[&str] = &[GRAFX2_CMD, "grafx"];
pub const IMG2CPC_CMDS: &[&str] = &["img2cpc", "imgconverter"];
pub const CPC2IMG_CMDS: &[&str] = &["cpc2img"];
pub const HIDEUR_CMDS: &[&str] = &[HIDEUR_CMD];
pub const HXCFE_CMDS: &[&str] = &["hxcfe"];
pub const IMPDISC_CMDS: &[&str] = &[IMPDISC_CMD, "impdisc"];
pub const MARTINE_CMDS: &[&str] = &[MARTINE_CMD];
pub const SNA_CMDS: &[&str] = &["sna", "snpashot"];
pub const XFER_CMDS: &[&str] = &["xfer", "cpcwifi", "m4"];

pub const CPR_CMDS: &[&str] = &["cpr"];
pub const CSL_CMDS: &[&str] = &["csl"];

pub const CRUNCH_CMDS: &[&str] = &["crunch", "compress"];

pub const RTZX_CMDS: &[&str] = &["rtzx"];
pub const TWO_CDT_CMDS: &[&str] = &[TWO_CDT_CMD];

pub const SONG2AKM_CMDS: &[&str] = &[SongToAkm::CMD];
pub const SONG2AKG_CMDS: &[&str] = &[SongToAkg::CMD];
pub const SONG2AKY_CMDS: &[&str] = &[SongToAky::CMD];
pub const SONG2EVENTS_CMDS: &[&str] = &[SongToEvents::CMD];
pub const SONG2RAW_CMDS: &[&str] = &[SongToRaw::CMD];
pub const SONG2SOUNDEFFECTS_CMDS: &[&str] = &[SongToSoundEffects::CMD];
pub const SONG2VGM_CMDS: &[&str] = &[SongToVgm::CMD];
pub const SONG2WAV_CMDS: &[&str] = &[SongToWav::CMD];
pub const SONG2YM_CMDS: &[&str] = &[SongToYm::CMD];
pub const Z80PROFILER_CMDS: &[&str] = &[Z80Profiler::CMD];

impl Display for InnerTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (cmd, s) = match self {
            Self::Assembler(a, s) => (a.get_command(), s),
            Self::Cdt(c, s) => (c.get_command(), s),
            Self::YmCruncher(t, s) => (t.get_command(), s),
            Self::BasmDoc(s) => (BASMDOC_CMDS[0], s),
            Self::BndBuild(s) => (BNDBUILD_CMDS[0], s),
            Self::Convgeneric(s) => (CONVGENERIC_CMDS[0], s),
            Self::Catalog(s) => (CATALOG_CMDS[0], s),
            Self::Locomotive(s) => (LOCOMOTIVE_CMDS[0], s),
            Self::Cp(s) => (CP_CMDS[0], s),
            Self::Mv(s) => (MV_CMDS[0], s),
            Self::CpcToImg(s) => (CPC2IMG_CMDS[0], s),
            Self::Cpr(s) => (CPR_CMDS[0], s),
            Self::Csl(s) => (CSL_CMDS[0], s),
            Self::Crunch(s) => (CRUNCH_CMDS[0], s),
            Self::Disassembler(d, s) => (d.get_command(), s),
            Self::Disc(s) => (DISC_CMDS[0], s),
            Self::Echo(s) => (ECHO_CMDS[0], s),
            Self::Emulator(e, s) => (e.get_command(), s),
            Self::Extern(s) => (EXTERN_CMDS[0], s),
            Self::Fade(s) => (FADE_CMDS[0], s),
            Self::Grafx2(s) => (GRAFX2_CMDS[0], s),
            Self::Hideur(s) => (HIDEUR_CMDS[0], s),
            Self::HspCompiler(s) => (HSPC_CMDS[0], s),
            Self::Hxcfe(s) => (HXCFE_CMDS[0], s),
            Self::ImgToCpc(s) => (IMG2CPC_CMDS[0], s),
            Self::ImpDsk(s) => (IMPDISC_CMDS[0], s),
            Self::Martine(s) => (MARTINE_CMDS[0], s),
            Self::Mkdir(s) => (MKDIR_CMDS[0], s),
            Self::Rm(s) => (RM_CMDS[0], s),
            Self::Snapshot(s) => (SNA_CMDS[0], s),
            Self::SongConverter(t, s) => (t.get_command(), s),
            Self::Tracker(t, s) => (t.get_command(), s),
            Self::Xfer(s) => (XFER_CMDS[0], s)
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
    ace, amspirit, at, ayt,
    basm, basmdoc, bdasm, bndbuild,
    catalog, capriceforever, chipnsfx, convgeneric, cpr, csl, crunch, cp, cpcec, cpcemupower, cpc2img,
    disark, disc,
    echo, emuctrl, r#extern,
    fade,
    grafx2,
    hideur,hspc,hxcfe,
    img2cpc, impdisc,
    locomotive,
    miny,
    martine, mkdir, mv,
    orgams,
    rasm, rm, rtzx, two_cdt,
    sjasmplus, sna, sugarbox, song2akm, song2akg, song2aky, song2events, song2raw, song2soundeffects, song2vgm, song2wav, song2ym,
    z80profiler,
    uz80,
    vasm,
    winape,
    xfer
);

#[cfg(feature = "fap")]
is_some_cmd!(fap);

#[cfg(not(feature = "fap"))]
pub fn is_fap_cmd(code: &str) -> bool {
    false
}

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
                    original_args: None,
                    ignore_error: ignore
                };
                match InnerTask::from_command_and_arguments(code, std) {
                    Ok(t) => Ok(t),
                    Err(e) => Err(E::custom(e))
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
        Self::ImgToCpc(StandardTaskArguments::new(args))
    }

    // Helper constructors that accept already-built StandardTaskArguments.
    pub fn with_assembler(a: Assembler, std: StandardTaskArguments) -> Self {
        Self::Assembler(a, std)
    }

    pub fn with_basmdoc(std: StandardTaskArguments) -> Self {
        Self::BasmDoc(std)
    }

    pub fn with_bndbuild(std: StandardTaskArguments) -> Self {
        Self::BndBuild(std)
    }

    pub fn with_catalog(std: StandardTaskArguments) -> Self {
        Self::Catalog(std)
    }

    pub fn with_locomotive(std: StandardTaskArguments) -> Self {
        Self::Locomotive(std)
    }

    pub fn with_convgeneric(std: StandardTaskArguments) -> Self {
        Self::Convgeneric(std)
    }

    pub fn with_cp(std: StandardTaskArguments) -> Self {
        Self::Cp(std)
    }

    pub fn with_cpc_to_img(std: StandardTaskArguments) -> Self {
        Self::CpcToImg(std)
    }

    pub fn with_cpr(std: StandardTaskArguments) -> Self {
        Self::Cpr(std)
    }

    pub fn with_csl(std: StandardTaskArguments) -> Self {
        Self::Csl(std)
    }

    pub fn with_crunch(std: StandardTaskArguments) -> Self {
        Self::Crunch(std)
    }

    pub fn with_disassembler(d: Disassembler, std: StandardTaskArguments) -> Self {
        Self::Disassembler(d, std)
    }

    pub fn with_disc(std: StandardTaskArguments) -> Self {
        Self::Disc(std)
    }

    pub fn with_echo(std: StandardTaskArguments) -> Self {
        Self::Echo(std)
    }

    pub fn with_extern(std: StandardTaskArguments) -> Self {
        Self::Extern(std)
    }

    pub fn with_hideur(std: StandardTaskArguments) -> Self {
        Self::Hideur(std)
    }

    pub fn with_img_to_cpc(std: StandardTaskArguments) -> Self {
        Self::ImgToCpc(std)
    }

    pub fn with_impdsk(std: StandardTaskArguments) -> Self {
        Self::ImpDsk(std)
    }

    pub fn with_hspcompiler(std: StandardTaskArguments) -> Self {
        Self::HspCompiler(std)
    }

    pub fn with_hxcfe(std: StandardTaskArguments) -> Self {
        Self::Hxcfe(std)
    }

    pub fn with_martine(std: StandardTaskArguments) -> Self {
        Self::Martine(std)
    }

    pub fn with_xfer(std: StandardTaskArguments) -> Self {
        Self::Xfer(std)
    }

    pub fn with_mkdir(std: StandardTaskArguments) -> Self {
        Self::Mkdir(std)
    }

    pub fn with_mv(std: StandardTaskArguments) -> Self {
        Self::Mv(std)
    }

    pub fn with_rm(std: StandardTaskArguments) -> Self {
        Self::Rm(std)
    }

    pub fn with_snapshot(std: StandardTaskArguments) -> Self {
        Self::Snapshot(std)
    }

    pub fn with_emulator(e: Emulator, std: StandardTaskArguments) -> Self {
        Self::Emulator(e, std)
    }

    pub fn with_songconverter(sc: SongConverter, std: StandardTaskArguments) -> Self {
        Self::SongConverter(sc, std)
    }

    pub fn with_tracker(t: Tracker, std: StandardTaskArguments) -> Self {
        Self::Tracker(t, std)
    }

    pub fn with_ym_cruncher(y: YmCruncher, std: StandardTaskArguments) -> Self {
        Self::YmCruncher(y, std)
    }

    pub fn with_grafx2(std: StandardTaskArguments) -> Self {
        Self::Grafx2(std)
    }

    pub fn with_fade(std: StandardTaskArguments) -> Self {
        Self::Fade(std)
    }

    pub fn with_rtzx(std: StandardTaskArguments) -> Self {
        Self::Cdt(CdtManager::Rtzx, std)
    }

    pub fn with_two_cdt(std: StandardTaskArguments) -> Self {
        Self::Cdt(CdtManager::TwoCdt, std)
    }

    /// Create an InnerTask from a command token and its standard arguments.
    pub fn from_command_and_arguments(
        code: &str,
        std: StandardTaskArguments
    ) -> Result<Self, String> {
        if is_ace_cmd(code) {
            Ok(Self::with_emulator(Emulator::new_ace_default(), std))
        }
        else if is_at_cmd(code) {
            Ok(Self::with_tracker(Tracker::new_at3_default(), std))
        }
        else if is_catalog_cmd(code) {
            Ok(Self::with_catalog(std))
        }
        else if is_locomotive_cmd(code) {
            Ok(Self::with_locomotive(std))
        }
        else if is_chipnsfx_cmd(code) {
            Ok(Self::with_tracker(Tracker::new_chipnsfx_default(), std))
        }
        else if is_song2akm_cmd(code) {
            Ok(Self::with_songconverter(
                SongConverter::new_song_to_akm_default(),
                std
            ))
        }
        else if is_song2aky_cmd(code) {
            Ok(Self::with_songconverter(
                SongConverter::new_song_to_aky_default(),
                std
            ))
        }
        else if is_song2akg_cmd(code) {
            Ok(Self::with_songconverter(
                SongConverter::new_song_to_akg_default(),
                std
            ))
        }
        else if is_song2events_cmd(code) {
            Ok(Self::with_songconverter(
                SongConverter::new_song_to_events_default(),
                std
            ))
        }
        else if is_song2raw_cmd(code) {
            Ok(Self::with_songconverter(
                SongConverter::new_song_to_raw_default(),
                std
            ))
        }
        else if is_song2soundeffects_cmd(code) {
            Ok(Self::with_songconverter(
                SongConverter::new_song_to_sound_effects_default(),
                std
            ))
        }
        else if is_song2vgm_cmd(code) {
            Ok(Self::with_songconverter(
                SongConverter::new_song_to_vgm_default(),
                std
            ))
        }
        else if is_song2wav_cmd(code) {
            Ok(Self::with_songconverter(
                SongConverter::new_song_to_wav_default(),
                std
            ))
        }
        else if is_song2ym_cmd(code) {
            Ok(Self::with_songconverter(
                SongConverter::new_song_to_ym_default(),
                std
            ))
        }
        else if is_z80profiler_cmd(code) {
            Ok(Self::with_songconverter(
                SongConverter::new_z80profiler_default(),
                std
            ))
        }
        else if is_crunch_cmd(code) {
            Ok(Self::with_crunch(std))
        }
        else if is_convgeneric_cmd(code) {
            Ok(Self::with_convgeneric(std))
        }
        else if is_cpcec_cmd(code) {
            Ok(Self::with_emulator(Emulator::new_cpcec_default(), std))
        }
        else if is_amspirit_cmd(code) {
            Ok(Self::with_emulator(Emulator::new_amspirit_default(), std))
        }
        else if is_sugarbox_cmd(code) {
            Ok(Self::with_emulator(Emulator::new_sugarbox_default(), std))
        }
        else if is_winape_cmd(code) {
            Ok(Self::with_emulator(Emulator::new_winape_default(), std))
        }
        else if is_cpcemupower_cmd(code) {
            Ok(Self::with_emulator(
                Emulator::new_cpcemupower_default(),
                std
            ))
        }
        else if is_capriceforever_cmd(code) {
            Ok(Self::with_emulator(
                Emulator::new_capriceforever_default(),
                std
            ))
        }
        else if is_emuctrl_cmd(code) {
            Ok(Self::with_emulator(Emulator::new_facade(), std))
        }
        else if is_basm_cmd(code) {
            Ok(Self::with_assembler(Assembler::Basm, std))
        }
        else if is_bdasm_cmd(code) {
            Ok(Self::with_disassembler(Disassembler::Bdasm, std))
        }
        else if is_basmdoc_cmd(code) {
            Ok(Self::with_basmdoc(std))
        }
        else if is_disark_cmd(code) {
            Ok(Self::with_disassembler(
                Disassembler::Extern(ExternDisassembler::Disark(DisarkVersion::default())),
                std
            ))
        }
        else if is_grafx2_cmd(code) {
            Ok(Self::with_grafx2(std))
        }
        else if is_fade_cmd(code) {
            Ok(Self::with_fade(std))
        }
        else if is_fap_cmd(code) {
            #[cfg(feature = "fap")]
            let res = Ok(Self::with_ym_cruncher(YmCruncher::Fap, std));

            #[cfg(not(feature = "fap"))]
            let res = Err("FAP command requires the 'fap' feature to be enabled".to_string());

            res
        }
        else if is_ayt_cmd(code) {
            Ok(Self::with_ym_cruncher(YmCruncher::Ayt, std))
        }
        else if is_miny_cmd(code) {
            Ok(Self::with_ym_cruncher(YmCruncher::Miny, std))
        }
        else if is_orgams_cmd(code) {
            Ok(Self::with_assembler(Assembler::Orgams, std))
        }
        else if is_rasm_cmd(code) {
            Ok(Self::with_assembler(
                Assembler::Extern(cpclib_runner::runner::assembler::ExternAssembler::Rasm(
                    RasmVersion::default()
                )),
                std
            ))
        }
        else if is_uz80_cmd(code) {
            Ok(Self::with_assembler(
                Assembler::Extern(cpclib_runner::runner::assembler::ExternAssembler::Uz80(
                    Default::default()
                )),
                std
            ))
        }
        else if is_sjasmplus_cmd(code) {
            Ok(Self::with_assembler(
                Assembler::Extern(
                    cpclib_runner::runner::assembler::ExternAssembler::Sjasmplus(Default::default())
                ),
                std
            ))
        }
        else if is_vasm_cmd(code) {
            Ok(Self::with_assembler(
                Assembler::Extern(cpclib_runner::runner::assembler::ExternAssembler::Vasm(
                    Default::default()
                )),
                std
            ))
        }
        else if is_sna_cmd(code) {
            Ok(Self::with_snapshot(std))
        }
        else if is_bndbuild_cmd(code) {
            Ok(Self::with_bndbuild(std))
        }
        else if is_disc_cmd(code) {
            Ok(Self::with_disc(std))
        }
        else if is_echo_cmd(code) {
            Ok(Self::with_echo(std))
        }
        else if is_extern_cmd(code) {
            Ok(Self::with_extern(std))
        }
        else if is_hideur_cmd(code) {
            Ok(Self::with_hideur(std))
        }
        else if is_img2cpc_cmd(code) {
            Ok(Self::with_img_to_cpc(std))
        }
        else if is_cpc2img_cmd(code) {
            Ok(Self::with_cpc_to_img(std))
        }
        else if is_cpr_cmd(code) {
            Ok(Self::with_cpr(std))
        }
        else if is_csl_cmd(code) {
            Ok(Self::with_csl(std))
        }
        else if is_impdisc_cmd(code) {
            Ok(Self::with_impdsk(std))
        }
        else if is_hspc_cmd(code) {
            Ok(Self::with_hspcompiler(std))
        }
        else if is_hxcfe_cmd(code) {
            Ok(Self::with_hxcfe(std))
        }
        else if is_martine_cmd(code) {
            Ok(Self::with_martine(std))
        }
        else if is_xfer_cmd(code) {
            Ok(Self::with_xfer(std))
        }
        else if is_cp_cmd(code) {
            Ok(Self::with_cp(std))
        }
        else if is_mv_cmd(code) {
            Ok(Self::with_mv(std))
        }
        else if is_mkdir_cmd(code) {
            Ok(Self::with_mkdir(std))
        }
        else if is_rm_cmd(code) {
            Ok(Self::with_rm(std))
        }
        else if is_rtzx_cmd(code) {
            Ok(Self::with_rtzx(std))
        }
        else if is_two_cdt_cmd(code) {
            Ok(Self::with_two_cdt(std))
        }
        else {
            Err(format!("{code} is an invalid command"))
        }
    }

    pub fn replace_automatic_variables(
        &mut self,
        first_dep: Option<&Utf8Path>,
        first_tgt: Option<&Utf8Path>
    ) -> Result<(), String> {
        self.standard_task_arguments_mut()
            .replace_automatic_variables(first_dep, first_tgt)
    }

    /// Indicates whether this task can be launched in parallel with other tasks.
    /// For now, only BndBuild tasks are considered non-parallelizable because they
    /// modify the current working directory.
    pub fn is_parallelizable(&self) -> bool {
        match self {
            InnerTask::BndBuild(..) => false,
            _ => true
        }
    }

    fn standard_task_arguments(&self) -> &StandardTaskArguments {
        match self {
            InnerTask::Assembler(_, t)
            | InnerTask::Cdt(_, t)
            | InnerTask::Catalog(t)
            | InnerTask::Locomotive(t)
            | InnerTask::YmCruncher(_, t)
            | InnerTask::BasmDoc(t)
            | InnerTask::BndBuild(t)
            | InnerTask::Convgeneric(t)
            | InnerTask::Crunch(t)
            | InnerTask::Cp(t)
            | InnerTask::Mv(t)
            | InnerTask::CpcToImg(t)
            | InnerTask::Disassembler(_, t)
            | InnerTask::Disc(t)
            | InnerTask::ImpDsk(t)
            | InnerTask::Echo(t)
            | InnerTask::Extern(t)
            | InnerTask::Fade(t)
            | InnerTask::Grafx2(t)
            | InnerTask::Hideur(t)
            | InnerTask::HspCompiler(t)
            | InnerTask::Hxcfe(t)
            | InnerTask::ImgToCpc(t)
            | InnerTask::Martine(t)
            | InnerTask::Mkdir(t)
            | InnerTask::Rm(t)
            | InnerTask::Xfer(t)
            | InnerTask::Cpr(t)
            | InnerTask::Csl(t)
            | InnerTask::Emulator(_, t)
            | InnerTask::Snapshot(t)
            | InnerTask::SongConverter(_, t)
            | InnerTask::Tracker(_, t) => t
        }
    }

    fn standard_task_arguments_mut(&mut self) -> &mut StandardTaskArguments {
        match self {
            InnerTask::Assembler(_, t)
            | InnerTask::Cdt(_, t)
            | InnerTask::YmCruncher(_, t)
            | InnerTask::BasmDoc(t)
            | InnerTask::BndBuild(t)
            | InnerTask::Convgeneric(t)
            | InnerTask::Crunch(t)
            | InnerTask::Cp(t)
            | InnerTask::Mv(t)
            | InnerTask::CpcToImg(t)
            | InnerTask::Disassembler(_, t)
            | InnerTask::Disc(t)
            | InnerTask::Echo(t)
            | InnerTask::Catalog(t)
            | InnerTask::Locomotive(t)
            | InnerTask::Emulator(_, t)
            | InnerTask::Extern(t)
            | InnerTask::Grafx2(t)
            | InnerTask::Fade(t)
            | InnerTask::Hideur(t)
            | InnerTask::HspCompiler(t)
            | InnerTask::Hxcfe(t)
            | InnerTask::ImgToCpc(t)
            | InnerTask::ImpDsk(t)
            | InnerTask::Martine(t)
            | InnerTask::Mkdir(t)
            | InnerTask::Rm(t)
            | InnerTask::Snapshot(t)
            | InnerTask::SongConverter(_, t)
            | InnerTask::Tracker(_, t)
            | InnerTask::Xfer(t)
            | InnerTask::Cpr(t)
            | InnerTask::Csl(t) => t
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

    /// Returns true if the task is "phony" (doesn't produce a file output).
    /// Phony tasks include: Echo, Emulator, Grafx2, Tracker, Xfer
    pub fn is_phony(&self) -> bool {
        match self {
            // Explicitly phony tasks (don't produce files)
            InnerTask::Echo(_) => true,
            InnerTask::Emulator(..) => true,
            InnerTask::Grafx2(_) => true,
            InnerTask::Tracker(..) => true,
            InnerTask::Xfer(_) => true,
            // All other tasks produce files
            _ => false
        }
    }

    /// Returns the kind of task based on its implementation
    pub fn kind(&self) -> TaskKind {
        match self {
            // Embedded tasks - implemented directly in bndbuild
            InnerTask::Assembler(Assembler::Basm, _) => TaskKind::Embedded,
            InnerTask::BasmDoc(_) => TaskKind::Embedded,
            InnerTask::BndBuild(_) => TaskKind::Embedded,
            InnerTask::Catalog(_) => TaskKind::Embedded,
            InnerTask::CpcToImg(_) => TaskKind::Embedded,
            InnerTask::Cpr(_) => TaskKind::Embedded,
            InnerTask::Csl(_) => TaskKind::Embedded,
            InnerTask::Crunch(_) => TaskKind::Embedded,
            InnerTask::Disassembler(Disassembler::Bdasm, _) => TaskKind::Embedded,
            InnerTask::Disc(_) => TaskKind::Embedded,
            InnerTask::Fade(_) => TaskKind::Embedded,
            InnerTask::Hideur(_) => TaskKind::Embedded,
            InnerTask::Hxcfe(_) => TaskKind::Embedded,
            InnerTask::ImgToCpc(_) => TaskKind::Embedded,
            InnerTask::Locomotive(_) => TaskKind::Embedded,
            InnerTask::Snapshot(_) => TaskKind::Embedded,
            InnerTask::Xfer(_) => TaskKind::Embedded,
            InnerTask::Cdt(crate::runners::cdt::CdtManager::Rtzx, _) => TaskKind::Embedded,
            InnerTask::Cdt(crate::runners::cdt::CdtManager::TwoCdt, _) => TaskKind::Delegated,

            // Emulated tasks - run through an emulator
            InnerTask::Assembler(Assembler::Orgams, _) => TaskKind::Emulated,

            // Delegated tasks - external programs that need to be installed
            InnerTask::Assembler(Assembler::Extern(_), _) => TaskKind::Delegated,
            InnerTask::Convgeneric(_) => TaskKind::Delegated,
            InnerTask::Disassembler(Disassembler::Extern(_), _) => TaskKind::Delegated,
            InnerTask::Emulator(..) => TaskKind::Delegated,
            InnerTask::Grafx2(_) => TaskKind::Delegated,
            InnerTask::HspCompiler(_) => TaskKind::Delegated,
            InnerTask::ImpDsk(_) => TaskKind::Delegated,
            InnerTask::Martine(_) => TaskKind::Delegated,
            InnerTask::SongConverter(..) => TaskKind::Delegated,
            InnerTask::Tracker(..) => TaskKind::Delegated,
            InnerTask::YmCruncher(..) => TaskKind::Delegated,

            // File system operations - could be considered embedded
            InnerTask::Cp(_) | InnerTask::Mv(_) | InnerTask::Mkdir(_) | InnerTask::Rm(_) => {
                TaskKind::Embedded
            },

            // Utilities
            InnerTask::Echo(_) => TaskKind::Embedded,
            InnerTask::Extern(_) => TaskKind::Delegated
        }
    }

    /// Returns an iterator over all possible InnerTask variants for testing
    pub fn all() -> impl Iterator<Item = Self> {
        let empty_args = StandardTaskArguments::new("");

        // Collect all tasks into a vec
        let mut tasks = Vec::new();

        // Add all assemblers (including external ones)
        for assembler in Assembler::all() {
            tasks.push(Self::Assembler(assembler, empty_args.clone()));
        }

        // Add all disassemblers (including external ones)
        for disassembler in Disassembler::all() {
            tasks.push(Self::Disassembler(disassembler, empty_args.clone()));
        }

        // Add all emulators
        for emulator in Emulator::all() {
            tasks.push(Self::Emulator(emulator, empty_args.clone()));
        }

        // Add all CDT managers
        for cdt in CdtManager::all() {
            tasks.push(Self::Cdt(cdt, empty_args.clone()));
        }

        // Add all trackers
        for tracker in Tracker::all() {
            tasks.push(Self::Tracker(tracker, empty_args.clone()));
        }

        // Add all song converters
        for converter in SongConverter::all() {
            tasks.push(Self::SongConverter(converter, empty_args.clone()));
        }

        // Add all YM crunchers
        for cruncher in YmCruncher::all() {
            tasks.push(Self::YmCruncher(cruncher, empty_args.clone()));
        }

        // Add simple tasks (no enum variants)
        tasks.extend(vec![
            Self::BasmDoc(empty_args.clone()),
            Self::BndBuild(empty_args.clone()),
            Self::Catalog(empty_args.clone()),
            Self::Convgeneric(empty_args.clone()),
            Self::Cp(empty_args.clone()),
            Self::CpcToImg(empty_args.clone()),
            Self::Cpr(empty_args.clone()),
            Self::Csl(empty_args.clone()),
            Self::Crunch(empty_args.clone()),
            Self::Disc(empty_args.clone()),
            Self::Echo(empty_args.clone()),
            Self::Extern(empty_args.clone()),
            Self::Fade(empty_args.clone()),
            Self::Grafx2(empty_args.clone()),
            Self::Hideur(empty_args.clone()),
            Self::HspCompiler(empty_args.clone()),
            Self::Hxcfe(empty_args.clone()),
            Self::ImgToCpc(empty_args.clone()),
            Self::ImpDsk(empty_args.clone()),
            Self::Locomotive(empty_args.clone()),
            Self::Martine(empty_args.clone()),
            Self::Mkdir(empty_args.clone()),
            Self::Mv(empty_args.clone()),
            Self::Rm(empty_args.clone()),
            Self::Snapshot(empty_args.clone()),
            Self::Xfer(empty_args.clone()),
        ]);

        tasks.into_iter()
    }
}

#[derive(Deserialize, Clone, PartialEq, Debug, Eq, Hash)]
pub struct StandardTaskArguments {
    pub(crate) args: String,
    #[serde(skip)]
    original_args: Option<String>,
    ignore_error: bool
}

impl StandardTaskArguments {
    pub fn new<S: Into<String>>(args: S) -> Self {
        Self {
            args: args.into(),
            original_args: None,
            ignore_error: false
        }
    }

    /// Get the original arguments before variable replacement, if available
    pub fn original_args(&self) -> Option<&str> {
        self.original_args.as_deref()
    }

    /// This method modify the args to replace automatic variables by the expected values
    pub fn replace_automatic_variables(
        &mut self,
        first_dep: Option<&Utf8Path>,
        first_tgt: Option<&Utf8Path>
    ) -> Result<(), String> {
        static RE_FIRST_DEP: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"\${1}(?!\$)<").expect("Valid regex pattern for first dependency")
        }); // 1 repetition does not seem to work :(
        static RE_FIRST_TGT: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"\${1}(?!\$)@").expect("Valid regex pattern for first target")
        });

        // Store original args before modification
        if self.original_args.is_none() {
            self.original_args = Some(self.args.clone());
        }
        let initial = self.args.clone();

        if let Some(first_dep) = first_dep {
            #[cfg(not(target_os = "windows"))]
            let first_dep = first_dep.as_str();
            #[cfg(target_os = "windows")]
            let first_dep = first_dep.as_str().replace("\\", "\\\\");
            self.args = RE_FIRST_DEP.replace_all(&self.args, first_dep).into_owned();
        }
        else if RE_FIRST_DEP.is_match(&self.args).unwrap_or(false) {
            self.args = initial;
            return Err(format!(
                "{} contains $<, but there are no available dependencies.",
                self.args
            ));
        }

        if let Some(first_tgt) = first_tgt {
            #[cfg(not(target_os = "windows"))]
            let first_tgt = first_tgt.as_str();
            #[cfg(target_os = "windows")]
            let first_tgt = first_tgt.as_str().replace("\\", "\\\\");

            self.args = RE_FIRST_TGT.replace_all(&self.args, first_tgt).into_owned();
        }
        else if RE_FIRST_TGT.is_match(&self.args).unwrap_or(false) {
            self.args = initial;
            return Err(format!(
                "{} contains $@, but there are no available targets.",
                self.args
            ));
        }
        Ok(())
    }
}

impl From<(&cpclib_common::clap::Command, &ArgMatches)> for StandardTaskArguments {
    fn from((cmd, matches): (&cpclib_common::clap::Command, &ArgMatches)) -> Self {
        // Helper that finds the declared token for an argument id using the
        // provided `Command` metadata. Prefer short (`-x`) when available,
        // then long (`--name`), otherwise fall back to canonical `--{id}`.
        fn declared_token_for(cmd: &cpclib_common::clap::Command, id: &str) -> Option<String> {
            for a in cmd.get_arguments() {
                if a.get_id().as_str() == id {
                    // Positional arguments should not be prefixed.
                    if a.get_index().is_some() {
                        return None;
                    }

                    // Prefer short form if available.
                    if let Some(s) = a.get_short() {
                        return Some(format!("-{}", s));
                    }

                    // Prefer a visible alias if any (this matches how some
                    // basm options expose alternate long names such as
                    // `--ace` for `REMU_OUTPUT`). If no visible alias is
                    // present, prefer any declared alias (hidden or not),
                    // then fall back to the main long name.
                    if let Some(aliases) = a.get_visible_aliases()
                        && let Some(first) = aliases.first()
                    {
                        return Some(format!("--{}", first));
                    }

                    if let Some(all_aliases) = a.get_all_aliases()
                        && let Some(first) = all_aliases.first()
                    {
                        return Some(format!("--{}", first));
                    }

                    // Fallback to the main long name.
                    if let Some(l) = a.get_long() {
                        return Some(format!("--{}", l));
                    }
                }
            }
            // If we couldn't find a declared token (no short/long/alias),
            // treat the argument as positional and return None so the
            // value is emitted without a preceding token.
            None
        }

        fn collect(
            cmd: &cpclib_common::clap::Command,
            matches: &ArgMatches,
            out: &mut Vec<String>
        ) {
            for id in matches.ids() {
                let id_str = id.as_str();

                // Attempt multi-value first.
                if let Ok(Some(values)) = matches.try_get_many::<String>(id_str) {
                    if let Some(cpclib_common::clap::parser::ValueSource::CommandLine) =
                        matches.value_source(id_str)
                    {
                        if let Some(token) = declared_token_for(cmd, id_str) {
                            out.push(token);
                        }
                        for v in values {
                            out.push(v.clone());
                        }
                    }
                    continue;
                }

                if let Ok(Some(v)) = matches.try_get_one::<String>(id_str) {
                    if let Some(cpclib_common::clap::parser::ValueSource::CommandLine) =
                        matches.value_source(id_str)
                    {
                        if let Some(token) = declared_token_for(cmd, id_str) {
                            out.push(token);
                        }
                        out.push(v.clone());
                    }
                    continue;
                }

                if let Ok(Some(b)) = matches.try_get_one::<bool>(id_str) {
                    if *b
                        && let Some(cpclib_common::clap::parser::ValueSource::CommandLine) =
                            matches.value_source(id_str)
                        && let Some(token) = declared_token_for(cmd, id_str)
                    {
                        out.push(token);
                    }
                    continue;
                }
            }

            if let Some((sub_name, sub_matches)) = matches.subcommand() {
                out.push(sub_name.to_string());
                // Find the subcommand `Command` metadata to continue mapping
                // declared tokens correctly. If not found, fall back to the
                // parent command (we can't map more precisely).
                if let Some(sub_cmd) = cmd.get_subcommands().find(|s| s.get_name() == sub_name) {
                    collect(sub_cmd, sub_matches, out);
                }
                else {
                    collect(cmd, sub_matches, out);
                }
            }
        }

        let mut parts = Vec::new();
        collect(cmd, matches, &mut parts);
        StandardTaskArguments::new(parts.join(" "))
    }
}

impl StandardTaskArguments {
    /// Public accessor for the raw args string.
    pub fn args(&self) -> &str {
        &self.args
    }

    /// Public accessor for the ignore flag.
    pub fn ignore_error(&self) -> bool {
        self.ignore_error
    }
}

#[cfg(test)]
mod test {
    use cpclib_common::clap::{Arg, ArgAction, Command};

    use super::InnerTask;
    use crate::task::StandardTaskArguments;

    #[test]
    fn test_automatic_arguments() {
        // no replacement expected
        let mut no_args = StandardTaskArguments::new("a b");
        assert!(dbg!(
            no_args.replace_automatic_variables(None, None).is_ok()
        ));
        assert_eq!(no_args.args, "a b");

        let mut no_args = StandardTaskArguments::new("a b");
        assert!(dbg!(
            no_args
                .replace_automatic_variables(Some("a".into()), None)
                .is_ok()
        ));
        assert_eq!(no_args.args, "a b");

        let mut no_args = StandardTaskArguments::new("a b");
        assert!(dbg!(
            no_args
                .replace_automatic_variables(None, Some("b".into()))
                .is_ok()
        ));
        assert_eq!(no_args.args, "a b");

        let mut no_args = StandardTaskArguments::new("a b");
        assert!(dbg!(
            no_args
                .replace_automatic_variables(Some("a".into()), Some("b".into()))
                .is_ok()
        ));
        assert_eq!(no_args.args, "a b");

        // tgt replacement expected
        let mut no_args = StandardTaskArguments::new("$@ b");
        assert!(dbg!(
            no_args.replace_automatic_variables(None, None).is_err()
        ));
        assert_eq!(no_args.args, "$@ b");

        let mut no_args = StandardTaskArguments::new("$@ b");
        assert!(dbg!(
            no_args
                .replace_automatic_variables(Some("a".into()), None)
                .is_err()
        ));
        assert_eq!(no_args.args, "$@ b");

        let mut no_args = StandardTaskArguments::new("$@ b");
        assert!(dbg!(
            no_args
                .replace_automatic_variables(None, Some("b".into()))
                .is_ok()
        ));
        assert_eq!(no_args.args, "b b");

        let mut no_args = StandardTaskArguments::new("$@ b");
        assert!(dbg!(
            no_args
                .replace_automatic_variables(Some("a".into()), Some("b".into()))
                .is_ok()
        ));
        assert_eq!(no_args.args, "b b");

        // tgt and dep replacements expected
        let mut no_args = StandardTaskArguments::new("$@ $<");
        assert!(dbg!(
            no_args.replace_automatic_variables(None, None).is_err()
        ));
        assert_eq!(no_args.args, "$@ $<");

        let mut no_args = StandardTaskArguments::new("$@ $<");
        assert!(dbg!(
            no_args
                .replace_automatic_variables(Some("a".into()), None)
                .is_err()
        ));
        assert_eq!(no_args.args, "$@ $<");

        let mut no_args = StandardTaskArguments::new("$@ $<");
        assert!(dbg!(
            no_args
                .replace_automatic_variables(None, Some("b".into()))
                .is_err()
        ));
        assert_eq!(no_args.args, "$@ $<");

        let mut no_args = StandardTaskArguments::new("$@ $<");
        assert!(dbg!(
            no_args
                .replace_automatic_variables(Some("a".into()), Some("b".into()))
                .is_ok()
        ));
        assert_eq!(no_args.args, "b a");

        // duplicated $ change nothing
        //        this onefails but i do not understand why
        // let mut no_args = StandardTaskArguments::new("$$@ $$<");
        // assert!(dbg!(no_args.replace_automatic_variables(Some("a".into()), Some("b".into())).is_ok()));
        // assert_eq!(no_args.args, "$$@ $$<");
        //
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
                    original_args: None,
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
                    original_args: None,
                    ignore_error: true
                }
            )
        );
    }

    #[test]
    fn test_from_command_and_arguments_basm() {
        // Build standard args as if provided by Python's quoted-join (but here unquoted)
        let std = StandardTaskArguments::new("toto.asm -o toto.o");
        let t = InnerTask::from_command_and_arguments("basm", std.clone()).unwrap();
        assert_eq!(
            t,
            InnerTask::Assembler(crate::runners::assembler::Assembler::Basm, std)
        );
    }

    #[test]
    fn test_from_command_and_arguments_rm() {
        let std = StandardTaskArguments::new("file1.txt");
        let t = InnerTask::from_command_and_arguments("rm", std.clone()).unwrap();
        assert_eq!(t, InnerTask::Rm(std));
    }

    #[test]
    fn test_from_argmatches_simple() {
        // Build a command with a couple of options, a flag and a subcommand
        let cmd = Command::new("prog")
            .arg(Arg::new("input").long("input").num_args(1))
            .arg(Arg::new("opt").long("opt").num_args(1))
            .arg(Arg::new("flag").long("flag").action(ArgAction::SetTrue))
            .subcommand(Command::new("sub").arg(Arg::new("subarg").long("subarg").num_args(1)));

        let argv = [
            "prog", "--input", "a.bin", "--opt", "x", "--flag", "sub", "--subarg", "y"
        ];

        let matches = cmd.clone().get_matches_from(&argv);

        let std = StandardTaskArguments::from((&cmd, &matches));
        let expected = argv[1..].join(" ");
        assert_eq!(std.args(), expected.as_str());
    }

    #[test]
    fn test_from_argmatches_multiple_values() {
        // repeated occurrences should be collected in order
        let cmd = Command::new("p").arg(Arg::new("list").long("list").num_args(1..));
        // provide two values in a single occurrence: `--list a b`
        let argv = ["p", "--list", "a", "b"];
        let matches = cmd.clone().get_matches_from(&argv);
        let std = StandardTaskArguments::from((&cmd, &matches));
        let expected = argv[1..].join(" ");
        assert_eq!(std.args(), expected.as_str());
    }

    #[test]
    fn test_from_argmatches_basm_command() {
        // Use the real basm command parser to ensure we don't reinvent the wheel.
        let cmd = cpclib_basm::build_args_parser();

        let argv = [
            "basm",
            "src/demosystem/private.asm",
            "-o",
            "demosystem.o",
            "--sym",
            "demosystem.sym",
            "--ace",
            "demosystem.rasm",
            "--lst",
            "demosystem.lst"
        ];

        let matches = cmd.clone().get_matches_from(&argv);
        let std = StandardTaskArguments::from((&cmd, &matches));
        let expected = argv[1..].join(" ");
        assert_eq!(std.args(), expected.as_str());
    }

    #[test]
    fn test_from_argmatches_filename() {
        // Ensure a filename argument is preserved
        let cmd = Command::new("prog").arg(Arg::new("file").long("file").num_args(1));
        let argv = ["prog", "--file", "path/to/some-file.txt"];
        let matches = cmd.clone().get_matches_from(&argv);
        let std = StandardTaskArguments::from((&cmd, &matches));
        let expected = argv[1..].join(" ");
        assert_eq!(std.args(), expected.as_str());
    }

    #[test]
    fn test_from_argmatches_nested_subcommand() {
        // Nested subcommands should be collected in order: outer inner <args>
        let inner = Command::new("inner").arg(Arg::new("msg").long("msg").num_args(1));
        let outer = Command::new("prog").subcommand(Command::new("outer").subcommand(inner));

        let argv = ["prog", "outer", "inner", "--msg", "hello"];
        let matches = outer.clone().get_matches_from(&argv);
        let std = StandardTaskArguments::from((&outer, &matches));
        let expected = argv[1..].join(" ");
        assert_eq!(std.args(), expected.as_str());
    }
}
