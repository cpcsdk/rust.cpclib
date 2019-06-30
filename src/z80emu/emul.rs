use crate::assembler::tokens::data_access;
use crate::assembler::tokens::*;
use crate::z80emu::z80::*;

impl Z80 {
    /// Execute the given token.
    /// XXX Currently only OpCode are managed whereas some other
    /// tokens also have a sense there
    pub fn execute(&mut self, opcode: &Token) {
        match opcode {
            &Token::OpCode(ref mnemonic, ref arg1, ref arg2) => {
                self.execute_opcode(mnemonic, arg1.as_ref(), arg2.as_ref());
            }
            _ => panic!("{:?} is not yet handled", opcode),
        }

        /// TODO use a cache to speed up that
        let opcode_size = opcode.number_of_bytes().unwrap();
        self.pc_mut().add(opcode_size as _);
    }

    /// Execute the given opcode. Parameters are assumed to be valid
    fn execute_opcode(
        &mut self,
        mnemonic: &Mnemonic,
        arg1: Option<&DataAccess>,
        arg2: Option<&DataAccess>,
    ) {
        match mnemonic {
            Mnemonic::Add => match (arg1, arg2) {
                (
                    Some(&DataAccess::Register8(crate::assembler::tokens::Register8::A)),
                    Some(&DataAccess::Register8(_)),
                ) => {
                    let val = self.get_register_8(arg2.unwrap()).value();
                    self.get_register_8_mut(arg1.unwrap()).add(val);
                }

                (
                    Some(&DataAccess::Register16(crate::assembler::tokens::Register16::Hl)),
                    Some(_),
                ) => {
                    let val = self.get_value(arg2.unwrap()).unwrap();
                    self.get_register_16_mut(arg1.unwrap()).add(val);
                }
                _ => panic!("Untreated case {} {:?} {:?}", mnemonic, arg1, arg2),
            },

            Mnemonic::Res => {
                let bit = self.get_value(arg1.unwrap()).unwrap() as _;
                self.get_register_8_mut(arg2.unwrap()).res_bit(bit)
            }

            Mnemonic::Set => {
                let bit = self.get_value(arg1.unwrap()).unwrap() as _;
                self.get_register_8_mut(arg2.unwrap()).set_bit(bit)
            }

            Mnemonic::Ret => self.ret(),

            Mnemonic::Pop => {
                let word = self.read_memory_word(self.sp().value());
                self.bc_mut().set(word);
                self.sp_mut().add(2);
            }

            Mnemonic::Ldi => {
                let byte = self.read_memory_byte(self.hl().value());
                self.write_memory_byte(self.de().value(), byte);
                self.bc_mut().inc();
                self.de_mut().inc();
                self.hl_mut().inc();
            }

            Mnemonic::ExHlDe => self.ex_de_hl(),

            Mnemonic::Inc => match arg1 {
                Some(&DataAccess::Register8(ref reg)) => {
                    self.get_register_8_mut(arg1.unwrap()).inc()
                }
                Some(&DataAccess::Register16(ref reg)) => {
                    self.get_register_16_mut(arg1.unwrap()).inc()
                }
                _ => unreachable!(),
            },

            Mnemonic::Dec => match arg1 {
                Some(&DataAccess::Register8(ref reg)) => {
                    self.get_register_8_mut(arg1.unwrap()).dec()
                }
                Some(&DataAccess::Register16(ref reg)) => {
                    self.get_register_16_mut(arg1.unwrap()).dec()
                }
                _ => unreachable!(),
            },

            Mnemonic::Ld => match (arg1, arg2) {
                // Load in reg8
                (Some(&DataAccess::Register8(_)), Some(_)) => {
                    let val = self
                        .get_value(arg2.unwrap())
                        .expect(&format!("Unable to get value of {:?}", &arg2));
                    self.get_register_8_mut(arg1.unwrap()).set(val as u8);
                }

                // Load in reg16
                (Some(&DataAccess::Register16(ref reg16)), Some(_)) => {
                    let val = self.get_value(arg2.unwrap()).unwrap();
                    self.get_register_16_mut(arg1.unwrap()).set(val);
                }

                // Write in memory
                (Some(&DataAccess::MemoryRegister16(ref reg)), Some(_)) => {
                    let address = self.get_value(arg1.unwrap()).unwrap();
                    let value = self.get_value(arg2.unwrap()).unwrap();
                    self.write_memory_byte(address, value as _);
                }

                _ => panic!("Untreated case {} {:?} {:?}", mnemonic, arg1, arg2),
            },

            _ => panic!("Untreated case {} {:?} {:?}", mnemonic, arg1, arg2),
        }
    }

    /// TODO need to manage memory
    fn write_memory_byte(&self, _addr: u16, _val: u8) {}

    /// TODO need to manage memory
    fn read_memory_byte(&self, _addr: u16) -> u8 {
        0
    }

    fn read_memory_word(&self, addr: u16) -> u16 {
        let low = self.read_memory_byte(addr);
        let high = self.read_memory_byte(addr + 1); // TODO manage overflow case
        low as u16 + high as u16 * 256
    }

    /// Read the value provided by the given access.
    /// None is returned if we do not have enough information to get it
    /// TODO better emulation to never return None
    fn get_value(&self, access: &DataAccess) -> Option<u16> {
        match access {
            &DataAccess::Memory(ref exp) => self
                .eval_expr(exp)
                .map(|address| self.read_memory_byte(address) as u16),
            &DataAccess::IndexRegister16WithIndex(_, _) => None,
            &DataAccess::IndexRegister16(_) => Some(self.get_register_16(access).value() as _),
            &DataAccess::IndexRegister8(_) => Some(self.get_register_8(access).value() as _),
            &DataAccess::Register16(_) => Some(self.get_register_16(access).value() as _),
            &DataAccess::Register8(_) => Some(self.get_register_8(access).value() as _),
            &DataAccess::MemoryRegister16(ref reg) => Some(
                self.read_memory_byte(
                    self.get_register_16(&DataAccess::Register16(reg.clone()))
                        .value(),
                ) as u16,
            ),
            &DataAccess::Expression(ref expr) => self.eval_expr(expr),
            &DataAccess::FlagTest(_) => panic!(),
            _ => unimplemented!(),
        }
    }

    /// Returns the register encoded by the DataAccess
    fn get_register_16(&self, reg: &DataAccess) -> &crate::z80emu::z80::Register16 {
        match reg {
            &DataAccess::IndexRegister16(ref reg) => match reg {
                IndexRegister16::Ix => self.ix(),
                IndexRegister16::Iy => self.iy(),
            },
            &DataAccess::Register16(ref reg) => match reg {
                crate::assembler::tokens::Register16::Af => self.af(),
                crate::assembler::tokens::Register16::Bc => self.bc(),
                crate::assembler::tokens::Register16::De => self.de(),
                crate::assembler::tokens::Register16::Hl => self.hl(),
                crate::assembler::tokens::Register16::Sp => self.sp(),
            },
            _ => unreachable!(),
        }
    }

    fn get_register_16_mut(&mut self, reg: &DataAccess) -> &mut crate::z80emu::z80::Register16 {
        match reg {
            &DataAccess::IndexRegister16(ref reg) => match reg {
                IndexRegister16::Ix => self.ix_mut(),
                IndexRegister16::Iy => self.iy_mut(),
            },
            &DataAccess::Register16(ref reg) => match reg {
                crate::assembler::tokens::Register16::Af => self.af_mut(),
                crate::assembler::tokens::Register16::Bc => self.bc_mut(),
                crate::assembler::tokens::Register16::De => self.de_mut(),
                crate::assembler::tokens::Register16::Hl => self.hl_mut(),
                crate::assembler::tokens::Register16::Sp => self.sp_mut(),
            },
            _ => unreachable!(),
        }
    }

    fn get_register_8(&self, reg: &DataAccess) -> &crate::z80emu::z80::Register8 {
        match reg {
            &DataAccess::Register8(ref reg) => match reg {
                crate::assembler::tokens::Register8::A => self.a(),
                crate::assembler::tokens::Register8::B => self.b(),
                crate::assembler::tokens::Register8::D => self.d(),
                crate::assembler::tokens::Register8::H => self.h(),
                crate::assembler::tokens::Register8::C => self.c(),
                crate::assembler::tokens::Register8::E => self.e(),
                crate::assembler::tokens::Register8::L => self.l(),
            },

            &DataAccess::IndexRegister8(ref reg) => match reg {
                IndexRegister8::Ixl => self.ixl(),
                IndexRegister8::Ixh => self.ixh(),
                IndexRegister8::Iyl => self.iyl(),
                IndexRegister8::Iyh => self.iyh(),
            },
            _ => panic!(),
        }
    }

    // Mutable version to be synced with the immutable one
    fn get_register_8_mut(&mut self, reg: &DataAccess) -> &mut crate::z80emu::z80::Register8 {
        match reg {
            &DataAccess::Register8(ref reg) => match reg {
                crate::assembler::tokens::Register8::A => self.a_mut(),
                crate::assembler::tokens::Register8::B => self.b_mut(),
                crate::assembler::tokens::Register8::D => self.d_mut(),
                crate::assembler::tokens::Register8::H => self.h_mut(),
                crate::assembler::tokens::Register8::C => self.c_mut(),
                crate::assembler::tokens::Register8::E => self.e_mut(),
                crate::assembler::tokens::Register8::L => self.l_mut(),
            },

            &DataAccess::IndexRegister8(ref reg) => match reg {
                IndexRegister8::Ixl => self.ixl_mut(),
                IndexRegister8::Ixh => self.ixh_mut(),
                IndexRegister8::Iyl => self.iyl_mut(),
                IndexRegister8::Iyh => self.iyh_mut(),
            },
            _ => panic!(),
        }
    }

    fn eval_expr(&self, expr: &Expr) -> Option<u16> {
        match expr.eval() {
            Ok(val) => Some(val as u16),
            Err(_) => None,
        }
    }

    pub fn ret(&mut self) {
        let address = self.read_memory_word(self.sp().value());
        self.sp_mut().add(2);
        self.pc_mut().set(address);
    }
}
