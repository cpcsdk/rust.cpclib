use std::fmt::Debug;
use std::sync::Arc;

use cpclib_common::itertools::Itertools;
use cpclib_common::smallvec::SmallVec;
use cpclib_tokens::symbols::*;
use cpclib_tokens::tokens::*;

use crate::assembler::{Env, Visited};
use crate::error::*;
use crate::implementation::expression::ExprEvaluationExt;
use crate::implementation::listing::ListingExt;
use crate::{AssemblingOptions, EnvOptions};

/// Needed methods for the Token defined in cpclib_tokens
pub trait TokenExt: ListingElement + Debug + Visited {
    fn estimated_duration(&self) -> Result<usize, Box<AssemblerError>>;
    /// Unroll the tokens when it represents a loop
    fn unroll(&self, env: &mut crate::Env) -> Option<Result<Vec<&Self>, Box<AssemblerError>>>;

    /// Generate the listing of opcodes for directives that embed bytes
    fn disassemble_data(&self) -> Result<Listing, String>;

    fn to_bytes_with_options(&self, option: EnvOptions) -> Result<Vec<u8>, Box<AssemblerError>> {
        let mut env = Env::new(option);
        // we need several passes in case the token is a directive that contains code
        loop {
            let _ = env.start_new_pass();
            // println!("[pass] {:?}", env.pass);

            if env.pass().is_finished() {
                break;
            }

            self.visited(&mut env)?;
        }

        Ok(env.produced_bytes())
    }

    /// Returns the number of bytes of the instructions by assembling it
    fn number_of_bytes(&self) -> Result<usize, String> {
        let bytes = self.to_bytes();
        if bytes.is_ok() {
            Ok(bytes.ok().unwrap().len())
        }
        else {
            Err(format!("Unable to get the bytes of this token: {self:?}"))
        }
    }

    /// returns the number of bytes without assembling it
    fn fallback_number_of_bytes(&self) -> Result<usize, String>;

    /// Return the number of bytes of the token given the provided context
    fn number_of_bytes_with_context(&self, table: &mut SymbolsTable) -> Result<usize, String> {
        let bytes = self.to_bytes_with_context(table);
        if bytes.is_ok() {
            Ok(bytes.ok().unwrap().len())
        }
        else {
            eprintln!("{bytes:?}");
            Err(format!("Unable to get the bytes of this token: {self:?}"))
        }
    }

    /// Dummy version that assemble without taking into account the context
    /// TODO find a way to not build a symbol table each time
    fn to_bytes(&self) -> Result<Vec<u8>, Box<AssemblerError>> {
        let mut table = SymbolsTable::laxist();
        let table = &mut table;
        self.to_bytes_with_context(table)
    }

    /// Assemble the symbol taking into account some context, but never modify this context
    #[allow(clippy::match_same_arms)]
    fn to_bytes_with_context(
        &self,
        table: &mut SymbolsTable
    ) -> Result<Vec<u8>, Box<AssemblerError>> {
        let mut options = if table.is_case_sensitive() {
            AssemblingOptions::new_case_sensitive()
        }
        else {
            AssemblingOptions::new_case_insensitive()
        };
        options.set_symbols(table);

        let options = EnvOptions::new(Default::default(), options, Arc::new(()));
        self.to_bytes_with_options(options)
    }

    /// Check if the token is valid. We consider a token vlaid if it is possible to assemble it
    fn is_valid(&self) -> bool {
        self.to_bytes().is_ok()
    }
}

// impl<'t> TokenExt for Cow<'t, Token> {
// fn disassemble_data(&self) -> Result<Listing, String> {
// self.deref().disassemble_data()
// }
//
// fn estimated_duration(&self) -> Result<usize, Box<AssemblerError>> {
// self.deref().estimated_duration()
// }
//
// fn to_bytes_with_options(&self, option: &AssemblingOptions) -> Result<Vec<u8>, Box<AssemblerError>> {
// self.deref().to_bytes_with_options(option)
// }
//
// fn unroll(&self, _env: &crate::Env) -> Option<Result<Vec<&Self>, Box<AssemblerError>>> {
// unimplemented!("signature issue. should be transformed/unused")
// }
// }

impl TokenExt for Token {
    /// Unroll the tokens when in a repetition loop
    /// TODO return an iterator in order to not produce the vector each time
    fn unroll(&self, env: &mut crate::Env) -> Option<Result<Vec<&Self>, Box<AssemblerError>>> {
        if let Token::Repeat(expr, tokens, _counter_label, _counter_start) = self {
            let count: Result<ExprResult, Box<AssemblerError>> = expr.resolve(env);
            if count.is_err() {
                Some(Err(count.err().unwrap()))
            }
            else {
                let count = count.unwrap().int().unwrap();
                let mut res = Vec::with_capacity(count as usize * tokens.len());
                for _i in 0..count {
                    // TODO add a specific token to control the loop counter (and change the return type)
                    for t in tokens.iter() {
                        res.push(t);
                    }
                }
                Some(Ok(res))
            }
        }
        else {
            None
        }
    }

    /// Generate the listing of opcodes for directives that contain data Defb/defw/Defs in order to have
    /// mnemonics. Fails when some values are not opcodes
    fn disassemble_data(&self) -> Result<Listing, String> {
        // Disassemble the bytes and return the listing ONLY if it has no more defb/w/s directives
        let wrap = |bytes: &[u8]| {
            use crate::disass::disassemble;

            let lst = disassemble(bytes);
            for token in lst.listing() {
                match token {
                    Token::Defb(_) | Token::Defw(_) | Token::Defs(_) => {
                        return Err(format!("{token} as not been disassembled"));
                    },
                    _ => {}
                }
            }

            Ok(lst)
        };

        match self {
            Token::Defs(l) => {
                let mut env = Env::default();
                l.iter()
                    .map(|(e, f)| {
                        env.assemble_defs_item(e, f.as_ref())
                            .map_err(|err| format!("Unable to assemble {self}: {err:?}"))
                    })
                    .fold_ok(SmallVec::<[u8; 4]>::new(), |mut acc, v| {
                        acc.extend_from_slice(v.as_slice());
                        acc
                    })
                    .and_then(|b| wrap(&b))
            },

            Token::Defb(e) | Token::Defw(e) | Token::Str(e) => {
                let mut env = Env::default();
                env.visit_db_or_dw_or_str(self.into(), e, 0.into())
                    .map_err(|err| format!("Unable to assemble {self}: {err:?}"))?;
                wrap(&env.produced_bytes())
            },

            _ => {
                let mut lst = Listing::new();
                lst.push(self.clone());
                Ok(lst)
            }
        }
    }

    /// Returns an estimation of the duration.
    /// This estimation may be wrong for instruction having several states.
    #[allow(clippy::match_same_arms)]
    fn estimated_duration(&self) -> Result<usize, Box<AssemblerError>> {
        let duration = match self {
            // A warning-wrapped token (fake instruction, redundant explicit
            // accumulator prefix, ...) has the same duration as the real
            // instruction it wraps - the warning itself has no effect on
            // timing.
            Token::WarningWrapper(inner, _) => return inner.estimated_duration(),

            Token::Assert(..)
            | Token::Breakpoint { .. }
            | Token::Comment(_)
            | Token::Label(_)
            | Token::Equ { .. }
            | Token::Protect(..) => 0,

            Token::Repeat(Expr::Value(count), lst, ..) if *count >= 0 => {
                let lst_count = lst.estimated_duration()?;
                lst_count * (*count as usize)
            },

            // Here, there is a strong limitation => it will works only if no symbols are used
            Token::Defw(_) | Token::Defb(_) | Token::Defs(_) => {
                self.disassemble_data()
                    .map_err(|e| Box::new(AssemblerError::DisassemblerError { msg: e }))?
                    .estimated_duration()?
            },

            Token::OpCode(mnemonic, arg1, arg2, _arg3) => {
                // Delegated to `cpclib-z80flow`, the one place in this
                // workspace that knows how long an opcode takes. It used to
                // live here, 464 lines of it, while `cpclib-lsp` carried a
                // second statement of the same facts in `data/timings.txt` -
                // two tables that could disagree about what `duration()` and
                // the editor's cycle counter each told the user.
                //
                // `NOP n` (basm's repeated-nop form) stays here: it needs real
                // expression evaluation, which is this crate's business.
                if *mnemonic == Mnemonic::Nop
                    && let Some(arg) = self.mnemonic_arg1()
                {
                    arg.get_expression().unwrap().eval()?.int()? as _
                }
                else {
                    cpclib_z80flow::cost::opcode_duration(mnemonic, arg1.as_ref(), arg2.as_ref())
                        .ok_or_else(|| {
                        Box::new(AssemblerError::BugInAssembler {
                            file: file!(),
                            line: line!(),
                            msg: format!(
                                "Duration not set for {mnemonic:?}, {arg1:?}, {arg2:?}"
                            )
                        })
                    })? as usize
                }
            },
            _ => {
                return Err(Box::new(AssemblerError::BugInAssembler {
                    file: file!(),
                    line: line!(),
                    msg: format!("Duration computation for {self:?} not yet coded")
                }));
            }
        };
        Ok(duration)
    }

    fn fallback_number_of_bytes(&self) -> Result<usize, String> {
        match self {
            // A warning-wrapped token (fake instruction, redundant explicit
            // accumulator prefix, ...) has the same byte size as the real
            // instruction it wraps.
            Self::WarningWrapper(inner, _) => inner.fallback_number_of_bytes(),
            Self::OpCode(mne, arg1, arg2, arg3) => {
                let arg1 = arg1.as_ref().map(|arg| arg.replace_expressions_by_0());
                let arg2 = arg2.as_ref().map(|arg| arg.replace_expressions_by_0());

                Self::OpCode(*mne, arg1, arg2, *arg3).number_of_bytes()
            },
            Self::Comment(..) | Self::Label(..) | Self::Assert(..) => Ok(0),
            _ => {
                Result::Err(format!(
                    "fallback_number_of_bytes not implemented for {self}"
                ))
            },
        }
    }
}

#[cfg(test)]
#[allow(clippy::pedantic)]
#[allow(warnings)]
mod tests {
    use crate::preamble::*;

    #[test]
    fn test_timing2() {
        // We are only able to disassemble nop ...
        assert_eq!(defs_expr_expr(10, 0).estimated_duration().unwrap(), 10);
        assert_eq!(defw(0).estimated_duration().unwrap(), 2);
        assert_eq!(defb(0).estimated_duration().unwrap(), 1);

        assert_eq!(exx().estimated_duration().unwrap(), 1);

        assert_eq!(pop_de().estimated_duration().unwrap(), 3);

        assert_eq!(inc_l().estimated_duration().unwrap(), 1);

        assert_eq!(jp_label("XX").estimated_duration().unwrap(), 3);

        assert_eq!(ld_l_mem_ix(14.into()).estimated_duration().unwrap(), 5);

        assert_eq!(ld_mem_hl_e().estimated_duration().unwrap(), 2);

        assert_eq!(ld_e_mem_hl().estimated_duration().unwrap(), 2);

        assert_eq!(ld_d_mem_hl().estimated_duration().unwrap(), 2);

        assert_eq!(out_c_d().estimated_duration().unwrap(), 4);
    }

    #[test]
    fn is_valid_ok() {
        assert!(out_c_d().is_valid());
    }

    #[test]
    fn is_valid_nok() {
        assert!(
            !Token::OpCode(
                Mnemonic::Out,
                Some(DataAccess::Register8(Register8::C)),
                Some(DataAccess::Register8(Register8::A)),
                None
            )
            .is_valid()
        );
    }

    #[cfg(test)]
    mod test {

        use ParseToken;

        use super::*;
        #[test]
        fn fixup_duration() {
            assert_eq!(
                Token::parse_token(" di")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                1
            );
            assert_eq!(
                Token::parse_token(" add a,c ")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                1
            );
            assert_eq!(
                Token::parse_token(" ld l, a")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                1
            );
            assert_eq!(
                Token::parse_token(" ld b, e")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                1
            );
            assert_eq!(
                Token::parse_token(" ld e, b")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                1
            );
            assert_eq!(
                Token::parse_token(" exx")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                1
            );
            assert_eq!(
                Token::parse_token(" push bc")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                4
            );
            assert_eq!(
                Token::parse_token(" pop bc")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                3
            );
            assert_eq!(
                Token::parse_token(" push ix")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                5
            );
            assert_eq!(
                Token::parse_token(" pop ix")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                4
            );
            assert_eq!(
                Token::parse_token(" ld b, nnn")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                2
            );
            assert_eq!(
                Token::parse_token(" ld e, (hl)")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                2
            );
            assert_eq!(
                Token::parse_token(" ld d, (hl)")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                2
            );
            assert_eq!(
                Token::parse_token(" ld a, (hl)")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                2
            );
            assert_eq!(
                dbg!(Token::parse_token(" ld a, (dd)").unwrap())
                    .estimated_duration()
                    .unwrap(),
                4
            );
            assert_eq!(
                Token::parse_token(" ld hl, (dd)")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                5
            );
            println!("{:?}", Token::parse_token(" ld de, (dd)").unwrap());
            assert_eq!(
                Token::parse_token(" ld de, (dd)")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                6
            );
            assert_eq!(
                Token::parse_token(" ld a, (ix+0)")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                5
            );
            assert_eq!(
                Token::parse_token(" ld l, (ix+0)")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                5
            );
            assert_eq!(
                Token::parse_token(" ldi")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                5
            );
            assert_eq!(
                Token::parse_token(" inc c")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                1
            );
            assert_eq!(
                Token::parse_token(" inc l")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                1
            );
            assert_eq!(
                Token::parse_token(" dec c")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                1
            );
            assert_eq!(
                Token::parse_token(" out (c), d")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                4
            );
            assert_eq!(
                Token::parse_token(" out (c), c")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                4
            );
            assert_eq!(
                Token::parse_token(" out (c), e")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                4
            );
            assert_eq!(
                Token::parse_token(" ld b, 0x7f")
                    .unwrap()
                    .estimated_duration()
                    .unwrap(),
                2
            );

            assert!(
                Token::Basic(None, None, "".to_owned())
                    .estimated_duration()
                    .is_err()
            );
        }
    }
}
