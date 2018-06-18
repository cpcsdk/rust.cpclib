/// Manage all the stuff related to mode 1 pixels
pub mod mode1 {
    use ga::Pen;

    /// Pixel ordering in a byte
    /// [Pixel0(), Pixel1(), Pixel2(), Pixel3()]
    #[repr(u8)]
    pub enum PixelPosition {
        Pixel0 = 0,
        Pixel1 = 1,
        Pixel2 = 2,
        Pixel3 = 3
    }

    /// Signification of the bits in the byte
    #[repr(u8)]
    pub enum BitMapping {
        Pixel3Bit1 = 0,
        Pixel2Bit1 = 1,
        Pixel1Bit1 = 2,
        Pixel0Bit1 = 3,
        Pixel3Bit0 = 4,
        Pixel2Bit0 = 5,
        Pixel1Bit0 = 6,
        Pixel0Bit0 = 7,
    }


    /// Convert the pen value to its byte representation at the proper place
    pub fn pen_to_pixel_byte(pen: Pen, pixel: PixelPosition) -> u8 {
        assert!(pen.number() < 4);

        // Bits of interest (attention order is good when reading it, not using it...)
        let bits_position:[u8;2] = {
            let mut pos = match pixel {
                // pixel pos [0,1,2,3]            bit1 idx                        bit0 idx
                PixelPosition::Pixel0 => [BitMapping::Pixel0Bit1 as u8, BitMapping::Pixel0Bit0 as u8],
                PixelPosition::Pixel1 => [BitMapping::Pixel1Bit1 as u8, BitMapping::Pixel1Bit0 as u8],
                PixelPosition::Pixel2 => [BitMapping::Pixel2Bit1 as u8, BitMapping::Pixel2Bit0 as u8],
                PixelPosition::Pixel3 => [BitMapping::Pixel3Bit1 as u8, BitMapping::Pixel3Bit0 as u8],
            };
            pos.reverse(); // reverse because reading order is opposite to storage order
            pos
        };

        // Get the position in the screen byte where the pen bits will be stored
        let byte_bit0: u8 = bits_position[0];
        let byte_bit1: u8 = bits_position[1];

        let pen_bit0 : u8 = pen.number() & (1 << 0);
        let pen_bit1 : u8 = (pen.number() & (1 << 1)) >> 1;

        let byte = pen_bit1 * (1<<byte_bit1) + pen_bit0 * (1<<byte_bit0);
        byte

    }

    /// Convert the 4 pens in a row (from left to right)
    pub fn pens_to_byte(pen0: Pen, pen1: Pen, pen2: Pen, pen3: Pen) -> u8 {
          pen_to_pixel_byte(pen0, PixelPosition::Pixel0)
        + pen_to_pixel_byte(pen1, PixelPosition::Pixel1)
        + pen_to_pixel_byte(pen2, PixelPosition::Pixel2)
        + pen_to_pixel_byte(pen3, PixelPosition::Pixel3)
    }



    /// Convert a vector of pens into a vector of bytes
    pub fn pens_to_vec(pens: &Vec<Pen>) -> Vec<u8> {
        assert!(pens.len() % 4 == 0);


        let mut res = Vec::new();
        for idx in 0..(pens.len()/4) {
            res.push(pens_to_byte(
                    pens[idx*4+0],
                    pens[idx*4+1],
                    pens[idx*4+2],
                    pens[idx*4+3]
                    )
                );
        }

        res
    }



/*
 * Initial python code to backport
def get_mode1_pixel0_byte_encoded(pen):
    """Compute the byte fraction for the required pixel.
    Order of pixels : 0 1 2 3
    """
    pen = int(pen)
    assert pen < 4

    byte = 0

    if pen & 1:
        byte = byte + (2**7)
    if pen & 2:
        byte = byte + (2**3)

    return byte

def get_mode1_pixel1_byte_encoded(pen):
    """Compute the byte fraction for the required pixel.
    Order of pixels : 0 1 2 3
    """
    pen = int(pen)
    assert pen < 4

    byte = 0

    if pen & 1:
        byte = byte + (2**6)
    if pen & 2:
        byte = byte + (2**2)

    return byte

def get_mode1_pixel2_byte_encoded(pen):
    """Compute the byte fraction for the required pixel.
    Order of pixels : 0 1 2 3
    """
    pen = int(pen)
    assert pen < 4

    byte = 0

    if pen & 1:
        byte = byte + (2**5)
    if pen & 2:
        byte = byte + (2**1)

    return byte

def get_mode1_pixel3_byte_encoded(pen):
    """Compute the byte fraction for the required pixel.
    Order of pixels : 0 1 2 3
    """
    pen = int(pen)
    assert pen < 4

    byte = 0

    if pen & 1:
        byte = byte + (2**4)
    if pen & 2:
        byte = byte + (2**0)

    return byte

*/


}



pub mod mode0 {
    use ga::Pen;


    /// Pixel ordering in a byte
    /// [Pixel0(), Pixel1()]
    #[repr(u8)]
    pub enum PixelPosition {
        Pixel0 = 0,
        Pixel1 = 1
    }

    /// Signification of the bites in the byte
    #[repr(u8)]
    pub enum BitMapping {
        Pixel1Bit3 = 0,
        Pixel0Bit3 = 1,
        Pixel1Bit1 = 2,
        Pixel0Bit1 = 3,
        Pixel1Bit2 = 4,
        Pixel0Bit2 = 5,
        Pixel1Bit0 = 6,
        Pixel0Bit0 = 7
    }


    /// For a given byte, returns the left and right represented pixels
    /// TODO rewrite using BitMapping and factorizing code
    pub fn byte_to_pens(b:u8) -> (Pen, Pen) {
        let mut pen0 = 0;
        for pos in [7, 3, 5, 1].into_iter().rev() {
            pen0 *= 2;
            if (b & 1<<*pos as u8) != 0 {
                pen0 += 1;
            }
        }

        let mut pen1 = 0;
        for pos in [6, 2, 4, 0].iter().rev() {
            pen1 *= 2;
            if (b & 1<<*pos as u8) != 0 {
                pen1 += 1;
            }
        }

         (pen0.into(), pen1.into())
    }

    pub fn pen_to_pixel_byte(pen: &Pen, pixel: PixelPosition) -> u8 {
        assert!(pen.number()<16, format!("{} >=16", pen.number()));

        let bits_position:[u8;4] = {
            let mut pos = match pixel {
                // pixel pos [0, 1]      bit3  bit2 bit1 bit0
                PixelPosition::Pixel0 =>
                    [
                    BitMapping::Pixel0Bit3 as u8,
                    BitMapping::Pixel0Bit2 as u8,
                    BitMapping::Pixel0Bit1 as u8,
                    BitMapping::Pixel0Bit0 as u8,
                    ],

                    PixelPosition::Pixel1 =>
                        [
                        BitMapping::Pixel1Bit3 as u8,
                        BitMapping::Pixel1Bit2 as u8,
                        BitMapping::Pixel1Bit1 as u8,
                        BitMapping::Pixel1Bit0 as u8,
                        ]
            };
            pos.reverse();
            pos
        };


        let byte_bit0: u8 = bits_position[0];
        let byte_bit1: u8 = bits_position[1];
        let byte_bit2: u8 = bits_position[2];
        let byte_bit3: u8 = bits_position[3];

        let pen_bit0 : u8 = (pen.number() & (1 << 0)) >> 0;
        let pen_bit1 : u8 = (pen.number() & (1 << 1)) >> 1;
        let pen_bit2 : u8 = (pen.number() & (1 << 2)) >> 2;
        let pen_bit3 : u8 = (pen.number() & (1 << 3)) >> 3;

        let byte =
            pen_bit3 * (1<<byte_bit3) +
            pen_bit2 * (1<<byte_bit2) +
            pen_bit1 * (1<<byte_bit1) +
            pen_bit0 * (1<<byte_bit0);

        byte
    }


    /// Convert the 2 pens in the corresponding byte
    pub fn pens_to_byte(pen0: &Pen, pen1: &Pen) -> u8 {
        pen_to_pixel_byte(pen0, PixelPosition::Pixel0)
            + pen_to_pixel_byte(pen1, PixelPosition::Pixel1)
    }


    /// Convert a vector of pens into a vector of bytes
    pub fn pens_to_vec(pens: &Vec<Pen>) -> Vec<u8> {
        assert!(pens.len() % 2 == 0);

        let mut res = Vec::new();
        for idx in 0..(pens.len()/2) {
            res.push(pens_to_byte(&pens[idx*2+0], &pens[idx*2+1]));
        }

        res
    }


    /// Returns a pen that corresponds to first argument in mode 0 and second in mode3
    pub fn mix_mode0_mode3(p0: &Pen, p3: &Pen) -> Pen {
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
        }
        ).into()
    }
}



#[cfg(test)]
mod tests {
    use ga::Pen;
    use pixels::*;


    fn test_couple(a: u8, b: u8) {
        let pa: Pen = a.into();
        let pb: Pen = b.into();

        assert_eq!(a, pa.number());
        assert_eq!(b, pb.number());

        let b = mode0::pens_to_byte(&pa, &pb);
        let (pa2, pb2) = mode0::byte_to_pens(b);


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
        let res = ::pixels::mode0::byte_to_pens(64);
        assert!(res.0.number() != res.1.number());
    }

    fn test_mode3(a: &Pen, b: &Pen, c: &Pen) {
        let d = mode0::mix_mode0_mode3(a, b);
        assert_eq!(d.number(), c.number());
    }

    #[test]
    fn mode3() {
        test_mode3(&0.into(), &0.into(), &0.into());

        test_mode3(&0.into(), &1.into(), &5.into());
        test_mode3(&0.into(), &2.into(), &6.into());
        test_mode3(&0.into(), &3.into(), &7.into());

        test_mode3(&3.into(), &0.into(), &4.into());
        test_mode3(&3.into(), &1.into(), &9.into());
        test_mode3(&3.into(), &2.into(), &14.into());
        test_mode3(&3.into(), &3.into(), &3.into());
    }
}
