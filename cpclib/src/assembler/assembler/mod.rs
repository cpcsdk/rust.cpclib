extern crate smallvec;

use assembler::tokens::*;
use std::collections::HashMap;
use smallvec::SmallVec;

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

#[derive(Default)]
struct OrgZone {
    ibank: usize,
    protect: bool,
    _memstart: usize,
    _memend: usize
}

#[derive(Debug)]
pub enum Symbol {
    Integer(i32)
}

#[derive(Debug)]
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
            None => Err("Current assembling adress is unknown".to_owned())
        }
    }

    pub fn set_current_address(&mut self, address: u16) {
        self.map.insert(String::from("$"), Symbol::Integer(address as _));
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
                eprintln!("'{}' not found {}", key, self.dummy);
                None
            }
        }
    }
}

/// Environment of the assembly
pub struct Env {
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


impl Default for Env {

    fn default() -> Env
    {
        Env{
            startadr: Some(0),
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



    /// Output one byte
    /// (RASM ___internal_output)
    pub fn output(&mut self, v: u8) {
        if self.outputadr <= self.maxptr {
            self.mem[self.activebank][self.outputadr] = v;
            self.outputadr += 1; // XXX will fail at 0xffff
            self.codeadr += 1;
        }
        else {
            panic!("Output exceeed limit");
        }
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
            (self.outputadr - self.codeadr) as u16
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
}

pub fn visit_tokens(tokens: & Vec<Token>) -> Result<Env, String> {

    let mut env = Env::default();

    for token in tokens.iter() {
        visit_token(token, &mut env)?;
    }

    Ok(env)
}


/// TODO org is a directive, not an opcode => need to change that
pub fn visit_token(token: &Token, env: &mut Env) -> Result<(), String>{
    match token {
        &Token::Org(ref address) => visit_org(address, env),
        &Token::Db(_) | &Token::Dw(_) => visit_db_or_dw(token, env),
        &Token::OpCode(ref mnemonic, ref arg1, ref arg2) => visit_opcode(&mnemonic, &arg1, &arg2, env),
        &Token::Comment(_) => Ok(()), // Nothing to do for a comment
        _ => panic!("Not treated")
    }
}

// TODO refactor code with assemble_opcode or other functions manipulating bytes
fn visit_db_or_dw(token: &Token, env: &mut Env) -> Result<(), String>{
    let bytes = assemble_db_or_dw(token, env.symbols())?;
    for b in bytes.iter() {
        env.output(*b);
    }
    Ok(())
}

pub fn assemble_db_or_dw(token: &Token, sym: &SymbolsTable) -> Result<Bytes, String> {
    let mut bytes = Bytes::new();

    let (ref exprs, mask) = {
        match token {
            &Token::Db(ref exprs) => (exprs, 0xff),
            &Token::Dw(ref exprs) => (exprs, 0xffff),
            _ => panic!("impossible case")
        }
    };

    for expr in exprs.iter() {
        let val = expr.resolve(sym).unwrap() & mask;
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
    for i in 0..hole {
        bytes.push(0);
    }

    println!("Expression {}, current {}, hole {}", expression, current, hole);
    // and return it
    Ok(bytes)


}


/// Assemble the opcode and inject in the environement
fn visit_opcode(mnemonic: &Mnemonic, arg1: &Option<DataAccess>, arg2: &Option<DataAccess> , env: &mut Env) -> Result<(), String>{

    // TODO update $ in the symbol table
    let bytes = assemble_opcode(mnemonic, arg1, arg2, env.symbols())?;
    for b in bytes.iter() {
        env.output(*b);
    }

    Ok(())
}

/// Assemble an opcode and returns the generated bytes or the error message if it is impossible to
/// assemble
pub fn assemble_opcode(mnemonic: &Mnemonic, arg1: &Option<DataAccess>, arg2: &Option<DataAccess> , sym: & SymbolsTable) -> Result<Bytes, String> {
    match mnemonic{
        &Mnemonic::Add | &Mnemonic::Adc
            => assemble_add_or_adc(mnemonic, arg1.as_ref().unwrap(), arg2.as_ref().unwrap(), sym),
        &Mnemonic::Dec | &Mnemonic::Inc
            => assemble_inc_dec(mnemonic, arg1.as_ref().unwrap()),
        &Mnemonic::Djnz
            => assemble_djnz(arg1.as_ref().unwrap(), sym),
        &Mnemonic::Ld
            => assemble_ld(arg1.as_ref().unwrap(), &arg2.as_ref().unwrap(), sym),
        &Mnemonic::Ldi | &Mnemonic::Ldd | &Mnemonic::Ei | &Mnemonic::Di | &Mnemonic::Exx
            => assemble_no_arg(mnemonic),
        &Mnemonic::Nop
            => assemble_nop(),
        &Mnemonic::Out
            => assemble_out(arg1.as_ref().unwrap(), &arg2.as_ref().unwrap(), sym),
        &Mnemonic::Jr | &Mnemonic::Jp
            => assemble_jr_or_jp(mnemonic, arg1, &arg2.as_ref().unwrap(), sym),
        &Mnemonic::Pop
            => assemble_pop(arg1.as_ref().unwrap()),
        &Mnemonic::Push
            => assemble_push(arg1.as_ref().unwrap()),
        &Mnemonic::Res | &Mnemonic::Set
            => assemble_res_or_set(mnemonic, arg1.as_ref().unwrap(), arg2.as_ref().unwrap(), sym),
        &Mnemonic::Ret
            => assemble_ret(arg1),
    }
}

fn visit_org(address: &Expr, env: &mut Env) -> Result<(), String>{
    let adr = env.eval(address)?;

    // TODO Check overlapping region
    // TODO manage rorg like instruction
    env.outputadr = adr as usize;
    env.codeadr = adr as usize;

    // Specify start address at first use
    if env.startadr.is_none() && env.outputadr == 0 {
        env.startadr = Some(adr as usize);
    }

    Ok(())
}


fn assemble_no_arg(mnemonic: &Mnemonic) -> Result<Bytes, String> {
    let mut bytes = Bytes::new();

    match mnemonic {
        &Mnemonic::Ldi => {
            bytes.push(0xED);
            bytes.push(0xA0);
        },
        &Mnemonic::Ldd => {
            bytes.push(0xED);
            bytes.push(0xA8);
        },
        &Mnemonic::Ei => {
            bytes.push(0xF3);
        },
        &Mnemonic::Exx => {
            bytes.push(0xD9);
        },
        &Mnemonic::Di => {
            bytes.push(0xFB);
        },
        _ => {
            return Err(format!("{} not treated", mnemonic));
        }
    };

    Ok(bytes)
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
                | register8_to_code(reg)
            );
        }
        _ => {
            return Err(format!("Inc/dec not implemented for {:?}", arg1));
        }
    }

    Ok(bytes)
}


/// Result represents -16 to 129
pub fn absolute_to_relative(_address: i32, _sym: &SymbolsTable) -> u8 {
    eprintln!("absolute_to_relative is not implemented and systematically returns 0");
    0
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
fn assemble_jr_or_jp(mne: &Mnemonic, arg1: &Option<DataAccess>, arg2: &DataAccess , sym: &SymbolsTable) -> Result<Bytes, String>{
    let mut bytes = Bytes::new();

    // check if it is jp or jr
    let is_jr = match mne {
        &Mnemonic::Jr => true,
        &Mnemonic::Jp => false,
        _ =>panic!()
    };

    // compute the flag code if any
    let flag_code = if arg1.is_some() {
        match arg1.as_ref() {
            Some(&DataAccess::FlagTest(ref test)) => Some(flag_test_to_code(test)),
            _ => panic!("Wrong argument")
        }
    } else {
        None
    };

    // Treat address
    match arg2 {
        &DataAccess::Expression(ref e) => {
            let address = e.resolve(sym)?;
            if is_jr {
                let relative =absolute_to_relative(address, sym);
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

fn assemble_djnz(arg1: &DataAccess, sym: &SymbolsTable) -> Result<Bytes, String> {

    if let &DataAccess::Expression(ref expr) = arg1 {
        let mut bytes = Bytes::new();
        let address = expr.resolve(sym)?;
        let relative = absolute_to_relative(address, sym);

        bytes.push(0x10);
        bytes.push(relative);

        return Ok(bytes)
    }
    else {
        return Err("DJNZ must be followed by an expression".to_owned());
    }

}


fn assemble_ld(arg1: &DataAccess, arg2: &DataAccess , sym: &SymbolsTable) -> Result<Bytes, String>{
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
                let val = (exp.resolve(sym)? & 0xff) as u8;

                bytes.push(0b00000110 | (dst<<3));
                bytes.push(val);

            }

            &DataAccess::IndexRegister16WithIndex(ref reg, ref op, ref exp) => {
                let mut val = exp.resolve(sym)?;
                if let &Oper::Sub = op {
                    val = -val;
                }

                add_index_register_code(&mut bytes, reg);
                add_byte(&mut bytes, 0b01000110 | (dst <<3));
                add_index(&mut bytes, val)?;

            }

            _ => return Err(format!("LD not properly implemented for '{:?}, {:?}'", arg1, arg2))
        }
    }

    else if let &DataAccess::Register16(ref dst) = arg1 {
        let dst = register16_to_code_with_sp(dst);

        match arg2 {
            &DataAccess::Expression(ref exp) => {
                let val = (exp.resolve(sym)? & 0xffff) as u16;

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
                let val = (exp.resolve(sym)? & 0xffff) as u16;

                add_byte(&mut bytes, code);
                add_byte(&mut bytes, 0x21);
                add_word(&mut bytes, val);
            },

            &DataAccess::Memory(ref exp) => {
                let val = (exp.resolve(sym)? & 0xffff) as u16;

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
        let address = exp.resolve(sym)?;

        match arg2 {
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


pub fn assemble_defs(expr: &Expr, sym: &SymbolsTable) -> Result<Bytes, String> {
    let count = expr.resolve(sym)?;
    let mut bytes = Bytes::with_capacity(count as usize);

    for _i in 0..count {
        bytes.push(0);
    }

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
    use assembler::tokens::*;
    use assembler::assembler::*;

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
            &SymbolsTable::default()
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
            &SymbolsTable::default()
        ).unwrap();
        assert_eq!(res.len(), 3);
        assert_eq!(res[0], 0x11);
        assert_eq!(res[1], 0x34);
        assert_eq!(res[2], 0x12);
    }


    #[test]
    #[should_panic]
    pub fn test_ld_fail() {
        let res = assemble_ld(
            &DataAccess::Register16(Register16::Af),
            &DataAccess::Expression(Expr::Value(0x1234)),
            &SymbolsTable::default()
        ).unwrap();
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
}
