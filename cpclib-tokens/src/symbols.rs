use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display};

use cpclib_common::itertools::Itertools;
use cpclib_common::smallvec::{smallvec, SmallVec};
use cpclib_common::{lazy_static, strsim};
use delegate::delegate;
use regex::Regex;

use crate::tokens::expression::LabelPrefix;
use crate::{ExprResult, MacroParam, Token};

use std::ops::Deref;
use std::ops::DerefMut;

/// Structure that ease the addresses manipulation to read/write at the right place
#[derive(Debug, Clone)]
pub struct PhysicalAddress {
    /// Page number (0 for base, 1 for first page, 2 ...)
    page: u8,
    /// Bank number in the page: 0 to 3
    bank: u8,
    /// Address manipulate by CPU 0x0000 to 0xffff
    address: u16,
}

impl From<u16> for PhysicalAddress {
    fn from(nb: u16) -> Self {
        PhysicalAddress::new(nb, 0xc0)
    }
}

impl PhysicalAddress {
    pub fn new(address: u16, mmr: u8) -> Self {
        let possible_page = ((mmr >> 3) & 0b111) + 1;
        let possible_bank = mmr & 0b11;
        let standard_bank = match address {
            0x0000..0x4000 => 0,
            0x4000..0x8000 => 1,
            0x8000..0xc000 => 2,
            0xc000.. => 3,
        };
        let is_4000 = address >= 0x4000 && address < 0x8000;
        let is_c000 = address >= 0xc000;

        let (page, bank) = if (mmr & 0b100) != 0 {
            if is_4000 {
                (possible_page, possible_bank)
            } else {
                (0, possible_bank)
            }
        } else {
            match mmr & 0b11 {
                0b000 => (0, standard_bank),
                0b001 => {
                    if is_c000 {
                        (possible_page, standard_bank)
                    } else {
                        (0, standard_bank)
                    }
                }
                0b010 => (possible_page, standard_bank),
                0b011 => {
                    if is_4000 {
                        (0, 3)
                    } else if is_c000 {
                        (possible_page, 3)
                    } else {
                        (0, standard_bank)
                    }
                }
                _ => unreachable!(),
            }
        };

        Self {
            address,
            bank,
            page,
        }
    }

    pub fn offset_in_bank(&self) -> u16 {
        self.address % 0x4000
    }
    pub fn offset_in_page(&self) -> u16 {
        self.offset_in_bank() + self.bank as u16 * 0x4000
    }
    pub fn offset_in_cpc(&self) -> u32 {
        self.offset_in_page() as u32 + self.page as u32 * 0x1_0000
    }
    pub fn address(&self) -> u16 {
        self.address
    }
    pub fn bank(&self) -> u8 {
        self.bank
    }
    pub fn page(&self) -> u8 {
        self.page
    }

    pub fn ga_bank(&self) -> u16 {
        let low = if self.page() == 0 {
            0b11_000_0_00
        } else {
            0b11_000_1_00 + ((self.page() - 1) << 3) + self.bank
        } as u16;
        low + 0x7f00
    }

    pub fn ga_page(&self) -> u16 {
        let low = if self.page() == 0 {
            0b11_000_0_00
        } else {
            0b11_000_0_10 + ((self.page() - 1) << 3)
        } as u16;
        low + 0x7f00
    }
}

#[derive(Debug, Clone)]
pub enum SymbolError {
    UnknownAssemblingAddress,
    CannotModify(Symbol),
    WrongSymbol(Symbol),
    NoNamespaceActive,
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

    /// Get the size of each field
    pub fn fields_size<T: SymbolsTableTrait>(&self, table: &T) -> Vec<(&str, i32)> {
        self.content
            .iter()
            .map(|(n, t)| (n.as_ref(), Self::field_size(t, table)))
            .collect_vec()
    }

    /// Get the len of any field
    pub fn field_size<T: SymbolsTableTrait>(token: &Token, table: &T) -> i32 {
        match token {
            Token::Defb(c) => c.len() as i32,
            Token::Defw(c) => 2 * c.len() as i32,
            Token::MacroCall(n, _) => {
                dbg!(n);
                let s = dbg!(table.struct_value(n)).ok().unwrap().unwrap(); // TODO handle error here
                s.len(table)
            }
            _ => unreachable!("{:?}", token),
        }
    }

    /// Get the len of the structure
    pub fn len<T: SymbolsTableTrait>(&self, table: &T) -> i32 {
        self.fields_size(table).iter().map(|(_, s)| *s).sum()
    }

    pub fn nb_args(&self) -> usize {
        self.content.len()
    }

    /// Generate the token that correspond to the current structure
    /// Current bersion does not handle at all directive with several arguments
    pub fn develop(&self, args: &[MacroParam]) -> String {
        assert_eq!(args.len(), self.content.len());

        dbg!(args);
        dbg!(&self.content);

        let mut developped = self
            .content
            .iter()
            .zip(args.iter())
            .enumerate()
            .map(|(_idx, ((_name, token), current_param))| {
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
                            (MacroParam::Single(_), _nb_default) => {
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
            .join("\n");

        let last = developped.pop().unwrap();
        developped.push(last);
        if last != 'n' {
            developped.push('\n');
        }
        developped
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
        //        assert_eq!(args.len(), self.nb_args());

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
    Number(ExprResult),
    /// Address (use in physical way to ensure all bank/page info are available)
    Address(PhysicalAddress),
    /// Macro information
    Macro(Macro),
    /// Structure information
    Struct(Struct),
    /// Counter for a repetition
    Counter(i32),
}

#[derive(Copy, Clone)]
pub enum SymbolFor {
    Number,
    Address,
    Macro,
    Struct,
    Counter,
    Any,
}

impl Value {
    pub fn integer(&self) -> Option<i32> {
        match self {
            Value::Number(ExprResult::Value(i)) => Some(*i),
            Value::Address(addr) => Some(addr.address as _),
            _ => None,
        }
    }

    pub fn address(&self) -> Option<&PhysicalAddress> {
        match self {
            Value::Address(addr) => Some(addr),
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

impl From<PhysicalAddress> for Value {
    fn from(a: PhysicalAddress) -> Self {
        Self::Address(a)
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

impl<I: Into<ExprResult>> From<I> for Value {
    fn from(i: I) -> Self {
        Self::Number(i.into())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct Symbol(String);

impl Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.0)
    }
}
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

/// Public signature of symbols functions
/// TODO add all the other methods
pub trait SymbolsTableTrait {
    /// Return the symbols that correspond to integer values
    fn integer_symbols(&self) -> Vec<&Symbol>;

    /// Return true if the symbol has already been used in an expression
    fn is_used<S: Into<Symbol>>(&self, symbol: S) -> bool;
    /// Add a symbol to the list of used symbols
    fn use_symbol<S: Into<Symbol>>(&mut self, symbol: S);

    /// Return the integer value corredponding to this symbol (if any)
    fn int_value<S: Into<Symbol>>(&self, symbol: S) -> Result<Option<i32>, SymbolError>;

    fn value<S: Into<Symbol>>(&self, symbol: S) -> Result<Option<&Value>, SymbolError>;
    fn counter_value<S: Into<Symbol>>(&self, symbol: S) -> Result<Option<i32>, SymbolError>;
    fn macro_value<S: Into<Symbol>>(&self, symbol: S) -> Result<Option<&Macro>, SymbolError>;
    fn struct_value<S: Into<Symbol>>(&self, symbol: S) -> Result<Option<&Struct>, SymbolError>;
    fn address_value<S: Into<Symbol>>(
        &self,
        symbol: S,
    ) -> Result<Option<&PhysicalAddress>, SymbolError>;

    fn remove_symbol<S: Into<Symbol>>(&mut self, symbol: S) -> Result<Option<Value>, SymbolError>;

    fn assign_symbol_to_value<S: Into<Symbol>, V: Into<Value>>(
        &mut self,
        symbol: S,
        value: V,
    ) -> Result<Option<Value>, SymbolError>;

    fn enter_namespace(&mut self, namespace: &str);
    fn leave_namespace(&mut self) -> Result<Symbol, SymbolError>;
}

/// Handle Tree like maps.
#[derive(Debug, Clone, Default)]
struct ModuleSymbolTable {
    current: HashMap<Symbol, Value>,
    children: HashMap<Symbol, ModuleSymbolTable>,
}

impl Deref for ModuleSymbolTable {
    type Target = HashMap<Symbol, Value>;
    fn deref(&self) -> &Self::Target {
        &self.current
    }
}

impl DerefMut for ModuleSymbolTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.current
    }
}

impl ModuleSymbolTable {
    /// Add a new branch in the module tree
    fn add_children(&mut self, new: Symbol) {
        self.children.insert(new, ModuleSymbolTable::default());
    }

    /// Check if the current module has this children
    fn has_children(&self, children: &Symbol) -> bool {
        self.children.contains_key(children)
    }

    fn children(&self, children: &Symbol) -> Option<&ModuleSymbolTable> {
        self.children.get(children)
    }

    fn children_mut(&mut self, children: &Symbol) -> Option<&mut ModuleSymbolTable> {
        self.children.get_mut(children)
    }

    fn iter(&self) -> ModuleSymbolTableIterator {
        ModuleSymbolTableIterator::new(self)
    }
}

struct ModuleSymbolTableIterator<'t> {
    others: Vec<&'t ModuleSymbolTable>,
    current: std::collections::hash_map::Iter<'t, Symbol, Value>,
}

impl<'t> ModuleSymbolTableIterator<'t> {
    fn new(table: &'t ModuleSymbolTable) -> Self {
        Self {
            others: table.children.values().collect_vec(),
            current: table.current.iter(),
        }
    }
}
impl<'t> Iterator for ModuleSymbolTableIterator<'t> {
    type Item = (&'t Symbol, &'t Value);

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current.next();
        if current.is_some() {
            return current;
        } else {
            if let Some(next) = self.others.pop() {
                let current = next.current.iter();
                self.others.extend(next.children.values());
                self.current = current;
                return self.current.next();
            } else {
                return None;
            }
        }
    }
}

#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub struct SymbolsTable {
    // Tree of symbols. The default one is the root
    map: ModuleSymbolTable,

    dummy: bool,
    current_global_label: Symbol, //  Value of the current label to allow local labels
    // Stack of namespaces
    namespace_stack: Vec<Symbol>,

    // list of symbols that are assignable (i.e. modified programmatically)
    assignable: HashSet<Symbol>,
    seed_stack: Vec<usize>, // stack of seeds for nested repeat to properly interpret the @ symbol

    /// Contains all the symbols that have been used in expressions
    used_symbols: HashSet<Symbol>,
}

impl Default for SymbolsTable {
    fn default() -> Self {
        let mut map = ModuleSymbolTable::default();
        map.add_children("".to_owned().into());
        Self {
            map,
            dummy: false,
            current_global_label: "".into(),
            assignable: Default::default(),
            seed_stack: Vec::new(),
            namespace_stack: Vec::new(),
            used_symbols: HashSet::new(),
        }
    }
}

/// Local/global label handling code
impl SymbolsTable {
    /// Setup the current label for local to global labels conversions
    pub fn set_current_global_label<S: Into<Symbol>>(
        &mut self,
        symbol: S,
    ) -> Result<(), SymbolError> {
        let label = symbol.into();

        if !label.value().starts_with(".") && !label.value().starts_with("@") {
            if label.value().contains(".") {
                return Err(SymbolError::WrongSymbol(label));
            }
            self.current_global_label = self.extend_local_and_patterns_for_symbol(label)?;
        }

        Ok(())
    }

    /// Some symbols are local and need to be converted to their global value.
    fn extend_local_and_patterns_for_symbol<S: Into<Symbol>>(
        &self,
        symbol: S,
    ) -> Result<Symbol, SymbolError> {
        let symbol = symbol.into();
        let mut symbol = symbol.value().to_owned();

        // Local symbols are expensed with their global symbol
        if symbol.starts_with('.') {
            symbol = self.current_global_label.clone().value().to_owned() + &symbol;
        }

        // handle the hidden labels from repeats
        if symbol.starts_with("@") {
            match self.seed_stack.last() {
                Some(seed) => {
                    // we need to rewrite the symbol name to make it unique
                    symbol = format!(".__hidden__{}__{}", seed, &symbol[1..]);
                }
                None => {
                    // we cannot have a symbol with @ here
                    return Err(SymbolError::WrongSymbol(symbol.into()));
                }
            }
        }

        // handle the labels build with patterns
        // Get the replacement strings
        lazy_static::lazy_static! {
            static ref RE: Regex = Regex::new("\\{[^\\}]+\\}").unwrap();
        }
        let mut replace = HashSet::new();
        for cap in RE.captures_iter(&symbol) {
            if cap[0] != symbol {
                replace.insert(cap[0].to_owned());
            }
        }
        // make the replacement
        for model in replace.iter() {
            let local_symbol = &model[1..model.len() - 1]; // remove {}
            let local_value = self.int_value(local_symbol)?.unwrap();
            symbol = symbol.replace(model, &local_value.to_string());
        }

        Ok(symbol.into())
    }
}

/// Module handling code
impl SymbolsTable {
    /// Retrieve the map for the currently selected module
    fn current_module_map(&self) -> &ModuleSymbolTable {
        if self.namespace_stack.is_empty() {
            &self.map
        } else {
            self.module_map(&self.namespace_stack)
        }
    }

    /// Retrieve the mutable map for the currently selected module
    fn current_module_map_mut(&mut self) -> &mut ModuleSymbolTable {
        if self.namespace_stack.is_empty() {
            &mut self.map
        } else {
            let stack = self.namespace_stack.clone();
            self.module_map_mut(&stack)
        }
    }

    /// Retreive the map for the requested module
    fn module_map(&self, namespace: &[Symbol]) -> &ModuleSymbolTable {
        let mut current_map = &self.map;
        for current_namespace in namespace.iter() {
            current_map = current_map.children(current_namespace).unwrap();
        }
        current_map
    }

    fn module_map_mut(&mut self, namespace: &[Symbol]) -> &mut ModuleSymbolTable {
        let mut current_map = &mut self.map;
        for current_namespace in namespace.iter() {
            current_map = current_map.children_mut(current_namespace).unwrap();
        }
        current_map
    }

    /// Split the namespaces of the symbol
    fn split_namespaces(symbol: &Symbol) -> Vec<Symbol> {
        symbol
            .value()
            .split(":")
            .map(|s| s.to_owned())
            .map(|s| s.into())
            .collect_vec()
    }
}

impl SymbolsTableTrait for SymbolsTable {
    fn integer_symbols(&self) -> Vec<&Symbol> {
        self.map
            .iter()
            .filter(|(_k, v)| match v {
                Value::Number(_) | Value::Address(_) => true,
                _ => false,
            })
            .map(|(k, _v)| k)
            .collect_vec()
    }

    fn int_value<S: Into<Symbol>>(&self, symbol: S) -> Result<Option<i32>, SymbolError> {
        Ok(self
            .value(symbol)?
            .map(|v| v.integer())
            .map(|v| v.unwrap())
            .or_else(|| if self.dummy { Some(1i32) } else { None }))
    }

    fn assign_symbol_to_value<S: Into<Symbol>, V: Into<Value>>(
        &mut self,
        symbol: S,
        value: V,
    ) -> Result<Option<Value>, SymbolError> {
        let symbol = self.extend_readable_symbol(symbol)?;

        if !self.assignable.contains(&symbol) && self.map.contains_key(&symbol) {
            return Err(SymbolError::CannotModify(symbol));
        }

        self.assignable.insert(symbol.clone());

        Ok(self.map.insert(symbol, value.into()))
    }

    fn enter_namespace(&mut self, namespace: &str) {
        self.namespace_stack.push(namespace.into())
    }

    fn leave_namespace(&mut self) -> Result<Symbol, SymbolError> {
        match self.namespace_stack.pop() {
            Some(s) => Ok(s),
            None => Err(SymbolError::NoNamespaceActive),
        }
    }

    /// Returns the Value at the given key
    fn value<S: Into<Symbol>>(&self, symbol: S) -> Result<Option<&Value>, SymbolError> {
        let symbol = self.extend_readable_symbol(symbol)?;
        Ok(self.map.get(&symbol))
    }

    fn counter_value<S: Into<Symbol>>(&self, symbol: S) -> Result<Option<i32>, SymbolError> {
        Ok(self
            .value(symbol.into())?
            .map(|v| v.counter())
            .map(|v| v.unwrap())
            .or_else(|| if self.dummy { Some(1i32) } else { None }))
    }

    fn macro_value<S: Into<Symbol>>(&self, symbol: S) -> Result<Option<&Macro>, SymbolError> {
        Ok(self.value(symbol)?.map(|v| v.r#macro()).unwrap_or(None))
    }
    fn struct_value<S: Into<Symbol>>(&self, symbol: S) -> Result<Option<&Struct>, SymbolError> {
        Ok(self.value(symbol)?.map(|v| v.r#struct()).unwrap_or(None))
    }
    fn address_value<S: Into<Symbol>>(
        &self,
        symbol: S,
    ) -> Result<Option<&PhysicalAddress>, SymbolError> {
        Ok(self.value(symbol)?.map(|v| v.address()).unwrap_or(None))
    }

    /// Remove the given symbol name from the table. (used by undef)
    fn remove_symbol<S: Into<Symbol>>(&mut self, symbol: S) -> Result<Option<Value>, SymbolError> {
        let symbol = self.extend_readable_symbol(symbol)?;
        Ok(self.map.remove(&symbol))
    }

    fn is_used<S: Into<Symbol>>(&self, symbol: S) -> bool {
        let symbol = self.extend_readable_symbol(symbol).unwrap();
        self.used_symbols.contains(&symbol)
    }

    fn use_symbol<S: Into<Symbol>>(&mut self, symbol: S) {
        let symbol = self.extend_readable_symbol(symbol).unwrap();
        self.used_symbols.insert(symbol);
    }
}

#[allow(missing_docs)]
impl SymbolsTable {
    pub fn laxist() -> Self {
        let mut map = ModuleSymbolTable::default();
        map.insert(Symbol::from("$"), Value::Number(0.into()));
        Self {
            map,
            dummy: true,
            current_global_label: "".into(),
            assignable: HashSet::new(),
            seed_stack: Vec::new(),
            namespace_stack: Vec::new(),
            used_symbols: HashSet::new(),
        }
    }

    /// Add a new seed for the @ symbol name resolution (we enter in a repeat)
    pub fn push_seed(&mut self, seed: usize) {
        self.seed_stack.push(seed)
    }

    /// Remove the previous seed for the @ symbol name resolution (<e leave a repeat)
    pub fn pop_seed(&mut self) {
        self.seed_stack.pop();
    }

    /// Symbol is either :
    /// - a global symbol from the current module
    /// - or a fully qualified that represents a module from the start
    pub fn get_potential_candidates(&self, symbol: Symbol) -> SmallVec<[Symbol; 2]> {
        if symbol.value().starts_with("::") {
            smallvec![symbol.value()[2..].to_owned().into()]
        } else if self.namespace_stack.is_empty() {
            smallvec![symbol]
        } else {
            let full = symbol.clone();

            let _global = self.namespace_stack.clone();
            let global = self.inject_current_namespace(symbol);

            smallvec![global, full]
        }
    }

    fn inject_current_namespace<S: Into<Symbol>>(&self, symbol: S) -> Symbol {
        let mut global = self.namespace_stack.clone();
        global.push(symbol.into());
        global.iter().join(".").into()
    }

    fn extend_readable_symbol<S: Into<Symbol>>(&self, symbol: S) -> Result<Symbol, SymbolError> {
        let symbol = self.extend_local_and_patterns_for_symbol(symbol)?;
        let candidates = self.get_potential_candidates(symbol);

        if candidates.len() == 1 {
            Ok(candidates[0].clone())
        } else {
            if self.map.contains_key(&candidates[0]) {
                Ok(candidates[0].clone())
            } else {
                Ok(candidates[1].clone())
            }
        }
    }

    fn extend_writable_symbol<S: Into<Symbol>>(&self, symbol: S) -> Result<Symbol, SymbolError> {
        let symbol = self.extend_local_and_patterns_for_symbol(symbol)?;
        let candidates = self.get_potential_candidates(symbol);

        Ok(candidates[0].clone())
    }

    /// Return the current addres if it is known or return an error
    pub fn current_address(&self) -> Result<u16, SymbolError> {
        match self.value("$")? {
            Some(address) => Ok(address.integer().unwrap() as u16),
            None => Err(SymbolError::UnknownAssemblingAddress),
        }
    }

    /// Update `$` value
    pub fn set_current_address(&mut self, address: PhysicalAddress) {
        self.map.insert("$".into(), Value::Address(address));
    }
    pub fn set_current_output_address(&mut self, address: PhysicalAddress) {
        self.map.insert("$$".into(), Value::Address(address));
    }

    /// Set the given symbol to $ value
    pub fn set_symbol_to_current_address<S: Into<Symbol>>(
        &mut self,
        symbol: S,
    ) -> Result<(), SymbolError> {
        let symbol = self.extend_local_and_patterns_for_symbol(symbol)?;
        let symbol = self.extend_readable_symbol(symbol)?;
        self.current_address().map(|val| {
            self.map.insert(symbol, Value::Number(val.into()));
        })
    }

    /// Set the given Value to the given value
    /// Return the previous value if any
    pub fn set_symbol_to_value<S: Into<Symbol>, V: Into<Value>>(
        &mut self,
        symbol: S,
        value: V,
    ) -> Result<Option<Value>, SymbolError> {
        let symbol = self.extend_local_and_patterns_for_symbol(symbol)?;
        let symbol = self.inject_current_namespace(symbol);

        Ok(self.map.insert(symbol, value.into()))
    }

    pub fn update_symbol_to_value<S: Into<Symbol>, V: Into<Value>>(
        &mut self,
        symbol: S,
        value: V,
    ) -> Result<(), SymbolError> {
        let symbol = self.extend_readable_symbol(symbol)?;
        let symbols = self.get_potential_candidates(symbol);
        let symbol = symbols
            .iter()
            .find(|symbol| self.map.contains_key(symbol))
            .unwrap();

        *(self.map.get_mut(&symbol).unwrap()) = value.into();
        Ok(())
    }

    /// Instead of returning the value, return the bank information
    /// logic stolen to rasm
    pub fn prefixed_value<S: Into<Symbol>>(
        &self,
        prefix: &LabelPrefix,
        key: S,
    ) -> Result<Option<u16>, SymbolError> {
        let key = key.into();
        let addr = self.address_value(key.clone())?;
        Ok(addr.map(|v| {
            match prefix {
                LabelPrefix::Bank => v.bank() as u16,
                LabelPrefix::Page => v.ga_bank() & 0x00ff,
                LabelPrefix::Pageset => v.ga_page() & 0x00ff, // delete 0x7f00
            }
        } as _))

        /*
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
                */
    }

    pub fn contains_symbol<S: Into<Symbol>>(&self, symbol: S) -> Result<bool, SymbolError> {
        let symbol = self.extend_local_and_patterns_for_symbol(symbol)?;
        let symbols = self.get_potential_candidates(symbol);
        Ok(symbols.iter().any(|symbol| self.map.contains_key(&symbol)))
    }

    /// Returns the closest Value
    pub fn closest_symbol<S: Into<Symbol>>(
        &self,
        symbol: S,
        r#for: SymbolFor,
    ) -> Result<Option<String>, SymbolError> {
        let symbol = self.extend_local_and_patterns_for_symbol(symbol)?;
        let symbol = self.extend_readable_symbol(symbol)?;
        Ok(self
            .map
            .iter()
            .filter(|(_k, v)| match (v, r#for) {
                (Value::Number(_), SymbolFor::Number)
                | (Value::Number(_), SymbolFor::Address)
                | (Value::Address(_), SymbolFor::Address)
                | (Value::Address(_), SymbolFor::Number)
                | (Value::Macro(_), SymbolFor::Macro)
                | (Value::Struct(_), SymbolFor::Struct)
                | (Value::Counter(_), SymbolFor::Counter)
                | (_, SymbolFor::Any) => true,
                _ => false,
            })
            .map(|(k, _v)| k)
            .map(move |symbol2| {
                (
                    strsim::levenshtein(&symbol2.0, &symbol.0).min(strsim::levenshtein(
                        &symbol2.0.to_ascii_uppercase(),
                        &symbol.0.to_ascii_uppercase(),
                    )),
                    symbol2.0.clone(),
                )
            })
            .min()
            .map(|(_distance, symbol2)| symbol2))
    }

    pub fn kind<S: Into<Symbol>>(&self, symbol: S) -> Result<&'static str, SymbolError> {
        Ok(match self.value(symbol)? {
            Some(Value::Number(_)) => "number",
            Some(Value::Address(_)) => "address",
            Some(Value::Macro(_)) => "macro",
            Some(Value::Struct(_)) => "struct",
            Some(Value::Counter(_)) => "counter",
            None => "any",
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
        self.table
            .set_current_global_label(self.normalize_symbol(symbol))
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

    pub fn update_symbol_to_value<S: Into<Symbol>, E: Into<Value>>(
        &mut self,
        symbol: S,
        value: E,
    ) -> Result<(), SymbolError> {
        self.table
            .update_symbol_to_value(self.normalize_symbol(symbol), value.into())
    }

    pub fn prefixed_value<S: Into<Symbol>>(
        &self,
        prefix: &LabelPrefix,
        symbol: S,
    ) -> Result<Option<u16>, SymbolError> {
        self.table
            .prefixed_value(prefix, self.normalize_symbol(symbol))
    }

    pub fn contains_symbol<S: Into<Symbol>>(&self, symbol: S) -> Result<bool, SymbolError> {
        self.table.contains_symbol(self.normalize_symbol(symbol))
    }

    pub fn kind<S: Into<Symbol>>(&self, symbol: S) -> Result<&'static str, SymbolError> {
        self.table.kind(symbol)
    }
    delegate! {
        to self.table {
            pub fn current_address(&self) -> Result<u16, SymbolError>;
            pub fn set_current_address(&mut self, addr: PhysicalAddress);
            pub fn set_current_output_address(&mut self, addr: PhysicalAddress);
            pub fn closest_symbol<S: Into<Symbol>>(&self, symbol: S, r#for: SymbolFor) -> Result<Option<String>, SymbolError>;
            pub fn push_seed(&mut self, seed: usize);
            pub fn pop_seed(&mut self);
        }
    }
}

impl SymbolsTableTrait for SymbolsTableCaseDependent {
    fn is_used<S: Into<Symbol>>(&self, symbol: S) -> bool {
        self.table.is_used(self.normalize_symbol(symbol))
    }

    fn use_symbol<S: Into<Symbol>>(&mut self, symbol: S) {
        self.table.use_symbol(self.normalize_symbol(symbol))
    }

    fn integer_symbols(&self) -> Vec<&Symbol> {
        self.table.integer_symbols()
    }

    fn int_value<S: Into<Symbol>>(&self, symbol: S) -> Result<Option<i32>, SymbolError> {
        self.table.int_value(self.normalize_symbol(symbol))
    }

    fn counter_value<S: Into<Symbol>>(&self, symbol: S) -> Result<Option<i32>, SymbolError> {
        self.table.counter_value(self.normalize_symbol(symbol))
    }

    fn macro_value<S: Into<Symbol>>(&self, symbol: S) -> Result<Option<&Macro>, SymbolError> {
        self.table.macro_value(self.normalize_symbol(symbol))
    }
    fn struct_value<S: Into<Symbol>>(&self, symbol: S) -> Result<Option<&Struct>, SymbolError> {
        self.table.struct_value(self.normalize_symbol(symbol))
    }

    fn value<S: Into<Symbol>>(&self, symbol: S) -> Result<Option<&Value>, SymbolError> {
        self.table.value(self.normalize_symbol(symbol))
    }

    fn remove_symbol<S: Into<Symbol>>(&mut self, symbol: S) -> Result<Option<Value>, SymbolError> {
        self.table.remove_symbol(self.normalize_symbol(symbol))
    }

    fn address_value<S: Into<Symbol>>(
        &self,
        symbol: S,
    ) -> Result<Option<&PhysicalAddress>, SymbolError> {
        self.table.address_value(self.normalize_symbol(symbol))
    }

    fn assign_symbol_to_value<S: Into<Symbol>, V: Into<Value>>(
        &mut self,
        symbol: S,
        value: V,
    ) -> Result<Option<Value>, SymbolError> {
        self.table
            .assign_symbol_to_value(self.normalize_symbol(symbol), value)
    }

    fn enter_namespace(&mut self, namespace: &str) {
        self.table
            .enter_namespace(self.normalize_symbol(namespace).value())
    }

    fn leave_namespace(&mut self) -> Result<Symbol, SymbolError> {
        self.table.leave_namespace()
    }
}
