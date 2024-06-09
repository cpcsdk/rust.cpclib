use std::{fs::File, io::{Read, Write}, ops::Deref, path::Path};

use cpclib_common::riff::{RiffChunk, RiffCode, RiffLen};


#[derive(PartialEq, Debug)]
pub struct CartridgeBloc(RiffChunk);


impl Deref for CartridgeBloc {
    type Target = RiffChunk;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}


impl TryFrom<RiffChunk> for CartridgeBloc {
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


impl CartridgeBloc {
    pub fn new(nb: u8) -> CartridgeBloc {
        assert!(nb<32);

        let data = vec![0; 0x4000];
        let code = Self::code_for(nb);
        let chunk = RiffChunk::new(code.as_str(), data);
        chunk.try_into().unwrap()
    }


    pub fn code_for(nb: u8) -> String {
        format!("cb{:02}", nb)
    }



}

#[derive(PartialEq, Debug)]
pub struct Cpr {
    banks: Vec<CartridgeBloc>
}

impl Cpr {

    pub fn empty() -> Self {
        Cpr{banks: Vec::default()}
    }

    pub fn banks(&self) -> &[CartridgeBloc] {
        &self.banks
    }

    pub fn len(&self) -> RiffLen {
        let size: u32 = self.banks.iter()
            .map(|b| 
                b.code().len() as u32 +
                b.len().len() as u32 + 
                b.len().value() )
            .sum();
        size.into()
    }

    pub fn add_bloc(&mut self, bloc: CartridgeBloc) {
        // TODO check if it is already present
        self.banks.push(bloc);
    }

    pub fn write<B: Write>(
        &self,
        buffer: &mut B,
    ) -> Result<(), std::io::Error> {
        
        let riff_code: RiffCode = "RIFF".into();
        let len : RiffLen = (self.len().value() + 4).into();
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
                let cb: CartridgeBloc = chunk.try_into()?;
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



#[cfg(test)]
mod test {
    use crate::CartridgeBloc;

    #[test]
    fn test_cartrdige_code() {
        assert_eq!("cb00", CartridgeBloc::code_for(0).as_str());
        assert_eq!("cb31", CartridgeBloc::code_for(31).as_str());
    }
}