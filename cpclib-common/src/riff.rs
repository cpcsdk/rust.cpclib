use std::fmt::{write, Display};
use std::io::Write;
use std::ops::Deref;

#[derive(Clone, Debug, PartialEq, PartialOrd, Ord, Eq)]
pub struct RiffCode(pub(crate) [u8; 4]);

impl Deref for RiffCode {
    type Target = [u8; 4];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<[u8; 4]> for RiffCode {
    fn from(value: [u8; 4]) -> Self {
        RiffCode::new(value)
    }
}

impl From<&[u8]> for RiffCode {
    fn from(value: &[u8]) -> Self {
        assert_eq!(value.len(), 4);
        RiffCode::new([value[0], value[1], value[2], value[3]])
    }
}

impl From<&str> for RiffCode {
    fn from(value: &str) -> Self {
        let code = value.as_bytes().take(..4).unwrap();
        RiffCode::new([code[0], code[1], code[2], code[3]])
    }
}

impl Display for RiffCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}{}{}",
            self.0[0] as char, self.0[1] as char, self.0[2] as char, self.0[3] as char
        )
    }
}
impl RiffCode {
    pub const fn new(value: [u8; 4]) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(&self.0) }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RiffLen(pub(crate) [u8; 4]);

impl Deref for RiffLen {
    type Target = [u8; 4];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<&[u8]> for RiffLen {
    fn from(value: &[u8]) -> Self {
        assert_eq!(4, value.len());
        Self([value[0], value[1], value[2], value[3]])
    }
}

impl From<u32> for RiffLen {
    fn from(mut size: u32) -> Self {
        let mut array = [0, 0, 0, 0];

        for item in &mut array {
            *item = (size % 256) as u8;
            size /= 256;
        }

        Self(array)
    }
}

impl From<usize> for RiffLen {
    fn from(value: usize) -> Self {
        Self::from(value as u32)
    }
}

impl Into<u32> for &RiffLen {
    fn into(self) -> u32 {
        let mut size = 0;
        for byte in self.0.iter().rev() {
            size = size * 256 + *byte as u32;
        }
        size
    }
}

impl Into<usize> for &RiffLen {
    fn into(self) -> usize {
        let size: u32 = self.into();
        size as _
    }
}

impl RiffLen {
    pub fn increment(&self) -> Self {
        let size: u32 = self.into();
        Self::from(size + 1)
    }

    pub fn add(&self, value: usize) -> Self {
        let size: u32 = self.into();
        Self::from(size + value as u32)
    }

    pub fn decrement(&self) -> Self {
        let size: u32 = self.into();
        Self::from(size - 1)
    }

    pub fn value(&self) -> u32 {
        self.into()
    }
}

pub struct RiffContainer {
    /// RIFF or LIST only
    ckid: RiffCode
}

#[derive(Clone, Debug)]
/// Raw chunk data.
#[derive(PartialEq)]
pub struct RiffChunk {
    /// Identifier of the chunk
    ckid: RiffCode,
    /// Length of the chunk (always data.len())
    cksz: RiffLen,
    /// Content of the chunk (size included)
    data: Vec<u8>
}

#[allow(missing_docs)]
impl RiffChunk {
    pub fn write_all<B: Write>(&self, buffer: &mut B) -> Result<(), std::io::Error> {
        buffer.write_all(self.code().deref())?;
        buffer.write_all(self.len().deref())?;
        buffer.write_all(self.data())?;

        Ok(())
    }

    pub fn from_buffer(file_content: &mut Vec<u8>) -> Self {
        // get the code and length
        let ckid: RiffCode = file_content.drain(0..4).as_slice().into();
        let cksz: RiffLen = file_content.drain(0..4).as_slice().into();

        // read the appropriate number of bytes
        let data = if cksz.value() > 0 {
            file_content.drain(0..cksz.value() as _).as_slice().to_vec()
        }
        else {
            Vec::with_capacity(0)
        };

        assert_eq!(data.len(), cksz.value() as usize);

        Self { ckid, cksz, data }
    }

    pub fn new<C: Into<RiffCode>>(code: C, data: Vec<u8>) -> Self {
        Self {
            ckid: code.into(),
            cksz: data.len().into(),
            data
        }
    }

    pub fn code(&self) -> &RiffCode {
        &(self.ckid)
    }

    pub fn len(&self) -> &RiffLen {
        &self.cksz
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    // todo increase the size
    pub fn add_bytes(&mut self, data: &[u8]) {
        self.data.extend_from_slice(data);
        self.update_cksz();
    }

    pub fn set_byte(&mut self, address: u16, byte: u8) {
        self.data[address as usize] = byte;
    }

    fn update_cksz(&mut self) {
        self.cksz = self.data.len().into();
    }
}
