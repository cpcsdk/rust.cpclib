///! Simple managment of ASM code through strings.

use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::fmt::{Debug, Formatter, Result};

pub fn bytes_to_db_str(bytes: &[u8]) -> String {
    let bytes_str:Vec<String> = bytes.iter().map(|b| format!("0x{:x}", b)).collect();
    format!("\tdb {}\n", bytes_str.join(","))
}

#[derive(Clone, Debug, PartialEq)]
pub enum Bank {
    Bank0,
    Bank1,
    Bank2,
    Bank3,

    Bank4,
    Bank5,
    Bank6,
    Bank7,
}


impl Bank {

    pub fn is_main_memory(&self) -> bool {
        use self::Bank::*;
        match self {
            &Bank0 | &Bank1 | &Bank2 | &Bank3 => true,
            _ => false
        }
    }


    pub fn is_extra_bank(&self) -> bool {
        ! self.is_main_memory()
    }

    pub fn num(&self) -> u8 {
        use self::Bank::*;
        match self {
            &Bank0 => 0,
            &Bank1 => 1,
            &Bank2 => 2,
            &Bank3 => 3,
            &Bank4 => 4,
            &Bank5 => 5,
            &Bank6 => 6,
            &Bank7 => 7,
        }
    }

    pub fn start_address(&self) -> u16 {
        use self::Bank::*;
        match self {
            &Bank0 => 0x0000,
            &Bank1 => 0x4000,
            &Bank2 => 0x8000,
            &Bank3 => 0xc000,
            _ => 0x4000
        }
    }

}




#[derive(Clone, PartialEq)]
pub struct PageDefinition {
    bank: Bank,
    start: u16,
    end: Option<u16>,
    name: Option<String>
}

impl Debug for PageDefinition {

    fn fmt(&self, f: &mut Formatter) -> Result {
        if self.end.is_some() {
            write!(f, "PageDefinition( bank: {:?}, start: 0x{:x}, end: 0x{:x})", &self.bank, &self.start, self.end().unwrap())
        }
        else {
            write!(f, "PageDefinition( bank: {:?}, start: 0x{:x}, end: None)", &self.bank, &self.start)
        }
    }
}


impl PageDefinition {

    pub fn new(bank: Bank, start:u16, end: Option<u16>) -> PageDefinition{
        assert!(start >= bank.start_address());
        assert!( (start as u32) < (bank.start_address() as u32 + 0x4000));

        if end.is_some() {
            assert!(end.unwrap() > start);
            assert!( (end.unwrap() as u32) < (bank.start_address() as u32 + 0x4000) );
        }

        PageDefinition {
            bank,
            start,
            end,
            name: None
        }
    }

    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    pub fn name(&self) -> Option<&String> {
        self.name.as_ref()
    }


    pub fn set_start(&mut self, start: u16) {
        assert!(start >= self.bank.start_address());
        assert!( (start as u32) < (self.bank.start_address() as u32 + 0x4000));

        if self.end.is_some() {
            assert!(self.end.unwrap() > start);
        }

        self.start = start;
    }


    pub fn bank(&self) -> &Bank {
        &self.bank
    }

    pub fn start(&self) -> u16 {
        self.start
    }

    pub fn end(&self) -> Option<u16> {
        self.end
    }

    pub fn contains_address(&self, address: u32) -> bool {
        self.start as u32 <= address && address <= self.end().unwrap() as u32
    }

    /// Check if there is overlapping between the two pages.
    /// Test is not done if one of the definition has no end
    pub fn overlaps(&self, other: &PageDefinition) -> bool {
        if self.end.is_none() && other.end.is_none() {
            // We do not test overlap when end is not specified
            false
        }
        else {
            let memory_overlaps = (self.start >= other.start && self.start <= other.end.unwrap())
            ||
            (self.end.unwrap() >= other.start && self.end.unwrap() <= other.end.unwrap());

            if !memory_overlaps {
                // We have no overlap
                false
            }
            else {
                // We have overlap BUT we do not care if the banks are different
                if self.bank() == other.bank() {
                    // Same bank meens overlap
                    true
                }
                else {
                    // overlap only when start/end is not in 0x4000-0x7fff
                    // get intersection (there IS an intersection)
                    let start = other.start;
                    let end = self.end.unwrap();

                    start < 0x4000 || start > 0x7fff || end < 0x4000 || end > 0x7fff
                }
            }

        }

    }


    /// Chekc that hte other page is contained by self. Cannot be used only when end address is
    /// given
    pub fn includes(&self, other: &PageDefinition) -> bool {
        if self.end.is_none() || other.end.is_none() {
            return false;
        }

        if self.bank != other.bank {
            return false;
        }

        // By definition, end is defined for everyone
        other.start() >= self.start() && other.end().unwrap() <= self.end().unwrap()
    }
}


pub struct StringCodePage {
    code: Vec<String>,
    current_address: u16,
    definition: PageDefinition
}


impl StringCodePage {
    pub fn new(definition: PageDefinition) -> StringCodePage {
        StringCodePage {
            code: Vec::new(),
            current_address: definition.start(),
            definition: definition
        }
    }


    pub fn get_page_definition(&self) -> & PageDefinition {
        &self.definition
    }

    fn maximum_address(&self) -> Option<u16> {
        self.definition.end
    }

    pub fn add_code( & mut self, asm:String, size:Option<u16>) {
        self.code.push(asm);

        if size.is_some() {
            assert!(self.remaining_space() >= size);
            self.current_address += size.unwrap();
        }
        else {
            assert!(!self.maximum_address().is_some());
        }

    }

    pub fn remaining_space(&self) -> Option<u16> {
        match self.maximum_address() {
            None => None,
            Some(address) => Some(address - self.current_address)
        }
    }

    pub fn can_contain(&self, size:Option<u16>) -> bool{
        match size {
            None => {assert!(!self.maximum_address().is_some()); true},
            Some(address) => self.remaining_space().unwrap() >= size.unwrap()
        }
    }


    pub fn save_code(&self, fname: &str) -> io::Result<()> {
         let mut out = File::create(fname)?;
         write!(out,"\torg 0x{:x}\n", self.definition.start())?;
         for instruction in self.code.iter() {
            write!(out,"{}", instruction)?;
         }
         if let Some(end) = self.definition.end() {
             write!(out,"\tassert $ <= 0x{:x}\n", end)?;
         }
         Ok(())
    }
}


pub struct StringCodePageContainer {
    pages: Vec<StringCodePage>,
    possibilities: Vec<PageDefinition>

}

impl StringCodePageContainer {

    pub fn new(mut possibilities: Vec<PageDefinition>) -> StringCodePageContainer {

        // As we pop possibilities, it is necessary to revert it
        possibilities.reverse();

        for page1 in possibilities.iter() {
            for page2 in possibilities.iter() {
                if page1 as *const _ != page2 as *const _ && page1.overlaps(page2) {
                    panic!("Error, {:?} overlaps {:?}", page1, page2);
                }
            }
        }

        StringCodePageContainer {
            pages: Vec::new(),
            possibilities: possibilities
        }
    }


    /// Add the current source code tp the current page if it can contain it.
    /// Otherwise, select another page.
    pub fn add_code( & mut self, asm:String, size:Option<u16>) {
        if self.pages.len() == 0 || !self.pages.last().unwrap().can_contain(size) {
            self.add_page();
        }

        // We are sure to unwrap
        self.pages.last_mut().unwrap().add_code(asm, size);
    }

    /// Return the current page. Attention, this page maybe full
    pub fn get_current_page_definition(& self) -> Option<&PageDefinition> {
        if self.pages.len() == 0 {
            None
        }
        else {
            Some(self.pages.last().unwrap().get_page_definition())
        }
    }


    pub fn add_page(&mut self) {
        println!("Create a new page");
        assert!(self.possibilities.len() > 0, "There is no room to add code");
        self.pages.push(StringCodePage::new(self.possibilities.pop().unwrap()));
    }


    pub fn save_code(&self, fname_prefix: &str) {
        for (idx, page) in self.pages.iter().enumerate() {
            page.save_code(&format!("{}_{}.asm", fname_prefix, idx));
        }
    }

}
