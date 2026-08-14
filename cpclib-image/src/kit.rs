//! `.kit` palette files - the 16 ASIC colours of an Amstrad Plus palette.
//!
//! <http://impdraw.wikidot.com/les-fichiers-palette-ink-kit>
//!
//! 32 bytes, two per colour, in pen order. See [`crate::asic`] for the layout
//! of one entry.

use std::path::Path;

use crate::asic::AsicColor;
use crate::ink::Ink;
use crate::palette::Palette;
use crate::pen::Pen;

/// The 16 colours of a Plus palette, as stored in a `.kit` file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Kit([AsicColor; 16]);

impl Kit {
    /// How many bytes a `.kit` file holds.
    pub const BYTE_SIZE: usize = 32;

    pub fn from_bytes(bytes: [u8; Self::BYTE_SIZE]) -> Self {
        let mut colors = [AsicColor::default(); 16];
        for (i, color) in colors.iter_mut().enumerate() {
            *color = AsicColor::from_bytes([bytes[i * 2], bytes[i * 2 + 1]]);
        }
        Self(colors)
    }

    pub fn to_bytes(&self) -> [u8; Self::BYTE_SIZE] {
        let mut bytes = [0; Self::BYTE_SIZE];
        for (i, color) in self.0.iter().enumerate() {
            let [high, low] = color.to_bytes();
            bytes[i * 2] = high;
            bytes[i * 2 + 1] = low;
        }
        bytes
    }

    /// Read a `.kit` from disk.
    ///
    /// A file of the wrong length is refused rather than padded or truncated:
    /// a short read here would silently produce black pens, which looks like a
    /// conversion bug rather than a wrong file.
    pub fn from_file<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let content = std::fs::read(path.as_ref())?;
        let bytes: [u8; Self::BYTE_SIZE] = content.as_slice().try_into().map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "{} is {} bytes; a .kit palette is exactly {}",
                    path.as_ref().display(),
                    content.len(),
                    Self::BYTE_SIZE
                )
            )
        })?;
        Ok(Self::from_bytes(bytes))
    }

    pub fn colors(&self) -> &[AsicColor; 16] {
        &self.0
    }
}

impl From<[AsicColor; 16]> for Kit {
    fn from(colors: [AsicColor; 16]) -> Self {
        Self(colors)
    }
}

impl From<Kit> for Palette<AsicColor> {
    fn from(kit: Kit) -> Self {
        let mut palette = Palette::<AsicColor>::empty();
        for (pen, color) in kit.0.iter().enumerate() {
            palette.set(Pen::from(pen as u8), *color);
        }
        palette
    }
}

impl From<Palette<AsicColor>> for Kit {
    fn from(palette: Palette<AsicColor>) -> Self {
        let mut colors = [AsicColor::default(); 16];
        for (pen, color) in colors.iter_mut().enumerate() {
            *color = *palette.get(&Pen::from(pen as u8));
        }
        Self(colors)
    }
}

impl From<Palette<Ink>> for Kit {
    fn from(palette: Palette<Ink>) -> Self {
        Palette::<AsicColor>::from(palette).into()
    }
}
