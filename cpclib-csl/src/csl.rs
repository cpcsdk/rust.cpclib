//! CSL (CPC Script Language) token modeling
//!
//! This module models the tokens and instructions of the CSL language v1.1
//! as specified in the CSL-STANDARD_EN.pdf document.
//!
//! CSL is a scripting language that allows precise automation of emulator control,
//! simulating user actions. It uses a simple text format with one instruction per line.
//! Semicolons are used for comments.

use std::fmt;
use cpclib_common::camino::Utf8PathBuf;
use cpclib_common::itertools::Itertools;
/// CSL language version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CslVersion {
    pub major: u8,
    pub minor: u8
}

impl CslVersion {
    pub fn new(major: u8, minor: u8) -> Self {
        Self { major, minor }
    }
}

/// Reset type for the emulator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetType {
    /// Memory cleared by ROM, only 64K central RAM
    Soft,
    /// Power on/off, all components reset
    Hard
}

impl Default for ResetType {
    fn default() -> Self {
        Self::Hard
    }
}

impl fmt::Display for ResetType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Soft => write!(f, "S"),
            Self::Hard => write!(f, "H")
        }
    }
}

/// CRTC model selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrtcModel {
    Type0,
    Type1,
    Type1A, // Same as 1B
    Type1B, // Same as 1A
    Type2,
    Type3,
    Type4
}

impl fmt::Display for CrtcModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Type0 => write!(f, "0"),
            Self::Type1 => write!(f, "1"),
            Self::Type1A => write!(f, "1A"),
            Self::Type1B => write!(f, "1B"),
            Self::Type2 => write!(f, "2"),
            Self::Type3 => write!(f, "3"),
            Self::Type4 => write!(f, "4")
        }
    }
}

/// Gate Array model
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateArrayModel {
    Model40007,
    Model40008,
    Model40010
}

impl fmt::Display for GateArrayModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Model40007 => write!(f, "40007"),
            Self::Model40008 => write!(f, "40008"),
            Self::Model40010 => write!(f, "40010")
        }
    }
}

/// CPC model selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpcModel {
    Cpc464,   // 0
    Cpc664,   // 1
    Cpc6128,  // 2
    Cpc6128Plus, // 4
    Cpc464Plus,  // 5
    GX4000    // 6
}

impl fmt::Display for CpcModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cpc464 => write!(f, "0"),
            Self::Cpc664 => write!(f, "1"),
            Self::Cpc6128 => write!(f, "2"),
            Self::Cpc6128Plus => write!(f, "4"),
            Self::Cpc464Plus => write!(f, "5"),
            Self::GX4000 => write!(f, "6")
        }
    }
}

/// Memory expansion configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryExpansion {
    /// 128k (C4..C7)
    Kb128,
    /// 256k (C4..DF)
    Kb256Standard,
    /// 256k (silicon E4FF)
    Kb256Silicon,
    /// 4M expansion
    Mb4,
    /// 512k (DK Tronics)
    Kb512DkTronics
}

impl fmt::Display for MemoryExpansion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Kb128 => write!(f, "128"),
            Self::Kb256Standard => write!(f, "256"),
            Self::Kb256Silicon => write!(f, "256S"),
            Self::Mb4 => write!(f, "4M"),
            Self::Kb512DkTronics => write!(f, "512")
        }
    }
}

/// ROM type configuration
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RomType {
    /// Upper ROM
    Upper,
    /// Lower ROM
    Lower,
    /// Cartridge (set of ROMs)
    Cartridge,
    /// Multiface 2
    Multiface2
}

impl fmt::Display for RomType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Upper => write!(f, "U"),
            Self::Lower => write!(f, "L"),
            Self::Cartridge => write!(f, "C"),
            Self::Multiface2 => write!(f, "M")
        }
    }
}

/// ROM configuration
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RomConfig {
    pub rom_type: RomType,
    pub num: u8,
    pub filename: Utf8PathBuf
}

/// Drive selection (A or B)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Drive {
    A,
    B
}

impl Default for Drive {
    fn default() -> Self {
        Self::A
    }
}

impl fmt::Display for Drive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::A => write!(f, "A"),
            Self::B => write!(f, "B")
        }
    }
}

/// Snapshot version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotVersion {
    V1,
    V2,
    V3
}

impl fmt::Display for SnapshotVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::V1 => write!(f, "1"),
            Self::V2 => write!(f, "2"),
            Self::V3 => write!(f, "3")
        }
    }
}

/// Special key codes for key_output
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialKey {
    Esc,        // \(ESC)
    Tab,        // \(TAB)
    CapsLock,   // \(CAP)
    Shift,      // \(SHI)
    Ctrl,       // \(CTR)
    Copy,       // \(COP)
    Clr,        // \(CLR)
    Del,        // \(DEL)
    Return,     // \(RET)
    Enter,      // \(ENT)
    ArrowLeft,  // \(ARL)
    ArrowRight, // \(ARR)
    ArrowUp,    // \(ARU)
    ArrowDown,  // \(ARD)
    F0,         // \(FN0)
    F1,         // \(FN1)
    F2,         // \(FN2)
    F3,         // \(FN3)
    F4,         // \(FN4)
    F5,         // \(FN5)
    F6,         // \(FN6)
    F7,         // \(FN7)
    F8,         // \(FN8)
    F9,         // \(FN9)
    LeftBrace,  // \({)
    RightBrace, // \(})
    Backslash,  // \(\)
    Quote,      // \(')
    NoDelayNextKey // \(KOF) - No delay for next key
}

impl SpecialKey {
    /// Get the CSL escape sequence for this key
    pub fn escape_sequence(&self) -> &'static str {
        match self {
            Self::Esc => r"\(ESC)",
            Self::Tab => r"\(TAB)",
            Self::CapsLock => r"\(CAP)",
            Self::Shift => r"\(SHI)",
            Self::Ctrl => r"\(CTR)",
            Self::Copy => r"\(COP)",
            Self::Clr => r"\(CLR)",
            Self::Del => r"\(DEL)",
            Self::Return => r"\(RET)",
            Self::Enter => r"\(ENT)",
            Self::ArrowLeft => r"\(ARL)",
            Self::ArrowRight => r"\(ARR)",
            Self::ArrowUp => r"\(ARU)",
            Self::ArrowDown => r"\(ARD)",
            Self::F0 => r"\(FN0)",
            Self::F1 => r"\(FN1)",
            Self::F2 => r"\(FN2)",
            Self::F3 => r"\(FN3)",
            Self::F4 => r"\(FN4)",
            Self::F5 => r"\(FN5)",
            Self::F6 => r"\(FN6)",
            Self::F7 => r"\(FN7)",
            Self::F8 => r"\(FN8)",
            Self::F9 => r"\(FN9)",
            Self::LeftBrace => r"\({)",
            Self::RightBrace => r"\(})",
            Self::Backslash => r"\(\)",
            Self::Quote => r"\(')",
            Self::NoDelayNextKey => r"\(KOF)"
        }
    }
}

/// Key output element (character or special key)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyElement {
    Character(char),
    Special(SpecialKey),
    /// Simultaneous key group: {abcd}
    Simultaneous(Vec<KeyElement>)
}

impl fmt::Display for KeyElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Character(c) => write!(f, "{}", c),
            Self::Special(key) => write!(f, "{}", key.escape_sequence()),
            Self::Simultaneous(elements) => {
                write!(f, "{{") ?;
                for elem in elements {
                    write!(f, "{}", elem)?;
                }
                write!(f, "}}")
            }
        }
    }
}
/// Check if a string is already pre-escaped (wrapped in single quotes)
/// This indicates the string already contains proper CSL escape sequences.
fn is_pre_escaped(s: &str) -> bool {
    s.starts_with('\'') && s.ends_with('\'')
}

/// Key output text - newtype wrapper for key output elements
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyOutput(pub Vec<KeyElement>);

impl KeyOutput {
    /// Create a new empty KeyOutput
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Create a KeyOutput from a vector of elements
    pub fn from_elements(elements: Vec<KeyElement>) -> Self {
        Self(elements)
    }

    /// Get a reference to the inner elements
    pub fn elements(&self) -> &[KeyElement] {
        &self.0
    }
}

impl Default for KeyOutput {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for KeyOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for elem in &self.0 {
            write!(f, "{}", elem)?;
        }
        Ok(())
    }
}

impl TryFrom<&str> for KeyOutput {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        use crate::csl_parser::parse_key_output_content;
        use cpclib_common::winnow::Parser;

		// replace newlines by \RET
		let s = s.split('\n').join(r"\(RET)");

        // Wrap the input with quotes as the parser expects quoted strings
        let quoted = if is_pre_escaped(&s) {
			// here we assume everything is already escaped properly
            s
        } else {
			// Do escaping ourselves
			// Need to escape: ', {, }, and \ (but not when \ is followed by ()
			let mut escaped = String::new();
			let chars: Vec<char> = s.chars().collect();
			let mut i = 0;
			while i < chars.len() {
				match chars[i] {
					'\'' => escaped.push_str(r"\(')"),
					'{' => escaped.push_str(r"\({)"),
					'}' => escaped.push_str(r"\(})"),
					'\\' => {
						// Only escape backslash if NOT followed by (
						if i + 1 < chars.len() && chars[i + 1] == '(' {
							escaped.push('\\');
						} else {
							escaped.push_str(r"\(\)");
						}
					}
					c => escaped.push(c),
				}
				i += 1;
			}
            format!("'{}'", escaped)
        };
        let mut input = cpclib_common::winnow::stream::LocatingSlice::new(quoted.as_str());
        
        match parse_key_output_content.parse_next(&mut input) {
            Ok(key_output) => Ok(key_output),
            Err(e) => Err(format!("Failed to parse key output: {:?}", e))
        }
    }
}
/// All CSL instructions as defined in the specification
#[derive(Debug, Clone, PartialEq)]
pub enum CslInstruction {
    // Miscellaneous
    /// Indicates the version of the CSL format
    CslVersion(CslVersion),
    
    /// Reset the emulator (soft or hard)
    Reset(ResetType),

    // Machine configuration
    /// Select a CRTC model
    CrtcSelect(CrtcModel),
    
    /// Select a Gate Array model (v1.1)
    GateArray(GateArrayModel),
    
    /// Select a CPC model (v1.1)
    CpcModel(CpcModel),
    
    /// Select memory expansion (v1.1)
    MemoryExp(MemoryExpansion),
    
    /// Specify ROM directory (v1.1)
    RomDir(Utf8PathBuf),
    
    /// Configure a ROM (v1.1)
    RomConfig(RomConfig),

    // Media
    /// Insert a disk file into a drive
    DiskInsert { drive: Drive, filename: Utf8PathBuf },
    
    /// Specify disk directory
    DiskDir(Utf8PathBuf),
    
    /// Insert a tape file
    TapeInsert(Utf8PathBuf),
    
    /// Specify tape directory
    TapeDir(Utf8PathBuf),
    
    /// Start tape playback
    TapePlay,
    
    /// Stop tape playback
    TapeStop,
    
    /// Rewind tape to beginning
    TapeRewind,
    
    /// Load a snapshot file
    SnapshotLoad(Utf8PathBuf),
    
    /// Specify snapshot directory
    SnapshotDir(Utf8PathBuf),

    // Key strokes
    /// Set delay between keystrokes (in microseconds)
    /// First param: delay between keys, Second param (optional): delay after CR, Third param (optional): delay after special key
    KeyDelay { delay: u64, delay_after_cr: Option<u64>, delay_after_key: Option<u64> },
    
    /// Send text as key strokes
    KeyOutput(KeyOutput),
    
    /// Send characters from file as key strokes
    KeyFromFile(Utf8PathBuf),

    // Meta instruction
    /// An instruction followed by an inline comment on the same line
    InstructionWithComment(Box<CslInstruction>, String),

    // Synchronization
    /// Wait for a delay in microseconds (emulated time)
    Wait(u64),
    
    /// Wait for drive motor to start and stop N times
    WaitDriveOnOff(u32),
    
    /// Wait for vsync signal to switch from off to on
    WaitVsyncOffOn,
    
    /// Wait for SSM Code 0000 (ED 00 ED 00)
    WaitSsm0000,

    // Exports
    /// Specify name for next screenshot (without extension)
    ScreenshotName(Utf8PathBuf),
    
    /// Specify screenshot directory
    ScreenshotDir(Utf8PathBuf),
    
    /// Take a screenshot (optionally wait for vsync)
    Screenshot { wait_vsync: bool },
    
    /// Specify name for next snapshot (without extension)
    SnapshotName(Utf8PathBuf),
    
    /// Take a snapshot (optionally wait for vsync)
    Snapshot { wait_vsync: bool },
    
    /// Select snapshot version
    SnapshotVersion(SnapshotVersion),
    
    /// Load and run another CSL file
    CslLoad(Utf8PathBuf),

    // Comments (for completeness)
    Comment(String),
    
    // Empty line
    Empty
}

/// Helper function to normalize paths for CSL output
/// On Linux, paths with Z: drive have their forward slashes replaced with backslashes
#[cfg(target_os = "linux")]
fn normalize_path_for_csl(path: &Utf8PathBuf, is_dir:bool) -> String {

    dbg!(&path);

    let path = if is_dir && !path.as_str().starts_with('/') {
        format!("{}/{}", std::env::current_dir().unwrap().to_str().unwrap(), path.as_str())
    } else if !is_dir && path.as_str().starts_with('/') {
        format!("{}/{}", std::env::current_dir().unwrap().to_str().unwrap(), path.as_str())
    } else {
        path.as_str().to_string()   
    };

    
    let path_str = path.as_str().replace('/', "\\");

    dbg!(&path_str);

    let path = if (path_str.len() >= 2 && !path_str[0..2].eq_ignore_ascii_case("z:") && is_dir) || path.starts_with('\\') {
        format!("Z:{}", path_str)
    } else {
        path_str
    };

    if is_dir {
        normalize_path_for_csl_windows(path)
    } else {
        path
    }
}

/// Helper function to normalize paths for CSL output
/// On non-Linux, paths are returned as-is
#[cfg(not(target_os = "linux"))]
fn normalize_path_for_csl(path: &Utf8PathBuf, is_dir:bool) -> String {
    let path = path.as_str().to_string();
    if is_dir {
        normalize_path_for_csl_windows(path)
    } else {
        path
    }
}
fn normalize_path_for_csl_windows(path: String) -> String {
    if !path.ends_with("\\") {
        format!("{}\\", path.as_str())
    } else {
        path
    }
}

impl fmt::Display for CslInstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CslVersion(v) => write!(f, "csl_version {}.{}", v.major, v.minor),
            Self::Reset(reset_type) => write!(f, "reset {}", reset_type),
            Self::CrtcSelect(model) => write!(f, "crtc_select {}", model),
            Self::GateArray(model) => write!(f, "gate_array {}", model),
            Self::CpcModel(model) => write!(f, "cpc_model {}", model),
            Self::MemoryExp(exp) => write!(f, "memory_exp {}", exp),
            Self::RomDir(dir) => write!(f, "rom_dir '{}'", normalize_path_for_csl(dir, true)),
            Self::RomConfig(config) => write!(f, "rom_config {} {} '{}'", config.rom_type, config.num, normalize_path_for_csl(&config.filename, false)),
            Self::DiskInsert { drive, filename } => write!(f, "disk_insert {} '{}'", drive, normalize_path_for_csl(filename, false)),
            Self::DiskDir(dir) => write!(f, "disk_dir '{}'", normalize_path_for_csl(dir, true)),
            Self::TapeInsert(file) => write!(f, "tape_insert '{}'", normalize_path_for_csl(file, false)),
            Self::TapeDir(dir) => write!(f, "tape_dir '{}'", normalize_path_for_csl(dir, true)),
            Self::TapePlay => write!(f, "tape_play"),
            Self::TapeStop => write!(f, "tape_stop"),
            Self::TapeRewind => write!(f, "tape_rewind"),
            Self::SnapshotLoad(file) => write!(f, "snapshot_load '{}'", normalize_path_for_csl(file, false)),
            Self::SnapshotDir(dir) => write!(f, "snapshot_dir '{}'", normalize_path_for_csl(dir, true)),
            Self::KeyDelay { delay, delay_after_cr, delay_after_key } => {
                write!(f, "key_delay {}", delay)?;
                if let Some(cr) = delay_after_cr {
                    write!(f, " {}", cr)?;
                    if let Some(key) = delay_after_key {
                        write!(f, " {}", key)?;
                    }
                }
                Ok(())
            },
            Self::KeyOutput(key_output) => {
                write!(f, "key_output '{}'" , key_output)
            },
            Self::KeyFromFile(file) => write!(f, "key_from_file '{}'", normalize_path_for_csl(file, false)),
            Self::InstructionWithComment(instruction, comment) => write!(f, "{} ;{}", instruction, comment),
            Self::Wait(time) => write!(f, "wait {}", time),
            Self::WaitDriveOnOff(n) => write!(f, "wait_driveonoff {}", n),
            Self::WaitVsyncOffOn => write!(f, "wait_vsyncoffon"),
            Self::WaitSsm0000 => write!(f, "wait_ssm0000"),
            Self::ScreenshotName(name) => write!(f, "screenshot_name '{}'", name),
            Self::ScreenshotDir(dir) => write!(f, "screenshot_dir '{}'", normalize_path_for_csl(dir, true)),
            Self::Screenshot { wait_vsync } => {
                if *wait_vsync {
                    write!(f, "screenshot V")
                } else {
                    write!(f, "screenshot")
                }
            },
            Self::SnapshotName(name) => write!(f, "snapshot_name '{}'", name),
            Self::Snapshot { wait_vsync } => {
                if *wait_vsync {
                    write!(f, "snapshot V")
                } else {
                    write!(f, "snapshot")
                }
            },
            Self::SnapshotVersion(v) => write!(f, "snapshot_version {}", v),
            Self::CslLoad(file) => write!(f, "csl_load '{}'", normalize_path_for_csl(file, false)),
            Self::Comment(text) => write!(f, ";{}", text),
            Self::Empty => write!(f, "")
        }
    }
}

impl CslInstruction {
    /// Check if this instruction is a v1.1 feature
    pub fn is_v1_1_feature(&self) -> bool {
        matches!(
            self,
            Self::GateArray(_)
                | Self::CpcModel(_)
                | Self::MemoryExp(_)
                | Self::RomDir(_)
                | Self::RomConfig(_)
        )
    }

    /// Get the instruction name as it appears in CSL files
    pub fn instruction_name(&self) -> &'static str {
        match self {
            Self::CslVersion(_) => "csl_version",
            Self::Reset(_) => "reset",
            Self::CrtcSelect(_) => "crtc_select",
            Self::GateArray(_) => "gate_array",
            Self::CpcModel(_) => "cpc_model",
            Self::MemoryExp(_) => "memory_exp",
            Self::RomDir(_) => "rom_dir",
            Self::RomConfig(_) => "rom_config",
            Self::DiskInsert { .. } => "disk_insert",
            Self::DiskDir(_) => "disk_dir",
            Self::TapeInsert(_) => "tape_insert",
            Self::TapeDir(_) => "tape_dir",
            Self::TapePlay => "tape_play",
            Self::TapeStop => "tape_stop",
            Self::TapeRewind => "tape_rewind",
            Self::SnapshotLoad(_) => "snapshot_load",
            Self::SnapshotDir(_) => "snapshot_dir",
            Self::InstructionWithComment(instruction, _) => instruction.instruction_name(),
            Self::KeyDelay { .. } => "key_delay",
            Self::KeyOutput(_) => "key_output",
            Self::KeyFromFile(_) => "key_from_file",
            Self::Wait(_) => "wait",
            Self::WaitDriveOnOff(_) => "wait_driveonoff",
            Self::WaitVsyncOffOn => "wait_vsyncoffon",
            Self::WaitSsm0000 => "wait_ssm0000",
            Self::ScreenshotName(_) => "screenshot_name",
            Self::ScreenshotDir(_) => "screenshot_dir",
            Self::Screenshot { .. } => "screenshot",
            Self::SnapshotName(_) => "snapshot_name",
            Self::Snapshot { .. } => "snapshot",
            Self::SnapshotVersion(_) => "snapshot_version",
            Self::CslLoad(_) => "csl_load",
            Self::Comment(_) => "comment",
            Self::Empty => "empty"
        }
    }
}

impl CslInstruction {
    // Builder pattern factory methods for common instructions

    /// Create a DiskDir instruction
    pub fn disk_dir(dir: Utf8PathBuf) -> Self {
        Self::DiskDir(dir)
    }

    /// Create a DiskInsert instruction
    pub fn disk_insert(drive: Drive, filename: Utf8PathBuf) -> Self {
        Self::DiskInsert { drive, filename }
    }

    /// Create a TapeDir instruction
    pub fn tape_dir(dir: Utf8PathBuf) -> Self {
        Self::TapeDir(dir)
    }

    /// Create a TapeInsert instruction
    pub fn tape_insert(filename: Utf8PathBuf) -> Self {
        Self::TapeInsert(filename)
    }

    /// Create a SnapshotDir instruction
    pub fn snapshot_dir(dir: Utf8PathBuf) -> Self {
        Self::SnapshotDir(dir)
    }

    /// Create a SnapshotLoad instruction
    pub fn snapshot_load(filename: Utf8PathBuf) -> Self {
        Self::SnapshotLoad(filename)
    }

    /// Create a SnapshotName instruction
    pub fn snapshot_name(name: Utf8PathBuf) -> Self {
        Self::SnapshotName(name)
    }

    /// Create a Snapshot instruction
    pub fn snapshot(wait_vsync: bool) -> Self {
        Self::Snapshot { wait_vsync }
    }

    /// Create a KeyOutput instruction
    pub fn key_output(output: KeyOutput) -> Self {
        Self::KeyOutput(output)
    }

    /// Create a KeyFromFile instruction
    /// The filename is canonicalized to an absolute path
    pub fn key_from_file(filename: Utf8PathBuf) -> Self {
        let filename = if !filename.is_absolute() {
            let filename = filename.canonicalize()
                .map(|p| Utf8PathBuf::from_path_buf(p).unwrap_or(filename.clone()))
                .unwrap_or(filename);
            filename.as_str().strip_prefix(r"\\?\").unwrap_or(filename.as_str()).into()
        } else {
            filename
        };
        Self::KeyFromFile(filename)
    }

    /// Create a KeyDelay instruction
    pub fn key_delay(delay: u64, delay_after_cr: Option<u64>, delay_after_key: Option<u64>) -> Self {
        Self::KeyDelay { delay, delay_after_cr, delay_after_key }
    }

    /// Create a MemoryExp instruction
    pub fn memory_exp(expansion: MemoryExpansion) -> Self {
        Self::MemoryExp(expansion)
    }

    /// Create a CrtcSelect instruction
    pub fn crtc_select(model: CrtcModel) -> Self {
        Self::CrtcSelect(model)
    }

    /// Create a GateArray instruction
    pub fn gate_array(model: GateArrayModel) -> Self {
        Self::GateArray(model)
    }

    /// Create a CpcModel instruction
    pub fn cpc_model(model: CpcModel) -> Self {
        Self::CpcModel(model)
    }

    /// Create a Reset instruction
    pub fn reset(reset_type: ResetType) -> Self {
        Self::Reset(reset_type)
    }

    /// Create a Wait instruction
    pub fn wait(time: u64) -> Self {
        Self::Wait(time)
    }

    /// Create a WaitDriveOnOff instruction
    pub fn wait_drive_on_off(count: u32) -> Self {
        Self::WaitDriveOnOff(count)
    }

    /// Create a ScreenshotName instruction
    pub fn screenshot_name(name: Utf8PathBuf) -> Self {
        Self::ScreenshotName(name)
    }

    /// Create a ScreenshotDir instruction
    pub fn screenshot_dir(dir: Utf8PathBuf) -> Self {
        Self::ScreenshotDir(dir)
    }

    /// Create a Screenshot instruction
    pub fn screenshot(wait_vsync: bool) -> Self {
        Self::Screenshot { wait_vsync }
    }

    /// Create a RomDir instruction
    pub fn rom_dir(dir: Utf8PathBuf) -> Self {
        Self::RomDir(dir)
    }

    /// Create a RomConfig instruction
    pub fn rom_config(rom_type: RomType, num: u8, filename: Utf8PathBuf) -> Self {
        Self::RomConfig(RomConfig { rom_type, num, filename })
    }

    /// Create a CslLoad instruction
    pub fn csl_load(filename: Utf8PathBuf) -> Self {
        Self::CslLoad(filename)
    }

    /// Create a CslVersion instruction
    pub fn csl_version(major: u8, minor: u8) -> Self {
        Self::CslVersion(CslVersion { major, minor })
    }

    /// Create a Comment instruction
    pub fn comment<S: Into<String>>(text: S) -> Self {
        Self::Comment(text.into())
    }

    /// Create an Empty instruction
    pub fn empty() -> Self {
        Self::Empty
    }
}

/// CSL script representation
#[derive(Debug, Clone, PartialEq)]
pub struct CslScript {
    pub instructions: Vec<CslInstruction>
}

impl CslScript {
    /// Create a new empty CSL script
    pub fn new() -> Self {
        Self {
            instructions: Vec::new()
        }
    }

    /// Add an instruction to the script
    pub fn add_instruction(&mut self, instruction: CslInstruction) {
        self.instructions.push(instruction);
    }

    /// Get the CSL version if specified in the script
    pub fn get_version(&self) -> Option<CslVersion> {
        self.instructions.iter().find_map(|i| {
            if let CslInstruction::CslVersion(v) = i {
                Some(*v)
            } else {
                None
            }
        })
    }

    // Builder pattern methods for fluent script construction

    /// Add an instruction and return self for chaining (builder pattern)
    pub fn with_instruction(mut self, instruction: CslInstruction) -> Self {
        self.instructions.push(instruction);
        self
    }

    /// Conditionally add an instruction (builder pattern)
    pub fn with_instruction_if<F>(mut self, condition: bool, f: F) -> Self
    where
        F: FnOnce() -> CslInstruction
    {
        if condition {
            self.instructions.push(f());
        }
        self
    }

    /// Add a disk directory instruction
    pub fn with_disk_dir(self, dir: Utf8PathBuf) -> Self {
        self.with_instruction(CslInstruction::DiskDir(dir))
    }

    /// Add a disk insert instruction
    pub fn with_disk_insert(self, drive: Drive, filename: Utf8PathBuf) -> Self {
        self.with_instruction(CslInstruction::DiskInsert { drive, filename })
    }

    /// Add a snapshot directory instruction
    pub fn with_snapshot_dir(self, dir: Utf8PathBuf) -> Self {
        self.with_instruction(CslInstruction::SnapshotDir(dir))
    }

    /// Add a snapshot load instruction
    pub fn with_snapshot_load(self, filename: Utf8PathBuf) -> Self {
        self.with_instruction(CslInstruction::SnapshotLoad(filename))
    }

    /// Add a key output instruction
    pub fn with_key_output(self, output: KeyOutput) -> Self {
        self.with_instruction(CslInstruction::KeyOutput(output))
    }

    /// Add a key from file instruction
    pub fn with_key_from_file(self, filename: Utf8PathBuf) -> Self {
        self.with_instruction(CslInstruction::KeyFromFile(filename))
    }

    /// Add a memory expansion instruction
    pub fn with_memory_exp(self, expansion: MemoryExpansion) -> Self {
        self.with_instruction(CslInstruction::MemoryExp(expansion))
    }

    /// Add a CRTC select instruction
    pub fn with_crtc_select(self, model: CrtcModel) -> Self {
        self.with_instruction(CslInstruction::CrtcSelect(model))
    }

    /// Add a reset instruction
    pub fn with_reset(self, reset_type: ResetType) -> Self {
        self.with_instruction(CslInstruction::Reset(reset_type))
    }

    /// Add a wait instruction
    pub fn with_wait(self, frames: u64) -> Self {
        self.with_instruction(CslInstruction::Wait(frames))
    }

    /// Ensure the script starts with a version instruction.
    /// If one exists elsewhere, it will be moved to the front.
    /// If none exists, a default version 1.0 will be added.
    pub fn ensure_version_first(mut self) -> Self {
        // Find and remove any existing version instruction
        let version = self.instructions.iter()
            .position(|inst| matches!(inst, CslInstruction::CslVersion(_)))
            .and_then(|pos| {
                if let CslInstruction::CslVersion(v) = self.instructions.remove(pos) {
                    Some(v)
                } else {
                    None
                }
            })
            .unwrap_or(CslVersion::new(1, 0));

        // Insert version at the beginning
        self.instructions.insert(0, CslInstruction::CslVersion(version));
        self
    }
}

impl Default for CslScript {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for CSL scripts that ensures version is always first
#[derive(Debug, Clone, PartialEq)]
pub struct CslScriptBuilder {
    script: CslScript
}

impl CslScriptBuilder {
    /// Create a new CSL script builder with the given version
    pub fn new(major: u8, minor: u8) -> Self {
        Self {
            script: CslScript {
                instructions: vec![CslInstruction::CslVersion(CslVersion::new(major, minor))]
            }
        }
    }

    /// Build the final CSL script, ensuring version is first
    /// Returns an error if the script is invalid
    pub fn build(mut self) -> Result<CslScript, String> {
        // Find and remove all version instructions, keeping the last one
        let mut last_version = None;
        self.script.instructions.retain(|inst| {
            if let CslInstruction::CslVersion(v) = inst {
                last_version = Some(v.clone());
                false // Remove this instruction
            } else {
                true // Keep non-version instructions
            }
        });

        // Use the last version found, or default to 1.0
        let version = last_version.unwrap_or(CslVersion::new(1, 0));

        // Insert version at the beginning
        self.script.instructions.insert(0, CslInstruction::CslVersion(version));
        
        // Validate that version is indeed first
        if self.script.instructions.is_empty() {
            return Err("CSL script is empty".to_string());
        }
        
        if !matches!(self.script.instructions[0], CslInstruction::CslVersion(_)) {
            return Err("CSL script must start with version instruction".to_string());
        }

        if let Some(CslInstruction::KeyFromFile(file)) = self.script.instructions.iter()
            .find(|inst| matches!(inst, CslInstruction::KeyFromFile(_))) {
                if !file.is_absolute() {
                    return Err("key_from_file instruction requires an absolute file path".to_string());
                }
            }
        Ok(self.script)
    }

    // Builder pattern wrapper methods that return CslScriptBuilder

    /// Add an instruction and return self for chaining
    pub fn with_instruction(mut self, instruction: CslInstruction) -> Self {
        self.script.instructions.push(instruction);
        self
    }

    /// Conditionally add an instruction
    pub fn with_instruction_if<F>(mut self, condition: bool, f: F) -> Self
    where
        F: FnOnce() -> CslInstruction
    {
        if condition {
            self.script.instructions.push(f());
        }
        self
    }

    /// Add a disk directory instruction
    pub fn with_disk_dir(self, dir: Utf8PathBuf) -> Self {
        self.with_instruction(CslInstruction::DiskDir(dir))
    }

    /// Add a disk insert instruction
    pub fn with_disk_insert(self, drive: Drive, filename: Utf8PathBuf) -> Self {
        self.with_instruction(CslInstruction::DiskInsert { drive, filename })
    }

    /// Add a snapshot directory instruction
    pub fn with_snapshot_dir(self, dir: Utf8PathBuf) -> Self {
        self.with_instruction(CslInstruction::SnapshotDir(dir))
    }

    /// Add a snapshot load instruction
    pub fn with_snapshot_load(self, filename: Utf8PathBuf) -> Self {
        self.with_instruction(CslInstruction::SnapshotLoad(filename))
    }

    /// Add a key output instruction
    pub fn with_key_output(self, output: KeyOutput) -> Self {
        self.with_instruction(CslInstruction::KeyOutput(output))
    }

    /// Add a key from file instruction
    pub fn with_key_from_file(self, filename: Utf8PathBuf) -> Self {
        self.with_instruction(CslInstruction::KeyFromFile(filename))
    }

    /// Add a memory expansion instruction
    pub fn with_memory_exp(self, expansion: MemoryExpansion) -> Self {
        self.with_instruction(CslInstruction::MemoryExp(expansion))
    }

    /// Add a CRTC select instruction
    pub fn with_crtc_select(self, model: CrtcModel) -> Self {
        self.with_instruction(CslInstruction::CrtcSelect(model))
    }

    /// Add a reset instruction
    pub fn with_reset(self, reset_type: ResetType) -> Self {
        self.with_instruction(CslInstruction::Reset(reset_type))
    }

    /// Add a wait instruction
    pub fn with_wait(self, frames: u64) -> Self {
        self.with_instruction(CslInstruction::Wait(frames))
    }
}

impl Default for CslScriptBuilder {
    fn default() -> Self {
        Self::new(1, 0)
    }
}

impl std::ops::Deref for CslScriptBuilder {
    type Target = CslScript;

    fn deref(&self) -> &Self::Target {
        &self.script
    }
}

impl std::ops::DerefMut for CslScriptBuilder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.script
    }
}

impl fmt::Display for CslScript {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Ensure version instruction is output first
        let version_inst = self.instructions.iter()
            .find(|inst| matches!(inst, CslInstruction::CslVersion(_)));
        
        if let Some(version) = version_inst {
            writeln!(f, "{}", version)?;
        }

        // Output all other instructions (excluding version)
        for instruction in &self.instructions {
            if !matches!(instruction, CslInstruction::CslVersion(_)) {
                writeln!(f, "{}", instruction)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_special_key_escape_sequences() {
        assert_eq!(SpecialKey::Esc.escape_sequence(), r"\(ESC)");
        assert_eq!(SpecialKey::Return.escape_sequence(), r"\(RET)");
        assert_eq!(SpecialKey::F5.escape_sequence(), r"\(FN5)");
    }

    #[test]
    fn test_instruction_names() {
        assert_eq!(CslInstruction::Reset(ResetType::Soft).instruction_name(), "reset");
        assert_eq!(CslInstruction::TapePlay.instruction_name(), "tape_play");
        assert_eq!(CslInstruction::WaitVsyncOffOn.instruction_name(), "wait_vsyncoffon");
    }

    #[test]
    fn test_v1_1_features() {
        assert!(CslInstruction::GateArray(GateArrayModel::Model40010).is_v1_1_feature());
        assert!(!CslInstruction::Reset(ResetType::Hard).is_v1_1_feature());
    }

    #[test]
    fn test_csl_version() {
        let version = CslVersion::new(1, 1);
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 1);
    }

    #[test]
    fn test_display_reset() {
        assert_eq!(CslInstruction::Reset(ResetType::Soft).to_string(), "reset S");
        assert_eq!(CslInstruction::Reset(ResetType::Hard).to_string(), "reset H");
    }

    #[test]
    fn test_display_wait() {
        assert_eq!(CslInstruction::Wait(1000).to_string(), "wait 1000");
    }

    #[test]
    fn test_display_key_delay() {
        assert_eq!(
            CslInstruction::KeyDelay { delay: 70000, delay_after_cr: Some(70000), delay_after_key: Some(400000) }.to_string(),
            "key_delay 70000 70000 400000"
        );
        assert_eq!(
            CslInstruction::KeyDelay { delay: 50000, delay_after_cr: None, delay_after_key: None }.to_string(),
            "key_delay 50000"
        );
    }

    #[test]
    fn test_display_instruction_with_comment() {
        let instruction = CslInstruction::InstructionWithComment(
            Box::new(CslInstruction::Wait(800000)),
            " fin affichage".to_string()
        );
        assert_eq!(instruction.to_string(), "wait 800000 ; fin affichage");
    }

    #[test]
    fn test_roundtrip_simple_instructions() {
        use crate::csl_parser::parse_csl;
        
        let test_cases = vec![
            "reset H",
            "reset S",
            "wait 1000000",
            "tape_play",
            "tape_stop",
            "wait_vsyncoffon",
            "crtc_select 1",
            "gate_array 40010",
            "cpc_model 2",
        ];

        for case in test_cases {
            let script1 = format!("{}\n", case);
            let parsed1 = parse_csl(&script1).expect(&format!("Failed to parse: {}", case));
            let generated = parsed1.to_string();
            let parsed2 = parse_csl(&generated).expect(&format!("Failed to parse generated: {}", generated));
            assert_eq!(parsed1, parsed2, "Roundtrip failed for: {}", case);
        }
    }

    #[test]
    fn test_builder_pattern() {
        use crate::{CslScript, CslInstruction, Drive, MemoryExpansion, CrtcModel, ResetType};

        // Test fluent builder pattern with helper methods
        let script = CslScript::new()
            .with_disk_dir("disks".into())
            .with_disk_insert(Drive::A, "game.dsk".into())
            .with_memory_exp(MemoryExpansion::Kb512DkTronics)
            .with_crtc_select(CrtcModel::Type1)
            .with_reset(ResetType::Hard)
            .with_wait(50);

        assert_eq!(script.instructions.len(), 6);
        
        // Test using CslInstruction factory methods
        let script2 = CslScript::new()
            .with_instruction(CslInstruction::disk_dir("disks".into()))
            .with_instruction(CslInstruction::disk_insert(Drive::A, "game.dsk".into()))
            .with_instruction(CslInstruction::snapshot_load("game.sna".into()))
            .with_instruction(CslInstruction::memory_exp(MemoryExpansion::Kb512DkTronics));

        assert_eq!(script2.instructions.len(), 4);
        
        // Test conditional builder with factory methods
        let with_snapshot = true;
        let script3 = CslScript::new()
            .with_disk_insert(Drive::A, "test.dsk".into())
            .with_instruction_if(with_snapshot, || {
                CslInstruction::snapshot_load("game.sna".into())
            });

        assert_eq!(script3.instructions.len(), 2);
        
        // Test without snapshot
        let without_snapshot = false;
        let script4 = CslScript::new()
            .with_disk_insert(Drive::A, "test.dsk".into())
            .with_instruction_if(without_snapshot, || {
                CslInstruction::snapshot_load("game.sna".into())
            });

        assert_eq!(script4.instructions.len(), 1);
    }

    #[test]
    fn test_roundtrip_with_parameters() {
        use crate::csl_parser::parse_csl;
        
        let test_cases = vec![
            "disk_insert A 'test.dsk'",
            "key_delay 70000 70000 400000",
            "snapshot_name 'mysnap'",
            "csl_load 'other.csl'",
            "rom_config U 7 'Amsdos.rom'",
            "key_output 'Hello\\(RET)'",
        ];

        for case in test_cases {
            let script1 = format!("{}\n", case);
            let parsed1 = parse_csl(&script1).expect(&format!("Failed to parse: {}", case));
            let generated = parsed1.to_string();
            let parsed2 = parse_csl(&generated).expect(&format!("Failed to parse generated: {}", generated));
            assert_eq!(parsed1, parsed2, "Roundtrip failed for: {}", case);
        }
    }

    #[test]
    fn test_roundtrip_with_comments() {
        use crate::csl_parser::parse_csl;
        
        let script1 = "wait 800000 ; fin affichage\n";
        let parsed1 = parse_csl(script1).expect("Failed to parse");
        let generated = parsed1.to_string();
        let parsed2 = parse_csl(&generated).expect("Failed to parse generated");
        assert_eq!(parsed1, parsed2, "Roundtrip failed with comments");
    }

    #[test]
    fn test_roundtrip_full_script() {
        use crate::csl_parser::parse_csl;
        
        let script = "csl_version 1.0\nreset H\nwait 1000000\ntape_play\nwait 500000\n";
        let parsed1 = parse_csl(script).expect("Failed to parse script");
        let generated = parsed1.to_string();
        let parsed2 = parse_csl(&generated).expect("Failed to parse generated script");
        assert_eq!(parsed1, parsed2, "Full script roundtrip failed");
    }

    #[test]
    fn test_key_output_try_from() {
        // Test simple text
        let result = KeyOutput::try_from("Hello");
        assert!(result.is_ok());
        let key_output = result.unwrap();
        assert_eq!(key_output.elements().len(), 5);
        assert_eq!(key_output.to_string(), "Hello");

        // Test with special key
        let result = KeyOutput::try_from("Test\\(RET)");
        assert!(result.is_ok());
        let key_output = result.unwrap();
        assert_eq!(key_output.elements().len(), 5); // T e s t \(RET)

        // Test empty string
        let result = KeyOutput::try_from("");
        assert!(result.is_ok());
        let key_output = result.unwrap();
        assert_eq!(key_output.elements().len(), 0);
    }

    #[test]
    fn test_key_output_display() {
        let key_output = KeyOutput::from_elements(vec![
            KeyElement::Character('H'),
            KeyElement::Character('i'),
            KeyElement::Special(SpecialKey::Return)
        ]);
        assert_eq!(key_output.to_string(), "Hi\\(RET)");
    }

    #[test]
    fn test_is_pre_escaped() {
        assert!(is_pre_escaped("'Hello'"));
        assert!(is_pre_escaped("'Test\\(RET)'"));
        assert!(is_pre_escaped("''"));
        assert!(!is_pre_escaped("Hello"));
        assert!(!is_pre_escaped("'Hello"));
        assert!(!is_pre_escaped("Hello'"));
        assert!(!is_pre_escaped(""));
    }

    #[test]
    fn test_version_is_first() {
        // Test ensure_version_first adds version if missing
        let script = CslScript::new()
            .with_reset(ResetType::Hard)
            .with_wait(1000)
            .ensure_version_first();

        assert_eq!(script.instructions.len(), 3);
        assert!(matches!(script.instructions[0], CslInstruction::CslVersion(_)));

        // Test ensure_version_first moves existing version to front
        let script = CslScript::new()
            .with_reset(ResetType::Hard)
            .with_instruction(CslInstruction::csl_version(1, 1))
            .with_wait(1000)
            .ensure_version_first();

        assert_eq!(script.instructions.len(), 3);
        assert!(matches!(script.instructions[0], CslInstruction::CslVersion(v) if v.major == 1 && v.minor == 1));

        // Test Display outputs version first
        let script = CslScript::new()
            .with_reset(ResetType::Hard)
            .with_wait(1000)
            .with_instruction(CslInstruction::csl_version(1, 0));

        let output = script.to_string();
        let lines: Vec<&str> = output.lines().collect();
        assert!(lines[0].starts_with("csl_version"), "First line should be version, got: {}", lines[0]);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_path_normalization_z_drive() {
        // Test that Z: drive paths have forward slashes replaced with backslashes on Linux
        let script = CslScript::new()
            .with_disk_dir("/path/to/disks".into())
            .with_disk_insert(Drive::A, "/path/to/game.dsk".into())
            .with_snapshot_dir("/snapshots/dir".into())
            .with_snapshot_load("/snapshots/game.sna".into());

        let output = script.to_string();

        eprint!("{}", &output);
        
        // Verify Z: paths use backslashes
        assert!(output.contains(r"disk_dir 'Z:\path\to\disks\'"), "Expected Z: path with backslashes, got: {}", output);
        assert!(output.contains(r"disk_insert A 'Z:\path\to\game.dsk'"), "Expected Z: path with backslashes, got: {}", output);
        assert!(output.contains(r"snapshot_dir 'Z:\snapshots\dir\'"), "Expected Z: path with backslashes, got: {}", output);
        assert!(output.contains(r"snapshot_load 'Z:\snapshots\game.sna'"), "Expected Z: path with backslashes, got: {}", output);
    }

    #[test]
    #[ignore]
    #[cfg(target_os = "linux")]
    fn test_path_normalization_regular_paths() {
        // Test that non-Z: paths are left unchanged
        let script = CslScript::new()
            .with_disk_dir("/home/user/disks".into())
            .with_disk_insert(Drive::B, "relative/path/game.dsk".into());

        let output = script.to_string();
        
        // Verify non-Z: paths use forward slashes
        assert!(output.contains(r"disk_dir 'Z:\home\user\disks\'"), "Expected regular path unchanged, got: {}", output);
        assert!(output.contains("disk_insert B 'relative\\path\\game.dsk'"), "Expected regular path unchanged, got: {}", output);
    }

    #[test]
    fn test_csl_script_builder() {
        // Test builder with build() method
        let builder = CslScriptBuilder::new(1, 0)
            .with_reset(ResetType::Hard)
            .with_wait(1000);

        let result = builder.build();
        assert!(result.is_ok());
        let script = result.unwrap();
        
        // Should have 3 instructions: version (added automatically), reset, wait
        assert_eq!(script.instructions.len(), 3);
        assert!(matches!(script.instructions[0], CslInstruction::CslVersion(_)));

        // Test builder moves version to front
        let builder = CslScriptBuilder::new(1, 0)
            .with_reset(ResetType::Hard)
            .with_instruction(CslInstruction::csl_version(1, 1))
            .with_wait(1000);

        let result = builder.build();
        assert!(result.is_ok());
        let script = result.unwrap();
        
        assert_eq!(script.instructions.len(), 3);
        assert!(matches!(script.instructions[0], CslInstruction::CslVersion(v) if v.major == 1 && v.minor == 1));

        // Test Deref works
        let mut builder = CslScriptBuilder::new(1, 0);
        builder.add_instruction(CslInstruction::reset(ResetType::Soft));
        assert_eq!(builder.instructions.len(), 2); // version + reset

        let script = builder.build().unwrap();
        assert_eq!(script.instructions.len(), 2); // version + reset
    }
}
