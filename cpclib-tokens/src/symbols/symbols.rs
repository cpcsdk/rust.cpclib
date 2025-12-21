use std::fmt::{Debug, Display};
use std::ops::Deref;

#[cfg(all(not(target_arch = "wasm32"), feature = "rayon"))]
use cpclib_common::rayon::{iter::IntoParallelRefIterator, iter::ParallelIterator};
use cpclib_common::smol_str::SmolStr;

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
    fname: String,
    line: usize,
    column: usize
}

impl Display for SourceLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", &self.fname, self.line, self.column)
    }
}

impl SourceLocation {
    pub fn new(fname: String, line: usize, column: usize) -> Self {
        SourceLocation {
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
    source: Option<SourceLocation>,
    flavor: AssemblerFlavor
}

impl Macro {
    pub fn new(
        name: SmolStr,
        params: &[&str],
        code: String,
        source: Option<SourceLocation>,
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

    pub fn r#macro(&self) -> Option<&Macro> {
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
        match value {
            ExprResult::String(s) => Value::String(s),
            v => Value::Expr(v)
        }
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
