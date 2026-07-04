use super::Formatter;
use crate::options::CaseStyle;

impl<'src> Formatter<'src> {
    pub(super) fn apply_case(text: &str, case: CaseStyle) -> String {
        match case {
            CaseStyle::UpperCase => text.to_ascii_uppercase(),
            CaseStyle::LowerCase => text.to_ascii_lowercase(),
            CaseStyle::Untouched => text.to_string()
        }
    }

    // Apply case to the first whitespace-delimited word only; rest is preserved verbatim.
    // Also normalises any run of whitespace between the keyword and its arguments to a single
    // space so that "ORG  40" → "ORG 40".
    pub(super) fn apply_case_to_first_word(content: &str, case: CaseStyle) -> String {
        let word_end = content
            .find(|c: char| c.is_ascii_whitespace())
            .unwrap_or(content.len());
        let keyword = Self::apply_case(&content[..word_end], case);
        let rest = content[word_end..].trim_start();
        if rest.is_empty() {
            keyword
        }
        else {
            format!("{keyword} {rest}")
        }
    }

    // Apply case to the second whitespace-delimited word (e.g., the EQU keyword in
    // "symbol EQU value"), leaving the first word (user symbol name) unchanged.
    // Also normalises inter-word whitespace to a single space.
    pub(super) fn apply_case_to_second_word(content: &str, case: CaseStyle) -> String {
        let bytes = content.as_bytes();
        let first_end = bytes
            .iter()
            .position(|b| b.is_ascii_whitespace())
            .unwrap_or(bytes.len());
        let second_start = bytes[first_end..]
            .iter()
            .position(|b| !b.is_ascii_whitespace())
            .map(|p| first_end + p)
            .unwrap_or(bytes.len());
        let second_end = bytes[second_start..]
            .iter()
            .position(|b| b.is_ascii_whitespace())
            .map(|p| second_start + p)
            .unwrap_or(bytes.len());
        let symbol = &content[..first_end];
        let keyword = Self::apply_case(&content[second_start..second_end], case);
        let rest = content[second_end..].trim_start();
        if rest.is_empty() {
            format!("{symbol} {keyword}")
        }
        else {
            format!("{symbol} {keyword} {rest}")
        }
    }

    // Apply case to a mnemonic line: transforms the mnemonic keyword and register names
    // in operands but leaves numeric literals / labels / expressions unchanged.
    // Also normalises the whitespace between mnemonic and operands to a single space.
    pub(super) fn apply_mnemonic_case(
        content: &str,
        mnemonic_case: CaseStyle,
        register_case: CaseStyle
    ) -> String {
        let word_end = content
            .find(|c: char| c.is_ascii_whitespace())
            .unwrap_or(content.len());
        let mnemonic = Self::apply_case(&content[..word_end], mnemonic_case);
        let rest = content[word_end..].trim_start();
        if rest.is_empty() {
            mnemonic
        }
        else {
            let operands = if matches!(register_case, CaseStyle::Untouched) {
                rest.to_string()
            }
            else {
                Self::apply_register_case(rest, register_case)
            };
            format!("{mnemonic} {operands}")
        }
    }

    pub(super) fn apply_register_case(operands: &str, case: CaseStyle) -> String {
        const REGISTERS: &[&str] = &[
            "AF'", "IXH", "IXL", "IYH", "IYL", "AF", "BC", "DE", "HL", "IX", "IY", "SP", "PC", "A",
            "B", "C", "D", "E", "H", "L", "F", "I", "R"
        ];
        let is_ident = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
        let bytes = operands.as_bytes();
        let mut result = String::with_capacity(operands.len());
        let mut i = 0;
        while i < bytes.len() {
            let b = bytes[i];
            let prev_ok = i == 0 || !is_ident(bytes[i - 1]);
            if prev_ok && b.is_ascii_alphabetic() {
                let mut matched = false;
                for &reg in REGISTERS {
                    let reg_bytes = reg.as_bytes();
                    if bytes[i..].len() >= reg_bytes.len()
                        && bytes[i..i + reg_bytes.len()].eq_ignore_ascii_case(reg_bytes)
                    {
                        let after = i + reg_bytes.len();
                        let next_ok = after >= bytes.len() || !is_ident(bytes[after]);
                        if next_ok {
                            result.push_str(&Self::apply_case(&operands[i..after], case));
                            i = after;
                            matched = true;
                            break;
                        }
                    }
                }
                if !matched {
                    result.push(b as char);
                    i += 1;
                }
            }
            else {
                result.push(b as char);
                i += 1;
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_label_not_treated_as_register() {
        let r1 = Formatter::apply_register_case("hl, bc_label", CaseStyle::UpperCase);
        assert_eq!(r1, "HL, bc_label", "bc_label was altered: {r1:?}");
        let r2 = Formatter::apply_register_case("a, hlabel", CaseStyle::UpperCase);
        assert_eq!(r2, "A, hlabel", "hlabel was altered: {r2:?}");
        let r3 = Formatter::apply_register_case("hl, bc", CaseStyle::UpperCase);
        assert_eq!(r3, "HL, BC", "registers not uppercased: {r3:?}");
    }
}
