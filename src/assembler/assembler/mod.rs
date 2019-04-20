use smallvec;

use crate::assembler::tokens::*;
use crate::basic::*;

use crate::assembler::AssemblerError;
use crate::assembler::AssemblingOptions;
use smallvec::SmallVec;
use std::collections::HashMap;
use std::fmt;

use delegate::delegate;
use either::*;



/// Use smallvec to put stuff on the stack not the heap and (hope so) speed up assembling
const MAX_SIZE: usize = 4;
pub type Bytes = SmallVec<[u8; MAX_SIZE]>;

/// Add the encoding of an indexed structure
fn add_index(m: &mut Bytes, idx: i32) -> Result<(), AssemblerError> {
    if idx < -127 || idx > 128 {
        Err(format!("Index error {}", idx).into())
    } else {
        let val = (idx & 0xff) as u8;
        add_byte(m, val);
        Ok(())
    }
}

fn add_byte(m: &mut Bytes, b: u8) {
    m.push(b);
}

fn add_word(m: &mut Bytes, w: u16) {
    m.push((w % 256) as u8);
    m.push((w / 256) as u8);
}

fn add_index_register_code(m: &mut Bytes, r: &IndexRegister16) {
    add_byte(m, indexed_register16_to_code(r));
}

const DD: u8 = 0xdd;
const FD: u8 = 0xfd;

///! Lots of things will probably be inspired from RASM
type Bank = [u8; 0x10000];

/// Several passes are needed to properly assemble a source file.
/// This structure allows to code which pass is going to be analysed.
/// First pass consists in collecting the various labels to manipulate and so on. Some labels stay unknown at this moment.
/// Second pass serves to get the final values
#[derive(Clone, Copy, Debug)]
enum AssemblingPass {
    Uninitialized,
    FirstPass,
    SecondPass,
    Finished,
}
impl fmt::Display for AssemblingPass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let content = match self {
            AssemblingPass::Uninitialized => "Uninitialized",
            AssemblingPass::FirstPass => "1",
            AssemblingPass::SecondPass => "2",
            AssemblingPass::Finished => "Finished",
        };
        write!(f, "{}", content)
    }
}

impl AssemblingPass {
    fn is_uninitialized(&self) -> bool {
        match self {
            AssemblingPass::Uninitialized => true,
            _ => false,
        }
    }

    fn is_finished(&self) -> bool {
        match self {
            AssemblingPass::Finished => true,
            _ => false,
        }
    }

    fn is_first_pass(&self) -> bool {
        match self {
            AssemblingPass::FirstPass => true,
            _ => false,
        }
    }

    fn is_second_pass(&self) -> bool {
        match self {
            AssemblingPass::SecondPass => true,
            _ => false,
        }
    }

    fn next_pass(self) -> AssemblingPass {
        match self {
            AssemblingPass::Uninitialized => AssemblingPass::FirstPass,
            AssemblingPass::FirstPass => AssemblingPass::SecondPass,
            AssemblingPass::SecondPass => AssemblingPass::Finished,
            AssemblingPass::Finished => panic!(),
        }
    }
}

/// Manage the stack of stable counters.
/// They are updated each time an opcode is visited
#[derive(Default)]
struct StableTickerCounters {
    counters: Vec<(String, usize)>,
}

impl StableTickerCounters {
    /// Check if a counter with the same name already exists
    pub fn has_counter<S: AsRef<str>>(&self, name: S) -> bool {
        let name = name.as_ref().to_owned();
        self.counters.iter().any(|(s, _)| s == &name)
    }

    /// Add a new counter if no counter has the same name
    pub fn add_counter<S: AsRef<str>>(&mut self, name: S) -> Result<(), AssemblerError> {
        let name: String = name.as_ref().to_owned();
        if self.has_counter(&name) {
            return Err(format!("A counter named `{}` already exists", name).into());
        }
        self.counters.push((name, 0));
        Ok(())
    }

    /// Release the latest counter (if exists)
    pub fn release_last_counter(&mut self) -> Option<(String, usize)> {
        self.counters.pop()
    }

    /// Update each opened counters by count
    pub fn update_counters(&mut self, count: usize) {
        self.counters.iter_mut().for_each(|(_, local_count)| {
            *local_count = *local_count + count;
        });
    }

    pub fn len(&self) -> usize {
        self.counters.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Default, Debug)]
struct OrgZone {
    ibank: usize,
    protect: bool,
    _memstart: usize,
    _memend: usize,
}

#[derive(Debug, Clone)]
pub enum Symbol {
    Integer(i32),
}

#[derive(Debug, Clone)]
pub struct SymbolsTable {
    map: HashMap<String, Symbol>,
    dummy: bool,
}

impl Default for SymbolsTable {
    fn default() -> SymbolsTable {
        SymbolsTable {
            map: HashMap::new(),
            dummy: false,
        }
    }
}

impl SymbolsTable {
    pub fn laxist() -> SymbolsTable {
        let mut map = HashMap::new();
        map.insert(String::from("$"), Symbol::Integer(0));
        SymbolsTable { map, dummy: true }
    }

    /// Return the current addres if it is known or return an error
    pub fn current_address(&self) -> Result<u16, AssemblerError> {
        match self.value(&"$".to_owned()) {
            Some(address) => Ok(address as u16),
            None => Err(AssemblerError::UnknownAssemblingAddress),
        }
    }

    /// Update `$` value
    pub fn set_current_address(&mut self, address: u16) {
        self.map
            .insert(String::from("$"), Symbol::Integer(address as _));
    }

    /// Set the given symbol to $ value
    pub fn set_symbol_to_current_address<S: AsRef<str>>(
        &mut self,
        label: S,
    ) -> Result<(), AssemblerError> {
        self.current_address().map(|val| {
            self.map
                .insert(label.as_ref().to_owned(), Symbol::Integer(val as i32));
        })
    }

    /// Set the given symbol to the given value
    pub fn set_symbol_to_value<S: AsRef<str>>(&mut self, label: S, value: i32) {
        self.map
            .insert(label.as_ref().into(), Symbol::Integer(value as _));
    }

    pub fn update_symbol_to_value<S: AsRef<str>>(&mut self, label: S, value: i32) {
        *(self.map.get_mut(label.as_ref()).unwrap()) = Symbol::Integer(value as _);
    }

    /// TODO return the symbol instead of the int
    pub fn value<S: AsRef<str>>(&self, key: S) -> Option<i32> {
        let key: String = key.as_ref().to_owned();

        let key = key.trim();
        let res = self.map.get(key);
        if res.is_some() {
            match res.unwrap() {
                &Symbol::Integer(val) => Some(val),
            }
        } else {
            if self.dummy == true {
                //eprintln!("{} not found in symbol table. I have replaced it by 1", key);
                Some(1)
            } else {
                //               eprintln!("Symbol table content {:?}", &self.map);
                None
            }
        }
    }

    /// Remove the given symbol name from the table. (used by undef)
    pub fn remove_symbol<S: AsRef<str>>(&mut self, key: S) -> Option<Symbol> {
        self.map.remove(key.as_ref().into())
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
pub struct SymbolsTableCaseDependent {
    table: SymbolsTable,
    case_sensitive: bool,
}

/// By default, the assembler is case sensitive
impl Default for SymbolsTableCaseDependent {
    fn default() -> SymbolsTableCaseDependent {
        SymbolsTableCaseDependent {
            table: Default::default(),
            case_sensitive: true,
        }
    }
}

impl AsRef<SymbolsTable> for SymbolsTableCaseDependent {
    fn as_ref(&self) -> &SymbolsTable {
        &self.table
    }
}

impl SymbolsTableCaseDependent {
    fn new(table: SymbolsTable, case_sensitive: bool) -> SymbolsTableCaseDependent {
        SymbolsTableCaseDependent {
            table,
            case_sensitive,
        }
    }

    /// Build a laxists vesion of the table : do not care of case and absences of symboles
    pub fn laxist() -> SymbolsTableCaseDependent {
        SymbolsTableCaseDependent::new(SymbolsTable::laxist(), false)
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

    #[deprecated(
        note = "Symbol table should be manipulated from the options. It sould be better to rewrite code."
    )]
    fn set_table(&mut self, table: SymbolsTable) {
        self.table = table
    }

    pub fn set_symbol_to_current_address<S: AsRef<str>>(
        &mut self,
        symbol: S,
    ) -> Result<(), AssemblerError> {
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

    pub fn remove_symbol<S: AsRef<str>>(&mut self, symbol: S) -> Option<Symbol> {
        self.table.remove_symbol(self.normalize_symbol(symbol))
    }

    pub fn contains_symbol<S: AsRef<str>>(&self, symbol: S) -> bool {
        self.table.contains_symbol(self.normalize_symbol(symbol))
    }

    delegate! {
        target self.table {
            pub fn current_address(&self) -> Result<u16, AssemblerError>;
            pub fn set_current_address(&mut self, address: u16);
            pub fn closest_symbol<S: AsRef<str>>(&self, symbol: S) -> Option<String>;
        }
    }
}

/// Environment of the assembly
pub struct Env {
    /// Current pass
    pass: AssemblingPass,

    /// Stable counter of nops
    stable_counters: StableTickerCounters,

    /// Start adr to use to write binary files. No use when working with snapshots.
    /// When working with binary file only 64K can be generated, not more
    startadr: Option<usize>,

    /// Current address to write to
    outputadr: usize,

    /// Current address used by the code
    codeadr: usize,

    /// Maximum possible address to write to
    maxptr: usize,

    /// Currently selected bank
    activebank: usize,

    /// Memory configuration
    /// XXX Currently we only have one bank
    mem: [Bank; 1],

    iorg: usize,
    org_zones: Vec<OrgZone>,
    symbols: SymbolsTableCaseDependent,
}
impl fmt::Debug for Env {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Env{{ pass: {:?}, symbols {:?} }}",
            self.pass,
            self.symbols()
        )
    }
}

impl Default for Env {
    fn default() -> Env {
        Env {
            pass: AssemblingPass::Uninitialized,
            stable_counters: StableTickerCounters::default(),

            startadr: None,
            outputadr: 0,
            codeadr: 0,
            maxptr: 0xffff,
            activebank: 0,
            mem: [[0; 0x10000]; 1],

            iorg: 0,
            org_zones: Vec::new(),

            symbols: Default::default(),
        }
    }
}

impl Env {
    /// Create an environment that embeds a copy of the given table and is configured to be in the latest pass.
    /// Mainly used for tests.
    pub fn with_table(symbols: &SymbolsTable) -> Env {
        let mut env = Env::default();
        env.symbols.set_table(symbols.clone());
        env.pass = AssemblingPass::SecondPass;
        env
    }

    pub fn with_table_case_dependent(symbols: &SymbolsTableCaseDependent) -> Env {
        let mut env = Env::default();
        env.symbols = symbols.clone();
        env.pass = AssemblingPass::SecondPass;
        env
    }

    /// Start a new pass by cleaning up datastructures.
    /// The only thing to keep is the symbol table
    fn start_new_pass(&mut self) {
        self.pass = self.pass.clone().next_pass();

        if !self.pass.is_finished() {
            // environnement is not reset when assembling is finished
            self.startadr = None;
            self.outputadr = 0;
            self.codeadr = 0;
            self.maxptr = 0xffff;
            self.activebank = 0;
            self.mem = [[0; 0x10000]; 1];
            self.iorg = 0;
            self.org_zones = Vec::new();
            self.stable_counters = StableTickerCounters::default();
        }
    }

    /// Return the address where the next byte will be written
    pub fn output_address(&self) -> usize {
        self.outputadr
    }

    /// Return the address of dollar
    pub fn code_address(&self) -> usize {
        self.codeadr
    }

    ///. Update the value of $ in the symbol table in order to take the current  output address
    pub fn update_dollar(&mut self) {
        self.symbols.set_current_address(self.code_address() as _);
    }

    /// Produce the memory for the required limits
    pub fn memory(&self, start: usize, size: usize) -> Vec<u8> {
        let mut mem = Vec::new();
        for pos in start..(start + size) {
            mem.push(self.mem[0][pos]); // XXX probably buggy later
        }
        mem
    }

    /// Returns the stream of bytes produces
    pub fn produced_bytes(&self) -> Vec<u8> {
        // assume we start at 0 if never provided
        let startadr = self.startadr.or(Some(0)).unwrap();
        self.memory(startadr as _, self.outputadr - startadr)
    }

    /// Returns the address of the 1st written byte
    pub fn loading_address(&self) -> Option<usize> {
        self.startadr
    }

    /// Returns the address from when to start the program
    /// TODO really configure this address
    pub fn execution_address(&self) -> Option<usize> {
        self.startadr
    }

    /// Output one byte
    /// (RASM ___internal_output)
    pub fn output(&mut self, v: u8) -> Result<(), AssemblerError> {
        if self.outputadr <= self.maxptr {
            eprintln!("==> 0x{:X} = 0x{:X}", self.outputadr, v);

            self.mem[self.activebank][self.outputadr] = v;
            self.outputadr += 1; // XXX will fail at 0xffff
            self.codeadr += 1;
            Ok(())
        } else {
            Err("Output exceeded limits".to_owned().into())
        }
    }

    /// TODO test if we will oversize the limit
    pub fn output_bytes(&mut self, bytes: &[u8]) -> Result<(), AssemblerError> {
        for b in bytes.iter() {
            self.output(*b)?;
        }
        Ok(())
    }

    pub fn byte(&self, address: usize) -> u8 {
        self.mem[self.activebank][address]
    }

    /// Get the size of the generated binary.
    /// ATTENTION it can only work when geneating 0x10000 files
    pub fn size(&self) -> u16 {
        if self.startadr.is_none() {
            panic!("Unable to compute size now");
        } else {
            (self.outputadr - self.startadr.unwrap()) as u16
        }
    }

    /// Evaluate the expression according to the current state of the environment
    pub fn eval(&self, expr: &Expr) -> Result<usize, AssemblerError> {
        if expr.is_context_independant() {
            match expr.eval() {
                Ok(val) => Ok(val as usize),
                Err(err) => Err(err),
            }
        } else {
            Err(String::from("Not yet implemented").into())
        }
    }

    pub fn symbols(&self) -> &SymbolsTableCaseDependent {
        &self.symbols
    }

    pub fn symbols_mut(&mut self) -> &mut SymbolsTableCaseDependent {
        &mut self.symbols
    }

    /// Compute the expression thanks to the symbol table of the environment.
    /// If the expression is not solvable in first pass, 0 is returned.
    /// If the expression is not solvable in second pass, an error is returned
    fn resolve_expr_may_fail_in_first_pass(&self, exp: &Expr) -> Result<i32, AssemblerError> {
        match exp.resolve(self.symbols()) {
            Ok(value) => Ok(value),
            Err(e) => {
                if self.pass.is_first_pass() {
                    Ok(0)
                } else {
                    Err(format!(
                        "Impossible to evaluate {:?} at pass {:?}. {:?}",
                        exp, self.pass, e
                    )
                    .into())
                }
            }
        }
    }

    /// Compute the expression thanks to the symbol table of the environment.
    /// An error is systematically raised if the expression is not solvable (i.e., labels are unknown)
    fn resolve_expr_must_never_fail(&self, exp: &Expr) -> Result<i32, AssemblerError> {
        match exp.resolve(self.symbols()) {
            Ok(value) => Ok(value),
            Err(e) => Err(format!(
                "Impossible to evaluate {:?} at pass {:?}. {:?}",
                exp, self.pass, e
            )
            .into()),
        }
    }

    /// Compute the relative address. Is authorized to fail at first pass
    fn absolute_to_relative_may_fail_in_first_pass(
        &self,
        address: i32,
        opcode_delta: i32,
    ) -> Result<u8, AssemblerError> {
        match absolute_to_relative(address, opcode_delta, self.symbols()) {
            Ok(value) => Ok(value),
            Err(e) => {
                if self.pass.is_first_pass() {
                    Ok(0)
                } else {
                    Err(format!(
                        "Impossible to compute relative address {:?} at pass {:?}",
                        address, e
                    )
                    .into())
                }
            }
        }
    }

    /// Add a symbol to the symbol table.
    /// In pass 1: the label MUST be absent
    /// In pass 2: the label MUST be present and of the same value
    fn add_symbol_to_symbol_table(
        &mut self,
        label: &str,
        value: i32,
    ) -> Result<(), AssemblerError> {
        let already_present = self.symbols().contains_symbol(&label.to_owned());

        match (already_present, self.pass) {
            (true, AssemblingPass::FirstPass) => Err(AssemblerError::SymbolAlreadyExists {
                symbol: label.to_string(),
            }),
            (false, AssemblingPass::SecondPass) => Err(format!(
                "Label {} is not present in the symbol table in pass {}",
                label, self.pass
            )
            .into()),
            (false, AssemblingPass::FirstPass) | (false, AssemblingPass::Uninitialized) => {
                self.symbols_mut()
                    .set_symbol_to_value(&label.to_owned(), value);
                Ok(())
            }
            (true, AssemblingPass::SecondPass) => {
                self.symbols_mut()
                    .update_symbol_to_value(&label.to_owned(), value);
                Ok(())
            }
            (_, _) => panic!(
                "add_symbol_to_symbol_table / unmanaged case {}, {}, {} {}",
                self.pass, label, already_present, value
            ),
        }
    }
}

impl Env {
    /// Visit all the tokens of the listing
    pub fn visit_listing(&mut self, listing: &Listing) -> Result<(), AssemblerError> {
        for token in listing.listing().iter() {
            visit_token(token, self)?;
        }

        Ok(())
    }

    fn visit_label(&mut self, label: &str) -> Result<(), AssemblerError> {
        // If the current address is not set up, we force it to be 0
        let value = match self.symbols().current_address() {
            Ok(address) => address,
            Err(_) => 0,
        };

        // A label cannot be defined multiple times
        if self.pass.is_first_pass() && self.symbols().contains_symbol(label) {
            Err(format!("Symbol \"{}\" already present in symbols table", label).into())
        } else {
            self.add_symbol_to_symbol_table(label, value as _)
        }
    }

    /// Manage a IF .. XXX ELSEIF YYY ELSE ZZZ structure
    fn visit_if(
        &mut self,
        cases: &Vec<(TestKind, Listing)>,
        other: Option<&Listing>,
    ) -> Result<(), AssemblerError> {
        assert!(cases.len() > 0);

        // Test all the if cases until reaching one != 0
        for case in cases.iter() {
            match case {
                // Expression must be true
                (TestKind::True(ref exp), ref listing) => {
                    let value = self.resolve_expr_must_never_fail(exp)?;
                    if value != 0 {
                        self.visit_listing(listing)?;
                        return Ok(());
                    }
                },

                // Expression must be false
                (TestKind::False(ref exp), ref listing) => {
                    let value = self.resolve_expr_must_never_fail(exp)?;
                    if value == 0 {
                        self.visit_listing(listing)?;
                        return Ok(());
                    }
                },

                // Label must exist
                (TestKind::LabelExists(ref label), ref listing) => {
                    if self.symbols().contains_symbol(label) {
                        self.visit_listing(listing)?;
                        return Ok(());
                    }
                },

                // Label must not exist
                (TestKind::LabelDoesNotExist(ref label), ref listing) => {
                    if !self.symbols().contains_symbol(label) {
                        self.visit_listing(listing)?;
                        return Ok(());
                    }
                }
            }
        }

        // Test the else if any
        match other {
            Some(listing) => self.visit_listing(listing),
            None => Ok(()),
        }
    }

    /// Remove the given variable from the table of symbols
    pub fn visit_undef(&mut self, label: &str) -> Result<(), AssemblerError> {
        match self.symbols_mut().remove_symbol(label) {
            Some(_) => Ok(()),
            None => Err(format!("Unknown symbol `{}`", label).into()),
        }
    }

    /// Print the evaluation of the expression in the 2nd pass
    pub fn visit_print(&self, info: Either<&Expr, &String>) -> Result<(), AssemblerError> {
        if self.pass.is_second_pass() {
            let text = match info {
                Left(ref exp) => format!("{} = {}", exp, self.resolve_expr_must_never_fail(exp)?),
                Right(string) => string.clone(),
            };
            println!("{}", text);
        }
        Ok(())
    }

    /// Continue to assemble at the right place, but change the value of $ to the specified one
    pub fn visit_rorg(&mut self, _exp: &Expr, listing: &Listing) -> Result<(), AssemblerError> {
        let backup = self.codeadr;
        self.visit_listing(listing)?;
        self.codeadr = backup;
        Ok(())
    }

    pub fn visit_incbin(&mut self, data: &[u8]) -> Result<(), AssemblerError> {
        self.output_bytes(data)
    }
}

/// Visit the tokens during several passes without providing a specific symbol table.
pub fn visit_tokens_all_passes(tokens: &[Token]) -> Result<Env, AssemblerError> {
    let options = AssemblingOptions::default();
    visit_tokens_all_passes_with_options(tokens, &options)
}

/// Visit the tokens during several passes by providing a specific symbol table.
pub fn visit_tokens_all_passes_with_options(
    tokens: &[Token],
    options: &AssemblingOptions,
) -> Result<Env, AssemblerError> {
    let mut env = Env::default();
    env.symbols =
        SymbolsTableCaseDependent::new(options.symbols().clone(), options.case_sensitive());

    loop {
        env.start_new_pass();

        if env.pass.is_finished() {
            break;
        }

        for token in tokens.iter() {
            visit_token(token, &mut env)?;
        }
    }

    Ok(env)
}

/// Visit the tokens during a single pass. Is deprecated in favor to the mulitpass version
#[deprecated]
pub fn visit_tokens(tokens: &[Token]) -> Result<Env, AssemblerError> {
    let mut env = Env::default();

    for token in tokens.iter() {
        visit_token(token, &mut env)?;
    }

    Ok(env)
}

/// TODO org is a directive, not an opcode => need to change that
pub fn visit_token(token: &Token, env: &mut Env) -> Result<(), AssemblerError> {
    env.update_dollar();
    match token {
        Token::Assert(ref exp, ref txt) => visit_assert(exp, txt.as_ref(), env),
        Token::Basic(ref variables, ref hidden_lines, ref code) => {
            env.visit_basic(variables.as_ref(), hidden_lines.as_ref(), code)
        }
        Token::Org(ref address, ref address2) => visit_org(address, address2.as_ref(), env),
        Token::Defb(_) | &Token::Defw(_) => visit_db_or_dw(token, env),
        Token::Defs(_, _) => visit_defs(token, env),
        Token::OpCode(ref mnemonic, ref arg1, ref arg2) => {
            visit_opcode(&mnemonic, &arg1, &arg2, env)?;
            // Compute duration only if it is necessary
            if !env.stable_counters.is_empty() {
                let duration = token.estimated_duration()?;
                env.stable_counters.update_counters(duration);
            }
            Ok(())
        }
        Token::Comment(_) => Ok(()), // Nothing to do for a comment
        Token::Include(_, Some(ref listing)) => env.visit_listing(listing),
        Token::Incbin(_, _, _, _, _, ref data) => env.visit_incbin(data.as_ref().unwrap()),
        Token::If(ref cases, ref other) => env.visit_if(cases, other.as_ref()),
        Token::Label(ref label) => env.visit_label(label),
        Token::Equ(ref label, ref exp) => visit_equ(label, exp, env),
        Token::Print(ref exp) => env.visit_print(exp.as_ref()),
        Token::Repeat(_, _, _) => visit_repeat(token, env),
        Token::Rorg(ref exp, ref code) => env.visit_rorg(exp, code),
        Token::StableTicker(ref ticker) => visit_stableticker(ticker, env),
        Token::Undef(ref label) => env.visit_undef(label),
        // ignored tokens
        Token::List | Token::NoList => Ok(()),
        _ => panic!("Not treated {:?}", token),
    }
}

fn visit_assert(exp: &Expr, txt: Option<&String>, env: &mut Env) -> Result<(), AssemblerError> {
    if env.pass.is_second_pass() {
        let value = env.resolve_expr_must_never_fail(exp)?;
        if value == 0 {
            return Err(AssemblerError::AssertionFailed {
                msg: (if txt.is_some() { &txt.unwrap() } else { "" }).to_owned(),
                test: exp.to_string(),
            });
        }
    }
    Ok(())
}

fn visit_equ(label: &str, exp: &Expr, env: &mut Env) -> Result<(), AssemblerError> {
    if env.symbols().contains_symbol(label) && env.pass.is_first_pass() {
        Err(format!("Symbol \"{}\" already present in symbols table", label).into())
    } else {
        let value = env.resolve_expr_may_fail_in_first_pass(exp)?;
        env.add_symbol_to_symbol_table(label, value)
    }
}

fn visit_defs(token: &Token, env: &mut Env) -> Result<(), AssemblerError> {
    match token {
        Token::Defs(expr, fill) => {
            let bytes = assemble_defs(expr, fill.as_ref(), env)?;
            env.output_bytes(&bytes)
        }
        _ => unreachable!(),
    }
}

// TODO refactor code with assemble_opcode or other functions manipulating bytes
fn visit_db_or_dw(token: &Token, env: &mut Env) -> Result<(), AssemblerError> {
    let bytes = assemble_db_or_dw(token, env)?;
    env.output_bytes(&bytes)
}

impl Env {
    pub fn visit_basic(
        &mut self,
        variables: Option<&Vec<String>>,
        hidden_lines: Option<&Vec<u16>>,
        code: &str,
    ) -> Result<(), AssemblerError> {
        let bytes = self.assemble_basic(variables, hidden_lines, code)?;

        // If the basic directive is the VERY first thing to output,
        // we assume startadr is 0x170 as for any basic program
        if self.startadr.is_none() {
            self.outputadr = 0x170;
            self.codeadr = self.outputadr;
            self.startadr = Some(self.outputadr);
        }

        self.output_bytes(&bytes)
    }

    pub fn assemble_basic(
        &mut self,
        variables: Option<&Vec<String>>,
        hidden_lines: Option<&Vec<u16>>,
        code: &str,
    ) -> Result<Vec<u8>, AssemblerError> {
        // Build the final basic code by replacing variables by value
        // Hexadecimal is used to ensure a consistent 2 bytes representation
        let basic_src = dbg!({
            let mut basic = code.to_owned();
            match variables {
                None => {}
                Some(arguments) => {
                    for argument in arguments {
                        let key = format!("{{{}}}", argument);
                        let value = format!(
                            "&{:X}",
                            self.resolve_expr_may_fail_in_first_pass(&Expr::from(
                                argument.as_ref()
                            ))?
                        );
                        basic = basic.replace(&key, &value);
                    }
                }
            }
            basic
        });

        // build the basic tokens
        let mut basic = BasicProgram::parse(basic_src)?;
        if hidden_lines.is_some() {
            basic.hide_lines(hidden_lines.unwrap());
        }
        Ok(basic.as_bytes())
    }
}

/// When visiting a repetition, we unroll the loop and stream the tokens
pub fn visit_repeat(rept: &Token, env: &mut Env) -> Result<(), AssemblerError> {
    let tokens = rept.unroll(env.symbols()).unwrap()?;

    for token in tokens.iter() {
        visit_token(token, env)?;
    }

    Ok(())
}

/// Manage the stable ticker stuff.
/// - Start: register a counter
/// - Stop: store counter count
pub fn visit_stableticker(
    ticker: &StableTickerAction,
    env: &mut Env,
) -> Result<(), AssemblerError> {
    match ticker {
        StableTickerAction::Start(ref name) => {
            env.stable_counters.add_counter(name)?;
            Ok(())
        }
        StableTickerAction::Stop => match env.stable_counters.release_last_counter() {
            None => Err(format!("No active counter.").into()),
            Some((label, count)) => env.add_symbol_to_symbol_table(&label, count as _),
        },
    }
}

pub fn assemble_defs(expr: &Expr, fill: Option<&Expr>, env: &Env) -> Result<Bytes, AssemblerError> {
    let count = env.resolve_expr_must_never_fail(expr)?;
    let mut bytes = Bytes::with_capacity(count as usize);
    let value = if fill.is_none() {
        0
    } else {
        let value = env.resolve_expr_may_fail_in_first_pass(fill.unwrap())?;
        (value & 0xff) as u8
    };

    for _i in 0..count {
        bytes.push(value);
    }

    Ok(bytes)
}

pub fn assemble_db_or_dw(token: &Token, env: &Env) -> Result<Bytes, AssemblerError> {
    let mut bytes = Bytes::new();

    let (ref exprs, mask) = {
        match token {
            &Token::Defb(ref exprs) => (exprs, 0xff),
            &Token::Defw(ref exprs) => (exprs, 0xffff),
            _ => unreachable!(),
        }
    };

    for exp in exprs.iter() {
        let val = env.resolve_expr_may_fail_in_first_pass(exp)? & mask;
        if mask == 0xff {
            add_byte(&mut bytes, val as u8);
        } else {
            add_word(&mut bytes, val as u16);
        }
    }

    Ok(bytes)
}

/// Assemble align directive. It can only work if current address is known...
pub fn assemble_align(
    expr: &Expr,
    fill: Option<&Expr>,
    env: &Env,
) -> Result<Bytes, AssemblerError> {
    let expression = env.resolve_expr_must_never_fail(expr)? as u16;
    let current = env.symbols().current_address()?;
    let value = if fill.is_none() {
        0
    } else {
        let value = env.resolve_expr_may_fail_in_first_pass(fill.unwrap())?;
        (value & 0xff) as u8
    };

    // compute the number of 0 to put
    let mut until = current;
    while until % expression != 0 {
        until += 1;
    }

    // Create the vector
    let hole = (until - current) as usize;
    let mut bytes = Bytes::with_capacity(hole);
    for _i in 0..hole {
        bytes.push(value);
    }

    // and return it
    Ok(bytes)
}

/// Assemble the opcode and inject in the environement
fn visit_opcode(
    mnemonic: &Mnemonic,
    arg1: &Option<DataAccess>,
    arg2: &Option<DataAccess>,
    env: &mut Env,
) -> Result<(), AssemblerError> {
    // TODO update $ in the symbol table
    let bytes = assemble_opcode(mnemonic, arg1, arg2, env)?;
    for b in bytes.iter() {
        env.output(*b)?;
    }

    Ok(())
}

/// Assemble an opcode and returns the generated bytes or the error message if it is impossible to
/// assemblea.
/// We assum the opcode is properlt coded. Panic occurs if it is not the case
pub fn assemble_opcode(
    mnemonic: &Mnemonic,
    arg1: &Option<DataAccess>,
    arg2: &Option<DataAccess>,
    env: &mut Env,
) -> Result<Bytes, AssemblerError> {
    let sym = env.symbols_mut();
    // TODO use env instead of the symbol table for each call
    match mnemonic {
        Mnemonic::And | Mnemonic::Or | Mnemonic::Xor => {
            assemble_logical_operator(mnemonic, arg1.as_ref().unwrap(), sym)
        }
        Mnemonic::Add | Mnemonic::Adc => assemble_add_or_adc(
            mnemonic,
            arg1.as_ref().unwrap(),
            arg2.as_ref().unwrap(),
            sym,
        ),
        Mnemonic::Bit => env.assemble_bit(
            arg1.as_ref().unwrap().expr().unwrap(),
            arg2.as_ref().unwrap(),
        ),
        Mnemonic::Cp => env.assemble_cp(arg1.as_ref().unwrap()),

        Mnemonic::Dec | Mnemonic::Inc => assemble_inc_dec(mnemonic, arg1.as_ref().unwrap()),
        Mnemonic::Djnz => assemble_djnz(arg1.as_ref().unwrap(), env),
        Mnemonic::In => assemble_in(arg1.as_ref().unwrap(), &arg2.as_ref().unwrap(), sym),
        &Mnemonic::Ld => assemble_ld(arg1.as_ref().unwrap(), &arg2.as_ref().unwrap(), env),
        Mnemonic::Ldi
        | Mnemonic::Ldd
        | Mnemonic::Ldir
        | Mnemonic::Lddr
        | Mnemonic::Outi
        | Mnemonic::Outd
        | Mnemonic::Ei
        | Mnemonic::Di
        | Mnemonic::ExAf
        | Mnemonic::Exx
        | Mnemonic::Halt
        | Mnemonic::Ind
        | Mnemonic::Indr
        | Mnemonic::Ini
        | Mnemonic::Inir
        | Mnemonic::Rra
        | Mnemonic::Scf => assemble_no_arg(mnemonic),
        &Mnemonic::Nop => assemble_nop(),
        &Mnemonic::Nops2 => assemble_nops2(),
        &Mnemonic::Out => assemble_out(arg1.as_ref().unwrap(), &arg2.as_ref().unwrap(), sym),
        Mnemonic::Jr | Mnemonic::Jp | Mnemonic::Call => {
            assemble_call_jr_or_jp(mnemonic, arg1, &arg2.as_ref().unwrap(), env)
        }
        &Mnemonic::Pop => assemble_pop(arg1.as_ref().unwrap()),
        &Mnemonic::Push => assemble_push(arg1.as_ref().unwrap()),
        &Mnemonic::Res | &Mnemonic::Set => assemble_res_or_set(
            mnemonic,
            arg1.as_ref().unwrap(),
            arg2.as_ref().unwrap(),
            sym,
        ),
        &Mnemonic::Ret => assemble_ret(arg1),

        Mnemonic::Sla | Mnemonic::Sra | Mnemonic::Srl => {
            env.assemble_shift(mnemonic, arg1.as_ref().unwrap())
        }
        _ => Err(format!("Unable to assemble opcode {:?}", mnemonic).into()),
    }
}

fn visit_org(address: &Expr, address2: Option<&Expr>, env: &mut Env) -> Result<(), AssemblerError> {
    if address2.is_some() {
        unimplemented!();
    }

    let adr = env.eval(address)?;

    // TODO Check overlapping region
    // TODO manage rorg like instruction
    env.outputadr = adr as usize;
    env.codeadr = adr as usize;

    // Specify start address at first use
    if env.startadr.is_none() {
        env.startadr = Some(env.outputadr as usize);
    }

    Ok(())
}

fn assemble_no_arg(mnemonic: &Mnemonic) -> Result<Bytes, AssemblerError> {
    let bytes: &[u8] = match mnemonic {
        Mnemonic::Ldi => &[0xED, 0xA0],
        Mnemonic::Ldd => &[0xED, 0xA8],
        Mnemonic::Lddr => &[0xED, 0xB8],
        Mnemonic::Ldir => &[0xED, 0xB0],
        Mnemonic::Di => &[0xF3],
        Mnemonic::ExAf => &[0x08],
        Mnemonic::Exx => &[0xD9],
        Mnemonic::Ei => &[0xFB],
        Mnemonic::Halt => &[0x76],
        Mnemonic::Ind => &[0xED, 0xAA],
        &Mnemonic::Indr => &[0xED, 0xBA],
        &Mnemonic::Ini => &[0xED, 0xA2],
        &Mnemonic::Inir => &[0xED, 0xB2],
        &Mnemonic::Outd => &[0xED, 0xAB],
        Mnemonic::Outi => &[0xED, 0xA3],
        Mnemonic::Rra => &[0x1f],
        Mnemonic::Scf => &[0x37],
        _ => {
            return Err(format!("{} not treated", mnemonic).into());
        }
    };

    Ok(Bytes::from_slice(bytes))
}

fn assemble_inc_dec(mne: &Mnemonic, arg1: &DataAccess) -> Result<Bytes, AssemblerError> {
    let mut bytes = Bytes::new();

    let is_inc = match mne {
        &Mnemonic::Inc => true,
        &Mnemonic::Dec => false,
        _ => panic!("Impossible case"),
    };

    match arg1 {
        &DataAccess::Register16(ref reg) => {
            let base = if is_inc { 0b00000011 } else { 0b00001011 };
            let byte = base | (register16_to_code_with_sp(reg) << 4);
            bytes.push(byte);
        }

        &DataAccess::IndexRegister16(ref reg) => {
            bytes.push(indexed_register16_to_code(reg));
            bytes.push(if is_inc { 0x23 } else { 0x2b });
        }

        &DataAccess::Register8(ref reg) => {
            bytes
                .push(if is_inc { 0b00000100 } else { 0b00000101 } | (register8_to_code(reg) << 3));
        }
        _ => {
            return Err(format!("Inc/dec not implemented for {:?}", arg1).into());
        }
    }

    Ok(bytes)
}

/// Converts an absolute address to a relative one (relative to $)
pub fn absolute_to_relative<T: AsRef<SymbolsTable>>(
    address: i32,
    opcode_delta: i32,
    sym: T,
) -> Result<u8, AssemblerError> {
    match sym.as_ref().current_address() {
        Err(msg) => Err(format!("Unable to compute the relative address {:?}", msg).into()),
        Ok(root) => {
            let delta = (address - (root as i32)) - opcode_delta;
            if delta > 128 || delta < -127 {
                Err(format!(
                    "Address 0x{:x} relative to 0x{:x} is too far {}",
                    address, root, delta
                )
                .into())
            } else {
                let res = (delta & 0xff) as u8;
                Ok(res)
            }
        }
    }
}

fn assemble_ret(arg1: &Option<DataAccess>) -> Result<Bytes, AssemblerError> {
    let mut bytes = Bytes::new();

    if arg1.is_some() {
        match arg1.as_ref() {
            Some(&DataAccess::FlagTest(ref test)) => {
                let flag = flag_test_to_code(test);
                bytes.push(0b11000000 | (flag << 3));
            }
            _ => {
                return Err(format!("Wrong argument for ret {:?}", arg1).into());
            }
        };
    } else {
        bytes.push(0xc9);
    };

    Ok(bytes)
}

/// arg1 contains the tests
/// arg2 contains the information
fn assemble_call_jr_or_jp(
    mne: &Mnemonic,
    arg1: &Option<DataAccess>,
    arg2: &DataAccess,
    env: &Env,
) -> Result<Bytes, AssemblerError> {
    let mut bytes = Bytes::new();

    let is_jr = match mne {
        Mnemonic::Jr => true,
        Mnemonic::Jp | Mnemonic::Call => false,
        _ => unreachable!(),
    };

    let is_call = match mne {
        Mnemonic::Call => true,
        Mnemonic::Jp | Mnemonic::Jr => false,
        _ => unreachable!(),
    };

    // compute the flag code if any
    // TODO raise an error if the flag test for jr is wrong
    let flag_code = if arg1.is_some() {
        match arg1.as_ref() {
            Some(&DataAccess::FlagTest(ref test)) => Some(flag_test_to_code(test)),
            _ => return Err(format!("Wrong flag argument {:?}", arg1).into()),
        }
    } else {
        None
    };

    // Treat address
    match arg2 {
        &DataAccess::Expression(ref e) => {
            let address = env.resolve_expr_may_fail_in_first_pass(e)?;
            if is_jr {
                let relative = env.absolute_to_relative_may_fail_in_first_pass(address, 2)?;
                if flag_code.is_some() {
                    // jr - flag
                    add_byte(&mut bytes, 0b00100000 | (flag_code.unwrap() << 3));
                } else {
                    // jr - no flag
                    add_byte(&mut bytes, 0b00011000);
                }
                add_byte(&mut bytes, relative);
            } else if is_call {
                match flag_code {
                    Some(flag) => add_byte(&mut bytes, 0b11000100 | (flag << 3)),
                    None => add_byte(&mut bytes, 0xCD),
                }
                add_word(&mut bytes, address as u16);
            } else {
                if flag_code.is_some() {
                    // jp - flag
                    add_byte(&mut bytes, 0b11000010 | (flag_code.unwrap() << 3))
                } else {
                    // jp - no flag
                    add_byte(&mut bytes, 0xc3);
                }
                add_word(&mut bytes, address as u16);
            }
        }
        _ => {
            return Err(format!("Parameter {:?} not treated", arg2).into());
        }
    };

    Ok(bytes)
}

fn assemble_djnz(arg1: &DataAccess, env: &Env) -> Result<Bytes, AssemblerError> {
    if let &DataAccess::Expression(ref expr) = arg1 {
        let mut bytes = Bytes::new();
        let address = env.resolve_expr_may_fail_in_first_pass(expr)?;
        let relative = env.absolute_to_relative_may_fail_in_first_pass(address, 1)?;

        bytes.push(0x10);
        bytes.push(relative);

        return Ok(bytes);
    } else {
        return Err("DJNZ must be followed by an expression".to_owned().into());
    }
}

impl Env {
    pub fn assemble_cp(&mut self, arg: &DataAccess) -> Result<Bytes, AssemblerError> {
        let mut bytes = Bytes::new();

        match arg {
            DataAccess::Register8(ref reg) => {
                add_byte(&mut bytes, 0b10111000 + register8_to_code(reg));
            }

            DataAccess::Expression(ref exp) => {
                add_byte(&mut bytes, 0xfe);
                add_byte(
                    &mut bytes,
                    self.resolve_expr_may_fail_in_first_pass(exp)? as _,
                );
            }

            DataAccess::MemoryRegister16(Register16::Hl) => {
                add_byte(&mut bytes, 0xbe);
            }

            DataAccess::IndexRegister16WithIndex(ref reg, ref idx) => {
                add_byte(&mut bytes, indexed_register16_to_code(reg));
                add_byte(&mut bytes, 0xbe);
                add_word(
                    &mut bytes,
                    self.resolve_expr_may_fail_in_first_pass(idx)? as _,
                );
            }

            _ => unreachable!(),
        }

        Ok(bytes)
    }

    pub fn assemble_bit(
        &mut self,
        arg1: &Expr,
        arg2: &DataAccess,
    ) -> Result<Bytes, AssemblerError> {
        let mut bytes = Bytes::new();

        let bit = (self.resolve_expr_may_fail_in_first_pass(arg1)? & 0xff) as u8;
        if bit > 7 {
            return Err(format!("BIT {}, {} is not possible", bit, arg2).into());
        }

        match arg2 {
            DataAccess::MemoryRegister16(Register16::Hl) => {
                add_byte(&mut bytes, 0xcb);
                add_byte(&mut bytes, 0b01000110 + bit << 3);
            }

            DataAccess::IndexRegister16WithIndex(_, _) => unimplemented!(),

            DataAccess::Register8(ref reg) => {
                add_byte(&mut bytes, 0xcb);
                add_byte(&mut bytes, 0b01000000 + (bit << 3) + register8_to_code(reg))
            }

            _ => unimplemented!("aseemble_bit {:?} {:?}", arg1, arg2),
        }

        Ok(bytes)
    }

    pub fn assemble_shift(
        &mut self,
        mne: &Mnemonic,
        target: &DataAccess,
    ) -> Result<Bytes, AssemblerError> {
        let mut bytes = Bytes::new();

        match target {
            DataAccess::Register8(ref reg) => {
                add_byte(&mut bytes, 0xcb);
                if mne.is_sla() {
                    add_byte(&mut bytes, 0b00100000 + register8_to_code(reg));
                } else if mne.is_sra() {
                    add_byte(&mut bytes, 0b00101000 + register8_to_code(reg));
                } else if mne.is_srl() {
                    add_byte(&mut bytes, 0b00111000 + register8_to_code(reg));
                } else {
                    unreachable!()
                }
            }

            DataAccess::MemoryRegister16(Register16::Hl) => {
                add_byte(&mut bytes, 0xcb);
                if mne.is_sla() {
                    add_byte(&mut bytes, 0x26);
                } else if mne.is_sra() {
                    add_byte(&mut bytes, 0x2e);
                } else if mne.is_srl() {
                    add_byte(&mut bytes, 0x3e);
                } else {
                    unreachable!()
                }
            }

            DataAccess::IndexRegister16WithIndex(ref reg, ref exp) => {
                let val = self.resolve_expr_may_fail_in_first_pass(exp)? as i32;
                bytes.push(indexed_register16_to_code(reg));
                add_byte(&mut bytes, 0xcb);
                bytes.push((val & 0xff) as _);
                if mne.is_sla() {
                    add_byte(&mut bytes, 0x26);
                } else if mne.is_sra() {
                    add_byte(&mut bytes, 0x2e);
                } else if mne.is_srl() {
                    add_byte(&mut bytes, 0x3e);
                } else {
                    unreachable!()
                }
            }

            _ => unreachable!(),
        }

        Ok(bytes)
    }
}

fn assemble_ld(arg1: &DataAccess, arg2: &DataAccess, env: &Env) -> Result<Bytes, AssemblerError> {
    let mut bytes = Bytes::new();

    // Destination is 8bits register
    if let &DataAccess::Register8(ref dst) = arg1 {
        let dst = register8_to_code(dst);
        match arg2 {
            &DataAccess::Register8(ref src) => {
                //R. Zaks p 297
                let src = register8_to_code(src);

                let code = 0b01000000 + (dst << 3) + src;
                bytes.push(code);
            }

            &DataAccess::Expression(ref exp) => {
                let val = (env.resolve_expr_may_fail_in_first_pass(exp)? & 0xff) as u8;

                bytes.push(0b00000110 | (dst << 3));
                bytes.push(val);
            }

            &DataAccess::IndexRegister16WithIndex(ref reg, ref exp) => {
                let val = env.resolve_expr_may_fail_in_first_pass(exp)?;

                add_index_register_code(&mut bytes, reg);
                add_byte(&mut bytes, 0b01000110 | (dst << 3));
                add_index(&mut bytes, val)?;
            }

            &DataAccess::MemoryRegister16(Register16::Hl) => {
                add_byte(&mut bytes, 0b01000110 | (dst << 3));
            }

            &DataAccess::Memory(ref expr) => {
                // dst is A
                let val = env.resolve_expr_may_fail_in_first_pass(expr)?;
                add_byte(&mut bytes, 0x3a);
                add_word(&mut bytes, val as _);
            }

            _ => {
                return Err(
                    format!("LD not properly implemented for '{:?}, {:?}'", arg1, arg2).into(),
                );
            }
        }
    }
    // Destination is 16 bits register
    else if let &DataAccess::Register16(ref dst) = arg1 {
        let dst_code = register16_to_code_with_sp(dst);

        match arg2 {
            &DataAccess::Expression(ref exp) => {
                let val = (env.resolve_expr_may_fail_in_first_pass(exp)? & 0xffff) as u16;

                add_byte(&mut bytes, 0b00000001 | (dst_code << 4));
                add_word(&mut bytes, val);
            }

            // Fake instruction splitted in 2 bits operations
            &DataAccess::Register16(ref src) => {
                println!("{:?}, {:?}", dst.split(), src.split());
                let bytes_high = assemble_ld(
                    &DataAccess::Register8(dst.high().unwrap()),
                    &DataAccess::Register8(src.high().unwrap()),
                    env,
                )
                .unwrap();
                let bytes_low = assemble_ld(
                    &DataAccess::Register8(dst.low().unwrap()),
                    &DataAccess::Register8(src.low().unwrap()),
                    env,
                )
                .unwrap();

                bytes.extend_from_slice(&bytes_low);
                bytes.extend_from_slice(&bytes_high);
            }

            DataAccess::Memory(ref expr) => {
                let val = (env.resolve_expr_may_fail_in_first_pass(expr)? & 0xffff) as u16;

                if let Register16::Hl = dst {
                    add_byte(&mut bytes, 0x2a);
                    add_word(&mut bytes, val);
                } else {
                    add_byte(&mut bytes, 0xED);
                    add_byte(
                        &mut bytes,
                        (register16_to_code_with_sp(dst) << 4) + 0b01001011,
                    );
                    add_word(&mut bytes, val);
                }
            }

            _ => {}
        }
    }
    // Distinatin is 16 bits indexed register
    else if let &DataAccess::IndexRegister16(ref dst) = arg1 {
        let code = indexed_register16_to_code(dst);

        match arg2 {
            &DataAccess::Expression(ref exp) => {
                let val = (env.resolve_expr_may_fail_in_first_pass(exp)? & 0xffff) as u16;

                add_byte(&mut bytes, code);
                add_byte(&mut bytes, 0x21);
                add_word(&mut bytes, val);
            }

            &DataAccess::Memory(ref exp) => {
                let val = (env.resolve_expr_may_fail_in_first_pass(exp)? & 0xffff) as u16;

                add_byte(&mut bytes, code);
                add_byte(&mut bytes, 0x2a);
                add_word(&mut bytes, val);
            }
            _ => {}
        }
    }
    // Destination is memory indexed by register
    else if let &DataAccess::MemoryRegister16(ref dst) = arg1 {
        // Want to store in memory pointed by register
        match dst {
            &Register16::Hl => {
                if let &DataAccess::Register8(ref src) = arg2 {
                    let src = register8_to_code(src);
                    let code = 0b01110000 | src;
                    bytes.push(code);
                }
            }

            &Register16::De if &DataAccess::Register8(Register8::A) == arg2 => {
                bytes.push(0b00010010);
            }

            &Register16::Bc if &DataAccess::Register8(Register8::A) == arg2 => {
                bytes.push(0b00000010);
            }

            _ => {}
        }
    }
    // Destination is memory
    else if let &DataAccess::Memory(ref exp) = arg1 {
        let address = env.resolve_expr_may_fail_in_first_pass(exp)?;

        match arg2 {
            &DataAccess::IndexRegister16(IndexRegister16::Ix) => {
                bytes.push(0xdd);
                bytes.push(0b00100010);
                add_word(&mut bytes, address as _);
            }
            &DataAccess::IndexRegister16(IndexRegister16::Iy) => {
                bytes.push(0xfd);
                bytes.push(0b00100010);
                add_word(&mut bytes, address as _);
            }
            &DataAccess::Register16(Register16::Hl) => {
                bytes.push(0b00100010);
                add_word(&mut bytes, address as _);
            }
            &DataAccess::Register16(ref reg) => {
                bytes.push(0xED);
                bytes.push(0b01000011 | (register16_to_code_with_sp(reg)));
                add_word(&mut bytes, address as _);
            }
            &DataAccess::Register8(Register8::A) => {
                bytes.push(0x32);
                add_word(&mut bytes, address as _);
            }

            _ => {}
        }
    }

    if bytes.len() == 0 {
        Err(format!("LD not properly implemented for '{:?}, {:?}'", arg1, arg2).into())
    } else {
        Ok(bytes)
    }
}

fn assemble_nop() -> Result<Bytes, AssemblerError> {
    let mut bytes = Bytes::new();
    bytes.push(0);
    Ok(bytes)
}

fn assemble_nops2() -> Result<Bytes, AssemblerError> {
    let mut bytes = Bytes::new();
    bytes.push(0xed);
    bytes.push(0xff);
    Ok(bytes)
}

fn assemble_in(
    arg1: &DataAccess,
    arg2: &DataAccess,
    sym: &SymbolsTableCaseDependent,
) -> Result<Bytes, AssemblerError> {
    let mut bytes = Bytes::new();

    match arg1 {
        &DataAccess::Register8(Register8::C) => match arg2 {
            &DataAccess::Register8(ref reg) => {
                bytes.push(0xED);
                bytes.push(0b01000000 | (register8_to_code(reg) << 3))
            }
            _ => panic!(),
        },

        &DataAccess::Memory(ref exp) => {
            if let &DataAccess::Register8(Register8::A) = arg2 {
                let val = (exp.resolve(sym)? & 0xff) as u8;
                bytes.push(0xDB);
                bytes.push(val);
            }
        }
        _ => panic!(),
    };

    if bytes.len() == 0 {
        Err(format!("IN not properly implemented for '{:?}, {:?}'", arg1, arg2).into())
    } else {
        Ok(bytes)
    }
}

fn assemble_out(
    arg1: &DataAccess,
    arg2: &DataAccess,
    sym: &SymbolsTableCaseDependent,
) -> Result<Bytes, AssemblerError> {
    let mut bytes = Bytes::new();

    match arg1 {
        &DataAccess::Register8(Register8::C) => match arg2 {
            &DataAccess::Register8(ref reg) => {
                bytes.push(0xED);
                bytes.push(0b01000001 | (register8_to_code(reg) << 3))
            }
            _ => {}
        },

        &DataAccess::Memory(ref exp) => {
            if let &DataAccess::Register8(Register8::A) = arg2 {
                let val = (exp.resolve(sym)? & 0xff) as u8;
                bytes.push(0xED);
                bytes.push(val);
            }
        }
        _ => {}
    };

    if bytes.len() == 0 {
        Err(format!("OUT not properly implemented for '{:?}, {:?}'", arg1, arg2).into())
    } else {
        Ok(bytes)
    }
}

fn assemble_pop(arg1: &DataAccess) -> Result<Bytes, AssemblerError> {
    let mut bytes = Bytes::new();

    match arg1 {
        &DataAccess::Register16(ref reg) => {
            let byte = 0b11000001 | (register16_to_code_with_af(reg) << 4);
            bytes.push(byte);
        }
        &DataAccess::IndexRegister16(ref reg) => {
            bytes.push(indexed_register16_to_code(reg));
            bytes.push(0xe1);
        }
        _ => {
            return Err(format!("Pop not implemented for {:?}", arg1).into());
        }
    }

    Ok(bytes)
}

fn assemble_push(arg1: &DataAccess) -> Result<Bytes, AssemblerError> {
    let mut bytes = Bytes::new();

    match arg1 {
        &DataAccess::Register16(ref reg) => {
            let byte = 0b11000101 | (register16_to_code_with_af(reg) << 4);
            bytes.push(byte);
        }
        &DataAccess::IndexRegister16(ref reg) => {
            bytes.push(indexed_register16_to_code(reg));
            bytes.push(0xe5);
        }
        _ => {
            return Err(format!("Pop not implemented for {:?}", arg1).into());
        }
    }

    Ok(bytes)
}

fn assemble_logical_operator(
    mnemonic: &Mnemonic,
    arg1: &DataAccess,
    sym: &SymbolsTableCaseDependent,
) -> Result<Bytes, AssemblerError> {
    let mut bytes = Bytes::new();

    let memory_code = || match mnemonic {
        Mnemonic::And => 0xA6,
        Mnemonic::Or => 0xB6,
        Mnemonic::Xor => 0xAE,
        _ => unreachable!(),
    };

    match *arg1 {
        DataAccess::Register8(ref reg) => {
            let base = match mnemonic {
                Mnemonic::And => 0b10100000,
                Mnemonic::Or => 0b10110000,
                Mnemonic::Xor => 0b10101000,
                _ => unreachable!(),
            };
            bytes.push(base + register8_to_code(reg));
        }

        DataAccess::Expression(ref exp) => {
            let base = match mnemonic {
                Mnemonic::And => 0xE6,
                Mnemonic::Or => 0xF6,
                Mnemonic::Xor => 0xAE,
                _ => unreachable!(),
            };
            let value = exp.resolve(sym)? & 0xff;
            bytes.push(base);
            bytes.push(value as u8);
        }

        DataAccess::MemoryRegister16(Register16::Hl) => {
            bytes.push(memory_code());
        }

        DataAccess::IndexRegister16WithIndex(ref reg, ref exp) => {
            let value = exp.resolve(sym)? & 0xff;
            bytes.push(indexed_register16_to_code(reg));
            bytes.push(memory_code());
            bytes.push(value as u8);
        }
        _ => unreachable!(),
    }

    Ok(bytes)
}

fn assemble_add_or_adc(
    mnemonic: &Mnemonic,
    arg1: &DataAccess,
    arg2: &DataAccess,
    sym: &SymbolsTableCaseDependent,
) -> Result<Bytes, AssemblerError> {
    let mut bytes = Bytes::new();
    let is_add = match mnemonic {
        &Mnemonic::Add => true,
        &Mnemonic::Adc => false,
        _ => panic!("Impossible case"),
    };

    match arg1 {
        &DataAccess::Register8(Register8::A) => {
            match arg2 {
                &DataAccess::MemoryRegister16(Register16::Hl) => {
                    if is_add {
                        bytes.push(0b10000110);
                    } else {
                        bytes.push(0b10001110);
                    }
                }

                &DataAccess::IndexRegister16WithIndex(ref reg, ref exp) => {
                    let val = exp.resolve(sym)? as i32;

                    // TODO check if the code is ok
                    bytes.push(indexed_register16_to_code(reg));
                    bytes.push(0b10000110);
                    add_index(&mut bytes, val)?;
                }

                &DataAccess::Expression(ref exp) => {
                    let val = exp.resolve(sym)? as u8;
                    if is_add {
                        bytes.push(0b11000110);
                    } else {
                        bytes.push(0b11001110);
                    }
                    bytes.push(val);
                }

                &DataAccess::Register8(ref reg) => {
                    let base = if is_add { 0b10000000 } else { 0b10001000 };
                    bytes.push(base | register8_to_code(reg));
                }
                _ => {}
            }
        }

        &DataAccess::Register16(Register16::Hl) => match arg2 {
            &DataAccess::Register16(ref reg) => {
                let base = if is_add {
                    0b00001001
                } else {
                    bytes.push(0xED);
                    0b01001010
                };

                bytes.push(base | (register16_to_code_with_sp(reg) << 4));
            }

            _ => {}
        },

        &DataAccess::IndexRegister16(ref reg1) => {
            match arg2 {
                &DataAccess::Register16(ref reg2) => {
                    // TODO Error if reg2 = HL
                    bytes.push(indexed_register16_to_code(reg1));
                    let base = if is_add {
                        0b00001001
                    } else {
                        panic!();
                    };
                    bytes.push(
                        base | (register16_to_code_with_indexed(&DataAccess::Register16(
                            reg2.clone(),
                        )) << 4),
                    )
                }

                &DataAccess::IndexRegister16(ref reg2) => {
                    if reg1 != reg2 {
                        return Err(String::from("Unable to add differetn indexed registers").into());
                    }

                    bytes.push(indexed_register16_to_code(reg1));
                    let base = if is_add {
                        0b00001001
                    } else {
                        panic!();
                    };
                    bytes.push(
                        base | (register16_to_code_with_indexed(&DataAccess::IndexRegister16(
                            reg2.clone(),
                        )) << 4),
                    )
                }

                _ => {}
            }
        }
        _ => {}
    }

    if 0 == bytes.len() {
        Err(format!("{:?} not implemented for {:?} {:?}", mnemonic, arg1, arg2).into())
    } else {
        Ok(bytes)
    }
}

fn assemble_res_or_set(
    mnemonic: &Mnemonic,
    arg1: &DataAccess,
    arg2: &DataAccess,
    sym: &SymbolsTableCaseDependent,
) -> Result<Bytes, AssemblerError> {
    let mut bytes = Bytes::new();
    let is_res = match mnemonic {
        &Mnemonic::Res => true,
        &Mnemonic::Set => false,
        _ => panic!("Impossible case"),
    };

    // Get the bit of interest
    let bit = match arg1 {
        &DataAccess::Expression(ref e) => e.resolve(sym)? as u8,
        _ => return Err(format!("unable to get the number of bits").into()),
    };

    // Apply it to the right thing
    match arg2 {
        &DataAccess::Register8(ref reg) => {
            bytes.push(0xcb);
            bytes.push(
                if is_res { 0b10000000 } else { 0b11000000 } | (bit << 3) | register8_to_code(reg),
            );
        }
        _ => {
            return Err(format!("Res not implemented for {:?}", arg2).into());
        }
    };

    Ok(bytes)
}

fn indexed_register16_to_code(reg: &IndexRegister16) -> u8 {
    match reg {
        &IndexRegister16::Ix => DD,
        &IndexRegister16::Iy => FD,
    }
}

/// Return the code that represents a 8bits register.
/// A: 0b111
/// B: 0b000
/// C: 0b001
/// D: 0b010
/// E: 0b011
/// H: 0b100
/// L: 0b101
#[inline]
fn register8_to_code(reg: &Register8) -> u8 {
    match reg {
        &Register8::A => 0b111,
        &Register8::B => 0b000,
        &Register8::C => 0b001,
        &Register8::D => 0b010,
        &Register8::E => 0b011,
        &Register8::H => 0b100,
        &Register8::L => 0b101,
    }
}

/// Return the code that represents a 16 bits register
fn register16_to_code_with_af(reg: &Register16) -> u8 {
    match reg {
        &Register16::Bc => 0b00,
        &Register16::De => 0b01,
        &Register16::Hl => 0b10,
        &Register16::Af => 0b11,
        _ => panic!("no mapping for {:?}", reg),
    }
}

fn register16_to_code_with_sp(reg: &Register16) -> u8 {
    match reg {
        &Register16::Bc => 0b00,
        &Register16::De => 0b01,
        &Register16::Hl => 0b10,
        &Register16::Sp => 0b11,
        _ => panic!("no mapping for {:?}", reg),
    }
}

fn register16_to_code_with_indexed(reg: &DataAccess) -> u8 {
    match reg {
        &DataAccess::Register16(Register16::Bc) => 0b00,
        &DataAccess::Register16(Register16::De) => 0b01,
        &DataAccess::IndexRegister16(_) => 0b10,
        &DataAccess::Register16(Register16::Sp) => 0b11,
        _ => panic!("no mapping for {:?}", reg),
    }
}

fn flag_test_to_code(flag: &FlagTest) -> u8 {
    match flag {
        &FlagTest::NZ => 0b000,
        &FlagTest::Z => 0b001,
        &FlagTest::NC => 0b010,
        &FlagTest::C => 0b011,

        // the following flags are not used for jr
        &FlagTest::PO => 0b100,
        &FlagTest::PE => 0b101,
        &FlagTest::P => 0b110,
        &FlagTest::M => 0b111,
    }
}

#[cfg(test)]
mod test {
    use crate::assembler::assembler::*;
    

    #[test]
    pub fn test_pop() {
        let res = assemble_pop(&DataAccess::Register16(Register16::Af)).unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0], 0b11110001);
    }

    #[test]
    fn test_jump() {
        let res = assemble_call_jr_or_jp(
            &Mnemonic::Jp,
            &Some(DataAccess::FlagTest(FlagTest::Z)),
            &DataAccess::Expression(Expr::Value(0x1234)),
            &Env::default(),
        )
        .unwrap();
        assert_eq!(res.len(), 3);
        assert_eq!(res[0], 0b11001010);
        assert_eq!(res[1], 0x34);
        assert_eq!(res[2], 0x12);
    }

    #[test]
    pub fn test_assert() {
        let mut env = Env::default();
        env.start_new_pass();
        env.start_new_pass();
        assert!(env.pass.is_second_pass());

        assert!(visit_assert(
            &Expr::Equal(Box::new(0.into()), Box::new(0.into())),
            None,
            &mut env
        )
        .is_ok());
        assert!(visit_assert(
            &Expr::Equal(Box::new(1.into()), Box::new(0.into())),
            None,
            &mut env
        )
        .is_err());
    }

    #[test]
    pub fn test_undef() {
        let mut env = Env::default();
        env.start_new_pass();

        env.visit_label("toto").unwrap();
        assert!(env.symbols().contains_symbol("toto"));
        env.visit_undef("toto").unwrap();
        assert!(!env.symbols().contains_symbol("toto"));
        assert!(env.visit_undef("toto").is_err());
    }

    #[test]
    pub fn test_inc_dec() {
        let res =
            assemble_inc_dec(&Mnemonic::Inc, &DataAccess::Register16(Register16::De)).unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0], 0x13);

        let res = assemble_inc_dec(&Mnemonic::Dec, &DataAccess::Register8(Register8::B)).unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0], 0x05);
    }

    #[test]
    pub fn test_ld() {
        let res = assemble_ld(
            &DataAccess::Register16(Register16::De),
            &DataAccess::Expression(Expr::Value(0x1234)),
            &Env::default(),
        )
        .unwrap();
        assert_eq!(res.len(), 3);
        assert_eq!(res[0], 0x11);
        assert_eq!(res[1], 0x34);
        assert_eq!(res[2], 0x12);
    }

    #[test]
    #[should_panic]
    pub fn test_ld_fail() {
        let _res = assemble_ld(
            &DataAccess::Register16(Register16::Af),
            &DataAccess::Expression(Expr::Value(0x1234)),
            &Env::default(),
        )
        .unwrap();
    }

    #[test]
    pub fn test_ld_R16_R16() {
        let res = assemble_ld(
            &DataAccess::Register16(Register16::De),
            &DataAccess::Register16(Register16::Hl),
            &Env::default(),
        )
        .unwrap();
        assert_eq!(res.len(), 2);
    }

    #[test]
    pub fn test_repeat() {
        let tokens = vec![
            Token::Org(0.into(), None),
            Token::Repeat(
                10.into(),
                vec![Token::OpCode(Mnemonic::Nop, None, None)].into(),
                None,
            ),
        ];

        let count = visit_tokens(&tokens).unwrap().size();
        assert_eq!(count, 10);
    }

    #[test]
    pub fn test_double_repeat() {
        let tokens = vec![
            Token::Org(0.into(), None),
            Token::Repeat(
                10.into(),
                vec![Token::Repeat(
                    10.into(),
                    vec![Token::OpCode(Mnemonic::Nop, None, None)].into(),
                    None,
                )]
                .into(),
                None,
            ),
        ];

        let count = visit_tokens(&tokens).unwrap().size();
        assert_eq!(count, 100);
    }

    #[test]
    pub fn test_assemble_logical_operator() {
        let operators = [Mnemonic::And, Mnemonic::Or, Mnemonic::Xor];
        let operands = [
            DataAccess::Register8(Register8::A),
            DataAccess::Expression(0.into()),
            DataAccess::MemoryRegister16(Register16::Hl),
            DataAccess::IndexRegister16WithIndex(IndexRegister16::Ix, 2.into()),
        ];

        for operator in operators.iter() {
            for operand in operands.iter() {
                let token = Token::OpCode(operator.clone(), Some(operand.clone()), None);
                visit_tokens(&[token]);
            }
        }
    }

    #[test]
    pub fn test_count() {
        let tokens = vec![
            Token::Org(0.into(), None),
            Token::OpCode(Mnemonic::Nop, None, None),
            Token::OpCode(Mnemonic::Nop, None, None),
            Token::OpCode(Mnemonic::Nop, None, None),
            Token::OpCode(Mnemonic::Nop, None, None),
            Token::OpCode(Mnemonic::Nop, None, None),
            Token::OpCode(Mnemonic::Nop, None, None),
            Token::OpCode(Mnemonic::Nop, None, None),
            Token::OpCode(Mnemonic::Nop, None, None),
            Token::OpCode(Mnemonic::Nop, None, None),
            Token::OpCode(Mnemonic::Nop, None, None),
        ];

        let count = visit_tokens(&tokens).unwrap().size();
        assert_eq!(count, 10);
    }

    #[test]
    pub fn test_stableticker() {
        let tokens = vec![
            Token::StableTicker(StableTickerAction::Start("myticker".to_owned())),
            Token::OpCode(
                Mnemonic::Inc,
                Some(DataAccess::Register16(Register16::Hl)),
                None,
            ),
            Token::StableTicker(StableTickerAction::Stop),
        ];

        let env = visit_tokens(&tokens);
        assert!(env.is_ok());
        let env = env.unwrap();

        let val = env.symbols().value("myticker");
        assert!(val.is_some());
        assert_eq!(val.unwrap(), 2);
    }

    #[test]
    pub fn basic_no_variable() {
        let tokens = vec![Token::Basic(None, None, "10 PRINT &DEAD".to_owned())];

        let env = visit_tokens(&tokens);
        println!("{:?}", env);
        assert!(env.is_ok());
    }

    #[test]
    pub fn basic_variable_unset() {
        let tokens = vec![Token::Basic(
            Some(vec!["STUFF".to_owned()]),
            None,
            "10 PRINT {STUFF}".to_owned(),
        )];

        let env = visit_tokens(&tokens);
        println!("{:?}", env);
        assert!(env.is_err());
    }

    #[test]
    pub fn basic_variable_set() {
        let tokens = vec![
            Token::Label("STUFF".to_owned()),
            Token::Basic(
                Some(vec!["STUFF".to_owned()]),
                None,
                "10 PRINT {STUFF}".to_owned(),
            ),
        ];

        let env = visit_tokens(&tokens);
        println!("{:?}", env);
        assert!(env.is_ok());
    }

    #[test]
    pub fn test_duration() {
        let tokens = vec![Token::OpCode(
            Mnemonic::Ld,
            Some(DataAccess::Register8(Register8::A)),
            Some(DataAccess::Expression(Expr::Duration(Box::new(
                Token::OpCode(
                    Mnemonic::Inc,
                    Some(DataAccess::Register16(Register16::Hl)),
                    None,
                ),
            )))),
        )];

        let env = visit_tokens(&tokens);
        assert!(env.is_ok());
        let env = env.unwrap();
        let bytes = env.memory(0, 2);
        assert_eq!(bytes[1], 2);
    }

    #[test]
    pub fn test_opcode() {
        let tokens = vec![Token::OpCode(
            Mnemonic::Ld,
            Some(DataAccess::Register8(Register8::A)),
            Some(DataAccess::Expression(Expr::OpCode(Box::new(
                Token::OpCode(
                    Mnemonic::Inc,
                    Some(DataAccess::Register16(Register16::Hl)),
                    None,
                ),
            )))),
        )];

        let env = visit_tokens(&tokens);
        assert!(env.is_ok());
        let env = env.unwrap();
        let bytes = env.memory(0, 2);
        assert_eq!(
            bytes[1],
            assemble_inc_dec(&Mnemonic::Inc, &DataAccess::Register16(Register16::Hl)).unwrap()[0]
        );
    }

    #[test]
    pub fn test_bytes() {
        let mut m = Bytes::new();

        add_byte(&mut m, 2);
        assert_eq!(m.len(), 1);
        assert_eq!(m[0], 2);

        add_word(&mut m, 0x1234 as u16);
        assert_eq!(m.len(), 3);
        assert_eq!(m[1], 0x34);
        assert_eq!(m[2], 0x12);
    }

    #[test]
    pub fn test_labels() {
        let mut env = Env::default();
        let res = visit_token(&Token::Org(0x4000.into(), None), &mut env);
        assert!(res.is_ok());
        assert!(!env.symbols().contains_symbol("hello"));
        let res = visit_token(&Token::Label("hello".into()), &mut env);
        assert!(res.is_ok());
        assert!(env.symbols().contains_symbol("hello"));
        assert_eq!(env.symbols().value("hello"), 0x4000.into());
    }

    /// Check if  label already exists
    #[test]
    pub fn label_exists() {
        let res = visit_tokens_all_passes(&[
            Token::Org(0x4000.into(), None),
            Token::Label("hello".into()),
            Token::Label("hello".into()),
        ]);
        assert!(res.is_err());
    }

    #[test]
    pub fn test_rorg() {
        let res = visit_tokens_all_passes(&[
            Token::Org(0x4000.into(), None),
            Token::Rorg(
                0x8000.into(),
                vec![Token::Defb(vec![Expr::Label("$".to_owned())])].into(),
            ),
        ]);
        assert!(res.is_ok());
    }

    #[test]
    pub fn test_two_passes() {
        let tokens = vec![
            Token::Org(0x123.into(), None),
            Token::OpCode(
                Mnemonic::Ld,
                Some(DataAccess::Register16(Register16::Hl)),
                Some(DataAccess::Expression(Expr::Label("test".to_string()))),
            ),
            Token::Label("test".to_string()),
        ];
        let env = visit_tokens(&tokens);
        assert!(env.is_err());

        let env = visit_tokens_all_passes(&tokens);
        assert!(env.is_ok());
        let env = env.ok().unwrap();

        let count = env.size();
        assert_eq!(count, 3);

        assert_eq!(env.symbols().value(&"test".to_owned()).unwrap(), 0x123 + 3);
        let buffer = env.memory(0x123, 3);
        assert_eq!(buffer[1], 0x23 + 3);
        assert_eq!(buffer[2], 0x1);
    }

    #[test]
    fn test_read_bytes() {
        let tokens = vec![
            Token::Org(0x100.into(), None),
            Token::Defb(vec![1.into(), 2.into()]),
            Token::Defb(vec![3.into(), 4.into()]),
        ];

        let env = visit_tokens(&tokens).unwrap();
        let bytes = env.memory(0x100, 4);
        assert_eq!(bytes, vec![1, 2, 3, 4]);
    }

}
