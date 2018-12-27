use smallvec;

use crate::assembler::tokens::*;
use std::collections::HashMap;
use smallvec::SmallVec;
use std::fmt;
/// Use smallvec to put stuff on the stack not the heap and (hope so) spead up assembling
const MAX_SIZE:usize = 4;
pub type Bytes =  SmallVec<[u8; MAX_SIZE]>;


/// Add the encoding of an indexed structure
fn add_index(m: &mut Bytes, idx: i32) -> Result<(), String>{
    if idx < -127 || idx > 128 {
        Err(format!("Index error {}", idx))
    }
    else {
        let val = (idx & 0xff) as u8;
        add_byte(m, val);
        Ok(())
    }
}

fn add_byte(m: &mut Bytes, b: u8) {
    m.push(b);
}

fn add_word(m: &mut Bytes, w: u16) {
    m.push( (w%256) as u8);
    m.push( (w/256) as u8);
}

fn add_index_register_code(m: &mut Bytes, r: &IndexRegister16) {
    add_byte(m, indexed_register16_to_code(r));
}

const DD:u8 = 0xdd;
const FD:u8 = 0xfd;

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
    Finished
}
impl fmt::Display for AssemblingPass{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let content = match self {
            AssemblingPass::Uninitialized => "Uninitialized",
            AssemblingPass::FirstPass => "1",
            AssemblingPass::SecondPass => "2",
            AssemblingPass::Finished => "Finished"
        };
        write!(f, "{}", content)
    }
}

impl AssemblingPass {
    fn is_uninitialized(&self) -> bool {
        match self {
            AssemblingPass::Uninitialized => true,
            _ => false
        }
    }

    fn is_finished(&self) -> bool {
        match self {
            AssemblingPass::Finished => true,
            _ => false
        }
    }

    fn is_first_pass(&self) -> bool {
        match self {
            AssemblingPass::FirstPass => true,
            _ => false
        }
    }

    fn is_second_pass(&self) -> bool {
        match self {
            AssemblingPass::SecondPass => true,
            _ => false
        }
    }

    fn next_pass(self) -> AssemblingPass {
        match self {
            AssemblingPass::Uninitialized => AssemblingPass::FirstPass,
            AssemblingPass::FirstPass => AssemblingPass::SecondPass,
            AssemblingPass::SecondPass => AssemblingPass::Finished,
            AssemblingPass::Finished => panic!()
        }
    }

}

#[derive(Default, Debug)]
struct OrgZone {
    ibank: usize,
    protect: bool,
    _memstart: usize,
    _memend: usize
}

#[derive(Debug, Clone)]
pub enum Symbol {
    Integer(i32)
}

#[derive(Debug, Clone)]
pub struct SymbolsTable {
    map: HashMap<String, Symbol>,
    dummy: bool
}

impl Default for SymbolsTable {
    fn default() -> SymbolsTable {
        SymbolsTable {
            map: HashMap::new(),
            dummy: false
        }
    }
}

impl SymbolsTable {

    pub fn laxist() -> SymbolsTable {
        let mut map = HashMap::new();
        map.insert(String::from("$"), Symbol::Integer(0));
        SymbolsTable {
            map,
            dummy: true
        }
    }

    /// Return the current addres if it is known or return an error
    pub fn current_address(&self) -> Result<u16, String> {
        match self.value(&"$".to_owned()) {
            Some(address) => Ok(address as u16),
            None => Err("Current assembling address is unknown".to_owned())
        }
    }

    /// Update `$` value
    pub fn set_current_address(&mut self, address: u16) {
        self.map.insert(String::from("$"), Symbol::Integer(address as _));
    }

    /// Set the given symbol to $ value
    pub fn set_symbol_to_current_address(&mut self, label: &String) -> Result<(), String> {

        self.current_address()
            .map(|val| {
                self.map.insert(label.clone(), Symbol::Integer(val as i32));
                ()
                }
            )
    }

    /// Set the given symbol to the given value
    pub fn set_symbol_to_value(&mut self, label: &String, value: i32) {
        self.map.insert(
            label.clone(), 
            Symbol::Integer(value as _)
        );
    }

    pub fn update_symbol_to_value(&mut self, label: &String, value: i32) {
        *(self.map.get_mut(label).unwrap()) = Symbol::Integer(value as _);
    }

    /// TODO return the symbol instead of the int
    pub fn value(&self, key:&String) -> Option<i32> {

        let key = key.trim();
        let res = self.map.get(key);
        if res.is_some() {
            match res.unwrap() {
                &Symbol::Integer(val) => Some(val),
            }
        }
        else {
            if self.dummy == true {
                //eprintln!("{} not found in symbol table. I have replaced it by 1", key);
                Some(1)
            }
            else {
                None
            }
        }
    }

    pub fn contains_symbol(&self, key:&String) -> bool {
        self.map.contains_key(key)
    }
}

/// Environment of the assembly
pub struct Env {
    /// Current pass
   pass: AssemblingPass, 

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
    mem: [Bank;1],

    iorg: usize,
    org_zones: Vec<OrgZone>,

    symbols: SymbolsTable
}
impl fmt::Debug for Env{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Env{{ pass: {:?}, symbols {:?} }}", self.pass, self.symbols())
    }
}


impl Default for Env {

    fn default() -> Env
    {
        Env{
            pass: AssemblingPass::Uninitialized,

            startadr: None,
            outputadr: 0,
            codeadr: 0,
            maxptr: 0xffff,
            activebank: 0,
            mem : [[0;0x10000];1],

            iorg: 0,
            org_zones: Vec::new(),

            symbols: SymbolsTable::default()
        }
    }
}

impl Env {
    /// Create an environment that embeds a copy of the given table and is configured to be in the latest pass
    pub fn with_table(symbols: &SymbolsTable) -> Env {
        let mut env = Env::default();
        env.symbols = symbols.clone();
        env.pass = AssemblingPass::SecondPass;
        env
    }

    /// Start a new pass by cleaning up datastructures.
    /// The only thing to keep is the symbol table
    fn start_new_pass(&mut self) {
        self.pass = self.pass.clone().next_pass();

        self.startadr = None;
        self.outputadr = 0;
        self.codeadr = 0;
        self.maxptr = 0xffff;
        self.activebank = 0;
        self.mem = [[0;0x10000];1];
        self.iorg = 0;
        self.org_zones = Vec::new();
    }

    pub fn output_address(&self) -> usize {
        self.outputadr
    }

    ///. Update the value of $ in the symbol table in order to take the current  output address
    pub fn update_dollar(&mut self) {
        self.symbols.set_current_address(self.output_address() as _);
    }

    /// Produce the memory for the required limits
    pub fn memory(&self, start: usize, size: usize) -> Vec<u8> {
        let mut mem = Vec::new();
        for pos in start..(start+size) {
            mem.push(self.mem[0][pos]); // XXX probably buggy later
        }
        mem
    }

    /// Returns the stream of bytes produces
    pub fn produced_bytes(&self) -> Vec<u8> {
        self.memory(self.startadr.unwrap() as _, self.outputadr - self.startadr.unwrap())
    }


    /// Output one byte
    /// (RASM ___internal_output)
    pub fn output(&mut self, v: u8) -> Result<(), String> {
        if self.outputadr <= self.maxptr {
            self.mem[self.activebank][self.outputadr] = v;
            self.outputadr += 1; // XXX will fail at 0xffff
            self.codeadr += 1;
            Ok(())
        }
        else {
            Err("Output exceeded limits" .to_owned())
        }
    }


    /// TODO test if we will oversize the limit
    pub fn output_bytes(&mut self, bytes: &[u8]) -> Result<(), String> {
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
        }
        else {
            (self.outputadr - self.startadr.unwrap()) as u16
        }
    }


    /// Evaluate the expression according to the current state of the environment
    pub fn eval(&self, expr: &Expr) -> Result<usize, String> {
        if expr.is_context_independant() {
            match expr.eval() {
                Ok(val) => Ok(val as usize),
                Err(e) => Err(String::from(e))
            }
        }
        else {
            Err(String::from("Not yet implemented"))
        }
    }



    pub fn symbols(&self) -> &SymbolsTable {
        &self.symbols
    }

    pub fn symbols_mut(&mut self) -> &mut SymbolsTable {
        &mut self.symbols
    }


    /// Compute the expression thanks to the symbol table of the environment.
    /// If the expression is not solvable in first pass, 0 is returned.
    /// If the expression is not solvable in second pass, an error is returned
    fn resolve_expr_may_fail_in_first_pass(&self, exp: &Expr) -> Result<i32, String> {
        match exp.resolve(self.symbols()) {
            Ok(value) => Ok(value),
            Err(e) => {
                if self.pass.is_first_pass() {
                    Ok(0)
                }
                else {
                    Err(format!("Impossible to evaluate {} at pass {}. {}", exp, self.pass, e))
                }
            }
        }
    }

    /// Compute the expression thanks to the symbol table of the environment.
    /// An error is systematically raised if the expression is not solvable (i.e., labels are unknown)
    fn resolve_expr_must_never_fail(&self, exp: &Expr) -> Result<i32, String> {
        match exp.resolve(self.symbols()) {
            Ok(value) => Ok(value),
            Err(e) => Err(format!("Impossible to evaluate {} at pass {}. {}", exp, self.pass, e))
        }
    }


    /// Compute the relative address. Is authorized to fail at first pass
    fn absolute_to_relative_may_fail_in_first_pass(&self, address: i32, opcode_delta: i32) -> Result<u8, String> {
        match absolute_to_relative(address, opcode_delta, self.symbols()) {
            Ok(value) => Ok(value),
            Err(e) => {
                if self.pass.is_first_pass() {
                    Ok(0)
                }
                else {
                    Err(format!("Impossible to compute relative address {} at pass {}", address, e))
                }
            }
        }
    }

    /// Add a symbol to the symbol table.
    /// In pass 1: the label MUST be absent
    /// In pass 2: the label MUST be present and of the same value
    fn add_symbol_to_symbol_table(&mut self, label: &str, value: i32) -> Result<(), String> {
        let already_present = self.symbols().contains_symbol(&label.to_owned());

        match (already_present, self.pass) {
            (true, AssemblingPass::FirstPass) => {
                Err(format!("Label {} already present in pass {}", label, self.pass))
            },
            (false, AssemblingPass::SecondPass) => {
                Err(format!("Label {} is not present in the symbol table in pass {}", label, self.pass))
            },
            (false, AssemblingPass::FirstPass) => {
                self.symbols_mut().set_symbol_to_value(
                    &label.to_owned(), value);
                Ok(())
            },
            (true, AssemblingPass::SecondPass) => {
               self.symbols_mut().update_symbol_to_value(
                   &label.to_owned(), value);
                   Ok(())
            },
            (_,_) => unreachable!()
        }
    }


}




/// Visit the tokens during several passes
pub fn visit_tokens_all_passes(tokens: &[Token]) -> Result<Env, String> {

    let mut env = Env::default();

    while !env.pass.is_finished() {
        env.start_new_pass();
        for token in tokens.iter() {
            visit_token(token, &mut env)?;
        }
    }

    Ok(env)
}

/// Visit the tokens during a single pass. Is deprecated in favor to the mulitpass version
#[deprecated]
pub fn visit_tokens(tokens: &[Token]) -> Result<Env, String> {

    let mut env = Env::default();

    for token in tokens.iter() {
        visit_token(token, &mut env)?;
    }

    Ok(env)
}


/// TODO org is a directive, not an opcode => need to change that
pub fn visit_token(token: &Token, env: &mut Env) -> Result<(), String>{
    env.update_dollar();
    match token {
        Token::Org(ref address) => visit_org(address, env),
        Token::Db(_) | &Token::Dw(_) => visit_db_or_dw(token, env),
        Token::Defs(_) => visit_defs(token, env),
        Token::OpCode(ref mnemonic, ref arg1, ref arg2) => visit_opcode(&mnemonic, &arg1, &arg2, env),
        Token::Comment(_) => Ok(()), // Nothing to do for a comment
        Token::Label(ref label) => visit_label(label, env),
        Token::Equ(ref label, ref exp) => visit_equ(label, exp, env),
        Token::Repeat(_, _) => visit_repeat(token, env),
        _ => panic!("Not treated {:?}", token)
    }
}


fn visit_equ(label: &String, exp: &Expr, env: &mut Env) -> Result<(), String> {
    if env.symbols().contains_symbol(label) && env.pass.is_first_pass() {
        Err(format!("Symbol \"{}\" already present in symbols table", label))
    }
    else {
        let value = env.resolve_expr_may_fail_in_first_pass(exp)?;
        env.add_symbol_to_symbol_table(label, value)
    }
}


fn visit_label(label: &String, env: &mut Env) -> Result<(), String> {
    let value = env.symbols().current_address().unwrap();
    if env.pass.is_first_pass() &&  env.symbols().contains_symbol(label) {
        Err(format!("Symbol \"{}\" already present in symbols table", label))
    }
    else {
        env.add_symbol_to_symbol_table(label, value as _)
    }
}

fn visit_defs(token: &Token, env: &mut Env) -> Result<(), String>{
    match token {
        Token::Defs(expr) => {
            let bytes = assemble_defs(expr, env)?;
            env.output_bytes(&bytes)
        },
        _ => unreachable!()
    }
}

// TODO refactor code with assemble_opcode or other functions manipulating bytes
fn visit_db_or_dw(token: &Token, env: &mut Env) -> Result<(), String>{
    let bytes = assemble_db_or_dw(token, env)?;
    env.output_bytes(&bytes)
}

/// When visiting a repetition, we unroll the loop and stream the tokens
pub fn visit_repeat(rept: &Token, env: &mut Env) -> Result<(), String> 
{
    let tokens = rept.unroll(env.symbols()).unwrap()?;

    for token in tokens.iter() {
        visit_token(token, env)?;
    }

    Ok(())
}


pub fn assemble_defs(expr: &Expr, env: &Env) -> Result<Bytes, String> {
    let count = env.resolve_expr_must_never_fail(expr)?;
    let mut bytes = Bytes::with_capacity(count as usize);

    for _i in 0..count {
        bytes.push(0);
    }

    Ok(bytes)

}

pub fn assemble_db_or_dw(token: &Token, env: &Env) -> Result<Bytes, String> {
    let mut bytes = Bytes::new();

    let (ref exprs, mask) = {
        match token {
            &Token::Db(ref exprs) => (exprs, 0xff),
            &Token::Dw(ref exprs) => (exprs, 0xffff),
            _ => unreachable!()
        }
    };

    for exp in exprs.iter() {
        let val = env.resolve_expr_may_fail_in_first_pass(exp)? & mask;
        if mask == 0xff {
            add_byte(&mut bytes, val as u8);
        }
        else {
            add_word(&mut bytes, val as u16);
        }
    }

    Ok(bytes)
}



/// Assemble align directive. It can only work if current address is known...
pub fn assemble_align(expr: &Expr, sym: &SymbolsTable) -> Result<Bytes, String> {
    let expression = expr.resolve(sym)? as u16;
    let current = sym.current_address()?;



    // compute the number of 0 to put
    let mut until = current;
    while until % expression != 0 {
        until +=1;
    }

    // Create the vector
    let hole = (until-current) as usize;
    let mut bytes = Bytes::with_capacity(hole);
    for _i in 0..hole {
        bytes.push(0);
    }

    println!("Expression {}, current {}, hole {}", expression, current, hole);
    // and return it
    Ok(bytes)
}



/// Assemble the opcode and inject in the environement
fn visit_opcode(mnemonic: &Mnemonic, arg1: &Option<DataAccess>, arg2: &Option<DataAccess> , env: &mut Env) -> Result<(), String>{

    // TODO update $ in the symbol table
    let bytes = assemble_opcode(mnemonic, arg1, arg2, env)?;
    for b in bytes.iter() {
        env.output(*b);
    }

    Ok(())
}

/// Assemble an opcode and returns the generated bytes or the error message if it is impossible to
/// assemble
pub fn assemble_opcode(
    mnemonic: &Mnemonic, 
    arg1: &Option<DataAccess>, 
    arg2: &Option<DataAccess>,
    env: &mut Env 
   ) -> Result<Bytes, String> {
    let sym = env.symbols_mut();
    // TODO use env instead of the symbol table for each call
    match mnemonic{
        Mnemonic::And | Mnemonic::Or | Mnemonic::Xor 
            => assemble_logical_operator(mnemonic, arg1.as_ref().unwrap(), sym),
        &Mnemonic::Add | &Mnemonic::Adc
            => assemble_add_or_adc(mnemonic, arg1.as_ref().unwrap(), arg2.as_ref().unwrap(), sym),
        &Mnemonic::Dec | &Mnemonic::Inc
            => assemble_inc_dec(mnemonic, arg1.as_ref().unwrap()),
        &Mnemonic::Djnz
            => assemble_djnz(arg1.as_ref().unwrap(), env),
        &Mnemonic::In
            => assemble_in(arg1.as_ref().unwrap(), &arg2.as_ref().unwrap(), sym),
        &Mnemonic::Ld
            => assemble_ld(arg1.as_ref().unwrap(), &arg2.as_ref().unwrap(), env),
        &Mnemonic::Ldi | &Mnemonic::Ldd | 
        Mnemonic::Ldir | Mnemonic::Lddr |
        &Mnemonic::Ei | &Mnemonic::Di | &Mnemonic::Exx | &Mnemonic::Halt | &Mnemonic::Rra
            => assemble_no_arg(mnemonic),
        &Mnemonic::Nop
            => assemble_nop(),
        &Mnemonic::Nops2
            => assemble_nops2(),
        &Mnemonic::Out
            => assemble_out(arg1.as_ref().unwrap(), &arg2.as_ref().unwrap(), sym),
        &Mnemonic::Jr | &Mnemonic::Jp
            => assemble_jr_or_jp(mnemonic, arg1, &arg2.as_ref().unwrap(), env),
        &Mnemonic::Pop
            => assemble_pop(arg1.as_ref().unwrap()),
        &Mnemonic::Push
            => assemble_push(arg1.as_ref().unwrap()),
        &Mnemonic::Res | &Mnemonic::Set
            => assemble_res_or_set(mnemonic, arg1.as_ref().unwrap(), arg2.as_ref().unwrap(), sym),
        &Mnemonic::Ret
            => assemble_ret(arg1),
        _ => Err(format!("Unable to assembler opcode {:?}", mnemonic))
    }
}

fn visit_org(address: &Expr, env: &mut Env) -> Result<(), String>{
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


fn assemble_no_arg(mnemonic: &Mnemonic) -> Result<Bytes, String> {
    let bytes : &[u8] = match mnemonic {
        Mnemonic::Ldi => {
            &[0xED, 0xA0]
        },
        Mnemonic::Ldd => {
            &[0xED, 0xA8]
        },
        Mnemonic::Lddr => {
            &[0xED, 0xB8]
        },
        Mnemonic::Ldir => {
            &[0xED, 0xB0]
        }
        Mnemonic::Di => {
            &[0xF3]
        },
        Mnemonic::Exx => {
            &[0xD9]
        },
        Mnemonic::Ei => {
            &[0xFB]
        },
        Mnemonic::Halt => {
            &[0x76]
        }
        Mnemonic::Rra => {
            &[0x1f]
        },
        _ => {
            return Err(format!("{} not treated", mnemonic));
        }
    };

    Ok(Bytes::from_slice(bytes))
}

fn assemble_inc_dec(mne: &Mnemonic, arg1: &DataAccess) -> Result<Bytes, String>{
    let mut bytes = Bytes::new();

    let is_inc = match mne {
        &Mnemonic::Inc => true,
        &Mnemonic::Dec => false,
        _ => panic!("Impossible case")
    };

    match arg1 {
        &DataAccess::Register16(ref reg) => {
            let base = if is_inc {
                0b00000011
            }
            else{
                0b00001011
            };
            let byte = base | (register16_to_code_with_sp(reg) << 4);
            bytes.push(byte);
        }

        &DataAccess::IndexRegister16(ref reg) => {
            bytes.push(indexed_register16_to_code(reg));
            bytes.push(
                if is_inc {
                    0x23
                }
                else {
                    0x2b
                }
            );
        }

        &DataAccess::Register8(ref reg) => {
            bytes.push(
                if is_inc {
                    0b00000100
                }
                else {
                    0b00000101
                }
                | (register8_to_code(reg) << 3)
            );
        }
        _ => {
            return Err(format!("Inc/dec not implemented for {:?}", arg1));
        }
    }

    Ok(bytes)
}


/// Converts an absolute address to a relative one (relative to $)
pub fn absolute_to_relative(address: i32, opcode_delta: i32, sym: &SymbolsTable) -> Result<u8, String> {
    match sym.current_address() {
        Err(msg) => Err(format!("Unable to compute the relative address {}", msg)),
        Ok(root) => {
            let delta = (address - (root as i32)) - opcode_delta;
            if delta > 128 || delta < -127 {
                Err(format!(
                    "Address 0x{:x} relative to 0x{:x} is too far {}",
                    address, root, delta
                ))
            }
            else {
                let res = (delta & 0xff) as u8;
                Ok(res)
            }
        }
    }
}

fn assemble_ret(arg1: &Option<DataAccess>) -> Result<Bytes, String> {
    let mut bytes = Bytes::new();

    if arg1.is_some() {
        match arg1.as_ref() {
            Some(&DataAccess::FlagTest(ref test)) => {
                let flag = flag_test_to_code(test);
                bytes.push(0b11000000 | (flag << 3));
            },
            _ => {
                return Err(format!("Wrong argument for ret {:?}", arg1));
            }
        };
    } else {
        bytes.push(0xc9);
    };

    Ok(bytes)
}

/// arg1 contains the tests
/// arg2 contains the information
fn assemble_jr_or_jp(
    mne: &Mnemonic, 
    arg1: &Option<DataAccess>, 
    arg2: &DataAccess , 
    env: &Env) -> Result<Bytes, String>{
    let mut bytes = Bytes::new();

    // check if it is jp or jr
    let is_jr = match mne {
        &Mnemonic::Jr => true,
        &Mnemonic::Jp => false,
        _ => unreachable!()
    };

    // compute the flag code if any
    let flag_code = if arg1.is_some() {
        match arg1.as_ref() {
            Some(&DataAccess::FlagTest(ref test)) => Some(flag_test_to_code(test)),
            _ => return Err(format!("Wrong flag argument {:?}", arg1))
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
                if flag_code.is_some(){
                    // jr - flag
                    add_byte(&mut bytes, 0b00100000 | (flag_code.unwrap() << 3));
                }
                else {
                    // jr - no flag
                    add_byte(&mut bytes, 0b00011000);
                }
                add_byte(&mut bytes, relative);
            }
            else {
                if flag_code.is_some() {
                    // jp - flag
                    add_byte(&mut bytes, 0b11000010 | (flag_code.unwrap() << 3))
                }
                else {
                    // jp - no flag
                    add_byte(&mut bytes, 0xc3);
                }
                add_word(&mut bytes, address as u16);
            }
        },
        _ => {
            return Err(format!("JP parameter {:?} not treated", arg2));
        }
    };

    Ok(bytes)
}

fn assemble_djnz(arg1: &DataAccess, env: &Env) -> Result<Bytes, String> {

    if let &DataAccess::Expression(ref expr) = arg1 {
        let mut bytes = Bytes::new();
        let address = env.resolve_expr_may_fail_in_first_pass(expr)?;
        let relative = env.absolute_to_relative_may_fail_in_first_pass(address, 1)?;

        bytes.push(0x10);
        bytes.push(relative);

        return Ok(bytes)
    }
    else {
        return Err("DJNZ must be followed by an expression".to_owned());
    }

}


fn assemble_ld(arg1: &DataAccess, arg2: &DataAccess, env: &Env) -> Result<Bytes, String>{
    let mut bytes = Bytes::new();

    if let &DataAccess::Register8(ref dst) = arg1 {
        let dst = register8_to_code(dst);
        match arg2 {
            &DataAccess::Register8(ref src) => {
                //R. Zaks p 297
                let src = register8_to_code(src);

                let code = 0b01000000 + (dst<<3) + src;
                bytes.push(code);

            },

            &DataAccess::Expression(ref exp) => {
                let val = (env.resolve_expr_may_fail_in_first_pass(exp)? & 0xff) as u8;

                bytes.push(0b00000110 | (dst<<3));
                bytes.push(val);

            }

            &DataAccess::IndexRegister16WithIndex(ref reg, ref op, ref exp) => {
                let mut val = env.resolve_expr_may_fail_in_first_pass(exp)?;
                if let &Oper::Sub = op {
                    val = -val;
                }

                add_index_register_code(&mut bytes, reg);
                add_byte(&mut bytes, 0b01000110 | (dst <<3));
                add_index(&mut bytes, val)?;

            }

            &DataAccess::MemoryRegister16(Register16::Hl) => {
                add_byte(&mut bytes, 0b01000110 | (dst<<3));
            }

            _ => return Err(format!("LD not properly implemented for '{:?}, {:?}'", arg1, arg2))
        }
    }

    else if let &DataAccess::Register16(ref dst) = arg1 {
        let dst = register16_to_code_with_sp(dst);

        match arg2 {
            &DataAccess::Expression(ref exp) => {
                let val = (env.resolve_expr_may_fail_in_first_pass(exp)? & 0xffff) as u16;

                add_byte(&mut bytes, 0b00000001 | (dst << 4) );
                add_word(&mut bytes, val);
            }

            _ => {}
        }
    }

    else if let &DataAccess::IndexRegister16(ref dst) = arg1 {
        let code = indexed_register16_to_code(dst);

        match arg2 {
            &DataAccess::Expression(ref exp) => {
                let val = (env.resolve_expr_may_fail_in_first_pass(exp)? & 0xffff) as u16;

                add_byte(&mut bytes, code);
                add_byte(&mut bytes, 0x21);
                add_word(&mut bytes, val);
            },

            &DataAccess::Memory(ref exp) => {
                let val = (env.resolve_expr_may_fail_in_first_pass(exp)? & 0xffff) as u16;

                add_byte(&mut bytes, code);
                add_byte(&mut bytes, 0x2a);
                add_word(&mut bytes, val);

            }
            _ => {}
        }
    }

    else if let &DataAccess::MemoryRegister16(ref dst) = arg1 {
        // Want to store in memory pointed by register
        match dst {
            &Register16::Hl
                => {
                    if let &DataAccess::Register8(ref src) = arg2 {
                        let src = register8_to_code(src);
                        let code = 0b01110000 | src;
                        bytes.push(code);
                    }
                }

            &Register16::De if &DataAccess::Register8(Register8::A) == arg2
                => {
                    bytes.push(0b00010010);
            },

            &Register16::Bc if &DataAccess::Register8(Register8::A) == arg2
                => {
                    bytes.push(0b00000010);
            },

            _ => {}
        }
    }


    else if let &DataAccess::Memory(ref exp) = arg1 {
        let address = env.resolve_expr_may_fail_in_first_pass(exp)?;

        match arg2 {
            &DataAccess::IndexRegister16(IndexRegister16::Ix) => {
                bytes.push(0xdd);
                bytes.push(0b00100010);
                add_word(&mut bytes, address as _);
            },
            &DataAccess::IndexRegister16(IndexRegister16::Iy) => {
                bytes.push(0xfd);
                bytes.push(0b00100010);
                add_word(&mut bytes, address as _);
            },
            &DataAccess::Register16(Register16::Hl) => {
                bytes.push(0b00100010);
                add_word(&mut bytes, address as _);
            },
            &DataAccess::Register16(ref reg) => {
                bytes.push(0xED);
                bytes.push(0b01000011 |  (register16_to_code_with_sp(reg)));
                add_word(&mut bytes, address as _);
            }

            _ => {}
        }
    }

    if bytes.len() == 0
    {
        Err(format!("LD not properly implemented for '{:?}, {:?}'", arg1, arg2))
    }
    else {
        Ok(bytes)
    }
}


fn assemble_nop() -> Result<Bytes, String> {
    let mut bytes = Bytes::new();
    bytes.push(0);
    Ok(bytes)
}


fn assemble_nops2() -> Result<Bytes, String> {
    let mut bytes = Bytes::new();
    bytes.push(0xed);
    bytes.push(0xff);
    Ok(bytes)
}



fn assemble_in(arg1: &DataAccess, arg2: &DataAccess, sym: &SymbolsTable) -> Result<Bytes, String>{
   let mut bytes = Bytes::new();

    match arg1 {
        &DataAccess::Register8(Register8::C) => {
            match arg2 {
                &DataAccess::Register8( ref reg) => {
                    bytes.push(0xED);
                    bytes.push(0b01000000 | (register8_to_code(reg)<<3))
                },
                _ => panic!()
            }
        },

        &DataAccess::Memory(ref exp) => {
            if let &DataAccess::Register8(Register8::A) = arg2 {
                let val = (exp.resolve(sym)? & 0xff) as u8;
                bytes.push(0xDB);
                bytes.push(val);

            }
        }
        _ => panic!()
    };

    if bytes.len() == 0
    {
        Err(format!("IN not properly implemented for '{:?}, {:?}'", arg1, arg2))
    }
    else {
        Ok(bytes)
    }
}

fn assemble_out(arg1: &DataAccess, arg2: &DataAccess, sym: &SymbolsTable) -> Result<Bytes, String>{
    let mut bytes = Bytes::new();

    match arg1 {
        &DataAccess::Register8(Register8::C) => {
            match arg2 {
                &DataAccess::Register8( ref reg) => {
                    bytes.push(0xED);
                    bytes.push(0b01000001 | (register8_to_code(reg)<<3))
                },
                _ => {}
            }
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

    if bytes.len() == 0
    {
        Err(format!("OUT not properly implemented for '{:?}, {:?}'", arg1, arg2))
    }
    else {
        Ok(bytes)
    }
}


fn assemble_pop(arg1: &DataAccess) -> Result<Bytes, String>{
    let mut bytes = Bytes::new();

    match arg1 {
        &DataAccess::Register16(ref reg) => {
            let byte = 0b11000001 | (register16_to_code_with_af(reg) << 4);
            bytes.push(byte);
        },
        &DataAccess::IndexRegister16(ref reg) => {
            bytes.push(indexed_register16_to_code(reg));
            bytes.push(0xe1);
        },
        _ => {
            return Err(format!("Pop not implemented for {:?}", arg1));
        }
    }

    Ok(bytes)
}


fn assemble_push(arg1: &DataAccess) -> Result<Bytes, String>{
    let mut bytes = Bytes::new();

    match arg1 {
        &DataAccess::Register16(ref reg) => {
            let byte = 0b11000101 | (register16_to_code_with_af(reg) << 4);
            bytes.push(byte);
        },
        &DataAccess::IndexRegister16(ref reg) => {
            bytes.push(indexed_register16_to_code(reg));
            bytes.push(0xe5);
        },
        _ => {
            return Err(format!("Pop not implemented for {:?}", arg1));
        }
    }

    Ok(bytes)
}


fn assemble_logical_operator(mnemonic: &Mnemonic, arg1: &DataAccess, sym : &SymbolsTable)-> Result<Bytes, String>{
    let mut bytes = Bytes::new();

    let memory_code = || {
        match mnemonic {
            Mnemonic::And => 0xA6,
            Mnemonic::Or =>  0xB6,
            Mnemonic::Xor=>  0xAE,
            _ => unreachable!()
        }
    };

    match *arg1 {
        DataAccess::Register8(ref reg) => {
            let base = match mnemonic {
                Mnemonic::And => 0b10100000,
                Mnemonic::Or =>  0b10110000,
                Mnemonic::Xor => 0b10101000,
                _ => unreachable!()
            };
            bytes.push(base + register8_to_code(reg));
        },

        DataAccess::Expression(ref exp) => {
            let base = match mnemonic {
                Mnemonic::And => 0xE6,
                Mnemonic::Or =>  0xF6,
                Mnemonic::Xor => 0xAE,
                _ => unreachable!()
            };
            let value = exp.resolve(sym)? & 0xff;
            bytes.push(base);
            bytes.push(value as u8);
        },

        DataAccess::MemoryRegister16(Register16::Hl) => {
            bytes.push(memory_code());
        },

        DataAccess::IndexRegister16WithIndex(ref reg, ref oper, ref exp) => {

            let value = exp.resolve(sym)? & 0xff;
            assert_eq!(oper, &Oper::Add); // XXX todo thing if it is not the case
            bytes.push(indexed_register16_to_code(reg));
            bytes.push(memory_code());
            bytes.push(value as u8);
        }
        _ => unreachable!()
    }

    Ok(bytes)
}

fn assemble_add_or_adc(mnemonic: &Mnemonic, arg1: &DataAccess, arg2: &DataAccess , sym: &SymbolsTable) -> Result<Bytes, String>{
    let mut bytes = Bytes::new();
    let is_add = match mnemonic {
        &Mnemonic::Add => true,
        &Mnemonic::Adc => false,
        _ => panic!("Impossible case")
    };


    match arg1 {
        &DataAccess::Register8(Register8::A) => {
            match arg2 {
                &DataAccess::MemoryRegister16(Register16::Hl) => {
                    if is_add {
                        bytes.push(0b10000110);
                    }
                    else {
                        bytes.push(0b10001110);
                    }
                },

                &DataAccess::IndexRegister16WithIndex(ref reg, ref op, ref exp) => {
                    let val = exp.resolve(sym)? as i32;
                    let val = match op {
                        &Oper::Add => val,
                        &Oper::Sub => -val,
                        _ => panic!()
                    };

                    bytes.push(indexed_register16_to_code(reg));
                    if is_add {
                        bytes.push(0b10000110);
                    }
                    else {
                        bytes.push(0b10001110);
                    }
                    add_index(&mut bytes, val)?;

                },

                &DataAccess::Expression(ref exp) => {
                    let val = exp.resolve(sym)? as u8;
                    if is_add {
                        bytes.push(0b11000110);
                    }
                    else {
                        bytes.push(0b11001110);
                    }
                    bytes.push(val);
                },

                &DataAccess::Register8(ref reg) => {
                    let base = if is_add {
                        0b10000000
                    }
                    else {
                        0b10001000
                    };
                    bytes.push(base | register8_to_code(reg));
                },
                _ => {}
            }

        },

        &DataAccess::Register16(Register16::Hl) => {
            match arg2 {
                &DataAccess::Register16(ref reg) => {
                    let base = if is_add {
                        0b00001001
                    }
                    else {
                        bytes.push(0xED);
                        0b01001010
                    };

                    bytes.push(base | (register16_to_code_with_sp(reg)<<4));
                },

                _ => {}
            }
        },

        &DataAccess::IndexRegister16(ref reg1) => {
            match arg2 {
                &DataAccess::Register16(ref reg2) => {
                    // TODO Error if reg2 = HL
                    bytes.push(indexed_register16_to_code(reg1));
                    let base = if is_add {
                        0b00001001
                    }
                    else {
                        panic!();
                    };
                    bytes.push(base | (register16_to_code_with_indexed(&DataAccess::Register16(reg2.clone())) <<4))
                },

                &DataAccess::IndexRegister16(ref reg2) => {
                    if reg1 != reg2 {
                        return Err(String::from("Unable to add differetn indexed registers"));
                    }

                    bytes.push(indexed_register16_to_code(reg1));
                    let base = if is_add {
                        0b00001001
                    }
                    else {
                        panic!();
                    };
                    bytes.push(base | (register16_to_code_with_indexed(&DataAccess::IndexRegister16(reg2.clone())) <<4))
                },





                _ => {}
            }
        }
        _ => {}
    }


    if 0 == bytes.len() {
        Err(format!("{:?} not implemented for {:?} {:?}", mnemonic, arg1, arg2))
    }
    else {
        Ok(bytes)
    }

}

fn assemble_res_or_set(mnemonic: &Mnemonic, arg1: &DataAccess, arg2: &DataAccess , sym: &SymbolsTable) -> Result<Bytes, String>{
    let mut bytes = Bytes::new();
    let is_res = match mnemonic {
        &Mnemonic::Res => true,
        &Mnemonic::Set => false,
        _ => panic!("Impossible case")
    };

    // Get the bit of interest
    let bit = match arg1 {
        &DataAccess::Expression(ref e) => e.resolve(sym)? as u8,
        _ => return Err(format!("unable to get the number of bits"))
    };

    // Apply it to the right thing
    match arg2 {
        &DataAccess::Register8(ref reg) => {
            bytes.push(0xcb);
            bytes.push(if is_res {0b10000000} else {0b11000000}
                       | (bit << 3)
                       | register8_to_code(reg));
        },
        _ => {
            return Err(format!("Res not implemented for {:?}", arg2));
        }
    };

    Ok(bytes)

}



fn indexed_register16_to_code(reg: &IndexRegister16) -> u8 {
    match reg {
        &IndexRegister16::Ix => DD,
        &IndexRegister16::Iy => FD
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
        &Register8::L => 0b101
    }
}


/// Return the code that represents a 16 bits register
fn register16_to_code_with_af(reg: &Register16) -> u8 {

    match reg {
        &Register16::Bc => 0b00,
        &Register16::De => 0b01,
        &Register16::Hl => 0b10,
        &Register16::Af => 0b11,
        _ => panic!("no mapping for {:?}", reg)
    }
}

fn register16_to_code_with_sp(reg: &Register16) -> u8 {

    match reg {
        &Register16::Bc => 0b00,
        &Register16::De => 0b01,
        &Register16::Hl => 0b10,
        &Register16::Sp => 0b11,
        _ => panic!("no mapping for {:?}", reg)
    }
}



fn register16_to_code_with_indexed(reg: &DataAccess) -> u8 {

    match reg {
        &DataAccess::Register16(Register16::Bc) => 0b00,
        &DataAccess::Register16(Register16::De) => 0b01,
        &DataAccess::IndexRegister16(_) => 0b10,
        &DataAccess::Register16(Register16::Sp) => 0b11,
        _ => panic!("no mapping for {:?}", reg)
    }
}


fn flag_test_to_code(flag: &FlagTest) -> u8 {

    match flag {
        &FlagTest::NZ => 0b000,
        &FlagTest::Z =>  0b001,
        &FlagTest::NC => 0b010,
        &FlagTest::C =>  0b011,
        &FlagTest::PO => 0b100,
        &FlagTest::PE => 0b101,
        &FlagTest::P =>  0b110,
        &FlagTest::M =>  0b111,
    }
}



#[cfg(test)]
mod test {
    use crate::assembler::tokens::*;
    use crate::assembler::assembler::*;

    #[test]
    pub fn test_pop() {
        let res = assemble_pop(&DataAccess::Register16(Register16::Af)).unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0], 0b11110001);
    }


    #[test]
    fn test_jump() {
        let res = assemble_jr_or_jp(
            &Mnemonic::Jp,
            &Some(DataAccess::FlagTest(FlagTest::Z)),
            &DataAccess::Expression(Expr::Value(0x1234)),
            &Env::default()
            ).unwrap();
        assert_eq!(res.len(), 3);
        assert_eq!(res[0], 0b11001010);
        assert_eq!(res[1], 0x34);
        assert_eq!(res[2], 0x12);
    }

    #[test]
    pub fn test_inc_dec() {
        let res = assemble_inc_dec(&Mnemonic::Inc, &DataAccess::Register16(Register16::De)).unwrap();
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
            &Env::default()
        ).unwrap();
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
            &Env::default()
        ).unwrap();
    }


    #[test]
    pub fn test_repeat() {
        let tokens = vec![
            Token::Org(0.into()),
            Token::Repeat(
                10.into(),
                vec![Token::OpCode(Mnemonic::Nop, None, None)])
        ];

        let count = visit_tokens(&tokens).unwrap().size();
        assert_eq!(count, 10);
    }

    #[test]
    pub fn test_double_repeat() {
        let tokens = vec![
            Token::Org(0.into()),
            Token::Repeat(
                10.into(),
                vec![
                    Token::Repeat(
                        10.into(),
                        vec![Token::OpCode(Mnemonic::Nop, None, None)])
                ]
            )
        ];

        let count = visit_tokens(&tokens).unwrap().size();
        assert_eq!(count, 100);
    }


    #[test]
    pub fn test_assemble_logical_operator() {
        let operators = [
            Mnemonic::And, 
            Mnemonic::Or, 
            Mnemonic::Xor
        ];
        let operands = [
            DataAccess::Register8(Register8::A),
            DataAccess::Expression(0.into()),
            DataAccess::MemoryRegister16(Register16::Hl),
            DataAccess::IndexRegister16WithIndex(IndexRegister16::Ix, Oper::Add, 2.into())
        ];

        for operator in operators.iter() {
            for operand in operands.iter() {
                let token = Token::OpCode(
                    operator.clone(), 
                    Some(operand.clone()), 
                    None
                );
                visit_tokens(&[token]);
            }
        }
    }

    #[test]
    pub fn test_count() {
        let tokens = vec![
            Token::Org(0.into()),
            Token::OpCode(Mnemonic::Nop, None, None),
            Token::OpCode(Mnemonic::Nop, None, None),
            Token::OpCode(Mnemonic::Nop, None, None),
            Token::OpCode(Mnemonic::Nop, None, None),
            Token::OpCode(Mnemonic::Nop, None, None),
            Token::OpCode(Mnemonic::Nop, None, None),
            Token::OpCode(Mnemonic::Nop, None, None),
            Token::OpCode(Mnemonic::Nop, None, None),
            Token::OpCode(Mnemonic::Nop, None, None),
            Token::OpCode(Mnemonic::Nop, None, None)
        ];

        let count = visit_tokens(&tokens).unwrap().size();
        assert_eq!(count, 10);
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
        let res = visit_token(
            &Token::Org(0x4000.into()),
            &mut env
        );
        assert!(res.is_ok());
        assert!(!env.symbols().contains_symbol(&"hello".into()));
        let res = visit_token(
            &Token::Label("hello".into()),
            &mut env
        );
        assert!(res.is_ok());
        assert!(env.symbols().contains_symbol(&"hello".into()));
        assert_eq!(env.symbols().value(&"hello".into()), 0x4000.into());
    }

    #[test]
    pub fn test_two_passes() {
        let tokens = vec![
            Token::Org(0x123.into()),
            Token::OpCode(
                Mnemonic::Ld, 
                Some(DataAccess::Register16(Register16::Hl)),
                Some(DataAccess::Expression(Expr::Label("test".to_string())))
            ),
            Token::Label("test".to_string())
        ];
        let env = visit_tokens(&tokens);
        assert!(env.is_err());

        let env = visit_tokens_all_passes(&tokens);
        assert!(env.is_ok());
        let env = env.ok().unwrap();

        let count = env.size();
        assert_eq!(count, 3);

        assert_eq!(
            env.symbols().value(&"test".to_owned()).unwrap(),
            0x123+3
        );
        let buffer = env.memory(0x123, 3);
        assert_eq!(buffer[1], 0x23+3);
        assert_eq!(buffer[2], 0x1);

    }


	#[test]
	fn test_read_bytes() {
        let tokens = vec![
            Token::Org(0x100.into()),
            Token::Db(vec![1.into(), 2.into()]),
            Token::Db(vec![3.into(), 4.into()]),
        ];

        let env = visit_tokens(&tokens).unwrap();
        let bytes = env.memory(0x100, 4);
        assert_eq!(bytes, vec![1, 2, 3, 4]);
	}

}
