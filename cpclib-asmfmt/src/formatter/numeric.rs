use super::Formatter;
use crate::options::{CaseStyle, HexEncoding, OctalEncoding, BinaryEncoding};
use cpclib_common::parse::{EncodingKind, scan_numeric_literals};

impl<'src> Formatter<'src> {
    // Reformat all numeric literals in `content` according to the hex/oct/bin encoding and
    // hex case settings.  When all four settings are Untouched the string is returned as-is.
    pub(super) fn reformat_numeric_literals(&self, content: &str) -> String {
        if matches!(self.hexadecimal_case, CaseStyle::Untouched)
            && matches!(self.hexadecimal_encoding, HexEncoding::Untouched)
            && matches!(self.octal_encoding, OctalEncoding::Untouched)
            && matches!(self.binary_encoding, BinaryEncoding::Untouched)
        {
            return content.to_string();
        }

        let spans = scan_numeric_literals(content);
        if spans.is_empty() {
            return content.to_string();
        }

        let mut result = String::with_capacity(content.len());
        let mut cursor = 0usize;
        for (start, end, value, kind) in spans {
            result.push_str(&content[cursor..start]);
            let original = &content[start..end];
            result.push_str(&self.reformat_number(value, kind, original));
            cursor = end;
        }
        result.push_str(&content[cursor..]);
        result
    }

    fn reformat_number(&self, value: u32, kind: EncodingKind, original: &str) -> String {
        match kind {
            EncodingKind::Hex => self.reformat_hex(value, original),
            EncodingKind::Oct => self.reformat_oct(value, original),
            EncodingKind::Bin => self.reformat_bin(value, original),
            _ => original.to_string(), // Dec and internal states: unchanged
        }
    }

    fn reformat_hex(&self, value: u32, original: &str) -> String {
        let enc = self.hexadecimal_encoding;
        let case = self.hexadecimal_case;

        if matches!(enc, HexEncoding::Untouched) && matches!(case, CaseStyle::Untouched) {
            return original.to_string();
        }

        if matches!(enc, HexEncoding::Untouched) {
            // Only change letter case; preserve prefix/suffix verbatim.
            return original.chars().map(|c| match c {
                'a'..='f' | 'A'..='F' => match case {
                    CaseStyle::UpperCase => c.to_ascii_uppercase(),
                    CaseStyle::LowerCase => c.to_ascii_lowercase(),
                    CaseStyle::Untouched => c,
                },
                _ => c,
            }).collect();
        }

        // Re-encode: format value as hex digits with the requested case.
        let raw = format!("{:X}", value); // always uppercase first
        let digits: String = raw.chars().map(|c| match case {
            CaseStyle::LowerCase => c.to_ascii_lowercase(),
            _ => c, // UpperCase or Untouched → uppercase
        }).collect();

        let is_suffix = matches!(enc, HexEncoding::SuffixLower | HexEncoding::SuffixUpper);
        // Suffix form must start with a digit to avoid being parsed as an identifier.
        let digits = if is_suffix && digits.chars().next().map_or(false, |c| c.is_ascii_alphabetic()) {
            format!("0{digits}")
        } else {
            digits
        };

        match enc {
            HexEncoding::Prefix0x     => format!("0x{digits}"),
            HexEncoding::Prefix0X     => format!("0X{digits}"),
            HexEncoding::PrefixHash   => format!("#{digits}"),
            HexEncoding::PrefixDollar => format!("${digits}"),
            HexEncoding::PrefixAmp    => format!("&{digits}"),
            HexEncoding::SuffixLower  => format!("{digits}h"),
            HexEncoding::SuffixUpper  => format!("{digits}H"),
            HexEncoding::Untouched    => unreachable!(),
        }
    }

    fn reformat_oct(&self, value: u32, original: &str) -> String {
        match self.octal_encoding {
            OctalEncoding::Untouched => original.to_string(),
            OctalEncoding::Prefix0o  => format!("0o{:o}", value),
            OctalEncoding::Prefix0O  => format!("0O{:o}", value),
            OctalEncoding::PrefixAt  => format!("@{:o}", value),
        }
    }

    fn reformat_bin(&self, value: u32, original: &str) -> String {
        match self.binary_encoding {
            BinaryEncoding::Untouched      => original.to_string(),
            BinaryEncoding::Prefix0b       => format!("0b{:b}", value),
            BinaryEncoding::Prefix0B       => format!("0B{:b}", value),
            BinaryEncoding::PrefixPercent  => format!("%{:b}", value),
        }
    }
}
