#[derive(Debug, Clone, Copy)]
pub enum Charset {
    English
}

// Sadly several of these chars are considered to be strings in rust world :(
static CPC_ENGLISH_CHARSET: &[char] = &[
    '◻', '⎾', '⏊', '⏌', '⚡', '⊠', '✓', '⍾', '←', '→', '↓', '↑', '↡', '↲', '⊗', '⊙', '⊟', '◷', '◶',
    '◵', '◴', '⍻', '⎍', '⊣', '⧖', '⍿', '␦', '⊖', '◰', '◱', '◲', '◳', ' ', '!', '"', '#', '$', '%',
    '&', '’', '(', ')', '*', '+', ',', '-', '.', '/', '0', '1', '2', '3', '4', '5', '6', '7', '8',
    '9', ':', ';', '<', '=', '>', '?', '@', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K',
    'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '[', '\\', ']', '↑',
    '_', '`', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q',
    'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '{', '|', '}', '~',
    '␡', // TODO replace DEL by another char
    ' ', '▘', '▝', '▀', '▖', '▌', '▞', '▛', '▗', '▚', '▐', '▜', '▄', '▙', '▟', '█', '·', '╵', '╶',
    '└', '╷', '│', '┌', '├', '╴', '┘', '─', '┴', '┐', '┤', '┬', '┼', '^', '´', '¨', '£', '©', '¶',
    '§', '‘', '¼', '½', '¾', '±', '÷', '¬', '¿', '¡', 'α', 'β', 'γ', 'δ', 'ε', 'θ', 'λ', 'μ', 'π',
    'σ', 'φ', 'ψ', 'χ', 'ω', 'Σ', 'Ω', '🮠', '🮡', '🮣', '🮢', '🮧', '🮥', '🮦', '🮤', '🮨', '🮩', '🮮', '╳',
    '╱', '╲', '🮕', '▒', '▔', '▕', '▁', '▏', '◤', '◥', '◢', '◣', '🮎', '🮍', '🮏', '🮌', '🮜', '🮝', '🮞',
    '🮟', '☺', '☹', '♣', '♦', '♥', '♠', '○', '●', '□', '■', '♂', '♀', '♩', '♪', '☼', '🙭', '⭡', '⭣',
    '⭠', '⭢', '▲', '▼', '▶', '◀', '🯆', '🯅', '🯇', '🯈', '🙯', '🛧', '⭥', '⭤'
];

impl Charset {
    pub fn name(&self) -> &str {
        match self {
            Self::English => "English"
        }
    }

    pub fn chars_in_strings(&self) -> &[char] {
        match self {
            Self::English => CPC_ENGLISH_CHARSET
        }
    }
}

pub fn char_to_amscii(c: char, charset: Charset) -> Option<u8> {
    charset
        .chars_in_strings()
        .iter()
        .skip(32) // some codes use the same representation than the real chars_in_strings
        .position(|s| c == *s)
        .map(|idx| idx as u8 + 32)
}

pub fn str_to_amscii(s: &str, charset: Charset) -> Result<Vec<u8>, String> {
    s.chars()
        .map(move |c| {
            char_to_amscii(c, charset).ok_or_else(|| {
                format!(
                    "{c} has no correspondance in the {} charset",
                    charset.name()
                )
            })
        })
        .collect()
}
#[cfg(test)]
mod test {
    use itertools::Itertools;

    use crate::chars::{char_to_amscii, str_to_amscii, Charset};

    #[test]
    fn test_english_code() {
        assert_eq!(
            char_to_amscii('▟', crate::chars::Charset::English),
            Some(0x8E)
        );

        assert_eq!(
            char_to_amscii('▌', crate::chars::Charset::English),
            Some(0x85)
        );
    }

    #[test]
    fn test_english_string() {
        assert_eq!(
            str_to_amscii(" !\"#$%&’()*+,-./", Charset::English),
            Ok((0x20..=0x2F).collect_vec())
        );

        assert_eq!(
            str_to_amscii("0123456789:;<=>?", Charset::English),
            Ok((0x30..=0x3F).collect_vec())
        );

        assert_eq!(
            str_to_amscii("@ABCDEFGHIJKLMNO", Charset::English),
            Ok((0x40..=0x4F).collect_vec())
        );

        assert_eq!(
            str_to_amscii("PQRSTUVWXYZ[\\]↑_", Charset::English),
            Ok((0x50..=0x5F).collect_vec())
        );

        assert_eq!(
            str_to_amscii("`abcdefghijklmno", Charset::English),
            Ok((0x60..=0x6F).collect_vec())
        );

        // here we skip 127/DEL
        assert_eq!(
            str_to_amscii("pqrstuvwxyz{|}~␡", Charset::English),
            Ok((0x70..=0x7F).collect_vec())
        );

        // here we skip the duplicated space 128
        assert_eq!(
            str_to_amscii("▘▝▀▖▌▞▛▗▚▐▜▄▙▟█", Charset::English),
            Ok((0x81..=0x8F).collect_vec())
        );

        assert_eq!(
            str_to_amscii("·╵╶└╷│┌├╴┘─┴┐┤┬┼", Charset::English),
            Ok((0x90..=0x9F).collect_vec())
        );

        assert_eq!(
            str_to_amscii("^´¨£©¶§‘¼½¾±÷¬¿¡", Charset::English),
            Ok((0xA0..=0xAF).collect_vec())
        );

        assert_eq!(
            str_to_amscii("αβγδεθλμπσφψχωΣΩ", Charset::English),
            Ok((0xB0..=0xBF).collect_vec())
        );

        assert_eq!(
            str_to_amscii("🮠🮡🮣🮢🮧🮥🮦🮤🮨🮩🮮╳╱╲🮕▒", Charset::English),
            Ok((0xC0..=0xCF).collect_vec())
        );

        assert_eq!(
            str_to_amscii("▔▕▁▏◤◥◢◣🮎🮍🮏🮌🮜🮝🮞🮟", Charset::English),
            Ok((0xD0..=0xDF).collect_vec())
        );

        assert_eq!(
            str_to_amscii("☺☹♣♦♥♠○●□■♂♀♩♪☼🙭", Charset::English),
            Ok((0xE0..=0xEF).collect_vec())
        );

        assert_eq!(
            str_to_amscii("⭡⭣⭠⭢▲▼▶◀🯆🯅🯇🯈🙯🛧⭥⭤", Charset::English),
            Ok((0xF0..=0xFF).collect_vec())
        );
    }
}
