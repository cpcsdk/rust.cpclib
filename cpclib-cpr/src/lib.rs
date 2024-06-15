use std::{fmt::Display, fs::File, hash::{DefaultHasher, Hash, Hasher}, io::{Read, Write}, ops::Deref, path::Path};

use cpclib_common::{itertools::Itertools, riff::{RiffChunk, RiffCode, RiffLen}, winnow::Parser};

const CODE_BANKS: [&'static str; 32] = [
    "cb00",
    "cb01",
    "cb02",
    "cb03",
    "cb04",
    "cb05",
    "cb06",
    "cb07",
    "cb08",
    "cb09",

    "cb10",
    "cb11",
    "cb12",
    "cb13",
    "cb14",
    "cb15",
    "cb16",
    "cb17",
    "cb18",
    "cb19",

    "cb20",
    "cb21",
    "cb22",
    "cb23",
    "cb24",
    "cb25",
    "cb26",
    "cb27",
    "cb28",
    "cb29",

    "cb30",
    "cb31",
];

#[derive(PartialEq, Debug, Clone)]
pub struct CartridgeBank(RiffChunk);


impl Deref for CartridgeBank {
    type Target = RiffChunk;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}


impl TryFrom<RiffChunk> for CartridgeBank {
    type Error = String;

    fn try_from(value: RiffChunk) -> Result<Self, Self::Error> {
        let code = value.code().to_string();
        if !code.starts_with("cb") {
            return Err(format!("{code} is an invalid cartridge bloc id"))
        }

        let nb : u16 = u16::from_str_radix(&code[2..], 10).unwrap_or(0xdead);
        if nb > 32 {
            return Err(format!("{code} is an invalid cartridge bloc id"))
        }

        if value.len().value() > 0x4000 {
            return Err(format!("{} is an invalid size", value.len().value()));
        }

        Ok(Self(value))
    }
}


impl CartridgeBank {
    pub fn new(nb: u8) -> CartridgeBank {
        assert!(nb<32);

        let data = vec![0; 0x4000];
        let code = Self::code_for(nb);
        let chunk = RiffChunk::new(code, data);
        chunk.try_into().unwrap()
    }

    pub fn number(&self) -> u8 {
        Self::nb_for_code(self.code().as_str())
    }

    pub fn code_for(nb: u8) -> &'static str {
        CODE_BANKS[nb as usize]
    }
    
    pub fn nb_for_code(code: &str) -> u8 {
        let idx = CODE_BANKS.iter().position(|&c| c == code).unwrap();
        idx as u8
    }

	pub fn set_byte(&mut self, address: u16, byte: u8) {
        self.0.set_byte(address, byte)
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct Cpr {
    banks: Vec<CartridgeBank>
}

impl Default for Cpr {
    fn default() -> Self {
        Cpr::empty()
    }
}


impl From<Vec<CartridgeBank>> for Cpr {
    fn from(banks: Vec<CartridgeBank>) -> Self {
        Self{banks}
    }
}

impl Cpr {

    pub fn empty() -> Self {
        Cpr{banks: Vec::default()}
    }

    pub fn banks(&self) -> &[CartridgeBank] {
        &self.banks
    }

    pub fn bank_mut(&mut self, idx: usize) -> Option<&mut CartridgeBank> {
        self.banks.get_mut(idx)
    }

    pub fn bank_at_index(&self, idx: usize) -> Option<&CartridgeBank> {
        self.banks.get(idx)
    }

    pub fn bank_by_num(&self, nb: u8) -> Option<&CartridgeBank> {
        self.bank_index(nb)
            .map(|idx| &self.banks[idx])
    }

    pub fn bank_by_code(&self, code: &RiffCode) -> Option<&CartridgeBank> {
        self.bank_index(CartridgeBank::nb_for_code(code.as_str()))
            .map(|idx| &self.banks[idx])
    }

    

    /// The len of a CPR is the len of each bloc + BAMS size
    pub fn len(&self) -> RiffLen {
        let size: u32 = self.banks.iter()
            .map(|b| 
                b.code().len() as u32 +
                b.len().len() as u32 + 
                b.len().value() )
            .sum::<u32>() + 4;
        size.into()
    }

    pub fn add_bank(&mut self, bloc: CartridgeBank) {
        // TODO check if it is already present
        self.banks.push(bloc);
    }

    pub fn remove_bank(&mut self, nb: u8) -> Option<CartridgeBank> {
        if let Some(idx) = self.bank_index(nb) {
            Some(self.banks.remove(idx))
        } else {
            None
        }
    }

    fn bank_index(&self, nb: u8) -> Option<usize> {
        self.banks()
            .iter()
            .position(|bank| bank.code().as_str() == CODE_BANKS[nb as usize])
    }

    
}


impl Cpr {
    pub fn save<P: AsRef<Path>>(
        &self,
        fname: P,
    ) -> Result<(), std::io::Error> {
        let mut buffer = File::create(fname.as_ref())?;
        self.write_all(&mut buffer)
    }

    pub fn write_all<B: Write>(
        &self,
        buffer: &mut B,
    ) -> Result<(), std::io::Error> {
        
        let riff_code: RiffCode = "RIFF".into();
        let len : RiffLen = (self.len().value()).into();
        let ams_code: RiffCode = "AMS!".into();

        buffer.write_all(riff_code.deref())?;
        buffer.write_all(len.deref())?;
        buffer.write_all(ams_code.deref())?;

        for bank in &self.banks {
            bank.write_all(buffer)?;
        }

        Ok(())

    }


    pub fn load<P: AsRef<Path>>(filename: P) -> Result<Self, String> {
        let filename = filename.as_ref();

        // Read the full content of the file
        let file_content = {
            let mut f = File::open(filename).map_err(|e| e.to_string())?;
            let mut content = Vec::new();
            f.read_to_end(&mut content).map_err(|e| e.to_string())?;
            content
        };

        Self::from_buffer(file_content)
    }


    pub fn from_buffer(mut file_content: Vec<u8>) -> Result<Self, String> {
        let tag: RiffCode = file_content.drain(0..4).as_slice().into();
        if tag != [b'R', b'I', b'F', b'F'].into() {
            return Err(format!("{tag:?} found instead of RIFF"));
        }

        let length: RiffLen = file_content.drain(0..4).as_slice().into();
        if length.value() as usize != file_content.len() {
            return Err(format!("Wrong size coding in CPR file
        {} != {}", length.value(), file_content.len()));
        }

        let tag: RiffCode = file_content.drain(0..4).as_slice().into();

        if tag != [b'A', b'M', b'S', b'!'].into() {
            return Err(format!("{tag} found instead of AMS!"));
        }

        let mut banks = Vec::new();
        while !file_content.is_empty() {
            let chunk = RiffChunk::from_buffer(&mut file_content);
            if chunk.code().to_string().as_bytes() != b"fmt " {
                let cb: CartridgeBank = chunk.try_into()?;
                banks.push(cb);
            }
        }

        let cpr = Cpr { banks };

        assert!(file_content.is_empty());
        if length.value() !=  cpr.len().value() {
            assert!(length.value() > cpr.len().value());
            eprintln!("CPR indicates a length of {} whereas current length is {}.", length.value(), cpr.len().value())
        }

        Ok(cpr)

    }

}


pub struct CartridgeBankInfo<'c> {
    riff_size: u32,
    checksum: u64,
    bank: &'c CartridgeBank,
}

impl<'c> Display for CartridgeBankInfo<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f, 
            "Code: {}\nRiff size: {}\nChecksum: {:X}", 
            self.code().as_str(),
            self.riff_size,
            self.checksum
        )
    }
}

impl<'c> From<&'c CartridgeBank> for CartridgeBankInfo<'c> {
    fn from(bank: &'c CartridgeBank) -> Self {
        let checksum = {
            let mut hasher = DefaultHasher::new();
            bank.deref().data().hash(&mut hasher);
            hasher.finish()
        };

        Self {
            riff_size: bank.len().value(),
            checksum,
            bank
        }
    }
}

impl<'c> Deref for CartridgeBankInfo<'c> {
    type Target = CartridgeBank;

    fn deref(&self) -> &Self::Target {
        &self.bank
    }
}


pub struct CprInfo<'c> {
    banks: Vec<CartridgeBankInfo<'c>>,
    cpr: &'c Cpr
}


impl<'c> Display for CprInfo<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Cartridge with {} banks", self.cpr.banks().len())?;
        for (idx, bank) in self.banks.iter().enumerate() {
            writeln!(f, "# Bank {idx}\n{}", bank)?;
        }
        Ok(())
    }
}

impl<'c> From<&'c Cpr> for CprInfo<'c> {
    fn from(cpr: &'c Cpr) -> Self {
        let banks: Vec<CartridgeBankInfo<'c>> = cpr.banks().iter()
            .map(|b| b.into())
            .collect_vec();
        Self {banks, cpr}
    }
}

impl<'c> Deref for CprInfo<'c> {
    type Target = Cpr;
    fn deref(&self) -> &Self::Target {
        &self.cpr
    }
}
#[cfg(test)]
mod test {
    use crate::CartridgeBank;

    #[test]
    fn test_cartrdige_code() {
        assert_eq!("cb00", CartridgeBank::code_for(0));
        assert_eq!("cb31", CartridgeBank::code_for(31));
    }
}