//! CSL (CPC Script Language) parser using winnow
//!
//! This module provides parsing capabilities for CSL script files.

#[cfg(test)]
use cpclib_common::winnow::ModalParser;
use cpclib_common::winnow::ascii::{dec_uint, line_ending};
use cpclib_common::winnow::combinator::{
    alt, cut_err, delimited, opt, preceded, repeat, terminated
};
use cpclib_common::winnow::error::{ContextError, StrContext};
use cpclib_common::winnow::stream::LocatingSlice;
use cpclib_common::winnow::token::{one_of, take_till, take_until, take_while};
use cpclib_common::winnow::{ModalResult, Parser};

use crate::csl::*;

/// Parse result type with Located input
type ParseResult<'a, T> = ModalResult<T, ContextError<StrContext>>;

/// Parse whitespace (spaces and tabs)
fn ws0<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, ()> {
    take_while(0.., [' ', '\t']).void().parse_next(input)
}

/// Parse whitespace (at least one)
fn ws1<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, ()> {
    take_while(1.., [' ', '\t']).void().parse_next(input)
}

/// Parse a quoted string (with single quotes)
fn quoted_string<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, String> {
    delimited('\'', take_until(0.., '\''), '\'')
        .map(|s: &str| s.to_string())
        .context(StrContext::Label("Quoted string"))
        .parse_next(input)
}

/// Parse a quoted path string and convert to Utf8PathBuf
fn quoted_path<'a>(
    input: &mut LocatingSlice<&'a str>
) -> ParseResult<'a, cpclib_common::camino::Utf8PathBuf> {
    quoted_string
        .map(cpclib_common::camino::Utf8PathBuf::from)
        .parse_next(input)
}

/// Parse optional inline comment (after an instruction, before line ending)
/// Returns Some(comment) if a comment is present, None otherwise
fn inline_comment<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, Option<String>> {
    opt(preceded(
        ws0,
        preceded(';', take_till(0.., |c| c == '\n' || c == '\r'))
    ))
    .map(|opt_str: Option<&str>| opt_str.map(|s| s.to_string()))
    .parse_next(input)
}

/// Parse a semicolon comment (everything after ; until end of line)
fn comment<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, CslInstruction> {
    preceded(';', take_till(0.., ['\n', '\r']))
        .map(|s: &str| CslInstruction::Comment(s.to_string()))
        .parse_next(input)
}

/// Parse CSL version number (e.g., "1.0" or "1.1")
fn csl_version_number<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, CslVersion> {
    (dec_uint::<_, u8, _>, '.', dec_uint::<_, u8, _>)
        .map(|(major, _, minor)| CslVersion::new(major, minor))
        .context(StrContext::Label("Version number"))
        .parse_next(input)
}

/// Parse csl_version instruction
fn parse_csl_version<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, CslInstruction> {
    ("csl_version", ws1)
        .context(StrContext::Label("csl_version"))
        .parse_next(input)?;

    cut_err(csl_version_number.context(StrContext::Label("version number (e.g. 1.1)")))
        .map(CslInstruction::CslVersion)
        .parse_next(input)
}

/// Parse reset type
fn parse_reset_type<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, ResetType> {
    alt((
        alt(("soft".value(ResetType::Soft), "S".value(ResetType::Soft))),
        alt(("hard".value(ResetType::Hard), "H".value(ResetType::Hard)))
    ))
    .context(StrContext::Label("Reset type"))
    .parse_next(input)
}

/// Parse reset instruction
fn parse_reset<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, CslInstruction> {
    ("reset", ws0)
        .context(StrContext::Label("reset"))
        .parse_next(input)?;

    cut_err(opt(parse_reset_type).context(StrContext::Label("reset type (soft or hard)")))
        .map(|t| CslInstruction::Reset(t.unwrap_or(ResetType::Hard)))
        .parse_next(input)
}

/// Parse CRTC model
fn parse_crtc_model<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, CrtcModel> {
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
fn parse_crtc_select<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, CslInstruction> {
    ("crtc_select", ws1)
        .context(StrContext::Label("crtc_select"))
        .parse_next(input)?;

    cut_err(parse_crtc_model.context(StrContext::Label("CRTC model (0, 1, 1A, 1B, 2, 3, 4)")))
        .map(CslInstruction::CrtcSelect)
        .parse_next(input)
}

/// Parse gate array model
fn parse_gate_array_model<'a>(
    input: &mut LocatingSlice<&'a str>
) -> ParseResult<'a, GateArrayModel> {
    alt((
        "40007".value(GateArrayModel::Model40007),
        "40008".value(GateArrayModel::Model40008),
        "40010".value(GateArrayModel::Model40010)
    ))
    .context(StrContext::Label("Gate array model"))
    .parse_next(input)
}

/// Parse gate_array instruction
fn parse_gate_array<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, CslInstruction> {
    ("gate_array", ws1)
        .context(StrContext::Label("gate_array"))
        .parse_next(input)?;

    cut_err(
        parse_gate_array_model.context(StrContext::Label("gate array model (40007, 40008, 40010)"))
    )
    .map(CslInstruction::GateArray)
    .parse_next(input)
}

/// Parse CPC model
fn parse_cpc_model<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, CpcModel> {
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
fn parse_cpc_model_instr<'a>(
    input: &mut LocatingSlice<&'a str>
) -> ParseResult<'a, CslInstruction> {
    ("cpc_model", ws1)
        .context(StrContext::Label("cpc_model"))
        .parse_next(input)?;

    cut_err(parse_cpc_model.context(StrContext::Label(
        "CPC model (0=464, 1=664, 2=6128, 4=6128+, 5=464+, 6=GX4000)"
    )))
    .map(CslInstruction::CpcModel)
    .parse_next(input)
}

/// Parse memory expansion
fn parse_memory_expansion<'a>(
    input: &mut LocatingSlice<&'a str>
) -> ParseResult<'a, MemoryExpansion> {
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
fn parse_memory_exp<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, CslInstruction> {
    ("memory_exp", ws1)
        .context(StrContext::Label("memory_exp"))
        .parse_next(input)?;

    cut_err(parse_memory_expansion.context(StrContext::Label("memory expansion (0-4)")))
        .map(CslInstruction::MemoryExp)
        .parse_next(input)
}

/// Parse rom_dir instruction
fn parse_rom_dir<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, CslInstruction> {
    ("rom_dir", ws1)
        .context(StrContext::Label("rom_dir"))
        .parse_next(input)?;

    cut_err(quoted_path.context(StrContext::Label("ROM directory path (quoted)")))
        .map(CslInstruction::RomDir)
        .parse_next(input)
}

/// Parse ROM type
fn parse_rom_type<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, RomType> {
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
fn parse_rom_config<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, CslInstruction> {
    ("rom_config", ws1)
        .context(StrContext::Label("rom_config"))
        .parse_next(input)?;

    cut_err((
        parse_rom_type.context(StrContext::Label(
            "ROM type (U=Upper, L=Lower, C=Cartridge, M=Multiface2)"
        )),
        preceded(ws1, dec_uint::<_, u8, _>).context(StrContext::Label("ROM number (0-255)")),
        preceded(ws1, quoted_path).context(StrContext::Label("ROM file path (quoted)"))
    ))
    .map(|(rom_type, num, filename)| {
        CslInstruction::RomConfig(RomConfig {
            rom_type,
            num,
            filename
        })
    })
    .parse_next(input)
}

/// Parse drive letter
fn parse_drive<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, Drive> {
    alt(("A".value(Drive::A), "B".value(Drive::B)))
        .context(StrContext::Label("Drive"))
        .parse_next(input)
}

/// Parse disk_insert instruction
fn parse_disk_insert<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, CslInstruction> {
    ("disk_insert", ws1)
        .context(StrContext::Label("disk_insert"))
        .parse_next(input)?;

    cut_err((
        opt(terminated(parse_drive, ws1)),
        quoted_path.context(StrContext::Label("disk file path (quoted)"))
    ))
    .map(|(drive, filename)| {
        CslInstruction::DiskInsert {
            drive: drive.unwrap_or(Drive::A),
            filename
        }
    })
    .parse_next(input)
}

/// Parse disk_dir instruction
fn parse_disk_dir<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, CslInstruction> {
    ("disk_dir", ws1)
        .context(StrContext::Label("disk_dir"))
        .parse_next(input)?;

    cut_err(quoted_path.context(StrContext::Label("disk directory path (quoted)")))
        .map(CslInstruction::DiskDir)
        .parse_next(input)
}

/// Parse tape_insert instruction
fn parse_tape_insert<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, CslInstruction> {
    ("tape_insert", ws1)
        .context(StrContext::Label("tape_insert"))
        .parse_next(input)?;

    cut_err(quoted_path.context(StrContext::Label("tape file path (quoted)")))
        .map(CslInstruction::TapeInsert)
        .parse_next(input)
}

/// Parse tape_dir instruction
fn parse_tape_dir<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, CslInstruction> {
    ("tape_dir", ws1)
        .context(StrContext::Label("tape_dir"))
        .parse_next(input)?;

    cut_err(quoted_path.context(StrContext::Label("tape directory path (quoted)")))
        .map(CslInstruction::TapeDir)
        .parse_next(input)
}

/// Parse tape_play instruction
fn parse_tape_play<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, CslInstruction> {
    "tape_play"
        .value(CslInstruction::TapePlay)
        .context(StrContext::Label("tape_play"))
        .parse_next(input)
}

/// Parse tape_stop instruction
fn parse_tape_stop<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, CslInstruction> {
    "tape_stop"
        .value(CslInstruction::TapeStop)
        .context(StrContext::Label("tape_stop"))
        .parse_next(input)
}

/// Parse tape_rewind instruction
fn parse_tape_rewind<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, CslInstruction> {
    "tape_rewind"
        .value(CslInstruction::TapeRewind)
        .context(StrContext::Label("tape_rewind"))
        .parse_next(input)
}

/// Parse snapshot_load instruction
fn parse_snapshot_load<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, CslInstruction> {
    ("snapshot_load", ws1)
        .context(StrContext::Label("snapshot_load"))
        .parse_next(input)?;

    cut_err(quoted_path.context(StrContext::Label("snapshot file path (quoted)")))
        .map(CslInstruction::SnapshotLoad)
        .parse_next(input)
}

/// Parse snapshot_dir instruction
fn parse_snapshot_dir<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, CslInstruction> {
    ("snapshot_dir", ws1)
        .context(StrContext::Label("snapshot_dir"))
        .parse_next(input)?;

    cut_err(quoted_path.context(StrContext::Label("snapshot directory path (quoted)")))
        .map(CslInstruction::SnapshotDir)
        .parse_next(input)
}

/// Parse key_delay instruction
fn parse_key_delay<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, CslInstruction> {
    ("key_delay", ws1)
        .context(StrContext::Label("key_delay"))
        .parse_next(input)?;

    cut_err((
        dec_uint::<_, u64, _>.context(StrContext::Label("press delay ")),
        opt(preceded(ws1, dec_uint::<_, u64, _>).context(StrContext::Label("delay_after_key"))),
        opt(preceded(ws1, dec_uint::<_, u64, _>))
            .context(StrContext::Label("optional delay_after_cr"))
    ))
    .map(|(delay, delay_after_key, delay_after_cr)| {
        CslInstruction::KeyDelay {
            press_delay: delay,
            delay_after_key,
            delay_after_cr
        }
    })
    .parse_next(input)
}

/// Parse special key escape sequence
fn parse_special_key<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, SpecialKey> {
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
                "FN0".value(SpecialKey::F0)
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
fn parse_key_element<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, KeyElement> {
    alt((
        parse_special_key.map(KeyElement::Special),
        one_of(|c: char| c != '\'' && c != '\\' && c != '{' && c != '}').map(KeyElement::Character)
    ))
    .parse_next(input)
}

/// Parse simultaneous key group: {keys}
fn parse_simultaneous_keys<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, KeyElement> {
    delimited('{', repeat(1.., parse_key_element), '}')
        .map(KeyElement::Simultaneous)
        .context(StrContext::Label("Simultaneous keys"))
        .parse_next(input)
}

/// Parse key output text
pub fn parse_key_output_content<'a>(
    input: &mut LocatingSlice<&'a str>
) -> ParseResult<'a, crate::csl::KeyOutput> {
    delimited(
        '\'',
        repeat(0.., alt((parse_simultaneous_keys, parse_key_element))),
        '\''
    )
    .map(crate::csl::KeyOutput::from_elements)
    .context(StrContext::Label("Key output content"))
    .parse_next(input)
}

/// Parse key_output instruction
fn parse_key_output<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, CslInstruction> {
    ("key_output", ws1)
        .context(StrContext::Label("key_output"))
        .parse_next(input)?;

    cut_err(parse_key_output_content.context(StrContext::Label(
        "key output content (quoted text with optional special keys)"
    )))
    .map(CslInstruction::KeyOutput)
    .parse_next(input)
}

/// Parse key_from_file instruction
fn parse_key_from_file<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, CslInstruction> {
    ("key_from_file", ws1)
        .context(StrContext::Label("key_from_file"))
        .parse_next(input)?;

    cut_err(quoted_path.context(StrContext::Label("key input file path (quoted)")))
        .map(CslInstruction::KeyFromFile)
        .parse_next(input)
}

/// Parse keyboard_write instruction (v1.2)
fn parse_keyboard_write<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, CslInstruction> {
    ("keyboard_write", ws1)
        .context(StrContext::Label("keyboard_write"))
        .parse_next(input)?;

    cut_err(
        (
            dec_uint::<_, u8, _>,
            preceded((ws0, ',', ws0), dec_uint::<_, u8, _>),
            preceded((ws0, ',', ws0), dec_uint::<_, u8, _>),
            preceded((ws0, ',', ws0), dec_uint::<_, u8, _>),
            preceded((ws0, ',', ws0), dec_uint::<_, u8, _>),
            preceded((ws0, ',', ws0), dec_uint::<_, u8, _>),
            preceded((ws0, ',', ws0), dec_uint::<_, u8, _>),
            preceded((ws0, ',', ws0), dec_uint::<_, u8, _>),
            preceded((ws0, ',', ws0), dec_uint::<_, u8, _>),
            preceded((ws0, ',', ws0), dec_uint::<_, u8, _>)
        )
            .context(StrContext::Label(
                "10 comma-separated byte values (row0 to row9)"
            ))
    )
    .map(|(r0, r1, r2, r3, r4, r5, r6, r7, r8, r9)| {
        CslInstruction::KeyboardWrite([r0, r1, r2, r3, r4, r5, r6, r7, r8, r9])
    })
    .parse_next(input)
}

/// Parse wait instruction
fn parse_wait<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, CslInstruction> {
    ("wait", ws1)
        .context(StrContext::Label("wait"))
        .parse_next(input)?;

    cut_err(dec_uint::<_, u64, _>.context(StrContext::Label("wait duration (number of cycles)")))
        .map(CslInstruction::Wait)
        .parse_next(input)
}

/// Parse wait_driveonoff instruction
fn parse_wait_driveonoff<'a>(
    input: &mut LocatingSlice<&'a str>
) -> ParseResult<'a, CslInstruction> {
    ("wait_driveonoff", ws0)
        .context(StrContext::Label("wait_driveonoff"))
        .parse_next(input)?;

    cut_err(
        opt(preceded(ws1, dec_uint::<_, u32, _>))
            .context(StrContext::Label("optional count (default 1)"))
    )
    .map(|n| CslInstruction::WaitDriveOnOff(n.unwrap_or(1)))
    .parse_next(input)
}

/// Parse wait_vsyncoffon instruction
fn parse_wait_vsyncoffon<'a>(
    input: &mut LocatingSlice<&'a str>
) -> ParseResult<'a, CslInstruction> {
    "wait_vsyncoffon"
        .value(CslInstruction::WaitVsyncOffOn)
        .context(StrContext::Label("wait_vsyncoffon"))
        .parse_next(input)
}

/// Parse wait_ssm0000 instruction
fn parse_wait_ssm0000<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, CslInstruction> {
    "wait_ssm0000"
        .value(CslInstruction::WaitSsm0000)
        .context(StrContext::Label("wait_ssm0000"))
        .parse_next(input)
}

/// Parse screenshot_name instruction
fn parse_screenshot_name<'a>(
    input: &mut LocatingSlice<&'a str>
) -> ParseResult<'a, CslInstruction> {
    ("screenshot_name", ws1)
        .context(StrContext::Label("screenshot_name"))
        .parse_next(input)?;

    cut_err(quoted_path.context(StrContext::Label("screenshot file path (quoted)")))
        .map(CslInstruction::ScreenshotName)
        .parse_next(input)
}

/// Parse screenshot_dir instruction
fn parse_screenshot_dir<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, CslInstruction> {
    ("screenshot_dir", ws1)
        .context(StrContext::Label("screenshot_dir"))
        .parse_next(input)?;

    cut_err(quoted_path.context(StrContext::Label("screenshot directory path (quoted)")))
        .map(CslInstruction::ScreenshotDir)
        .parse_next(input)
}

/// Parse screenshot instruction
fn parse_screenshot<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, CslInstruction> {
    ("screenshot", ws0)
        .context(StrContext::Label("screenshot"))
        .parse_next(input)?;

    cut_err(opt(preceded(ws1, "vsync")).context(StrContext::Label("optional 'vsync' flag")))
        .map(|vsync| {
            CslInstruction::Screenshot {
                wait_vsync: vsync.is_some()
            }
        })
        .parse_next(input)
}

/// Parse snapshot_name instruction
fn parse_snapshot_name<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, CslInstruction> {
    ("snapshot_name", ws1)
        .context(StrContext::Label("snapshot_name"))
        .parse_next(input)?;

    cut_err(quoted_path.context(StrContext::Label("snapshot file path (quoted)")))
        .map(CslInstruction::SnapshotName)
        .parse_next(input)
}

/// Parse snapshot instruction
fn parse_snapshot<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, CslInstruction> {
    ("snapshot", ws0)
        .context(StrContext::Label("snapshot"))
        .parse_next(input)?;

    cut_err(opt(preceded(ws1, "vsync")).context(StrContext::Label("optional 'vsync' flag")))
        .map(|vsync| {
            CslInstruction::Snapshot {
                wait_vsync: vsync.is_some()
            }
        })
        .parse_next(input)
}

/// Parse snapshot version
fn parse_snapshot_version_num<'a>(
    input: &mut LocatingSlice<&'a str>
) -> ParseResult<'a, SnapshotVersion> {
    alt((
        "1".value(SnapshotVersion::V1),
        "2".value(SnapshotVersion::V2),
        "3".value(SnapshotVersion::V3)
    ))
    .context(StrContext::Label("Snapshot version"))
    .parse_next(input)
}

/// Parse snapshot_version instruction
fn parse_snapshot_version<'a>(
    input: &mut LocatingSlice<&'a str>
) -> ParseResult<'a, CslInstruction> {
    ("snapshot_version", ws1)
        .context(StrContext::Label("snapshot_version"))
        .parse_next(input)?;

    cut_err(parse_snapshot_version_num.context(StrContext::Label("snapshot version (1, 2, or 3)")))
        .map(CslInstruction::SnapshotVersion)
        .parse_next(input)
}

/// Parse csl_load instruction
fn parse_csl_load<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, CslInstruction> {
    (alt(("csl_load", "cls_load")), ws1) // XXX cls_load is kept for compatability with wrong shaker files
        .context(StrContext::Label("csl_load"))
        .parse_next(input)?;

    cut_err(quoted_path.context(StrContext::Label("CSL script file path (quoted)")))
        .map(CslInstruction::CslLoad)
        .parse_next(input)
}

/// Parse empty line (whitespace + line ending)
fn parse_empty_line<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, CslInstruction> {
    terminated(ws0, line_ending)
        .value(CslInstruction::Empty)
        .parse_next(input)
}

/// Parse any CSL instruction
pub fn parse_instruction<'a>(
    input: &mut LocatingSlice<&'a str>
) -> ParseResult<'a, CslInstruction> {
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
                parse_key_output
            )),
            alt((
                parse_key_from_file,
                parse_keyboard_write,
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
pub fn parse_line<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, CslInstruction> {
    alt((
        parse_empty_line,
        terminated(
            (parse_instruction, inline_comment).map(|(instruction, comment)| {
                match comment {
                    Some(comment_text) => {
                        CslInstruction::InstructionWithComment(Box::new(instruction), comment_text)
                    },
                    None => instruction
                }
            }),
            opt(line_ending) // Make line ending optional for last line
        )
    ))
    .parse_next(input)
}

/// Parse a complete CSL script
pub fn parse_csl_script<'a>(input: &mut LocatingSlice<&'a str>) -> ParseResult<'a, CslScript> {
    use cpclib_common::winnow::error::ErrMode;

    // Use the builder pattern to construct and validate the script as we parse
    let mut builder = CslScriptBuilder::new();

    // Parse lines one by one, adding them to the builder for immediate validation
    loop {
        // Skip whitespace/newlines
        let _ = take_while(0.., [' ', '\t', '\n', '\r']).parse_next(input)?;

        // Check if we've reached end of input
        if input.is_empty() {
            break;
        }

        // Parse next instruction
        let instruction = parse_line(input)?;

        // Add instruction to builder with validation
        builder = builder
            .with_instruction(instruction)
            .map_err(|_| ErrMode::Cut(ContextError::new()))?;
    }

    // Build final script
    builder
        .build()
        .map_err(|_| ErrMode::Cut(ContextError::new()))
}

/// Parse a CSL script from a string with enhanced error reporting
pub fn parse_csl_with_rich_errors(
    input: &str,
    filename: Option<String>
) -> Result<CslScript, crate::error::CslError> {
    use crate::error::CslError;

    let mut located_input = LocatingSlice::new(input);

    // Parse with builder to catch validation errors
    let mut builder = CslScriptBuilder::new();
    loop {
        // Skip whitespace/newlines
        let _ = take_while(0.., [' ', '\t', '\n', '\r'])
            .parse_next(&mut located_input)
            .map_err(|_e| {
                convert_parse_error_to_csl_error(input, &located_input, _e, filename.clone())
            })?;

        // Check if we've reached end of input
        if located_input.is_empty() {
            break;
        }

        // Get current offset before parsing
        let offset_before = input.len() - located_input.len();

        // Parse next instruction
        let instruction = parse_line(&mut located_input).map_err(|e| {
            convert_parse_error_to_csl_error(input, &located_input, e, filename.clone())
        })?;

        // Add instruction to builder with validation - capture validation errors
        builder = builder
            .with_instruction(instruction)
            .map_err(|validation_err| {
                // Create a rich error for validation failures
                let span = offset_before..offset_before.saturating_add(1);
                let mut error = CslError::new(input.to_string(), span, validation_err);
                if let Some(fname) = filename.clone() {
                    error = error.with_filename(fname);
                }
                error
            })?;
    }

    // Build final script
    builder.build().map_err(|validation_err| {
        // Create error for build-time validation failures
        let span = 0..1;
        let mut error = CslError::new(input.to_string(), span, validation_err);
        if let Some(fname) = filename.clone() {
            error = error.with_filename(fname);
        }
        error
    })
}

// Helper to convert parse errors to CSL errors
fn convert_parse_error_to_csl_error(
    input: &str,
    located_input: &LocatingSlice<&str>,
    e: cpclib_common::winnow::error::ErrMode<ContextError<StrContext>>,
    filename: Option<String>
) -> crate::error::CslError {
    use cpclib_common::winnow::error::ErrMode;

    use crate::error::{CslError, suggest_instruction};

    let offset = input.len().saturating_sub(located_input.len());
    let span = offset..offset.saturating_add(1);

    // Try to extract a meaningful error message from the context
    let mut message = "Parse error".to_string();
    let mut notes = Vec::new();

    let inner = match e {
        ErrMode::Incomplete(_) => None,
        ErrMode::Backtrack(err) | ErrMode::Cut(err) => Some(err)
    };

    if let Some(ctx_error) = inner {
        // Try to extract context information
        let contexts: Vec<_> = ctx_error.context().collect();
        if !contexts.is_empty()
            && let Some(StrContext::Label(label)) = contexts.last()
        {
            message = format!("Invalid CSL syntax: expected {}", label);
        }
    }

    // Check if this looks like an instruction name error
    if offset < input.len() {
        let remaining = &input[offset..];
        if let Some(word_end) = remaining.find(|c: char| c.is_whitespace() || c == '\n') {
            let word = &remaining[..word_end];
            if let Some(suggestion) = suggest_instruction(word) {
                notes.push(format!("Did you mean '{}'?", suggestion));
            }
        }
    }

    let mut error = CslError::new(input.to_string(), span, message);

    if let Some(fname) = filename {
        error = error.with_filename(fname.clone());
    }

    for note in notes {
        error = error.with_note(note);
    }

    error
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function for tests that wraps input in LocatingSlice
    fn parse_test<'a, O>(
        mut parser: impl ModalParser<LocatingSlice<&'a str>, O, ContextError<StrContext>>,
        input: &'a str
    ) -> Result<O, ContextError<StrContext>> {
        let mut located_input = LocatingSlice::new(input);
        parser.parse_next(&mut located_input).map_err(|e| {
            match e.into_inner() {
                Ok(inner) => inner,
                Err(_) => ContextError::new()
            }
        })
    }

    #[test]
    fn test_parse_comment() {
        let result = parse_test(parse_line, "; This is a comment\n");
        assert!(result.is_ok());
        if let Ok(CslInstruction::Comment(text)) = result {
            assert_eq!(text, " This is a comment");
        }
    }

    #[test]
    fn test_parse_csl_version() {
        let result = parse_test(parse_line, "csl_version 1.1\n");
        assert!(result.is_ok());
        if let Ok(CslInstruction::CslVersion(v)) = result {
            assert_eq!(v.major, 1);
            assert_eq!(v.minor, 1);
        }
    }

    #[test]
    fn test_parse_reset() {
        let result = parse_test(parse_line, "reset soft\n");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), CslInstruction::Reset(ResetType::Soft));

        let result = parse_test(parse_line, "reset\n");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), CslInstruction::Reset(ResetType::Hard));
    }

    #[test]
    fn test_parse_disk_insert() {
        let result = parse_test(parse_line, "disk_insert 'SHAKER25.DSK'\n");
        assert!(result.is_ok());
        if let Ok(CslInstruction::DiskInsert { drive, filename }) = result {
            assert_eq!(drive, Drive::A);
            assert_eq!(filename, "SHAKER25.DSK");
        }

        let result = parse_test(parse_line, "disk_insert B 'AMAZING.DSK'\n");
        assert!(result.is_ok());
        if let Ok(CslInstruction::DiskInsert { drive, filename }) = result {
            assert_eq!(drive, Drive::B);
            assert_eq!(filename, "AMAZING.DSK");
        }
    }

    #[test]
    fn test_parse_key_output() {
        let result = parse_test(parse_line, "key_output 'RUN \"SHAKE25A\"\\(RET)'\n");
        assert!(result.is_ok());
        if let Ok(CslInstruction::KeyOutput(key_output)) = result {
            assert_eq!(key_output.elements().len(), 15); // R U N space " S H A K E 2 5 A " \(RET)
        }
    }

    #[test]
    fn test_parse_wait() {
        let result = parse_test(parse_line, "wait 1300455\n");
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
        let result = parse_csl_with_rich_errors(script, None);
        assert!(result.is_ok());
        let script = result.unwrap();
        assert!(script.len() > 0);
    }

    #[test]
    fn test_parse_windows_line_endings() {
        let input = ";comment\r\ncsl_version 1.0\r\nreset\r\nwait 1000\r\n";
        let result = parse_csl_with_rich_errors(input, None);
        assert!(
            result.is_ok(),
            "Failed to parse with Windows line endings: {:?}",
            result
        );
        let script = result.unwrap();
        assert_eq!(script.len(), 4);
    }

    #[test]
    fn test_parse_with_trailing_content() {
        let input = "csl_version 1.0\nreset\n;";
        let result = parse_csl_with_rich_errors(input, None);
        assert!(
            result.is_ok(),
            "Failed to parse with trailing semicolon: {:?}",
            result
        );
    }

    #[test]
    fn test_parse_inline_comment() {
        let input = "wait 800000\t\t\t; fin affichage 1er ecran\n";
        let result = parse_test(parse_line, input);
        assert!(
            result.is_ok(),
            "Failed to parse inline comment: {:?}",
            result
        );

        match result.unwrap() {
            CslInstruction::InstructionWithComment(boxed_instruction, comment) => {
                assert!(matches!(*boxed_instruction, CslInstruction::Wait(800000)));
                assert_eq!(comment, " fin affichage 1er ecran");
            },
            other => panic!("Expected InstructionWithComment, got {:?}", other)
        }
    }

    #[test]
    fn test_parse_trailing_space() {
        let input = "wait 3000000 \n";
        let mut input_mut = LocatingSlice::new(input);
        let result = parse_line.parse_next(&mut input_mut);
        assert!(
            result.is_ok(),
            "Failed to parse with trailing space: {:?}",
            result
        );
        // Verify no comment wrapper when there's no comment
        assert!(matches!(result.unwrap(), CslInstruction::Wait(3000000)));
    }

    #[test]
    fn test_parse_key_output_space() {
        let input = "key_output ' '\n";
        let result = parse_test(parse_line, input);
        assert!(
            result.is_ok(),
            "Failed to parse key_output with space: {:?}",
            result
        );
    }

    #[test]
    fn test_parse_empty_line() {
        let result = parse_test(parse_line, "\n");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), CslInstruction::Empty);
    }

    #[test]
    fn test_parse_instruction_with_and_without_comment() {
        // Test instruction without comment - should NOT be wrapped
        let input_no_comment = "wait 1000\n";
        let result_no_comment = parse_test(parse_line, input_no_comment);
        assert!(result_no_comment.is_ok());
        assert!(matches!(
            result_no_comment.unwrap(),
            CslInstruction::Wait(1000)
        ));

        // Test instruction with comment - should be wrapped
        let input_with_comment = "wait 2000 ; a comment\n";
        let result_with_comment = parse_test(parse_line, input_with_comment);
        assert!(result_with_comment.is_ok());
        match result_with_comment.unwrap() {
            CslInstruction::InstructionWithComment(boxed, comment) => {
                assert!(matches!(*boxed, CslInstruction::Wait(2000)));
                assert_eq!(comment, " a comment");
            },
            other => panic!("Expected InstructionWithComment, got {:?}", other)
        }
    }

    #[test]
    fn test_parse_rom_config() {
        let result = parse_test(parse_line, "rom_config U 7 'Amsdos.rom'\n");
        assert!(result.is_ok());
        if let Ok(CslInstruction::RomConfig(config)) = result {
            assert_eq!(config.rom_type, RomType::Upper);
            assert_eq!(config.num, 7);
            assert_eq!(config.filename, "Amsdos.rom");
        }
    }

    #[test]
    fn test_parse_csl_with_rich_errors_invalid_instruction() {
        // Test 1: Malformed quoted string - should NOT suggest instruction name
        let input = "disk_insert 'missing_end_quote\n";
        let result = parse_csl_with_rich_errors(input, Some("test.csl".to_string()));
        assert!(
            result.is_err(),
            "Expected error for malformed quoted string"
        );

        let error = result.unwrap_err();
        assert!(!error.source.is_empty());
        assert_eq!(error.filename, Some("test.csl".to_string()));
        // Should NOT have "Did you mean" since "disk_insert" is correct
        let formatted = error.format_error();
        assert!(
            !formatted.contains("Did you mean"),
            "Should not suggest instruction name when instruction is correct: {}",
            formatted
        );

        // Test 2: Incomplete instruction - should NOT suggest instruction name
        let input2 = "disk_insert ";
        let result2 = parse_csl_with_rich_errors(input2, Some("test2.csl".to_string()));
        assert!(
            result2.is_err(),
            "Expected error for incomplete instruction"
        );

        let error2 = result2.unwrap_err();
        assert!(!error2.source.is_empty());
        assert_eq!(error2.filename, Some("test2.csl".to_string()));
        let formatted2 = error2.format_error();
        assert!(
            !formatted2.contains("Did you mean"),
            "Should not suggest instruction name when instruction is correct: {}",
            formatted2
        );

        // Test 3: Misspelled instruction - SHOULD suggest correct name
        let input3 = "disk_inser 'test.dsk'\n";
        let result3 = parse_csl_with_rich_errors(input3, Some("test3.csl".to_string()));
        assert!(
            result3.is_err(),
            "Expected error for misspelled instruction"
        );

        let error3 = result3.unwrap_err();
        let formatted3 = error3.format_error();
        assert!(
            formatted3.contains("Did you mean 'disk_insert'?"),
            "Should suggest correct instruction name for typo: {}",
            formatted3
        );
    }

    #[test]
    fn test_parse_csl_with_rich_errors_valid_script() {
        let input = "csl_version 1.1\nreset\nwait 1000\n";
        let result = parse_csl_with_rich_errors(input, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_csl_with_rich_errors_version_compatibility() {
        // Test v1.0 with v1.2 feature - should report error
        let input =
            "csl_version 1.0\nreset\nkeyboard_write 255,255,255,255,255,255,239,255,255,255\n";
        let result = parse_csl_with_rich_errors(input, Some("test_v10_v12.csl".to_string()));

        assert!(result.is_err(), "Expected error for v1.0 with v1.2 feature");

        let error = result.unwrap_err();
        assert_eq!(error.filename, Some("test_v10_v12.csl".to_string()));
        assert!(!error.source.is_empty());

        let formatted = error.format_error();
        // Should mention the incompatibility
        assert!(
            formatted.contains("1.2") || formatted.contains("keyboard_write"),
            "Error should mention version incompatibility: {}",
            formatted
        );

        // Test v1.0 with v1.1 feature
        let input2 = "csl_version 1.0\ngate_array 40010\n";
        let result2 = parse_csl_with_rich_errors(input2, Some("test_v10_v11.csl".to_string()));

        assert!(
            result2.is_err(),
            "Expected error for v1.0 with v1.1 feature"
        );

        let error2 = result2.unwrap_err();
        assert_eq!(error2.filename, Some("test_v10_v11.csl".to_string()));

        let formatted2 = error2.format_error();
        eprintln!("{}", formatted2);
        assert!(
            formatted2.contains("1.1") || formatted2.contains("gate_array"),
            "Error should mention version incompatibility: {}",
            formatted2
        );

        // Test v1.1 with v1.2 feature
        let input3 =
            "csl_version 1.1\nreset\nkeyboard_write 255,255,255,255,255,255,239,255,255,255\n";
        let result3 = parse_csl_with_rich_errors(input3, Some("test_v11_v12.csl".to_string()));

        assert!(
            result3.is_err(),
            "Expected error for v1.1 with v1.2 feature"
        );

        // Test valid v1.2 with v1.2 feature - should succeed
        let input4 = "csl_version 1.2\nkeyboard_write 255,255,255,255,255,255,239,255,255,255\n";
        let result4 = parse_csl_with_rich_errors(input4, None);

        assert!(result4.is_ok(), "v1.2 with v1.2 feature should succeed");
    }
}
