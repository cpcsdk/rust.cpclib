use crate::ga::{Ink, Palette};
use crate::image::Mode;

/// ! Utility code related to OCP

pub fn compress<D: as_slice::AsSlice<Element = u8>>(data: D) -> Vec<u8> {
    eprintln!("[WARNING] OCP compression has never been tested");

    let data = data.as_slice();
    const MARKER: u8 = 1;

    let mut res = Vec::new();

    res.push(b'M');
    res.push(b'J');
    res.push(b'H');

    let length = data.len();
    let high = (length >> 8) as u8;
    let low = (length % 256) as u8;

    res.push(low);
    res.push(high);

    let mut previous = 0;
    let mut count = 0;

    for current in &data[1..] {
        let current = *current;

        if current == MARKER {
            if count != 0 {
                res.push(MARKER);
                res.push(count);
                res.push(previous);
            }

            res.push(MARKER);
            res.push(1);
            res.push(MARKER);
        }
        else if previous == current {
            if count == 255 {
                res.push(MARKER);
                res.push(0);
                res.push(current);
                count = 0;
            }
            else {
                count += 1;
            }
        }
        else {
            if count == 1 {
                debug_assert!(MARKER != current);
                res.push(current);
            }
            else {
                res.push(MARKER);
                res.push(count);
                res.push(current);
            }
            count = 0;
        }

        previous = current;
    }

    if count == 1 {
        res.push(previous);
    }
    else if count > 1 {
        res.push(MARKER);
        res.push(count);
        res.push(previous);
    }

    res
}

const NB_PAL: usize = 12;

pub struct OcpPalette {
    screen_mode: Mode,
    cycling: bool,
    cycling_delay: u8,
    palettes: [Palette; NB_PAL],

    excluded: [u8; 16],
    projected: [u8; 16]
}

impl OcpPalette {
    /// Get the palette of interest (0..12)
    pub fn palette(&self, nb: usize) -> &Palette {
        assert!(nb < 12);
        &self.palettes[nb]
    }

    pub fn palettes(&self) -> &[Palette; NB_PAL] {
        &self.palettes
    }

    pub fn from_buffer(data: &[u8]) -> Self {
        let mut data = data.iter().cloned();

        let screen_mode: Mode = (data.next().unwrap()).into();
        let cycling = data.next().unwrap() == 0xFF;
        let cycling_delay = data.next().unwrap();

        let mut palettes: [Palette; NB_PAL] = Default::default(); // arrays::from_iter((0..NB_PAL).into_iter().map(|_| Palette::default())).unwrap();
        for pen in 0..=16i32 {
            for idx in 0..NB_PAL {
                let ga_ink = data.next().unwrap();
                let ink = Ink::from_gate_array_color_number(ga_ink);
                palettes[idx].set(pen, ink);
            }
        }

        let excluded = data.next_chunk().unwrap();
        let projected = data.next_chunk().unwrap();

        assert!(data.next().is_none());

        Self {
            screen_mode,
            cycling,
            cycling_delay,
            palettes,
            excluded,
            projected
        }
    }
}
