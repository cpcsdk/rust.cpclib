use smallvec::SmallVec;
use std::collections::HashMap;
use std::fmt;

use delegate::delegate;
use either::*;
use std::fmt::Display;

#[derive(Debug, Clone, Copy)]
pub enum SymbolError {
    UnknownAssemblingAddress
}




#[derive(Debug, Clone)]
pub struct Macro {
    name: String,
    args: Vec<String>,
    code: String
}


#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub enum Symbol {
    Integer(i32),
    Macro(Macro)
}

impl Symbol {
    pub fn integer(&self) -> Option<i32> {
        match self {
            Symbol::Integer(i) => Some(*i),
            _ => None
        }
    }

    pub fn r#macro(&self) -> Option<&Macro> {
        match self {
            Symbol::Macro(m) => Some(m),
            _ => None
        }
    }
}

impl From<Macro> for Symbol  {
    fn from(m: Macro) -> Self {
        Self::Macro(m)
    }
}


impl From<i32> for Symbol  {
    fn from(i: i32) -> Self {
        Self::Integer(i)
    }
}



#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub struct SymbolsTable {
    map: HashMap<String, Symbol>,
    dummy: bool,
}

impl Default for SymbolsTable {
    fn default() -> Self {
        Self {
            map: HashMap::new(),
            dummy: false,
        }
    }
}

#[allow(missing_docs)]
impl SymbolsTable {
    pub fn laxist() -> Self {
        let mut map = HashMap::new();
        map.insert(String::from("$"), Symbol::Integer(0));
        Self { map, dummy: true }
    }

    /// Return the current addres if it is known or return an error
    pub fn current_address(&self) -> Result<u16, SymbolError> {
        match self.value(&"$".to_owned()) {
            Some(address) => Ok(address.integer().unwrap() as u16),
            None => Err(SymbolError::UnknownAssemblingAddress),
        }
    }

    /// Update `$` value
    pub fn set_current_address(&mut self, address: u16) {
        self.map
            .insert(String::from("$"), Symbol::Integer(i32::from(address)));
    }

    /// Set the given symbol to $ value
    pub fn set_symbol_to_current_address<S: AsRef<str>>(
        &mut self,
        label: S,
    ) -> Result<(), SymbolError> {
        self.current_address().map(|val| {
            self.map
                .insert(label.as_ref().to_owned(), Symbol::Integer(i32::from(val)));
        })
    }

    /// Set the given symbol to the given value
    /// Return the previous value if any
    pub fn set_symbol_to_value<S: AsRef<str>, V: Into<Symbol>>(&mut self, label: S, value: V) -> Option<Symbol> {
        self.map
            .insert(
                label.as_ref().into(), 
                value.into()
        )
    }

    pub fn update_symbol_to_value<S: AsRef<str>, V: Into<Symbol>>(&mut self, label: S, value: V) {
        *(self.map.get_mut(label.as_ref()).unwrap()) = value.into();
    }

    /// Returns the symbol at the given key
    pub fn value<S: AsRef<str>>(&self, key: S) -> Option<&Symbol> {
        let key: String = key.as_ref().to_owned();

        let key = key.trim();
        self.map.get(key)
    }

    pub fn int_value<S: AsRef<str>>(&self, key: S) -> Option<i32> {
        self.value(key)
            .map(|v| v.integer())
            .map(|v| v.unwrap())
            .or_else(|| {if self.dummy {Some(1i32)}else{None}})
    }
    pub fn macro_value<S: AsRef<str>>(&self, key: S) -> Option<&Macro> {
        self.value(key)
            .map(|v| v.r#macro())
            .map(|v| v.unwrap())
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

    pub fn is_case_sensitive(&self) -> bool {
        self.case_sensitive
    }

    pub fn table(&self) -> &SymbolsTable {
        &self.table
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

    pub fn set_symbol_to_value<S: AsRef<str>, V: Into<Symbol>>(&mut self, symbol: S, value: V) -> Option<Symbol> {
        self.table
            .set_symbol_to_value(self.normalize_symbol(symbol), value)
    }

    pub fn update_symbol_to_value<S: AsRef<str>>(&mut self, symbol: S, value: i32) {
        self.table
            .update_symbol_to_value(self.normalize_symbol(symbol), value)
    }

    pub fn value<S: AsRef<str>>(&self, symbol: S) -> Option<&Symbol> {
        self.table.value(self.normalize_symbol(symbol))
    }

    pub fn int_value<S: AsRef<str>>(&self, symbol: S) -> Option<i32> {
        self.table.int_value(self.normalize_symbol(symbol))
    }

    pub fn macro_value<S: AsRef<str>>(&self, symbol: S) -> Option<&Macro> {
        self.table.macro_value(self.normalize_symbol(symbol))
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
            pub fn closest_symbol<S: AsRef<str>>(&self, symbol: S) -> Option<String>;
        }
    }
}