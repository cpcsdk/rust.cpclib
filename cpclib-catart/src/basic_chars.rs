//! Amstrad CPC character set constants
//!
//! This module provides constants for Amstrad CPC characters that have
//! Locomotive BASIC equivalents, based on the character set defined at:
//! https://en.wikipedia.org/wiki/Amstrad_CPC_character_set
//!
//! The control characters (0x00-0x1F) have special functions and many
//! have corresponding BASIC commands.

use paste::paste;

/// Macro to define character constants with their BASIC equivalents
///
/// # Arguments
/// * `hex_value` - The hexadecimal value of the character
/// * `const_name` - The Rust constant name
/// * `basic_equiv` - The Locomotive BASIC equivalent (use underscores for spaces)
/// * `function_doc` - A string describing the character's function
///
/// # Example
/// ```ignore
/// define_basic_char!(0x02, STX, CURSOR_0, "Turn off text cursor");
/// ```
macro_rules! define_basic_char {
    ($hex_value:expr, $const_name:ident =$nb:expr, $basic_equiv:ident, $function_doc:expr) => {
        #[doc = concat!(
            "Character code `0x",
            stringify!($hex_value),
            "` - BASIC equivalent: `",
            stringify!($basic_equiv),
            "`\n\n",
            $function_doc
        )]
        pub const $const_name: u8 = $hex_value;
        pub const $basic_equiv: u8 = $const_name;

        paste::item! {
            const [<__CHAR_PARAMS_ $hex_value __> ]: u8 = $nb;
        }
    };

    ($hex_value:expr, $const_name:ident, $basic_equiv:ident, $function_doc:expr) => {
        define_basic_char!($hex_value, $const_name = 0, $basic_equiv, $function_doc);
        
    };


    ($hex_value:expr, $const_name:ident, $function_doc:expr) => {
        define_basic_char!($hex_value, $const_name = 0, $function_doc);
    };

    ($hex_value:expr, $const_name:ident = $nb:expr, $function_doc:expr) => {
        #[doc = concat!(
            "Character code `0x",
            stringify!($hex_value),
            "`\n\n",
            $function_doc
        )]
        pub const $const_name: u8 = $hex_value;

        paste::item! {
            const [<__CHAR_PARAMS_ $hex_value __> ]: u8 = $nb;
        }
    };
}

// Control characters with Locomotive BASIC equivalents (0x00-0x1F)

define_basic_char!(0x00, NUL, "No effect. Ignored.");
define_basic_char!(0x01, SOH = 1, "Print a specific character symbol (parameter: 0-255). This allows the symbols in the range 0 to 31 to be printed.");
define_basic_char!(0x02, STX, CURSOR_0, "Turn off text cursor.");
define_basic_char!(0x03, ETX, CURSOR_1, "Turn on text cursor.");
define_basic_char!(0x04, EOT = 1, MODE, "Set screen mode (parameter: 0-2).");
define_basic_char!(0x05, ENQ = 1, "Send the parameter character to the graphics cursor");
define_basic_char!(0x06, ACK, "Enable text screen.");
define_basic_char!(0x07, BEL, "Beep.");
define_basic_char!(0x08, BS, "BackSpace / Cursor Left.");
define_basic_char!(0x09, TAB, "Cursor Right / Tab.");
define_basic_char!(0x0A, LF, "Line Feed / Cursor Down.");
define_basic_char!(0x0B, VT, "Vertical Tab / Cursor Up.");
define_basic_char!(0x0C, FF, CLS, "Clear text window and move cursor to top left corner.");
define_basic_char!(0x0D, CR, "Carriage Return. Left edge of window on current line");
define_basic_char!(0x0E, SO = 1, PAPER, "Set Paper Ink (parameter: 0-15).");
define_basic_char!(0x0F, SI = 1, PEN, "Set Pen Ink (parameter: 0-15).");
define_basic_char!(0x10, DLE, "Delete character under cursor.");
define_basic_char!(0x11, DC1, "Clear to start of line.");
define_basic_char!(0x12, DC2, "Clear to end of line.");
define_basic_char!(0x13, DC3, "Clear to start of screen.");
define_basic_char!(0x14, DC4, "Clear to end of screen.");
define_basic_char!(0x15, NAK,  "Disable text screen.");
define_basic_char!(0x16, SYN = 1, "Set transparency 0 disable 1 enable");
define_basic_char!(0x17, ETB = 1, "Set graphics ink mode. 0 normal, 1 XOR, 2 AND, 3 OR");
define_basic_char!(0x18, CAN, "Exchange Pen and Paper inks (Reverse).");
define_basic_char!(0x19, EM = 9, SYMBOL, "Set the matrix for user definable character (9 parameters).");
define_basic_char!(0x1A, SUB = 4, WINDOW, "Set Window (parameters: left, right, top, bottom edges).");
define_basic_char!(0x1B, ESC, "No effecT. Ignored");
define_basic_char!(0x1C, FS= 3, INK, "Set Ink to a pair of colors (3 parameters).");
define_basic_char!(0x1D, GS = 2, BORDER, "Set Border to a pair of colors (2 parameters).");
define_basic_char!(0x1E, RS, "Move cursor to top left corner.");
define_basic_char!(0x1F, US = 2, LOCATE, "Move cursor to position (parameters: column, line).");

// Build NB_PARAMS_FOR_CODE at compile time using the __CHAR_PARAMS_XX constants
pub(crate) const NB_PARAMS_FOR_CODE: &[u8; 32] = &[
    __CHAR_PARAMS_0x00__,
    __CHAR_PARAMS_0x01__,
    __CHAR_PARAMS_0x02__,
    __CHAR_PARAMS_0x03__,
    __CHAR_PARAMS_0x04__,
    __CHAR_PARAMS_0x05__,
    __CHAR_PARAMS_0x06__,
    __CHAR_PARAMS_0x07__,
    __CHAR_PARAMS_0x08__,
    __CHAR_PARAMS_0x09__,
    __CHAR_PARAMS_0x0A__,
    __CHAR_PARAMS_0x0B__,
    __CHAR_PARAMS_0x0C__,
    __CHAR_PARAMS_0x0D__,
    __CHAR_PARAMS_0x0E__,
    __CHAR_PARAMS_0x0F__,
    __CHAR_PARAMS_0x10__,
    __CHAR_PARAMS_0x11__,
    __CHAR_PARAMS_0x12__,
    __CHAR_PARAMS_0x13__,
    __CHAR_PARAMS_0x14__,
    __CHAR_PARAMS_0x15__,
    __CHAR_PARAMS_0x16__,
    __CHAR_PARAMS_0x17__,
    __CHAR_PARAMS_0x18__,
    __CHAR_PARAMS_0x19__,
    __CHAR_PARAMS_0x1A__,
    __CHAR_PARAMS_0x1B__,
    __CHAR_PARAMS_0x1C__,
    __CHAR_PARAMS_0x1D__,
    __CHAR_PARAMS_0x1E__,
    __CHAR_PARAMS_0x1F__,
];

// Standard printable ASCII range (0x20-0x7E)
// These are standard ASCII and don't need special BASIC equivalents
// but are included for completeness



/// Character code `0x20` - Space
pub const SPACE: u8 = 0x20;

/// Character code `0x21` - Exclamation mark `!`
pub const EXCLAMATION: u8 = 0x21;

/// Character code `0x22` - Double quote `"`
pub const DOUBLE_QUOTE: u8 = 0x22;

/// Character code `0x23` - Hash/Number sign `#`
pub const HASH: u8 = 0x23;

/// Character code `0x24` - Dollar sign `$`
pub const DOLLAR: u8 = 0x24;

/// Character code `0x25` - Percent sign `%`
pub const PERCENT: u8 = 0x25;

/// Character code `0x26` - Ampersand `&`
pub const AMPERSAND: u8 = 0x26;

/// Character code `0x27` - Single quote/Apostrophe `'`
pub const SINGLE_QUOTE: u8 = 0x27;

/// Character code `0x28` - Left parenthesis `(`
pub const LEFT_PAREN: u8 = 0x28;

/// Character code `0x29` - Right parenthesis `)`
pub const RIGHT_PAREN: u8 = 0x29;

/// Character code `0x2A` - Asterisk `*`
pub const ASTERISK: u8 = 0x2A;

/// Character code `0x2B` - Plus sign `+`
pub const PLUS: u8 = 0x2B;

/// Character code `0x2C` - Comma `,`
pub const COMMA: u8 = 0x2C;

/// Character code `0x2D` - Hyphen/Minus `-`
pub const HYPHEN: u8 = 0x2D;

/// Character code `0x2E` - Period/Dot `.`
pub const PERIOD: u8 = 0x2E;

/// Character code `0x2F` - Forward slash `/`
pub const SLASH: u8 = 0x2F;

/// Character code `0x30` - Digit `0`
pub const DIGIT_0: u8 = 0x30;

/// Character code `0x31` - Digit `1`
pub const DIGIT_1: u8 = 0x31;

/// Character code `0x32` - Digit `2`
pub const DIGIT_2: u8 = 0x32;

/// Character code `0x33` - Digit `3`
pub const DIGIT_3: u8 = 0x33;

/// Character code `0x34` - Digit `4`
pub const DIGIT_4: u8 = 0x34;

/// Character code `0x35` - Digit `5`
pub const DIGIT_5: u8 = 0x35;

/// Character code `0x36` - Digit `6`
pub const DIGIT_6: u8 = 0x36;

/// Character code `0x37` - Digit `7`
pub const DIGIT_7: u8 = 0x37;

/// Character code `0x38` - Digit `8`
pub const DIGIT_8: u8 = 0x38;

/// Character code `0x39` - Digit `9`
pub const DIGIT_9: u8 = 0x39;

/// Character code `0x3A` - Colon `:`
pub const COLON: u8 = 0x3A;

/// Character code `0x3B` - Semicolon `;`
pub const SEMICOLON: u8 = 0x3B;

/// Character code `0x3C` - Less than `<`
pub const LESS_THAN: u8 = 0x3C;

/// Character code `0x3D` - Equals `=`
pub const EQUALS: u8 = 0x3D;

/// Character code `0x3E` - Greater than `>`
pub const GREATER_THAN: u8 = 0x3E;

/// Character code `0x3F` - Question mark `?`
pub const QUESTION_MARK: u8 = 0x3F;

/// Character code `0x40` - At sign `@`
pub const AT_SIGN: u8 = 0x40;

// Uppercase letters A-Z (0x41-0x5A)
/// Character code `0x41` - Uppercase `A`
pub const UPPER_A: u8 = 0x41;
/// Character code `0x42` - Uppercase `B`
pub const UPPER_B: u8 = 0x42;
/// Character code `0x43` - Uppercase `C`
pub const UPPER_C: u8 = 0x43;
/// Character code `0x44` - Uppercase `D`
pub const UPPER_D: u8 = 0x44;
/// Character code `0x45` - Uppercase `E`
pub const UPPER_E: u8 = 0x45;
/// Character code `0x46` - Uppercase `F`
pub const UPPER_F: u8 = 0x46;
/// Character code `0x47` - Uppercase `G`
pub const UPPER_G: u8 = 0x47;
/// Character code `0x48` - Uppercase `H`
pub const UPPER_H: u8 = 0x48;
/// Character code `0x49` - Uppercase `I`
pub const UPPER_I: u8 = 0x49;
/// Character code `0x4A` - Uppercase `J`
pub const UPPER_J: u8 = 0x4A;
/// Character code `0x4B` - Uppercase `K`
pub const UPPER_K: u8 = 0x4B;
/// Character code `0x4C` - Uppercase `L`
pub const UPPER_L: u8 = 0x4C;
/// Character code `0x4D` - Uppercase `M`
pub const UPPER_M: u8 = 0x4D;
/// Character code `0x4E` - Uppercase `N`
pub const UPPER_N: u8 = 0x4E;
/// Character code `0x4F` - Uppercase `O`
pub const UPPER_O: u8 = 0x4F;
/// Character code `0x50` - Uppercase `P`
pub const UPPER_P: u8 = 0x50;
/// Character code `0x51` - Uppercase `Q`
pub const UPPER_Q: u8 = 0x51;
/// Character code `0x52` - Uppercase `R`
pub const UPPER_R: u8 = 0x52;
/// Character code `0x53` - Uppercase `S`
pub const UPPER_S: u8 = 0x53;
/// Character code `0x54` - Uppercase `T`
pub const UPPER_T: u8 = 0x54;
/// Character code `0x55` - Uppercase `U`
pub const UPPER_U: u8 = 0x55;
/// Character code `0x56` - Uppercase `V`
pub const UPPER_V: u8 = 0x56;
/// Character code `0x57` - Uppercase `W`
pub const UPPER_W: u8 = 0x57;
/// Character code `0x58` - Uppercase `X`
pub const UPPER_X: u8 = 0x58;
/// Character code `0x59` - Uppercase `Y`
pub const UPPER_Y: u8 = 0x59;
/// Character code `0x5A` - Uppercase `Z`
pub const UPPER_Z: u8 = 0x5A;

/// Character code `0x5B` - Left square bracket `[`
pub const LEFT_BRACKET: u8 = 0x5B;

/// Character code `0x5C` - Backslash `\`
pub const BACKSLASH: u8 = 0x5C;

/// Character code `0x5D` - Right square bracket `]`
pub const RIGHT_BRACKET: u8 = 0x5D;

/// Character code `0x5E` - Up arrow `↑` (not circumflex in CPC charset)
pub const UP_ARROW: u8 = 0x5E;

/// Character code `0x5F` - Underscore `_`
pub const UNDERSCORE: u8 = 0x5F;

/// Character code `0x60` - Backtick/Grave accent `` ` ``
pub const BACKTICK: u8 = 0x60;

// Lowercase letters a-z (0x61-0x7A)
/// Character code `0x61` - Lowercase `a`
pub const LOWER_A: u8 = 0x61;
/// Character code `0x62` - Lowercase `b`
pub const LOWER_B: u8 = 0x62;
/// Character code `0x63` - Lowercase `c`
pub const LOWER_C: u8 = 0x63;
/// Character code `0x64` - Lowercase `d`
pub const LOWER_D: u8 = 0x64;
/// Character code `0x65` - Lowercase `e`
pub const LOWER_E: u8 = 0x65;
/// Character code `0x66` - Lowercase `f`
pub const LOWER_F: u8 = 0x66;
/// Character code `0x67` - Lowercase `g`
pub const LOWER_G: u8 = 0x67;
/// Character code `0x68` - Lowercase `h`
pub const LOWER_H: u8 = 0x68;
/// Character code `0x69` - Lowercase `i`
pub const LOWER_I: u8 = 0x69;
/// Character code `0x6A` - Lowercase `j`
pub const LOWER_J: u8 = 0x6A;
/// Character code `0x6B` - Lowercase `k`
pub const LOWER_K: u8 = 0x6B;
/// Character code `0x6C` - Lowercase `l`
pub const LOWER_L: u8 = 0x6C;
/// Character code `0x6D` - Lowercase `m`
pub const LOWER_M: u8 = 0x6D;
/// Character code `0x6E` - Lowercase `n`
pub const LOWER_N: u8 = 0x6E;
/// Character code `0x6F` - Lowercase `o`
pub const LOWER_O: u8 = 0x6F;
/// Character code `0x70` - Lowercase `p`
pub const LOWER_P: u8 = 0x70;
/// Character code `0x71` - Lowercase `q`
pub const LOWER_Q: u8 = 0x71;
/// Character code `0x72` - Lowercase `r`
pub const LOWER_R: u8 = 0x72;
/// Character code `0x73` - Lowercase `s`
pub const LOWER_S: u8 = 0x73;
/// Character code `0x74` - Lowercase `t`
pub const LOWER_T: u8 = 0x74;
/// Character code `0x75` - Lowercase `u`
pub const LOWER_U: u8 = 0x75;
/// Character code `0x76` - Lowercase `v`
pub const LOWER_V: u8 = 0x76;
/// Character code `0x77` - Lowercase `w`
pub const LOWER_W: u8 = 0x77;
/// Character code `0x78` - Lowercase `x`
pub const LOWER_X: u8 = 0x78;
/// Character code `0x79` - Lowercase `y`
pub const LOWER_Y: u8 = 0x79;
/// Character code `0x7A` - Lowercase `z`
pub const LOWER_Z: u8 = 0x7A;

/// Character code `0x7B` - Left brace `{`
pub const LEFT_BRACE: u8 = 0x7B;

/// Character code `0x7C` - Vertical bar/Pipe `|`
pub const PIPE: u8 = 0x7C;

/// Character code `0x7D` - Right brace `}`
pub const RIGHT_BRACE: u8 = 0x7D;

/// Character code `0x7E` - Tilde `~`
pub const TILDE: u8 = 0x7E;





#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_control_chars() {
        assert_eq!(STX, 0x02);
        assert_eq!(ETX, 0x03);
        assert_eq!(EOT, 0x04);
        assert_eq!(FF, 0x0C);
    }

    #[test]
    fn test_printable_chars() {
        assert_eq!(SPACE, 0x20);
        assert_eq!(UPPER_A, 0x41);
        assert_eq!(LOWER_A, 0x61);
        assert_eq!(DIGIT_0, 0x30);
    }

    #[test]
    fn test_special_chars() {
        assert_eq!(UP_ARROW, 0x5E);
        assert_eq!(PIPE, 0x7C);
    }
}
