use std::fmt::{Debug, Display};
use std::ops::Deref;

use cpclib_common::smol_str::SmolStr;

use crate::macro_segment::TokenizedMacroContent;
use crate::symbols::{PhysicalAddress, SymbolsTableTrait};
use crate::{AssemblerFlavor, ExprResult, ListingElement, ToSimpleToken, Token, expression};

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
    source: Option<SourceLocation>
}

impl Struct {
    pub fn new<T: ListingElement + ToSimpleToken, S: AsRef<str>>(
        name: impl AsRef<str>,
        content: &[(S, T)],
        source: Option<SourceLocation>
    ) -> Self {
        Self {
            name: name.as_ref().into(),
            content: content
                .iter()
                .map(|(s, t)| (SmolStr::from(s.as_ref()), t.as_simple_token().into_owned()))
                .collect::<Vec<_>>(),
            source
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn source(&self) -> Option<&SourceLocation> {
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
            .collect::<Vec<_>>()
    }

    /// Get the len of any field
    pub fn field_size<T: SymbolsTableTrait>(token: &Token, table: &T) -> i32 {
        match token {
            Token::Defb(c) => c.len().max(1) as i32,
            Token::Defw(c) => (2 * c.len()).max(2) as i32,
            Token::MacroCall(n, _) => {
                let s = table.struct_value(n).ok().unwrap().unwrap(); // TODO handle error here
                s.len(table)
            },
            _ => unreachable!("{:?}", token)
        }
    }

    /// Get the len of the structure
    pub fn len<T: SymbolsTableTrait>(&self, table: &T) -> i32 {
        self.content
            .iter()
            .map(|(_, t)| Self::field_size(t, table))
            .sum()
    }

    pub fn nb_args(&self) -> usize {
        self.content.len()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceLocation {
    fname: Box<str>,
    line: usize,
    column: usize
}

impl Display for SourceLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.fname, self.line, self.column)
    }
}

impl SourceLocation {
    pub fn new(fname: impl Into<Box<str>>, line: usize, column: usize) -> Self {
        SourceLocation {
            fname: fname.into(),
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

    /// Same rendering as `Display`/`to_string()`, but built directly into
    /// a `Box<str>` at its final size instead of going through a growable
    /// `String`: `self.to_string()` writes `fname`, then `:`, then `line`,
    /// then `:`, then `column` one at a time, and std's capacity heuristics
    /// aim to avoid reallocating *while writing*, not to land on the exact
    /// final length - so the resulting `String` almost always carries a
    /// few bytes of spare capacity that a later `.into_boxed_str()` would
    /// have to reallocate-and-copy away. Since this is built once per
    /// macro/struct call (`MacroExpansionKey::def_location`) and kept as
    /// `Box<str>` for the rest of its life, this computes the exact byte
    /// length up front and writes straight into a `Box<str>` allocated at
    /// that size - no `String`, no `Vec`, no resize, ever.
    pub fn to_boxed_str(&self) -> Box<str> {
        fn digits(mut n: usize) -> usize {
            if n == 0 {
                return 1;
            }
            let mut count = 0;
            while n > 0 {
                count += 1;
                n /= 10;
            }
            count
        }

        let exact_len = self.fname.len() + 1 + digits(self.line) + 1 + digits(self.column);
        crate::boxed_str_builder::build_boxed_str(exact_len, |cursor| {
            let _ = std::io::Write::write_fmt(
                cursor,
                format_args!("{}:{}:{}", self.fname, self.line, self.column)
            );
        })
    }
}

#[derive(Debug, Clone)]
pub struct ValueMacro {
    // The name of the macro
    name: SmolStr,
    // The name of its arguments
    params: Vec<SmolStr>,
    // The content
    code: Box<str>,
    segments: TokenizedMacroContent,
    // Origin of the macro (for error messages)
    source: Option<SourceLocation>,
    flavor: AssemblerFlavor,
    // Whether the declaration ended with a trailing `...`, opting the macro
    // into accepting (and indexing, via `{N}`/`{#}` in the body) extra
    // arguments beyond `params`.
    has_variadic: bool
}

impl ValueMacro {
    pub fn new(
        name: SmolStr,
        params: &[&str],
        code: impl Into<Box<str>>,
        tokenized_content: crate::macro_segment::TokenizedMacroContent,
        source: Option<SourceLocation>,
        flavor: AssemblerFlavor,
        has_variadic: bool
    ) -> Self {
        ValueMacro {
            name,
            params: params.iter().map(|&s| SmolStr::from(s)).collect(),
            code: code.into(),
            segments: tokenized_content,
            source,
            flavor,
            has_variadic
        }
    }

    #[inline]
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    #[inline]
    pub fn source(&self) -> Option<&SourceLocation> {
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

    #[inline]
    pub fn has_variadic(&self) -> bool {
        self.has_variadic
    }

    #[inline]
    pub fn segments(&self) -> &TokenizedMacroContent {
        &self.segments
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
    Macro(ValueMacro),
    /// Structure information
    Struct(Struct),
    /// Counter for a repetition
    Counter(i32)
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
        if let Value::Expr(e) = self {
            Some(e)
        }
        else {
            None
        }
    }

    pub fn is_expr(&self) -> bool {
        matches!(self, Value::Expr(_))
    }

    pub fn integer(&self) -> Option<i32> {
        match self {
            Value::Expr(ExprResult::Value(i)) => Some(*i),
            Value::Address(addr) => Some(addr.address() as _),
            _ => None
        }
    }

    pub fn address(&self) -> Option<&PhysicalAddress> {
        if let Value::Address(addr) = self {
            Some(addr)
        }
        else {
            None
        }
    }

    pub fn counter(&self) -> Option<i32> {
        if let Value::Counter(i) = self {
            Some(*i)
        }
        else {
            None
        }
    }

    pub fn r#macro(&self) -> Option<&ValueMacro> {
        if let Value::Macro(m) = self {
            Some(m)
        }
        else {
            None
        }
    }

    pub fn r#struct(&self) -> Option<&Struct> {
        if let Value::Struct(m) = self {
            Some(m)
        }
        else {
            None
        }
    }

    pub fn string(&self) -> Option<&str> {
        if let Value::String(s) = self {
            Some(s.as_str())
        }
        else {
            None
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

impl From<ValueMacro> for Value {
    fn from(m: ValueMacro) -> Self {
        Self::Macro(m)
    }
}

impl From<ExprResult> for Value {
    fn from(value: ExprResult) -> Self {
        match value {
            ExprResult::String(s) => Value::String(s),
            v => Value::Expr(v)
        }
    }
}

impl From<String> for Value {
    fn from(i: String) -> Self {
        let value: ExprResult = i.into();
        match value {
            ExprResult::String(s) => Value::String(s),
            v => Value::Expr(v)
        }
    }
}

impl From<SmolStr> for Value {
    fn from(i: SmolStr) -> Self {
        let value: ExprResult = i.into();
        match value {
            ExprResult::String(s) => Value::String(s),
            v => Value::Expr(v)
        }
    }
}

impl From<&SmolStr> for Value {
    fn from(i: &SmolStr) -> Self {
        let value: ExprResult = i.into();
        match value {
            ExprResult::String(s) => Value::String(s),
            v => Value::Expr(v)
        }
    }
}

impl From<f64> for Value {
    fn from(i: f64) -> Self {
        Value::Expr(i.into())
    }
}

impl From<bool> for Value {
    fn from(i: bool) -> Self {
        Value::Expr(i.into())
    }
}

impl From<usize> for Value {
    fn from(i: usize) -> Self {
        Value::Expr(i.into())
    }
}

impl From<i32> for Value {
    fn from(i: i32) -> Self {
        Value::Expr(i.into())
    }
}

impl From<u16> for Value {
    fn from(i: u16) -> Self {
        Value::Expr(i.into())
    }
}

impl From<u8> for Value {
    fn from(i: u8) -> Self {
        Value::Expr(i.into())
    }
}

impl From<i8> for Value {
    fn from(i: i8) -> Self {
        Value::Expr(i.into())
    }
}

impl From<char> for Value {
    fn from(i: char) -> Self {
        Value::Expr(i.into())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct Symbol(SmolStr);

impl Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.as_str())
    }
}

impl From<&str> for Symbol {
    fn from(s: &str) -> Symbol {
        Symbol(SmolStr::from(s))
    }
}

impl From<String> for Symbol {
    fn from(s: String) -> Symbol {
        Symbol(s.into())
    }
}

impl From<&String> for Symbol {
    fn from(s: &String) -> Symbol {
        Symbol(SmolStr::from(s.as_str()))
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

#[derive(Clone, Debug)]
pub struct ValueAndSource {
    value: Value,
    location: Option<SourceLocation>
}

impl From<ValueAndSource> for Value {
    fn from(val: ValueAndSource) -> Self {
        val.value
    }
}

impl From<expression::ExprResult> for ValueAndSource {
    fn from(value: expression::ExprResult) -> Self {
        let value: Value = value.into();
        value.into()
    }
}

impl From<ValueAndSource> for Option<SourceLocation> {
    fn from(val: ValueAndSource) -> Self {
        val.location
    }
}

impl ValueAndSource {
    pub fn new<V: Into<Value>, L: Into<SourceLocation>>(value: V, location: Option<L>) -> Self {
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

    pub fn location(&self) -> Option<&SourceLocation> {
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
