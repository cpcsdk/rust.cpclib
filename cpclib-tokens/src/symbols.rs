use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display};
use std::ops::{Deref, DerefMut};
use std::sync::LazyLock;

use cpclib_common::itertools::Itertools;
#[cfg(all(not(target_arch = "wasm32"), feature = "rayon"))]
use cpclib_common::rayon::{iter::IntoParallelRefIterator, iter::ParallelIterator};
use cpclib_common::smallvec::{smallvec, SmallVec};
use cpclib_common::smol_str::SmolStr;
use cpclib_common::strsim;
use delegate::delegate;
use evalexpr::{build_operator_tree, ContextWithMutableVariables, HashMapContext};
use regex::Regex;

use crate::tokens::expression::LabelPrefix;
use crate::{expression, AssemblerFlavor, ExprResult, ListingElement, ToSimpleToken, Token};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalAddress {
    Memory(MemoryPhysicalAddress),
    Bank(BankPhysicalAddress),
    Cpr(CprPhysicalAddress)
}

impl From<u16> for PhysicalAddress {
    fn from(value: u16) -> Self {
        Self::Memory(MemoryPhysicalAddress::new(value, 0xC0))
    }
}
impl Display for PhysicalAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PhysicalAddress::Memory(address) => {
                write!(
                    f,
                    "0x{:X} (0x{:X} in page {})",
                    address.address(),
                    address.offset_in_page(),
                    address.page(),
                )
            },
            PhysicalAddress::Cpr(address) => {
                write!(
                    f,
                    "0x{:X} in Cartridge bloc {}",
                    address.address(),
                    address.bloc()
                )
            },
            PhysicalAddress::Bank(address) => {
                write!(f, "0x{:X} in bank {}", address.address(), address.bank())
            }
        }
    }
}

impl PhysicalAddress {
    #[inline(always)]
    pub fn address(&self) -> u16 {
        match self {
            PhysicalAddress::Memory(adr) => adr.address(),
            PhysicalAddress::Bank(adr) => adr.address(),
            PhysicalAddress::Cpr(adr) => adr.address()
        }
    }

    /// not really coherent to use that with cpr and bank
    #[inline(always)]
    pub fn offset_in_cpc(&self) -> u32 {
        match self {
            PhysicalAddress::Memory(adr) => adr.offset_in_cpc(),
            PhysicalAddress::Bank(adr) => adr.address() as _,
            PhysicalAddress::Cpr(adr) => adr.address() as _
        }
    }

    #[inline(always)]
    pub fn to_memory(self) -> MemoryPhysicalAddress {
        match self {
            PhysicalAddress::Memory(adr) => adr,
            _ => panic!()
        }
    }

    #[inline(always)]
    pub fn to_bank(self) -> BankPhysicalAddress {
        match self {
            PhysicalAddress::Bank(adr) => adr,
            _ => panic!()
        }
    }

    #[inline(always)]
    pub fn to_cpr(self) -> CprPhysicalAddress {
        match self {
            PhysicalAddress::Cpr(adr) => adr,
            _ => panic!()
        }
    }

    pub fn remu_bank(&self) -> u16 {
        match self {
            PhysicalAddress::Memory(m) => (4 * m.page as u16 + (m.address / 0x4000)) as _,
            PhysicalAddress::Bank(b) => b.bank() as _,
            PhysicalAddress::Cpr(c) => c.bloc() as _
        }
    }
}

impl From<MemoryPhysicalAddress> for PhysicalAddress {
    #[inline(always)]
    fn from(value: MemoryPhysicalAddress) -> Self {
        Self::Memory(value)
    }
}

impl From<BankPhysicalAddress> for PhysicalAddress {
    #[inline(always)]
    fn from(value: BankPhysicalAddress) -> Self {
        Self::Bank(value)
    }
}

impl From<CprPhysicalAddress> for PhysicalAddress {
    #[inline(always)]
    fn from(value: CprPhysicalAddress) -> Self {
        Self::Cpr(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CprPhysicalAddress {
    bloc: u8,
    address: u16
}

impl CprPhysicalAddress {
    #[inline]
    pub fn new(address: u16, bloc: u8) -> Self {
        Self { bloc, address }
    }

    #[inline]
    pub fn address(&self) -> u16 {
        self.address
    }

    #[inline]
    pub fn bloc(&self) -> u8 {
        self.bloc
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BankPhysicalAddress {
    bank: usize,
    address: u16
}

impl BankPhysicalAddress {
    #[inline]
    pub fn new(address: u16, bank: usize) -> Self {
        Self { bank, address }
    }

    #[inline]
    pub fn address(&self) -> u16 {
        self.address
    }

    #[inline]
    pub fn bank(&self) -> usize {
        self.bank
    }
}

/// Structure that ease the addresses manipulation to read/write at the right place
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryPhysicalAddress {
    /// Page number (0 for base, 1 for first page, 2 ...)
    page: u8,
    /// Bank number in the page: 0 to 3
    bank: u8,
    /// Address manipulate by CPU 0x0000 to 0xffff
    address: u16
}

impl From<u16> for MemoryPhysicalAddress {
    fn from(nb: u16) -> Self {
        MemoryPhysicalAddress::new(nb, 0xC0)
    }
}

impl MemoryPhysicalAddress {
    pub fn new(address: u16, mmr: u8) -> Self {
        if mmr == 0xC1 {
            return MemoryPhysicalAddress {
                page: 1,
                bank: (address / 0x4000) as u8,
                address: address % 0x4000
            };
        }

        let possible_page = ((mmr >> 3) & 0b111) + 1;
        let possible_bank = mmr & 0b11;
        let standard_bank = match address {
            0x0000..0x4000 => 0,
            0x4000..0x8000 => 1,
            0x8000..0xC000 => 2,
            0xC000.. => 3
        };
        let is_4000 = (0x4000..0x8000).contains(&address);
        let is_c000 = address >= 0xC000;

        let (page, bank) = if (mmr & 0b100) != 0 {
            if is_4000 {
                (possible_page, possible_bank)
            }
            else {
                (0, possible_bank)
            }
        }
        else {
            match mmr & 0b11 {
                0b000 => (0, standard_bank),
                0b001 => {
                    if is_c000 {
                        (possible_page, standard_bank)
                    }
                    else {
                        (0, standard_bank)
                    }
                },
                0b010 => (possible_page, standard_bank),
                0b011 => {
                    if is_4000 {
                        (0, 3)
                    }
                    else if is_c000 {
                        (possible_page, 3)
                    }
                    else {
                        (0, standard_bank)
                    }
                },
                _ => unreachable!()
            }
        };

        Self {
            address,
            bank,
            page
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
            0b1100_0000
        }
        else {
            0b1100_0100 + ((self.page() - 1) << 3) + self.bank
        } as u16;
        low + 0x7F00
    }

    pub fn ga_page(&self) -> u16 {
        let low = if self.page() == 0 {
            0b1100_0000
        }
        else {
            0b1100_0010 + ((self.page() - 1) << 3)
        } as u16;
        low + 0x7F00
    }
}

#[derive(Debug, Clone)]
pub enum SymbolError {
    UnknownAssemblingAddress,
    CannotModify(Symbol),
    WrongSymbol(Symbol),
    NoNamespaceActive
}

/// Encode the data for the structure directive
#[derive(Debug, Clone)]
pub struct Struct {
    name: SmolStr,
    content: Vec<(SmolStr, Token)>,
    source: Option<Source>
}

impl Struct {
    pub fn new<T: ListingElement + ToSimpleToken, S: AsRef<str>>(
        name: impl AsRef<str>,
        content: &[(S, T)],
        source: Option<Source>
    ) -> Self {
        Self {
            name: name.as_ref().into(),
            content: content
                .iter()
                .map(|(s, t)| (SmolStr::from(s.as_ref()), t.as_simple_token().into_owned()))
                .collect_vec(),
            source
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn source(&self) -> Option<&Source> {
        self.source.as_ref()
    }

    pub fn content(&self) -> &[(SmolStr, Token)] {
        self.content.as_ref()
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
                let s = table.struct_value(n).ok().unwrap().unwrap(); // TODO handle error here
                s.len(table)
            },
            _ => unreachable!("{:?}", token)
        }
    }

    /// Get the len of the structure
    pub fn len<T: SymbolsTableTrait>(&self, table: &T) -> i32 {
        self.fields_size(table).iter().map(|(_, s)| *s).sum()
    }

    pub fn nb_args(&self) -> usize {
        self.content.len()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Source {
    fname: String,
    line: usize,
    column: usize
}

impl Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", &self.fname, self.line, self.column)
    }
}

impl Source {
    pub fn new(fname: String, line: usize, column: usize) -> Self {
        Source {
            fname,
            line,
            column
        }
    }

    pub fn fname(&self) -> &str {
        &self.fname
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn column(&self) -> usize {
        self.column
    }
}

#[derive(Debug, Clone)]
pub struct Macro {
    // The name of the macro
    name: SmolStr,
    // The name of its arguments
    params: Vec<SmolStr>,
    // The content
    code: String,
    // Origin of the macro (for error messages)
    source: Option<Source>,
    flavor: AssemblerFlavor
}

impl Macro {
    pub fn new(
        name: SmolStr,
        params: &[&str],
        code: String,
        source: Option<Source>,
        flavor: AssemblerFlavor
    ) -> Self {
        Macro {
            name,
            params: params.iter().map(|&s| SmolStr::from(s)).collect(),
            code,
            source,
            flavor
        }
    }

    #[inline]
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    #[inline]
    pub fn source(&self) -> Option<&Source> {
        self.source.as_ref()
    }

    #[inline]
    pub fn code(&self) -> &str {
        self.code.as_ref()
    }

    #[inline]
    pub fn flavor(&self) -> AssemblerFlavor {
        self.flavor
    }

    #[inline]
    pub fn params(&self) -> &[SmolStr] {
        &self.params
    }

    #[inline]
    pub fn nb_args(&self) -> usize {
        self.params.len()
    }
}

#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub enum Value {
    /// Integer value used in an expression
    Expr(ExprResult),
    String(SmolStr),
    /// Address (use in physical way to ensure all bank/page info are available)
    Address(PhysicalAddress),
    /// Macro information
    Macro(Macro),
    /// Structure information
    Struct(Struct),
    /// Counter for a repetition
    Counter(i32)
}

impl From<Value> for evalexpr::Value {
    fn from(val: Value) -> Self {
        match val {
            Value::Expr(e) => {
                match e {
                    ExprResult::Float(f) => evalexpr::Value::Float(f.into()),
                    ExprResult::Value(v) => evalexpr::Value::Int(v as _),
                    ExprResult::Char(c) => evalexpr::Value::Int(c as _),
                    ExprResult::Bool(b) => evalexpr::Value::Boolean(b),
                    ExprResult::String(s) => evalexpr::Value::String(s.into()),
                    ExprResult::List(_l) => unimplemented!(),
                    ExprResult::Matrix {
                        width: _,
                        height: _,
                        content: _
                    } => unimplemented!()
                }
            },
            Value::String(s) => evalexpr::Value::String(s.into()),
            Value::Address(v) => evalexpr::Value::Int(v.address() as _),
            Value::Macro(m) => evalexpr::Value::String(m.name.into()),
            Value::Struct(s) => evalexpr::Value::String(s.name.into()),
            Value::Counter(c) => evalexpr::Value::Int(c as _)
        }
    }
}

#[derive(Copy, Clone)]
pub enum SymbolFor {
    Number,
    Address,
    Macro,
    Struct,
    Counter,
    Any
}

impl Value {
    pub fn expr(&self) -> Option<&ExprResult> {
        match self {
            Value::Expr(e) => Some(e),
            _ => None
        }
    }

    pub fn is_expr(&self) -> bool {
        match self {
            Value::Expr(_) => true,
            _ => false
        }
    }

    pub fn integer(&self) -> Option<i32> {
        match self {
            Value::Expr(ExprResult::Value(i)) => Some(*i),
            Value::Address(addr) => Some(addr.address() as _),
            _ => None
        }
    }

    pub fn address(&self) -> Option<&PhysicalAddress> {
        match self {
            Value::Address(addr) => Some(addr),
            _ => None
        }
    }

    pub fn counter(&self) -> Option<i32> {
        match self {
            Value::Counter(i) => Some(*i),
            _ => None
        }
    }

    pub fn r#macro(&self) -> Option<&Macro> {
        match self {
            Value::Macro(m) => Some(m),
            _ => None
        }
    }

    pub fn r#struct(&self) -> Option<&Struct> {
        match self {
            Value::Struct(m) => Some(m),
            _ => None
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
        let value = i.into();
        match &value {
            ExprResult::String(s) => Value::String(s.clone()),
            _ => Value::Expr(value)
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct Symbol(SmolStr);

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
        Symbol(s.into())
    }
}

impl From<&String> for Symbol {
    fn from(s: &String) -> Symbol {
        Symbol(s.into())
    }
}

impl From<SmolStr> for Symbol {
    fn from(s: SmolStr) -> Symbol {
        Symbol(s)
    }
}

impl From<&SmolStr> for Symbol {
    fn from(s: &SmolStr) -> Symbol {
        Symbol(s.clone())
    }
}

impl From<Symbol> for SmolStr {
    fn from(val: Symbol) -> Self {
        val.0
    }
}

impl From<&Symbol> for SmolStr {
    fn from(val: &Symbol) -> Self {
        val.0.clone()
    }
}

impl AsRef<str> for Symbol {
    fn as_ref(&self) -> &str {
        self.value()
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
    fn expression_symbol(&self) -> Vec<(&Symbol, &ValueAndSource)>;

    /// Return true if the symbol has already been used in an expression
    fn is_used<S>(&self, symbol: S) -> bool
    where
        Symbol: From<S>,
        S: AsRef<str>;
    /// Add a symbol to the list of used symbols
    fn use_symbol<S>(&mut self, symbol: S)
    where
        Symbol: From<S>,
        S: AsRef<str>;

    /// Return the integer value corredponding to this symbol (if any)
    fn int_value<S>(&self, symbol: S) -> Result<Option<i32>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>;

    fn value<S>(&self, symbol: S) -> Result<Option<&ValueAndSource>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>;
    fn counter_value<S>(&self, symbol: S) -> Result<Option<i32>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>;
    fn macro_value<S>(&self, symbol: S) -> Result<Option<&Macro>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>;
    fn struct_value<S>(&self, symbol: S) -> Result<Option<&Struct>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>;
    fn address_value<S>(&self, symbol: S) -> Result<Option<&PhysicalAddress>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>;

    fn remove_symbol<S>(&mut self, symbol: S) -> Result<Option<ValueAndSource>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>;

    fn assign_symbol_to_value<S, V: Into<ValueAndSource>>(
        &mut self,
        symbol: S,
        value: V
    ) -> Result<Option<ValueAndSource>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>;

    fn enter_namespace(&mut self, namespace: &str);
    fn leave_namespace(&mut self) -> Result<Symbol, SymbolError>;
}

#[derive(Clone, Debug)]
pub struct ValueAndSource {
    value: Value,
    location: Option<Source>
}

impl From<ValueAndSource> for Value {
    fn from(val: ValueAndSource) -> Self {
        val.value
    }
}

impl From<ValueAndSource> for evalexpr::Value {
    fn from(val: ValueAndSource) -> Self {
        val.value.into()
    }
}

impl From<expression::ExprResult> for ValueAndSource {
    fn from(value: expression::ExprResult) -> Self {
        let value: Value = value.into();
        value.into()
    }
}

impl From<ValueAndSource> for Option<Source> {
    fn from(val: ValueAndSource) -> Self {
        val.location
    }
}

impl ValueAndSource {
    pub fn new<V: Into<Value>, L: Into<Source>>(value: V, location: Option<L>) -> Self {
        let value = value.into();
        let location = location.map(|l| l.into());
        Self { location, value }
    }

    pub fn new_unlocated<V: Into<Value>>(value: V) -> Self {
        Self {
            location: None,
            value: value.into()
        }
    }

    pub fn location(&self) -> Option<&Source> {
        self.location.as_ref()
    }

    pub fn is_located(&self) -> bool {
        self.location.is_some()
    }

    pub fn value(&self) -> &Value {
        &self.value
    }
}

impl From<Value> for ValueAndSource {
    fn from(val: Value) -> Self {
        ValueAndSource {
            value: val,
            location: None
        }
    }
}

impl Deref for ValueAndSource {
    type Target = Value;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
/// Handle Tree like maps.
#[derive(Debug, Clone, Default)]
struct ModuleSymbolTable {
    current: HashMap<Symbol, ValueAndSource>,
    children: HashMap<Symbol, ModuleSymbolTable>
}

impl Deref for ModuleSymbolTable {
    type Target = HashMap<Symbol, ValueAndSource>;

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
    current: std::collections::hash_map::Iter<'t, Symbol, ValueAndSource>
}

impl<'t> ModuleSymbolTableIterator<'t> {
    fn new(table: &'t ModuleSymbolTable) -> Self {
        Self {
            others: table.children.values().collect_vec(),
            current: table.current.iter()
        }
    }
}
impl<'t> Iterator for ModuleSymbolTableIterator<'t> {
    type Item = (&'t Symbol, &'t ValueAndSource);

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current.next();
        if current.is_some() {
            current
        }
        else if let Some(next) = self.others.pop() {
            let current = next.current.iter();
            self.others.extend(next.children.values());
            self.current = current;
            self.current.next()
        }
        else {
            None
        }
    }
}

#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub struct SymbolsTable {
    /// Tree of symbols. The default one is the root. build and maintained all over assembling
    map: ModuleSymbolTable,

    /// A kind of clone of map that contains only the information of the current pass
    current_pass_map: ModuleSymbolTable,

    dummy: bool,
    current_global_label: Symbol, //  Value of the current label to allow local labels
    // Stack of namespaces
    namespace_stack: Vec<Symbol>,

    // list of symbols that are assignable (i.e. modified programmatically)
    assignable: HashSet<Symbol>,
    seed_stack: Vec<usize>, // stack of seeds for nested repeat to properly interpret the @ symbol

    /// Contains all the symbols that have been used in expressions
    used_symbols: HashSet<Symbol>,

    counters: Vec<ExprResult>
}

impl Default for SymbolsTable {
    fn default() -> Self {
        let mut map = ModuleSymbolTable::default();
        map.add_children("".to_owned().into());
        Self {
            map: map.clone(),
            current_pass_map: map.clone(),
            dummy: false,
            current_global_label: "".into(),
            assignable: Default::default(),
            seed_stack: Vec::new(),
            namespace_stack: Vec::new(),
            used_symbols: HashSet::new(),
            counters: Default::default()
        }
    }
}

/// Local/global label handling code
impl SymbolsTable {
    pub fn new_pass(&mut self) {
        self.current_pass_map = ModuleSymbolTable::default();
        self.current_pass_map.add_children("".to_owned().into());
    }

    pub fn used_symbols(&self) -> impl Iterator<Item=&Symbol> {
        self.used_symbols.iter()
    }

    pub fn available_symbols(&self) -> impl Iterator<Item=&Symbol> {
        self.map.keys()
    }

    /// Setup the current label for local to global labels conversions
    #[inline]
    pub fn set_current_global_label<S>(&mut self, symbol: S) -> Result<(), SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        let label = Symbol::from(symbol);

        if !label.value().starts_with('.') && !label.value().starts_with('@') {
            if label.value().contains('.') {
                return Err(SymbolError::WrongSymbol(label));
            }
            self.current_global_label =
                self.extend_local_and_patterns_for_symbol::<Symbol>(label)?;
        }

        Ok(())
    }

    #[inline]
    pub fn get_current_label(&self) -> &Symbol {
        &self.current_global_label
    }

    /// Some symbols are local and need to be converted to their global value.
    /// Some have expressions that need to be expended
    #[inline]
    pub fn extend_local_and_patterns_for_symbol<S>(&self, symbol: S) -> Result<Symbol, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        let symbol: Symbol = symbol.into();
        let mut symbol = symbol.value().to_owned();

        // handle the labels build with patterns
        // Get the replacement strings
        static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{+[^\}]+\}+").unwrap());
        let mut replace = HashSet::new();
        for cap in RE.captures_iter(&symbol) {
            if cap[0] != symbol {
                replace.insert(cap[0].to_owned());
            }
        }

        // make the replacement
        for model in replace.iter() {
            let local_expr = &model[1..model.len() - 1]; // remove {}

            let local_value = match self.value::<&str>(local_expr)?.map(|vl| vl.value()) {
                Some(Value::String(s)) => s.to_string(),
                Some(Value::Expr(e)) => e.to_string(),
                Some(Value::Counter(e)) => e.to_string(),
                _ => {
                    let tree = build_operator_tree(local_expr)
                        .expect("Expression should be valid here. There is a bug in the assembler");

                    // Fill the variable values to allow an evaluation
                    let mut context = HashMapContext::new();
                    for variable in tree.iter_variable_identifiers() {
                        let variable_value = self
                            .value::<&str>(variable)?
                            .ok_or_else(|| { SymbolError::WrongSymbol(variable.into()) })?;
                        context
                            .set_value(variable.to_owned(), variable_value.clone().into())
                            .unwrap();
                    }

                    // evaluate the expression
                    let res = tree
                        .eval_with_context(&context)
                        .map_err(|_e| SymbolError::CannotModify(local_expr.into()))?;

                    res.to_string()
                }
            };
            symbol = symbol.replace(model, &local_value);
        }

        // Local symbols are expensed with their global symbol
        if symbol.starts_with('.') {
            symbol = self.current_global_label.clone().value().to_owned() + &symbol;
        }

        // handle the hidden labels from repeats
        if symbol.starts_with('@') {
            match self.seed_stack.last() {
                Some(seed) => {
                    // we need to rewrite the symbol name to make it unique
                    symbol = format!(".__hidden__{}__{}", seed, &symbol[1..]);
                },
                None => {
                    // we cannot have a symbol with @ here
                    return Err(SymbolError::WrongSymbol(symbol.into()));
                }
            }
        }

        Ok(symbol.into())
    }
}

/// Module handling code
impl SymbolsTable {
    /// Retrieve the map for the currently selected module
    #[inline]
    fn current_module_map(&self) -> &ModuleSymbolTable {
        if self.namespace_stack.is_empty() {
            &self.map
        }
        else {
            self.module_map(&self.namespace_stack)
        }
    }

    /// Retrieve the mutable map for the currently selected module
    #[inline]
    fn current_module_map_mut(&mut self) -> &mut ModuleSymbolTable {
        if self.namespace_stack.is_empty() {
            &mut self.map
        }
        else {
            let stack = self.namespace_stack.clone();
            self.module_map_mut(&stack)
        }
    }

    /// Retreive the map for the requested module
    #[inline]
    fn module_map(&self, namespace: &[Symbol]) -> &ModuleSymbolTable {
        let mut current_map = &self.map;
        for current_namespace in namespace.iter() {
            current_map = current_map.children(current_namespace).unwrap();
        }
        current_map
    }

    #[inline]
    fn module_map_mut(&mut self, namespace: &[Symbol]) -> &mut ModuleSymbolTable {
        let mut current_map = &mut self.map;
        for current_namespace in namespace.iter() {
            current_map = current_map.children_mut(current_namespace).unwrap();
        }
        current_map
    }

    /// Split the namespaces of the symbol
    #[inline]
    fn split_namespaces(symbol: Symbol) -> Vec<Symbol> {
        symbol
            .value()
            .split(':')
            .map(|s| s.to_owned())
            .map(|s| s.into())
            .collect_vec()
    }
}

impl SymbolsTableTrait for SymbolsTable {
    #[inline]
    fn expression_symbol(&self) -> Vec<(&Symbol, &ValueAndSource)> {
        self.map
            .iter()
            .filter(|(_k, v)| {
                match v.value() {
                    Value::Expr(_) | Value::Address(_) => true,
                    _ => false
                }
            })
            .collect_vec()
    }

    #[inline]
    fn int_value<S>(&self, symbol: S) -> Result<Option<i32>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        Ok(self.value(symbol)?.and_then(|v| v.integer()).or({
            if self.dummy {
                Some(1i32)
            }
            else {
                None
            }
        }))
    }

    #[inline]
    fn assign_symbol_to_value<S, V: Into<ValueAndSource>>(
        &mut self,
        symbol: S,
        value: V
    ) -> Result<Option<ValueAndSource>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        let symbol = self.extend_readable_symbol(symbol)?;
        let value = value.into();

        if !self.assignable.contains(&symbol) && self.map.contains_key(&symbol) {
            return Err(SymbolError::CannotModify(symbol));
        }

        self.assignable.insert(symbol.clone());

        self.current_pass_map.insert(symbol.clone(), value.clone());
        Ok(self.map.insert(symbol, value))
    }

    #[inline]
    fn enter_namespace(&mut self, namespace: &str) {
        self.namespace_stack.push(namespace.into())
    }

    #[inline]
    fn leave_namespace(&mut self) -> Result<Symbol, SymbolError> {
        match self.namespace_stack.pop() {
            Some(s) => Ok(s),
            None => Err(SymbolError::NoNamespaceActive)
        }
    }

    /// Returns the Value at the given key
    #[inline]
    fn value<S>(&self, symbol: S) -> Result<Option<&ValueAndSource>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        let symbol = self.extend_readable_symbol(symbol)?;
        Ok(self.map.get(&symbol))
    }

    #[inline]
    fn counter_value<S>(&self, symbol: S) -> Result<Option<i32>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        Ok(self
            .value(symbol)?
            .map(|v| v.counter())
            .map(|v| v.unwrap())
            .or({
                if self.dummy {
                    Some(1i32)
                }
                else {
                    None
                }
            }))
    }

    #[inline]
    fn macro_value<S>(&self, symbol: S) -> Result<Option<&Macro>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        Ok(self.value(symbol)?.map(|v| v.r#macro()).unwrap_or(None))
    }

    #[inline]
    fn struct_value<S>(&self, symbol: S) -> Result<Option<&Struct>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        Ok(self.value(symbol)?.map(|v| v.r#struct()).unwrap_or(None))
    }

    #[inline]
    fn address_value<S>(&self, symbol: S) -> Result<Option<&PhysicalAddress>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        Ok(self.value(symbol)?.map(|v| v.address()).unwrap_or(None))
    }

    /// Remove the given symbol name from the table. (used by undef)
    #[inline]
    fn remove_symbol<S>(&mut self, symbol: S) -> Result<Option<ValueAndSource>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        let symbol = self.extend_readable_symbol(symbol)?;
        Ok(self.map.remove(&symbol))
    }

    #[inline]
    fn is_used<S>(&self, symbol: S) -> bool
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        let symbol = self.extend_readable_symbol(symbol).unwrap();
        self.used_symbols.contains(&symbol)
    }

    #[inline]
    fn use_symbol<S>(&mut self, symbol: S)
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        let symbol = self.extend_readable_symbol(symbol).unwrap();
        self.used_symbols.insert(symbol);
    }
}

impl SymbolsTable {
    /// We are leaving the inner loop and remove its value
    pub fn pop_counter_value(&mut self) -> ExprResult {
        self.clear_counters_lut();
        let res = self.counters.pop().unwrap();
        self.rebuild_counters_lut();
        res
    }

    /// We are entering a new loop and add its value
    pub fn push_counter_value(&mut self, e: ExprResult) {
        self.clear_counters_lut();
        self.counters.push(e);
        self.rebuild_counters_lut();
    }

    fn clear_counters_lut(&mut self) {
        let mut key = "".to_owned();
        for _ in 0..self.counters.len() {
            key.push('#');
            self.remove_symbol(key.clone())
                .expect("[BUG] symbol {key} MUST be present");
        }
    }

    fn rebuild_counters_lut(&mut self) {
        let mut key = "".to_owned();
        for value in self.counters.clone() {
            key.push('#');
            self.assign_symbol_to_value(key.clone(), value).unwrap(); // Here we lost the location :(
        }
    }
}

#[allow(missing_docs)]
impl SymbolsTable {
    pub fn laxist() -> Self {
        let mut map = ModuleSymbolTable::default();
        map.insert(Symbol::from("$"), Value::Expr(0.into()).into());
        let mut table = SymbolsTable::default();
        table.dummy = true;
        table.current_global_label = "".into();
        table
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
    #[inline]
    pub fn get_potential_candidates(&self, symbol: Symbol) -> SmallVec<[Symbol; 2]> {
        if symbol.value().starts_with("::") {
            smallvec![symbol.value()[2..].to_owned().into()]
        }
        else if self.namespace_stack.is_empty() {
            smallvec![symbol]
        }
        else {
            let full = symbol.clone();

            let _global = self.namespace_stack.clone();
            let global = self.inject_current_namespace(symbol);

            smallvec![global, full]
        }
    }

    #[inline]
    fn inject_current_namespace<S>(&self, symbol: S) -> Symbol
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        let mut global = self.namespace_stack.clone();
        global.push(symbol.into());
        global.iter().join(".").into()
    }

    #[inline]
    fn extend_readable_symbol<S>(&self, symbol: S) -> Result<Symbol, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        let symbol = self.extend_local_and_patterns_for_symbol(symbol)?;
        let candidates = self.get_potential_candidates(symbol);

        if candidates.len() == 1 {
            Ok(candidates[0].clone())
        }
        else if self.map.contains_key(&candidates[0]) {
            Ok(candidates[0].clone())
        }
        else {
            Ok(candidates[1].clone())
        }
    }

    #[inline]
    fn extend_writable_symbol<S>(&self, symbol: S) -> Result<Symbol, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        let symbol = self.extend_local_and_patterns_for_symbol(symbol)?;
        let candidates = self.get_potential_candidates(symbol);

        Ok(candidates[0].clone())
    }

    /// Return the current addres if it is known or return an error
    #[inline]
    pub fn current_address(&self) -> Result<u16, SymbolError> {
        match self.value("$")? {
            Some(address) => Ok(address.integer().unwrap() as u16),
            None => Err(SymbolError::UnknownAssemblingAddress)
        }
    }

    /// Update `$` value
    #[inline]
    pub fn set_current_address(&mut self, address: PhysicalAddress) {
        self.map.insert("$".into(), Value::Address(address).into());
    }

    #[inline]
    pub fn set_current_output_address(&mut self, address: PhysicalAddress) {
        self.map.insert("$$".into(), Value::Address(address).into());
    }

    /// Set the given symbol to $ value
    #[inline]
    pub fn set_symbol_to_current_address<S>(&mut self, symbol: S) -> Result<(), SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        let symbol = self.extend_local_and_patterns_for_symbol(symbol)?;
        let symbol = self.extend_readable_symbol::<Symbol>(symbol)?;
        self.current_address().map(|val| {
            let value = Value::Expr(val.into());
            let value: ValueAndSource = value.into();
            self.map.insert(symbol.clone(), value.clone());
            self.current_pass_map.insert(symbol, value);
        })
    }

    /// Set the given Value to the given value
    /// Return the previous value if any
    #[inline]
    pub fn set_symbol_to_value<S, V: Into<ValueAndSource>>(
        &mut self,
        symbol: S,
        value: V
    ) -> Result<Option<ValueAndSource>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        let symbol = self.extend_local_and_patterns_for_symbol(symbol)?;
        let symbol = self.inject_current_namespace::<Symbol>(symbol);

        let value = value.into();
        self.current_pass_map.insert(symbol.clone(), value.clone());
        Ok(self.map.insert(symbol, value))
    }

    #[inline]
    pub fn update_symbol_to_value<S, V: Into<ValueAndSource>>(
        &mut self,
        symbol: S,
        value: V
    ) -> Result<(), SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        let symbol = self.extend_readable_symbol(symbol)?;
        let symbols = self.get_potential_candidates(symbol);
        let symbol = symbols
            .iter()
            .find(|symbol| self.map.contains_key(symbol))
            .unwrap();

        let value = value.into();

        self.current_pass_map.insert(symbol.clone(), value.clone());

        *(self.map.get_mut(symbol).unwrap()) = value;

        Ok(())
    }

    /// Instead of returning the value, return the bank information
    /// logic stolen to rasm
    #[inline]
    pub fn prefixed_value<S>(
        &self,
        prefix: &LabelPrefix,
        key: S
    ) -> Result<Option<u16>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        let key = Symbol::from(key);
        let addr = self.address_value::<Symbol>(key)?;
        Ok(addr.map(|v| {
            match prefix {
                LabelPrefix::Bank => match v {
                    PhysicalAddress::Memory(v) => v.bank() as u16,
                    PhysicalAddress::Bank(v) => v.bank() as _,
                    PhysicalAddress::Cpr(v) => v.bloc() as _,
                }
                LabelPrefix::Page => match v {
                    PhysicalAddress::Memory(v) => v.ga_bank() & 0x00ff,
                    PhysicalAddress::Bank(v) => todo!(),
                    PhysicalAddress::Cpr(_) => todo!(),
                }
                LabelPrefix::Pageset => match v {
                    PhysicalAddress::Memory(v) => v.ga_page() & 0x00ff, // delete 0x7f00
                    PhysicalAddress::Bank(_) => todo!(),
                    PhysicalAddress::Cpr(_) => todo!(),
                }
            }
        } as _))

        // Ok(match prefix {
        // LabelPrefix::Bank => Some(bank as _),
        //
        // LabelPrefix::Page => {
        // if page == 0 {
        // Some(0x7fc0)
        // } else {
        // Some(0x7FC4 + (bank & 3) + ((bank & 31) >> 2) * 8 - 0x100 * (bank >> 5))
        // }
        // }
        //
        // LabelPrefix::Pageset => {
        // if page == 0 {
        // Some(0x7fc0)
        // } else {
        // Some(0x7FC2 + ((bank & 31) >> 2) * 8 - 0x100 * (bank >> 5))
        // }
        // }
        // })
    }

    /// Check if the symbol table contains the expected symbol, whatever is the pass
    #[inline]
    pub fn contains_symbol<S>(&self, symbol: S) -> Result<bool, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        let symbol = self.extend_local_and_patterns_for_symbol(symbol)?;
        let symbols = self.get_potential_candidates(symbol);
        Ok(symbols.iter().any(|symbol| self.map.contains_key(symbol)))
    }

    /// Check if the symbol table contains the expected symbol, added during the current pass
    #[inline]
    pub fn symbol_exist_in_current_pass<S>(&self, symbol: S) -> Result<bool, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        let symbol = self.extend_local_and_patterns_for_symbol(symbol)?;
        let symbols = self.get_potential_candidates(symbol);
        Ok(symbols
            .iter()
            .any(|symbol| self.current_pass_map.contains_key(symbol)))
    }

    /// Returns the closest Value
    #[inline]
    pub fn closest_symbol<S>(
        &self,
        symbol: S,
        r#for: SymbolFor
    ) -> Result<Option<SmolStr>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        let symbol = self.extend_local_and_patterns_for_symbol(symbol)?;
        let symbol = self.extend_readable_symbol::<Symbol>(symbol)?;
        #[cfg(all(not(target_arch = "wasm32"), feature = "rayon"))]
        let iter = self.map.par_iter();
        #[cfg(any(target_arch = "wasm32", not(feature = "rayon")))]
        let iter = self.map.iter();

        Ok(iter
            .filter(|(_k, v)| {
                match (v.value(), r#for) {
                    (Value::Expr(_), SymbolFor::Number)
                    | (Value::Expr(_), SymbolFor::Address)
                    | (Value::Address(_), SymbolFor::Address)
                    | (Value::Address(_), SymbolFor::Number)
                    | (Value::Macro(_), SymbolFor::Macro)
                    | (Value::Struct(_), SymbolFor::Struct)
                    | (Value::Counter(_), SymbolFor::Counter)
                    | (_, SymbolFor::Any) => true,
                    _ => false
                }
            })
            .map(|(k, _v)| k)
            .map(move |symbol2| {
                let symbol_upper = symbol.0.to_ascii_uppercase();
                let symbol2_upper = symbol2.0.to_ascii_uppercase();
                let levenshtein_distance = strsim::levenshtein(&symbol2.0, &symbol.0)
                    .min(strsim::levenshtein(&symbol2_upper, &symbol_upper));
                let included = if symbol2_upper.contains(&symbol_upper) {
                    0
                }
                else {
                    1
                };

                ((included, levenshtein_distance), symbol2.0.clone())
            })
            .min()
            .map(|(_distance, symbol2)| symbol2))
    }

    #[inline]
    pub fn kind<S>(&self, symbol: S) -> Result<&'static str, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        Ok(match self.value(symbol)?.map(|v| v.value()) {
            Some(Value::Expr(_)) => "number",
            Some(Value::Address(_)) => "address",
            Some(Value::Macro(_)) => "macro",
            Some(Value::Struct(_)) => "struct",
            Some(Value::Counter(_)) => "counter",
            Some(Value::String(_)) => "string",
            None => "any"
        })
    }
}

/// Wrapper around the Values table in order to easily manage the fact that the assembler is case dependent or independant
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub struct SymbolsTableCaseDependent {
    table: SymbolsTable,
    case_sensitive: bool
}

/// By default, the assembler is case sensitive
impl Default for SymbolsTableCaseDependent {
    fn default() -> Self {
        Self {
            table: SymbolsTable::default(),
            case_sensitive: true
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
    delegate! {
        to self.table {
            pub fn current_address(&self) -> Result<u16, SymbolError>;
            pub fn set_current_address(&mut self, addr: PhysicalAddress);
            pub fn set_current_output_address(&mut self, addr: PhysicalAddress);
            pub fn push_seed(&mut self, seed: usize);
            pub fn pop_seed(&mut self);
            pub fn pop_counter_value(&mut self);
            pub fn push_counter_value(&mut self, e: ExprResult);
        }
    }

    pub fn new(table: SymbolsTable, case_sensitive: bool) -> Self {
        Self {
            table,
            case_sensitive
        }
    }

    #[inline]
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

    /// Modify the Value value depending on the case configuration (do nothing, or set uppercase)
    #[inline]
    pub fn normalize_symbol<S>(&self, symbol: S) -> Symbol
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        if self.case_sensitive {
            symbol.into()
        }
        else {
            symbol.as_ref().to_uppercase().into()
        }
    }

    pub fn set_table(&mut self, table: SymbolsTable) {
        self.table = table
    }

    // Setup the current label for local to global labels conversions
    #[inline]
    pub fn set_current_label<S>(&mut self, symbol: S) -> Result<(), SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        self.table
            .set_current_global_label::<Symbol>(self.normalize_symbol(symbol))
    }

    #[inline]
    pub fn get_current_label(&self) -> &Symbol {
        self.table.get_current_label()
    }

    #[inline]
    pub fn set_symbol_to_current_address<S>(&mut self, symbol: S) -> Result<(), SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        self.table
            .set_symbol_to_current_address::<Symbol>(self.normalize_symbol(symbol))
    }

    #[inline]
    pub fn set_symbol_to_value<S, V: Into<ValueAndSource>>(
        &mut self,
        symbol: S,
        value: V
    ) -> Result<Option<ValueAndSource>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        self.table
            .set_symbol_to_value::<Symbol, _>(self.normalize_symbol(symbol), value)
    }

    #[inline]
    pub fn update_symbol_to_value<S, E: Into<ValueAndSource>>(
        &mut self,
        symbol: S,
        value: E
    ) -> Result<(), SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        self.table
            .update_symbol_to_value::<Symbol, _>(self.normalize_symbol(symbol), value.into())
    }

    #[inline]
    pub fn prefixed_value<S>(
        &self,
        prefix: &LabelPrefix,
        symbol: S
    ) -> Result<Option<u16>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        self.table
            .prefixed_value::<Symbol>(prefix, self.normalize_symbol(symbol))
    }

    #[inline]
    pub fn contains_symbol<S>(&self, symbol: S) -> Result<bool, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        self.table
            .contains_symbol::<Symbol>(self.normalize_symbol(symbol))
    }

    #[inline]
    pub fn symbol_exist_in_current_pass<S>(&self, symbol: S) -> Result<bool, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        self.table
            .symbol_exist_in_current_pass::<Symbol>(self.normalize_symbol(symbol))
    }

    pub fn new_pass(&mut self) {
        self.table.new_pass();
    }

    pub fn kind<S>(&self, symbol: S) -> Result<&'static str, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        self.table.kind(symbol)
    }

    pub fn closest_symbol<S>(
        &self,
        symbol: S,
        r#for: SymbolFor
    ) -> Result<Option<SmolStr>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        let symbol = self.normalize_symbol(symbol);
        self.table.closest_symbol::<Symbol>(symbol, r#for)
    }

    pub fn extend_local_and_patterns_for_symbol<S>(&self, symbol: S) -> Result<Symbol, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        let symbol = self.normalize_symbol(symbol);
        self.table
            .extend_local_and_patterns_for_symbol::<Symbol>(symbol)
    }
}

impl SymbolsTableTrait for SymbolsTableCaseDependent {
    #[inline]
    fn is_used<S>(&self, symbol: S) -> bool
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        self.table.is_used::<Symbol>(self.normalize_symbol(symbol))
    }

    #[inline]
    fn use_symbol<S>(&mut self, symbol: S)
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        self.table
            .use_symbol::<Symbol>(self.normalize_symbol(symbol))
    }

    #[inline]
    fn expression_symbol(&self) -> Vec<(&Symbol, &ValueAndSource)> {
        self.table.expression_symbol()
    }

    #[inline]
    fn int_value<S>(&self, symbol: S) -> Result<Option<i32>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        self.table
            .int_value::<Symbol>(self.normalize_symbol(symbol))
    }

    #[inline]
    fn counter_value<S>(&self, symbol: S) -> Result<Option<i32>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        self.table
            .counter_value::<Symbol>(self.normalize_symbol(symbol))
    }

    #[inline]
    fn macro_value<S>(&self, symbol: S) -> Result<Option<&Macro>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        self.table
            .macro_value::<Symbol>(self.normalize_symbol(symbol))
    }

    #[inline]
    fn struct_value<S>(&self, symbol: S) -> Result<Option<&Struct>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        self.table
            .struct_value::<Symbol>(self.normalize_symbol(symbol))
    }

    #[inline]
    fn value<S>(&self, symbol: S) -> Result<Option<&ValueAndSource>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        self.table.value::<Symbol>(self.normalize_symbol(symbol))
    }

    #[inline]
    fn remove_symbol<S>(&mut self, symbol: S) -> Result<Option<ValueAndSource>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        let normalized = self.normalize_symbol(symbol);
        let _ = self.table.current_pass_map.remove(&normalized);
        self.table.remove_symbol::<Symbol>(normalized)
    }

    #[inline]
    fn address_value<S>(&self, symbol: S) -> Result<Option<&PhysicalAddress>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        self.table
            .address_value::<Symbol>(self.normalize_symbol(symbol))
    }

    #[inline]
    fn assign_symbol_to_value<S, V: Into<ValueAndSource>>(
        &mut self,
        symbol: S,
        value: V
    ) -> Result<Option<ValueAndSource>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        self.table
            .assign_symbol_to_value::<Symbol, _>(self.normalize_symbol(symbol), value)
    }

    #[inline]
    fn enter_namespace(&mut self, namespace: &str) {
        self.table
            .enter_namespace(self.normalize_symbol(namespace).value())
    }

    #[inline]
    fn leave_namespace(&mut self) -> Result<Symbol, SymbolError> {
        self.table.leave_namespace()
    }
}
