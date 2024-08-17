#![feature(slice_take)]

pub mod clap_extra;
pub mod parse;
pub mod riff;

#[cfg(feature = "cmdline")]
pub use ::clap;
pub use clap_extra::*;
pub use parse::*;
#[cfg(all(not(target_arch = "wasm32"), feature = "rayon"))]
pub use rayon;
#[cfg(feature = "cmdline")]
pub use semver;
#[cfg(feature = "cmdline")]
pub use time;
pub use {
    bitfield, bitflags, bitvec, camino, itertools, num, resolve_path, smallvec, smol_str, strsim,
    winnow
};

#[cfg(test)]
mod tests {
    use winnow::error::ContextError;
    use winnow::stream::AsBStr;
    use winnow::{BStr, Parser};

    use super::*;

    #[test]
    fn test_parse_value() {
        let mut fortytwo = "42".as_bstr();
        assert_eq!(
            dbg!(parse_value::<_, ContextError>.parse_next(&mut fortytwo)).unwrap(),
            42
        );
        assert_eq!(
            parse_value::<_, ContextError>
                .parse(BStr::new(b"0x12"))
                .unwrap(),
            0x12
        );
        assert_eq!(
            parse_value::<_, ContextError>
                .parse(BStr::new(b"0x0000"))
                .unwrap(),
            0x0000
        );
        assert_eq!(
            parse_value::<_, ContextError>
                .parse(BStr::new(b"0x4000"))
                .unwrap(),
            0x4000
        );
        assert_eq!(
            parse_value::<_, ContextError>
                .parse(BStr::new(b"0x8000"))
                .unwrap(),
            0x8000
        );
        assert_eq!(
            parse_value::<_, ContextError>
                .parse(BStr::new(b"0xc000"))
                .unwrap(),
            0xC000
        );
        assert_eq!(
            parse_value::<_, ContextError>
                .parse(BStr::new(b"0x1_2"))
                .unwrap(),
            0x12
        );
        assert_eq!(
            dbg!(parse_value::<_, ContextError>.parse(BStr::new(b"0b0100101"))).unwrap(),
            0b0100101
        );
        assert_eq!(
            dbg!(parse_value::<_, ContextError>.parse(BStr::new(b"0b0_100_101"))).unwrap(),
            0b0100101
        );
        assert_eq!(
            dbg!(parse_value::<_, ContextError>.parse(BStr::new(b"%0100101"))).unwrap(),
            0b0100101
        );
        assert_eq!(
            dbg!(parse_value::<_, ContextError>.parse(BStr::new(b"0100101b"))).unwrap(),
            0b0100101
        );
        assert_eq!(
            dbg!(parse_value::<_, ContextError>.parse(BStr::new(b"160"))).unwrap(),
            160
        );
        assert_eq!(
            dbg!(parse_value::<_, ContextError>.parse(BStr::new(b"1_60"))).unwrap(),
            160
        );
        assert_eq!(
            dbg!(parse_value::<_, ContextError>.parse(BStr::new(b"0b0h"))).unwrap(),
            0x0B0
        );
        assert_eq!(
            dbg!(parse_value::<_, ContextError>.parse(BStr::new(b"0bh"))).unwrap(),
            0xB
        );

        assert_eq!(
            dbg!(parse_value::<_, ContextError>.parse(BStr::new(b"CH"))).unwrap(),
            0xC
        );

        assert!(dbg!(parse_value::<_, ContextError>.parse_next(&mut BStr::new(b"CHECK"))).is_err());
    }
}
