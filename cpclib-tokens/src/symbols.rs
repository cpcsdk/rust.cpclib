use smallvec::SmallVec;
use std::collections::HashMap;
use std::fmt;

use delegate::delegate;
use either::*;
use std::fmt::Display;

use crate::tokens::expression::LabelPrefix;

#[derive(Debug, Clone, Copy)]
pub enum SymbolError {
    UnknownAssemblingAddress
}


#[derive(Debug, Clone, Copy)]
#[allow(missing_docs)]
pub enum Symbol {
    Integer(i32),
}


#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub struct SymbolsTable {
    /// The value of each symbol
    map: HashMap<String, Symbol>,
    /// The page of each symbol
    page: HashMap<String, u8>,
    /// The current page. it is automatically set to a symbol when the symbol is added
    current_page: u8,
    dummy: bool,
}

impl Default for SymbolsTable {
    fn default() -> Self {
        Self {
            map: HashMap::new(),
            page: HashMap::new(),
            dummy: false,
            current_page: 0
        }
    }
}

#[allow(missing_docs)]
impl SymbolsTable {
    pub fn laxist() -> Self {
        let mut new = Self::default();
        new.map.insert(String::from("$"), Symbol::Integer(0));
        new.dummy = true;
        new
    }

    /// Return the current addres if it is known or return an error
    pub fn current_address(&self) -> Result<u16, SymbolError> {
        match self.value(&"$".to_owned()) {
            Some(address) => Ok(address as u16),
            None => Err(SymbolError::UnknownAssemblingAddress),
        }
    }

    /// Update `$` value
    pub fn set_current_address(&mut self, address: u16) {
        self.map
            .insert(String::from("$"), Symbol::Integer(i32::from(address)));
    }

    pub fn set_current_page(&mut self, page: u8) {
        self.current_page = page;
    }

    /// Set the given symbol to $ value
    pub fn set_symbol_to_current_address<S: AsRef<str>>(
        &mut self,
        label: S,
    ) -> Result<(), SymbolError> {
        self.current_address().map(|val| {
            let label = label.as_ref().to_owned();
            self.map
                .insert(label.clone(), Symbol::Integer(i32::from(val)));
            self.page.insert(label, self.current_page);
        })
    }

    /// Set the given symbol to the given value
    pub fn set_symbol_to_value<S: AsRef<str>>(&mut self, label: S, value: i32) {
        let label = label.as_ref().to_owned();
        self.map
            .insert(label.clone(), Symbol::Integer(value));
        self.page.insert(label, self.current_page);

    }

    /// Update the value of the symbol.
    /// Does not update its page. No idea if it is necessary
    pub fn update_symbol_to_value<S: AsRef<str>>(&mut self, label: S, value: i32) {
        *(self.map.get_mut(label.as_ref()).unwrap()) = Symbol::Integer(value);
    }

    /// TODO return the symbol instead of the int
    pub fn value<S: AsRef<str>>(&self, key: S) -> Option<i32> {
        let key: String = key.as_ref().to_owned();

        let key = key.trim();
        let res = self.map.get(key);
        if let Some(&Symbol::Integer(val)) = res {
            Some(val)
        } else if self.dummy {
            //eprintln!("{} not found in symbol table. I have replaced it by 1", key);
            Some(1)
        } else {
            //               eprintln!("Symbol table content {:?}", &self.map);
            None
        }
    }

    /// Instead of returning the value, return the bank information
    /// logic stolen to rasm
    pub fn prefixed_value<S: AsRef<str>>(&self, prefix:& LabelPrefix, key: S) -> Option<u16> {

        /* rasm code
        for (i=0;i<4;i++) {
            ae->bankgate[i]=0x7FC0; /* video memory has no paging */
            ae->setgate[i]=0x7FC0; /* video memory has no paging */
        }
        for (i=0;i<256;i++) {
            /* 4M expansion support on lower gate array port */
            ae->bankgate[i+4]=0x7FC4+(i&3)+((i&31)>>2)*8-0x100*(i>>5);
            ae->setgate[i+4] =0x7FC2      +((i&31)>>2)*8-0x100*(i>>5);
            //printf("%04X %04X\n",ae->bankgate[i+4],ae->setgate[i+4]);
        }
 */
        

        let key = key.as_ref();
        let page = *self.page.get(key).unwrap() as u16;
        let value = self.value(key).unwrap() as u16;
        let bank = value/0x4000;

        eprintln!("{} {} {} {}", key, value, page, bank);
        match prefix {
            LabelPrefix::Bank => {
                Some(bank as _)
            },

            LabelPrefix::Page => {
                if page == 0 {
                    Some(0x7fc0)
                }
                else {
                    Some(0x7FC4+(bank&3)+((bank&31)>>2)*8-0x100*(bank>>5))
                }
            },

            LabelPrefix::Pageset => {
                if page == 0 {
                    Some(0x7fc0)
                }
                else {
                    Some(0x7FC2+((bank&31)>>2)*8-0x100*(bank>>5))
                }          
            }
        }
    }


    /// Remove the given symbol name from the table. (used by undef)
    pub fn remove_symbol<S: AsRef<str>>(&mut self, key: S) -> Option<Symbol> {
        self.map.remove(key.as_ref())
    }

    pub fn contains_symbol<S: AsRef<str>>(&self, key: S) -> bool {
        self.map.contains_key(&key.as_ref().to_owned())
    }

    /// Returns the closest symbol
    pub fn closest_symbol<S: AsRef<str>>(&self, symbol: S) -> Option<String> {
        self.map
            .keys()
            .map(move |symbol2| (strsim::levenshtein(symbol2, symbol.as_ref()), symbol2))
            .min()
            .map(|(_distance, symbol2)| symbol2.to_owned())
    }
}

/// Wrapper around the symbols table in order to easily manage the fact that the assembler is case dependent or independant
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub struct SymbolsTableCaseDependent {
    table: SymbolsTable,
    case_sensitive: bool,
}

/// By default, the assembler is case sensitive
impl Default for SymbolsTableCaseDependent {
    fn default() -> Self {
        Self {
            table: SymbolsTable::default(),
            case_sensitive: true,
        }
    }
}

impl AsRef<SymbolsTable> for SymbolsTableCaseDependent {
    fn as_ref(&self) -> &SymbolsTable {
        &self.table
    }
}

#[allow(missing_docs)]
impl SymbolsTableCaseDependent {
    pub fn new(table: SymbolsTable, case_sensitive: bool) -> Self {
        Self {
            table,
            case_sensitive,
        }
    }

    /// Build a laxists vesion of the table : do not care of case and absences of symboles
    pub fn laxist() -> Self {
        Self::new(SymbolsTable::laxist(), false)
    }

    /// Modify the symbol value depending on the case confurigration (do nothing, or set uppercase)
    fn normalize_symbol<S: AsRef<str>>(&self, symbol: S) -> String {
        let new = if self.case_sensitive {
            symbol.as_ref().to_owned()
        } else {
            symbol.as_ref().to_uppercase()
        };

        eprintln!(
            "[{}] {:?} => {:?} ",
            self.case_sensitive,
            symbol.as_ref(),
            new
        );

        new
    }

    pub fn set_table(&mut self, table: SymbolsTable) {
        self.table = table
    }

    pub fn set_symbol_to_current_address<S: AsRef<str>>(
        &mut self,
        symbol: S,
    ) -> Result<(), SymbolError> {
        self.table
            .set_symbol_to_current_address(self.normalize_symbol(symbol))
    }

    pub fn set_symbol_to_value<S: AsRef<str>>(&mut self, symbol: S, value: i32) {
        self.table
            .set_symbol_to_value(self.normalize_symbol(symbol), value)
    }

    pub fn update_symbol_to_value<S: AsRef<str>>(&mut self, symbol: S, value: i32) {
        self.table
            .update_symbol_to_value(self.normalize_symbol(symbol), value)
    }

    pub fn value<S: AsRef<str>>(&self, symbol: S) -> Option<i32> {
        self.table.value(self.normalize_symbol(symbol))
    }
    pub fn prefixed_value<S: AsRef<str>>(&self, prefix:& LabelPrefix, symbol: S) -> Option<u16> {
        self.table.prefixed_value(prefix, self.normalize_symbol(symbol))
    }

    pub fn remove_symbol<S: AsRef<str>>(&mut self, symbol: S) -> Option<Symbol> {
        self.table.remove_symbol(self.normalize_symbol(symbol))
    }

    pub fn contains_symbol<S: AsRef<str>>(&self, symbol: S) -> bool {
        self.table.contains_symbol(self.normalize_symbol(symbol))
    }

    delegate! {
        target self.table {
            pub fn current_address(&self) -> Result<u16, SymbolError>;
            pub fn set_current_address(&mut self, address: u16);
            pub fn set_current_page(&mut self, page: u8);
            pub fn closest_symbol<S: AsRef<str>>(&self, symbol: S) -> Option<String>;
        }
    }
}