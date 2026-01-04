//! CSL (CPC Script Language) token modeling
//!
//! This module models the tokens and instructions of the CSL language v1.1
//! as specified in the CSL-STANDARD_EN.pdf document.
//!
//! CSL is a scripting language that allows precise automation of emulator control,
//! simulating user actions. It uses a simple text format with one instruction per line.
//! Semicolons are used for comments.

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

/// Gate Array model
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateArrayModel {
    Model40007,
    Model40008,
    Model40010
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

/// ROM configuration
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RomConfig {
    pub rom_type: RomType,
    pub num: u8,
    pub filename: String
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

/// Snapshot version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotVersion {
    V1,
    V2,
    V3
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
    RomDir(String),
    
    /// Configure a ROM (v1.1)
    RomConfig(RomConfig),

    // Media
    /// Insert a disk file into a drive
    DiskInsert { drive: Drive, filename: String },
    
    /// Specify disk directory
    DiskDir(String),
    
    /// Insert a tape file
    TapeInsert(String),
    
    /// Specify tape directory
    TapeDir(String),
    
    /// Start tape playback
    TapePlay,
    
    /// Stop tape playback
    TapeStop,
    
    /// Rewind tape to beginning
    TapeRewind,
    
    /// Load a snapshot file
    SnapshotLoad(String),
    
    /// Specify snapshot directory
    SnapshotDir(String),

    // Key strokes
    /// Set delay between keystrokes (in microseconds)
    /// First param: delay between keys, Second param (optional): delay after CR, Third param (optional): delay after special key
    KeyDelay { delay: u64, delay_after_cr: Option<u64>, delay_after_key: Option<u64> },
    
    /// Send text as key strokes
    KeyOutput(Vec<KeyElement>),
    
    /// Send characters from file as key strokes
    KeyFromFile(String),

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
    ScreenshotName(String),
    
    /// Specify screenshot directory
    ScreenshotDir(String),
    
    /// Take a screenshot (optionally wait for vsync)
    Screenshot { wait_vsync: bool },
    
    /// Specify name for next snapshot (without extension)
    SnapshotName(String),
    
    /// Take a snapshot (optionally wait for vsync)
    Snapshot { wait_vsync: bool },
    
    /// Select snapshot version
    SnapshotVersion(SnapshotVersion),
    
    /// Load and run another CSL file
    CslLoad(String),

    // Comments (for completeness)
    Comment(String),
    
    // Empty line
    Empty
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
}

impl Default for CslScript {
    fn default() -> Self {
        Self::new()
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
}
