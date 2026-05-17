use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use cpclib_asm::preamble::*;

#[derive(Debug, Clone, PartialEq, Eq)]
enum NopsSpec {
    Fixed(usize),
    Variable(String)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TimingEntry {
    section: String,
    mnemonic: String,
    opcodes: String,
    flags: String,
    nops: NopsSpec
}

#[derive(Debug, Clone, Copy)]
struct TimingCase {
    timing_mnemonic: &'static str,
    asm: &'static str
}

fn timings_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("timings.txt")
}

fn parse_timings_file() -> Vec<TimingEntry> {
    let content = fs::read_to_string(timings_path()).expect("Unable to read timings.txt");
    let mut section = String::new();
    let mut entries = Vec::new();

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        if !line.contains('|') {
            if !line.starts_with(';') {
                section = line.to_owned();
            }
            continue;
        }

        if line.starts_with(';') {
            continue;
        }

        let mut columns = line.split('|').map(str::trim);
        let mnemonic = columns.next().unwrap_or_default();
        let opcodes = columns.next().unwrap_or_default();
        let flags = columns.next().unwrap_or_default();
        let nops = columns.next().unwrap_or_default();

        if mnemonic.is_empty() || opcodes.is_empty() || flags.is_empty() || nops.is_empty() {
            continue;
        }

        let nops = nops.split(';').next().unwrap_or_default().trim();

        let nops = match nops.parse::<usize>() {
            Ok(value) => NopsSpec::Fixed(value),
            Err(_) => NopsSpec::Variable(nops.to_owned())
        };

        entries.push(TimingEntry {
            section: section.clone(),
            mnemonic: mnemonic.to_owned(),
            opcodes: opcodes.to_owned(),
            flags: flags.to_owned(),
            nops
        });
    }

    entries
}

fn nops_candidates(spec: &NopsSpec) -> Vec<usize> {
    match spec {
        NopsSpec::Fixed(value) => vec![*value],
        NopsSpec::Variable(text) => {
            let mut values = Vec::new();
            let mut current = String::new();
            for ch in text.chars() {
                if ch.is_ascii_digit() {
                    current.push(ch);
                }
                else if !current.is_empty() {
                    values.push(current.parse::<usize>().unwrap());
                    current.clear();
                }
            }

            if !current.is_empty() {
                values.push(current.parse::<usize>().unwrap());
            }

            values
        }
    }
}

fn timing_entry_by_mnemonic<'a>(entries: &'a [TimingEntry], mnemonic: &str) -> &'a TimingEntry {
    entries
        .iter()
        .find(|entry| entry.mnemonic == mnemonic)
        .unwrap_or_else(|| panic!("Missing timing entry for {}", mnemonic))
}

fn timing_cases() -> &'static [TimingCase] {
    &[
        TimingCase { timing_mnemonic: "ld r,r'", asm: "ld b,c" },
        TimingCase { timing_mnemonic: "ld r,(hl)", asm: "ld a,(hl)" },
        TimingCase { timing_mnemonic: "ld r,(ix+n)", asm: "ld l,(ix+0)" },
        TimingCase { timing_mnemonic: "ld r,n", asm: "ld b,0" },
        TimingCase { timing_mnemonic: "ld (hl),r", asm: "ld (hl),e" },
        TimingCase { timing_mnemonic: "ld (ix+n),r", asm: "ld (ix+0),e" },
        TimingCase { timing_mnemonic: "ld (hl),n", asm: "ld (hl),0" },
        TimingCase { timing_mnemonic: "ld a,(bc)", asm: "ld a,(bc)" },
        TimingCase { timing_mnemonic: "ld a,(de)", asm: "ld a,(de)" },
        TimingCase { timing_mnemonic: "ld a,(nn)", asm: "ld a,(0x1234)" },
        TimingCase { timing_mnemonic: "ld (bc),a", asm: "ld (bc),a" },
        TimingCase { timing_mnemonic: "ld (de),a", asm: "ld (de),a" },
        TimingCase { timing_mnemonic: "ld (nn),a", asm: "ld (0x1234),a" },
        TimingCase { timing_mnemonic: "ld i,a", asm: "ld i,a" },
        TimingCase { timing_mnemonic: "ld r,a", asm: "ld r,a" },
        TimingCase { timing_mnemonic: "ld a,i", asm: "ld a,i" },
        TimingCase { timing_mnemonic: "ld a,r", asm: "ld a,r" },
        TimingCase { timing_mnemonic: "ld rr,nn", asm: "ld bc,0x1234" },
        TimingCase { timing_mnemonic: "ld ix,nn", asm: "ld ix,0x1234" },
        TimingCase { timing_mnemonic: "ld hl,(nn)", asm: "ld hl,(0x1234)" },
        TimingCase { timing_mnemonic: "ld ix,(nn)", asm: "ld ix,(0x1234)" },
        TimingCase { timing_mnemonic: "ld rr,(nn)", asm: "ld de,(0x1234)" },
        TimingCase { timing_mnemonic: "ld (nn),hl", asm: "ld (0x1234),hl" },
        TimingCase { timing_mnemonic: "ld (nn),ix", asm: "ld (0x1234),ix" },
        TimingCase { timing_mnemonic: "ld (nn),rr", asm: "ld (0x1234),de" },
        TimingCase { timing_mnemonic: "ld sp,hl", asm: "ld sp,hl" },
        TimingCase { timing_mnemonic: "ld sp,ix", asm: "ld sp,ix" },
        TimingCase { timing_mnemonic: "push qq", asm: "push bc" },
        TimingCase { timing_mnemonic: "push ix", asm: "push ix" },
        TimingCase { timing_mnemonic: "pop qq", asm: "pop bc" },
        TimingCase { timing_mnemonic: "pop ix", asm: "pop ix" },
        TimingCase { timing_mnemonic: "ex de,hl", asm: "ex de,hl" },
        TimingCase { timing_mnemonic: "ex af,af'", asm: "ex af,af'" },
        TimingCase { timing_mnemonic: "exx", asm: "exx" },
        TimingCase { timing_mnemonic: "ex (sp),hl", asm: "ex (sp),hl" },
        TimingCase { timing_mnemonic: "ex (sp),ix", asm: "ex (sp),ix" },
        TimingCase { timing_mnemonic: "ldi", asm: "ldi" },
        TimingCase { timing_mnemonic: "ldd", asm: "ldd" },
        TimingCase { timing_mnemonic: "ldir", asm: "ldir" },
        TimingCase { timing_mnemonic: "lddr", asm: "lddr" },
        TimingCase { timing_mnemonic: "cpi", asm: "cpi" },
        TimingCase { timing_mnemonic: "cpd", asm: "cpd" },
        TimingCase { timing_mnemonic: "cpir", asm: "cpir" },
        TimingCase { timing_mnemonic: "cpdr", asm: "cpdr" },
        TimingCase { timing_mnemonic: "add r", asm: "add a,c" },
        TimingCase { timing_mnemonic: "add (hl)", asm: "add a,(hl)" },
        TimingCase { timing_mnemonic: "add (ix+n)", asm: "add a,(ix+0)" },
        TimingCase { timing_mnemonic: "add n", asm: "add a,0" },
        TimingCase { timing_mnemonic: "adc r", asm: "adc a,c" },
        TimingCase { timing_mnemonic: "sub r", asm: "sub c" },
        TimingCase { timing_mnemonic: "sbc r", asm: "sbc a,c" },
        TimingCase { timing_mnemonic: "and r", asm: "and c" },
        TimingCase { timing_mnemonic: "or r", asm: "or c" },
        TimingCase { timing_mnemonic: "xor r", asm: "xor c" },
        TimingCase { timing_mnemonic: "cp  r", asm: "cp c" },
        TimingCase { timing_mnemonic: "inc r", asm: "inc c" },
        TimingCase { timing_mnemonic: "inc (hl)", asm: "inc (hl)" },
        TimingCase { timing_mnemonic: "inc (ix+n)", asm: "inc (ix+0)" },
        TimingCase { timing_mnemonic: "dec r", asm: "dec c" },
        TimingCase { timing_mnemonic: "dec (hl)", asm: "dec (hl)" },
        TimingCase { timing_mnemonic: "dec (ix+n)", asm: "dec (ix+0)" },
        TimingCase { timing_mnemonic: "add hl,rr", asm: "add hl,bc" },
        TimingCase { timing_mnemonic: "add ix,rr", asm: "add ix,bc" },
        TimingCase { timing_mnemonic: "adc hl,rr", asm: "adc hl,bc" },
        TimingCase { timing_mnemonic: "sbc hl,rr", asm: "sbc hl,bc" },
        TimingCase { timing_mnemonic: "inc rr", asm: "inc bc" },
        TimingCase { timing_mnemonic: "inc ix", asm: "inc ix" },
        TimingCase { timing_mnemonic: "dec rr", asm: "dec bc" },
        TimingCase { timing_mnemonic: "dec ix", asm: "dec ix" },
        TimingCase { timing_mnemonic: "daa", asm: "daa" },
        TimingCase { timing_mnemonic: "neg", asm: "neg" },
        TimingCase { timing_mnemonic: "cpl", asm: "cpl" },
        TimingCase { timing_mnemonic: "ccf", asm: "ccf" },
        TimingCase { timing_mnemonic: "scf", asm: "scf" },
        TimingCase { timing_mnemonic: "nop", asm: "nop" },
        TimingCase { timing_mnemonic: "halt", asm: "halt" },
        TimingCase { timing_mnemonic: "di", asm: "di" },
        TimingCase { timing_mnemonic: "ei", asm: "ei" },
        TimingCase { timing_mnemonic: "im 0", asm: "im 0" },
        TimingCase { timing_mnemonic: "im 1", asm: "im 1" },
        TimingCase { timing_mnemonic: "im 2", asm: "im 2" },
        TimingCase { timing_mnemonic: "rlca", asm: "rlca" },
        TimingCase { timing_mnemonic: "rrca", asm: "rrca" },
        TimingCase { timing_mnemonic: "rla", asm: "rla" },
        TimingCase { timing_mnemonic: "rra", asm: "rra" },
        TimingCase { timing_mnemonic: "rlc r", asm: "rlc c" },
        TimingCase { timing_mnemonic: "rlc (hl)", asm: "rlc (hl)" },
        TimingCase { timing_mnemonic: "rlc (ix+n)", asm: "rlc (ix+0)" },
        TimingCase { timing_mnemonic: "rlc (ix+n),r", asm: "rlc (ix+0),b" },
        TimingCase { timing_mnemonic: "rrc r", asm: "rrc c" },
        TimingCase { timing_mnemonic: "rl r", asm: "rl c" },
        TimingCase { timing_mnemonic: "rr r", asm: "rr c" },
        TimingCase { timing_mnemonic: "sla r", asm: "sla c" },
        TimingCase { timing_mnemonic: "sra r", asm: "sra c" },
        TimingCase { timing_mnemonic: "sl1 r", asm: "sll c" },
        TimingCase { timing_mnemonic: "srl r", asm: "srl c" },
        TimingCase { timing_mnemonic: "rld", asm: "rld" },
        TimingCase { timing_mnemonic: "rrd", asm: "rrd" },
        TimingCase { timing_mnemonic: "bit b,r", asm: "bit 3,c" },
        TimingCase { timing_mnemonic: "bit b,(hl)", asm: "bit 3,(hl)" },
        TimingCase { timing_mnemonic: "bit b,(ix+n)", asm: "bit 3,(ix+0)" },
        TimingCase { timing_mnemonic: "res b,r", asm: "res 3,c" },
        TimingCase { timing_mnemonic: "res b,(hl)", asm: "res 3,(hl)" },
        TimingCase { timing_mnemonic: "res b,(ix+n)", asm: "res 3,(ix+0)" },
        TimingCase { timing_mnemonic: "res b,(ix+n),r", asm: "res 3,(ix+0),a" },
        TimingCase { timing_mnemonic: "set b,r", asm: "set 3,c" },
        TimingCase { timing_mnemonic: "set b,(hl)", asm: "set 3,(hl)" },
        TimingCase { timing_mnemonic: "set b,(ix+n)", asm: "set 3,(ix+0)" },
        TimingCase { timing_mnemonic: "set b,(ix+n),r", asm: "set 3,(ix+0),a" },
        TimingCase { timing_mnemonic: "jp nn", asm: "jp 0x1234" },
        TimingCase { timing_mnemonic: "jp ccc,nn", asm: "jp nz,0x1234" },
        TimingCase { timing_mnemonic: "jp hl", asm: "jp (hl)" },
        TimingCase { timing_mnemonic: "jp ix", asm: "jp (ix)" },
        TimingCase { timing_mnemonic: "jr n", asm: "jr 0" },
        TimingCase { timing_mnemonic: "jr cc,nn", asm: "jr nz,0" },
        TimingCase { timing_mnemonic: "djnz n", asm: "djnz 0" },
        TimingCase { timing_mnemonic: "call nn", asm: "call 0x1234" },
        TimingCase { timing_mnemonic: "call ccc,nn", asm: "call nz,0x1234" },
        TimingCase { timing_mnemonic: "rst ttt", asm: "rst 0" },
        TimingCase { timing_mnemonic: "ret", asm: "ret" },
        TimingCase { timing_mnemonic: "ret ccc", asm: "ret nz" },
        TimingCase { timing_mnemonic: "reti", asm: "reti" },
        TimingCase { timing_mnemonic: "retn", asm: "retn" },
        TimingCase { timing_mnemonic: "in a,(n)", asm: "in a,(0)" },
        TimingCase { timing_mnemonic: "in r,(C)", asm: "in b,(c)" },
        TimingCase { timing_mnemonic: "in 0,(C)", asm: "in 0,(c)" },
        TimingCase { timing_mnemonic: "ini", asm: "ini" },
        TimingCase { timing_mnemonic: "ind", asm: "ind" },
        TimingCase { timing_mnemonic: "inir", asm: "inir" },
        TimingCase { timing_mnemonic: "indr", asm: "indr" },
        TimingCase { timing_mnemonic: "out (n),a", asm: "out (0),a" },
        TimingCase { timing_mnemonic: "out (C),r", asm: "out (c),b" },
        TimingCase { timing_mnemonic: "out (C),0", asm: "out (c),0" },
        TimingCase { timing_mnemonic: "outi", asm: "outi" },
        TimingCase { timing_mnemonic: "outd", asm: "outd" },
        TimingCase { timing_mnemonic: "outir", asm: "otir" },
        TimingCase { timing_mnemonic: "outdr", asm: "otdr" }
    ]
}

#[test]
fn parse_timings_file_collects_high_level_entries() {
    let entries = parse_timings_file();

    assert!(entries.len() > 100, "Expected to parse many timing rows, got {}", entries.len());
    assert!(entries.iter().any(|entry| {
        entry.section == "8-bit Load"
            && entry.mnemonic == "ld r,r'"
            && entry.nops == NopsSpec::Fixed(1)
    }));
    assert!(entries.iter().any(|entry| {
        entry.section == "Jump, Call and Return"
            && entry.mnemonic == "jr cc,nn"
            && matches!(entry.nops, NopsSpec::Variable(_))
    }));
}

#[test]
fn timing_cases_cover_all_documented_mnemonics() {
    let entries = parse_timings_file();
    let documented: BTreeSet<_> = entries.iter().map(|entry| entry.mnemonic.as_str()).collect();
    let covered: BTreeSet<_> = timing_cases()
        .iter()
        .map(|case| case.timing_mnemonic)
        .collect();

    let missing: Vec<_> = documented.difference(&covered).copied().collect();
    let extra: Vec<_> = covered.difference(&documented).copied().collect();

    assert!(
        missing.is_empty() && extra.is_empty(),
        "Timing coverage mismatch. Missing: {:?}. Extra: {:?}.",
        missing,
        extra
    );
}

#[test]
fn estimated_duration_matches_documented_timings() {
    let entries = parse_timings_file();

    for case in timing_cases() {
        let entry = timing_entry_by_mnemonic(&entries, case.timing_mnemonic);
        let expected = nops_candidates(&entry.nops);
        let token = Token::parse_token(case.asm)
            .unwrap_or_else(|err| panic!("Unable to parse `{}`: {}", case.asm, err));
        let actual = token
            .estimated_duration()
            .unwrap_or_else(|err| panic!("Unable to compute duration for `{}`: {}", case.asm, err));

        assert!(
            expected.contains(&actual),
            "Duration mismatch for `{}` (timings row `{}`): got {}, expected one of {:?}",
            case.asm,
            case.timing_mnemonic,
            actual,
            expected
        );
    }
}