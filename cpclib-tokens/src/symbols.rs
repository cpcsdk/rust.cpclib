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

impl Macro {
    pub fn new(name: String, args: Vec<String>, code: String) -> Self {
        Macro {
            name,
            args,
            code
        }
    }

    pub fn code(&self) -> &str  {
        self.code.as_ref()
    }

    pub fn args(&self) -> &[String] {
        &self.args
    }

    pub fn nb_args(&self) -> usize {
        self.args.len()
    }
}
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub enum Value {
    Integer(i32),
    Macro(Macro)
}

impl Value {
    pub fn integer(&self) -> Option<i32> {
        match self {
            Value::Integer(i) => Some(*i),
            _ => None
        }
    }

    pub fn r#macro(&self) -> Option<&Macro> {
        match self {
            Value::Macro(m) => Some(m),
            _ => None
        }
    }
}

impl From<Macro> for Value  {
    fn from(m: Macro) -> Self {
        Self::Macro(m)
    }
}


impl From<i32> for Value  {
    fn from(i: i32) -> Self {
        Self::Integer(i)
    }
}


#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Symbol(String);

impl From<&str> for Symbol {
    fn from(s: &str) -> Symbol {
        s.to_owned().into()
    }
}

impl From<String> for Symbol {
    fn from(s: String) -> Symbol {
        Symbol(s)
    }
}

impl From<&String> for Symbol {
    fn from(s: &String) -> Symbol {
        Symbol(s.clone())
    }
}


impl Symbol {
    pub fn value(&self) -> &str {
        &self.0
    }

    pub fn is_local(&self) -> bool {
        self.0.contains('.')
    }

    pub fn to_uppercase(&self) -> Symbol {
        self.0.to_uppercase().into()
    }
}

#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub struct SymbolsTable {
    map: HashMap<Symbol, Value>,
    dummy: bool,
    current_label: String, //  Value of the current label to allow local labels
}

impl Default for SymbolsTable {
    fn default() -> Self {
        Self {
            map: HashMap::new(),
            dummy: false,
            current_label: "".into()
        }
    }
}

#[allow(missing_docs)]
impl SymbolsTable {
    pub fn laxist() -> Self {
        let mut map = HashMap::new();
        map.insert(Symbol::from("$"), Value::Integer(0));
        Self { map, dummy: true, current_label: "".into() }
    }

    // Setup the current label for local to global labels conversions
    pub fn set_current_label<S: Into<Symbol>>(&mut self, symbol: S) {
        self. current_label = symbol.into().value().to_owned();
    }

    /// Some symbols are local and need to be converted to their global value
    pub fn extend_symbol<S: Into<Symbol>>(&self, symbol: S) -> Symbol {
        let symbol = symbol.into();

        if symbol.value().starts_with('.') {
            (self.current_label.clone() + symbol.value()).into()
        } 
        else {
            symbol
        }
    }

    /// Return the current addres if it is known or return an error
    pub fn current_address(&self) -> Result<u16, SymbolError> {
        match self.value("$") {
            Some(address) => Ok(address.integer().unwrap() as u16),
            None => Err(SymbolError::UnknownAssemblingAddress),
        }
    }

    /// Update `$` value
    pub fn set_current_address(&mut self, address: u16) {
        self.map
            .insert(
                "$".into(), 
                Value::Integer(i32::from(address))
            );
    }

    /// Set the given Value to $ value
    pub fn set_symbol_to_current_address<S: Into<Symbol>>(
        &mut self,
        symbol: S,
    ) -> Result<(), SymbolError> {
        let symbol = self.extend_symbol(symbol);
        self.current_address().map(|val| {
            self.map
                .insert(
                    symbol, 
                    Value::Integer(i32::from(val))
                );
        })
    }

    /// Set the given Value to the given value
    /// Return the previous value if any
    pub fn set_symbol_to_value<S: Into<Symbol>, V: Into<Value>>(&mut self, symbol: S, value: V) -> Option<Value> {
        let symbol = self.extend_symbol(symbol);

        self.map
            .insert(
                symbol, 
                value.into()
        )
    }

    pub fn update_symbol_to_value<S: Into<Symbol>, V: Into<Value>>(&mut self, symbol: S, value: V) {

        let symbol = self.extend_symbol(symbol);
        *(self.map.get_mut(&symbol).unwrap()) = value.into();
    }

    /// Returns the Value at the given key
    pub fn value<S: Into<Symbol>>(&self, symbol: S) -> Option<&Value> {
        let symbol = self.extend_symbol(symbol);
        self.map.get(&symbol)
    }

    pub fn int_value<S: Into<Symbol>>(&self, symbol: S) -> Option<i32> {
        let symbol = self.extend_symbol(symbol);
        self.value(symbol)
            .map(|v| v.integer())
            .map(|v| v.unwrap())
            .or_else(|| {if self.dummy {Some(1i32)}else{None}})
    }
    pub fn macro_value<S: Into<Symbol>>(&self, symbol: S) -> Option<&Macro> {
        let symbol = self.extend_symbol(symbol);
        self.value(symbol)
            .map(|v| v.r#macro())
            .map(|v| v.unwrap())
    }


    /// Remove the given Value name from the table. (used by undef)
    pub fn remove_symbol<S: Into<Symbol>>(&mut self, symbol: S) -> Option<Value> {
        let symbol = self.extend_symbol(symbol);
        self.map.remove(&symbol)
    }

    pub fn contains_symbol<S: Into<Symbol>>(&self, symbol: S) -> bool {
        let symbol = self.extend_symbol(symbol);
        self.map.contains_key(&symbol)
    }

    /// Returns the closest Value
    pub fn closest_symbol<S: Into<Symbol>>(&self, symbol: S) -> Option<String> {
        let symbol = self.extend_symbol(symbol);
        self.map
            .keys()
            .map(move |symbol2| 
                (
                    strsim::levenshtein(&symbol2.0, &symbol.0), 
                    symbol2.0.clone()
                )
            )
            .min()
            .map(|(_distance, symbol2)| symbol2)
    }
}

/// Wrapper around the Values table in order to easily manage the fact that the assembler is case dependent or independant
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

    /// Build a laxists vesion of the table : do not care of case and absences of Valuees
    pub fn laxist() -> Self {
        Self::new(SymbolsTable::laxist(), false)
    }

    /// Modify the Value value depending on the case confurigration (do nothing, or set uppercase)
    fn normalize_symbol<S: Into<Symbol>>(&self, symbol: S) -> Symbol {
        if self.case_sensitive {
            symbol.into()
        } else {
            symbol.into().to_uppercase()
        }

    }

    pub fn set_table(&mut self, table: SymbolsTable) {
        self.table = table
    }

        // Setup the current label for local to global labels conversions
    pub fn set_current_label<S: Into<Symbol>>(&mut self, symbol: S) {
            self.table
                .set_current_label(self.normalize_symbol(symbol))
     }

    pub fn set_symbol_to_current_address<S: Into<Symbol>>(
        &mut self,
        symbol: S,
    ) -> Result<(), SymbolError> {
        self.table
            .set_symbol_to_current_address(self.normalize_symbol(symbol))
    }

    pub fn set_symbol_to_value<S: Into<Symbol>, V: Into<Value>>(&mut self, symbol: S, value: V) -> Option<Value> {
        self.table
            .set_symbol_to_value(self.normalize_symbol(symbol), value)
    }

    pub fn update_symbol_to_value<S: Into<Symbol>>(&mut self, symbol: S, value: i32) {
        self.table
            .update_symbol_to_value(self.normalize_symbol(symbol), value)
    }

    pub fn value<S: Into<Symbol>>(&self, symbol: S) -> Option<&Value> {
        self.table.value(self.normalize_symbol(symbol))
    }

    pub fn int_value<S: Into<Symbol>>(&self, symbol: S) -> Option<i32> {
        self.table.int_value(self.normalize_symbol(symbol))
    }

    pub fn macro_value<S: Into<Symbol>>(&self, symbol: S) -> Option<&Macro> {
        self.table.macro_value(self.normalize_symbol(symbol))
    }


    pub fn remove_symbol<S: Into<Symbol>>(&mut self, symbol: S) -> Option<Value> {
        self.table.remove_symbol(self.normalize_symbol(symbol))
    }

    pub fn contains_symbol<S: Into<Symbol>>(&self, symbol: S) -> bool {
        self.table.contains_symbol(self.normalize_symbol(symbol))
    }

    delegate! {
        target self.table {
            pub fn current_address(&self) -> Result<u16, SymbolError>;
            pub fn set_current_address(&mut self, address: u16);
            pub fn closest_symbol<S: Into<Symbol>>(&self, symbol: S) -> Option<String>;
        }
    }
}