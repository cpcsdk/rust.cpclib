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
use xcap::image::{ImageBuffer, Rgba, open};

use crate::ace_config::{AceConfig, AceConfigFlag};
use crate::delegated::{DelegatedRunner, clear_base_cache_folder};
use crate::embedded::EmbeddedRoms;
use crate::event::EventObserver;
use crate::runner::Runner;
use crate::runner::emulator::Emulator;
use crate::runner::runner::RunnerWithClap;

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

type EmuScreenShot = ImageBuffer<Rgba<u8>, Vec<u8>>;

#[derive(Debug)]
pub enum EmuWindow {
    Xcap(xcap::Window),
    Xvfb(usize, Option<wmctrl::Window>)
}

impl EmuWindow {
    pub fn capture_image(&self) -> EmuScreenShot {
        match self {
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

enum WindowEventsManager {
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

    pub fn shift_char<K: Into<HostKey>>(&mut self, c: K) {
        let c = c.into();

        match self {
            Self::Enigo(_enigo) => {
                self.enigo_press_with_extra(Key::Shift, c);
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
    /// Generate the args for the corresponding emulator
    pub fn args_for_emu(&self, emu: &Emulator) -> Result<Vec<String>, String> {
        // Use CSL script for Amspirit emulator
        if let Emulator::Amspirit(_) = emu {
            return self.args_for_emu_amspirit_with_csl(emu);
        }

        let mut args = Vec::default();

        if let Some(drive_a) = &self.drive_a {
            match emu {
                Emulator::Ace(_) | Emulator::Cpcec(_) => args.push(drive_a.to_string()),
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
            }
        }

        if let Some(drive_b) = &self.drive_b {
            match emu {
                Emulator::Ace(_) => return Err("Drive B not yet handled".to_owned()),
                Emulator::Cpcec(_) => return Err("Drive B not yet handled".to_owned()),
                Emulator::Winape(_) => return Err("Drive B not yet handled".to_owned()),
                Emulator::Amspirit(_) => return Err("Drive B not yet handled".to_owned()),
                Emulator::SugarBoxV2(_) => return Err("Drive B not yet handled".to_owned()),
                Emulator::CpcEmuPower(_cpc_emu_power_version) => {
                    args.push(format!("--dsk1={}", emu.wine_compatible_fname(drive_b)?))
                },
                Emulator::CapriceForever(_) => args.push(format!("/DriveB={drive_b}"))
            }
        }

        if let Some(sna) = &self.snapshot {
            match emu {
                Emulator::Ace(_ace_version) => args.push(sna.to_string()),
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
                Emulator::CpcEmuPower(_v) => {
                    args.push(format!("--sna={sna}"));
                },
                Emulator::CapriceForever(_v) => {
                    args.push(format!("/SNA=\"{sna}\""));
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
                Emulator::Ace(_) => todo!(),
                Emulator::Cpcec(_) => todo!(),
                Emulator::Winape(_) => todo!(),
                Emulator::Amspirit(_) => todo!(),
                Emulator::SugarBoxV2(_) => todo!(),
                Emulator::CpcEmuPower(_cpc_emu_power_version) => todo!(),
                Emulator::CapriceForever(_caprice_forever_version) => todo!()
            }
        }

        if !self.debug_files.is_empty() {
            match emu {
                Emulator::Ace(_) => {
                    let fname = merge_rasm_debug_to_tmp(&self.debug_files[..]).unwrap();
                    args.push(fname.to_string());
                },
                Emulator::Winape(_) => {
                    eprintln!(
                        "Breapoints are currently ignored. TODO convert them in the appropriate format"
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
                    eprintln!(
                        "Debug files are currently ignored. TODO convert them in the appropriate format"
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
                _ => unimplemented!()
            };
            args.push(arg.to_owned());
        }

        if let Some(ftype) = &self.auto_type {
            match emu {
                Emulator::Ace(_) => {
                    if let Some(ext) = dbg!(ftype.extension())
                        && ext == "txt"
                    {
                        args.push(ftype.as_str().to_string());
                    }
                    else {
                        return Err(format!("`{ftype}` should end by .txt"));
                    }
                },
                _ => {
                    eprintln!(
                        "Auto type file is currently ignored for this emulator {:?}",
                        emu
                    )
                }
            }
        }

        if let Some(run) = &self.auto_run {
            match emu {
                Emulator::Ace(_) => {
                    args.push("-autoRunFile".to_owned());
                    args.push(run.clone())
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
                Emulator::SugarBoxV2(_) => unimplemented!(),
                Emulator::CpcEmuPower(_) => args.push(format!("--auto=RUN\"{run}")),
                Emulator::CapriceForever(_v) => {
                    args.push(format!("/Command=RUN\"\"{run}"));
                }
            }
        }

        dbg!(&args);
        Ok(args)
    }

    /// Generate args for Amspirit emulator using CSL script
    fn args_for_emu_amspirit_with_csl(&self, emu: &Emulator) -> Result<Vec<String>, String> {
        // Generate CSL script from configuration
        let csl_script: cpclib_csl::CslScript = self.clone().into();

        // Convert to string
        let csl_content = csl_script.to_string();

        // Save to temporary file
        let tempfile = camino_tempfile::Builder::new()
            .suffix(".csl")
            .tempfile()
            .map_err(|e| format!("Failed to create temporary CSL file: {}", e))?;

        // Get the path as Utf8PathBuf before writing
        let temp_path_utf8 = tempfile.path().to_owned();

        fs_err::write(&temp_path_utf8, csl_content)
            .map_err(|e| format!("Failed to write CSL file: {}", e))?;

        // Keep the temporary file (prevent automatic deletion)
        let _kept = tempfile
            .into_temp_path()
            .keep()
            .map_err(|e| format!("Failed to keep temporary CSL file: {}", e))?;

        // Ensure we have an absolute path
        let absolute_path = if temp_path_utf8.is_absolute() {
            temp_path_utf8
        }
        else {
            let canonical = temp_path_utf8
                .canonicalize()
                .map_err(|e| format!("Failed to canonicalize CSL file path: {}", e))?;
            Utf8PathBuf::from_path_buf(canonical)
                .map_err(|p| format!("Invalid UTF-8 in canonical path: {:?}", p))?
        };

        // Get wine-compatible absolute path if needed
        let csl_path = emu.wine_compatible_fname(&absolute_path)?;

        // Return args with CSL file using absolute path
        dbg!(Ok(vec![format!("--csl={}", csl_path)]))
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

pub fn start_emulator(emu: &Emulator, conf: &EmulatorConf) -> Result<(), String> {
    let args = conf.args_for_emu(emu)?;
    let app = emu.configuration();

    let cmd = emu.get_command().into();
    let runner = if conf.transparent {
        DelegatedRunner::new_transparent(app, cmd)
    }
    else {
        DelegatedRunner::new(app, cmd)
    };

    runner.inner_run(&args, &())
}

pub fn get_emulator_window(emu: &Emulator, conf: &EmulatorConf) -> EmuWindow {
    if !conf.transparent {
        get_emulator_window_xcap(emu)
    }
    else {
        get_emulator_window_xvfb(emu)
    }
}

// XX this code seems buggy ATM it is unable to collect the window, no idea why
fn get_emulator_window_xvfb(emu: &Emulator) -> EmuWindow {
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
        .filter(|win| emu.window_name_corresponds(dbg!(win.title())))
        .collect_vec();

    let window = match windows.len() {
        0 => None,
        1 => windows.pop(),
        _ => {
            eprintln!("There are several available windows. I pick one, but it may be wrong");
            windows.pop()
        }
    };

    EmuWindow::Xvfb(display_nb, window)
}

fn get_emulator_window_xcap(emu: &Emulator) -> EmuWindow {
    let windows = xcap::Window::all().unwrap();
    let mut windows = windows
        .into_iter()
        .filter(|win| emu.window_name_corresponds(&win.title().unwrap()))
        .collect_vec();

    let window = match windows.len() {
        0 => panic!("No window emulator found"),
        1 => windows.pop().unwrap(),
        _ => {
            eprintln!("There are several available windows. I pick one, but it may be wrong");
            windows.pop().unwrap()
        }
    };

    EmuWindow::Xcap(window)
}

trait UsedEmulator: Sized {
    /// the default behavior consists in capturing the full window emulator.
    /// This can of course be tailored to get the emulated screen area
    fn screenshot(robot: &mut RobotImpl<Self>) -> EmuScreenShot
    where Self: Sized {
        robot.window.capture_image()
    }
}

struct AceUsedEmulator {}
struct CpcecUsedEmulator {}
struct WinapeUsedEmulator {}
struct AmspiritUsedEmulator {}
struct SugarBoxV2UsedEmulator {}
struct CpcEmuPowerUsedEmulator {}

struct CapriceForeverUsedEmulator {}

impl UsedEmulator for AceUsedEmulator {
    // here we delegate the creation of screenshot to Ace to avoid some issues i do not understand
    fn screenshot(robot: &mut RobotImpl<Self>) -> Screenshot {
        let folder = robot.emu.screenshots_folder();
        let before_screenshots: HashSet<_> = glob::glob(folder.join("*.png").as_str())
            .unwrap()
            .map(|p| p.unwrap().as_path().to_owned())
            .collect();

        // handlekey press
        robot.type_key(HostKey::F10);

        let mut file = None;
        while file.is_none() {
            WindowEventsManager::wait_a_bit();
            let after_screenshots: HashSet<_> = glob::glob(folder.join("*.png").as_str())
                .unwrap()
                .map(|p| p.unwrap().as_path().to_owned())
                .collect();
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
        fs_err::remove_file(file).unwrap();
        im
    }
}
impl UsedEmulator for CpcecUsedEmulator {}
impl UsedEmulator for WinapeUsedEmulator {}
impl UsedEmulator for SugarBoxV2UsedEmulator {}
impl UsedEmulator for AmspiritUsedEmulator {}
impl UsedEmulator for CpcEmuPowerUsedEmulator {}
impl UsedEmulator for CapriceForeverUsedEmulator {}

struct RobotImpl<E: UsedEmulator> {
    pub(crate) window: EmuWindow,
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

pub enum Robot {
    Ace(RobotImpl<AceUsedEmulator>),
    Cpcec(RobotImpl<CpcecUsedEmulator>),
    Winape(RobotImpl<WinapeUsedEmulator>),
    Amspirit(RobotImpl<AmspiritUsedEmulator>),
    SugarboxV2(RobotImpl<SugarBoxV2UsedEmulator>),
    CpcEmuPower(RobotImpl<CpcEmuPowerUsedEmulator>),
    CapriceForever(RobotImpl<CapriceForeverUsedEmulator>)
}

impl From<RobotImpl<AceUsedEmulator>> for Robot {
    fn from(value: RobotImpl<AceUsedEmulator>) -> Self {
        Self::Ace(value)
    }
}

impl From<RobotImpl<CpcecUsedEmulator>> for Robot {
    fn from(value: RobotImpl<CpcecUsedEmulator>) -> Self {
        Self::Cpcec(value)
    }
}

impl From<RobotImpl<WinapeUsedEmulator>> for Robot {
    fn from(value: RobotImpl<WinapeUsedEmulator>) -> Self {
        Self::Winape(value)
    }
}

impl From<RobotImpl<AmspiritUsedEmulator>> for Robot {
    fn from(value: RobotImpl<AmspiritUsedEmulator>) -> Self {
        Self::Amspirit(value)
    }
}

impl From<RobotImpl<SugarBoxV2UsedEmulator>> for Robot {
    fn from(value: RobotImpl<SugarBoxV2UsedEmulator>) -> Self {
        Self::SugarboxV2(value)
    }
}

impl From<RobotImpl<CpcEmuPowerUsedEmulator>> for Robot {
    fn from(value: RobotImpl<CpcEmuPowerUsedEmulator>) -> Self {
        Self::CpcEmuPower(value)
    }
}

impl From<RobotImpl<CapriceForeverUsedEmulator>> for Robot {
    fn from(value: RobotImpl<CapriceForeverUsedEmulator>) -> Self {
        Self::CapriceForever(value)
    }
}

impl<E: UsedEmulator> From<(EmuWindow, WindowEventsManager, &Emulator)> for RobotImpl<E> {
    fn from(value: (EmuWindow, WindowEventsManager, &Emulator)) -> Self {
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
        match self {
            OrgamsRobotAction::LoadOrImportAndEdit { .. } => true,
            _ => false
        }
    }

    pub fn jump(&self) -> bool {
        match self {
            OrgamsRobotAction::LoadOrImportAndAssembleJump { .. } => true,
            _ => false
        }
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
            Robot::CapriceForever(r) => r,
        } {
            fn handle_orgams(
                &mut self,
                drivea: Option<&str>,
                albireo: Option<&str>,
                action: OrgamsRobotAction<'_, '_>
            ) -> Result<(), String>;
            fn type_text(&mut self, s: &str);
            fn close(&mut self);
        }

    }

    pub fn new(emu: &Emulator, window: EmuWindow, eventsManager: WindowEventsManager) -> Self {
        match emu {
            Emulator::Ace(_) => {
                RobotImpl::<AceUsedEmulator>::from((window, eventsManager, emu)).into()
            },
            Emulator::Cpcec(_) => {
                RobotImpl::<CpcecUsedEmulator>::from((window, eventsManager, emu)).into()
            },
            Emulator::Winape(_) => {
                RobotImpl::<WinapeUsedEmulator>::from((window, eventsManager, emu)).into()
            },
            Emulator::Amspirit(_) => {
                RobotImpl::<AmspiritUsedEmulator>::from((window, eventsManager, emu)).into()
            },
            Emulator::SugarBoxV2(_) => {
                RobotImpl::<SugarBoxV2UsedEmulator>::from((window, eventsManager, emu)).into()
            },
            Emulator::CpcEmuPower(_) => {
                RobotImpl::<CpcEmuPowerUsedEmulator>::from((window, eventsManager, emu)).into()
            },
            Emulator::CapriceForever(_caprice_forever_version) => {
                RobotImpl::<CapriceForeverUsedEmulator>::from((window, eventsManager, emu)).into()
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
    pub fn screenshot(&mut self) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
        E::screenshot(self)
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
    pub fn handle_orgams(
        &mut self,
        drivea: Option<&str>,
        albireo: Option<&str>,
        action: OrgamsRobotAction<'_, '_>
    ) -> Result<(), String> {
        // we assume that we do not need to select the window as well launched it. it is already selected

        self.unidos_select_drive(drivea, albireo);
        let mut res;

        // Handle file loading
        let src = action.src();
        res = if src.ends_with('o') || src.ends_with('O') {
            // here we directly load an orgams file
            self.orgams_load(src)
        }
        else {
            // here we need to import
            self.orgams_import(src)
        }
        .map_err(|screen| (format!("Error while loading {src}"), screen));

        // if file has been loaded, handle next action
        if res.is_ok() {
            // need to update res
            res = {
                if action.edit() {
                    dbg!("edit requested");
                    // No need to do more when we want to edit a file
                    Ok(())
                }
                else if let Some(dst) = dbg!(action.save_orgams_binary_source()) {
                    self.orgams_save_source(dst)
                        .map_err(|screen| ("Error while saving sources".to_string(), screen))
                }
                else if let Some(dst) = dbg!(action.save_orgams_ascii_source()) {
                    dbg!("export file");

                    self.orgams_export_source(dst).map_err(|screen| {
                        (format!("Error while exporting source to {dst}"), screen)
                    })
                }
                else {
                    dbg!("Assemble file");
                    // we want to assemble the file
                    self.orgams_assemble(src)
                        .map_err(|screen| ("Error while assembling".to_string(), screen))
                        .and_then(|_| {
                            if action.jump() {
                                self.orgams_jump()
                                    .map_err(|screen| ("Error while jumping".to_string(), screen))
                            }
                            else {
                                self.orgams_save(action.dst()).map_err(|screen| {
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

    fn orgams_jump(&mut self) -> Result<(), Screenshot> {
        self.type_char('j');
        Ok(())
    }

    fn orgams_wait_import(&mut self) -> Result<(), Screenshot> {
        self.orgams_wait_save()
    }

    fn orgams_wait_save(&mut self) -> Result<(), Screenshot> {
        loop {
            let screen = self.screenshot();
            let coord_of_interest = (0, 48);
            let pix_of_interest = screen.get_pixel(coord_of_interest.0, coord_of_interest.1);

            if !(pix_of_interest == &Rgba([1, 1, 1, 255])
                || pix_of_interest == &Rgba([1, 2, 1, 255]))
            {
                println!("  done.");
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

    fn orgams_save_source(&mut self, dst: &str) -> Result<(), Screenshot> {
        dbg!("Tentative to save {dst}");
        self.ctrl_char('s');
        self.type_text(dst);
        self.r#return();

        std::thread::sleep(Duration::from_millis(3000 / 2)); // we consider it takes at minimum to assemble a file
        self.orgams_wait_save()
    }

    fn orgams_export_source(&mut self, dst: &str) -> Result<(), Screenshot> {
        dbg!("Tentative to export {dst}");
        self.ctrl_char('e');
        self.type_text(dst);
        self.r#return();

        std::thread::sleep(Duration::from_millis(1000 / 2));
        self.type_char('W');

        std::thread::sleep(Duration::from_millis(3000 / 2)); // we consider it takes at minimum to assemble a file
        self.orgams_wait_save()
    }

    fn orgams_save(&mut self, dst: Option<&str>) -> Result<(), Screenshot> {
        println!("> Save result");
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
        println!("  Filename provided.");

        // wait save is done
        std::thread::sleep(Duration::from_millis(3000 / 2)); // we consider it takes at minimum to assemble a file
        self.orgams_wait_save()
    }

    fn orgams_import(&mut self, src: &str) -> Result<(), ImageBuffer<Rgba<u8>, Vec<u8>>> {
        self.type_text("ùo");
        self.r#return();

        std::thread::sleep(Duration::from_secs(1)); // we wait one second for orgams loading

        self.ctrl_char('i');
        std::thread::sleep(Duration::from_secs(1)); // we wait one second for orgams loading

        self.type_text(src);
        self.r#return();

        self.orgams_wait_import()
    }

    fn orgams_load(&mut self, src: &str) -> Result<(), ImageBuffer<Rgba<u8>, Vec<u8>>> {
        // Open orgams
        println!("> Launch orgams and open file \"{src}\"");

        // French setup ?
        let chars = "ùo,\"".to_owned() + src + "\"";
        self.type_text(chars.as_str());
        self.r#return();

        let res = self.wait_orgams_loading();
        println!("  done.");

        res
    }

    fn orgams_assemble(&mut self, src: &str) -> Result<(), ImageBuffer<Rgba<u8>, Vec<u8>>> {
        println!("> Assemble {src}");
        self.ctrl_char('1');

        self.wait_orgams_assembling();
        println!("  done.");

        let result: ImageBuffer<Rgba<u8>, Vec<u8>> = self.window.capture_image();

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

    #[arg(short, long, value_parser = clap::builder::PossibleValuesParser::new(&["64", "128", "192", "256", "320", "576", "1088", "2112"]), help="Memory configuration")]
    memory: Option<String>,

    #[arg(short, long, value_parser = value_parser!(Crtc), help="Choice of the CRTC [possible values: 0, 1, 2, 3, 4]")]
    crtc: Option<Crtc>,

    #[arg(short, long, default_value = "ace", alias = "emu")]
    emulator: Emu,

    #[arg(short, long, action = ArgAction::SetTrue, help = "Keep the emulator open after the interaction")]
    keepemulator: bool,

    #[arg(short='C', long, action = ArgAction::SetTrue, help = "Clear the cache folder")]
    clear_cache: bool,

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

#[derive(ValueEnum, Clone, Debug, PartialEq)]
pub enum Emu {
    Ace,
    Winape,
    Cpcec,
    Amspirit,
    Sugarbox,
    Cpcemupower,
    Caprice
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
    Orgams(OrgamsCli),

    Run {
        #[arg(short, long, help = "Simple text to type")]
        text: Option<String>
    }
}

pub const EMUCTRL_CMD: &str = "cpc";

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

impl<E: EventObserver> Runner for EmulatorFacadeRunner<E> {
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

impl<E: EventObserver + 'static> RunnerWithClap for EmulatorFacadeRunner<E> {
    fn get_clap_command(&self) -> &clap::Command {
        &self.command
    }
}

pub fn handle_arguments<E: EventObserver>(mut cli: EmuCli, o: &E) -> Result<(), String> {
    dbg!(&cli);

    if cli.clear_cache {
        clear_base_cache_folder().map_err(|e| format!("Unable to clear the cache folder. {e}"))?;
    }

    let builder = EmulatorConf::builder()
        .transparent(cli.transparent)
        .maybe_drive_a(cli.drive_a.clone().map(|a| a.into()))
        .maybe_drive_b(cli.drive_b.clone().map(|a| a.into()))
        .maybe_crtc(cli.crtc.map(|c| c.try_into().unwrap()))
        .maybe_snapshot(cli.snapshot.clone().map(|a| a.into()))
        .debug_files(cli.debug.clone())
        .maybe_auto_run(cli.auto_run_file.clone())
        .maybe_auto_type(cli.auto_type_file.clone())
        .maybe_memory(cli.memory.clone().map(|v| v.parse::<u32>().unwrap()))
        .break_on_bad_hbl(cli.break_on_bad_hbl)
        .break_on_bad_vbl(cli.break_on_bad_vbl);
    let conf = builder.build();

    let emu = match cli.emulator {
        Emu::Ace => Emulator::Ace(Default::default()),
        Emu::Winape => Emulator::Winape(Default::default()),
        Emu::Cpcec => Emulator::Cpcec(Default::default()),
        Emu::Caprice => Emulator::CapriceForever(Default::default()),
        Emu::Amspirit => Emulator::Amspirit(Default::default()),
        Emu::Sugarbox => Emulator::SugarBoxV2(Default::default()),
        Emu::Cpcemupower => Emulator::CpcEmuPower(Default::default())
    };

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

        let extra_roms: &[(AmstradRom, &[(&str, usize, Option<AceConfigFlag>)])] = &[
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
                    println!("Install {src} in {dst}");
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
    let _emu_thread = std::thread::spawn(move || {
        start_emulator(&t_emu, &conf_thread).expect("Error detected while closing the emulator")
    });

    if cli.albireo.is_some() {
        std::thread::sleep(Duration::from_secs(5));
    }
    else {
        std::thread::sleep(Duration::from_secs(3));
    }

    let window = get_emulator_window(&emu, &conf);
    let enigo_settings = {
        let mut settings = Settings::default();
        settings.linux_delay = 1000 / 10;
        if let EmuWindow::Xvfb(display, _) = &window {
            settings.x11_display = Some(format!(":{display}"));
            settings.x11_display = Some(format!("{display}"));
        }
        settings
    };
    let enigo = Enigo::new(&enigo_settings).unwrap();
    let events = enigo.into();
    let mut robot = Robot::new(&emu, window, events);

    #[cfg(windows)]
    std::thread::sleep(Duration::from_millis(1000 * 3));

    dbg!(&cli.command);
    let res = match cli.command {
        Commands::Orgams(OrgamsCli {
            src,
            dst,
            jump,
            edit,
            basm2orgamsa,
            orgamsa2orgamsb,
            orgamsb2orgamsa
        }) => {
            if basm2orgamsa {
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

                robot.handle_orgams(cli.drive_a.as_deref(), cli.albireo.as_deref(), action)
            }
        },

        Commands::Run { text } => {
            cli.keepemulator = true;

            if let Some(text) = text {
                robot.handle_raw_text(text);
            }

            Ok(())
        }
    };

    dbg!(&res);

    if !cli.keepemulator {
        robot.close();
    }

    #[allow(unused_variables)]
    if let Some((backup_folder, emu_folder)) = albireo_backup_and_original {
        if cli.keepemulator {
            eprintln!(
                "Albireo folder not cleaned automatically. you'll have to do it if necessary"
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
}
