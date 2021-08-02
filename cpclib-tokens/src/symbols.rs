use std::collections::HashMap;

use delegate::delegate;
use itertools::Itertools;

use crate::tokens::expression::LabelPrefix;
use crate::{Expr, MacroParam, Token};

use std::ops::Deref;
#[derive(Debug)]
pub enum SymbolError {
    UnknownAssemblingAddress,
    WrongSymbol(String),
}

/// Encode the data for the structure directive
#[derive(Debug, Clone)]
pub struct Struct {
    name: String,
    content: Vec<(String, Token)>,
}

impl Struct {
    pub fn new(name: impl AsRef<str>, content: &[(String, Token)]) -> Self {
        Self {
            name: name.as_ref().to_owned(),
            content: content
                .iter()
                .map(|(s, t)| (s.clone(), t.clone()))
                .collect_vec(),
        }
    }

    pub fn fields_size(&self, table: &SymbolsTable) -> Vec<(&str, i32)> {
        self.content
            .iter()
            .map(|(n, t)| (n.as_ref(), Self::field_size(t, table)))
            .collect_vec()
    }

    /// Get the len of any field
    pub fn field_size(token: &Token, table: &SymbolsTable) -> i32 {
        match token {
            Token::Defb(c) => c.len() as i32,
            Token::Defw(c) => 2 * c.len() as i32,
            Token::MacroCall(n, _) => {
                let s = table.struct_value(n).ok().unwrap().unwrap(); // TODO handle error here
                s.len(table)
            }
            _ => unreachable!("{:?}", token),
        }
    }

    /// Get the len of the structure
    pub fn len(&self, table: &SymbolsTable) -> i32 {
        self.fields_size(table).iter().map(|(_, s)| *s).sum()
    }

    pub fn nb_args(&self) -> usize {
        self.content.len()
    }

    /// Generate the token that correspond to the current structure
    /// Current bersion does not handle at all directive with several arguments
    pub fn develop(&self, args: &[MacroParam]) -> String {
        assert_eq!(args.len(), self.content.len());

        self.content
            .iter()
            .zip(args.iter())
            .enumerate()
            .map(|(idx, ((name, token), current_param))| {
                match token {
                    Token::Defb(c) | Token::Defw(c) => {
                        assert_eq!(c.len(), 1);

                        let tok = if matches!(token, Token::Defb(_)) {
                            "db"
                        } else {
                            "dw"
                        };

                        if current_param.is_empty() {
                            format!(" {} {}", tok, c[0].to_string())
                        } else {
                            format!(" {} {}", tok, current_param.expand())
                        }
                    }

                    Token::MacroCall(n, current_default_arg) => {
                        let mut call = format!(" {} ", n);

                        // The way to manage default/provided params differ depending on the combination
                        let args = match (current_param, current_default_arg.len()) {
                            // no default
                            (_, 0) => {
                                vec![current_param.expand()]
                            }

                            // one default
                            (_, 1) => {
                                let val = if current_param.is_empty() {
                                    &current_default_arg[0]
                                } else {
                                    current_param
                                };
                                vec![val.expand()]
                            }

                            // default is several, provided is single. Use provided only if not empty
                            (MacroParam::Single(_), nb_default) => {
                                let mut default_iter = current_default_arg.iter();
                                let first_default = default_iter.next().unwrap();
                                let mut collected = Vec::new();
                                collected.push(if current_param.is_empty() {
                                    first_default
                                } else {
                                    current_param
                                });
                                collected.extend(default_iter);

                                collected.iter().map(|p| p.expand()).collect_vec()
                            }

                            // default and provided are several
                            (MacroParam::List(all_curr), nb_default) => {
                                let max_size = all_curr.len().max(nb_default);

                                let mut collected = Vec::new();
                                for idx2 in 0..max_size {
                                    if idx2 >= all_curr.len() {
                                        collected.push(current_default_arg[idx2].expand());
                                    } else if idx2 >= nb_default {
                                        collected.push(all_curr[idx2].expand());
                                    } else {
                                        let current = &all_curr[idx2];
                                        let default = &current_default_arg[idx2];

                                        if current.is_empty() {
                                            collected.push(default.expand());
                                        } else {
                                            collected.push(current.expand());
                                        }
                                    }
                                }
                                collected
                            }
                        };

                        call.push_str(&args.join(","));
                        call
                    }
                    _ => unreachable!("{:?}", token),
                }
            })
            .join("\n")
    }
}
#[derive(Debug, Clone)]
pub struct Macro {
    // The name of the macro
    name: String,
    // The name of its arguments
    args: Vec<String>,
    // The content
    code: String,
}

impl Macro {
    pub fn new(name: String, args: Vec<String>, code: String) -> Self {
        Macro { name, args, code }
    }

    pub fn code(&self) -> &str {
        self.code.as_ref()
    }

    pub fn args(&self) -> &[String] {
        &self.args
    }

    pub fn nb_args(&self) -> usize {
        self.args.len()
    }

    /// Develop the macro with the given arguments
    pub fn develop(&self, args: &[MacroParam]) -> String {

        dbg!(&self);
        assert_eq!(args.len(), self.nb_args());

        let mut listing = self.code.to_string();

        // replace the arguments for the listing
        for (argname, argvalue) in self.args.iter().zip(args.iter()) {
            listing = listing.replace(&format!("{{{}}}", argname), &argvalue.expand());
        }

        listing
    }
}

#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub enum Value {
    /// Integer value used in an expression
    Integer(i32),
    /// Macro information
    Macro(Macro),
    /// Structure information
    Struct(Struct),
    /// Counter for a repetition
    Counter(i32)
}

impl Value {
    pub fn integer(&self) -> Option<i32> {
        match self {
            Value::Integer(i) => Some(*i),
            _ => None,
        }
    }

    pub fn counter(&self) -> Option<i32> {
        match self {
            Value::Counter(i) => Some(*i),
            _ => None,
        }
    }

    pub fn r#macro(&self) -> Option<&Macro> {
        match self {
            Value::Macro(m) => Some(m),
            _ => None,
        }
    }
    pub fn r#struct(&self) -> Option<&Struct> {
        match self {
            Value::Struct(m) => Some(m),
            _ => None,
        }
    }
}
impl From<Struct> for Value {
    fn from(m: Struct) -> Self {
        Self::Struct(m)
    }
}

impl From<Macro> for Value {
    fn from(m: Macro) -> Self {
        Self::Macro(m)
    }
}

impl From<i32> for Value {
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
    /// The page of each symbol
    page: HashMap<Symbol, u8>,
    /// The current page. it is automatically set to a symbol when the symbol is added
    current_page: u8,
    map: HashMap<Symbol, Value>,
    dummy: bool,
    current_label: String, //  Value of the current label to allow local labels
    seed_stack: Vec<usize> // stack of seeds for nested repeat to properly interpret the @ symbol
}

impl Default for SymbolsTable {
    fn default() -> Self {
        Self {
            map: HashMap::new(),
            page: HashMap::new(),
            dummy: false,
            current_page: 0,
            current_label: "".into(),
            seed_stack: Vec::new()
        }
    }
}

#[allow(missing_docs)]
impl SymbolsTable {
    pub fn laxist() -> Self {
        let mut map = HashMap::new();
        map.insert(Symbol::from("$"), Value::Integer(0));
        Self {
            map,
            dummy: true,
            current_page: 0,
            page: Default::default(),
            current_label: "".into(),
            seed_stack: Vec::new()
        }
    }

    // Setup the current label for local to global labels conversions
    pub fn set_current_label<S: Into<Symbol>>(&mut self, symbol: S) -> Result<(), SymbolError> {
        self.current_label = self.extend_symbol(symbol)?.value().to_owned();
        Ok(())
    }

    /// Add a new seed for the @ symbol name resolution (we enter in a repeat)
    pub fn push_seed(&mut self, seed: usize) {
        self.seed_stack.push(seed)
    }

    /// Remove the previous seed for the @ symbol name resolution (<e leave a repeat)
    pub fn pop_seed(&mut self) {
        self.seed_stack.pop();
    }


    /// Some symbols are local and need to be converted to their global value.
    pub fn extend_symbol<S: Into<Symbol>>(&self, symbol: S) -> Result<Symbol, SymbolError> {
        let symbol = symbol.into();
        let mut symbol = symbol.value().to_owned();

         if symbol.starts_with('.') {
             symbol = self.current_label.clone() + &symbol ;
        }

        // handle the hidden labels from repeats
        if symbol.starts_with("@") {
            match self.seed_stack.last() {
                Some(seed) => {
                    // we need to rewrite the symbol name to make it unique
                    symbol = format!("__hidden__{}__{}", seed, &symbol[1..]);
                }
                None => {
                    // we cannot have a symbol with @ here
                    return Err(SymbolError::WrongSymbol(symbol));
                }
            }

        }

        Ok(symbol.into())
    }

    /// Return the current addres if it is known or return an error
    pub fn current_address(&self) -> Result<u16, SymbolError> {
        match self.value("$")? {
                    Some(address) => Ok(address.integer().unwrap() as u16),
                    None => Err(SymbolError::UnknownAssemblingAddress),
                }
    }

    /// Update `$` value
    pub fn set_current_address(&mut self, address: u16) {
        self.map
            .insert("$".into(), Value::Integer(i32::from(address)));
    }

    pub fn set_current_page(&mut self, page: u8) {
        self.current_page = page;
    }

    /// Set the given symbol to $ value
    pub fn set_symbol_to_current_address<S: Into<Symbol>>(
        &mut self,
        symbol: S,
    ) -> Result<(), SymbolError> {
        let symbol = self.extend_symbol(symbol)?;
        self.current_address().map(|val| {
            self.map.insert(symbol, Value::Integer(i32::from(val)));
        })
    }

    /// Set the given Value to the given value
    /// Return the previous value if any
    pub fn set_symbol_to_value<S: Into<Symbol>, V: Into<Value>>(
        &mut self,
        symbol: S,
        value: V,
    ) -> Result<Option<Value>, SymbolError> {
        let symbol = self.extend_symbol(symbol)?;

        Ok(self.map.insert(symbol, value.into()))
    }

    pub fn update_symbol_to_value<S: Into<Symbol>, V: Into<Value>>(&mut self, symbol: S, value: V) -> Result<(), SymbolError>{
        let symbol = self.extend_symbol(symbol)?;
        *(self.map.get_mut(&symbol).unwrap()) = value.into();
        Ok(())
    }

    /// Returns the Value at the given key
    pub fn value<S: Into<Symbol>>(&self, symbol: S) -> Result<Option<&Value>, SymbolError> {
        let symbol = self.extend_symbol(symbol)?;
        Ok(self.map.get(&symbol))
    }

    pub fn counter_value<S: Into<Symbol>>(&self, symbol: S) -> Result<Option<i32>, SymbolError> {
        Ok(self.value(symbol.into())?
                    .map(|v| v.counter())
                    .map(|v| v.unwrap())
                    .or_else(|| if self.dummy { Some(1i32) } else { None }))
    }

    pub fn int_value<S: Into<Symbol>>(&self, symbol: S) -> Result<Option<i32>, SymbolError> {
        Ok(self.value(symbol)?
                    .map(|v| v.integer())
                    .map(|v| v.unwrap())
                    .or_else(|| if self.dummy { Some(1i32) } else { None }))
    }
    pub fn macro_value<S: Into<Symbol>>(&self, symbol: S) -> Result<Option<&Macro>, SymbolError> {
        Ok(self.value(symbol)?.map(|v| v.r#macro()).unwrap_or(None))
    }
    pub fn struct_value<S: Into<Symbol>>(&self, symbol: S) -> Result<Option<&Struct>, SymbolError> {
        Ok(self.value(symbol)?.map(|v| v.r#struct()).unwrap_or(None))
    }

    /// Instead of returning the value, return the bank information
    /// logic stolen to rasm
    pub fn prefixed_value<S: Into<Symbol>>(&self, prefix: &LabelPrefix, key: S) -> Result< Option<u16>, SymbolError> {
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

        let key = key.into();

        let page = *self
            .page
            .get(&key)
            .or_else(|| Some(&self.current_page))
            .unwrap() as u16;
        let value = self.value(key)?.unwrap().integer().unwrap() as u16;
        let bank = value / 0x4000;

        Ok(match prefix {
                    LabelPrefix::Bank => Some(bank as _),
        
                    LabelPrefix::Page => {
                        if page == 0 {
                            Some(0x7fc0)
                        } else {
                            Some(0x7FC4 + (bank & 3) + ((bank & 31) >> 2) * 8 - 0x100 * (bank >> 5))
                        }
                    }
        
                    LabelPrefix::Pageset => {
                        if page == 0 {
                            Some(0x7fc0)
                        } else {
                            Some(0x7FC2 + ((bank & 31) >> 2) * 8 - 0x100 * (bank >> 5))
                        }
                    }
                })
    }

    /// Remove the given symbol name from the table. (used by undef)
    pub fn remove_symbol<S: Into<Symbol>>(&mut self, symbol: S) -> Result<Option<Value>, SymbolError> {
        let symbol = self.extend_symbol(symbol)?;
        Ok(self.map.remove(&symbol))
    }

    pub fn contains_symbol<S: Into<Symbol>>(&self, symbol: S) -> Result<bool, SymbolError> {
        let symbol = self.extend_symbol(symbol)?;
        Ok(self.map.contains_key(&symbol))
    }

    /// Returns the closest Value
    pub fn closest_symbol<S: Into<Symbol>>(&self, symbol: S) -> Result<Option<String>, SymbolError> {
        let symbol = self.extend_symbol(symbol)?;
        Ok(self.map
                    .keys()
                    .map(move |symbol2| {
                        (
                            strsim::levenshtein(&symbol2.0, &symbol.0),
                            symbol2.0.clone(),
                        )
                    })
                    .min()
                    .map(|(_distance, symbol2)| symbol2))
    }


    pub fn kind<S: Into<Symbol>>(&self, symbol: S) -> Result<&'static str, SymbolError> {
        Ok(match self.value(symbol)? {
                    Some(Value::Integer(_)) => "integer",
                    Some(Value::Macro(_)) => "macro",
                    Some(Value::Struct(_)) => "struct",
                    _ => panic!()
                })
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
    pub fn set_current_label<S: Into<Symbol>>(&mut self, symbol: S) -> Result<(), SymbolError> {
        self.table.set_current_label(self.normalize_symbol(symbol))
    }

    pub fn set_symbol_to_current_address<S: Into<Symbol>>(
        &mut self,
        symbol: S,
    ) -> Result<(), SymbolError> {
        self.table
            .set_symbol_to_current_address(self.normalize_symbol(symbol))
    }

    pub fn set_symbol_to_value<S: Into<Symbol>, V: Into<Value>>(
        &mut self,
        symbol: S,
        value: V,
    ) -> Result<Option<Value>, SymbolError> {
        self.table
            .set_symbol_to_value(self.normalize_symbol(symbol), value)
    }

    pub fn update_symbol_to_value<S: Into<Symbol>>(&mut self, symbol: S, value: i32) -> Result<(), SymbolError> {
        self.table
            .update_symbol_to_value(self.normalize_symbol(symbol), value)
    }

    pub fn value<S: Into<Symbol>>(&self, symbol: S) -> Result<Option<&Value>, SymbolError> {
        self.table.value(self.normalize_symbol(symbol))
    }
    pub fn prefixed_value<S: Into<Symbol>>(&self, prefix: &LabelPrefix, symbol: S) -> Result<Option<u16>, SymbolError> {
        self.table
            .prefixed_value(prefix, self.normalize_symbol(symbol))
    }

    pub fn int_value<S: Into<Symbol>>(&self, symbol: S) -> Result<Option<i32>, SymbolError> {
        self.table.int_value(self.normalize_symbol(symbol))
    }

    pub fn counter_value<S: Into<Symbol>>(&self, symbol: S) -> Result<Option<i32>, SymbolError> {
        self.table.counter_value(self.normalize_symbol(symbol))
    }


    pub fn macro_value<S: Into<Symbol>>(&self, symbol: S) -> Result<Option<&Macro>, SymbolError>  {
        self.table.macro_value(self.normalize_symbol(symbol))
    }
    pub fn struct_value<S: Into<Symbol>>(&self, symbol: S) -> Result<Option<&Struct>, SymbolError>  {
        self.table.struct_value(self.normalize_symbol(symbol))
    }

    pub fn remove_symbol<S: Into<Symbol>>(&mut self, symbol: S) -> Result<Option<Value>, SymbolError>  {
        self.table.remove_symbol(self.normalize_symbol(symbol))
    }

    pub fn contains_symbol<S: Into<Symbol>>(&self, symbol: S) -> Result<bool, SymbolError>  {
        self.table.contains_symbol(self.normalize_symbol(symbol))
    }

    pub fn kind<S: Into<Symbol>>(&self, symbol: S) -> Result<&'static str, SymbolError> {
        self.table.kind(symbol)
    }
    delegate! {
        to self.table {
            pub fn current_address(&self) -> Result<u16, SymbolError>;
            pub fn set_current_address(&mut self, address: u16);
            pub fn set_current_page(&mut self, page: u8);
            pub fn closest_symbol<S: Into<Symbol>>(&self, symbol: S) -> Result<Option<String>, SymbolError>;
            pub fn push_seed(&mut self, seed: usize);
            pub fn pop_seed(&mut self);
        }
    }
}
