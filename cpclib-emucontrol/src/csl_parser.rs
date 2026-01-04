//! CSL (CPC Script Language) parser using winnow
//!
//! This module provides parsing capabilities for CSL script files.

use crate::csl::*;
use cpclib_common::winnow::ascii::{dec_uint, line_ending};
use cpclib_common::winnow::combinator::{
    alt, delimited, opt, preceded, repeat, terminated
};
use cpclib_common::winnow::error::{ContextError, StrContext};
use cpclib_common::winnow::token::{one_of, take_till, take_until, take_while};
use cpclib_common::winnow::{ModalResult, Parser};

/// Parse result type
type ParseResult<'a, T> = ModalResult<T, ContextError<StrContext>>;

/// Parse whitespace (spaces and tabs)
fn ws0<'a>(input: &mut &'a str) -> ParseResult<'a, ()> {
    take_while(0.., [' ', '\t'])
        .void()
        .parse_next(input)
}

/// Parse whitespace (at least one)
fn ws1<'a>(input: &mut &'a str) -> ParseResult<'a, ()> {
    take_while(1.., [' ', '\t'])
        .void()
        .parse_next(input)
}

/// Parse a quoted string (with single quotes)
fn quoted_string<'a>(input: &mut &'a str) -> ParseResult<'a, String> {
    delimited(
        '\'',
        take_until(0.., '\''),
        '\''
    )
    .map(|s: &str| s.to_string())
    .context(StrContext::Label("Quoted string"))
    .parse_next(input)
}

/// Parse optional inline comment (after an instruction, before line ending)
fn inline_comment<'a>(input: &mut &'a str) -> ParseResult<'a, ()> {
    opt(preceded(
        ws0,
        preceded(
            ';',
            take_till(0.., |c| c == '\n' || c == '\r')
        )
    ))
    .void()
    .parse_next(input)
}

/// Parse a semicolon comment (everything after ; until end of line)
fn comment<'a>(input: &mut &'a str) -> ParseResult<'a, CslInstruction> {
    preceded(
        ';',
        take_till(0.., ['\n', '\r'])
    )
    .map(|s: &str| CslInstruction::Comment(s.to_string()))
    .parse_next(input)
}

/// Parse CSL version number (e.g., "1.0" or "1.1")
fn csl_version_number<'a>(input: &mut &'a str) -> ParseResult<'a, CslVersion> {
    (dec_uint::<_, u8, _>, '.', dec_uint::<_, u8, _>)
        .map(|(major, _, minor)| CslVersion::new(major, minor))
        .context(StrContext::Label("Version number"))
        .parse_next(input)
}

/// Parse csl_version instruction
fn parse_csl_version<'a>(input: &mut &'a str) -> ParseResult<'a, CslInstruction> {
    preceded(
        ("csl_version", ws1),
        csl_version_number
    )
    .map(CslInstruction::CslVersion)
    .context(StrContext::Label("csl_version"))
    .parse_next(input)
}

/// Parse reset type
fn parse_reset_type<'a>(input: &mut &'a str) -> ParseResult<'a, ResetType> {
    alt((
        "soft".value(ResetType::Soft),
        "hard".value(ResetType::Hard)
    ))
    .context(StrContext::Label("Reset type"))
    .parse_next(input)
}

/// Parse reset instruction
fn parse_reset<'a>(input: &mut &'a str) -> ParseResult<'a, CslInstruction> {
    preceded(
        ("reset", ws0),
        opt(parse_reset_type)
    )
    .map(|t| CslInstruction::Reset(t.unwrap_or(ResetType::Hard)))
    .context(StrContext::Label("reset"))
    .parse_next(input)
}

/// Parse CRTC model
fn parse_crtc_model<'a>(input: &mut &'a str) -> ParseResult<'a, CrtcModel> {
    alt((
        "0".value(CrtcModel::Type0),
        "1A".value(CrtcModel::Type1A),
        "1B".value(CrtcModel::Type1B),
        "1".value(CrtcModel::Type1),
        "2".value(CrtcModel::Type2),
        "3".value(CrtcModel::Type3),
        "4".value(CrtcModel::Type4)
    ))
    .context(StrContext::Label("CRTC model"))
    .parse_next(input)
}

/// Parse crtc_select instruction
fn parse_crtc_select<'a>(input: &mut &'a str) -> ParseResult<'a, CslInstruction> {
    preceded(
        ("crtc_select", ws1),
        parse_crtc_model
    )
    .map(CslInstruction::CrtcSelect)
    .context(StrContext::Label("crtc_select"))
    .parse_next(input)
}

/// Parse gate array model
fn parse_gate_array_model<'a>(input: &mut &'a str) -> ParseResult<'a, GateArrayModel> {
    alt((
        "40007".value(GateArrayModel::Model40007),
        "40008".value(GateArrayModel::Model40008),
        "40010".value(GateArrayModel::Model40010)
    ))
    .context(StrContext::Label("Gate array model"))
    .parse_next(input)
}

/// Parse gate_array instruction
fn parse_gate_array<'a>(input: &mut &'a str) -> ParseResult<'a, CslInstruction> {
    preceded(
        ("gate_array", ws1),
        parse_gate_array_model
    )
    .map(CslInstruction::GateArray)
    .context(StrContext::Label("gate_array"))
    .parse_next(input)
}

/// Parse CPC model
fn parse_cpc_model<'a>(input: &mut &'a str) -> ParseResult<'a, CpcModel> {
    alt((
        "0".value(CpcModel::Cpc464),
        "1".value(CpcModel::Cpc664),
        "2".value(CpcModel::Cpc6128),
        "4".value(CpcModel::Cpc6128Plus),
        "5".value(CpcModel::Cpc464Plus),
        "6".value(CpcModel::GX4000)
    ))
    .context(StrContext::Label("CPC model"))
    .parse_next(input)
}

/// Parse cpc_model instruction
fn parse_cpc_model_instr<'a>(input: &mut &'a str) -> ParseResult<'a, CslInstruction> {
    preceded(
        ("cpc_model", ws1),
        parse_cpc_model
    )
    .map(CslInstruction::CpcModel)
    .context(StrContext::Label("cpc_model"))
    .parse_next(input)
}

/// Parse memory expansion
fn parse_memory_expansion<'a>(input: &mut &'a str) -> ParseResult<'a, MemoryExpansion> {
    alt((
        "0".value(MemoryExpansion::Kb128),
        "1".value(MemoryExpansion::Kb256Standard),
        "2".value(MemoryExpansion::Kb256Silicon),
        "3".value(MemoryExpansion::Mb4),
        "4".value(MemoryExpansion::Kb512DkTronics)
    ))
    .context(StrContext::Label("Memory expansion"))
    .parse_next(input)
}

/// Parse memory_exp instruction
fn parse_memory_exp<'a>(input: &mut &'a str) -> ParseResult<'a, CslInstruction> {
    preceded(
        ("memory_exp", ws1),
        parse_memory_expansion
    )
    .map(CslInstruction::MemoryExp)
    .context(StrContext::Label("memory_exp"))
    .parse_next(input)
}

/// Parse rom_dir instruction
fn parse_rom_dir<'a>(input: &mut &'a str) -> ParseResult<'a, CslInstruction> {
    preceded(
        ("rom_dir", ws1),
        quoted_string
    )
    .map(CslInstruction::RomDir)
    .context(StrContext::Label("rom_dir"))
    .parse_next(input)
}

/// Parse ROM type
fn parse_rom_type<'a>(input: &mut &'a str) -> ParseResult<'a, RomType> {
    alt((
        "U".value(RomType::Upper),
        "L".value(RomType::Lower),
        "C".value(RomType::Cartridge),
        "M".value(RomType::Multiface2)
    ))
    .context(StrContext::Label("ROM type"))
    .parse_next(input)
}

/// Parse rom_config instruction
fn parse_rom_config<'a>(input: &mut &'a str) -> ParseResult<'a, CslInstruction> {
    preceded(
        ("rom_config", ws1),
        (
            parse_rom_type,
            preceded(ws1, dec_uint::<_, u8, _>),
            preceded(ws1, quoted_string)
        )
    )
    .map(|(rom_type, num, filename)| {
        CslInstruction::RomConfig(RomConfig {
            rom_type,
            num,
            filename
        })
    })
    .context(StrContext::Label("rom_config"))
    .parse_next(input)
}

/// Parse drive letter
fn parse_drive<'a>(input: &mut &'a str) -> ParseResult<'a, Drive> {
    alt((
        "A".value(Drive::A),
        "B".value(Drive::B)
    ))
    .context(StrContext::Label("Drive"))
    .parse_next(input)
}

/// Parse disk_insert instruction
fn parse_disk_insert<'a>(input: &mut &'a str) -> ParseResult<'a, CslInstruction> {
    preceded(
        ("disk_insert", ws1),
        (
            opt(terminated(parse_drive, ws1)),
            quoted_string
        )
    )
    .map(|(drive, filename)| {
        CslInstruction::DiskInsert {
            drive: drive.unwrap_or(Drive::A),
            filename
        }
    })
    .context(StrContext::Label("disk_insert"))
    .parse_next(input)
}

/// Parse disk_dir instruction
fn parse_disk_dir<'a>(input: &mut &'a str) -> ParseResult<'a, CslInstruction> {
    preceded(
        ("disk_dir", ws1),
        quoted_string
    )
    .map(CslInstruction::DiskDir)
    .context(StrContext::Label("disk_dir"))
    .parse_next(input)
}

/// Parse tape_insert instruction
fn parse_tape_insert<'a>(input: &mut &'a str) -> ParseResult<'a, CslInstruction> {
    preceded(
        ("tape_insert", ws1),
        quoted_string
    )
    .map(CslInstruction::TapeInsert)
    .context(StrContext::Label("tape_insert"))
    .parse_next(input)
}

/// Parse tape_dir instruction
fn parse_tape_dir<'a>(input: &mut &'a str) -> ParseResult<'a, CslInstruction> {
    preceded(
        ("tape_dir", ws1),
        quoted_string
    )
    .map(CslInstruction::TapeDir)
    .context(StrContext::Label("tape_dir"))
    .parse_next(input)
}

/// Parse tape_play instruction
fn parse_tape_play<'a>(input: &mut &'a str) -> ParseResult<'a, CslInstruction> {
    "tape_play"
        .value(CslInstruction::TapePlay)
        .context(StrContext::Label("tape_play"))
        .parse_next(input)
}

/// Parse tape_stop instruction
fn parse_tape_stop<'a>(input: &mut &'a str) -> ParseResult<'a, CslInstruction> {
    "tape_stop"
        .value(CslInstruction::TapeStop)
        .context(StrContext::Label("tape_stop"))
        .parse_next(input)
}

/// Parse tape_rewind instruction
fn parse_tape_rewind<'a>(input: &mut &'a str) -> ParseResult<'a, CslInstruction> {
    "tape_rewind"
        .value(CslInstruction::TapeRewind)
        .context(StrContext::Label("tape_rewind"))
        .parse_next(input)
}

/// Parse snapshot_load instruction
fn parse_snapshot_load<'a>(input: &mut &'a str) -> ParseResult<'a, CslInstruction> {
    preceded(
        ("snapshot_load", ws1),
        quoted_string
    )
    .map(CslInstruction::SnapshotLoad)
    .context(StrContext::Label("snapshot_load"))
    .parse_next(input)
}

/// Parse snapshot_dir instruction
fn parse_snapshot_dir<'a>(input: &mut &'a str) -> ParseResult<'a, CslInstruction> {
    preceded(
        ("snapshot_dir", ws1),
        quoted_string
    )
    .map(CslInstruction::SnapshotDir)
    .context(StrContext::Label("snapshot_dir"))
    .parse_next(input)
}

/// Parse key_delay instruction
fn parse_key_delay<'a>(input: &mut &'a str) -> ParseResult<'a, CslInstruction> {
    preceded(
        ("key_delay", ws1),
        (
            dec_uint::<_, u64, _>,
            opt(preceded(ws1, dec_uint::<_, u64, _>)),
            opt(preceded(ws1, dec_uint::<_, u64, _>))
        )
    )
    .map(|(delay, delay_after_cr, delay_after_key)| {
        CslInstruction::KeyDelay {
            delay,
            delay_after_cr,
            delay_after_key
        }
    })
    .context(StrContext::Label("key_delay"))
    .parse_next(input)
}

/// Parse special key escape sequence
fn parse_special_key<'a>(input: &mut &'a str) -> ParseResult<'a, SpecialKey> {
    delimited(
        "\\(",
        alt((
            alt((
                "ESC".value(SpecialKey::Esc),
                "TAB".value(SpecialKey::Tab),
                "CAP".value(SpecialKey::CapsLock),
                "SHI".value(SpecialKey::Shift),
                "CTR".value(SpecialKey::Ctrl),
                "COP".value(SpecialKey::Copy),
                "CLR".value(SpecialKey::Clr),
                "DEL".value(SpecialKey::Del),
                "RET".value(SpecialKey::Return),
                "ENT".value(SpecialKey::Enter),
                "ARL".value(SpecialKey::ArrowLeft),
                "ARR".value(SpecialKey::ArrowRight),
                "ARU".value(SpecialKey::ArrowUp),
                "ARD".value(SpecialKey::ArrowDown),
                "FN0".value(SpecialKey::F0),
            )),
            alt((
                "FN1".value(SpecialKey::F1),
                "FN2".value(SpecialKey::F2),
                "FN3".value(SpecialKey::F3),
                "FN4".value(SpecialKey::F4),
                "FN5".value(SpecialKey::F5),
                "FN6".value(SpecialKey::F6),
                "FN7".value(SpecialKey::F7),
                "FN8".value(SpecialKey::F8),
                "FN9".value(SpecialKey::F9),
                "{".value(SpecialKey::LeftBrace),
                "}".value(SpecialKey::RightBrace),
                "\\".value(SpecialKey::Backslash),
                "'".value(SpecialKey::Quote),
                "KOF".value(SpecialKey::NoDelayNextKey)
            ))
        )),
        ")"
    )
    .context(StrContext::Label("Special key"))
    .parse_next(input)
}

/// Parse a key element (character or special key)
fn parse_key_element<'a>(input: &mut &'a str) -> ParseResult<'a, KeyElement> {
    alt((
        parse_special_key.map(KeyElement::Special),
        one_of(|c: char| c != '\'' && c != '\\' && c != '{' && c != '}')
            .map(KeyElement::Character)
    ))
    .parse_next(input)
}

/// Parse simultaneous key group: {keys}
fn parse_simultaneous_keys<'a>(input: &mut &'a str) -> ParseResult<'a, KeyElement> {
    delimited(
        '{',
        repeat(1.., parse_key_element),
        '}'
    )
    .map(KeyElement::Simultaneous)
    .context(StrContext::Label("Simultaneous keys"))
    .parse_next(input)
}

/// Parse key output text
fn parse_key_output_content<'a>(input: &mut &'a str) -> ParseResult<'a, Vec<KeyElement>> {
    delimited(
        '\'',
        repeat(0.., alt((parse_simultaneous_keys, parse_key_element))),
        '\''
    )
    .context(StrContext::Label("Key output content"))
    .parse_next(input)
}

/// Parse key_output instruction
fn parse_key_output<'a>(input: &mut &'a str) -> ParseResult<'a, CslInstruction> {
    preceded(
        ("key_output", ws1),
        parse_key_output_content
    )
    .map(CslInstruction::KeyOutput)
    .context(StrContext::Label("key_output"))
    .parse_next(input)
}

/// Parse key_from_file instruction
fn parse_key_from_file<'a>(input: &mut &'a str) -> ParseResult<'a, CslInstruction> {
    preceded(
        ("key_from_file", ws1),
        quoted_string
    )
    .map(CslInstruction::KeyFromFile)
    .context(StrContext::Label("key_from_file"))
    .parse_next(input)
}

/// Parse wait instruction
fn parse_wait<'a>(input: &mut &'a str) -> ParseResult<'a, CslInstruction> {
    preceded(
        ("wait", ws1),
        dec_uint::<_, u64, _>
    )
    .map(CslInstruction::Wait)
    .context(StrContext::Label("wait"))
    .parse_next(input)
}

/// Parse wait_driveonoff instruction
fn parse_wait_driveonoff<'a>(input: &mut &'a str) -> ParseResult<'a, CslInstruction> {
    preceded(
        ("wait_driveonoff", ws0),
        opt(preceded(ws1, dec_uint::<_, u32, _>))
    )
    .map(|n| CslInstruction::WaitDriveOnOff(n.unwrap_or(1)))
    .context(StrContext::Label("wait_driveonoff"))
    .parse_next(input)
}

/// Parse wait_vsyncoffon instruction
fn parse_wait_vsyncoffon<'a>(input: &mut &'a str) -> ParseResult<'a, CslInstruction> {
    "wait_vsyncoffon"
        .value(CslInstruction::WaitVsyncOffOn)
        .context(StrContext::Label("wait_vsyncoffon"))
        .parse_next(input)
}

/// Parse wait_ssm0000 instruction
fn parse_wait_ssm0000<'a>(input: &mut &'a str) -> ParseResult<'a, CslInstruction> {
    "wait_ssm0000"
        .value(CslInstruction::WaitSsm0000)
        .context(StrContext::Label("wait_ssm0000"))
        .parse_next(input)
}

/// Parse screenshot_name instruction
fn parse_screenshot_name<'a>(input: &mut &'a str) -> ParseResult<'a, CslInstruction> {
    preceded(
        ("screenshot_name", ws1),
        quoted_string
    )
    .map(CslInstruction::ScreenshotName)
    .context(StrContext::Label("screenshot_name"))
    .parse_next(input)
}

/// Parse screenshot_dir instruction
fn parse_screenshot_dir<'a>(input: &mut &'a str) -> ParseResult<'a, CslInstruction> {
    preceded(
        ("screenshot_dir", ws1),
        quoted_string
    )
    .map(CslInstruction::ScreenshotDir)
    .context(StrContext::Label("screenshot_dir"))
    .parse_next(input)
}

/// Parse screenshot instruction
fn parse_screenshot<'a>(input: &mut &'a str) -> ParseResult<'a, CslInstruction> {
    preceded(
        ("screenshot", ws0),
        opt(preceded(ws1, "vsync"))
    )
    .map(|vsync| CslInstruction::Screenshot {
        wait_vsync: vsync.is_some()
    })
    .context(StrContext::Label("screenshot"))
    .parse_next(input)
}

/// Parse snapshot_name instruction
fn parse_snapshot_name<'a>(input: &mut &'a str) -> ParseResult<'a, CslInstruction> {
    preceded(
        ("snapshot_name", ws1),
        quoted_string
    )
    .map(CslInstruction::SnapshotName)
    .context(StrContext::Label("snapshot_name"))
    .parse_next(input)
}

/// Parse snapshot instruction
fn parse_snapshot<'a>(input: &mut &'a str) -> ParseResult<'a, CslInstruction> {
    preceded(
        ("snapshot", ws0),
        opt(preceded(ws1, "vsync"))
    )
    .map(|vsync| CslInstruction::Snapshot {
        wait_vsync: vsync.is_some()
    })
    .context(StrContext::Label("snapshot"))
    .parse_next(input)
}

/// Parse snapshot version
fn parse_snapshot_version_num<'a>(input: &mut &'a str) -> ParseResult<'a, SnapshotVersion> {
    alt((
        "1".value(SnapshotVersion::V1),
        "2".value(SnapshotVersion::V2),
        "3".value(SnapshotVersion::V3)
    ))
    .context(StrContext::Label("Snapshot version"))
    .parse_next(input)
}

/// Parse snapshot_version instruction
fn parse_snapshot_version<'a>(input: &mut &'a str) -> ParseResult<'a, CslInstruction> {
    preceded(
        ("snapshot_version", ws1),
        parse_snapshot_version_num
    )
    .map(CslInstruction::SnapshotVersion)
    .context(StrContext::Label("snapshot_version"))
    .parse_next(input)
}

/// Parse csl_load instruction
fn parse_csl_load<'a>(input: &mut &'a str) -> ParseResult<'a, CslInstruction> {
    preceded(
        ("csl_load", ws1),
        quoted_string
    )
    .map(CslInstruction::CslLoad)
    .context(StrContext::Label("csl_load"))
    .parse_next(input)
}

/// Parse empty line (whitespace + line ending)
fn parse_empty_line<'a>(input: &mut &'a str) -> ParseResult<'a, CslInstruction> {
    terminated(ws0, line_ending)
        .value(CslInstruction::Empty)
        .parse_next(input)
}

/// Parse any CSL instruction
pub fn parse_instruction<'a>(input: &mut &'a str) -> ParseResult<'a, CslInstruction> {
    preceded(
        ws0,
        alt((
            alt((
                comment,
                parse_csl_version,
                parse_reset,
                parse_crtc_select,
                parse_gate_array,
                parse_cpc_model_instr,
                parse_memory_exp,
                parse_rom_dir,
                parse_rom_config,
                parse_disk_insert,
                parse_disk_dir,
                parse_tape_insert,
                parse_tape_dir,
                parse_tape_play,
                parse_tape_stop,
                parse_tape_rewind,
                parse_snapshot_load,
                parse_snapshot_dir,
                parse_key_delay,
                parse_key_output,
            )),
            alt((
                parse_key_from_file,
                parse_wait_driveonoff,
                parse_wait_vsyncoffon,
                parse_wait_ssm0000,
                parse_wait,
                parse_screenshot_name,
                parse_screenshot_dir,
                parse_screenshot,
                parse_snapshot_name,
                parse_snapshot_version,
                parse_snapshot,
                parse_csl_load,
                parse_empty_line
            ))
        ))
    )
    .context(StrContext::Label("CSL instruction"))
    .parse_next(input)
}

/// Parse a single line (instruction + optional inline comment + optional line ending)
pub fn parse_line<'a>(input: &mut &'a str) -> ParseResult<'a, CslInstruction> {
    alt((
        parse_empty_line,
        terminated(
            terminated(
                parse_instruction,
                inline_comment  // Consume inline comments after instruction
            ),
            opt(line_ending)  // Make line ending optional for last line
        )
    ))
    .parse_next(input)
}

/// Parse a complete CSL script
pub fn parse_csl_script<'a>(input: &mut &'a str) -> ParseResult<'a, CslScript> {
    terminated(
        repeat(0.., parse_line),
        ws0  // Allow trailing whitespace at end of file
    )
    .map(|instructions| CslScript { instructions })
    .parse_next(input)
}

/// Parse a CSL script from a string
pub fn parse_csl(input: &str) -> Result<CslScript, ContextError<StrContext>> {
    // Use parse_next instead of parse to allow unconsumed input
    let mut input_mut = input;
    let result = parse_csl_script.parse_next(&mut input_mut);
    
    result.map_err(|e| match e.into_inner() {
        Some(inner) => inner,
        None => ContextError::new()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_comment() {
        let result = parse_line.parse("; This is a comment\n");
        assert!(result.is_ok());
        if let Ok(CslInstruction::Comment(text)) = result {
            assert_eq!(text, " This is a comment");
        }
    }

    #[test]
    fn test_parse_csl_version() {
        let result = parse_line.parse("csl_version 1.1\n");
        assert!(result.is_ok());
        if let Ok(CslInstruction::CslVersion(v)) = result {
            assert_eq!(v.major, 1);
            assert_eq!(v.minor, 1);
        }
    }

    #[test]
    fn test_parse_reset() {
        let result = parse_line.parse("reset soft\n");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), CslInstruction::Reset(ResetType::Soft));

        let result = parse_line.parse("reset\n");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), CslInstruction::Reset(ResetType::Hard));
    }

    #[test]
    fn test_parse_disk_insert() {
        let result = parse_line.parse("disk_insert 'SHAKER25.DSK'\n");
        assert!(result.is_ok());
        if let Ok(CslInstruction::DiskInsert { drive, filename }) = result {
            assert_eq!(drive, Drive::A);
            assert_eq!(filename, "SHAKER25.DSK");
        }

        let result = parse_line.parse("disk_insert B 'AMAZING.DSK'\n");
        assert!(result.is_ok());
        if let Ok(CslInstruction::DiskInsert { drive, filename }) = result {
            assert_eq!(drive, Drive::B);
            assert_eq!(filename, "AMAZING.DSK");
        }
    }

    #[test]
    fn test_parse_key_output() {
        let result = parse_line.parse("key_output 'RUN \"SHAKE25A\"\\(RET)'\n");
        assert!(result.is_ok());
        if let Ok(CslInstruction::KeyOutput(keys)) = result {
            assert_eq!(keys.len(), 15); // R U N space " S H A K E 2 5 A " \(RET)
        }
    }

    #[test]
    fn test_parse_wait() {
        let result = parse_line.parse("wait 1300455\n");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), CslInstruction::Wait(1300455));
    }

    #[test]
    fn test_parse_script() {
        let script = r#"
; Test CSL script
csl_version 1.1
reset soft
disk_insert 'test.dsk'
tape_play
wait 100000
"#;
        let result = parse_csl(script);
        assert!(result.is_ok());
        let script = result.unwrap();
        assert!(script.instructions.len() > 0);
    }

    #[test]
    fn test_parse_windows_line_endings() {
        let input = ";comment\r\ncsl_version 1.0\r\nreset\r\nwait 1000\r\n";
        let result = parse_csl(input);
        assert!(result.is_ok(), "Failed to parse with Windows line endings: {:?}", result);
        let script = result.unwrap();
        assert_eq!(script.instructions.len(), 4);
    }

    #[test]
    fn test_parse_with_trailing_content() {
        let input = "csl_version 1.0\nreset\n;";
        let result = parse_csl(input);
        assert!(result.is_ok(), "Failed to parse with trailing semicolon: {:?}", result);
    }

    #[test]
    fn test_parse_inline_comment() {
        let input = "wait 800000\t\t\t; fin affichage 1er ecran\n";
        let result = parse_line.parse(input);
        assert!(result.is_ok(), "Failed to parse inline comment: {:?}", result);
        assert!(matches!(result.unwrap(), CslInstruction::Wait(800000)));
    }

    #[test]
    fn test_parse_trailing_space() {
        let input = "wait 3000000 \n";
        let mut input_mut = input;
        let result = parse_line.parse_next(&mut input_mut);
        assert!(result.is_ok(), "Failed to parse with trailing space: {:?}", result);
    }

    #[test]
    fn test_parse_key_output_space() {
        let input = "key_output ' '\n";
        let result = parse_line.parse(input);
        assert!(result.is_ok(), "Failed to parse key_output with space: {:?}", result);
    }

    #[test]
    fn test_parse_empty_line() {
        let result = parse_line.parse("\n");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), CslInstruction::Empty);
    }

    #[test]
    fn test_parse_rom_config() {
        let result = parse_line.parse("rom_config U 7 'Amsdos.rom'\n");
        assert!(result.is_ok());
        if let Ok(CslInstruction::RomConfig(config)) = result {
            assert_eq!(config.rom_type, RomType::Upper);
            assert_eq!(config.num, 7);
            assert_eq!(config.filename, "Amsdos.rom");
        }
    }
}
