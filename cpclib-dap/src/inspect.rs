//! Making the emulator's answers readable.
//!
//! The emulator reports what a Z80 knows: sixteen-bit registers as hex, and
//! disassembled instructions as addresses. Both are correct and both are hard
//! to work with - `AF = 0x4A45` does not say which flags are set, and an
//! address does not say which line of yours it came from. Everything here turns
//! one into the other on the way past.

use cpclib_image::color::AmstradColor;
use cpclib_image::image::{ColorMatrix, Mode};
use cpclib_image::ink::Ink;
use cpclib_image::palette::Palette;
use cpclib_project::srcmap::{SourceLocation, SourceMap};
use serde_json::{Value, json};

/// Variable references we answer ourselves, chosen far from the emulator's own
/// (which are small and derived from its stop epoch).
pub const CRTC_REFERENCE: i64 = 0x7C00_0001;
pub const GATE_ARRAY_REFERENCE: i64 = 0x7C00_0002;
pub const PSG_REFERENCE: i64 = 0x7C00_0004;
pub const PPI_REFERENCE: i64 = 0x7C00_0005;
pub const DISC_REFERENCE: i64 = 0x7C00_0006;

/// The Z80 flag register, bit by bit.
///
/// Bits 3 and 5 have no defined meaning - they are whatever the last operation
/// left there - so they are shown but not named, rather than invented.
const FLAGS: [(u8, &str, &str); 8] = [
    (7, "S", "sign (result was negative)"),
    (6, "Z", "zero"),
    (5, "5", "undocumented bit 5"),
    (4, "H", "half carry"),
    (3, "3", "undocumented bit 3"),
    (2, "P/V", "parity or overflow"),
    (1, "N", "last operation was a subtraction"),
    (0, "C", "carry")
];

/// A compact rendering of `F`: the set flags, in bit order.
///
/// `SZ-H-P-C` reads at a glance; `0x4A` does not.
pub fn describe_flags(f: u8) -> String {
    FLAGS
        .iter()
        .map(|(bit, name, _)| {
            if f & (1 << bit) != 0 {
                (*name).to_string()
            }
            else {
                "-".to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("")
}

/// One child variable per flag, so the pane can be expanded.
pub fn flag_variables(f: u8) -> Vec<Value> {
    FLAGS
        .iter()
        .map(|(bit, name, meaning)| {
            let set = f & (1 << bit) != 0;
            json!({
                "name": *name,
                "value": if set { "1" } else { "0" },
                "type": *meaning,
                "variablesReference": 0
            })
        })
        .collect()
}

/// The `F` byte of a register list, if it reports one.
pub fn flags_of(variables: &[Value]) -> Option<u8> {
    let af = variables
        .iter()
        .find(|v| v.get("name").and_then(Value::as_str) == Some("AF"))
        .and_then(|v| v.get("value").and_then(Value::as_str))
        .and_then(parse_hex_value)?;
    Some((af & 0xFF) as u8)
}

/// Add a decoded `F` to a register list the emulator answered with.
///
/// `AF` is one 16-bit value there; the flags live in its low byte and are the
/// half people actually read while stepping.
pub fn annotate_registers(variables: &mut Vec<Value>, flags_reference: i64) {
    annotate_registers_with(variables, flags_reference, None)
}

/// How far past a label a register may point and still be described by it.
///
/// A pointer three bytes into `screen_buffer` is in `screen_buffer`. A pointer
/// two kilobytes past the last label is not in it, and saying otherwise is
/// worse than saying nothing.
const LABEL_WINDOW: u32 = 0x100;

/// As [`annotate_registers`], plus the label each pointer register points at.
///
/// `HL = 0xC000` is a number; `HL = 0xC000 (screen_buffer)` is an answer. The
/// symbol table is already here, from the same build as the addresses.
pub fn annotate_registers_with(
    variables: &mut Vec<Value>,
    flags_reference: i64,
    map: Option<&SourceMap>
) {
    if let Some(map) = map {
        for variable in variables.iter_mut() {
            // Only the registers that hold addresses. `AF` is a value and a
            // flag byte, and labelling it would be noise on every step.
            let name = variable
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if !matches!(name, "BC" | "DE" | "HL" | "IX" | "IY" | "SP" | "PC") {
                continue;
            }
            let Some(address) = variable
                .get("value")
                .and_then(Value::as_str)
                .and_then(parse_hex_value)
            else {
                continue;
            };
            let described = match map.symbol_at(address) {
                Some(symbol) => symbol.to_string(),
                None => {
                    match map.symbol_near(address, LABEL_WINDOW) {
                        Some((symbol, 0)) => symbol.to_string(),
                        Some((symbol, offset)) => format!("{symbol}+{offset}"),
                        None => continue
                    }
                },
            };
            if let Some(value) = variable.get("value").and_then(Value::as_str) {
                let value = format!("{value} ({described})");
                variable["value"] = json!(value);
            }
        }
    }
    annotate_flags(variables, flags_reference)
}

fn annotate_flags(variables: &mut Vec<Value>, flags_reference: i64) {
    let af = variables
        .iter()
        .find(|v| v.get("name").and_then(Value::as_str) == Some("AF"))
        .and_then(|v| v.get("value").and_then(Value::as_str))
        .and_then(parse_hex_value);

    let Some(af) = af
    else {
        return;
    };
    let f = (af & 0xFF) as u8;
    let a = ((af >> 8) & 0xFF) as u8;

    // Inserted right after AF, where someone reading the pane is already
    // looking.
    let position = variables
        .iter()
        .position(|v| v.get("name").and_then(Value::as_str) == Some("AF"))
        .map(|i| i + 1)
        .unwrap_or(variables.len());

    variables.insert(
        position,
        json!({
            "name": "F (flags)",
            "value": describe_flags(f),
            "type": "S Z 5 H 3 P/V N C",
            "variablesReference": flags_reference
        })
    );
    variables.insert(
        position,
        json!({
            "name": "A",
            "value": format!("0x{a:02X} ({a})"),
            "variablesReference": 0
        })
    );
}

fn parse_hex_value(raw: &str) -> Option<u32> {
    let trimmed = raw.trim();
    let digits = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .or_else(|| trimmed.strip_prefix('&'))?;
    u32::from_str_radix(digits, 16).ok()
}

/// An operand this pass could not name on its own: several labels share the
/// address, and nothing here has the evidence to prefer one.
///
/// `annotate_disassembly` writes its usual best guess anyway (today's
/// behaviour, unchanged), but also hands this back so a caller that *can*
/// read source text - `Session`, which owns the file cache `source_line`
/// needs - gets a chance to do better. Keeping that resolution outside this
/// function is not a style choice: `annotate_disassembly` takes `&SourceMap`
/// while disambiguating needs `&mut self` for `source_line`, and the two
/// borrows cannot both be live in one call.
pub(crate) struct AmbiguousOperand {
    pub index: usize,
    pub location: SourceLocation,
    /// Preference-sorted, as `SourceMap::symbols_at` returns them. Always
    /// more than one - a single candidate is not ambiguous and is handled
    /// inline instead of being reported here.
    pub candidates: Vec<String>
}

/// Put each disassembled instruction back on the line it came from.
///
/// This is what lets the disassembly view show your source beside the opcodes,
/// and what makes "which line is actually executing" answerable by reading
/// rather than by counting instructions.
///
/// Returns the operands where more than one label shared an address, so a
/// caller with access to source text can try to do better than the guess
/// written here - see [`AmbiguousOperand`].
///
/// `costs`, when given, is one entry per instruction, straight from
/// `Instruction::cost` - `None` at an index means that row is a `DB` the
/// data overlay wrote, not a decoded instruction, and its rendered text is
/// raw byte values rather than operands, so it is never scanned for embedded
/// addresses. Passing `costs: None` altogether (rather than a list) means no
/// cost information exists at all for this batch - the emulator-forwarded
/// `disassemble` path never runs it through `Instruction`/`overlay_data_rows`
/// in the first place - and every row is scanned exactly as before; that is
/// a different thing from "this row's cost is known to be absent" and must
/// not be treated the same way.
pub(crate) fn annotate_disassembly(
    instructions: &mut [Value],
    map: &SourceMap,
    page: Option<u8>,
    physical: Option<u32>,
    costs: Option<&[Option<usize>]>
) -> Vec<AmbiguousOperand> {
    let mut ambiguous = Vec::new();
    for (index, instruction) in instructions.iter_mut().enumerate() {
        let address = instruction
            .get("address")
            .and_then(Value::as_str)
            .and_then(crate::protocol::parse_address_reference);
        let Some(address) = address
        else {
            continue;
        };

        // An address belonging to no line stays bare: the view then shows the
        // instruction alone, which is the honest answer for firmware or data.
        //
        // In a banked program the page has to come from somewhere, since the
        // logical address alone is claimed by more than one; `page` is what the
        // bytes at `PC` turned out to match. `physical`, when the emulator named
        // its own banking, is finer still: a single-window remap (`C4`-`C7`)
        // changes which bank of one page is paged in at `&4000` without
        // changing the page, so `page` alone still leaves several rows of a
        // disassembly listing claiming each other's addresses - exactly the
        // fault this view exists to avoid. `physical` does not have that
        // ambiguity, so it is tried first and, on a hit, is the whole answer.
        //
        // Hoisted above the operand block below: disambiguating an operand
        // needs this row's own location to read the source line from.
        let located = physical
            .and_then(|physical| {
                u16::try_from(address)
                    .ok()
                    .map(|address| (physical & !0x3FFF) | u32::from(address & 0x3FFF))
            })
            .and_then(|physical| map.location_at_physical(physical))
            .or_else(|| {
                page.and_then(|page| u16::try_from(address).ok().map(|address| (page, address)))
                    .and_then(|(page, address)| map.location_at_long(page, address))
            })
            .or_else(|| map.location_at(address));

        // The addresses in the operands, named. `CALL 0xBB5A` is a routine you
        // have to look up; `CALL 0xBB5A ; TXT_OUTPUT` is one you can read. The
        // same for your own labels - a jump target is a name in the source and
        // should be one here too.
        //
        // A `DB` row is the exception: its text is raw byte values, not a
        // decoded operand, so a zero byte must not be read as a reference to
        // whatever label happens to sit at address 0. `costs[index] ==
        // Some(None)` is that row saying so of itself, via the same signal
        // `overlay_data_rows` already sets - see the doc comment above.
        let is_data_row = matches!(costs.and_then(|c| c.get(index).copied()), Some(None));
        if !is_data_row && let Some(text) = instruction.get("instruction").and_then(Value::as_str) {
            let named = name_operand_addresses(text, map, &located, index, &mut ambiguous);
            if !named.is_empty() {
                instruction["symbols"] = json!(named);
            }
        }

        // A label at this address, shown as a heading in the view. Worth more
        // here than anywhere else: a screenful of macro-generated opcodes all
        // carry the same source line, and the labels are the only thing that
        // says where one thing ends and the next begins.
        //
        // No source-line evidence exists for a heading - nothing "mentions"
        // it, the way a call site mentions its target - so an ambiguity here
        // is shown rather than silently resolved to a guess.
        let symbols = map.symbols_at(address);
        if let Some(symbol) = symbols.first() {
            instruction["symbol"] = json!(symbol);
            if symbols.len() > 1 {
                instruction["symbolAlternatives"] = json!(symbols[1..]);
            }
        }

        let Some(location) = located
        else {
            continue;
        };
        instruction["location"] = json!({
            "name": location
                .file
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            "path": location.file.to_string_lossy()
        });
        instruction["line"] = json!(location.line);
        if location.column_end > location.column {
            instruction["column"] = json!(location.column.max(1));
            instruction["endLine"] = json!(location.line);
            instruction["endColumn"] = json!(location.column_end);
        }
    }
    ambiguous
}

/// The scopes we add beside the emulator's registers.
///
/// The CPC's behaviour is decided as much by the CRTC and the Gate Array as by
/// the Z80, and reading a demo without them is guesswork. The emulator core
/// does not expose them yet, so they are presented as present-but-unavailable
/// rather than omitted: the shape is settled, and filling them in later is a
/// change in one place.
pub fn extra_scopes() -> Vec<Value> {
    // `expensive` is the honest answer and a load-bearing one: none of this is
    // on the emulator's debug API, so reading it means saving a whole snapshot
    // and parsing it. The flag is what stops the editor doing that on every
    // step and only does it when you expand the scope.
    vec![
        json!({
            "name": "CRTC",
            "variablesReference": CRTC_REFERENCE,
            "expensive": true,
            "presentationHint": "registers"
        }),
        json!({
            "name": "Gate Array",
            "variablesReference": GATE_ARRAY_REFERENCE,
            "expensive": true,
            "presentationHint": "registers"
        }),
        json!({
            "name": "PSG",
            "variablesReference": PSG_REFERENCE,
            "expensive": true,
            "presentationHint": "registers"
        }),
        json!({
            "name": "PPI",
            "variablesReference": PPI_REFERENCE,
            "expensive": true,
            "presentationHint": "registers"
        }),
        json!({
            "name": "Disc",
            "variablesReference": DISC_REFERENCE,
            "expensive": true,
            "presentationHint": "registers"
        }),
    ]
}

/// Whether `reference` is one of the chip scopes.
pub fn is_chip_scope(reference: i64) -> bool {
    matches!(
        reference,
        CRTC_REFERENCE | GATE_ARRAY_REFERENCE | PSG_REFERENCE | PPI_REFERENCE | DISC_REFERENCE
    )
}

/// Underline a name, so the one the hardware currently points at stands out.
///
/// A DAP variable is plain text - there is no styling to ask for - but a
/// combining low line under each character is text, and renders as an
/// underline wherever the pane's font supports it.
pub fn underlined(name: &str) -> String {
    let mut out = String::with_capacity(name.len() * 2);
    for character in name.chars() {
        out.push(character);
        out.push('\u{0332}');
    }
    out
}

/// The colour a Gate Array palette byte stands for.
///
/// Returns the nearest coloured square that exists as a character, plus the
/// exact RGB - the square is what you see at a glance while stepping, the hex
/// is what is actually true. `None` for a byte that is not a colour the Gate
/// Array can produce.
/// The nearest coloured square to an RGB value.
///
/// Nine squares is all the character set offers, so this is the nearest of them
/// rather than the colour itself - which is why the exact hex is printed beside
/// it wherever this is used.
pub fn swatch_for_rgb(r: u8, g: u8, b: u8) -> char {
    const SWATCHES: [(char, (u8, u8, u8)); 9] = [
        ('\u{2B1B}', (0, 0, 0)),
        ('\u{2B1C}', (255, 255, 255)),
        ('\u{1F7E5}', (255, 0, 0)),
        ('\u{1F7E9}', (0, 255, 0)),
        ('\u{1F7E6}', (0, 0, 255)),
        ('\u{1F7E8}', (255, 255, 0)),
        ('\u{1F7EA}', (255, 0, 255)),
        ('\u{1F7E7}', (255, 128, 0)),
        ('\u{1F7EB}', (128, 64, 0))
    ];
    SWATCHES
        .iter()
        .min_by_key(|(_, (sr, sg, sb))| {
            let d = |a: u8, b: u8| (a as i32 - b as i32).pow(2);
            d(r, *sr) + d(g, *sg) + d(b, *sb)
        })
        .map(|(square, _)| *square)
        .unwrap_or('\u{2B1B}')
}

/// How a pen reads: the colour it holds, by number and by the byte that sets it.
///
/// The RGB decides the swatch and is then dropped. What a demo coder needs from
/// a palette entry is which *ink* it is and what to write to `&7Fxx` for it -
/// the exact sRGB triple of ink 20 answers no question anyone was asking.
///
/// `None` for a byte that is not a colour the Gate Array can produce.
pub fn gate_array_pen(written: u8) -> Option<(String, String)> {
    // `gate_array_value()` is the byte a program *writes* - bit 6 already set -
    // and so is what a snapshot stores, so the comparison is against that form
    // rather than against a bare colour number.
    let ink = cpclib_image::ink::Ink::INKS
        .iter()
        .find(|ink| ink.gate_array_value() == written)?;
    let rgb = ink.color();
    let square = swatch_for_rgb(rgb[0], rgb[1], rgb[2]);
    Some((
        format!("{square} ink {} (GA 0x{written:02X})", ink.number()),
        format!("write 0x{written:02X} to &7Fxx for this colour")
    ))
}

/// The symbols any address-shaped operand in `text` could stand for.
///
/// Only exact matches: an operand that is a label's address is that label, and
/// an operand that is three bytes into one is left alone. A guess here would be
/// read as fact, and the whole point of this column is to be trusted.
///
/// When several labels share an operand's address, the top-preference one is
/// still written here - today's default, unchanged - but the ambiguity is
/// also recorded into `ambiguous` (when this row resolved to a source
/// `location`; with none, there is no line to disambiguate against, so the
/// guess stands as-is). `index` identifies this instruction within the slice
/// `annotate_disassembly` is walking, for the caller to write back into.
/// Whether a numeric operand of `text` is plausibly an address, rather than a
/// plain immediate that happens to equal one - `LD A,0` is not a reference to
/// whatever label a program happens to define at address 0, `LD HL,0` (a
/// buffer/table address, routinely) is.
fn looks_like_an_address_operand(text: &str, value: u32) -> bool {
    // Large enough that it is implausible as an 8-bit immediate, a bit
    // index (0-7) or a port number: address it is, whatever the instruction
    // turns out to be - a safety net for any shape not enumerated below.
    if value > 0xFF {
        return true;
    }
    let trimmed = text.trim_start();
    let mnemonic = trimmed
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_uppercase();
    // The whole point of these: their one numeric operand is where control
    // goes, or (RST) a fixed low address - never a plain immediate.
    if matches!(mnemonic.as_str(), "CALL" | "JP" | "JR" | "DJNZ" | "RST") {
        return true;
    }
    // `(nn)` - a memory reference, not the byte value living at a small
    // address. Z80 has no instruction mixing an indirect operand with an
    // unrelated bare-immediate one, so "this text has parentheses at all"
    // is enough without tracking which token sits inside them.
    if text.contains('(') {
        return true;
    }
    // `LD HL,0x4000` and its 16-bit-register siblings: routinely a
    // buffer/table address. The matching 8-bit forms (`LD A,0`, `LD B,5`...)
    // are not, and neither is any ALU immediate (`XOR 0`, `CP 10`...) or a
    // bit index (`BIT 3,A`) - both fall through to `false` below.
    if mnemonic == "LD" {
        // Safe to slice at byte 2: `mnemonic == "LD"` only holds when
        // `trimmed`'s first token really is two ASCII bytes.
        let destination = trimmed[2..]
            .split(',')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_uppercase();
        return matches!(destination.as_str(), "HL" | "DE" | "BC" | "IX" | "IY" | "SP" | "AF");
    }
    false
}

fn name_operand_addresses(
    text: &str,
    map: &SourceMap,
    location: &Option<SourceLocation>,
    index: usize,
    ambiguous: &mut Vec<AmbiguousOperand>
) -> Vec<String> {
    let mut named = Vec::new();
    for piece in text.split(|c: char| !c.is_ascii_alphanumeric() && c != 'x' && c != 'X') {
        let value = piece
            .strip_prefix("0x")
            .or_else(|| piece.strip_prefix("0X"))
            .and_then(|hex| u32::from_str_radix(hex, 16).ok())
            .or_else(|| {
                // Bare hex, as the disassembler writes it in some operands.
                (piece.len() >= 2 && piece.chars().all(|c| c.is_ascii_hexdigit()))
                    .then(|| u32::from_str_radix(piece, 16).ok())
                    .flatten()
            });
        let Some(value) = value
        else {
            continue;
        };
        if !looks_like_an_address_operand(text, value) {
            continue;
        }
        let candidates = map.symbols_at(value);
        let Some(symbol) = candidates.first()
        else {
            continue;
        };
        if named.iter().any(|n| n == symbol) {
            continue;
        }
        named.push(symbol.to_string());
        if candidates.len() > 1
            && let Some(location) = location
        {
            ambiguous.push(AmbiguousOperand {
                index,
                location: location.clone(),
                candidates: candidates.into_iter().map(str::to_owned).collect()
            });
        }
    }
    named
}

fn byte(name: &str, value: u8, meaning: &str) -> Value {
    json!({
        "name": name,
        "value": format!("0x{value:02X} ({value})"),
        "type": meaning,
        "variablesReference": 0
    })
}

/// A CRTC register combination known to misbehave on real hardware - not
/// merely unusual, one that visibly breaks (lost sync, a raster line the
/// wrong length).
#[derive(Debug)]
pub struct CrtcWarning {
    /// Every register the rule that raised this reads from - the CRTC pane
    /// highlights each of these, not just one.
    pub registers: &'static [&'static str],
    pub severity: CrtcSeverity,
    pub message: String
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrtcSeverity {
    /// Loses sync or otherwise stops the picture outright.
    Error,
    /// Runs, but not the way the source most likely intends.
    Warning
}

/// Rules that catch CRTC configurations known to misbehave on real hardware.
///
/// A `Vec` of independent checks on purpose, so the next rule is a new entry
/// here rather than a change to the two that exist - and both backends
/// already have the raw `R0..R17` bytes parsed by the time they build their
/// CRTC pane, which is what this takes.
pub fn validate_crtc(regs: &[u8]) -> Vec<CrtcWarning> {
    let mut out = Vec::new();
    if regs.len() < 4 {
        return out;
    }
    let r0 = u32::from(regs[0]);
    let r2 = u32::from(regs[2]);
    let r3_low = u32::from(regs[3]) & 0x0f;

    // CRTC type 2 loses horizontal sync unless the sync position plus the
    // HSYNC width stays *inside* the line total - the safe relationship is
    // R2+(R3&0x0f) < R0, so the warning is raised on its negation, not on
    // the relationship itself.
    if r2 + r3_low >= r0 {
        out.push(CrtcWarning {
            registers: &["R0", "R2", "R3"],
            severity: CrtcSeverity::Error,
            message: format!(
                "R2+(R3&0x0f) >= R0 ({r2}+{r3_low}={} >= {r0}): a CRTC type 2 loses horizontal \
                 sync with this combination",
                r2 + r3_low
            )
        });
    }

    // The raster-timing idiom (`defs 64 - duration(djnz $)-1`, NOP-budget
    // counting, every fixed timing loop this toolchain's demos rely on)
    // assumes a line is 64 NOPs long, which only holds at R0=63.
    if regs[0] != 63 {
        out.push(CrtcWarning {
            registers: &["R0"],
            severity: CrtcSeverity::Warning,
            message: format!(
                "R0={} (not 63): a raster line here is not 64 NOPs long, which most fixed-\
                 timing code in this toolchain assumes",
                regs[0]
            )
        });
    }

    out
}

/// Just the raw `R0..R17` bytes, for `validate_crtc` - `chip_variables`
/// already reads every one of these but returns them already formatted for
/// the pane.
pub fn crtc_registers(sna: &cpclib_sna::Snapshot) -> [u8; 18] {
    use cpclib_sna::SnapshotFlag as F;
    let get = |i: usize| -> u8 {
        match sna.get_value(&F::CRTC_REG(Some(i))) {
            cpclib_sna::FlagValue::Byte(v) => v,
            cpclib_sna::FlagValue::Word(v) => (v & 0xFF) as u8,
            _ => 0
        }
    };
    std::array::from_fn(get)
}

/// The same, from an AmspiritLite `/api/crtc` body (`crtc_pane`'s own `regs`
/// array).
pub fn crtc_registers_from_json(body: &Value) -> Option<[u8; 18]> {
    let regs = body.get("regs")?.as_array()?;
    let mut out = [0u8; 18];
    for (slot, value) in out.iter_mut().zip(regs.iter()) {
        *slot = value.as_u64().unwrap_or(0) as u8;
    }
    Some(out)
}

/// R12/R13 (display start address, high/low) as a byte address.
///
/// The 16K page (`R12`'s top 2 bits, 0/0x4000/0x8000/0xC000) and the
/// intra-page offset (`R12`'s low nibble + all of `R13`, a 12-bit value)
/// do **not** share one uniform shift - live-verified 2026-08-27 (see the
/// WinAPE-style screen viewer plan) against three independent data points:
/// two live AMSpiriT boot-state pokes (R12=0x30/R13=0x00 -> 0xC000,
/// R12=0x20/R13=0x00 -> 0x8000, both with a zero offset, which is why an
/// initial `<<2`-everything version of this function passed both) and a
/// real, already-scrolled BASIC snapshot's own CRTC state
/// (`cpclib-dap/tests/graphics/hello/snapshot.sna`, R12=0x32/R13=0xF0)
/// checked against a WinAPE capture of the exact same memory - the only
/// data point with a non-zero offset, and the `<<2`-everything formula
/// missed it by 1504 bytes (0xCBC0 instead of the correct 0xC5E0).
/// Reverse-engineered from that mismatch: the page contributes
/// `page * 0x4000`, but the 12-bit offset contributes only `offset * 2`,
/// not `offset * 4` - i.e. the CRTC's 14-bit MA counter's top 2 bits are
/// rerouted through separate page-select logic instead of sharing the
/// same address-line weighting the other 12 bits get. All three data
/// points confirm this corrected formula exactly.
pub fn crtc_screen_start_address(r12: u8, r13: u8) -> usize {
    let page = ((r12 >> 4) & 0x3) as usize;
    let offset = (((r12 & 0x0F) as usize) << 8) | r13 as usize;
    page * 0x4000 + offset * 2
}

/// `-sv`'s own defaults for width and "lines per character row", read
/// straight off the live CRTC rather than assumed as a fixed 80x8: `R1`
/// (horizontal displayed) counts *character* positions, and the CPC's
/// screen memory is addressed in word pairs - one address step per
/// character, two bytes per step - so the byte width of a line is `R1 * 2`.
/// `R9` (maximum raster address) is the last raster line of a character
/// row, 0-based, so a row is `R9 + 1` lines tall; that is also the modulus
/// `ColorMatrix::from_screen_at`'s own interleaved addressing needs (see its
/// `lines_per_char_row` parameter's doc comment) - the standard `R9 = 7`
/// (8 lines) was hard-coded everywhere until this existed only because
/// every fixture this crate had ever seen used it.
///
/// `R1 == 0` is treated as "unknown" (an all-zero/never-answered register
/// bank, not a real CRTC state actively displaying anything) and falls back
/// to [`DEFAULT_SCREEN_WIDTH`]; `R9` has no such fallback; needed since
/// `R9 = 0` (one raster line per row) is unusual but real.
pub fn crtc_screen_defaults(regs: &[u8; 18]) -> (usize, usize) {
    let width = (regs[1] as usize) * 2;
    let width = if width == 0 { DEFAULT_SCREEN_WIDTH } else { width };
    let lines_per_char_row = regs[9] as usize + 1;
    (width, lines_per_char_row)
}

/// `-sv`'s 6th argument, parsed: a comma-separated list of up to 16 CPC
/// hardware ink numbers (0-26), one per pen starting at pen 0 - an empty
/// entry between commas (`",,5,"`) means "no override for this pen, keep
/// whatever the live Gate Array put there", the same way every other
/// unset `-sv` argument already means "use the live default". Not sent at
/// all (an empty slice) is exactly the same as every entry being empty -
/// nothing overridden.
pub fn parse_palette_override(text: &str) -> Vec<Option<Ink>> {
    text.split(',')
        .take(16)
        .map(|token| {
            token
                .trim()
                .parse::<usize>()
                .ok()
                .and_then(|index| Ink::INKS.get(index))
                .copied()
        })
        .collect()
}

/// The char-row-height argument, resolved: real WinAPE's own memory-
/// browsing tool this view is modelled on treats it as `R9 + 1` for the
/// address interleaving itself, not only as a display value - the CRTC's
/// own live value is only the *default*, not the only value this view can
/// ever use. Reported live: a taller/shorter row than the live CRTC's own
/// is exactly the point of browsing raw memory this way (finding a
/// repeating structure whose real height is not yet known), so leaving the
/// address math pinned to the live CRTC while only a cosmetic value moved
/// missed the entire feature. Unset or an explicit `0` both mean "use the
/// live CRTC value".
pub fn resolve_char_row_height(row_height_override: Option<usize>, live_lines_per_char_row: usize) -> usize {
    match row_height_override {
        Some(n) if n > 0 => n,
        _ => live_lines_per_char_row
    }
}

/// A `Palette<Ink>` built from 17 raw Gate Array pen values (pens 0-15, then
/// the border at index 16) - the same shape `chip_variables`'s
/// `GATE_ARRAY_REFERENCE` branch already reads via `GA_PAL(Some(index))`.
/// `Ink::from_gate_array_color_number` (like `gate_array_pen`, its
/// Variables-pane sibling) matches against the full "written" byte a
/// program writes to `&7Fxx` - bit 6 set, not a bare 5-bit colour number -
/// so the raw stored value is masked to 5 bits and `0x40` reattached first,
/// exactly as `gate_array_pen`'s own doc comment explains.
pub fn palette_from_raw_ga(pens: &[u8; 17]) -> Palette<Ink> {
    let written = |raw: u8| 0x40 | (raw & 0x1F);
    let mut palette = Palette::new();
    for (i, &raw) in pens.iter().enumerate().take(16) {
        palette.set(i as u8, Ink::from_gate_array_color_number(written(raw)));
    }
    palette.set_border(Ink::from_gate_array_color_number(written(pens[16])));
    palette
}

/// Screen mode + palette, from an AmspiritLite `/api/ga` body
/// (`gate_array_pane`'s own `mode`/`ink_idx`/`border_idx` fields).
pub fn mode_and_palette_from_ga_json(body: &Value) -> Option<(u8, Palette<Ink>)> {
    let mode = (body.get("mode").and_then(Value::as_u64)? as u8) & 0b11;
    let inks = body.get("ink_idx")?.as_array()?;
    let mut pens = [0u8; 17];
    for (i, slot) in pens.iter_mut().take(16).enumerate() {
        *slot = inks.get(i).and_then(Value::as_u64).unwrap_or(0) as u8;
    }
    pens[16] = body.get("border_idx").and_then(Value::as_u64).unwrap_or(0) as u8;
    Some((mode, palette_from_raw_ga(&pens)))
}

/// The screen mode and palette, read straight from a `.sna` snapshot - same
/// source `chip_variables`'s `GATE_ARRAY_REFERENCE` branch already reads
/// (`GA_ROMCFG` for the mode, `GA_PAL(Some(index))` per pen), packaged for
/// the screen viewer instead of a Variables-pane row list.
pub fn mode_and_palette_from_snapshot(sna: &cpclib_sna::Snapshot) -> (u8, Palette<Ink>) {
    use cpclib_sna::SnapshotFlag as F;
    let get = |flag: F| -> u8 {
        match sna.get_value(&flag) {
            cpclib_sna::FlagValue::Byte(v) => v,
            cpclib_sna::FlagValue::Word(v) => (v & 0xFF) as u8,
            _ => 0
        }
    };
    let mode = get(F::GA_ROMCFG) & 0b11;
    let mut pens = [0u8; 17];
    for (i, slot) in pens.iter_mut().enumerate() {
        *slot = get(F::GA_PAL(Some(i)));
    }
    (mode, palette_from_raw_ga(&pens))
}

/// The CPC's own standard screen geometry - 80 bytes wide (the real, fixed
/// hardware width every mode shares, only the *pixel* count per byte
/// differs) and 200 lines tall - `-sv`'s own defaults when the interactive
/// panel's controls haven't overridden them yet.
pub const DEFAULT_SCREEN_WIDTH: usize = 80;
pub const DEFAULT_SCREEN_HEIGHT: usize = 200;

/// A sane range for the interactive panel's own width/height controls - a
/// user is free to type anything, but nothing here should be able to ask
/// for a multi-megabyte PNG by mistake. `MAX_SCREEN_HEIGHT` is generous on
/// purpose, and deliberately large enough to cover the panel's own multi-
/// column tiling: reported live, a narrow requested width tiles into many
/// columns side by side, each wanting a full column's worth of real lines
/// of its own - `columns * linesPerColumn` real lines requested in total,
/// which a merely screen-sized cap (a couple of thousand) silently
/// truncated, leaving the bottom of a many-column layout blank even though
/// there was real panel space left to fill. `0x10000` is a natural ceiling
/// regardless: no `-sv` request ever needs to show more real lines than
/// there are bytes in the whole address space to read them from.
const MAX_SCREEN_WIDTH: usize = 255;
const MAX_SCREEN_HEIGHT: usize = 0x10000;

/// WinAPE's own two "browse memory as pixels" encodings, next to each other
/// in its own menu - `-sv`'s 7th argument selects between them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenEncoding {
    /// The CRTC-accurate one: interleaved (`MA`/`RA`), confined to the
    /// screen's own 16K bank - `ColorMatrix::from_screen_at`.
    Screen,
    /// The raw one: sequential bytes, wrapped at the full 64K space,
    /// `charRowHeight` a pure display spacer with no effect on which bytes
    /// are read - `ColorMatrix::from_linear_memory`.
    Cpc
}

impl ScreenEncoding {
    pub fn from_wire(value: u8) -> Self {
        match value {
            1 => Self::Cpc,
            _ => Self::Screen
        }
    }

    fn as_wire(self) -> u8 {
        match self {
            Self::Screen => 0,
            Self::Cpc => 1
        }
    }
}

/// A known screen address, mode and palette plus a raw memory window
/// (`memory[0]` = real address 0x0000, i.e. the full 64K space - see
/// `ColorMatrix::from_screen_at`'s own doc comment for why) turned into a
/// `cpclib/screenView` event's body - shared by both session types' `-sv`
/// command, one rendering pipeline rather than two to keep in sync.
///
/// `width`/`height` are already-resolved (defaulted by the caller, e.g.
/// via [`DEFAULT_SCREEN_WIDTH`]/[`DEFAULT_SCREEN_HEIGHT`]) rather than
/// `Option`s here - this function only clamps them to a sane range, it
/// does not decide what "unset" means, since the two session types
/// resolve that from slightly different pending-state shapes.
///
/// No visual spacer/separator of any kind is drawn into the image itself:
/// an earlier version inserted a black row every `lines_per_char_row` real
/// lines (`ColorMatrix::insert_blank_row_every`), but a real gap looked
/// nothing like the padding the panel's own multi-column tiling already
/// draws between columns - reported live as visually inconsistent. All
/// spacing, in both directions alike, is the panel's own doing now,
/// entirely client-side, on a plain, ungapped render.
pub fn render_screen_view(
    address: usize,
    width: usize,
    height: usize,
    mode: u8,
    palette: &Palette<Ink>,
    memory: &[u8],
    lines_per_char_row: usize,
    palette_override: &[Option<Ink>],
    encoding: ScreenEncoding
) -> Result<Value, String> {
    if memory.is_empty() {
        return Err("could not read the screen's own memory".to_string());
    }
    let mode = Mode::from(mode.min(3));
    let bytes_width = width.clamp(1, MAX_SCREEN_WIDTH);
    let pixel_height = height.clamp(1, MAX_SCREEN_HEIGHT);

    // The window's own palette, not the CPC's: the Gate Array supplies the
    // starting point (`palette`) for every pen, but nothing here is ever
    // written back to it - there is no known way to write ink registers
    // through the emulator's own debug API, and there does not need to be
    // one for this. A pen this session has overridden keeps that choice
    // across every re-render (including the automatic ones `-sv` never
    // asked for again), until the view is asked to auto-detect afresh.
    let mut palette = palette.clone();
    for (pen, ink) in palette_override.iter().enumerate() {
        if let Some(ink) = ink {
            palette.set(pen as u8, *ink);
        }
    }
    let palette = &palette;

    let mut matrix: ColorMatrix<Ink> = match encoding {
        ScreenEncoding::Screen => {
            ColorMatrix::from_screen_at(
                memory,
                address,
                bytes_width,
                pixel_height,
                lines_per_char_row,
                mode,
                palette
            )
        },
        // `lines_per_char_row` is deliberately not passed here at all - the
        // whole point of this encoding, per the user's own correction, is
        // that the char-row-height value never touches address computation
        // at all for this encoding, only the panel's own client-side
        // layout.
        ScreenEncoding::Cpc => {
            ColorMatrix::from_linear_memory(memory, address, bytes_width, pixel_height, mode, palette)
        }
    };

    // The CPC's own pixel aspect ratio, reported live as missing from the
    // first cut of this view: every mode shares the same physical screen
    // width, achieved by wider dots in a lower-resolution mode - Mode 2's
    // 8 dots/byte are already the real width, Mode 1's 4 dots/byte are each
    // shown twice, Mode 0/3's 2 dots/byte are each shown four times, so
    // every mode ends up exactly 8 displayed dots wide per byte regardless
    // of its own native resolution. `double_horizontally` doubles in place,
    // so 4x is two calls, 2x is one, 1x is none.
    match mode {
        Mode::Zero | Mode::Three => {
            matrix.double_horizontally();
            matrix.double_horizontally();
        },
        Mode::One => matrix.double_horizontally(),
        Mode::Two => {}
    }
    // Vertical stretch is uniform across every mode - a raster line reads
    // roughly twice as tall as it is a Mode 2 dot wide.
    matrix.double_vertically();

    let png = matrix
        .as_png_bytes()
        .map_err(|problem| format!("could not encode the screen as an image: {problem}"))?;

    Ok(json!({
        "png": crate::amspiritlite::encode_base64(&png),
        "address": address,
        "width": bytes_width,
        "height": pixel_height,
        "mode": mode as u8 as i64,
        "bytes": crate::amspiritlite::encode_base64(memory),
        "charRowHeight": lines_per_char_row,
        "palette": palette_as_hex(palette),
        "hardwarePalette": hardware_palette_as_hex(),
        "encoding": encoding.as_wire()
    }))
}

/// [`render_screen_view`] plus the DAP envelope every `-sv` path wraps it
/// in: the `cpclib/screenView` event, and - when something is actually
/// waiting on an answer - a console receipt or a failure.
///
/// Shared by `Session` and `BasicSession`'s own identically-shaped
/// `screen_view_answer` methods, which otherwise built this same event/
/// response/failure envelope independently around the same call.
///
/// `config_stamp`, when given, is `(mode, page)` - the RAM configuration this
/// frame was actually read under, echoed onto the event body so the panel's
/// own config picker stays in sync with it, the same way the Memory/
/// Disassembly View's own pickers do. Only the Z80 session's memory-mapped-
/// RAM views ever resolve one; `BasicSession` passes `None`.
///
/// `request` is `None` for the refresh a stop triggers automatically - event
/// only, nothing to answer - and `Some` for a console command or a request
/// that wants a receipt. `next_seq` is called once per message actually
/// produced, never for one that ends up not being sent, so a caller's own
/// seq counter advances exactly as it did before this was factored out.
#[allow(clippy::too_many_arguments)]
pub fn screen_view_event_and_receipt(
    address: usize,
    width_override: Option<usize>,
    height_override: Option<usize>,
    row_height_override: Option<usize>,
    palette_override: &[Option<Ink>],
    encoding_override: Option<u8>,
    config_stamp: Option<(u8, Option<u32>)>,
    regs: &[u8; 18],
    mode: u8,
    palette: &Palette<Ink>,
    memory: &[u8],
    request: Option<&Value>,
    mut next_seq: impl FnMut() -> i64
) -> Vec<Value> {
    let (default_width, live_lines_per_char_row) = crtc_screen_defaults(regs);
    let width = width_override.unwrap_or(default_width);
    let height = height_override.unwrap_or(DEFAULT_SCREEN_HEIGHT);
    let lines_per_char_row = resolve_char_row_height(row_height_override, live_lines_per_char_row);
    let encoding = ScreenEncoding::from_wire(encoding_override.unwrap_or(0));
    match render_screen_view(
        address,
        width,
        height,
        mode,
        palette,
        memory,
        lines_per_char_row,
        palette_override,
        encoding
    ) {
        Ok(mut body) => {
            if let Some((mode, page)) = config_stamp
                && let Some(object) = body.as_object_mut()
            {
                object.insert("config".to_string(), json!(mode));
                object.insert("page".to_string(), json!(page));
            }
            let seq = next_seq();
            let event = crate::protocol::event("cpclib/screenView", body, seq);
            match request {
                Some(request) => {
                    let seq = next_seq();
                    let receipt = crate::protocol::response(
                        request,
                        json!({ "result": "screen view opened", "variablesReference": 0 }),
                        seq
                    );
                    vec![event, receipt]
                },
                None => vec![event]
            }
        },
        Err(problem) => {
            match request {
                Some(request) => {
                    let seq = next_seq();
                    vec![crate::protocol::failure(request, &problem, seq)]
                },
                None => Vec::new()
            }
        }
    }
}

/// The live palette as `"#RRGGBB"` strings, pen 0 first - what the
/// interactive panel's own swatches show. Always all 16 slots: a Mode 1/2/3
/// screen only *uses* fewer of them, but the Gate Array still holds real
/// values in the rest, and showing all sixteen is simpler than threading
/// "how many pens does this mode have" through here too when the panel can
/// decide that for itself from `mode` alone.
fn palette_as_hex(palette: &Palette<Ink>) -> Vec<String> {
    (0..16u8)
        .map(|pen| {
            let ink = *palette.get(&cpclib_image::pen::Pen::from(pen));
            ink_to_hex(ink)
        })
        .collect()
}

fn ink_to_hex(ink: Ink) -> String {
    let rgb: image::Rgb<u8> = ink.into();
    format!("#{:02X}{:02X}{:02X}", rgb.0[0], rgb.0[1], rgb.0[2])
}

/// The 27 real CPC hardware inks, in their canonical ink-number order - what
/// the palette editor's own picker offers, one pen at a time, in place of
/// whatever the live Gate Array put there. `Ink::INKS` itself is 32 long
/// (see its own doc comment: "do not take into account the few duplicates")
/// - only the first 27 are real, distinct hardware colours.
pub fn hardware_palette_as_hex() -> Vec<String> {
    Ink::INKS[..27].iter().copied().map(ink_to_hex).collect()
}

/// Extra rows the CRTC scope prepends whenever `validate_crtc` finds
/// something - the standard Variables view has no per-row colour or
/// severity, so a bad combination becomes a row of its own instead, right
/// where the registers it is about already are. Run every time the CRTC
/// scope is read, by both backends, rather than needing a console command
/// first: `-crtcview`'s own panel is a nicer (and literally red) look at the
/// same data for whoever opens it, not the only way to see it.
pub fn crtc_warning_variables(regs: &[u8]) -> Vec<Value> {
    validate_crtc(regs)
        .into_iter()
        .map(|w| {
            let icon = match w.severity {
                CrtcSeverity::Error => "\u{26D4}",
                CrtcSeverity::Warning => "\u{26A0}"
            };
            json!({
                "name": format!("{icon} {}", w.registers.join(",")),
                "value": w.message,
                "type": "a known-bad CRTC register combination",
                "variablesReference": 0
            })
        })
        .collect()
}

/// What each of the CRTC's 18 registers actually holds, in register-number
/// order - shared by the snapshot-based CRTC pane here and AMSpiriT Lite's
/// own live one (`amspiritlite::crtc_pane`), which reads the same registers
/// out of a `cpclib/machineState` body instead of a `.sna`.
pub(crate) const CRTC_REGISTER_MEANING: [&str; 18] = [
    "R0 horizontal total",
    "R1 horizontal displayed",
    "R2 horizontal sync position",
    "R3 sync widths (VSYNC:HSYNC)",
    "R4 vertical total",
    "R5 vertical total adjust",
    "R6 vertical displayed",
    "R7 vertical sync position",
    "R8 interlace and skew",
    "R9 maximum raster address",
    "R10 cursor start raster",
    "R11 cursor end raster",
    "R12 display start address (high)",
    "R13 display start address (low)",
    "R14 cursor address (high)",
    "R15 cursor address (low)",
    "R16 light pen address (high)",
    "R17 light pen address (low)"
];

/// Read one chip's state out of a snapshot of the running machine.
///
/// The emulator exposes nothing for the CRTC, the Gate Array, the PSG or the
/// PPI - its debug API is the Z80, memory, breakpoints and stepping. But a
/// `.sna` header carries all of it, and the emulator can write one, so the
/// machine is asked to describe itself and the answer is parsed with the same
/// code that reads a snapshot from disk.
///
/// The **counters** are the reason this is worth the round trip on a demo: a
/// raster effect is a race against `HCC` and `RLC`, and until now there was no
/// way to see where in the frame a breakpoint had landed.
pub fn chip_variables(reference: i64, sna: &cpclib_sna::Snapshot) -> Option<Vec<Value>> {
    use cpclib_sna::SnapshotFlag as F;

    let get = |flag: F| -> u8 {
        match sna.get_value(&flag) {
            cpclib_sna::FlagValue::Byte(v) => v,
            cpclib_sna::FlagValue::Word(v) => (v & 0xFF) as u8,
            _ => 0
        }
    };

    let variables = match reference {
        CRTC_REFERENCE => {
            let mut out = crtc_warning_variables(&crtc_registers(sna));
            out.push(byte(
                "selected",
                get(F::CRTC_SEL),
                "the register &BCxx writes reach"
            ));
            out.push(byte(
                "type",
                get(F::CRTC_TYPE),
                "which CRTC this machine has"
            ));
            // The register &BCxx writes reach, underlined - the next &BDxx
            // write lands there.
            let selected = get(F::CRTC_SEL) as usize;
            out.extend((0..18usize).map(|i| {
                let name = format!("R{i}");
                let name = if i == selected {
                    underlined(&name)
                }
                else {
                    name
                };
                byte(&name, get(F::CRTC_REG(Some(i))), CRTC_REGISTER_MEANING[i])
            }));
            // Where in the frame we are - the question a raster effect is
            // actually asking.
            out.push(byte(
                "HCC",
                get(F::CRTC_HCC),
                "counter: horizontal character"
            ));
            out.push(byte("VCC", get(F::CRTC_VAC), "counter: vertical character"));
            out.push(byte(
                "VLC",
                get(F::CRTC_CLC),
                "counter: raster line within a character"
            ));
            out.push(byte("RLC", get(F::CRTC_RLC), "counter: raster line"));
            out.push(byte("VSWC", get(F::CRTC_VSWC), "counter: VSYNC width"));
            out.push(byte("HSWC", get(F::CRTC_HSWC), "counter: HSYNC width"));
            out.push(byte("state", get(F::CRTC_STATE), "internal status flags"));
            out
        },
        GATE_ARRAY_REFERENCE => {
            let romcfg = get(F::GA_ROMCFG);
            let selected = get(F::GA_PEN) & 0x1F;
            let mut out = vec![
                byte("pen", selected, "the pen &7Fxx writes reach"),
                json!({
                    "name": "mode",
                    "value": format!("{}", romcfg & 0b11),
                    "type": "screen mode, from the ROM/mode byte",
                    "variablesReference": 0
                }),
                byte(
                    "ROM config",
                    romcfg,
                    "bit 2 lower ROM off, bit 3 upper ROM off"
                ),
                byte(
                    "RAM config",
                    get(F::GA_RAMCFG),
                    "the &7Fxx RAM banking selection"
                ),
                byte(
                    "VSYNC counter",
                    get(F::GA_VSC),
                    "counter: since the last VSYNC"
                ),
                byte(
                    "interrupt counter",
                    get(F::GA_ISC),
                    "counter: raster lines to the next interrupt"
                ),
            ];

            // Sixteen pens plus the border. They are *pens* - the slot - and
            // what is in one is an ink; calling the slots inks was simply
            // wrong.
            out.extend((0..17usize).map(|index| {
                let raw = get(F::GA_PAL(Some(index))) & 0x1F;
                // What a program writes to &7Fxx is the colour with bit 6 set,
                // so that is the byte to show: it is the one you would look for
                // in your own source.
                let written = 0x40 | raw;
                let name = if index == 16 {
                    "border".to_string()
                }
                else {
                    format!("pen {index}")
                };
                // The pen the Gate Array is currently pointed at, underlined -
                // the next &7Fxx colour write lands there.
                let name = if index == selected as usize {
                    underlined(&name)
                }
                else {
                    name
                };
                let (value, meaning) = gate_array_pen(written).unwrap_or_else(|| {
                    (
                        format!("0x{written:02X}"),
                        "not a colour the Gate Array can produce".to_string()
                    )
                });
                json!({
                    "name": name,
                    "value": value,
                    "type": meaning,
                    "variablesReference": 0
                })
            }));
            out
        },
        PSG_REFERENCE => {
            let mut out = vec![byte(
                "selected",
                get(F::PSG_SEL),
                "the AY register the PPI writes reach"
            )];
            out.extend((0..16usize).map(|i| {
                byte(
                    &format!("R{i}"),
                    get(F::PSG_REG(Some(i))),
                    "AY-3-8912 register"
                )
            }));
            out
        },
        PPI_REFERENCE => {
            vec![
                byte("A", get(F::PPI_A), "port A: PSG data"),
                byte("B", get(F::PPI_B), "port B: VSYNC, printer, tape, jumpers"),
                byte(
                    "C",
                    get(F::PPI_C),
                    "port C: keyboard line, tape and PSG control"
                ),
                byte("control", get(F::PPI_CTL), "the 8255 control byte"),
            ]
        },
        DISC_REFERENCE => {
            vec![
                byte(
                    "motor",
                    get(F::FDD_MOTOR),
                    "drive motor: non-zero is running"
                ),
                byte("track", get(F::FDD_TRACK), "the track the head is over"),
                json!({
                    "name": "(FDC registers)",
                    "value": "not available",
                    // Worth saying rather than leaving a half-empty scope: the
                    // uPD765's own main status, data and result-phase registers
                    // are in neither place they could come from - the snapshot
                    // format carries only the drive's motor and track, and the
                    // emulator's debug API exposes no FDC call at all.
                    "type": "the snapshot format carries only motor and track; the \
                             emulator exposes no FDC registers",
                    "variablesReference": 0
                }),
            ]
        },
        _ => return None
    };
    Some(variables)
}

/// What a chip scope says before the machine has been asked to describe itself.
pub fn chip_placeholder(reference: i64, why: &str) -> Option<Vec<Value>> {
    if !is_chip_scope(reference) {
        return None;
    }
    Some(vec![json!({
        "name": "(unavailable)",
        "value": why,
        "variablesReference": 0
    })])
}

#[cfg(test)]
mod tests {
    use cpclib_asm::assembler::listing_output::{RawSourceMap, SourceMapRow};

    use super::*;

    /// A second, independent real fixture (`cpclib-dap/tests/graphics/
    /// blight/`) - a page-aligned address (0xC000, unlike `hello`'s
    /// 0xC5E0) rendered *past* the standard 200-line screen height, the
    /// same way the user enlarged WinAPE's own window specifically to
    /// reveal what the wrap-around shows beyond the visible screen. WinAPE's
    /// own "Screen" capture at this configuration shows the same demo-
    /// credits text (`BLIGHT`, `A 1 SCREENED WONDER`, `BY BND AT BND 5`,
    /// `CODE BY KRUSTY`, ...) genuinely *repeating*, wrapped and shifted,
    /// once the interleaved addressing runs past the end of memory -
    /// visually confirmed pixel-for-pixel matching this crate's own render
    /// of the identical snapshot. This pins the repeat down as a real,
    /// non-background render (not a blank/wrong-address void) at both the
    /// original position and well into the wrapped repeat.
    #[test]
    fn blight_snapshot_wraps_into_a_real_repeat_past_200_lines() {
        let sna = cpclib_sna::Snapshot::load("tests/graphics/blight/snapshot.sna")
            .expect("load sna - run from the cpclib-dap crate root");
        let regs = crtc_registers(&sna);
        let address = crtc_screen_start_address(regs[12], regs[13]);
        assert_eq!(address, 0xC000, "regs: {regs:?}");

        let (mode, palette) = mode_and_palette_from_snapshot(&sna);
        let full_memory = sna.memory_dump();
        let memory = &full_memory[..0x10000.min(full_memory.len())];
        let matrix: cpclib_image::image::ColorMatrix<cpclib_image::ink::Ink> =
            cpclib_image::image::ColorMatrix::from_screen_at(
                memory, address, 80, 400, 8, mode.into(), &palette
            );

        let lit_pixels_in = |y_range: std::ops::Range<u32>| {
            (0..matrix.width())
                .flat_map(|x| y_range.clone().map(move |y| (x, y)))
                .filter(|&(x, y)| {
                    *matrix.get_color(x as usize, y as usize)
                        != *palette.get(&cpclib_image::pen::Pen::from(0))
                })
                .count()
        };
        let original = lit_pixels_in(0..190);
        let wrapped_repeat = lit_pixels_in(210..400);
        assert!(original > 2000, "only {original} lit pixels in the original text");
        assert!(
            wrapped_repeat > 2000,
            "only {wrapped_repeat} lit pixels past the wrap - reading blank/wrong memory?"
        );
    }

    /// Reported live: a scrolled BASIC screen's CRTC address (R12=0x30,
    /// R13=0x88 -> 0xC110, not page-aligned) rendered via `-sv` cut off
    /// "Ready" and everything after it. Root cause and the actual fix live
    /// in `cpclib-image` (`ColorMatrix::from_screen_at` now wraps within
    /// the full 64K address space instead of indexing a plain, non-
    /// wrapping slice - see that crate's own `from_screen_at_wraps_at_the_
    /// full_64k_address_space_not_just_one_page` for the focused
    /// regression test, including the follow-up correction from an initial,
    /// wrong "wraps within one 16K page" theory). This confirms
    /// `render_screen_view` itself no longer needs (or has) a safety clamp
    /// shrinking the image to stay in bounds - 200 lines, unconditionally.
    #[test]
    fn render_screen_view_honours_the_default_and_an_overridden_height() {
        let palette = palette_from_raw_ga(&[0u8; 17]);
        // The full 64K address space, as every caller now provides - see
        // `Session`/`BasicSession`'s own `screen_view_command` doc
        // comments for why (real hardware wraps the interleaved display
        // read at the full 16-bit address boundary, not within any 16K
        // page).
        let memory = vec![0u8; 0x10000];
        let body = render_screen_view(0xC110, 80, 200, 1, &palette, &memory, 8, &[], ScreenEncoding::Screen).unwrap();
        assert_eq!(body["height"], 200);

        // A user enlarging the interactive panel's own height control past
        // the standard screen - exactly how the wrap-around bugs in this
        // feature's own history were actually found - must not be silently
        // clamped back down to 200.
        let body = render_screen_view(0xC000, 80, 400, 1, &palette, &memory, 8, &[], ScreenEncoding::Screen).unwrap();
        assert_eq!(body["height"], 400);
    }

    /// The PNG's own pixel dimensions, not the wire `"width"`/`"height"`
    /// (those stay the logical byte-grid values everything else - the
    /// mouse-over readout's own byte-address math - keys off unchanged),
    /// carry the CPC's pixel aspect ratio: every mode ends up displayed at
    /// exactly 8 dots per byte horizontally (Mode 2's real 8, Mode 1's 4
    /// doubled, Mode 0/3's 2 quadrupled), and every mode's rows are doubled
    /// vertically. Reported live as missing from the first cut of this
    /// view - Mode 1 and Mode 0 screens rendered squashed relative to a
    /// real monitor.
    #[test]
    fn render_screen_view_stretches_the_png_to_the_cpc_pixel_aspect_ratio() {
        let palette = palette_from_raw_ga(&[0u8; 17]);
        let memory = vec![0u8; 0x10000];

        // (mode, bytes_width, expected png width, png height for a 10-line request)
        let cases = [
            (0u8, 10usize, 10 * 8, 20),
            (1u8, 10usize, 10 * 8, 20),
            (2u8, 10usize, 10 * 8, 20),
            (3u8, 10usize, 10 * 8, 20)
        ];
        for (mode, bytes_width, expected_png_width, expected_png_height) in cases {
            let body = render_screen_view(
                0xC000, bytes_width, 10, mode, &palette, &memory, 8, &[],
                ScreenEncoding::Screen
            )
            .unwrap();
            // The wire field is still the logical, unscaled grid - the
            // mouse-over math and every other consumer keys off this.
            assert_eq!(body["width"], bytes_width, "mode {mode}");
            assert_eq!(body["height"], 10, "mode {mode}");

            let png = crate::session::decode_base64(body["png"].as_str().unwrap());
            // PNG signature (8 bytes) + chunk length (4) + "IHDR" (4), then
            // width and height as big-endian u32 - the cheapest correct way
            // to read a PNG's own pixel size without a new dependency.
            let png_width = u32::from_be_bytes(png[16..20].try_into().unwrap());
            let png_height = u32::from_be_bytes(png[20..24].try_into().unwrap());
            assert_eq!(png_width, expected_png_width as u32, "mode {mode}");
            assert_eq!(png_height, expected_png_height as u32, "mode {mode}");
        }
    }

    /// Three independent live data points - see `crtc_screen_start_address`'s
    /// own doc comment for how each was obtained. The first two alone
    /// (both zero-offset) do not distinguish `page*0x4000 + offset*2` from
    /// the simpler-but-wrong `MA << 2`; the third (a real, scrolled
    /// snapshot, non-zero offset) is the one that actually pins the `*2`
    /// down - `MA << 2` gives 0xCBC0 for it, not 0xC5E0.
    #[test]
    fn crtc_screen_start_address_matches_what_was_observed_live() {
        assert_eq!(crtc_screen_start_address(0x30, 0x00), 0xC000);
        assert_eq!(crtc_screen_start_address(0x20, 0x00), 0x8000);
        assert_eq!(crtc_screen_start_address(0x10, 0x00), 0x4000);
        assert_eq!(crtc_screen_start_address(0x00, 0x00), 0x0000);
        assert_eq!(crtc_screen_start_address(0x32, 0xF0), 0xC5E0);
    }

    /// `crtc_screen_defaults`: `2*R1` for width, `R9+1` for lines-per-char-
    /// row - both a standard config (R1=40, R9=7, matching the constants
    /// this replaced) and a non-standard one, so a bug that silently
    /// ignores the registers and falls back to the old hard-coded 80/8
    /// cannot hide behind a fixture that happens to match them anyway
    /// (the one real integration test covering this, in `basic_session.rs`,
    /// uses exactly the standard config for its own regs and would not
    /// catch that).
    #[test]
    fn crtc_screen_defaults_reads_width_and_char_row_height_off_the_live_registers() {
        let mut standard = [0u8; 18];
        standard[1] = 40;
        standard[9] = 7;
        assert_eq!(crtc_screen_defaults(&standard), (80, 8));

        let mut narrow_tall = [0u8; 18];
        narrow_tall[1] = 20;
        narrow_tall[9] = 3;
        assert_eq!(crtc_screen_defaults(&narrow_tall), (40, 4));

        // R1 == 0 reads as "unknown", not a real zero-width screen.
        let unknown = [0u8; 18];
        assert_eq!(crtc_screen_defaults(&unknown), (DEFAULT_SCREEN_WIDTH, 1));
    }

    /// Unset or an explicit `0` both mean "use the live CRTC value"; any
    /// other explicit value overrides the address math's own row height,
    /// not only a display value - reported live: a taller/shorter row than
    /// the live CRTC's own is the whole point of browsing raw memory this
    /// way, and this is the one thing that actually has to change for that.
    #[test]
    fn resolve_char_row_height_overrides_the_address_math_itself() {
        assert_eq!(resolve_char_row_height(None, 8), 8, "unset: live CRTC value");
        assert_eq!(resolve_char_row_height(Some(0), 8), 8, "explicit 0: also the live CRTC value");
        assert_eq!(resolve_char_row_height(Some(16), 8), 16, "explicit 16: overrides it");
    }

    /// End-to-end rendering check against a real snapshot and a real
    /// WinAPE capture of the same memory
    /// (`cpclib-dap/tests/graphics/hello/`) - a BASIC program that printed
    /// "hello" until the screen scrolled, `BREAK`-ed. Confirms the whole
    /// chain (CRTC regs -> `crtc_screen_start_address` -> `.sna` memory ->
    /// `ColorMatrix::from_screen_at`) lands on an image dominated by ink
    /// (lit "hello" text pixels), not the small residual of a wrong,
    /// mostly-background-colour address a few hundred bytes off would
    /// produce.
    #[test]
    fn hello_snapshot_renders_a_screen_dominated_by_lit_text() {
        let sna = cpclib_sna::Snapshot::load("tests/graphics/hello/snapshot.sna")
            .expect("load sna - run from the cpclib-dap crate root");
        let regs = crtc_registers(&sna);
        let address = crtc_screen_start_address(regs[12], regs[13]);
        assert_eq!(address, 0xC5E0, "regs: {regs:?}");

        let (mode, palette) = mode_and_palette_from_snapshot(&sna);
        let full_memory = sna.memory_dump();
        // The full 64K address space, from 0 - not just from `address`
        // (0xC5E0, mid-page): `data` is still addressed as the whole 64K
        // buffer even though `from_screen_at` itself confines the actual
        // *wrap* to the one 16K bank `address` falls in - see that
        // function's own doc comment and `cpclib-image`'s
        // `from_screen_at_wraps_within_the_same_16k_bank_not_into_the_next_one`.
        let memory = &full_memory[..0x10000.min(full_memory.len())];
        let matrix: cpclib_image::image::ColorMatrix<cpclib_image::ink::Ink> =
            cpclib_image::image::ColorMatrix::from_screen_at(
                memory, address, 80, 200, 8, mode.into(), &palette
            );

        // Pen 0 is the background; text is lit in other pens. A correctly
        // addressed render has thousands of non-background pixels; a wrong
        // address landing mostly in never-written memory would not.
        let lit_pixels = (0..matrix.width())
            .flat_map(|x| (0..matrix.height()).map(move |y| (x, y)))
            .filter(|&(x, y)| {
                *matrix.get_color(x as usize, y as usize)
                    != *palette.get(&cpclib_image::pen::Pen::from(0))
            })
            .count();
        assert!(
            lit_pixels > 2000,
            "only {lit_pixels} non-background pixels - wrong screen address?"
        );
    }

    #[test]
    fn palette_from_raw_ga_round_trips_gate_array_values() {
        let ink = Ink::INKS[5];
        // The raw, *stored* form (5 bits, no bit 6) - what `GA_PAL` actually
        // holds - not `gate_array_value()`'s "written to &7Fxx" form.
        let raw = ink.gate_array_value() & 0x1F;
        let mut pens = [0u8; 17];
        pens[3] = raw;
        pens[16] = raw; // border too
        let palette = palette_from_raw_ga(&pens);
        assert_eq!(*palette.get(&cpclib_image::pen::Pen::from(3)), ink);
        assert_eq!(*palette.get_border(), ink);
    }

    /// The palette swatch grid's own data: all 16 pens, `"#RRGGBB"`, pen 0
    /// first - real hardware ink 0 (black) pins the format down exactly
    /// rather than just "is a 7-character string"; every other pen is
    /// checked for that same shape, since this crate's own `Ink` constants
    /// do not promise which named ink is pure white (`Ink::WHITE` turned
    /// out to be `#808080`, not `#FFFFFF`, when first written).
    #[test]
    fn render_screen_view_reports_the_live_palette_as_hex_colours() {
        let mut palette = Palette::<Ink>::new();
        palette.set(0u8, Ink::BLACK);
        palette.set(1u8, Ink::WHITE);
        let memory = vec![0u8; 0x10000];

        let body = render_screen_view(0xC000, 2, 2, 1, &palette, &memory, 8, &[], ScreenEncoding::Screen).unwrap();
        let colours: Vec<&str> = body["palette"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(colours.len(), 16);
        assert_eq!(colours[0], "#000000");
        for colour in &colours {
            assert!(
                colour.len() == 7
                    && colour.starts_with('#')
                    && colour[1..].chars().all(|c| c.is_ascii_hexdigit()),
                "{colour}"
            );
        }
    }

    /// The palette override is the window's own, never sent anywhere near
    /// the Gate Array (there is no known way to write ink registers through
    /// the emulator's debug API, and no need for one here): pen 0 keeps
    /// whatever the live palette says because its override slot is `None`,
    /// pen 1's explicit override wins over the live value, and both survive
    /// into the reported `"palette"` field exactly as rendered.
    #[test]
    fn render_screen_view_lets_the_window_override_individual_pens() {
        let mut palette = Palette::<Ink>::new();
        palette.set(0u8, Ink::BLACK);
        palette.set(1u8, Ink::BLACK);
        let memory = vec![0u8; 0x10000];

        let overrides = [None, Some(Ink::WHITE)];
        let body =
            render_screen_view(0xC000, 2, 2, 1, &palette, &memory, 8, &overrides, ScreenEncoding::Screen).unwrap();
        let colours: Vec<&str> = body["palette"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(colours[0], "#000000", "pen 0 has no override, stays live");
        assert_eq!(colours[1], ink_to_hex(Ink::WHITE), "pen 1's override wins");
    }

    /// `ScreenEncoding::Cpc` reads straight through, wrapping at the full
    /// 64K space - unlike `Screen`, which would confine the very same
    /// request to the 16K bank `address` falls in. The wire `"encoding"`
    /// field round-trips whichever was asked for, and `"charRowHeight"`
    /// is unaffected either way (it plays no addressing role for `Cpc`).
    #[test]
    fn render_screen_view_cpc_encoding_wraps_at_the_full_64k_space_not_the_bank() {
        let mut palette = Palette::<Ink>::new();
        palette.set(0u8, Ink::BLACK);
        palette.set(1u8, Ink::WHITE);
        let mut memory = vec![0u8; 0x10000];
        memory[0x0000] = 0xFF; // only reachable by wrapping past 0xFFFF

        let body = render_screen_view(
            0xFFFF, 1, 2, 2, &palette, &memory, 8, &[], ScreenEncoding::Cpc
        )
        .unwrap();
        assert_eq!(body["encoding"], 1);

        let png = crate::session::decode_base64(body["png"].as_str().unwrap());
        let decoded = image::load_from_memory(&png).unwrap().to_rgb8();
        // Line 1, doubled vertically, starts at pixel row 2; Mode 2's own
        // 8 dots/byte are shown 1x horizontally (already the real width).
        let pixel = decoded.get_pixel(0, 2);
        let expected: image::Rgb<u8> = Ink::WHITE.into();
        assert_eq!(
            *pixel, expected,
            "line 1 must read the byte wrapped past 0xFFFF to 0x0000, not a blank one"
        );
    }

    /// The picker's own data: all 27 real hardware inks, none of the
    /// duplicate aliases `Ink::INKS` also carries past index 26.
    #[test]
    fn hardware_palette_as_hex_lists_exactly_the_27_real_inks() {
        let hardware = hardware_palette_as_hex();
        assert_eq!(hardware.len(), 27);
        assert_eq!(hardware[0], ink_to_hex(Ink::INKS[0]));
        assert_eq!(hardware[26], ink_to_hex(Ink::INKS[26]));
    }

    /// R0=63, R2=60, R3=0x8c does not respect R2+(R3&0x0f) < R0
    /// (0x8c & 0x0f = 12, 60+12=72, and 72 is not < 63) - the warning is
    /// raised on that failure, not on the safe relationship itself.
    #[test]
    fn a_known_sync_losing_combination_is_flagged() {
        let mut regs = [0u8; 18];
        regs[0] = 63;
        regs[2] = 60;
        regs[3] = 0x8c;
        let warnings = validate_crtc(&regs);
        let sync = warnings
            .iter()
            .find(|w| w.registers.contains(&"R2"))
            .expect("R2+(R3&0x0f) < R0 not holding should be flagged");
        assert_eq!(sync.severity, CrtcSeverity::Error);
        assert!(sync.registers.contains(&"R0"));
        assert!(sync.registers.contains(&"R3"));
    }

    /// R0=63 exactly is the case the raster-timing rule exists to permit.
    #[test]
    fn r0_63_raises_no_line_duration_warning() {
        let mut regs = [0u8; 18];
        regs[0] = 63;
        regs[2] = 10; // well under R0, so this does not also raise sync-loss
        regs[3] = 0x00;
        let warnings = validate_crtc(&regs);
        assert!(
            !warnings
                .iter()
                .any(|w| w.message.contains("64 NOPs")),
            "{warnings:?}"
        );
    }

    #[test]
    fn a_line_not_64_nops_long_is_flagged() {
        let mut regs = [0u8; 18];
        regs[0] = 50;
        regs[2] = 10; // well under R0, so this does not also raise sync-loss
        let warnings = validate_crtc(&regs);
        let duration = warnings
            .iter()
            .find(|w| w.registers == &["R0"])
            .expect("R0 != 63 should be flagged");
        assert_eq!(duration.severity, CrtcSeverity::Warning);
    }

    /// The CRTC scope carries its own warnings now, automatically - not just
    /// the separate `-crtcview` panel. Reported from real use: the panel
    /// requires a console command the user has to remember to run, but the
    /// scope is already open every time the Variables view is.
    #[test]
    fn the_crtc_scope_flags_a_known_bad_configuration_on_its_own() {
        use cpclib_sna::{Snapshot, SnapshotFlag};

        let mut sna = Snapshot::default();
        sna.set_value(SnapshotFlag::CRTC_REG(Some(0)), 63).unwrap();
        sna.set_value(SnapshotFlag::CRTC_REG(Some(2)), 60).unwrap();
        sna.set_value(SnapshotFlag::CRTC_REG(Some(3)), 0x8c).unwrap();

        let crtc = chip_variables(CRTC_REFERENCE, &sna).unwrap();
        let warning = crtc
            .iter()
            .find(|v| v["name"].as_str().unwrap_or_default().contains("R2"))
            .unwrap_or_else(|| panic!("no warning row: {crtc:?}"));
        assert!(
            warning["value"].as_str().unwrap().contains("horizontal sync"),
            "{warning:?}"
        );
        // Still first, ahead of the registers it is about - the reason to
        // look is the first thing read, not the last.
        assert!(
            crtc.iter().position(|v| v == warning).unwrap()
                < crtc.iter().position(|v| v["name"] == json!("selected")).unwrap(),
            "{crtc:?}"
        );
    }

    #[test]
    fn a_well_formed_configuration_is_not_flagged() {
        let mut regs = [0u8; 18];
        regs[0] = 63;
        // R2 + (R3 & 0x0f) = 50 + 12 = 62, strictly less than R0 (63): the
        // safe relationship holds, so nothing is raised.
        regs[2] = 50;
        regs[3] = 0x8c;
        assert!(validate_crtc(&regs).is_empty());
    }

    #[test]
    fn flags_are_readable_at_a_glance() {
        // Z and C set, nothing else.
        assert_eq!(describe_flags(0b0100_0001), "-Z-----C");
        assert_eq!(describe_flags(0b1111_1111), "SZ5H3P/VNC");
        assert_eq!(describe_flags(0), "--------");
    }

    #[test]
    fn every_flag_gets_its_own_row() {
        let rows = flag_variables(0b0100_0000);
        assert_eq!(rows.len(), 8);
        let zero = rows.iter().find(|r| r["name"] == json!("Z")).unwrap();
        assert_eq!(zero["value"], json!("1"));
        let carry = rows.iter().find(|r| r["name"] == json!("C")).unwrap();
        assert_eq!(carry["value"], json!("0"));
    }

    /// The flags are inserted next to AF, and A is broken out beside them -
    /// the two things actually read while stepping.
    #[test]
    fn registers_gain_a_decoded_f() {
        let mut variables = vec![
            json!({"name": "AF", "value": "0x4A45", "variablesReference": 0}),
            json!({"name": "BC", "value": "0x0000", "variablesReference": 0}),
        ];
        annotate_registers(&mut variables, 42);

        let names: Vec<&str> = variables
            .iter()
            .map(|v| v["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, vec!["AF", "A", "F (flags)", "BC"]);

        let flags = &variables[2];
        assert_eq!(flags["value"], json!(describe_flags(0x45)));
        assert_eq!(flags["variablesReference"], json!(42), "expandable");
        assert_eq!(variables[1]["value"], json!("0x4A (74)"));
    }

    /// A register list without AF is left exactly as it was.
    #[test]
    fn registers_without_af_are_untouched() {
        let mut variables = vec![json!({"name": "BC", "value": "0x0000"})];
        let before = variables.clone();
        annotate_registers(&mut variables, 42);
        assert_eq!(variables, before);
    }

    /// The chips are not on the emulator's debug API, so every one of these
    /// scopes has to be answered here - and each is declared expensive, because
    /// answering means saving a whole machine.
    #[test]
    fn every_chip_scope_is_expensive_and_ours_to_answer() {
        let scopes = extra_scopes();
        assert_eq!(scopes.len(), 5, "CRTC, Gate Array, PSG, PPI, Disc");
        for scope in &scopes {
            let reference = scope["variablesReference"].as_i64().unwrap();
            assert_eq!(scope["expensive"], json!(true), "{scope}");
            assert!(is_chip_scope(reference), "{scope}");
            // Before the machine has been asked, the scope says why rather than
            // showing nothing.
            let placeholder = chip_placeholder(reference, "no answer yet").expect("answers");
            assert_eq!(placeholder.len(), 1);
            assert_eq!(placeholder[0]["value"], json!("no answer yet"));
        }
        assert!(
            !is_chip_scope(1),
            "the emulator's own references are its own"
        );
        assert!(chip_placeholder(1, "x").is_none());
    }

    /// A single label at an operand's address is named directly, with nothing
    /// to disambiguate - today's behaviour, unchanged by C.
    #[test]
    fn a_single_operand_candidate_is_named_as_before() {
        let map = SourceMap::from_raw(&RawSourceMap {
            files: vec!["main.asm".into()],
            rows: vec![SourceMapRow::flat(0, 2, 0x4000, 3)]
        })
        .with_symbols(
            [("table_data".to_string(), 0x5000u32)]
                .into_iter()
                .collect()
        );

        let mut instructions = vec![json!({"address": "0x4000", "instruction": "JP 0x5000"})];
        let ambiguous = annotate_disassembly(&mut instructions, &map, None, None, None);

        assert_eq!(instructions[0]["symbols"], json!(["table_data"]));
        assert!(ambiguous.is_empty(), "nothing to disambiguate");
    }

    /// A heading with several labels shows the top-preference one, as before,
    /// and lists the rest in `symbolAlternatives`, preference-sorted - the
    /// same order `symbols_at` already returns them in. No evidence exists
    /// for a heading, so this is the honest answer rather than a guess.
    #[test]
    fn an_ambiguous_heading_lists_its_alternatives_in_order() {
        let map = SourceMap::from_raw(&RawSourceMap {
            files: vec!["main.asm".into()],
            rows: vec![SourceMapRow::flat(0, 2, 0x4000, 3)]
        })
        .with_symbols(
            [
                ("b".to_string(), 0x4000u32),
                ("cd".to_string(), 0x4000u32),
                ("table_data".to_string(), 0x4000u32)
            ]
            .into_iter()
            .collect()
        );

        let mut instructions = vec![json!({"address": "0x4000", "instruction": "NOP"})];
        annotate_disassembly(&mut instructions, &map, None, None, None);

        assert_eq!(instructions[0]["symbol"], json!("b"), "shortest first");
        assert_eq!(
            instructions[0]["symbolAlternatives"],
            json!(["cd", "table_data"]),
            "the rest, in the same preference order"
        );
    }

    /// The disassembly-view counterpart of `Session::annotate_stack_trace`'s
    /// same-page-remap fix: `spectral_sprites.asm` (config `C4`) and
    /// `animate.asm` (config `C5`) both claim the very same logical address
    /// in extended-RAM page 1, so `page` alone cannot tell them apart -
    /// every row of a disassembly listing must be resolved against the exact
    /// bank, or a screenful of opcodes at one genuine address flips between
    /// two unrelated files exactly the way the reported bug did.
    #[test]
    fn a_disassembly_listing_follows_the_exact_bank_not_just_the_page() {
        let row = |file: u16, line: u32, physical: u32, len: u16| SourceMapRow {
            file,
            line,
            logical: 0x42A8,
            physical,
            page: 1,
            column: 1,
            column_end: 1,
            len,
            is_data: false
        };
        let map = SourceMap::from_raw(&RawSourceMap {
            files: vec!["spectral_sprites.asm".into(), "animate.asm".into()],
            rows: vec![
                row(0, 177, 0x102A8, 2), // C4: page 1, bank 0
                row(1, 308, 0x142A8, 1), // C5: page 1, bank 1
            ]
        });

        // Fetched under `C4`: the exact bank says `spectral_sprites.asm`,
        // even though `page` alone would leave this address ambiguous.
        let mut c4 = vec![json!({"address": "0x42a8", "instruction": "LD B, 0x8"})];
        annotate_disassembly(&mut c4, &map, Some(1), Some(0x102A8), None);
        assert_eq!(c4[0]["line"], json!(177));
        assert_eq!(c4[0]["location"]["name"], json!("spectral_sprites.asm"));

        // The very same logical address, fetched under `C5` instead: the
        // answer follows the bank, not whichever row happened to win the
        // coarse, page-only pick.
        let mut c5 = vec![json!({"address": "0x42a8", "instruction": "NOP"})];
        annotate_disassembly(&mut c5, &map, Some(1), Some(0x142A8), None);
        assert_eq!(c5[0]["line"], json!(308));
        assert_eq!(c5[0]["location"]["name"], json!("animate.asm"));
    }

    /// Several labels at an operand's address, and this row resolved to a
    /// source location: the top-preference guess is still written (today's
    /// default), and the ambiguity is handed back so a caller that can read
    /// that location's source line - `Session::resolve_ambiguous_operand_symbols`
    /// - gets a chance to prefer the one actually named.
    #[test]
    fn an_ambiguous_operand_with_a_location_is_reported() {
        let map = SourceMap::from_raw(&RawSourceMap {
            files: vec!["main.asm".into()],
            rows: vec![SourceMapRow::flat(0, 2, 0x4000, 3)]
        })
        .with_symbols(
            [
                ("b".to_string(), 0x5000u32),
                ("table_data".to_string(), 0x5000u32)
            ]
            .into_iter()
            .collect()
        );

        let mut instructions = vec![json!({"address": "0x4000", "instruction": "JP 0x5000"})];
        let ambiguous = annotate_disassembly(&mut instructions, &map, None, None, None);

        // The default guess still stands until a caller overrides it.
        assert_eq!(instructions[0]["symbols"], json!(["b"]));

        assert_eq!(ambiguous.len(), 1);
        assert_eq!(ambiguous[0].index, 0);
        assert_eq!(ambiguous[0].location.line, 2);
        assert_eq!(
            ambiguous[0].candidates,
            vec!["b".to_string(), "table_data".to_string()]
        );
    }

    /// The same ambiguity, but this row resolves to no source location at
    /// all (firmware, or an address the source map has nothing to say
    /// about). There is no line to read, so nothing is reported - the guess
    /// stands, undisturbed, rather than being flagged with no way to ever
    /// resolve it.
    #[test]
    fn an_ambiguous_operand_without_a_location_is_not_reported() {
        let map = SourceMap::from_raw(&RawSourceMap {
            files: vec!["main.asm".into()],
            rows: vec![]
        })
        .with_symbols(
            [
                ("b".to_string(), 0x5000u32),
                ("table_data".to_string(), 0x5000u32)
            ]
            .into_iter()
            .collect()
        );

        let mut instructions = vec![json!({"address": "0x4000", "instruction": "JP 0x5000"})];
        let ambiguous = annotate_disassembly(&mut instructions, &map, None, None, None);

        assert_eq!(instructions[0]["symbols"], json!(["b"]));
        assert!(
            ambiguous.is_empty(),
            "no location, nothing to disambiguate against"
        );
    }

    /// A `DB` row's own byte values must never be read as operand-address
    /// references. `inks` sitting at address 0 is the live symptom this
    /// pins: an ordinary instruction whose rendered text mentions `0x0000`
    /// still gets it named (cost known, and not `None`), but a data row with
    /// the exact same text and address - `cost: None`, `overlay_data_rows`'s
    /// own signal for "this is a DB, not a decoded instruction" - gets no
    /// `symbols` at all, because there is no operand here to name.
    #[test]
    fn a_db_rows_bytes_are_not_read_as_operand_addresses() {
        let map = SourceMap::from_raw(&RawSourceMap {
            files: vec!["main.asm".into()],
            rows: vec![]
        })
        .with_symbols([("inks".to_string(), 0x0000u32)].into_iter().collect());

        let mut instructions = vec![
            json!({"address": "0x4000", "instruction": "DB 0x0,0x0,0x0,0x0,0x0,0x0,0x0,0x0"}),
            json!({"address": "0x4008", "instruction": "LD HL, 0x0"}),
        ];
        let costs: Vec<Option<usize>> = vec![None, Some(4)];
        let ambiguous = annotate_disassembly(&mut instructions, &map, None, None, Some(&costs));

        assert!(
            instructions[0].get("symbols").is_none(),
            "a data row's own byte values are not addresses: {:?}",
            instructions[0]
        );
        assert_eq!(
            instructions[1]["symbols"],
            json!(["inks"]),
            "an ordinary instruction at the same address is still annotated"
        );
        assert!(ambiguous.is_empty());
    }

    /// `LD A,0` is not a reference to whatever label a program happens to
    /// define at address 0 - unlike `LD HL,0` just above, an 8-bit register
    /// load's immediate is a plain value, never an address. Reported from
    /// real use: `ld a, 0 : .writer_enable equ $-1` (a self-modifying-code
    /// idiom) disassembled with a spurious `; inks` because some unrelated
    /// symbol happened to sit at address 0.
    #[test]
    fn an_8_bit_register_load_is_not_read_as_an_operand_address() {
        let map = SourceMap::from_raw(&RawSourceMap {
            files: vec!["main.asm".into()],
            rows: vec![]
        })
        .with_symbols([("inks".to_string(), 0x0000u32)].into_iter().collect());

        let mut instructions = vec![json!({"address": "0x4000", "instruction": "LD A, 0x0"})];
        annotate_disassembly(&mut instructions, &map, None, None, None);

        assert!(
            instructions[0].get("symbols").is_none(),
            "an 8-bit immediate is not an address: {:?}",
            instructions[0]
        );
    }

    /// Nor is any ALU immediate, or a bit index - neither ever names an
    /// address either.
    #[test]
    fn alu_immediates_and_bit_indices_are_not_read_as_operand_addresses() {
        let map = SourceMap::from_raw(&RawSourceMap {
            files: vec!["main.asm".into()],
            rows: vec![]
        })
        .with_symbols([("inks".to_string(), 0x0000u32)].into_iter().collect());

        for text in ["XOR 0x0", "CP 0x0", "AND 0x0", "BIT 0x0, A"] {
            let mut instructions = vec![json!({"address": "0x4000", "instruction": text})];
            annotate_disassembly(&mut instructions, &map, None, None, None);
            assert!(
                instructions[0].get("symbols").is_none(),
                "{text}: {:?}",
                instructions[0]
            );
        }
    }

    /// A jump/call family instruction still names its target even when the
    /// target's own value is small - `RST 0x00` is a real, meaningful
    /// firmware entry point, not a coincidence.
    #[test]
    fn a_small_jump_target_is_still_named() {
        let map = SourceMap::from_raw(&RawSourceMap {
            files: vec!["main.asm".into()],
            rows: vec![]
        })
        .with_symbols([("reset".to_string(), 0x0000u32)].into_iter().collect());

        let mut instructions = vec![json!({"address": "0x4000", "instruction": "RST 0x00"})];
        annotate_disassembly(&mut instructions, &map, None, None, None);

        assert_eq!(instructions[0]["symbols"], json!(["reset"]));
    }

    /// `LD (0x0),A` is a genuine memory reference even though the address is
    /// small, unlike `LD A,0`.
    #[test]
    fn a_small_indirect_memory_reference_is_still_named() {
        let map = SourceMap::from_raw(&RawSourceMap {
            files: vec!["main.asm".into()],
            rows: vec![]
        })
        .with_symbols([("inks".to_string(), 0x0000u32)].into_iter().collect());

        let mut instructions = vec![json!({"address": "0x4000", "instruction": "LD (0x0), A"})];
        annotate_disassembly(&mut instructions, &map, None, None, None);

        assert_eq!(instructions[0]["symbols"], json!(["inks"]));
    }
}
