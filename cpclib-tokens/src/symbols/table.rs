use std::collections::HashSet;
use std::ops::{Deref, DerefMut};
use std::sync::LazyLock;

use ahash::AHashMap as HashMap;
use cpclib_common::smallvec::{SmallVec, smallvec};
use cpclib_common::smol_str::ToSmolStr;
use cpclib_common::strsim;
use delegate::delegate;
use evalexpr::{ContextWithMutableVariables, HashMapContext, build_operator_tree};
use regex::Regex;

use crate::symbols::{
    PhysicalAddress, Struct, Symbol, SymbolError, SymbolFor, Value, ValueAndSource, ValueMacro
};
use crate::{ExprResult, LabelPrefix};

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

    fn any_value<S>(&self, symbol: S) -> Result<Option<&ValueAndSource>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>;
    fn counter_value<S>(&self, symbol: S) -> Result<Option<i32>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>;
    fn macro_value<S>(&self, symbol: S) -> Result<Option<&ValueMacro>, SymbolError>
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

#[derive(Debug, Clone, Default)]
struct TableFrame(HashMap<Symbol, ValueAndSource>);

impl Deref for TableFrame {
    type Target = HashMap<Symbol, ValueAndSource>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TableFrame {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Contains the variables and parameters for the successive calls
/// of hand written functions.
/// This allows to play properly with recursive ones
#[derive(Debug, Clone, Default)]
struct FunctionsStack(Vec<TableFrame>);

#[allow(dead_code)]
impl FunctionsStack {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Create the frame for the current function
    pub fn enter_function(&mut self) {
        self.0.push(TableFrame::default());
    }

    /// Destroy the frame of the current function
    pub fn leave_function(&mut self) -> TableFrame {
        self.0.pop().unwrap() // must failed when called for nothing
    }

    /// Provide the frame for the current function call
    pub fn current_frame(&self) -> Option<&TableFrame> {
        self.0.last()
    }

    /// Mutably provide the frame for the current function call
    pub fn current_frame_mut(&mut self) -> Option<&mut TableFrame> {
        self.0.last_mut()
    }
}

/// Handle Tree like maps.
#[derive(Debug, Clone, Default)]
struct ModuleSymbolTable {
    current: TableFrame,
    children: HashMap<Symbol, ModuleSymbolTable>
}

impl Deref for ModuleSymbolTable {
    type Target = TableFrame;

    fn deref(&self) -> &Self::Target {
        &self.current
    }
}

impl DerefMut for ModuleSymbolTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.current
    }
}

#[allow(dead_code)]
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

    fn iter(&self) -> ModuleSymbolTableIterator<'_> {
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
            others: table.children.values().collect(),
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

    /// the frame specific to the call of user defined functions in order to make them reentrant
    functions_stack: FunctionsStack,

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
            counters: Default::default(),
            functions_stack: Default::default()
        }
    }
}

/// Local/global label handling code
#[allow(dead_code)]
impl SymbolsTable {
    pub fn enter_function(&mut self) {
        self.functions_stack.enter_function();
    }

    pub fn leave_function(&mut self) {
        self.functions_stack.leave_function();
    }

    pub fn new_pass(&mut self) {
        self.current_pass_map = ModuleSymbolTable::default();
        self.current_pass_map.add_children("".to_owned().into());
    }

    pub fn used_symbols(&self) -> impl Iterator<Item = &Symbol> {
        self.used_symbols.iter()
    }

    pub fn available_symbols(&self) -> impl Iterator<Item = &Symbol> {
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
        // dbg!("Input", &symbol);

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

            let local_value = match self.any_value::<&str>(local_expr)?.map(|vl| vl.value()) {
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
                            .any_value::<&str>(variable)?
                            .ok_or_else(|| SymbolError::WrongSymbol(variable.into()))?;
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

        let symbol: Symbol = symbol.into();
        // dbg!("output", &symbol);

        Ok(symbol)
    }
}

/// Module handling code
impl SymbolsTable {
    /// Retrieve the map for the currently selected module
    #[inline]
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    fn module_map(&self, namespace: &[Symbol]) -> &ModuleSymbolTable {
        let mut current_map = &self.map;
        for current_namespace in namespace.iter() {
            current_map = current_map.children(current_namespace).unwrap();
        }
        current_map
    }

    #[inline]
    #[allow(dead_code)]
    fn module_map_mut(&mut self, namespace: &[Symbol]) -> &mut ModuleSymbolTable {
        let mut current_map = &mut self.map;
        for current_namespace in namespace.iter() {
            current_map = current_map.children_mut(current_namespace).unwrap();
        }
        current_map
    }

    /// Split the namespaces of the symbol
    #[inline]
    #[allow(dead_code)]
    fn split_namespaces(symbol: Symbol) -> Vec<Symbol> {
        symbol
            .value()
            .split(':')
            .map(|s| s.to_owned())
            .map(|s| s.into())
            .collect()
    }
}

impl SymbolsTableTrait for SymbolsTable {
    #[inline]
    fn expression_symbol(&self) -> Vec<(&Symbol, &ValueAndSource)> {
        self.map
            .iter()
            .filter(|(_k, v)| matches!(v.value(), Value::Expr(_) | Value::Address(_)))
            .collect()
    }

    #[inline]
    fn int_value<S>(&self, symbol: S) -> Result<Option<i32>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        Ok(self
            .any_value(symbol)?
            .and_then(|v| v.integer())
            .or(if self.dummy { Some(1i32) } else { None }))
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

        let frame = if let Some(frame) = self.functions_stack.current_frame_mut() {
            frame
        }
        else {
            self.assignable.insert(symbol.clone());
            self.current_pass_map.insert(symbol.clone(), value.clone());
            self.map.deref_mut()
        };

        Ok(frame.insert(symbol, value))
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
    fn any_value<S>(&self, symbol: S) -> Result<Option<&ValueAndSource>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        let symbol = self.extend_readable_symbol(symbol)?;
        let res = if let Some(frame) = self.functions_stack.current_frame() {
            frame.get(&symbol).or_else(|| self.map.get(&symbol))
        }
        else {
            self.map.get(&symbol)
        };

        Ok(res)
    }

    #[inline]
    fn counter_value<S>(&self, symbol: S) -> Result<Option<i32>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        Ok(self
            .any_value(symbol)?
            .map(|v| v.counter())
            .map(|v| v.unwrap())
            .or(if self.dummy { Some(1i32) } else { None }))
    }

    #[inline]
    fn macro_value<S>(&self, symbol: S) -> Result<Option<&ValueMacro>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        Ok(self.any_value(symbol)?.map(|v| v.r#macro()).unwrap_or(None))
    }

    #[inline]
    fn struct_value<S>(&self, symbol: S) -> Result<Option<&Struct>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        Ok(self
            .any_value(symbol)?
            .map(|v| v.r#struct())
            .unwrap_or(None))
    }

    #[inline]
    fn address_value<S>(&self, symbol: S) -> Result<Option<&PhysicalAddress>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        Ok(self.any_value(symbol)?.map(|v| v.address()).unwrap_or(None))
    }

    /// Remove the given symbol name from the table. (used by undef)
    #[inline]
    fn remove_symbol<S>(&mut self, symbol: S) -> Result<Option<ValueAndSource>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        let symbol: Symbol = symbol.into();

        // try to remove for the function first
        if let Some(frame) = self.functions_stack.current_frame_mut() {
            let res = frame.remove(&symbol);
            if res.is_some() {
                return Ok(res);
            }
        }

        // then to the assembler
        let symbol = self.extend_readable_symbol::<Symbol>(symbol)?;
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

impl SymbolsTable {
    pub fn laxist() -> Self {
        let mut map = ModuleSymbolTable::default();
        map.insert(Symbol::from("$"), Value::Expr(0.into()).into());
        SymbolsTable {
            dummy: true,
            current_global_label: "".into(),
            ..Default::default()
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
        // Precompute capacity for final string: sum of segment lengths + separators dots
        let seg_lens: usize = global.iter().map(|s| s.to_smolstr().len()).sum();
        let separators = if global.is_empty() {
            0
        }
        else {
            global.len() - 1
        };
        let mut acc = String::with_capacity(seg_lens + separators);
        for (i, s) in global.iter().enumerate() {
            acc.push_str(s.to_smolstr().as_str());
            if i + 1 < global.len() {
                acc.push('.');
            }
        }
        acc.into()
    }

    #[inline]
    fn extend_readable_symbol<S>(&self, symbol: S) -> Result<Symbol, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        let symbol = self.extend_local_and_patterns_for_symbol(symbol)?;
        let candidates = self.get_potential_candidates(symbol);

        if candidates.len() == 1 || self.map.contains_key(&candidates[0]) {
            Ok(candidates[0].clone())
        }
        else {
            Ok(candidates[1].clone())
        }
    }

    #[inline]
    #[allow(dead_code)]
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
        match self.any_value("$")? {
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

            let frame = if let Some(frame) = self.functions_stack.current_frame_mut() {
                frame
            }
            else {
                self.current_pass_map.insert(symbol.clone(), value.clone());
                self.map.deref_mut()
            };
            frame.insert(symbol, value);
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

        let frame = if let Some(frame) = self.functions_stack.current_frame_mut() {
            frame
        }
        else {
            self.current_pass_map.insert(symbol.clone(), value.clone());
            self.map.deref_mut()
        };

        Ok(frame.insert(symbol, value))
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

        let frame = if let Some(frame) = self.functions_stack.current_frame_mut() {
            frame
        }
        else {
            self.current_pass_map.insert(symbol.clone(), value.clone());
            self.map.deref_mut()
        };

        *(frame.get_mut(symbol).unwrap()) = value;

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
                    PhysicalAddress::Bank(_v) => todo!(),
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
        S: AsRef<str> + Clone
    {
        if let Some(frame) = self.functions_stack.current_frame()
            && frame.contains_key(&symbol.clone().into())
        {
            return Ok(true);
        }

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
        // TODO check if there is something to do with functions symbols

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
    ) -> Result<Option<&str>, SymbolError>
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

        let iter: Box<dyn Iterator<Item = (&Symbol, &ValueAndSource)>> =
            if let Some(frame) = self.functions_stack.current_frame() {
                Box::new(frame.iter().chain(iter))
            }
            else {
                Box::new(iter)
            };

        Ok(iter
            .filter(|(_k, v)| {
                matches!((v.value(), r#for),
                    (Value::Expr(_), SymbolFor::Number)
                    | (Value::Expr(_), SymbolFor::Address)
                    | (Value::Address(_), SymbolFor::Address)
                    | (Value::Address(_), SymbolFor::Number)
                    | (Value::Macro(_), SymbolFor::Macro)
                    | (Value::Struct(_), SymbolFor::Struct)
                    | (Value::Counter(_), SymbolFor::Counter)
                    | (_, SymbolFor::Any)
                )
            })
            .map(|(k, _v)| k)
            .map(move |symbol2| {
                let symbol_upper = symbol.value().to_ascii_uppercase();
                let symbol2_upper = symbol2.value().to_ascii_uppercase();
                let levenshtein_distance = strsim::levenshtein(symbol2.value(), symbol.value())
                    .min(strsim::levenshtein(&symbol2_upper, &symbol_upper));
                let included = if symbol2_upper.contains(&symbol_upper) {
                    0
                }
                else {
                    1
                };

                ((included, levenshtein_distance), symbol2.value())
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
        Ok(match self.any_value(symbol)?.map(|v| v.value()) {
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

impl From<SymbolsTableCaseDependent> for SymbolsTable {
    fn from(val: SymbolsTableCaseDependent) -> Self {
        val.table
    }
}

impl From<&SymbolsTableCaseDependent> for SymbolsTable {
    fn from(val: &SymbolsTableCaseDependent) -> Self {
        val.table.clone()
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
            pub fn enter_function(&mut self);
            pub fn leave_function(&mut self);
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
    ) -> Result<Option<&str>, SymbolError>
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
    fn macro_value<S>(&self, symbol: S) -> Result<Option<&ValueMacro>, SymbolError>
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
    fn any_value<S>(&self, symbol: S) -> Result<Option<&ValueAndSource>, SymbolError>
    where
        Symbol: From<S>,
        S: AsRef<str>
    {
        self.table
            .any_value::<Symbol>(self.normalize_symbol(symbol))
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
