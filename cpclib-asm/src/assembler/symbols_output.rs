use std::io::Write;

use cpclib_common::itertools::Itertools;
use cpclib_tokens::symbols::{Symbol, SymbolsTableTrait, Value};
use cpclib_tokens::ExprResult;

/// Manage the generation of the symbols output.
/// Could be parametrize by some directives
#[derive(Clone)]
pub struct SymbolOutputGenerator {
    forbidden: Vec<Symbol>,
    allowed: Vec<Symbol>,

    all_forbidden: bool,
    all_allowed: bool
}

impl Default for SymbolOutputGenerator {
    fn default() -> Self {
        Self {
            forbidden: Vec::new(),
            allowed: Vec::new(),

            all_forbidden: false,
            all_allowed: true
        }
    }
}

impl SymbolOutputGenerator {
    /// Generate the symbol table in w
    pub fn generate<W: Write>(
        &self,
        w: &mut W,
        symbs: &impl SymbolsTableTrait
    ) -> std::io::Result<()> {
        for (k, v) in symbs
            .expression_symbol()
            .iter()
            .filter(|(s, _v)| self.keep_symbol(s))
            .sorted_by_key(|(s, _v)| s.to_string().to_ascii_lowercase())
        {
            match v {
                Value::Address(a) => {
                    writeln!(w, "{} equ #{:04X}", k.value(), a.address())
                }
                Value::Expr(ExprResult::Value(i)) => {
                    writeln!(w, "{} equ #{:04X}", k.value(), i)
                }
                Value::Expr(ExprResult::Bool(b)) => {
                    writeln!(w, "{} equ {}", k.value(), *b)
                }
                Value::Expr(e @ ExprResult::Float(_f)) => {
                    writeln!(w, "{} equ #{:04X}", k.value(), e.int().unwrap())
                }
                Value::Expr(ExprResult::String(s)) => {
                    writeln!(w, "{} equ {}", k.value(), s)
                }
                Value::Expr(l @ ExprResult::List(_)) => {
                    writeln!(w, "{} equ {}", k.value(), l)
                }
                Value::Expr(m @ ExprResult::Matrix { .. }) => {
                    writeln!(w, "{} equ {}", k.value(), m)
                }

                _ => unimplemented!("{:?}", v)
            }?;
        }

        Ok(())
    }

    /// Returns true if the symbol needs to be printed
    pub fn keep_symbol(&self, sym: &Symbol) -> bool {
        assert!(self.all_allowed ^ self.all_forbidden);

        if sym.value() == "$" {
            return false;
        }
        if sym.value() == "$$" {
            return false;
        }
        else if self.all_allowed {
            !Self::is_included(&self.forbidden, sym)
        }
        else
        // if self.all_forbidden
        {
            Self::is_included(&self.allowed, sym)
        }
    }

    fn is_included(list: &[Symbol], sym: &Symbol) -> bool {
        list.iter()
            .find(|s2| {
                if **s2 == *sym {
                    return true;
                }
                // if !s2.value().contains(".") {return false;}
                sym.value().starts_with(&format!("{}.", s2.value()))
            })
            .is_some()
    }

    pub fn forbid_all_symbols(&mut self) {
        self.forbidden.clear();
        self.all_forbidden = true;
        self.all_allowed = false;
    }

    pub fn allow_all_symbols(&mut self) {
        self.allowed.clear();
        self.all_allowed = true;
        self.all_forbidden = false;
    }

    pub fn forbid_symbol<S: Into<Symbol>>(&mut self, s: S) {
        self.forbidden.push(s.into());
    }

    pub fn allow_symbol<S: Into<Symbol>>(&mut self, s: S) {
        self.allowed.push(s.into());
    }
}
