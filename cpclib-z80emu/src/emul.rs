use crate::preamble::*;
use crate::z80::*;

impl Z80 {

  


    /// Execute the given token.
    /// XXX Currently only OpCode are managed whereas some other
    /// tokens also have a sense there
    /// BUGGY flags are not properly updated
    /// Returns the number of noprs
    pub fn execute(&mut self, opcode: &Token) -> usize {
        self.context
            .symbols
            .set_symbol_to_value("$", self.pc().value() as i32);

        match opcode {
            Token::OpCode(ref mnemonic, ref arg1, ref arg2) => {
                self.execute_opcode(*mnemonic, arg1.as_ref(), arg2.as_ref())
            },

            // Transform the raw data as real opcodes
            // and indiivudally execute them
            // crash when it is not possible to disassemble
            Token::Defs(_, _) | Token::Defb(_) | Token::Defw(_) => {
                let lst = opcode.disassemble_data().unwrap();
                lst.listing().iter()
                    .map(|token|{self.execute(token)})
                    .sum()
            },

            _ => panic!("{:?} is not yet handled", opcode),
        }
    }

    /// Execute the given opcode. Parameters are assumed to be valid.
    /// PC has already been incremented.
    /// Returns the duration (that can depend on the value of flags/registers)
    fn execute_opcode(
        &mut self,
        mnemonic: Mnemonic,
        arg1: Option<&DataAccess>,
        arg2: Option<&DataAccess>,
    ) -> usize {

        let opcode = Token::OpCode(mnemonic, arg1.cloned(), arg2.cloned());
        self.pc_mut().add(opcode.number_of_bytes().unwrap() as _);


        // this is the minimal duration; it can be updated depending on the instruction
        let mut duration = opcode.estimated_duration()
                                .unwrap();
        let mut inc_duration = ||{
            duration += 1;
        };

        match mnemonic {
            Mnemonic::Add => match (arg1, arg2) {
                (Some(&DataAccess::Register8(tokens::Register8::A)), Some(_)) => {
                    let val = self.get_value(arg1.unwrap()).unwrap();
                    self.get_register_8_mut(arg1.unwrap()).add(val as _);
                }

                (
                    Some(&DataAccess::Register16(tokens::Register16::Hl)),
                    Some(_),
                ) => {
                    let val = self.get_value(arg2.unwrap()).unwrap();
                    if mnemonic == Mnemonic::Add {
                        self.get_register_16_mut(arg1.unwrap()).add(val);
                    } else {
                        self.get_register_16_mut(arg1.unwrap()).sub(val);
                    }
                }
                _ => panic!("Untreated case {} {:?} {:?}", mnemonic, arg1, arg2),
            },

            Mnemonic::Sub => match (arg1, arg2) {
                (Some(_), None) => {
                    let val = self.get_value(arg1.unwrap()).unwrap();
                    self.a_mut().sub(val as _);
                }
                _ => unimplemented!(),
            },

            Mnemonic::And => {
                let val = self.get_value(arg1.unwrap()).unwrap() as _;
                self.get_register_8_mut(&tokens::Register8::A.into()).and(val);
            }

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
                Some(&DataAccess::Register8(ref _reg)) => {
                    self.get_register_8_mut(arg1.unwrap()).inc()
                }
                Some(&DataAccess::Register16(ref _reg)) => {
                    self.get_register_16_mut(arg1.unwrap()).inc()
                }
                _ => unreachable!(),
            },

            Mnemonic::Dec => match arg1 {
                Some(&DataAccess::Register8(ref _reg)) => {
                    self.get_register_8_mut(arg1.unwrap()).dec()
                }
                Some(&DataAccess::Register16(ref _reg)) => {
                    self.get_register_16_mut(arg1.unwrap()).dec()
                }
                _ => unreachable!(),
            },

            Mnemonic::Djnz => {
                // dec b
                self.b_mut().dec();

                // jump as soon as b != 0
                if self.b().value() != 0 {
                    inc_duration();

                    let delta = match arg1 {
                        Some(&DataAccess::Expression(Expr::Label(_))) => {
                            self.get_value(arg1.unwrap()).unwrap() as i32
                                - self
                                    .get_value(&DataAccess::Expression(Expr::Label("$".to_owned())))
                                    .unwrap() as i32
                                - 2
                        }
                        _ => self.get_value(arg2.unwrap()).unwrap() as i32,
                    };

                    if delta > 0 {
                        self.pc_mut().add(delta as _);
                    } else if delta < 0 {
                        self.pc_mut().sub(-delta as _);
                    }
                    else {
                        // go back at the beginning of the instruction
                        self.pc_mut().sub(opcode.number_of_bytes().unwrap() as _);
                    }
                }
                
            }

            Mnemonic::Jr => {
                dbg!(arg2);
                let delta = match arg2 {
                    Some(&DataAccess::Expression(Expr::Label(_))) => {
                        self.get_value(arg2.unwrap()).unwrap() as i32
                            - self
                                .get_value(&DataAccess::Expression(Expr::Label("$".to_owned())))
                                .unwrap() as i32
                            - 2
                    }
                    _ => self.get_value(arg2.unwrap()).unwrap() as i32,
                };
                if match arg1 {
                    None => true,
                    Some(DataAccess::FlagTest(ref flag)) => self.is_flag_active(flag),
                    _ => unreachable!(),
                } {
                    inc_duration();

                    if delta > 0 {
                        self.pc_mut().add(delta as _);
                    } else if delta < 0 {
                        self.pc_mut().sub(-delta as _);
                    }
                    else if delta == 0 {
                        self.pc_mut().sub(opcode.number_of_bytes().unwrap() as _);
                    }
                }
            }

            Mnemonic::Jp => match (arg1, arg2) {
                (None, Some(_)) => {
                    let value = self.get_value(arg2.unwrap()).unwrap();
                    self.pc_mut().set(value);
                }

                (Some(DataAccess::FlagTest(ref flag)), _) => {
                    if self.is_flag_active(flag) {
                        // BUGGY when label are used
                        // it would be better to ensure there are never labels in the stream of opcodes
                        let value = self.get_value(arg2.unwrap()).unwrap();
                        self.pc_mut().set(value);
                        inc_duration();
                    }
                    else {
                        self.pc_mut().sub(opcode.number_of_bytes().unwrap() as _);

                    }
                }

                _ => unreachable!(),
            },

            Mnemonic::Ld => match (arg1, arg2) {
                // Load in reg8
                (Some(&DataAccess::Register8(_)), Some(_)) => {
                    let val = self
                        .get_value(arg2.unwrap())
                        .unwrap_or_else(|| panic!("Unable to get value of {:?}", &arg2));
                    self.get_register_8_mut(arg1.unwrap()).set(val as u8);
                }

                // Load in reg16
                (Some(&DataAccess::Register16(ref _reg16)), Some(_)) => {
                    let val = self.get_value(arg2.unwrap()).unwrap();
                    self.get_register_16_mut(arg1.unwrap()).set(val);
                }

                // Write in memory
                (Some(&DataAccess::MemoryRegister16(ref _reg)), Some(_)) => {
                    let address = self.get_value(arg1.unwrap()).unwrap();
                    let value = self.get_value(arg2.unwrap()).unwrap();
                    self.write_memory_byte(address, value as _);
                }

                _ => panic!("Untreated case {} {:?} {:?}", mnemonic, arg1, arg2),
            },

            Mnemonic::Nop => {
                // nothing to do
            }


            _ => panic!("Untreated case {} {:?} {:?}", mnemonic, arg1, arg2),
        }

        duration
    }

    /// TODO need to manage memory
    fn write_memory_byte(&self, _addr: u16, _val: u8) {
        eprintln!("[ERROR] Memory byte not written");
    }

    /// TODO need to manage memory
    fn read_memory_byte(&self, _addr: u16) -> u8 {
        eprintln!("[ERROR] Memory byte not read");
        u8::default()
    }

    fn read_memory_word(&self, addr: u16) -> u16 {
        let low = self.read_memory_byte(addr);
        let high = self.read_memory_byte(addr + 1); // TODO manage overflow case
        u16::from(low) + u16::from(high) * 256
    }

    /// Read the value provided by the given access.
    /// None is returned if we do not have enough information to get it
    /// TODO better emulation to never return None
    fn get_value(&self, access: &DataAccess) -> Option<u16> {
        match access {
            DataAccess::Memory(ref exp) => self
                .eval_expr(exp)
                .map(|address| u16::from(self.read_memory_byte(address))),
            DataAccess::IndexRegister16WithIndex(_, _) => None,
            DataAccess::IndexRegister16(_) | &DataAccess::Register16(_) => {
                Some(self.get_register_16(access).value())
            }
            DataAccess::IndexRegister8(_) | &DataAccess::Register8(_) => {
                Some(self.get_register_8(access).value().into())
            }
            DataAccess::MemoryRegister16(ref reg) => Some(u16::from(
                self.read_memory_byte(self.get_register_16(&DataAccess::Register16(*reg)).value()),
            )),
            DataAccess::Expression(ref expr) => self.eval_expr(expr),
            DataAccess::FlagTest(_) => panic!(),
            _ => unimplemented!(),
        }
    }

    /// Return true if the flag is active
    fn is_flag_active(&self, flag: &FlagTest) -> bool {
        flag.flag_is_active(self.f().value())
    }

    /// Returns the register encoded by the DataAccess
    fn get_register_16(&self, reg: &DataAccess) -> &crate::z80::Register16 {
        match reg {
            DataAccess::IndexRegister16(ref reg) => match reg {
                IndexRegister16::Ix => self.ix(),
                IndexRegister16::Iy => self.iy(),
            },
            DataAccess::Register16(ref reg) => match reg {
                tokens::Register16::Af => self.af(),
                tokens::Register16::Bc => self.bc(),
                tokens::Register16::De => self.de(),
                tokens::Register16::Hl => self.hl(),
                tokens::Register16::Sp => self.sp(),
            },
            _ => unreachable!(),
        }
    }

    fn get_register_16_mut(&mut self, reg: &DataAccess) -> &mut crate::z80::Register16 {
        match reg {
            DataAccess::IndexRegister16(ref reg) => match reg {
                IndexRegister16::Ix => self.ix_mut(),
                IndexRegister16::Iy => self.iy_mut(),
            },
            DataAccess::Register16(ref reg) => match reg {
                tokens::Register16::Af => self.af_mut(),
                tokens::Register16::Bc => self.bc_mut(),
                tokens::Register16::De => self.de_mut(),
                tokens::Register16::Hl => self.hl_mut(),
                tokens::Register16::Sp => self.sp_mut(),
            },
            _ => unreachable!(),
        }
    }

    fn get_register_8(&self, reg: &DataAccess) -> &crate::z80::Register8 {
        match reg {
            DataAccess::Register8(ref reg) => match reg {
                tokens::Register8::A => self.a(),
                tokens::Register8::B => self.b(),
                tokens::Register8::D => self.d(),
                tokens::Register8::H => self.h(),
                tokens::Register8::C => self.c(),
                tokens::Register8::E => self.e(),
                tokens::Register8::L => self.l(),
            },

            DataAccess::IndexRegister8(ref reg) => match reg {
                IndexRegister8::Ixl => self.ixl(),
                IndexRegister8::Ixh => self.ixh(),
                IndexRegister8::Iyl => self.iyl(),
                IndexRegister8::Iyh => self.iyh(),
            },
            _ => panic!(),
        }
    }

    // Mutable version to be synced with the immutable one
    fn get_register_8_mut(&mut self, reg: &DataAccess) -> &mut crate::z80::Register8 {
        match reg {
            DataAccess::Register8(ref reg) => match reg {
                tokens::Register8::A => self.a_mut(),
                tokens::Register8::B => self.b_mut(),
                tokens::Register8::D => self.d_mut(),
                tokens::Register8::H => self.h_mut(),
                tokens::Register8::C => self.c_mut(),
                tokens::Register8::E => self.e_mut(),
                tokens::Register8::L => self.l_mut(),
            },

            DataAccess::IndexRegister8(ref reg) => match reg {
                IndexRegister8::Ixl => self.ixl_mut(),
                IndexRegister8::Ixh => self.ixh_mut(),
                IndexRegister8::Iyl => self.iyl_mut(),
                IndexRegister8::Iyh => self.iyh_mut(),
            },
            _ => panic!(),
        }
    }

    #[allow(clippy::cast_sign_loss)]
    fn eval_expr(&self, expr: &Expr) -> Option<u16> {
        match expr.resolve(&self.context.symbols) {
            Ok(val) => Some(val.abs() as u16),
            Err(_) => None,
        }
    }

    /// Replace the current symbol table by a copy of the one in argument
    pub fn setup_symbol_table(&mut self, symbols: &SymbolsTableCaseDependent) {
        self.context.symbols = symbols.clone();
    }

    /// Execute the RET instruction
    pub fn ret(&mut self) {
        let address = self.read_memory_word(self.sp().value());
        self.sp_mut().add(2);
        self.pc_mut().set(address);
    }
}

pub trait FlagIsActive{
    fn flag_is_active(self, f_value: u8) -> bool;
}

impl FlagIsActive for &FlagTest {
    /// Add extra functionality to flag test
    fn flag_is_active(self, f_value: u8) -> bool {
        (match self {
            FlagTest::C => f_value & (1 << FlagPos::Carry as u8),
            FlagTest::NC => !(f_value & (1 << FlagPos::Carry as u8)),

            FlagTest::Z => f_value & (1 << FlagPos::Zero as u8),
            FlagTest::NZ => !(f_value & (1 << FlagPos::Zero as u8)),

            FlagTest::PO => f_value & (1 << FlagPos::Parity as u8),
            FlagTest::PE => !(f_value & (1 << FlagPos::Parity as u8)),

            FlagTest::P => f_value & (1 << FlagPos::Sign as u8),
            FlagTest::M => !(f_value & (1 << FlagPos::Sign as u8)),
        }) != 0
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_flags() {
        assert!(FlagTest::Z.flag_is_active(0b01000000));
        assert!(FlagTest::NZ.flag_is_active(0b00000000));
    }

    #[test]
    fn jp_value() {

        let mut z80 = Z80::default();
        z80.pc_mut().set(0x4000);

        assert_eq!(z80.pc().value(), 0x4000);

        z80.execute(&Token::OpCode(
            Mnemonic::Jp,
            None,
            Some(DataAccess::Expression(Expr::Value(0x4000))),
        ));

        assert_eq!(z80.pc().value(), 0x4000);
    }

    #[test]
    fn jp_symbol() {

        let mut z80 = Z80::default();
        let mut symbols = SymbolsTableCaseDependent::default();
        symbols.set_symbol_to_value("LABEL", 0x4000);
        z80.setup_symbol_table(&symbols);

        z80.pc_mut().set(0x4000);

        assert_eq!(z80.pc().value(), 0x4000);

        z80.execute(&Token::OpCode(
            Mnemonic::Jp,
            None,
            Some(DataAccess::Expression(Expr::Label("LABEL".to_owned()))),
        ));

        assert_eq!(z80.pc().value(), 0x4000);
    }

    #[test]
    fn jp_dollar() {

        let mut z80 = Z80::default();
        z80.pc_mut().set(0x4000);

        assert_eq!(z80.pc().value(), 0x4000);

        z80.execute(&Token::OpCode(
            Mnemonic::Jp,
            None,
            Some(DataAccess::Expression(Expr::Label("$".to_owned()))),
        ));

        assert_eq!(z80.pc().value(), 0x4000);
    }

    #[test]
    fn jr_dollar() {

        let mut z80 = Z80::default();
        z80.pc_mut().set(0x4000);

        assert_eq!(z80.pc().value(), 0x4000);

        z80.execute(&Token::OpCode(
            Mnemonic::Jr,
            None,
            Some(DataAccess::Expression(Expr::Label("$".to_owned()))),
        ));

        assert_eq!(z80.pc().value(), 0x4000);
    }
}
