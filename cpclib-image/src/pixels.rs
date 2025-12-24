use crate::ga::Pen;
use crate::image::Mode;

pub fn byte_to_pens(byte: u8, mode: Mode) -> Box<dyn Iterator<Item = Pen>> {
    match mode {
        Mode::Zero => Box::new(mode0::byte_to_pens(byte).into_iter()),
        Mode::One => Box::new(mode1::byte_to_pens(byte).into_iter()),
        Mode::Two => Box::new(mode2::byte_to_pens(byte).into_iter()),
        _ => unimplemented!()
    }
}

pub fn bytes_to_pens<'bytes>(
    bytes: &'bytes [u8],
    mode: Mode
) -> Box<dyn Iterator<Item = Pen> + 'bytes> {
    Box::new(bytes.iter().flat_map(move |&byte| byte_to_pens(byte, mode)))
}

pub fn pens_to_vec(pens: &[Pen], mode: Mode) -> Vec<u8> {
    match mode {
        Mode::Zero => mode0::pens_to_vec_with_crop(pens),
        Mode::One => mode1::pens_to_vec_with_crop(pens),
        Mode::Two => mode2::pens_to_vec_with_crop(pens),
        _ => unimplemented!()
    }
}

/// Mode 2 specific pixels managment functions
pub mod mode2 {
    use crate::ga::Pen;

    /// Pixel ordering in a byte
    /// [First(), Second(), Third(), Fourth()]
    #[repr(u8)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    #[allow(missing_docs)]
    pub enum PixelPosition {
        First = 0,
        Second = 1,
        Third = 2,
        Fourth = 3,
        Fifth = 5,
        Sixth = 6,
        Seventh = 7,
        Heighth = 8
    }

    impl From<u8> for PixelPosition {
        fn from(b: u8) -> Self {
            match b {
                0 => PixelPosition::First,
                1 => PixelPosition::Second,
                2 => PixelPosition::Third,
                3 => PixelPosition::Fourth,
                4 => PixelPosition::Fifth,
                5 => PixelPosition::Sixth,
                6 => PixelPosition::Seventh,
                7 => PixelPosition::Heighth,
                _ => unreachable!()
            }
        }
    }

    pub fn pen_to_pixel_byte(pen: Pen, pixel: PixelPosition) -> u8 {
        let pen = if pen.number() > 3 {
            eprintln!("[MODE2] with pen {:?}", &pen);
            Pen::from(0)
        }
        else {
            pen
        };

        if pen == 0.into() {
            0
        }
        else {
            match pixel {
                PixelPosition::First => 1 << 7,
                PixelPosition::Second => 1 << 6,
                PixelPosition::Third => 1 << 5,
                PixelPosition::Fourth => 1 << 4,
                PixelPosition::Fifth => 1 << 3,
                PixelPosition::Sixth => 1 << 2,
                PixelPosition::Seventh => 1 << 1,
                PixelPosition::Heighth => 1 << 0
            }
        }
    }

    /// Returns the 8 pens for the given byte in mode 0
    pub fn byte_to_pens(byte: u8) -> [Pen; 8] {
        let get_bit = |pos: u8| {
            if byte & (1 << pos) != 0 {
                Pen::from(1)
            }
            else {
                Pen::from(0)
            }
        };

        [
            get_bit(7),
            get_bit(6),
            get_bit(5),
            get_bit(4),
            get_bit(3),
            get_bit(2),
            get_bit(1),
            get_bit(0)
        ]
    }

    /// Convert a vector of pens into a vector of bytes
    pub fn pens_to_vec_with_crop(pens: &[Pen]) -> Vec<u8> {
        let mut res = Vec::new();
        for idx in 0..(pens.len() / 8) {
            res.push(pens_to_byte(
                pens[idx * 8],
                pens[idx * 8 + 1],
                pens[idx * 8 + 2],
                pens[idx * 8 + 3],
                pens[idx * 8 + 4],
                pens[idx * 8 + 5],
                pens[idx * 8 + 6],
                pens[idx * 8 + 7]
            ));
        }

        res
    }

    /// Convert a vector of pens into a vector of bytes
    pub fn pens_to_vec_with_replacement(pens: &[Pen], replacement: Pen) -> Vec<u8> {
        let get_pen = |at| pens.get(at).cloned().unwrap_or(replacement);

        let mut res = Vec::new();
        let mut idx = 0;
        while idx < pens.len() {
            res.push(pens_to_byte(
                get_pen(idx * 8),
                get_pen(idx * 8 + 1),
                get_pen(idx * 8 + 2),
                get_pen(idx * 8 + 3),
                get_pen(idx * 8 + 4),
                get_pen(idx * 8 + 5),
                get_pen(idx * 8 + 6),
                get_pen(idx * 8 + 7)
            ));

            idx += 8;
        }

        res
    }

    pub fn pens_to_byte(
        pen0: Pen,
        pen1: Pen,
        pen2: Pen,
        pen3: Pen,
        pen4: Pen,
        pen5: Pen,
        pen6: Pen,
        pen7: Pen
    ) -> u8 {
        pen_to_pixel_byte(pen0, PixelPosition::First)
            + pen_to_pixel_byte(pen1, PixelPosition::Second)
            + pen_to_pixel_byte(pen2, PixelPosition::Third)
            + pen_to_pixel_byte(pen3, PixelPosition::Fourth)
            + pen_to_pixel_byte(pen4, PixelPosition::Fifth)
            + pen_to_pixel_byte(pen5, PixelPosition::Sixth)
            + pen_to_pixel_byte(pen6, PixelPosition::Seventh)
            + pen_to_pixel_byte(pen7, PixelPosition::Heighth)
    }
}

/// Manage all the stuff related to mode 1 pixels
#[allow(clippy::identity_op)]
pub mod mode1 {
    use crate::ga::Pen;

    /// Pixel ordering in a byte
    /// [First(), Second(), Third(), Fourth()]
    #[repr(u8)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    #[allow(missing_docs)]
    pub enum PixelPosition {
        First = 0,
        Second = 1,
        Third = 2,
        Fourth = 3
    }

    impl From<u8> for PixelPosition {
        fn from(b: u8) -> Self {
            match b {
                0 => PixelPosition::First,
                1 => PixelPosition::Second,
                2 => PixelPosition::Third,
                3 => PixelPosition::Fourth,
                _ => unreachable!()
            }
        }
    }

    /// Signification of the bits in the byte
    #[repr(u8)]
    #[derive(Copy, Clone, Debug)]
    #[allow(missing_docs)]
    pub enum BitMapping {
        FourthBit1 = 0,
        ThirdBit1 = 1,
        SecondBit1 = 2,
        FirstBit1 = 3,
        FourthBit0 = 4,
        ThirdBit0 = 5,
        SecondBit0 = 6,
        FirstBit0 = 7
    }

    /// Return the 4 pens encoded by this byte from left to right
    pub fn byte_to_pens(b: u8) -> [Pen; 4] {
        let pen1 = (BitMapping::FirstBit1, BitMapping::FirstBit0);
        let pen2 = (BitMapping::SecondBit1, BitMapping::SecondBit0);
        let pen3 = (BitMapping::ThirdBit1, BitMapping::ThirdBit0);
        let pen4 = (BitMapping::FourthBit1, BitMapping::FourthBit0);

        let compute = |bits: (BitMapping, BitMapping)| -> Pen {
            let mut value = 0;
            if b & (1 << bits.0 as u8) != 0 {
                value += 2;
            }

            if b & (1 << bits.1 as u8) != 0 {
                value += 1;
            }

            value.into()
        };

        [compute(pen1), compute(pen2), compute(pen3), compute(pen4)]
    }

    pub fn pen_to_bits_position<P: Into<PixelPosition>>(pixel: P) -> [u8; 2] {
        let pixel = pixel.into();

        let mut pos = match pixel {
            // pixel pos [0,1,2,3]            bit1 idx                        bit0 idx
            PixelPosition::First => [BitMapping::FirstBit1 as u8, BitMapping::FirstBit0 as u8],
            PixelPosition::Second => [BitMapping::SecondBit1 as u8, BitMapping::SecondBit0 as u8],
            PixelPosition::Third => [BitMapping::ThirdBit1 as u8, BitMapping::ThirdBit0 as u8],
            PixelPosition::Fourth => [BitMapping::FourthBit1 as u8, BitMapping::FourthBit0 as u8]
        };
        pos.reverse(); // reverse because reading order is opposite to storage order
        pos
    }

    /// Convert the pen value to its byte representation at the proper place
    pub fn pen_to_pixel_byte<P: Into<PixelPosition>>(pen: Pen, pixel: P) -> u8 {
        let pen = if pen.number() > 3 {
            eprintln!("[MODE1] with pen {:?} replaced by pen 0", &pen);
            Pen::from(0)
        }
        else {
            pen
        };

        // Bits of interest (attention order is good when reading it, not using it...)
        let bits_position: [u8; 2] = pen_to_bits_position(pixel);

        // Get the position in the screen byte where the pen bits will be stored
        let byte_bit0: u8 = bits_position[0];
        let byte_bit1: u8 = bits_position[1];

        let pen_bit0: u8 = (pen.number() & (1 << 0)) >> 0;
        let pen_bit1: u8 = (pen.number() & (1 << 1)) >> 1;

        pen_bit1 * (1 << byte_bit1) + pen_bit0 * (1 << byte_bit0)
    }

    /// Convert the 4 pens in a row (from left to right)
    pub fn pens_to_byte(pen0: Pen, pen1: Pen, pen2: Pen, pen3: Pen) -> u8 {
        assert!(pen0.number() < 4);
        assert!(pen1.number() < 4);
        assert!(pen2.number() < 4);
        assert!(pen3.number() < 4);

        pen_to_pixel_byte(pen0, PixelPosition::First)
            + pen_to_pixel_byte(pen1, PixelPosition::Second)
            + pen_to_pixel_byte(pen2, PixelPosition::Third)
            + pen_to_pixel_byte(pen3, PixelPosition::Fourth)
    }

    /// Convert a vector of pens into a vector of bytes.
    /// Crop extra pens that do not enter in a byte
    pub fn pens_to_vec_with_crop(pens: &[Pen]) -> Vec<u8> {
        let mut res = Vec::new();
        for idx in 0..(pens.len() / 4) {
            res.push(pens_to_byte(
                pens[idx * 4 + 0],
                pens[idx * 4 + 1],
                pens[idx * 4 + 2],
                pens[idx * 4 + 3]
            ));
        }

        res
    }

    pub fn pens_to_vec_with_replacement(pens: &[Pen], replacement: Pen) -> Vec<u8> {
        let get_pen = |at: usize| pens.get(at).cloned().unwrap_or(replacement);

        let mut res = Vec::new();
        let mut idx = 0;
        while idx < pens.len() {
            res.push(pens_to_byte(
                get_pen(idx * 4 + 0),
                get_pen(idx * 4 + 1),
                get_pen(idx * 4 + 2),
                get_pen(idx * 4 + 3)
            ));

            idx += 4;
        }

        res
    }

    // Initial python code to backport
    // def get_mode1_pixel0_byte_encoded(pen):
    // """Compute the byte fraction for the required pixel.
    // Order of pixels : 0 1 2 3
    // """
    // pen = int(pen)
    // assert pen < 4
    //
    // byte = 0
    //
    // if pen & 1:
    // byte = byte + (2**7)
    // if pen & 2:
    // byte = byte + (2**3)
    //
    // return byte
    //
    // def get_mode1_pixel1_byte_encoded(pen):
    // """Compute the byte fraction for the required pixel.
    // Order of pixels : 0 1 2 3
    // """
    // pen = int(pen)
    // assert pen < 4
    //
    // byte = 0
    //
    // if pen & 1:
    // byte = byte + (2**6)
    // if pen & 2:
    // byte = byte + (2**2)
    //
    // return byte
    //
    // def get_mode1_pixel2_byte_encoded(pen):
    // """Compute the byte fraction for the required pixel.
    // Order of pixels : 0 1 2 3
    // """
    // pen = int(pen)
    // assert pen < 4
    //
    // byte = 0
    //
    // if pen & 1:
    // byte = byte + (2**5)
    // if pen & 2:
    // byte = byte + (2**1)
    //
    // return byte
    //
    // def get_mode1_pixel3_byte_encoded(pen):
    // """Compute the byte fraction for the required pixel.
    // Order of pixels : 0 1 2 3
    // """
    // pen = int(pen)
    // assert pen < 4
    //
    // byte = 0
    //
    // if pen & 1:
    // byte = byte + (2**4)
    // if pen & 2:
    // byte = byte + (2**0)
    //
    // return byte
    //
}

/// Mode 0 pixels specific operations
#[allow(clippy::identity_op)]
pub mod mode0 {
    // use contracts::{ensures, requires};

    use crate::ga::Pen;

    /// Pixel ordering in a byte
    /// [First(), Second()]
    #[repr(u8)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    #[allow(missing_docs)]
    pub enum PixelPosition {
        First = 0,
        Second = 1
    }

    impl From<u8> for PixelPosition {
        fn from(b: u8) -> Self {
            match b {
                0 => PixelPosition::First,
                1 => PixelPosition::Second,
                _ => unreachable!()
            }
        }
    }

    /// Signification of the bites in the byte
    #[repr(u8)]
    #[derive(Copy, Clone, Debug)]
    #[allow(missing_docs)]
    pub enum BitMapping {
        SecondBit3 = 0,
        FirstBit3 = 1,
        SecondBit1 = 2,
        FirstBit1 = 3,
        SecondBit2 = 4,
        FirstBit2 = 5,
        SecondBit0 = 6,
        FirstBit0 = 7
    }

    /// For a given byte, returns the left and right represented pixels
    /// TODO rewrite using BitMapping and factorizing code
    //#[ensures(ret[0].number()<16)]
    //#[ensures(ret[1].number()<16)]
    pub fn byte_to_pens(b: u8) -> [Pen; 2] {
        let mut pen0 = 0;
        for pos in [7, 3, 5, 1].into_iter().rev() {
            pen0 *= 2;
            if (b & (1 << pos as u8)) != 0 {
                pen0 += 1;
            }
        }

        let mut pen1 = 0;
        for pos in [6, 2, 4, 0].into_iter().rev() {
            pen1 *= 2;
            if (b & (1 << pos as u8)) != 0 {
                pen1 += 1;
            }
        }

        [pen0.into(), pen1.into()]
    }

    /// Convert a couple of pen and pixel position to the corresponding byte value
    //#[requires(pen.number()<16)]
    pub fn pen_to_pixel_byte(pen: Pen, pixel: PixelPosition) -> u8 {
        let bits_position: [u8; 4] = {
            let mut pos = match pixel {
                // pixel pos [0, 1]      bit3  bit2 bit1 bit0
                PixelPosition::First => {
                    [
                        BitMapping::FirstBit3 as u8,
                        BitMapping::FirstBit2 as u8,
                        BitMapping::FirstBit1 as u8,
                        BitMapping::FirstBit0 as u8
                    ]
                },

                PixelPosition::Second => {
                    [
                        BitMapping::SecondBit3 as u8,
                        BitMapping::SecondBit2 as u8,
                        BitMapping::SecondBit1 as u8,
                        BitMapping::SecondBit0 as u8
                    ]
                },
            };
            pos.reverse();
            pos
        };

        let byte_bit0: u8 = bits_position[0];
        let byte_bit1: u8 = bits_position[1];
        let byte_bit2: u8 = bits_position[2];
        let byte_bit3: u8 = bits_position[3];

        let pen_bit0: u8 = (pen.number() & (1 << 0)) >> 0;
        let pen_bit1: u8 = (pen.number() & (1 << 1)) >> 1;
        let pen_bit2: u8 = (pen.number() & (1 << 2)) >> 2;
        let pen_bit3: u8 = (pen.number() & (1 << 3)) >> 3;

        pen_bit3 * (1 << byte_bit3)
            + pen_bit2 * (1 << byte_bit2)
            + pen_bit1 * (1 << byte_bit1)
            + pen_bit0 * (1 << byte_bit0)
    }

    /// Convert the 2 pens in the corresponding byte
    pub fn pens_to_byte(pen0: Pen, pen1: Pen) -> u8 {
        pen_to_pixel_byte(pen0, PixelPosition::First)
            + pen_to_pixel_byte(pen1, PixelPosition::Second)
    }

    /// Convert a vector of pens into a vector of bytes.
    /// In case of an odd number of pens, the last one is lost
    pub fn pens_to_vec_with_crop(pens: &[Pen]) -> Vec<u8> {
        let mut res = Vec::with_capacity(pens.len());
        for idx in 0..(pens.len() / 2) {
            res.push(pens_to_byte(pens[idx * 2 + 0], pens[idx * 2 + 1]));
        }

        res
    }

    /// Convert a vector of pens into a vector of bytes.
    /// In case of an odd number of pens, the missing ones are forced
    pub fn pens_to_vec_with_replacement(pens: &[Pen], replacement: Pen) -> Vec<u8> {
        let mut res = Vec::with_capacity(pens.len());
        for idx in 0..(pens.len() / 2) {
            res.push(pens_to_byte(pens[idx * 2 + 0], pens[idx * 2 + 1]));
        }

        // last pen is 0 if needed
        if pens.len() % 2 == 1 {
            res.push(pens_to_byte(pens[pens.len() - 1], replacement));
        }

        res
    }

    /// Convert a vector of bytes as a vector of pens
    pub fn bytes_to_pens(bytes: &[u8]) -> Vec<Pen> {
        super::bytes_to_pens(bytes, crate::image::Mode::Zero).collect()
    }

    /// Returns a pen that corresponds to first argument in mode 0 and second in mode3
    pub fn mix_mode0_mode3(p0: Pen, p3: Pen) -> Pen {
        (match (p0.number(), p3.number()) {
            (0, 0) => 0,

            (0, 1) => 5,
            (0, 2) => 6,
            (0, 3) => 7,

            (1, 0) => 8,
            (1, 1) => 1,
            (1, 2) => 10,
            (1, 3) => 11,

            (2, 0) => 12,
            (2, 1) => 13,
            (2, 2) => 2,
            (2, 3) => 15,

            (3, 0) => 4,
            (3, 1) => 9,
            (3, 2) => 14,
            (3, 3) => 3,

            _ => panic!()
        })
        .into()
    }

    /// Generate the needed table to write a masked sprite on screen when mask_pen corresponds to the pen of the background.
    ///
    /// Code for the display
    /// ld e, sprite byte to display
    /// ld d, mask_table / 256
    /// ld a, (de) ; get the mask
    /// and (hl) ; set to 0 all pixels that will be replaced
    /// add e ; add the sprite value
    /// ld (hl), a
    ///
    /// ld a, background
    ///
    /// untested code ..
    pub fn generate_sprite_transparency_for_pen0() -> [u8; 256] {
        // Build the bit mask for the given pen
        let pen_to_mask = |pen: Pen| -> Pen {
            if pen.number() == 0 {
                // scren pen must be reseted when sprite as pixels to be drawn
                0xF.into()
            }
            else {
                0x0.into()
            }
        };

        // Masking table to construct
        let mut table = [0; 256];

        // Generate the table
        for (idx, byte) in (0..=255).enumerate() {
            let [pen0, pen1] = byte_to_pens(byte);
            table[idx] = pens_to_byte(pen_to_mask(pen0), pen_to_mask(pen1))
        }

        table
    }
}

#[cfg(test)]
#[allow(clippy::pedantic)]
mod tests {
    use super::*;
    use crate::ga::Pen;

    #[allow(clippy::similar_names)]
    fn test_couple(a: u8, b: u8) {
        let pa: Pen = a.into();
        let pb: Pen = b.into();

        assert_eq!(a, pa.number());
        assert_eq!(b, pb.number());

        let b = mode0::pens_to_byte(pa, pb);
        let [pa2, pb2] = mode0::byte_to_pens(b);

        assert_eq!(pa2.number(), pa2.number());
        assert_eq!(pb2.number(), pb2.number());
    }

    #[test]
    fn mode0() {
        for a in 0..16 {
            for b in 0..16 {
                test_couple(a, b);
            }
        }
    }

    #[test]
    fn bytes_to_pen() {
        // 1000000
        let res = crate::pixels::mode0::byte_to_pens(64);
        assert!(res[0].number() != res[1].number());

        let res = crate::pixels::mode1::byte_to_pens(0b10001000);
        assert_eq!(res[0], Pen::from(3));
        assert_eq!(res[1], Pen::from(0));
        assert_eq!(res[2], Pen::from(0));
        assert_eq!(res[3], Pen::from(0));

        let res = crate::pixels::mode1::byte_to_pens(0b01000100);
        assert_eq!(res[0], Pen::from(0));
        assert_eq!(res[1], Pen::from(3));
        assert_eq!(res[2], Pen::from(0));
        assert_eq!(res[3], Pen::from(0));

        let res = crate::pixels::mode1::byte_to_pens(0b00100010);
        assert_eq!(res[0], Pen::from(0));
        assert_eq!(res[1], Pen::from(0));
        assert_eq!(res[2], Pen::from(3));
        assert_eq!(res[3], Pen::from(0));

        let res = crate::pixels::mode1::byte_to_pens(0b00010001);
        assert_eq!(res[0], Pen::from(0));
        assert_eq!(res[1], Pen::from(0));
        assert_eq!(res[2], Pen::from(0));
        assert_eq!(res[3], Pen::from(3));
    }

    fn test_mode3(a: Pen, b: Pen, c: Pen) {
        let d = mode0::mix_mode0_mode3(a, b);
        assert_eq!(d.number(), c.number());
    }

    #[test]
    fn mode3() {
        test_mode3(0.into(), 0.into(), 0.into());

        test_mode3(0.into(), 1.into(), 5.into());
        test_mode3(0.into(), 2.into(), 6.into());
        test_mode3(0.into(), 3.into(), 7.into());

        test_mode3(3.into(), 0.into(), 4.into());
        test_mode3(3.into(), 1.into(), 9.into());
        test_mode3(3.into(), 2.into(), 14.into());
        test_mode3(3.into(), 3.into(), 3.into());
    }

    #[test]
    fn mode2() {
        let res = mode2::byte_to_pens(0b11000100);
        assert_eq!(res[0], Pen::from(1));
        assert_eq!(res[1], Pen::from(1));
        assert_eq!(res[2], Pen::from(0));
        assert_eq!(res[3], Pen::from(0));
        assert_eq!(res[4], Pen::from(0));
        assert_eq!(res[5], Pen::from(1));
        assert_eq!(res[6], Pen::from(0));
        assert_eq!(res[7], Pen::from(0));
    }
}
