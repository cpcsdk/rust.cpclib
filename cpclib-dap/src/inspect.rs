//! Making the emulator's answers readable.
//!
//! The emulator reports what a Z80 knows: sixteen-bit registers as hex, and
//! disassembled instructions as addresses. Both are correct and both are hard
//! to work with - `AF = 0x4A45` does not say which flags are set, and an
//! address does not say which line of yours it came from. Everything here turns
//! one into the other on the way past.

use cpclib_image::color::AmstradColor;
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
        // bytes at `PC` turned out to match.
        //
        // Hoisted above the operand block below: disambiguating an operand
        // needs this row's own location to read the source line from.
        let located = page
            .and_then(|page| u16::try_from(address).ok().map(|address| (page, address)))
            .and_then(|(page, address)| map.location_at_long(page, address))
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
    // HSYNC width stays inside the line total.
    if r2 + r3_low < r0 {
        out.push(CrtcWarning {
            registers: &["R0", "R2", "R3"],
            severity: CrtcSeverity::Error,
            message: format!(
                "R2+(R3&0x0f) < R0 ({r2}+{r3_low}={} < {r0}): a CRTC type 2 loses horizontal \
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
            const MEANING: [&str; 18] = [
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
            let mut out = vec![
                byte(
                    "selected",
                    get(F::CRTC_SEL),
                    "the register &BCxx writes reach"
                ),
                byte("type", get(F::CRTC_TYPE), "which CRTC this machine has"),
            ];
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
                byte(&name, get(F::CRTC_REG(Some(i))), MEANING[i])
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

/// The example from the bug report: R0=63, R2=50, R3=0x8c loses sync
    /// (0x8c & 0x0f = 12, 50+12=62 < 63).
    #[test]
    fn a_known_sync_losing_combination_is_flagged() {
        let mut regs = [0u8; 18];
        regs[0] = 63;
        regs[2] = 50;
        regs[3] = 0x8c;
        let warnings = validate_crtc(&regs);
        let sync = warnings
            .iter()
            .find(|w| w.registers.contains(&"R2"))
            .expect("R2+(R3&0x0f) < R0 should be flagged");
        assert_eq!(sync.severity, CrtcSeverity::Error);
        assert!(sync.registers.contains(&"R0"));
        assert!(sync.registers.contains(&"R3"));
    }

    /// R0=63 exactly is the case the raster-timing rule exists to permit.
    #[test]
    fn r0_63_raises_no_line_duration_warning() {
        let mut regs = [0u8; 18];
        regs[0] = 63;
        regs[2] = 63;
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
        regs[2] = 50; // no sync loss on its own, isolating the R0 rule
        let warnings = validate_crtc(&regs);
        let duration = warnings
            .iter()
            .find(|w| w.registers == &["R0"])
            .expect("R0 != 63 should be flagged");
        assert_eq!(duration.severity, CrtcSeverity::Warning);
    }

    #[test]
    fn a_well_formed_configuration_is_not_flagged() {
        let mut regs = [0u8; 18];
        regs[0] = 63;
        // R2 + (R3 & 0x0f) = 50 + 13 = 63, exactly R0: not a sync loss.
        regs[2] = 50;
        regs[3] = 0x8d;
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
        let ambiguous = annotate_disassembly(&mut instructions, &map, None, None);

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
        annotate_disassembly(&mut instructions, &map, None, None);

        assert_eq!(instructions[0]["symbol"], json!("b"), "shortest first");
        assert_eq!(
            instructions[0]["symbolAlternatives"],
            json!(["cd", "table_data"]),
            "the rest, in the same preference order"
        );
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
        let ambiguous = annotate_disassembly(&mut instructions, &map, None, None);

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
        let ambiguous = annotate_disassembly(&mut instructions, &map, None, None);

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
        let ambiguous = annotate_disassembly(&mut instructions, &map, None, Some(&costs));

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
}
