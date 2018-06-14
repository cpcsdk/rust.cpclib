use z80emu::z80::*;
use assembler::tokens::*;

impl Z80 {



    /// Execute the given token.
    /// XXX Currently only OpCode are managed whereas some other
    /// tokens also have a sense there
    pub fn execute(&mut self, opcode: &Token) {
        match opcode {
            &Token::OpCode(ref mnemonic, ref arg1, ref arg2) => {
                self.execute_opcode(mnemonic, arg1.as_ref(),  arg2.as_ref());
            }
            _ => panic!()
        }
    }


    /// Execute the given opcode
    fn execute_opcode(&mut self, mneomonic: &Mnemonic, arg1: Option<&DataAccess>, arg2: Option<&DataAccess>) {
    
    }

    /// Read the value provided by the given access.
    /// None is returned if we do not have enough information to get it
    /// TODO better emulation to never return None
    fn get_value(&self, access: &DataAccess) -> Option<u16> {
        match access {
            &DataAccess::Memory(_) => None, // TODO read the value from memory
            &DataAccess::IndexRegister16WithIndex(_, _, _) => None,
            &DataAccess::IndexRegister16(ref reg) => Some(self.get_register_16(access).value() as _),
            &DataAccess::IndexRegister8(ref reg) => Some(self.get_register_8(access).value() as _),
            &DataAccess::Register16(ref reg) => Some(self.get_register_16(access).value() as _),
            &DataAccess::Register8(ref reg) => Some(self.get_register_8(access).value() as _),
            &DataAccess::MemoryRegister16(_) => None,
            &DataAccess::Expression(ref expr) => self.eval_expr(expr),
            &DataAccess::FlagTest(_) => panic!()
        }
    }


    /// Returns the register encoded by the DataAccess
    fn get_register_16(&self, reg: &DataAccess) -> & ::z80emu::z80::Register16{
        match reg{
            &DataAccess::IndexRegister16(ref reg) => {
                match reg {
                    IndexRegister16::Ix => self.ix(),
                    IndexRegister16::Iy => self.iy(),
                }
            },
            &DataAccess::Register16(ref reg) => {
                match reg {
                    ::assembler::tokens::Register16::Af => self.af(),
                    ::assembler::tokens::Register16::Bc => self.bc(),
                    ::assembler::tokens::Register16::De => self.de(),
                    ::assembler::tokens::Register16::Hl => self.hl(),
                    ::assembler::tokens::Register16::Sp => self.sp(),
                }
            },
            _ => panic!()
        }

    }

    fn get_register_8(&self, reg: &DataAccess) -> &::z80emu::z80::Register8{
        match reg{
            &DataAccess::Register8(ref reg) => {
                match reg {
                    ::assembler::tokens::Register8::A => self.a(),
                    ::assembler::tokens::Register8::B => self.b(),
                    ::assembler::tokens::Register8::D => self.d(),
                    ::assembler::tokens::Register8::H => self.h(),
                    ::assembler::tokens::Register8::C => self.c(),
                    ::assembler::tokens::Register8::E => self.e(),
                    ::assembler::tokens::Register8::L => self.l(),
                }
            },

            &DataAccess::IndexRegister8(ref reg) => 
                match reg {
                    IndexRegister8::Ixl => self.ixl(),
                    IndexRegister8::Ixh => self.ixh(),
                    IndexRegister8::Iyl => self.iyl(),
                    IndexRegister8::Iyh => self.iyh(),
                },
            _ => panic!()
        }
    }

    fn eval_expr(&self, expr: &Expr) -> Option<u16> {
        match expr.eval() {
            Ok(val) => Some(val as u16),
            Err(_) => None
        }
    }
}
