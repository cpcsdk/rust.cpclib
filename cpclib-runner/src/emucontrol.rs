use std::collections::HashSet;
use std::fmt::Display;
use std::io::Read;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;
use std::time::Duration;

use clap::{ArgAction, Command, CommandFactory, Parser, Subcommand, ValueEnum, value_parser};
use cpclib_common::camino::{Utf8Path, Utf8PathBuf};
use cpclib_common::itertools::Itertools;
use cpclib_common::parse_value;
use cpclib_csl::ResetType;
use delegate;
use enigo::{Enigo, Key, Keyboard, Settings};
#[cfg(windows)]
use fs_extra;
#[cfg(feature = "screenshot")]
use xcap::image::{ImageBuffer, Rgba, open};

use crate::ace_config::{AceConfig, AceConfigFlag};
use crate::delegated::{DelegatedRunner, clear_base_cache_folder};
use crate::embedded::EmbeddedRoms;
use crate::event::EventObserver;
use crate::runner::Runner;
use crate::runner::emulator::Emulator;
#[cfg(feature = "screenshot")]
use crate::runner::emulator::amspiritlite_api;
#[cfg(feature = "screenshot")]
use crate::runner::emulator::sugarbox_api;
use crate::runner::exec::RunnerWithClap;

#[cfg(feature = "screenshot")]
type Screenshot = ImageBuffer<Rgba<u8>, Vec<u8>>;

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq)]
pub enum AmstradRom {
    Orgams,
    Unidos
}

#[derive(Debug, Default, Clone, Copy)]
#[repr(u8)]
pub enum Crtc {
    #[default]
    Zero = 0,
    One,
    Two,
    Three,
    Four
}

impl Display for Crtc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let val = *self as u8;
        write!(f, "{val}")
    }
}

impl TryFrom<u8> for Crtc {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Crtc::Zero),
            1 => Ok(Crtc::One),
            2 => Ok(Crtc::Two),
            3 => Ok(Crtc::Three),
            4 => Ok(Crtc::Four),

            val => Err(format!("{val} is not a valid CRTC value"))
        }
    }
}

impl FromStr for Crtc {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "0" | "zero" => Ok(Crtc::Zero),
            "1" | "one" => Ok(Crtc::One),
            "2" | "two" => Ok(Crtc::Two),
            "3" | "three" => Ok(Crtc::Three),
            "4" | "four" => Ok(Crtc::Four),

            val => Err(format!("{val} is not a valid CRTC value"))
        }
    }
}

impl Crtc {
    /// Convert to CSL CrtcModel
    pub fn to_csl_model(self) -> cpclib_csl::CrtcModel {
        match self {
            Crtc::Zero => cpclib_csl::CrtcModel::Type0,
            Crtc::One => cpclib_csl::CrtcModel::Type1,
            Crtc::Two => cpclib_csl::CrtcModel::Type2,
            Crtc::Three => cpclib_csl::CrtcModel::Type3,
            Crtc::Four => cpclib_csl::CrtcModel::Type4
        }
    }
}

/// Convert memory size (in KB) to CSL MemoryExpansion
fn memory_to_csl_expansion(memory: u32) -> cpclib_csl::MemoryExpansion {
    match memory {
        128 => cpclib_csl::MemoryExpansion::Kb128,
        256 => cpclib_csl::MemoryExpansion::Kb256Standard,
        512 => cpclib_csl::MemoryExpansion::Kb512DkTronics,
        4096 => cpclib_csl::MemoryExpansion::Mb4,
        _ => cpclib_csl::MemoryExpansion::Kb128 // default
    }
}

#[cfg(feature = "screenshot")]
type EmuScreenShot = ImageBuffer<Rgba<u8>, Vec<u8>>;

#[derive(Debug)]
pub enum EmuWindow {
    #[cfg(feature = "screenshot")]
    Xcap(xcap::Window),
    Xvfb(usize, Option<wmctrl::Window>)
}

impl EmuWindow {
    #[cfg(feature = "screenshot")]
    pub fn capture_image(&self) -> EmuScreenShot {
        match self {
            #[cfg(feature = "screenshot")]
            EmuWindow::Xcap(window) => window.capture_image().unwrap(),
            EmuWindow::Xvfb(_display, window) => {
                match window {
                    Some(window) => {
                        let _cmd = std::process::Command::new("xwd")
                            .args(["-name", window.title(), "-out", "/tmp/screen.xwd"])
                            .output()
                            .unwrap();

                        unimplemented!()
                    },

                    None => {
                        let _cmd = std::process::Command::new("xwd")
                            .args(["-out", "/tmp/screen.xwd"])
                            .output()
                            .unwrap();

                        unimplemented!()
                    }
                }
            },
        }
    }
}

pub(crate) enum WindowEventsManager {
    Enigo(Enigo)
}

impl From<Enigo> for WindowEventsManager {
    fn from(value: Enigo) -> Self {
        Self::Enigo(value)
    }
}

#[derive(Clone, Debug, Copy)]
pub enum HostKey {
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    Ascii(char),
    Return
}

impl HostKey {
    pub fn enigo(&self) -> (Option<enigo::Key>, enigo::Key) {
        match self {
            Self::F1 => (None, Key::F1),
            Self::F2 => (None, Key::F2),
            Self::F3 => (None, Key::F3),
            Self::F4 => (None, Key::F4),
            Self::F5 => (None, Key::F5),
            Self::F6 => (None, Key::F6),
            Self::F7 => (None, Key::F7),
            Self::F8 => (None, Key::F8),
            Self::F9 => (None, Key::F9),
            Self::F10 => (None, Key::F10),
            Self::F11 => (None, Key::F11),
            Self::F12 => (None, Key::F12),
            Self::Ascii(c) => {
                // handle boring French keyboard ?
                if *c == '1' {
                    (Some(Key::Shift), Key::Unicode('&'))
                }
                else if *c == '2' {
                    (Some(Key::Shift), Key::Unicode('é'))
                }
                else if *c == '3' {
                    (Some(Key::Shift), Key::Unicode('"'))
                }
                else if *c == '4' {
                    (Some(Key::Shift), Key::Unicode('\''))
                }
                else if *c == '5' {
                    (Some(Key::Shift), Key::Unicode('('))
                }
                else if *c == '6' {
                    (Some(Key::Shift), Key::Unicode('-'))
                }
                else if *c == '7' {
                    (Some(Key::Shift), Key::Unicode('è'))
                }
                else if *c == '8' {
                    (Some(Key::Shift), Key::Unicode('_'))
                }
                else if *c == '9' {
                    (Some(Key::Shift), Key::Unicode('ç'))
                }
                else if *c == '0' {
                    (Some(Key::Shift), Key::Unicode('à'))
                }
                else if *c == '?' {
                    (Some(Key::Shift), Key::Unicode(','))
                }
                else if *c == '.' {
                    (Some(Key::Shift), Key::Unicode(';'))
                }
                else if *c == '/' {
                    (Some(Key::Shift), Key::Unicode(':'))
                }
                else if *c == '§' {
                    (Some(Key::Shift), Key::Unicode('!'))
                }
                else if *c == '%' {
                    (Some(Key::Shift), Key::Unicode('ù'))
                }
                else if *c == '£' {
                    (Some(Key::Shift), Key::Unicode('$'))
                }
                else if *c == '+' {
                    (Some(Key::Shift), Key::Unicode('='))
                }
                else if c.is_ascii_uppercase() {
                    (Some(Key::Shift), Key::Unicode(c.to_ascii_lowercase()))
                }
                else {
                    (None, Key::Unicode(*c))
                }
            },
            Self::Return => (None, Key::Return)
        }
    }
}

impl HostKey {
    pub fn char(&self) -> Option<char> {
        match self {
            Self::Ascii(c) => Some(*c),
            _ => None
        }
    }
}

#[derive(Clone, Debug)]
pub struct HostKeys(Vec<HostKey>);

impl From<char> for HostKey {
    fn from(value: char) -> Self {
        if value == '\n' {
            HostKey::Return
        }
        else {
            HostKey::Ascii(value)
        }
    }
}

impl Deref for HostKeys {
    type Target = Vec<HostKey>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<&str> for HostKeys {
    fn from(value: &str) -> Self {
        Self(value.chars().map(|c| c.into()).collect_vec())
    }
}

impl WindowEventsManager {
    pub fn wait_a_bit() {
        std::thread::sleep(Duration::from_millis(1000 / 20));
    }

    pub fn alt_key<K: Into<HostKey>>(&mut self, key: K) {
        let key = key.into();
        match self {
            Self::Enigo(_enigo) => {
                self.enigo_press_with_extra(Key::Alt, key);
            }
        }
    }

    // TODO check why it is not written as the ALT version
    pub fn ctrl_char<K: Into<HostKey>>(&mut self, c: K) {
        let c = c.into();
        match self {
            Self::Enigo(_enigo) => {
                self.enigo_press_with_extra(Key::Control, c);
            }
        }
    }

    fn enigo_press_with_extra<K: Into<HostKey>>(&mut self, extra: Key, c: K) {
        let c = c.into();

        match self {
            Self::Enigo(enigo) => {
                let (extra2, c) = c.enigo();
                if let Some(extra2) = extra2
                    && extra2 != extra
                {
                    // Left as eprintln!: `WindowEventsManager`/`RobotImpl` have
                    // no `EventObserver` in scope, and this low-level keystroke
                    // helper is reached from many Robot methods (`close`,
                    // `unidos_select_drive`, `type_text`, ...) that don't carry
                    // one either - threading it through would touch the whole
                    // keystroke-simulation surface for one rare diagnostic.
                    eprintln!("{c:?} requires a different modifier than {extra:?}");
                }

                enigo.key(extra, enigo::Direction::Press).unwrap();
                Self::wait_a_bit();
                enigo.key(c, enigo::Direction::Press).unwrap();
                Self::wait_a_bit();
                enigo.key(c, enigo::Direction::Release).unwrap();
                Self::wait_a_bit();
                enigo.key(extra, enigo::Direction::Release).unwrap();
                Self::wait_a_bit();
            }
        }
    }

    pub fn type_text<T: Into<HostKeys>>(&mut self, txt: T) {
        let txt = txt.into();
        match self {
            Self::Enigo(_enigo) => {
                // asking enigo to write the full char does not work at all
                for k in txt.iter() {
                    self.type_key(*k)
                }
            }
        }
    }

    pub fn type_char(&mut self, c: char) {
        match self {
            Self::Enigo(_enigo) => self.type_key(c)
        }
    }

    pub fn type_key<K: Into<HostKey>>(&mut self, k: K) {
        let k = k.into();
        match self {
            Self::Enigo(_) => {
                let (meta, key) = k.enigo();
                if let Some(meta) = meta {
                    self.enigo_press_with_extra(meta, k);
                }
                else {
                    self.enigo_click_key(key);
                }
            }
        }
    }

    pub fn r#return(&mut self) {
        self.type_char('\n')
    }

    fn enigo_click_key(&mut self, key: Key) {
        match self {
            // TODO really do this way ? This is more complex than expected
            Self::Enigo(enigo) => {
                // enigo.key(key, Direction::Click).unwrap(); // this does not work :(

                enigo.key(key, enigo::Direction::Press).unwrap();
                Self::wait_a_bit();
                enigo.key(key, enigo::Direction::Release).unwrap();
                Self::wait_a_bit();
            }
        }
    }

    // #[cfg(windows)]
    // fn click_key(&mut self, key: Key) {
    // dbg!(&key);
    //
    // #[cfg(windows)]
    // match key {
    // https://boostrobotics.eu/windows-key-codes/
    // Key::Unicode(v) if v.is_ascii_digit() => {
    // if false {
    // let nb = v as u32 - '0' as u32;
    //
    // self.enigo
    // .key(Key::RShift, enigo::Direction::Press)
    // .unwrap();
    // Self::wait_a_bit();
    // Self::wait_a_bit();
    //
    // let lut = ['à', '&', 'é', '"', '\'', '(', '-', 'è', '_', 'ç'][nb as usize];
    // dbg!(nb, lut);
    // let key = Key::Unicode(lut);
    //
    // self.enigo.key(key, enigo::Direction::Press).unwrap();
    // Self::wait_a_bit();
    // Self::wait_a_bit();
    // self.enigo.key(key, enigo::Direction::Release).unwrap();
    //
    // self.enigo
    // .key(Key::RShift, enigo::Direction::Release)
    // .unwrap();
    // Self::wait_a_bit();
    // }
    //
    // self.enigo.text(dbg!(&format!("{v}"))).unwrap();
    // },
    // _ => {
    // dbg!(key);
    // self.enigo.key(key, enigo::Direction::Press).unwrap();
    // Self::wait_a_bit();
    // self.enigo.key(key, enigo::Direction::Release).unwrap();
    // Self::wait_a_bit();
    // }
    // };
    // }
}

/// It seems ACE-DL is not able to read several rasm debug file. Only the latest one is taken into account.
/// So here we merge them
pub fn merge_rasm_debug<P: AsRef<Utf8Path>>(from: &[P]) -> std::io::Result<String> {
    // collect all of them
    let mut content = String::new();
    for (i, fname) in from.iter().enumerate() {
        fs_err::File::open(fname.as_ref())?.read_to_string(&mut content)?;
        if i != from.len() - 1 && !content.ends_with(';') {
            content.push(';')
        }
    }

    // filter duplicates
    // TODO no semantic of the file is used so it will fail with the presence of any ';'
    let mut content = content.split(';').sorted().unique().join(";");
    content.push(';');

    Ok(content)
}

/// Marge all .rasm files in a temporary file
pub fn merge_rasm_debug_to_tmp<P: AsRef<Utf8Path>>(from: &[P]) -> std::io::Result<Utf8PathBuf> {
    let content = merge_rasm_debug(from)?;

    let tempfile = camino_tempfile::Builder::new()
        .suffix("_merged.rasm")
        .tempfile()
        .unwrap();
    let path = tempfile.into_temp_path();
    let path = path.keep().unwrap();
    fs_err::write(&path, content)?;

    Ok(path)
}

/// Read a rasm debug file and convert it in winape sym string
pub fn rasm_debug_to_winape_sym(src: &Utf8Path) -> std::io::Result<String> {
    let content = fs_err::read_to_string(src)?;
    Ok(content
        .split(";")
        .filter(|code| code.starts_with("label") | code.starts_with("alias"))
        .map(|code| {
            let mut spliter = code.split_ascii_whitespace();
            spliter.next(); // consume alias or label

            let label = spliter.next().unwrap().replace('.', "_");
            let value = spliter.next().unwrap();
            let value = &mut value.as_bytes();
            let value = parse_value::<_, ()>(value).unwrap();

            format!("{label} #{value:.X}")
        })
        .join("\n"))
}

#[derive(Debug, Clone, bon::Builder)]
pub struct EmulatorConf {
    pub drive_a: Option<Utf8PathBuf>,
    pub drive_b: Option<Utf8PathBuf>,
    pub snapshot: Option<Utf8PathBuf>,

    #[builder(default)]
    pub roms_configuration: HashSet<AmstradRom>,

    #[builder(default)]
    pub debug_files: Vec<Utf8PathBuf>,
    pub break_on_bad_vbl: bool,
    pub break_on_bad_hbl: bool,

    /// The file name to launch automatically
    pub auto_run: Option<String>,

    /// The file that contains the command to automaticall type
    pub auto_type: Option<Utf8PathBuf>,

    pub memory: Option<u32>,

    pub crtc: Option<Crtc>,

    /// Do not display the window
    pub transparent: bool
}

impl EmulatorConf {
    /// Convert HFE drive to DSK if needed for emulators that don't support HFE
    #[cfg(feature = "hfe")]
    fn convert_drive_if_needed(
        drive: &Option<Utf8PathBuf>,
        emu: &Emulator
    ) -> Result<Option<Utf8PathBuf>, String> {
        if let Some(drive_path) = drive {
            if drive_path
                .extension()
                .map(|ext| ext.eq_ignore_ascii_case("hfe"))
                .unwrap_or(false)
                && !emu.accept_hfe()
            {
                // Convert HFE to temporary DSK
                let tempfile = camino_tempfile::Builder::new()
                    .suffix(".dsk")
                    .tempfile()
                    .map_err(|e| format!("Failed to create temporary DSK file: {}", e))?;

                let dsk_path = tempfile.into_temp_path();

                // Keep the temporary file and get the path as Utf8PathBuf
                let dsk_path_utf8 = dsk_path
                    .keep()
                    .map_err(|e| format!("Failed to keep temporary DSK file: {}", e))?;

                cpclib_disc::convert_hfe_to_dsk(drive_path, &dsk_path_utf8)?;

                Ok(Some(dsk_path_utf8))
            }
            else {
                Ok(Some(drive_path.clone()))
            }
        }
        else {
            Ok(None)
        }
    }

    /// Check if drive uses HFE format when feature is not enabled
    #[cfg(not(feature = "hfe"))]
    fn check_hfe_not_supported(drive: &Option<Utf8PathBuf>) -> Result<Option<Utf8PathBuf>, String> {
        if let Some(drive_path) = drive {
            if drive_path
                .extension()
                .map(|ext| ext.eq_ignore_ascii_case("hfe"))
                .unwrap_or(false)
            {
                return Err(format!(
                    "HFE format is not supported. File '{}' requires the 'hfe' feature. \
                    Please rebuild with --features hfe or use a DSK file instead.",
                    drive_path
                ));
            }
            Ok(Some(drive_path.clone()))
        }
        else {
            Ok(None)
        }
    }

    /// Generate the args for the corresponding emulator
    pub fn args_for_emu(
        &self,
        emu: &Emulator,
        o: &dyn EventObserver
    ) -> Result<Vec<String>, String> {
        // Emulators that natively accept a CSL file get one synthesized
        // from this whole config, instead of per-field ad-hoc CLI args.
        if emu.accept_csl() {
            return self.args_for_emu_amspirit_with_csl(emu);
        }

        // Convert HFE to DSK if needed
        #[cfg(feature = "hfe")]
        let drive_a = Self::convert_drive_if_needed(&self.drive_a, emu)?;

        #[cfg(not(feature = "hfe"))]
        let drive_a = Self::check_hfe_not_supported(&self.drive_a)?;

        #[cfg(feature = "hfe")]
        let drive_b = Self::convert_drive_if_needed(&self.drive_b, emu)?;

        #[cfg(not(feature = "hfe"))]
        let drive_b = Self::check_hfe_not_supported(&self.drive_b)?;

        let mut args = Vec::default();

        // AMSpiriT Lite's screenshot() (below, on AmspiritUsedEmulator)
        // needs to know the exact port its HTTP debug server is listening
        // on - pin it explicitly rather than trusting whatever
        // `amspirit-lite.conf` the current user happens to have saved.
        #[cfg(feature = "screenshot")]
        if let Emulator::AmspiritLite(_) = emu {
            args.push("--web-port".to_owned());
            args.push(AMSPIRIT_LITE_ROBOT_WEB_PORT.to_string());
        }

        if let Some(drive_a) = &drive_a {
            match emu {
                Emulator::Ace(_)
                | Emulator::CpcEmu(_)
                | Emulator::Cpcec(_)
                | Emulator::RetroVm(_)
                | Emulator::Cadence(_) => args.push(drive_a.to_string()),
                // `amspirit-lite [OPTION...] [FILE]` - the medium is the
                // trailing positional argument, whatever kind it is.
                Emulator::AmspiritLite(_) => args.push(drive_a.to_string()),
                Emulator::SugarBoxV2(_) => args.push(drive_a.to_string()),
                Emulator::Winape(_) | Emulator::Amspirit(_) => {
                    args.push(emu.wine_compatible_fname(drive_a)?.to_string())
                },
                Emulator::CpcEmuPower(_cpc_emu_power_version) => {
                    args.push(format!("--dsk0={}", emu.wine_compatible_fname(drive_a)?))
                },
                Emulator::CapriceForever(_) => {
                    args.push(format!("/DriveA={}", emu.wine_compatible_fname(drive_a)?))
                },
                Emulator::Emulator1984(_) => {
                    args.push(format!("--disk-a={drive_a}"));
                }
            }
        }

        if let Some(drive_b) = &drive_b {
            match emu {
                Emulator::Ace(_) => return Err("Drive B not yet handled".to_owned()),
                Emulator::CpcEmu(_) => return Err("Drive B not yet handled".to_owned()),
                Emulator::Cpcec(_) => return Err("Drive B not yet handled".to_owned()),
                Emulator::Winape(_) => return Err("Drive B not yet handled".to_owned()),
                Emulator::Amspirit(_) => return Err("Drive B not yet handled".to_owned()),
                Emulator::AmspiritLite(_) => return Err("Drive B not yet handled".to_owned()),
                Emulator::SugarBoxV2(_) => return Err("Drive B not yet handled".to_owned()),
                Emulator::CpcEmuPower(_cpc_emu_power_version) => {
                    args.push(format!("--dsk1={}", emu.wine_compatible_fname(drive_b)?))
                },
                Emulator::CapriceForever(_) => args.push(format!("/DriveB={drive_b}")),
                Emulator::RetroVm(_) => return Err("Drive B not yet handled".to_owned()),
                Emulator::Cadence(_) => return Err("Drive B not yet handled".to_owned()),
                Emulator::Emulator1984(_) => args.push(format!("--disk-b={drive_b}"))
            }
        }

        if let Some(sna) = &self.snapshot {
            match emu {
                Emulator::Ace(_ace_version) => args.push(sna.to_string()),
                Emulator::CpcEmu(_) => args.push(sna.to_string()),
                Emulator::Cpcec(_cpcec_version) => args.push(sna.to_string()),
                Emulator::SugarBoxV2(_) => args.push(sna.to_string()),
                Emulator::Winape(_winape_version) => {
                    let fname = emu.wine_compatible_fname(sna)?;
                    args.push(format!("/SN:{fname}"));
                },
                Emulator::Amspirit(_v) => {
                    let fname = emu.wine_compatible_fname(sna)?;
                    args.push(format!("--file={fname}"));
                },
                // Not `--file=`: the Lite build takes the medium as its
                // trailing positional argument and has no such option.
                Emulator::AmspiritLite(_) => args.push(sna.to_string()),
                Emulator::CpcEmuPower(_v) => {
                    args.push(format!("--sna={sna}"));
                },
                Emulator::CapriceForever(_v) => {
                    args.push(format!("/SNA=\"{sna}\""));
                },
                Emulator::RetroVm(_) => args.push(sna.to_string()),
                Emulator::Cadence(_) => args.push(sna.to_string()),
                Emulator::Emulator1984(_) => {
                    o.emit_stderr("snapshot loading is currently ignored for 1984\n");
                }
            }
        }

        if let Some(crtc) = &self.crtc
            && let Emulator::CpcEmuPower(_) = emu
        {
            args.push(format!("--crtc={crtc}"));
        }

        // is it really usefull ? seems it is really done by playing with the conf files
        if !self.roms_configuration.is_empty() {
            match emu {
                Emulator::AmspiritLite(_) => {
                    // `-R/--rom-path` points at a directory of ROMs rather than
                    // naming them one by one, so a per-slot configuration has
                    // nowhere to go.
                    return Err("AMSpiriT Lite takes a ROM *directory* (--rom-path), not \
                         individual ROM slots"
                        .to_owned());
                },
                Emulator::Ace(_)
                | Emulator::CpcEmu(_)
                | Emulator::Cpcec(_)
                | Emulator::Winape(_)
                | Emulator::Amspirit(_)
                | Emulator::SugarBoxV2(_)
                | Emulator::CpcEmuPower(_)
                | Emulator::CapriceForever(_)
                | Emulator::RetroVm(_)
                | Emulator::Cadence(_)
                | Emulator::Emulator1984(_) => {
                    return Err(format!(
                        "ROM configuration is not yet supported for {emu:?}"
                    ));
                }
            }
        }

        if !self.debug_files.is_empty() {
            match emu {
                Emulator::Ace(_) => {
                    let fname = merge_rasm_debug_to_tmp(&self.debug_files[..]).unwrap();
                    args.push(fname.to_string());
                },
                Emulator::Winape(_) => {
                    o.emit_stderr(
                        "Breapoints are currently ignored. TODO convert them in the appropriate format\n"
                    );
                    let mut sym_string = String::new();
                    for rasm_fname in &self.debug_files {
                        if !sym_string.is_empty() && !sym_string.ends_with('\n') {
                            sym_string.push('\n');
                        }
                        sym_string.push_str(&rasm_debug_to_winape_sym(rasm_fname).unwrap());
                    }

                    if !sym_string.is_empty() {
                        let tempfile = camino_tempfile::Builder::new()
                            .suffix(".winape.sym")
                            .tempfile()
                            .unwrap();
                        let path = tempfile.into_temp_path();
                        let path = path.keep().unwrap();
                        fs_err::write(&path, sym_string).unwrap();
                        let fname = emu.wine_compatible_fname(&path)?;
                        args.push(format!("/SYM:{fname}"));
                    }
                },
                _ => {
                    o.emit_stderr(
                        "Debug files are currently ignored. TODO convert them in the appropriate format\n"
                    )
                }
            }
        }

        if let Some(memory) = &self.memory
            && let Emulator::Cpcec(_) = emu
        {
            let arg = match memory {
                64 => "-k0",
                128 => "-k1",
                192 => "-k2",
                320 => "-k3",
                576 => "-k4",
                1088 => "-k5",
                2112 => "-k6",
                _ => {
                    return Err(format!(
                        "Unsupported memory size {memory}KB for CPCEC (expected one of \
                         64/128/192/320/576/1088/2112)"
                    ));
                }
            };
            args.push(arg.to_owned());
        }

        if let Some(ftype) = &self.auto_type {
            match emu {
                Emulator::Ace(_) => {
                    if let Some(ext) = ftype.extension()
                        && ext == "txt"
                    {
                        args.push(ftype.as_str().to_string());
                    }
                    else {
                        return Err(format!("`{ftype}` should end by .txt"));
                    }
                },
                Emulator::Emulator1984(_) => {
                    let text = fs_err::read_to_string(ftype)
                        .map_err(|e| format!("Failed to read auto-type file `{ftype}`: {e}"))?;
                    let text = text.replace('\n', "\\n");
                    args.push(format!("--paste={text}"));
                },
                _ => {
                    o.emit_stderr(&format!(
                        "Auto type file is currently ignored for this emulator {:?}\n",
                        emu
                    ))
                }
            }
        }

        if let Some(run) = &self.auto_run {
            match emu {
                Emulator::Ace(_) => {
                    args.push("-autoRunFile".to_owned());
                    args.push(run.clone())
                },
                Emulator::CpcEmu(_) => {
                    o.emit_stderr("auto_run is currently ignored for CPCEmu\n");
                },
                Emulator::Winape(_) => {
                    args.push(format!("/A:{run}"));
                },
                Emulator::Cpcec(_) => {
                    // is it automatic ?
                },
                Emulator::Amspirit(_) => {
                    args.push(format!("--run={run}"));
                },
                // `-T/--run <name>` types `RUN"<name>`; the short `-A` form
                // runs the first program on the medium instead.
                Emulator::AmspiritLite(_) => {
                    args.push(format!("--run={run}"));
                },
                Emulator::SugarBoxV2(_) => {
                    o.emit_stderr("auto_run is currently ignored for SugarBox v2\n");
                },
                Emulator::CpcEmuPower(_) => args.push(format!("--auto=RUN\"{run}")),
                Emulator::CapriceForever(_v) => {
                    args.push(format!("/Command=RUN\"\"{run}"));
                },
                Emulator::RetroVm(_) => {
                    o.emit_stderr("auto_run is currently ignored for RetroVM\n");
                },
                Emulator::Cadence(_) => {
                    o.emit_stderr("auto_run is currently ignored for Cadence\n");
                },
                Emulator::Emulator1984(_) => {
                    args.push(format!("--autostart={run}"));
                }
            }
        }

        Ok(args)
    }

    /// Generate args for Amspirit emulator using CSL script
    fn args_for_emu_amspirit_with_csl(&self, emu: &Emulator) -> Result<Vec<String>, String> {
        // Generate CSL script from configuration
        let csl_script: cpclib_csl::CslScript = self.clone().into();
        let absolute_path = write_csl_script_to_temp_file(&csl_script)?;

        // Get wine-compatible absolute path if needed
        let csl_path = emu.wine_compatible_fname(&absolute_path)?;
        let mut args = native_csl_args(emu, &csl_path);

        // SugarBoxV2::screenshot() (below, on SugarBoxV2UsedEmulator) talks
        // to the same JSON-over-TCP debug server an external debugger
        // integration elsewhere in this workspace uses to fetch a real
        // screen capture - it needs a debug server actually running to
        // connect to. Only
        // matters for this Robot-driven synthesis path, not for a user's
        // own pre-existing `.csl` file run via `run_csl_file` (which
        // reuses `native_csl_args` directly, unmodified).
        #[cfg(feature = "screenshot")]
        if let Emulator::SugarBoxV2(_) = emu {
            args.push("--ds".to_owned());
            args.push(SUGARBOX_ROBOT_DEBUG_SERVER_PORT.to_string());
        }

        Ok(args)
    }
}

/// Serializes `script` and writes it to a fresh, kept (not
/// auto-deleted-on-drop) temp `.csl` file, returning its absolute path.
/// Shared by `EmulatorConf::args_for_emu_amspirit_with_csl` (a script
/// synthesized from scalar config fields) and `run_csl_file` (a user's own
/// `.csl` file, after `csl_interpreter::resolve_relative_paths` has
/// rewritten its paths to absolute) - both need "a real file on disk the
/// emulator can be pointed at", never the in-memory `CslScript` itself.
fn write_csl_script_to_temp_file(script: &cpclib_csl::CslScript) -> Result<Utf8PathBuf, String> {
    let csl_content = script.to_string();

    let tempfile = camino_tempfile::Builder::new()
        .suffix(".csl")
        .tempfile()
        .map_err(|e| format!("Failed to create temporary CSL file: {}", e))?;
    let temp_path_utf8 = tempfile.path().to_owned();

    fs_err::write(&temp_path_utf8, csl_content)
        .map_err(|e| format!("Failed to write CSL file: {}", e))?;

    // Keep the temporary file (prevent automatic deletion).
    let _kept = tempfile
        .into_temp_path()
        .keep()
        .map_err(|e| format!("Failed to keep temporary CSL file: {}", e))?;

    if temp_path_utf8.is_absolute() {
        Ok(temp_path_utf8)
    }
    else {
        let canonical = temp_path_utf8
            .canonicalize()
            .map_err(|e| format!("Failed to canonicalize CSL file path: {}", e))?;
        Utf8PathBuf::from_path_buf(canonical)
            .map_err(|p| format!("Invalid UTF-8 in canonical path: {:?}", p))
    }
}

/// Builds the launch args for `emu.accept_csl()`-shaped CSL support - the
/// flag shape is per-emulator, since their CLIs disagree on it (both
/// confirmed against each emulator's own documented flags): used both by
/// `EmulatorConf::args_for_emu_amspirit_with_csl` (a script synthesized
/// from an `EmulatorConf`) and `run_csl_file` (a user's own pre-existing
/// `.csl` file), so there is exactly one place that knows the shape.
fn native_csl_args(emu: &Emulator, csl_path: &Utf8Path) -> Vec<String> {
    match emu {
        // SugarboxV2: `-s, --csl <script>` - "Run a CSL script on start"
        // (Tom1975/SugarboxV2's own docs) - a space-separated value, not a
        // `--csl=` one.
        Emulator::SugarBoxV2(_) => vec!["--csl".to_string(), csl_path.to_string()],
        // AMSpiriT and everything else `accept_csl()` might cover in the
        // future default to the `--csl=<path>` shape already verified for
        // AMSpiriT.
        _ => vec![format!("--csl={csl_path}")]
    }
}

impl From<EmulatorConf> for cpclib_csl::CslScript {
    fn from(conf: EmulatorConf) -> Self {
        use cpclib_common::camino::Utf8PathBuf;
        use cpclib_csl::{CslInstruction, CslScriptBuilder, Drive, KeyOutput};

        let cwd = Utf8PathBuf::from_path_buf(std::env::current_dir().unwrap()).unwrap();
        // Start with builder with version 1.0
        let mut builder = CslScriptBuilder::new();

        // Set disk directory if any disk is configured
        if conf.drive_a.is_some() || conf.drive_b.is_some() {
            builder = builder
                .with_instruction(CslInstruction::disk_dir(cwd.clone()))
                .expect("There is a bug there");
        }

        // Insert disks if configured
        if let Some(drive_a) = conf.drive_a {
            builder = builder
                .with_instruction(CslInstruction::disk_insert(Drive::A, drive_a))
                .expect("There is a bug there");
        }

        if let Some(drive_b) = conf.drive_b {
            builder = builder
                .with_instruction(CslInstruction::disk_insert(Drive::B, drive_b))
                .expect("There is a bug there");
        }

        // Configure memory expansion if specified
        if let Some(memory) = conf.memory {
            builder = builder
                .with_instruction(CslInstruction::memory_exp(memory_to_csl_expansion(memory)))
                .expect("There is a bug there");
        }

        // Configure CRTC model if specified
        if let Some(crtc) = conf.crtc {
            builder = builder
                .with_instruction(CslInstruction::crtc_select(crtc.to_csl_model()))
                .expect("There is a bug there");
        }

        if conf.crtc.is_some() || conf.memory.is_some() {
            builder = builder
                .with_reset(ResetType::Hard)
                .expect("There is a bug there");
        }

        // Set snapshot directory and load if configured
        if let Some(snapshot) = conf.snapshot {
            builder = builder
                .with_instruction(CslInstruction::snapshot_dir(cwd))
                .expect("There is a bug there")
                .with_instruction(CslInstruction::snapshot_load(snapshot))
                .expect("There is a bug there");
        }

        if conf.auto_run.is_some() || conf.auto_type.is_some() {
            builder = builder
                .with_instruction(CslInstruction::wait(19968 * 50))
                .expect("There is a bug there");
            builder = builder
                .with_instruction(CslInstruction::key_delay(70000, Some(70000), Some(400000)))
                .expect("There is a bug there");
        }

        // Add auto-run command if configured
        if let Some(auto_run) = conf.auto_run {
            let key_string = format!("RUN\"{}\n", auto_run);
            if let Ok(key_output) = KeyOutput::try_from(key_string.as_str()) {
                builder = builder
                    .with_instruction(CslInstruction::key_output(key_output))
                    .expect("Ther is a bug there");
            }
        }

        // Add auto-type file if configured
        if let Some(auto_type) = conf.auto_type {
            if false {
                builder = builder
                    .with_instruction(CslInstruction::key_from_file(auto_type))
                    .expect("There is a bug there");
            }
            else {
                let content =
                    fs_err::read_to_string(&auto_type).expect("Failed to read auto-type file");
                let key_output = KeyOutput::try_from(content.as_str())
                    .expect("Failed to convert auto-type content to KeyOutput");
                builder = builder
                    .with_instruction(CslInstruction::key_output(key_output))
                    .expect("There is a bug there");
            }
        }

        // Build the final script with version first
        builder.build().expect("Failed to build CSL script")
    }
}

pub fn start_emulator<E: EventObserver>(
    emu: &Emulator,
    conf: &EmulatorConf,
    o: &E
) -> Result<(), String> {
    let args = conf.args_for_emu(emu, o)?;
    spawn_emulator_with_args(emu, conf.transparent, &args, o)
}

/// The actual process-spawn primitive `start_emulator` builds its args for -
/// factored out so a caller that already has its own args (e.g. a raw
/// `--csl=<path>` for a natively-CSL-capable emulator, bypassing
/// `EmulatorConf::args_for_emu`'s synthesis entirely) can reuse the same
/// install-check-free spawn without going through an `EmulatorConf` at all.
fn spawn_emulator_with_args<E: EventObserver>(
    emu: &Emulator,
    #[cfg_attr(not(feature = "transparent-x11"), allow(unused_variables))] transparent: bool,
    args: &[String],
    o: &E
) -> Result<(), String> {
    let app = emu.configuration();

    let cmd = emu.get_command().into();
    #[cfg(feature = "transparent-x11")]
    let runner = if transparent {
        DelegatedRunner::new_transparent(app, cmd)
    }
    else {
        DelegatedRunner::new(app, cmd)
    };
    #[cfg(not(feature = "transparent-x11"))]
    let runner = DelegatedRunner::new(app, cmd);

    runner.inner_run(args, o)
}

pub fn get_emulator_window(
    emu: &Emulator,
    _conf: &EmulatorConf,
    #[cfg_attr(
        not(any(feature = "transparent-x11", feature = "screenshot")),
        allow(unused_variables)
    )]
    o: &dyn EventObserver
) -> Option<EmuWindow> {
    #[cfg(feature = "transparent-x11")]
    if conf.transparent {
        return Some(get_emulator_window_xvfb(emu, o));
    }
    #[cfg(feature = "screenshot")]
    return get_emulator_window_xcap(emu, o);

    #[cfg(not(feature = "screenshot"))]
    None
}

// XX this code seems buggy ATM it is unable to collect the window, no idea why
#[cfg(feature = "transparent-x11")]
fn get_emulator_window_xvfb(emu: &Emulator, o: &dyn EventObserver) -> EmuWindow {
    // get the latest x server. Lets' hope it is the virtual one of the transparent emulator
    let display = fs_err::read_dir("/tmp/.X11-unix")
        .unwrap()
        .filter_map(|f| f.ok())
        .filter(|f| f.file_name().to_str().unwrap().starts_with("X"))
        .map(|f| (f.file_name(), f.metadata().unwrap().created().unwrap()))
        .sorted_by_key(|f| f.1)
        .rev()
        .take(1)
        .map(|(f, _d)| f.to_str().unwrap()[1..].to_owned())
        .next()
        .unwrap();
    let display_nb = display.parse::<usize>().unwrap();
    // XX this part seems to work

    // XX next seem to not work :(
    // change the display to get the window list of the framebuffer
    let _backup_display = std::env::var("DISPLAY").unwrap();

    unsafe { std::env::set_var("DISPLAY", format!(":{}", &display)) };
    let windows = wmctrl::get_windows();

    let mut windows = windows
        .into_iter()
        .filter(|win| emu.window_name_corresponds(win.title()))
        .collect_vec();

    let window = match windows.len() {
        0 => None,
        1 => windows.pop(),
        _ => {
            o.emit_stderr("There are several available windows. I pick one, but it may be wrong\n");
            windows.pop()
        }
    };

    EmuWindow::Xvfb(display_nb, window)
}

#[cfg(feature = "screenshot")]
fn get_emulator_window_xcap(emu: &Emulator, o: &dyn EventObserver) -> Option<EmuWindow> {
    const MAX_ATTEMPTS: usize = 30;
    const RETRY_DELAY_MS: u64 = 200;

    for _ in 0..MAX_ATTEMPTS {
        if let Some(window) = get_emulator_window_xcap_once(emu, o) {
            return Some(window);
        }

        std::thread::sleep(Duration::from_millis(RETRY_DELAY_MS));
    }

    o.emit_stderr(&format!(
        "No window emulator found after {} ms\n",
        MAX_ATTEMPTS as u64 * RETRY_DELAY_MS
    ));

    None
}

#[cfg(feature = "screenshot")]
fn get_emulator_window_xcap_once(emu: &Emulator, o: &dyn EventObserver) -> Option<EmuWindow> {
    let windows = xcap::Window::all().unwrap();
    let mut windows = windows
        .into_iter()
        .filter(|win| emu.window_name_corresponds(&win.title().unwrap()))
        .collect_vec();

    let window = match windows.len() {
        0 => return None,
        1 => windows.pop().unwrap(),
        _ => {
            o.emit_stderr("There are several available windows. I pick one, but it may be wrong\n");
            windows.pop().unwrap()
        }
    };

    Some(EmuWindow::Xcap(window))
}

pub(crate) trait UsedEmulator: Sized {
    /// the default behavior consists in capturing the full window emulator.
    /// This can of course be tailored to get the emulated screen area
    #[cfg(feature = "screenshot")]
    fn screenshot(robot: &mut RobotImpl<Self>) -> EmuScreenShot
    where Self: Sized {
        robot.window.as_ref().map(|w| w.capture_image()).unwrap_or_else(|| {
            panic!("Emulator screenshot is not available for this emulator. This is a bug, please report it")
        })
    }

    /// The default behavior simulates host-level keystrokes (Enigo) into
    /// whichever window this Robot is driving. Tailored where an emulator
    /// has its own, more direct way to inject text.
    fn type_text(robot: &mut RobotImpl<Self>, s: &str)
    where Self: Sized {
        robot.events_manager.type_text(s);
    }

    /// The default behavior is "not available": most emulators expose no
    /// way at all to peek at memory from outside a real debug session.
    /// Tailored where an emulator has its own API for it.
    #[cfg(feature = "screenshot")]
    fn read_memory(_robot: &mut RobotImpl<Self>, _address: u16, _count: u16) -> Result<Vec<u8>, String>
    where Self: Sized {
        Err("This emulator has no memory-read facility reachable from Robot automation".to_owned())
    }

    /// See `read_memory`'s own doc comment - same reasoning, opposite direction.
    #[cfg(feature = "screenshot")]
    fn write_memory(_robot: &mut RobotImpl<Self>, _address: u16, _data: &[u8]) -> Result<(), String>
    where Self: Sized {
        Err("This emulator has no memory-write facility reachable from Robot automation".to_owned())
    }

    /// The default behavior is "not available" - most emulators expose no
    /// way to load a snapshot file into an already-running instance from
    /// outside a real debug session. Tailored where an emulator has its
    /// own API for it.
    #[cfg(feature = "screenshot")]
    fn load_snapshot(_robot: &mut RobotImpl<Self>, _path: &Utf8Path) -> Result<(), String>
    where Self: Sized {
        Err("This emulator has no snapshot-load facility reachable from Robot automation".to_owned())
    }

    /// See `load_snapshot`'s own doc comment - same reasoning, for discs.
    #[cfg(feature = "screenshot")]
    fn load_disc(_robot: &mut RobotImpl<Self>, _drive: u8, _path: &Utf8Path) -> Result<(), String>
    where Self: Sized {
        Err("This emulator has no disc-load facility reachable from Robot automation".to_owned())
    }

    /// See `load_snapshot`'s own doc comment - same reasoning, exporting
    /// the disc currently in a drive rather than inserting one. Returns
    /// the raw `.dsk` bytes; writing them to a file is the caller's job
    /// (mirrors `load_disc`/`load_snapshot` taking a path rather than
    /// bytes only in the direction each emulator's own API actually wants
    /// them).
    #[cfg(feature = "screenshot")]
    fn save_disc(_robot: &mut RobotImpl<Self>, _drive: u8) -> Result<Vec<u8>, String>
    where Self: Sized {
        Err("This emulator has no disc-save facility reachable from Robot automation".to_owned())
    }
}

pub(crate) struct AceUsedEmulator {}
pub(crate) struct CpcecUsedEmulator {}
pub(crate) struct WinapeUsedEmulator {}
pub(crate) struct AmspiritUsedEmulator {}
pub(crate) struct SugarBoxV2UsedEmulator {}
pub(crate) struct CpcEmuPowerUsedEmulator {}
pub(crate) struct CpcEmuUsedEmulator {}
pub(crate) struct RetroVmUsedEmulator {}

pub(crate) struct CapriceForeverUsedEmulator {}
pub(crate) struct CadenceUsedEmulator {}
pub(crate) struct Emulator1984UsedEmulator {}

impl UsedEmulator for AceUsedEmulator {
    // here we delegate the creation of screenshot to Ace to avoid some issues i do not understand
    #[cfg(feature = "screenshot")]
    fn screenshot(robot: &mut RobotImpl<Self>) -> Screenshot {
        let folder = robot.emu.screenshots_folder();
        let glob_pattern = folder.join("*.png");
        // A malformed glob pattern would be a structural bug (the pattern is
        // just a fixed "*.png" suffix), not a transient race - panicking
        // with a clear message is appropriate here, unlike the per-entry
        // races handled by `filter_map` below.
        let list_screenshots = || {
            glob::glob(glob_pattern.as_str())
                .unwrap_or_else(|e| panic!("Invalid glob pattern `{glob_pattern}`: {e}"))
                // Each entry can transiently fail to be read if the emulator
                // is mid-write/rename to the same directory; skip those
                // instead of crashing the whole capture over a race.
                .filter_map(|p| p.ok())
                .map(|p| p.as_path().to_owned())
                .collect::<HashSet<_>>()
        };
        let before_screenshots = list_screenshots();

        // handlekey press
        robot.type_key(HostKey::F10);

        let mut file = None;
        while file.is_none() {
            WindowEventsManager::wait_a_bit();
            let after_screenshots = list_screenshots();
            let mut new_screenshots = after_screenshots
                .difference(&before_screenshots)
                .cloned()
                .collect_vec();
            if !new_screenshots.is_empty() {
                file = Some(new_screenshots.pop().unwrap());
            }
        }

        let file = file.as_ref().unwrap();
        let mut im = xcap::image::open(file);
        while im.is_err() {
            WindowEventsManager::wait_a_bit();
            im = xcap::image::open(file);
        }
        let im = im.unwrap().into_rgba8();
        // Best-effort cleanup: the image is already decoded into `im` at
        // this point, so a locked/already-gone file here (e.g. the emulator
        // or an antivirus still holding it) shouldn't crash the capture -
        // it just leaves a harmless leftover screenshot file behind.
        let _ = fs_err::remove_file(file);
        im
    }
}

/// The port `args_for_emu` explicitly passes to a Robot-launched AMSpiriT
/// Lite via `--web-port` (its own documented out-of-the-box default,
/// deliberately not left to whatever `amspirit-lite.conf` the current user
/// happens to have saved) - the functions below need to know this port
/// with certainty, not guess at the user's own local configuration.
#[cfg(feature = "screenshot")]
const AMSPIRIT_LITE_ROBOT_WEB_PORT: u16 = 8765;

/// Retries `f` against AMSpiriT Lite/SugarBoxV2's own API for up to five
/// seconds - both web/debug servers bind very early in their boot
/// sequence, but a Robot method can in principle be called right after the
/// process is spawned, before either has had a chance to start listening.
/// Panics past the deadline: every caller here already has no better
/// fallback (that's exactly why it reached this native-API path rather
/// than the trait's own default behavior).
#[cfg(feature = "screenshot")]
fn retry_native_api_call<T>(what: &str, mut f: impl FnMut() -> Result<T, String>) -> T {
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        match f() {
            Ok(v) => return v,
            Err(e) if std::time::Instant::now() < deadline => {
                WindowEventsManager::wait_a_bit();
                let _ = e;
            },
            Err(e) => panic!("{what} failed: {e}. This is a bug, please report it")
        }
    }
}

impl UsedEmulator for AmspiritUsedEmulator {
    // Shared with full (non-Lite) AMSpiriT (see `Robot::new`'s comment on
    // why), which has no HTTP API of its own - only AmspiritLite gets each
    // native path below, the rest fall through to the trait's own default
    // behavior (window capture / Enigo keystrokes / "not available").
    #[cfg(feature = "screenshot")]
    fn screenshot(robot: &mut RobotImpl<Self>) -> Screenshot {
        if matches!(robot.emu, Emulator::AmspiritLite(_)) {
            return retry_native_api_call("AMSpiriT Lite screenshot via its own HTTP API", || {
                let png = amspiritlite_api::get_screenshot_png(amspiritlite_endpoint())?;
                xcap::image::load_from_memory(&png)
                    .map(|img| img.into_rgba8())
                    .map_err(|e| format!("Failed to decode AMSpiriT Lite screenshot PNG: {e}"))
            });
        }

        robot.window.as_ref().map(|w| w.capture_image()).unwrap_or_else(|| {
            panic!("Emulator screenshot is not available for this emulator. This is a bug, please report it")
        })
    }

    #[cfg(feature = "screenshot")]
    fn type_text(robot: &mut RobotImpl<Self>, s: &str) {
        if matches!(robot.emu, Emulator::AmspiritLite(_)) {
            return retry_native_api_call("AMSpiriT Lite keytype via its own HTTP API", || {
                amspiritlite_api::keytype(amspiritlite_endpoint(), s)
            });
        }

        robot.events_manager.type_text(s);
    }

    #[cfg(feature = "screenshot")]
    fn read_memory(robot: &mut RobotImpl<Self>, address: u16, count: u16) -> Result<Vec<u8>, String> {
        if matches!(robot.emu, Emulator::AmspiritLite(_)) {
            return Ok(retry_native_api_call(
                "AMSpiriT Lite readMemory via its own HTTP API",
                || amspiritlite_api::read_memory(amspiritlite_endpoint(), address, count)
            ));
        }
        Err("Full AMSpiriT (non-Lite) has no memory-read API reachable from Robot automation"
            .to_owned())
    }

    #[cfg(feature = "screenshot")]
    fn write_memory(robot: &mut RobotImpl<Self>, address: u16, data: &[u8]) -> Result<(), String> {
        if matches!(robot.emu, Emulator::AmspiritLite(_)) {
            retry_native_api_call("AMSpiriT Lite writeMemory via its own HTTP API", || {
                amspiritlite_api::write_memory(amspiritlite_endpoint(), address, data)
            });
            return Ok(());
        }
        Err("Full AMSpiriT (non-Lite) has no memory-write API reachable from Robot automation"
            .to_owned())
    }

    // Deliberately no retry loop, same reasoning as SugarBoxV2UsedEmulator's
    // own load_snapshot/load_disc below: a bad path or a file the emulator
    // rejects is a real, non-transient failure the caller needs to see,
    // not something to paper over by retrying for 5 seconds and then
    // panicking.
    #[cfg(feature = "screenshot")]
    fn load_snapshot(robot: &mut RobotImpl<Self>, path: &Utf8Path) -> Result<(), String> {
        if matches!(robot.emu, Emulator::AmspiritLite(_)) {
            return amspiritlite_api::load_snapshot(amspiritlite_endpoint(), path);
        }
        Err("Full AMSpiriT (non-Lite) has no snapshot-load API reachable from Robot automation"
            .to_owned())
    }

    #[cfg(feature = "screenshot")]
    fn load_disc(robot: &mut RobotImpl<Self>, drive: u8, path: &Utf8Path) -> Result<(), String> {
        if matches!(robot.emu, Emulator::AmspiritLite(_)) {
            return amspiritlite_api::load_disc(amspiritlite_endpoint(), drive, path);
        }
        Err("Full AMSpiriT (non-Lite) has no disc-load API reachable from Robot automation"
            .to_owned())
    }

    // Deliberately no retry loop, unlike this impl's other native-API
    // calls: `amspiritlite_api::save_disc`'s own doc comment explains that
    // every attempt against a live 1.14.3 build answered
    // `{"error":"no disk or save failed"}` regardless of a real disk being
    // present - a functional error, not a "server not listening yet"
    // startup race `retry_native_api_call`'s 5-second-then-panic behavior
    // is meant for, so it would just retry a real failure for 5 seconds
    // and then panic on it instead of returning the emulator's own error
    // text to the caller.
    #[cfg(feature = "screenshot")]
    fn save_disc(robot: &mut RobotImpl<Self>, drive: u8) -> Result<Vec<u8>, String> {
        if matches!(robot.emu, Emulator::AmspiritLite(_)) {
            return amspiritlite_api::save_disc(amspiritlite_endpoint(), drive);
        }
        Err("Full AMSpiriT (non-Lite) has no disc-save API reachable from Robot automation"
            .to_owned())
    }
}

/// `AMSPIRIT_LITE_ROBOT_WEB_PORT` as the `http://host:port` shape
/// `amspiritlite_api`'s functions take.
#[cfg(feature = "screenshot")]
fn amspiritlite_endpoint() -> &'static str {
    // `AMSPIRIT_LITE_ROBOT_WEB_PORT` is fixed, so this is fixed too -
    // leaked once rather than formatted on every single call.
    static ENDPOINT: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    ENDPOINT.get_or_init(|| format!("http://127.0.0.1:{AMSPIRIT_LITE_ROBOT_WEB_PORT}"))
}

/// The port `args_for_emu_amspirit_with_csl` explicitly passes to a
/// Robot-launched SugarBoxV2 via `--ds` - the functions below need a
/// fixed, known port to connect their own debug-server client to, same
/// reasoning as `AMSPIRIT_LITE_ROBOT_WEB_PORT` above. A different number
/// from that one: nothing stops a screenshot-capable Robot session and a
/// live external debug session from existing on the same machine at once,
/// and each needs its own port.
#[cfg(feature = "screenshot")]
const SUGARBOX_ROBOT_DEBUG_SERVER_PORT: u16 = 8766;

impl UsedEmulator for SugarBoxV2UsedEmulator {
    #[cfg(feature = "screenshot")]
    fn screenshot(_robot: &mut RobotImpl<Self>) -> Screenshot {
        retry_native_api_call("SugarBoxV2 screenshot via its own debug-server API", || {
            let png = sugarbox_api::get_screenshot_png(SUGARBOX_ROBOT_DEBUG_SERVER_PORT)?;
            xcap::image::load_from_memory(&png)
                .map(|img| img.into_rgba8())
                .map_err(|e| format!("Failed to decode SugarBoxV2 screenshot PNG: {e}"))
        })
    }

    #[cfg(feature = "screenshot")]
    fn read_memory(_robot: &mut RobotImpl<Self>, address: u16, count: u16) -> Result<Vec<u8>, String> {
        Ok(retry_native_api_call(
            "SugarBoxV2 readMemory via its own debug-server API",
            || sugarbox_api::read_memory(SUGARBOX_ROBOT_DEBUG_SERVER_PORT, address, count)
        ))
    }

    #[cfg(feature = "screenshot")]
    fn write_memory(_robot: &mut RobotImpl<Self>, address: u16, data: &[u8]) -> Result<(), String> {
        retry_native_api_call("SugarBoxV2 writeMemory via its own debug-server API", || {
            sugarbox_api::write_memory(SUGARBOX_ROBOT_DEBUG_SERVER_PORT, address, data)
        });
        Ok(())
    }

    // Deliberately no retry loop here, unlike this impl's other native-API
    // calls: an invalid path or a file the emulator rejects (confirmed
    // live: a non-.sna file answers `{"status":"error","message":"snapshot
    // load failed: .."}`) is a real, meaningful, non-transient failure a
    // caller needs to see - `retry_native_api_call`'s 5-second-then-panic
    // behavior is for the narrow "server not listening yet" startup race,
    // not for "the emulator looked at this file and said no".
    #[cfg(feature = "screenshot")]
    fn load_snapshot(_robot: &mut RobotImpl<Self>, path: &Utf8Path) -> Result<(), String> {
        sugarbox_api::load_snapshot(SUGARBOX_ROBOT_DEBUG_SERVER_PORT, path)
    }

    #[cfg(feature = "screenshot")]
    fn load_disc(_robot: &mut RobotImpl<Self>, drive: u8, path: &Utf8Path) -> Result<(), String> {
        sugarbox_api::load_disc(SUGARBOX_ROBOT_DEBUG_SERVER_PORT, drive, path)
    }
}

pub(crate) struct RobotImpl<E: UsedEmulator> {
    pub(crate) window: Option<EmuWindow>,
    pub(crate) events_manager: WindowEventsManager,
    pub(crate) emu: Emulator,
    _emu: PhantomData<E>
}

impl<E: UsedEmulator> Deref for RobotImpl<E> {
    type Target = WindowEventsManager;

    fn deref(&self) -> &Self::Target {
        &self.events_manager
    }
}

impl<E: UsedEmulator> DerefMut for RobotImpl<E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.events_manager
    }
}

pub(crate) enum Robot {
    Ace(RobotImpl<AceUsedEmulator>),
    Cpcec(RobotImpl<CpcecUsedEmulator>),
    Winape(RobotImpl<WinapeUsedEmulator>),
    Amspirit(RobotImpl<AmspiritUsedEmulator>),
    SugarboxV2(RobotImpl<SugarBoxV2UsedEmulator>),
    CpcEmuPower(RobotImpl<CpcEmuPowerUsedEmulator>),
    CpcEmu(RobotImpl<CpcEmuUsedEmulator>),
    RetroVm(RobotImpl<RetroVmUsedEmulator>),
    CapriceForever(RobotImpl<CapriceForeverUsedEmulator>),
    Cadence(RobotImpl<CadenceUsedEmulator>),
    Emulator1984(RobotImpl<Emulator1984UsedEmulator>)
}

/// Generates the `From<RobotImpl<_>> for Robot` boilerplate shared by every
/// emulator variant. Optionally also generates the trivial `UsedEmulator`
/// marker impl (default screenshot behavior) for emulators that need no
/// per-emulator override - `AceUsedEmulator`, `AmspiritUsedEmulator` and
/// `SugarBoxV2UsedEmulator` are the exceptions (each has a custom
/// `screenshot` impl above) and keep their own hand-written `UsedEmulator`
/// impl, so all three are listed with `robot_from_only`.
macro_rules! used_emulators {
    (robot_from_only: $($used:ident => $variant:ident),+ $(,)?) => {
        $(
            impl From<RobotImpl<$used>> for Robot {
                fn from(value: RobotImpl<$used>) -> Self {
                    Self::$variant(value)
                }
            }
        )+
    };
    ($($used:ident => $variant:ident),+ $(,)?) => {
        $(
            impl UsedEmulator for $used {}
        )+
        used_emulators!(robot_from_only: $($used => $variant),+);
    };
}

used_emulators!(
    robot_from_only:
    AceUsedEmulator => Ace,
    AmspiritUsedEmulator => Amspirit,
    SugarBoxV2UsedEmulator => SugarboxV2,
);
used_emulators! {
    CpcecUsedEmulator => Cpcec,
    WinapeUsedEmulator => Winape,
    CpcEmuPowerUsedEmulator => CpcEmuPower,
    CpcEmuUsedEmulator => CpcEmu,
    RetroVmUsedEmulator => RetroVm,
    CapriceForeverUsedEmulator => CapriceForever,
    CadenceUsedEmulator => Cadence,
    Emulator1984UsedEmulator => Emulator1984,
}

impl<E: UsedEmulator> From<(Option<EmuWindow>, WindowEventsManager, &Emulator)> for RobotImpl<E> {
    fn from(value: (Option<EmuWindow>, WindowEventsManager, &Emulator)) -> Self {
        Self {
            window: value.0,
            events_manager: value.1,
            emu: value.2.clone(),
            _emu: PhantomData::<E>
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum OrgamsRobotAction<'a, 'b> {
    LoadOrImportAndEdit { src: &'a str },
    LoadOrImportAndSave { src: &'a str, tgt: &'b str },
    LoadOrImportAndAssembleJump { src: &'a str },
    LoadOrImportAndAssembleAndSave { src: &'a str, tgt: Option<&'b str> }
}

impl<'a, 'b> OrgamsRobotAction<'a, 'b> {
    pub fn new_edit(src: &'a str) -> Self {
        Self::LoadOrImportAndEdit { src }
    }

    pub fn new_jump(src: &'a str) -> Self {
        Self::LoadOrImportAndAssembleJump { src }
    }

    pub fn new_save_sources(src: &'a str, tgt: &'b str) -> Result<Self, String> {
        if !(tgt.ends_with(".o") || tgt.ends_with(".O")) {
            Err(format!("{tgt} is not a binary orgams file format"))
        }
        else {
            Ok(Self::LoadOrImportAndSave { src, tgt })
        }
    }

    pub fn new_export_sources(src: &'a str, tgt: &'b str) -> Result<Self, String> {
        if !(src.ends_with(".o") || src.ends_with(".O")) {
            Err(format!("{src} is not a binary orgams file format"))
        }
        else {
            Ok(Self::LoadOrImportAndSave { src, tgt })
        }
    }

    pub fn new_save_binary(src: &'a str, tgt: Option<&'b str>) -> Self {
        Self::LoadOrImportAndAssembleAndSave { src, tgt }
    }
}

impl OrgamsRobotAction<'_, '_> {
    pub fn src(&self) -> &str {
        match self {
            OrgamsRobotAction::LoadOrImportAndEdit { src, .. }
            | OrgamsRobotAction::LoadOrImportAndSave { src, .. }
            | OrgamsRobotAction::LoadOrImportAndAssembleJump { src, .. }
            | OrgamsRobotAction::LoadOrImportAndAssembleAndSave { src, .. } => src
        }
    }

    pub fn dst(&self) -> Option<&str> {
        match self {
            OrgamsRobotAction::LoadOrImportAndSave { tgt, .. } => Some(tgt),
            OrgamsRobotAction::LoadOrImportAndAssembleAndSave { tgt, .. } => *tgt,
            _ => None
        }
    }

    pub fn edit(&self) -> bool {
        matches!(self, OrgamsRobotAction::LoadOrImportAndEdit { .. })
    }

    pub fn jump(&self) -> bool {
        matches!(self, OrgamsRobotAction::LoadOrImportAndAssembleJump { .. })
    }

    pub fn save_orgams_binary_source(&self) -> Option<&str> {
        match self {
            OrgamsRobotAction::LoadOrImportAndSave { tgt, .. }
                if tgt.ends_with(".o") || tgt.ends_with(".O") =>
            {
                Some(tgt)
            },
            _ => None
        }
    }

    pub fn save_orgams_ascii_source(&self) -> Option<&str> {
        match self {
            OrgamsRobotAction::LoadOrImportAndSave { tgt, .. }
                if !(tgt.ends_with(".o") || tgt.ends_with(".O")) =>
            {
                Some(tgt)
            },
            _ => None
        }
    }

    pub fn save_orgams_binary(&self) -> Option<Option<&str>> {
        match self {
            OrgamsRobotAction::LoadOrImportAndAssembleAndSave { tgt, .. } => Some(*tgt),
            _ => None
        }
    }

    pub fn request_assembling(&self) -> bool {
        matches!(
            self,
            OrgamsRobotAction::LoadOrImportAndAssembleAndSave { .. }
                | OrgamsRobotAction::LoadOrImportAndAssembleJump { .. }
        )
    }
}

impl Robot {
    delegate::delegate! {
        to match self {
            Robot::Ace(r) => r,
            Robot::Cpcec(r) => r,
            Robot::Winape(r) => r,
            Robot::Amspirit(r) => r,
            Robot::SugarboxV2(r) => r,
            Robot::CpcEmuPower(r) => r,
            Robot::CpcEmu(r) => r,
            Robot::RetroVm(r) => r,
            Robot::CapriceForever(r) => r,
            Robot::Cadence(r) => r,
            Robot::Emulator1984(r) => r,
        } {
            #[cfg(feature = "screenshot")]
            fn handle_orgams(
                &mut self,
                drivea: Option<&str>,
                albireo: Option<&str>,
                action: OrgamsRobotAction<'_, '_>,
                o: &dyn EventObserver
            ) -> Result<(), String>;
            fn type_text(&mut self, s: &str);
            #[cfg(feature = "screenshot")]
            fn read_memory(&mut self, address: u16, count: u16) -> Result<Vec<u8>, String>;
            #[cfg(feature = "screenshot")]
            fn write_memory(&mut self, address: u16, data: &[u8]) -> Result<(), String>;
            #[cfg(feature = "screenshot")]
            fn load_snapshot(&mut self, path: &Utf8Path) -> Result<(), String>;
            #[cfg(feature = "screenshot")]
            fn load_disc(&mut self, drive: u8, path: &Utf8Path) -> Result<(), String>;
            #[cfg(feature = "screenshot")]
            fn save_disc(&mut self, drive: u8) -> Result<Vec<u8>, String>;
            fn close(&mut self);
        }

    }

    pub fn new(
        emu: &Emulator,
        window: Option<EmuWindow>,
        events_manager: WindowEventsManager
    ) -> Self {
        match emu {
            Emulator::Ace(_) => {
                RobotImpl::<AceUsedEmulator>::from((window, events_manager, emu)).into()
            },
            Emulator::CpcEmu(_) => {
                RobotImpl::<CpcEmuUsedEmulator>::from((window, events_manager, emu)).into()
            },
            Emulator::Cpcec(_) => {
                RobotImpl::<CpcecUsedEmulator>::from((window, events_manager, emu)).into()
            },
            Emulator::Winape(_) => {
                RobotImpl::<WinapeUsedEmulator>::from((window, events_manager, emu)).into()
            },
            Emulator::Amspirit(_) => {
                RobotImpl::<AmspiritUsedEmulator>::from((window, events_manager, emu)).into()
            },
            // Driven by the same window automation as its bigger sibling: the
            // Lite build presents the same CPC screen and keyboard.
            Emulator::AmspiritLite(_) => {
                RobotImpl::<AmspiritUsedEmulator>::from((window, events_manager, emu)).into()
            },
            Emulator::SugarBoxV2(_) => {
                RobotImpl::<SugarBoxV2UsedEmulator>::from((window, events_manager, emu)).into()
            },
            Emulator::CpcEmuPower(_) => {
                RobotImpl::<CpcEmuPowerUsedEmulator>::from((window, events_manager, emu)).into()
            },
            Emulator::RetroVm(_) => {
                RobotImpl::<RetroVmUsedEmulator>::from((window, events_manager, emu)).into()
            },
            Emulator::CapriceForever(_caprice_forever_version) => {
                RobotImpl::<CapriceForeverUsedEmulator>::from((window, events_manager, emu)).into()
            },
            Emulator::Cadence(_) => {
                RobotImpl::<CadenceUsedEmulator>::from((window, events_manager, emu)).into()
            },
            Emulator::Emulator1984(_) => {
                RobotImpl::<Emulator1984UsedEmulator>::from((window, events_manager, emu)).into()
            },
        }
    }

    pub fn handle_raw_text<S: AsRef<str>>(&mut self, text: S) {
        let text = text.as_ref();
        let text = text.replace(r"\n", "\n");
        let text = &text;

        self.type_text(text);
    }
}

impl<E: UsedEmulator> RobotImpl<E> {
    #[cfg(feature = "screenshot")]
    pub fn screenshot(&mut self) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
        E::screenshot(self)
    }

    /// Inherent method, so it's picked over the `Deref`-to-`WindowEventsManager`
    /// one of the same name - lets `E::type_text` override the default
    /// Enigo-based behavior per emulator, same shape as `screenshot` above.
    pub fn type_text(&mut self, s: &str) {
        E::type_text(self, s);
    }

    #[cfg(feature = "screenshot")]
    pub fn read_memory(&mut self, address: u16, count: u16) -> Result<Vec<u8>, String> {
        E::read_memory(self, address, count)
    }

    #[cfg(feature = "screenshot")]
    pub fn write_memory(&mut self, address: u16, data: &[u8]) -> Result<(), String> {
        E::write_memory(self, address, data)
    }

    #[cfg(feature = "screenshot")]
    pub fn load_snapshot(&mut self, path: &Utf8Path) -> Result<(), String> {
        E::load_snapshot(self, path)
    }

    #[cfg(feature = "screenshot")]
    pub fn load_disc(&mut self, drive: u8, path: &Utf8Path) -> Result<(), String> {
        E::load_disc(self, drive, path)
    }

    #[cfg(feature = "screenshot")]
    pub fn save_disc(&mut self, drive: u8) -> Result<Vec<u8>, String> {
        E::save_disc(self, drive)
    }
}

impl<E: UsedEmulator> RobotImpl<E> {
    pub fn close(&mut self) {
        self.events_manager.alt_key(HostKey::F4);
    }
}

impl<E: UsedEmulator> RobotImpl<E> {
    pub fn unidos_select_drive(&mut self, drivea: Option<&str>, albireo: Option<&str>) {
        if drivea.is_some() {
            self.events_manager.type_text("load\"dfa:");
            self.events_manager.r#return();
        }
        else if albireo.is_some() {
            self.events_manager.type_text("load\"sd:");
            self.events_manager.r#return();
        }
        else {
            panic!("No storage selected");
        }
    }
}

impl<E: UsedEmulator> RobotImpl<E> {
    #[cfg(feature = "screenshot")]
    pub fn handle_orgams(
        &mut self,
        drivea: Option<&str>,
        albireo: Option<&str>,
        action: OrgamsRobotAction<'_, '_>,
        o: &dyn EventObserver
    ) -> Result<(), String> {
        // we assume that we do not need to select the window as well launched it. it is already selected

        self.unidos_select_drive(drivea, albireo);
        let mut res;

        // Handle file loading
        let src = action.src();
        res = if src.ends_with('o') || src.ends_with('O') {
            // here we directly load an orgams file
            self.orgams_load(src, o)
        }
        else {
            // here we need to import
            self.orgams_import(src, o)
        }
        .map_err(|screen| (format!("Error while loading {src}"), screen));

        // if file has been loaded, handle next action
        if res.is_ok() {
            // need to update res
            res = {
                if action.edit() {
                    // No need to do more when we want to edit a file
                    Ok(())
                }
                else if let Some(dst) = action.save_orgams_binary_source() {
                    self.orgams_save_source(dst, o)
                        .map_err(|screen| ("Error while saving sources".to_string(), screen))
                }
                else if let Some(dst) = action.save_orgams_ascii_source() {
                    self.orgams_export_source(dst, o).map_err(|screen| {
                        (format!("Error while exporting source to {dst}"), screen)
                    })
                }
                else {
                    // we want to assemble the file
                    self.orgams_assemble(src, o)
                        .map_err(|screen| ("Error while assembling".to_string(), screen))
                        .and_then(|_| {
                            if action.jump() {
                                self.orgams_jump()
                                    .map_err(|screen| ("Error while jumping".to_string(), screen))
                            }
                            else {
                                self.orgams_save(action.dst(), o).map_err(|screen| {
                                    ("Error while saving binary".to_string(), screen)
                                })
                            }
                        })
                }
            }
        };

        res.map_err(|(msg, screen)| {
            let path = {
                let tempfile = camino_tempfile::Builder::new()
                    .prefix("bnd_stuff")
                    .suffix(".png")
                    .tempfile()
                    .unwrap();
                let (_f, path) = tempfile.keep().unwrap();
                path
            };
            screen.save(&path).unwrap();
            open(&path).unwrap();
            format!("An error occurred.\n{msg}\nLook at {path}.")
        })
    }

    #[cfg(feature = "screenshot")]
    fn orgams_jump(&mut self) -> Result<(), Screenshot> {
        self.type_char('j');
        Ok(())
    }

    #[cfg(feature = "screenshot")]
    fn orgams_wait_import(&mut self, o: &dyn EventObserver) -> Result<(), Screenshot> {
        self.orgams_wait_save(o)
    }

    #[cfg(feature = "screenshot")]
    fn orgams_wait_save(&mut self, o: &dyn EventObserver) -> Result<(), Screenshot> {
        loop {
            let screen = self.screenshot();
            let coord_of_interest = (0, 48);
            let pix_of_interest = screen.get_pixel(coord_of_interest.0, coord_of_interest.1);

            if !(pix_of_interest == &Rgba([1, 1, 1, 255])
                || pix_of_interest == &Rgba([1, 2, 1, 255]))
            {
                o.emit_stdout("  done.\n");
                return Ok(()); // success
            }

            let coord_of_interest = (56, 508); // XXX Plus (63, 508)
            let pix_of_interest = screen.get_pixel(coord_of_interest.0, coord_of_interest.1);
            if pix_of_interest == &Rgba([247, 247, 247, 255])
                || pix_of_interest == &Rgba([255, 243, 249, 255])
            {
                return Err(screen);
            }
        }
    }

    #[cfg(feature = "screenshot")]
    fn orgams_save_source(&mut self, dst: &str, o: &dyn EventObserver) -> Result<(), Screenshot> {
        self.ctrl_char('s');
        self.type_text(dst);
        self.r#return();

        std::thread::sleep(Duration::from_millis(3000 / 2)); // we consider it takes at minimum to assemble a file
        self.orgams_wait_save(o)
    }

    #[cfg(feature = "screenshot")]
    fn orgams_export_source(&mut self, dst: &str, o: &dyn EventObserver) -> Result<(), Screenshot> {
        self.ctrl_char('e');
        self.type_text(dst);
        self.r#return();

        std::thread::sleep(Duration::from_millis(1000 / 2));
        self.type_char('W');

        std::thread::sleep(Duration::from_millis(3000 / 2)); // we consider it takes at minimum to assemble a file
        self.orgams_wait_save(o)
    }

    #[cfg(feature = "screenshot")]
    fn orgams_save(&mut self, dst: Option<&str>, o: &dyn EventObserver) -> Result<(), Screenshot> {
        o.emit_stdout("> Save result\n");
        // handle saving
        self.type_char('b');
        std::thread::sleep(Duration::from_millis(2000));
        if let Some(dst) = dst {
            self.type_text(dst);
            self.r#return();
        }
        else {
            self.r#return();
            std::thread::sleep(Duration::from_millis(1000));
            self.r#return();
        }
        o.emit_stdout("  Filename provided.\n");

        // wait save is done
        std::thread::sleep(Duration::from_millis(3000 / 2)); // we consider it takes at minimum to assemble a file
        self.orgams_wait_save(o)
    }

    #[cfg(feature = "screenshot")]
    fn orgams_import(
        &mut self,
        src: &str,
        o: &dyn EventObserver
    ) -> Result<(), ImageBuffer<Rgba<u8>, Vec<u8>>> {
        self.type_text("ùo");
        self.r#return();

        std::thread::sleep(Duration::from_secs(1)); // we wait one second for orgams loading

        self.ctrl_char('i');
        std::thread::sleep(Duration::from_secs(1)); // we wait one second for orgams loading

        self.type_text(src);
        self.r#return();

        self.orgams_wait_import(o)
    }

    #[cfg(feature = "screenshot")]
    fn orgams_load(
        &mut self,
        src: &str,
        o: &dyn EventObserver
    ) -> Result<(), ImageBuffer<Rgba<u8>, Vec<u8>>> {
        // Open orgams
        o.emit_stdout(&format!("> Launch orgams and open file \"{src}\"\n"));

        // French setup ?
        let chars = "ùo,\"".to_owned() + src + "\"";
        self.type_text(chars.as_str());
        self.r#return();

        let res = self.wait_orgams_loading();
        o.emit_stdout("  done.\n");

        res
    }

    #[cfg(feature = "screenshot")]
    fn orgams_assemble(
        &mut self,
        src: &str,
        o: &dyn EventObserver
    ) -> Result<(), ImageBuffer<Rgba<u8>, Vec<u8>>> {
        o.emit_stdout(&format!("> Assemble {src}\n"));
        self.ctrl_char('1');

        self.wait_orgams_assembling();
        o.emit_stdout("  done.\n");

        let result: ImageBuffer<Rgba<u8>, Vec<u8>> = self.window.as_ref().unwrap().capture_image();

        if result
            .pixels()
            .any(|p| p == &Rgba([99, 247, 99, 255]) || p == &Rgba([113, 243, 107, 255]))
        {
            Ok(())
        }
        else {
            Err(result)
        }
    }

    #[cfg(feature = "screenshot")]
    fn wait_orgams_assembling(&mut self) {
        let coord_of_interest = (0, 200);
        let mut finished = false;
        while !finished {
            std::thread::sleep(Duration::from_millis(1000 / 10));
            let screen = E::screenshot(self);
            let pix_of_interest = screen.get_pixel(coord_of_interest.0, coord_of_interest.1);

            finished = !(pix_of_interest == &Rgba([1, 1, 1, 255])
                || pix_of_interest == &Rgba([1, 2, 1, 255]));
        }
    }

    #[cfg(feature = "screenshot")]
    fn wait_orgams_loading(&mut self) -> Result<(), Screenshot> {
        #[derive(PartialEq)]
        enum State {
            Basic,
            Loading,
            Loaded
        }

        // we check a specific pixel that goes from blue to black then purple
        let mut state = State::Basic;
        while state != State::Loaded {
            let screen = E::screenshot(self);
            let coord_of_interest = (8, 48); // 2 to work on Amstrad plus and old
            let pix_of_interest = screen.get_pixel(coord_of_interest.0, coord_of_interest.1);

            state = match state {
                State::Basic => {
                    if pix_of_interest == &Rgba([1, 1, 99, 255])
                        || pix_of_interest == &Rgba([1, 2, 107, 255])
                    {
                        State::Basic
                    }
                    else {
                        State::Loading
                    }
                },
                State::Loading => {
                    if pix_of_interest == &Rgba([1, 1, 1, 255])
                        || pix_of_interest == &Rgba([1, 2, 1, 255])
                    {
                        State::Loading
                    }
                    else {
                        State::Loaded
                    }
                },
                State::Loaded => State::Loaded
            };

            let coord_of_interest = (56, 508); // XXX Plus (63, 508)
            let pix_of_interest = screen.get_pixel(coord_of_interest.0, coord_of_interest.1);
            if pix_of_interest == &Rgba([247, 247, 247, 255])
                || pix_of_interest == &Rgba([255, 243, 249, 255])
            {
                return Err(screen);
            }

            std::thread::sleep(Duration::from_millis(1000 / 10));
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct EmuCli {
    #[arg(
        short,
        long,
        help = "Completely hide the emulator window (not really tested ATM)",
        default_value = "false"
    )]
    transparent: bool,

    #[arg(
        short = 'a',
        long = "drivea",
        value_name = "DISCA",
        help = "Disc A image"
    )]
    drive_a: Option<String>,

    #[arg(
        short = 'b',
        long = "driveb",
        value_name = "DISCB",
        help = "Disc B image"
    )]
    drive_b: Option<String>,

    #[arg(
        long = "albireo",
        value_name = "FOLDER",
        help = "Albireo content (only for ACE) - WARNING. It is destructive as it completely replaces the existing content"
    )]
    albireo: Option<String>,

    #[arg(
        long = "snapshot",
        value_name = "SNAPSHOT",
        help = "Specify the snapshot to launch"
    )]
    snapshot: Option<String>,

    #[arg(
        long = "csl",
        value_name = "FILE",
        help = "Run this CSL (CPC Script Language) file. Emulators with native CSL support \
                (see Emulator::accept_csl) receive it directly; every other emulator is driven \
                through the CSL interpreter instead (see cpclib_runner::csl_interpreter) - only \
                leading disk/snapshot instructions and key_output/wait* are honored there.",
        conflicts_with_all = ["snapshot", "drive_a", "drive_b", "auto_run_file", "auto_type_file"]
    )]
    csl: Option<Utf8PathBuf>,

    #[arg(
        long = "csl-base-dir",
        value_name = "DIR",
        help = "Resolve --csl's script's own relative disk_insert/snapshot_load/etc. paths \
                against this directory instead of the --csl file's own parent directory. Set \
                automatically by editor integrations when the script is a temp copy of a file \
                that lives (and whose relative paths are meant to resolve) somewhere else.",
        requires = "csl"
    )]
    csl_base_dir: Option<Utf8PathBuf>,

    #[arg(short, long, value_parser = clap::builder::PossibleValuesParser::new(&["64", "128", "192", "256", "320", "576", "1088", "2112"]), help="Memory configuration")]
    memory: Option<String>,

    #[arg(short, long, value_parser = value_parser!(Crtc), help="Choice of the CRTC [possible values: 0, 1, 2, 3, 4]")]
    crtc: Option<Crtc>,

    #[arg(
        short,
        long,
        default_value = "ace",
        alias = "emu",
        help = "Which emulator to use [possible values: ace, winape, cpcec, amspirit, sugarbox, cpcemupower, cpcemu, caprice, cadence, emulator1984, rvm]"
    )]
    emulator: Emu,

    #[arg(short, long, action = ArgAction::SetTrue, help = "Keep the emulator open after the interaction")]
    keepemulator: bool,

    #[arg(short='C', long, action = ArgAction::SetTrue, help = "Clear the cache folder")]
    clear_cache: bool,

    #[arg(
        short = 'B',
        long,
        action = ArgAction::SetTrue,
        help = "Do not wait for the emulator window to close after the task completes (fire and forget)"
    )]
    background: bool,

    #[arg(short, long, action = ArgAction::Append, help = "rasm-compatible debug file (for ace ATM)")]
    debug: Vec<Utf8PathBuf>,

    #[arg(long, action= ArgAction::SetTrue)]
    break_on_bad_vbl: bool,

    #[arg(long, action= ArgAction::SetTrue)]
    break_on_bad_hbl: bool,

    #[arg(short='r', long, aliases = ["auto", "run", "autoRunFile"], action = ArgAction::Set, help = "The file to run" )]
    auto_run_file: Option<String>,

    #[arg(long, aliases = ["autotype", "type", "autoTypeFile"], action=ArgAction::Set, help = "The file that contains the text to type", conflicts_with="auto_run_file")]
    auto_type_file: Option<Utf8PathBuf>,

    #[arg(long, action=ArgAction::Append, help="List the ROMS to deactivate")]
    disable_rom: Vec<AmstradRom>,

    #[arg(long, action=ArgAction::Append, help="List the ROMS to activate")]
    enable_rom: Vec<AmstradRom>,

    #[command(subcommand)]
    command: Commands
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Emu {
    Ace,
    /// The portable AMSpiriT build - a different emulator from `Amspirit`,
    /// with its own releases and its own command line.
    Amspiritlite,
    Winape,
    Cpcec,
    Amspirit,
    Sugarbox,
    Cpcemupower,
    Cpcemu,
    Caprice,
    Cadence,
    #[value(alias = "1984")]
    Emulator1984,
    /// The emscripten build of 1984, served in a browser rather than spawned.
    ///
    /// `run` serves it on loopback and opens a browser - there is no process
    /// to spawn - see [`Dispatch::of`]. An external debugger integration
    /// elsewhere in this workspace can attach to it separately, through its
    /// own, differently patched install.
    #[value(alias = "1984js")]
    Emulator1984Js,
    #[value(alias = "retrovm")]
    Rvm
}

/// One emulator, as an editor integration would want to offer it in a
/// picker: the exact CLI string this crate's own `--emulator` flag accepts
/// (via [`clap::ValueEnum`], never hand-derived, so it can't drift from
/// what `EmuCli` actually parses), a display label, whether it is one of
/// the two an external debugger integration elsewhere in this workspace can
/// debug, and whether it is already installed - so a caller can show a
/// "needs installing" hint before spending the time to fetch it.
pub struct EmulatorListEntry {
    pub id: String,
    pub label: &'static str,
    pub debuggable: bool,
    pub installed: bool,
    /// The exact string that debugger integration's own launch-time
    /// `emulator` property wants - a *different* naming scheme from `id`
    /// above (this crate's own `--emulator` flag), so a caller driving that
    /// integration directly (an editor's debug launch) needs this instead
    /// of `id`. `None` for every non-`debuggable` entry, since that
    /// integration has no name for those at all.
    pub debugger_launch_id: Option<&'static str>
}

/// Every emulator this crate knows how to run, for an editor integration to
/// list (e.g. a "run/debug with..." picker) - see [`EmulatorListEntry`].
///
/// `debuggable` here means "an external debugger integration elsewhere in
/// this workspace can debug it" - 1984js and AMSpiriT Lite, the same set
/// that integration's own launch path checks against. This crate's own CLI
/// has no notion of a debug launch at all: `run --emulator 1984js` always
/// serves the plain page (see [`Dispatch::of`]).
pub fn list_emulators() -> Vec<EmulatorListEntry> {
    fn id(value: Emu) -> String {
        value
            .to_possible_value()
            .expect("every Emu variant has a possible value")
            .get_name()
            .to_string()
    }

    vec![
        EmulatorListEntry {
            id: id(Emu::Ace),
            label: "ACE-DL",
            debuggable: false,
            debugger_launch_id: None,
            installed: Emulator::Ace(Default::default())
                .configuration::<()>()
                .is_cached()
        },
        EmulatorListEntry {
            id: id(Emu::Amspirit),
            label: "AMSpiriT",
            debuggable: false,
            debugger_launch_id: None,
            installed: Emulator::Amspirit(Default::default())
                .configuration::<()>()
                .is_cached()
        },
        EmulatorListEntry {
            id: id(Emu::Amspiritlite),
            label: "AMSpiriT Lite",
            debuggable: true,
            debugger_launch_id: Some("amspiritlite"),
            installed: Emulator::AmspiritLite(Default::default())
                .configuration::<()>()
                .is_cached()
        },
        EmulatorListEntry {
            id: id(Emu::Winape),
            label: "WinAPE",
            debuggable: false,
            debugger_launch_id: None,
            installed: Emulator::Winape(Default::default())
                .configuration::<()>()
                .is_cached()
        },
        EmulatorListEntry {
            id: id(Emu::Cpcec),
            label: "CPCEC",
            debuggable: false,
            debugger_launch_id: None,
            installed: Emulator::Cpcec(Default::default())
                .configuration::<()>()
                .is_cached()
        },
        EmulatorListEntry {
            id: id(Emu::Sugarbox),
            label: "SugarBox v2",
            debuggable: false,
            debugger_launch_id: None,
            installed: Emulator::SugarBoxV2(Default::default())
                .configuration::<()>()
                .is_cached()
        },
        EmulatorListEntry {
            id: id(Emu::Cpcemupower),
            label: "CPCEmuPower",
            debuggable: false,
            debugger_launch_id: None,
            installed: Emulator::CpcEmuPower(Default::default())
                .configuration::<()>()
                .is_cached()
        },
        EmulatorListEntry {
            id: id(Emu::Cpcemu),
            label: "CPCEmu",
            debuggable: false,
            debugger_launch_id: None,
            installed: Emulator::CpcEmu(Default::default())
                .configuration::<()>()
                .is_cached()
        },
        EmulatorListEntry {
            id: id(Emu::Caprice),
            label: "CaPriCe Forever",
            debuggable: false,
            debugger_launch_id: None,
            installed: Emulator::CapriceForever(Default::default())
                .configuration::<()>()
                .is_cached()
        },
        EmulatorListEntry {
            id: id(Emu::Cadence),
            label: "Cadence",
            debuggable: false,
            debugger_launch_id: None,
            installed: Emulator::Cadence(Default::default())
                .configuration::<()>()
                .is_cached()
        },
        EmulatorListEntry {
            id: id(Emu::Emulator1984),
            label: "1984 (native)",
            debuggable: false,
            debugger_launch_id: None,
            installed: Emulator::Emulator1984(Default::default())
                .configuration::<()>()
                .is_cached()
        },
        EmulatorListEntry {
            id: id(Emu::Emulator1984Js),
            label: "1984js (browser)",
            debuggable: true,
            debugger_launch_id: Some("1984js"),
            installed: crate::web::js1984::is_installed()
        },
        EmulatorListEntry {
            id: id(Emu::Rvm),
            label: "Retro Virtual Machine",
            debuggable: false,
            debugger_launch_id: None,
            installed: Emulator::RetroVm(Default::default())
                .configuration::<()>()
                .is_cached()
        },
    ]
}

use clap::Args;

#[derive(Args, Clone, Debug)]
pub struct OrgamsCli {
    /// lists test values
    #[arg(short, long, help = "Filename to assemble or edit", aliases = &["source", "input"])]
    src: String,

    #[arg(
        short,
        long,
        help = "Filename to save after assembling. By default use the one provided by orgams"
        , aliases = &["destination", "tgt", "target", "ouput"])]
    dst: Option<String>,

    #[arg(
            long,
            action = ArgAction::SetTrue,
            requires = "dst",
            help = "Convert a Z80 source file into an ascii orgams file",
            aliases = &["basm2o"],
            group = "convert"
        )]
    basm2orgamsa: bool,

    #[arg(
        long,
        action = ArgAction::SetTrue,
        requires = "dst",
        help = "Convert an ASCII-compatible orgams file  into a binary orgams file",
        aliases = &["a2o"],
        group = "convert"
    )]
    orgamsa2orgamsb: bool,

    #[arg(
        long,
        action = ArgAction::SetTrue,
        requires = "dst",
        help = "Convert a binary orgams file into an ASCII-compatible orgams file",
        aliases = &["o2a"],
        group = "convert"
    )]
    orgamsb2orgamsa: bool,

    #[arg(
            short,
            long,
            action = ArgAction::SetTrue,
            alias = "monogams",
            conflicts_with_all = ["dst", "jump"],
            help = "Launch the editor in an emulator"
        )]
    edit: bool,

    #[arg(
            short,
            long,
            action = ArgAction::SetTrue,
            conflicts_with_all = ["dst", "edit"],
            help="Jump on the program instead of saving it")]
    jump: bool
}

#[derive(Subcommand, Clone, Debug)]
pub enum Commands {
    /// Assemble or interactively edit a file with the Orgams editor,
    /// running inside the emulator.
    #[cfg(feature = "screenshot")]
    Orgams(OrgamsCli),

    /// Launch the emulator and keep its window open afterwards (implies
    /// --keepemulator), optionally typing --text into it once it's ready.
    Run {
        #[arg(short, long, help = "Simple text to type")]
        text: Option<String>
    }
}

pub const EMUCTRL_CMD: &str = "cpc";

/// Serve the web emulator and hold the process open.
///
/// Separate from `start_emulator` because nothing is shared: there is no
/// process, no window to find, no argv translation - just files on a loopback
/// port and the snapshot the user named. This always serves the plain page:
/// an external debugger integration elsewhere in this workspace installs and
/// serves its own, separately patched, copy when it needs one to attach to.
fn serve_web_emulator<E: EventObserver>(conf: &EmulatorConf, o: &E) -> Result<(), String> {
    let snapshot = match conf.snapshot.as_ref() {
        Some(path) => Some(fs_err::read(path).map_err(|e| format!("cannot read {path}: {e}"))?),
        None => None
    };

    let root = crate::web::js1984::install()?;
    let server = crate::web::serve(&root, snapshot)
        .map_err(|e| format!("cannot serve the emulator: {e}"))?;
    let url = server.plain_url();

    o.emit_stdout(&format!("1984js is serving at {url}\n"));
    if let Err(problem) = webbrowser::open(&url) {
        o.emit_stderr(&format!("could not open a browser: {problem}\n"));
    }
    o.emit_stdout("Press Ctrl-C to stop.\n");
    loop {
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
}

pub struct EmulatorFacadeRunner<E: EventObserver> {
    command: Command,
    _phantom: PhantomData<E>
}

impl<E: EventObserver> Default for EmulatorFacadeRunner<E> {
    fn default() -> Self {
        Self {
            command: EmuCli::command(),
            _phantom: Default::default()
        }
    }
}

impl<E: EventObserver + Clone + 'static> Runner for EmulatorFacadeRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
        let mut itr = itr.iter().map(|s| s.as_ref()).collect_vec();
        itr.insert(0, EMUCTRL_CMD);
        let cli = EmuCli::try_parse_from(itr).map_err(|e| e.to_string())?;

        handle_arguments(cli, o)
    }

    fn get_command(&self) -> &str {
        EMUCTRL_CMD
    }
}

impl<E: EventObserver + Clone + 'static> RunnerWithClap for EmulatorFacadeRunner<E> {
    fn get_clap_command(&self) -> &clap::Command {
        &self.command
    }
}

/// What a command line asks for, decided before anything is installed, served
/// or spawned.
///
/// Pulled out of `handle_arguments` because it is the whole of a bug worth a
/// test: `run --emulator 1984js` used to fall past this decision into the
/// emulator table, whose `Emulator1984Js` entry quietly handed back the
/// *desktop* 1984. It started the native emulator and said nothing about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dispatch {
    /// Serve 1984js on loopback rather than spawning it as a process.
    ServeWeb,
    /// Install and spawn a desktop emulator.
    Native
}

impl Dispatch {
    pub fn of(emulator: Emu) -> Self {
        if matches!(emulator, Emu::Emulator1984Js) {
            Self::ServeWeb
        }
        else {
            Self::Native
        }
    }
}

/// Resolves an `--emulator` choice into the `Emulator` it spawns - factored
/// out of `handle_arguments`'s own match so `run_csl_file` can reuse it
/// without duplicating the arm list. `Emulator1984Js` has no `Emulator`
/// variant (it is served on loopback, never spawned as a process - see
/// `Dispatch::of`), so it is the one choice this returns `Err` for.
fn emulator_from_choice(choice: Emu) -> Result<Emulator, String> {
    Ok(match choice {
        Emu::Ace => Emulator::Ace(Default::default()),
        Emu::Winape => Emulator::Winape(Default::default()),
        Emu::Cpcec => Emulator::Cpcec(Default::default()),
        Emu::Caprice => Emulator::CapriceForever(Default::default()),
        Emu::Amspirit => Emulator::Amspirit(Default::default()),
        Emu::Amspiritlite => Emulator::AmspiritLite(Default::default()),
        Emu::Sugarbox => Emulator::SugarBoxV2(Default::default()),
        Emu::Cpcemupower => Emulator::CpcEmuPower(Default::default()),
        Emu::Cpcemu => Emulator::CpcEmu(Default::default()),
        Emu::Cadence => Emulator::Cadence(Default::default()),
        Emu::Emulator1984 => Emulator::Emulator1984(Default::default()),
        Emu::Emulator1984Js => {
            return Err(
                "1984js is served in a browser, not spawned as a process - it cannot run a CSL \
                 file this way."
                    .to_string()
            );
        },
        Emu::Rvm => Emulator::RetroVm(Default::default())
    })
}

/// Runs a `.csl` file against `cli.emulator` - the `--csl` entry point,
/// independent of `handle_arguments`'s main flow (no Ace ROM/albireo setup,
/// no HFE conversion - a CSL script's own `disk_insert`/`snapshot_load` are
/// the only media it can express anyway).
///
/// Emulators natively accepting CSL (`Emulator::accept_csl`) get the file
/// passed straight through as `--csl=<path>`, bypassing
/// `EmulatorConf::args_for_emu`'s synthesis-from-scalar-fields path
/// entirely - the user's own file is what runs, not a reconstruction of it.
/// Every other emulator is driven through `csl_interpreter`: leading
/// disk/snapshot instructions become launch arguments, then
/// `key_output`/`key_from_file`/`wait*` are replayed live via the same
/// `Robot`/enigo layer `Commands::Run`'s `--text` already uses. See
/// `csl_interpreter`'s own doc comment for what's deliberately left
/// unsupported.
fn run_csl_file<E: EventObserver + Clone + 'static>(
    cli: &EmuCli,
    csl_path: &Utf8Path,
    o: &E
) -> Result<(), String> {
    let source = fs_err::read_to_string(csl_path)
        .map_err(|e| format!("Could not read {csl_path}: {e}"))?;
    let script = cpclib_csl::parse_csl_with_rich_errors(&source, Some(csl_path.to_string()))
        .map_err(|e| e.to_string())?;

    // Every native-CSL-capable emulator launches with its own install
    // directory as its working directory (see `crate::runner::exec`'s
    // `RunInDir::AppDir`), not this file's location - a relative
    // `disk_insert`/`snapshot_load`/etc. path in the script means "next to
    // this script", so it must be made absolute *before* the script ever
    // reaches an emulator (native: re-serialized below; non-native:
    // folded into `EmulatorConf` further down) - otherwise a perfectly
    // ordinary, portable CSL script (real Shaker-authored ones routinely
    // look exactly like this) fails with "unable to find the dsk".
    //
    // `--csl-base-dir` overrides this when `csl_path` is itself a temp
    // copy living somewhere unrelated to the script's *real* location
    // (e.g. an editor's "unsaved changes" copy) - see that flag's own
    // help text and `cpclib_bndbuild::pipeline::csl_run`'s doc comment.
    let base_dir = cli
        .csl_base_dir
        .as_deref()
        .or_else(|| csl_path.parent())
        .unwrap_or_else(|| Utf8Path::new("."));
    let script = crate::csl_interpreter::resolve_relative_paths(&script, base_dir);

    let emu = emulator_from_choice(cli.emulator)?;

    {
        let conf = emu.configuration();
        if !conf.is_cached() {
            conf.install(o)?;
        }
    }

    if emu.accept_csl() {
        let rewritten_path = write_csl_script_to_temp_file(&script)?;
        let csl_arg = emu.wine_compatible_fname(&rewritten_path)?;
        let args = native_csl_args(&emu, &csl_arg);
        if cli.background {
            let (emu, args, transparent, o_owned) = (emu.clone(), args, cli.transparent, o.clone());
            std::thread::spawn(move || {
                spawn_emulator_with_args(&emu, transparent, &args, &o_owned)
            });
            return Ok(());
        }
        return spawn_emulator_with_args(&emu, cli.transparent, &args, o);
    }

    // Non-native: fold leading config into a fresh EmulatorConf (drive_a/
    // drive_b/memory/crtc still honor whatever the CLI itself was also
    // given, same as the main flow - a CSL script's own leading
    // disk_insert/snapshot_load additionally override those, since the
    // script is what the user actually asked to run).
    let (leading, live) = crate::csl_interpreter::split_leading_and_live(&script);
    let base_conf = EmulatorConf::builder()
        .transparent(cli.transparent)
        .maybe_drive_a(cli.drive_a.clone().map(Into::into))
        .maybe_drive_b(cli.drive_b.clone().map(Into::into))
        .maybe_crtc(cli.crtc)
        .maybe_snapshot(cli.snapshot.clone().map(Into::into))
        .debug_files(cli.debug.clone())
        .maybe_memory(cli.memory.clone().map(|v| v.parse::<u32>().unwrap()))
        .break_on_bad_hbl(cli.break_on_bad_hbl)
        .break_on_bad_vbl(cli.break_on_bad_vbl)
        .build();
    let conf = crate::csl_interpreter::fold_leading_instructions(&leading, base_conf);

    let (t_emu, conf_thread, o_thread) = (emu.clone(), conf.clone(), o.clone());
    let emu_thread = std::thread::spawn(move || start_emulator(&t_emu, &conf_thread, &o_thread));

    // A script with nothing left to replay live (every instruction folded
    // into `conf` above - a bare `disk_insert`/`snapshot_load` script with
    // no `key_output`/`wait*` is a real, common case) needs no window at
    // all: skip the settle-time sleep and the window search, since both
    // exist solely to prepare for the typing loop below.
    if !live.is_empty() {
        std::thread::sleep(Duration::from_secs(3));

        let window = get_emulator_window(&emu, &conf, o);
        if window.is_none() {
            o.emit_stderr(&format!(
                "No emulator window found for '{}' - live CSL instructions (key_output/wait*) will \
                 not be replayed.\n",
                emu.get_command()
            ));
        }
        else {
            let enigo_settings = {
                let mut settings = Settings {
                    linux_delay: 1000 / 10,
                    ..Default::default()
                };
                if let Some(EmuWindow::Xvfb(display, _)) = &window {
                    settings.x11_display = Some(format!(":{display}"));
                }
                settings
            };
            let enigo = Enigo::new(&enigo_settings).map_err(|e| e.to_string())?;
            let events = enigo.into();
            let mut robot = Robot::new(&emu, window, events);

            for step in crate::csl_interpreter::plan_live_steps(&live) {
                match step {
                    crate::csl_interpreter::CslLiveStep::TypeText(text) => robot.handle_raw_text(text),
                    crate::csl_interpreter::CslLiveStep::Sleep(d) => std::thread::sleep(d),
                    crate::csl_interpreter::CslLiveStep::Unsupported(msg) => {
                        o.emit_stdout(&format!("CSL: skipped unsupported live step - {msg}\n"));
                    }
                }
            }

            if !cli.keepemulator {
                robot.close();
            }
        }
    }

    if cli.background {
        return Ok(());
    }
    emu_thread
        .join()
        .unwrap_or_else(|_| Err("emulator thread panicked".to_string()))
}

pub fn handle_arguments<E: EventObserver + Clone + 'static>(
    mut cli: EmuCli,
    o: &E
) -> Result<(), String> {
    if cli.clear_cache {
        clear_base_cache_folder().map_err(|e| format!("Unable to clear the cache folder. {e}"))?;
    }

    if let Some(csl_path) = cli.csl.clone() {
        return run_csl_file(&cli, &csl_path, o);
    }

    let builder = EmulatorConf::builder()
        .transparent(cli.transparent)
        .maybe_drive_a(cli.drive_a.clone().map(|a| a.into()))
        .maybe_drive_b(cli.drive_b.clone().map(|a| a.into()))
        .maybe_crtc(cli.crtc)
        .maybe_snapshot(cli.snapshot.clone().map(|a| a.into()))
        .debug_files(cli.debug.clone())
        .maybe_auto_run(cli.auto_run_file.clone())
        .maybe_auto_type(cli.auto_type_file.clone())
        .maybe_memory(cli.memory.clone().map(|v| v.parse::<u32>().unwrap()))
        .break_on_bad_hbl(cli.break_on_bad_hbl)
        .break_on_bad_vbl(cli.break_on_bad_vbl);
    let conf = builder.build();

    // Answered before anything native happens: a web emulator is *served*, not
    // spawned, and falling through would install and start the desktop 1984 as
    // well.
    match Dispatch::of(cli.emulator) {
        Dispatch::ServeWeb => {
            return serve_web_emulator(&conf, o);
        },
        Dispatch::Native => {}
    }

    // Guaranteed not `Emulator1984Js` here: `Dispatch::of` above already
    // served it before anything native is reached.
    let emu = emulator_from_choice(cli.emulator)
        .expect("1984js is served above, never reaches emulator_from_choice");

    {
        // ensure emulator is installed to properly handle its setup
        let conf = emu.configuration();
        if !conf.is_cached() {
            conf.install(o)?;
        }
    }

    // setup emulator
    // todo standardize that to shorten code and avoid copy paste
    if cli.emulator == Emu::Ace {
        // copy the non standard roms and configure the emu (at least ace)
        let ace_conf_path = emu.ace_version().unwrap().config_file(); // todo get it programmatically
        let mut ace_conf = AceConfig::open_or_default(&ace_conf_path);
        ace_conf.sanitize();

        ace_conf.remove_cartridge();
        ace_conf.select_crtc(cli.crtc.unwrap_or_default());
        ace_conf.set_bool("BVBLBREAK", cli.break_on_bad_vbl);
        ace_conf.set_bool("BHBLBREAK", cli.break_on_bad_hbl);

        if let Some(mem) = &cli.memory {
            ace_conf.set("RAM", mem);
        }
        else {
            ace_conf.set("RAM", 128);
        }

        // Ensure system is French. TODO handle that properly for foreign partners !
        ace_conf.set(
            "OS",
            emu.configuration::<E>()
                .cache_folder()
                .join("private/firmware/OS6128_FR.rom") // TODO handle different languages and versions (at least 6128 vs 464
        );
        ace_conf.set("KTRANS", 1);
        ace_conf.set("KGTRANS", 1);

        // (rom filename, ACE slot number, optional plugin flag it also enables)
        type RomFile = (&'static str, usize, Option<AceConfigFlag>);
        let extra_roms: &[(AmstradRom, &[RomFile])] = &[
            (
                AmstradRom::Unidos,
                &[
                    ("unidos.rom", 7, None),
                    ("nova.rom", 8, Some(AceConfigFlag::PluginNova)),
                    ("albireo.rom", 9, Some(AceConfigFlag::PluginAlbireo1)),
                    ("parados12.fixedanyslot.fixedname.quiet.rom", 10, None)
                ]
            ),
            (AmstradRom::Orgams, &[("Orgams_FF240128.e0f", 15, None)])
        ];

        // ensure we force unidos rom when using alibreo
        if cli.albireo.is_some() {
            if cli.disable_rom.contains(&AmstradRom::Unidos) {
                return Err(
                    "You cannot disable Unidos when using Albireo as it is required".to_string()
                );
            }
            else if !cli.enable_rom.contains(&AmstradRom::Unidos) {
                cli.enable_rom.push(AmstradRom::Unidos);
            }
        }

        // for fname in EmbeddedRoms::iter() {
        // println!("{fname}");
        // }
        for (kind, roms) in extra_roms {
            let remove = cli.disable_rom.contains(kind);
            let install = cli.enable_rom.contains(kind);

            if remove && install {
                return Err(format!(
                    "You cannot both enable and disable the same ROM {kind:?}. Make a choice between --enable-rom and --disable-rom"
                ));
            }

            // a minimum ammount of memory is required
            if !remove && kind == &AmstradRom::Orgams && cli.memory.is_none() {
                ace_conf.set("RAM", 576);
            }

            for (rom, slot, plugin) in roms.iter() {
                let dst = emu.roms_folder().join(rom);
                let exists = dst.exists();

                if !exists && install {
                    let src = format!("roms://{rom}");
                    o.emit_stdout(&format!("Install {src} in {dst}\n"));
                    let data =
                        EmbeddedRoms::get(&src).unwrap_or_else(|| panic!("{src} not embedded"));
                    fs_err::write(&dst, data.data).unwrap();
                }
                else if exists && remove {
                    fs_err::remove_file(&dst).unwrap();
                }

                let key = format!("ROM{slot}");
                if remove {
                    ace_conf.remove(&key);
                }
                else if install {
                    ace_conf.set(key, dst.to_string());
                    if let Some(plugin) = plugin {
                        ace_conf.enable(*plugin);
                    }
                }
            }
        }
        ace_conf.save().unwrap();
    }

    let albireo_backup_and_original = {
        if emu.is_ace() {
            let emu_folder = emu.albireo_folder();
            let backup_folder = emu_folder
                .parent()
                .unwrap()
                .join(emu_folder.file_name().unwrap().to_owned() + ".bak");

            if backup_folder.exists() {
                fs_err::remove_dir_all(&backup_folder).unwrap();
            }

            if emu_folder.exists() {
                fs_err::rename(&emu_folder, &backup_folder).map_err(|e| e.to_string())?;
            }

            Some((backup_folder, emu_folder))
        }
        else {
            None
        }
    };

    if emu.is_ace() {
        #[allow(unused_variables)]
        let emu_folder = emu.albireo_folder();

        if let Some(albireo) = &cli.albireo {
            #[cfg(unix)]
            {
                let (_backup_folder, emu_folder) = albireo_backup_and_original.as_ref().unwrap();

                std::os::unix::fs::symlink(
                    std::path::absolute(albireo).unwrap(),
                    std::path::absolute(emu_folder).unwrap()
                )
                .unwrap();
            }

            #[cfg(windows)]
            {
                let option = fs_extra::dir::CopyOptions::new()
                    .copy_inside(true)
                    .overwrite(true)
                    .skip_exist(false)
                    .content_only(true);
                fs_extra::dir::copy(albireo, &emu_folder, &option).unwrap();
            }
        }
    }

    // I had issues with symlinks on windows. no time to search why
    #[cfg(windows)]
    if let Some(albireo) = &cli.albireo {
        let option = fs_extra::dir::CopyOptions::new()
            .copy_inside(true)
            .overwrite(true)
            .skip_exist(false)
            .content_only(true);
        let emu_folder = emu.albireo_folder();
        if emu_folder.exists() {
            fs_err::remove_dir_all(&emu_folder).unwrap();
        }
        fs_extra::dir::copy(albireo, &emu_folder, &option).unwrap();
    }

    let t_emu = emu.clone();
    let conf_thread = conf.clone();
    // Clone the observer so the emulator thread writes its stdout/stderr in real-time
    // through the same observer as the rest of the task.
    let o_for_emu = o.clone();
    let emu_thread = if cli.background {
        // Fire and forget: spawn without joining; emulator output still forwarded live.
        std::thread::spawn(move || {
            let _ = start_emulator(&t_emu, &conf_thread, &o_for_emu);
        });
        None
    }
    else {
        let handle = std::thread::spawn(move || start_emulator(&t_emu, &conf_thread, &o_for_emu));
        Some(handle)
    };

    // This sleep exists to give the emulator's window time to actually
    // appear before the code below goes looking for it with
    // `get_emulator_window` - it is dead time otherwise. Only `Orgams` and
    // `Run` with `--text` ever call `get_emulator_window`; a plain
    // `run --background` (by far the most common shape - every editor
    // "Run in emulator"/"Debug" integration launches exactly this) needs no
    // window at all and used to pay this tax unconditionally regardless of
    // `--background` or whether `--text` was even given, turning every such
    // launch into a flat 3-5 extra real seconds for nothing. `get_emulator_
    // window`'s own retry loop (`get_emulator_window_xcap`, up to ~6s on its
    // own) already tolerates the window not being up yet on its first poll,
    // so skipping this upfront wait when it is not needed costs nothing when
    // it *is* needed elsewhere - just a couple of extra poll iterations.
    let needs_window_settle_time = match &cli.command {
        #[cfg(feature = "screenshot")]
        Commands::Orgams(_) => true,
        Commands::Run { text } => text.is_some()
    };
    if needs_window_settle_time {
        if cli.albireo.is_some() {
            std::thread::sleep(Duration::from_secs(5));
        }
        else {
            std::thread::sleep(Duration::from_secs(3));
        }
    }

    let res = match cli.command {
        #[cfg(feature = "screenshot")]
        Commands::Orgams(OrgamsCli {
            src,
            dst,
            jump,
            edit,
            basm2orgamsa,
            orgamsa2orgamsb,
            orgamsb2orgamsa
        }) => {
            let window = get_emulator_window(&emu, &conf, o).ok_or_else(|| {
                format!(
                    "No emulator window found for '{}'. The emulator may be on another desktop/workspace.",
                    emu.get_command()
                )
            })?;
            let enigo_settings = {
                let mut settings = Settings {
                    linux_delay: 1000 / 10,
                    ..Default::default()
                };
                if let EmuWindow::Xvfb(display, _) = &window {
                    // The X11 DISPLAY convention is `:N` (see the identical
                    // `std::env::set_var("DISPLAY", ...)` call above) - a
                    // second, colon-less assignment used to immediately
                    // overwrite this with a value X11 wouldn't recognize.
                    settings.x11_display = Some(format!(":{display}"));
                }
                settings
            };
            let enigo = Enigo::new(&enigo_settings).unwrap();
            let events = enigo.into();
            let mut robot = Robot::new(&emu, Some(window), events);

            #[cfg(windows)]
            std::thread::sleep(Duration::from_millis(1000 * 3));

            let res = if basm2orgamsa {
                if let Some(albi) = &cli.albireo {
                    let src = Utf8Path::new(albi).join(src);
                    let dst = dst.as_ref().unwrap();
                    cpclib_asm::orgams::convert_from_to(src, dst).map_err(|e| e.to_string())
                }
                else {
                    unimplemented!("Need to code the necessary conversion stuff from disc")
                }
            }
            else if (jump || edit) && !cli.keepemulator {
                robot.close();
                Err("You must request to keep the emulator open with -k".to_string())
            }
            else {
                let action = if orgamsa2orgamsb {
                    OrgamsRobotAction::new_save_sources(&src, dst.as_ref().unwrap())?
                }
                else if orgamsb2orgamsa {
                    OrgamsRobotAction::new_export_sources(&src, dst.as_ref().unwrap())?
                }
                else if edit {
                    OrgamsRobotAction::new_edit(&src)
                }
                else if jump {
                    OrgamsRobotAction::new_jump(&src)
                }
                else {
                    OrgamsRobotAction::new_save_binary(&src, dst.as_deref())
                };

                robot.handle_orgams(cli.drive_a.as_deref(), cli.albireo.as_deref(), action, o)
            };

            if !cli.keepemulator {
                robot.close();
            }

            res
        },

        Commands::Run { text } => {
            cli.keepemulator = true;

            if let Some(text) = text {
                let window = get_emulator_window(&emu, &conf, o).ok_or_else(|| {
                    format!(
                        "No emulator window found for '{}'. The emulator may be on another desktop/workspace.",
                        emu.get_command()
                    )
                })?;
                let enigo_settings = {
                    let mut settings = Settings {
                        linux_delay: 1000 / 10,
                        ..Default::default()
                    };
                    if let EmuWindow::Xvfb(display, _) = &window {
                        // See the identical fix's comment in the other
                        // enigo_settings block above.
                        settings.x11_display = Some(format!(":{display}"));
                    }
                    settings
                };
                let enigo = Enigo::new(&enigo_settings).unwrap();
                let events = enigo.into();
                let mut robot = Robot::new(&emu, Some(window), events);
                robot.handle_raw_text(text);
            }

            Ok(())
        }
    };

    #[allow(unused_variables)]
    if let Some((backup_folder, emu_folder)) = albireo_backup_and_original {
        if cli.keepemulator {
            o.emit_stderr(
                "Albireo folder not cleaned automatically. you'll have to do it if necessary\n"
            );
        }
        else {
            #[cfg(windows)]
            {
                // need to copy back modifications
                let option = fs_extra::dir::CopyOptions::new()
                    .copy_inside(true)
                    .overwrite(true)
                    .skip_exist(false)
                    .content_only(true);
                let albireo = cli.albireo.as_ref().unwrap();
                fs_err::remove_dir_all(albireo).unwrap();
                fs_extra::dir::copy(&emu_folder, albireo, &option).unwrap();

                // restore previous
                if backup_folder.exists() {
                    fs_err::rename(&backup_folder, &emu_folder).map_err(|e| e.to_string())?;
                }
            }
        }
    }

    // For non-background tasks: block until the emulator window is closed.
    if let Some(handle) = emu_thread {
        let emu_result = handle
            .join()
            .unwrap_or_else(|_| Err("emulator thread panicked".to_string()));
        if let Err(e) = emu_result
            && res.is_ok()
        {
            return Err(e);
        }
    }

    res
}

#[cfg(test)]
mod tests {
    use cpclib_common::camino::Utf8PathBuf;

    use super::*;

    #[test]
    fn test_from_emulator_conf() {
        let conf = EmulatorConf {
            drive_a: Some(Utf8PathBuf::from("test.dsk")),
            drive_b: Some(Utf8PathBuf::from("data.dsk")),
            snapshot: Some(Utf8PathBuf::from("game.sna")),
            auto_run: Some("DISC".to_string()),
            auto_type: None,
            memory: Some(512),
            crtc: Some(Crtc::One),
            roms_configuration: Default::default(),
            debug_files: Vec::new(),
            break_on_bad_vbl: false,
            break_on_bad_hbl: false,
            transparent: false
        };

        let script = cpclib_csl::CslScript::from(conf);

        // Check that we have at least the expected instructions (may have more auto-generated ones)
        assert!(
            script.instructions().len() >= 8,
            "Expected at least 8 instructions, got {}",
            script.instructions().len()
        );

        // Verify DiskDir exists
        assert!(
            script
                .instructions()
                .iter()
                .any(|inst| matches!(inst, cpclib_csl::CslInstruction::DiskDir(_))),
            "Expected DiskDir instruction"
        );

        // Verify DiskInsert for drive A
        assert!(
            script.instructions().iter().any(|inst| {
                matches!(inst, cpclib_csl::CslInstruction::DiskInsert { drive, filename }
                    if *drive == cpclib_csl::Drive::A && filename == &Utf8PathBuf::from("test.dsk"))
            }),
            "Expected DiskInsert for drive A with test.dsk"
        );

        // Verify DiskInsert for drive B
        assert!(
            script.instructions().iter().any(|inst| {
                matches!(inst, cpclib_csl::CslInstruction::DiskInsert { drive, filename }
                    if *drive == cpclib_csl::Drive::B && filename == &Utf8PathBuf::from("data.dsk"))
            }),
            "Expected DiskInsert for drive B with data.dsk"
        );

        // Verify SnapshotDir exists
        assert!(
            script
                .instructions()
                .iter()
                .any(|inst| matches!(inst, cpclib_csl::CslInstruction::SnapshotDir(_))),
            "Expected SnapshotDir instruction"
        );

        // Verify SnapshotLoad
        assert!(
            script.instructions().iter().any(|inst| {
                matches!(inst, cpclib_csl::CslInstruction::SnapshotLoad(path)
                    if path == &Utf8PathBuf::from("game.sna"))
            }),
            "Expected SnapshotLoad with game.sna"
        );

        // Verify KeyOutput for auto_run
        assert!(
            script
                .instructions()
                .iter()
                .any(|inst| matches!(inst, cpclib_csl::CslInstruction::KeyOutput(_))),
            "Expected KeyOutput for auto_run"
        );

        // Verify MemoryExp
        assert!(
            script.instructions().iter().any(|inst| {
                matches!(inst, cpclib_csl::CslInstruction::MemoryExp(mem) 
                    if *mem == cpclib_csl::MemoryExpansion::Kb512DkTronics)
            }),
            "Expected MemoryExp with 512KB"
        );

        // Verify CrtcSelect
        assert!(
            script.instructions().iter().any(|inst| {
                matches!(inst, cpclib_csl::CslInstruction::CrtcSelect(crtc) 
                    if *crtc == cpclib_csl::CrtcModel::Type1)
            }),
            "Expected CrtcSelect with Type1"
        );
    }

    /// `run --emulator 1984js` serves the web emulator.
    ///
    /// It used to fall through to the emulator table and get the *desktop*
    /// 1984 instead - silently, because the substituting match arm carried a
    /// comment claiming this could not happen.
    #[test]
    fn asking_for_1984js_never_spawns_the_desktop_emulator() {
        assert_eq!(
            Dispatch::of(Emu::Emulator1984Js),
            Dispatch::ServeWeb,
            "1984js must be served, not spawned"
        );
    }

    /// Every other emulator is still spawned.
    #[test]
    fn other_emulators_are_still_spawned() {
        for emulator in [Emu::Ace, Emu::Winape, Emu::Cpcec, Emu::Emulator1984] {
            assert_eq!(Dispatch::of(emulator), Dispatch::Native, "{emulator:?}");
        }
    }

    #[test]
    fn csl_flag_parses_against_the_real_cli() {
        let args = ["cpc", "--csl", "script.csl", "--emulator", "winape", "run"];
        let cli = EmuCli::try_parse_from(args).unwrap();
        assert_eq!(cli.csl, Some(Utf8PathBuf::from("script.csl")));
    }

    #[test]
    fn csl_flag_conflicts_with_snapshot_and_drives() {
        let args = [
            "cpc", "--csl", "script.csl", "--snapshot", "game.sna", "run"
        ];
        assert!(
            EmuCli::try_parse_from(args).is_err(),
            "--csl and --snapshot must be mutually exclusive"
        );
    }

    #[test]
    fn csl_base_dir_flag_parses_alongside_csl_and_survives_a_space() {
        let args = [
            "cpc",
            "--csl",
            "script.csl",
            "--csl-base-dir",
            "MODULE A",
            "run"
        ];
        let cli = EmuCli::try_parse_from(args).unwrap();
        assert_eq!(cli.csl_base_dir, Some(Utf8PathBuf::from("MODULE A")));
    }

    #[test]
    fn csl_base_dir_flag_requires_csl() {
        let args = ["cpc", "--csl-base-dir", "somewhere", "run"];
        assert!(
            EmuCli::try_parse_from(args).is_err(),
            "--csl-base-dir without --csl should be rejected"
        );
    }

    #[test]
    fn native_csl_args_uses_the_flag_shape_each_emulator_actually_documents() {
        let path = Utf8PathBuf::from("/tmp/test.csl");

        // AMSpiriT: `--csl=<path>` as one argv element.
        assert_eq!(
            native_csl_args(&Emulator::Amspirit(Default::default()), &path),
            vec!["--csl=/tmp/test.csl".to_string()]
        );

        // SugarboxV2: `--csl <path>` as two argv elements (its own docs
        // show `-s, --csl <script>`, a space-separated value).
        assert_eq!(
            native_csl_args(&Emulator::SugarBoxV2(Default::default()), &path),
            vec!["--csl".to_string(), "/tmp/test.csl".to_string()]
        );
    }

    #[test]
    fn emulator_from_choice_rejects_only_1984js() {
        assert!(emulator_from_choice(Emu::Emulator1984Js).is_err());
        for choice in [
            Emu::Ace,
            Emu::Winape,
            Emu::Cpcec,
            Emu::Caprice,
            Emu::Amspirit,
            Emu::Amspiritlite,
            Emu::Sugarbox,
            Emu::Cpcemupower,
            Emu::Cpcemu,
            Emu::Cadence,
            Emu::Emulator1984,
            Emu::Rvm
        ] {
            assert!(emulator_from_choice(choice).is_ok(), "{choice:?}");
        }
    }
}

