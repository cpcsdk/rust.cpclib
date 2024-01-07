use std::io::Write;
use std::str::FromStr;

use cpclib_common::itertools::Itertools;
use cpclib_sna::{AceSymbolChunk, AceSymbol, AceSymbolType};
use cpclib_tokens::symbols::{Symbol, SymbolsTableTrait, Value};
use cpclib_tokens::ExprResult;

pub enum SymbolOutputFormat {
    Basm,
    Winape
}

impl SymbolOutputFormat {
    pub fn format(&self, k: &Symbol, v: &Value) -> String {
        match self {
            SymbolOutputFormat::Basm => {
                match v {
                    Value::Address(a) => {
                        format!("{} equ #{:04X}", k.value(), a.address())
                    },
                    Value::Expr(ExprResult::Value(i)) => {
                        format!("{} equ #{:04X}", k.value(), i)
                    },
                    Value::Expr(ExprResult::Bool(b)) => {
                        format!("{} equ {}", k.value(), *b)
                    },
                    Value::Expr(e @ ExprResult::Float(_f)) => {
                        format!("{} equ #{:04X}", k.value(), e.int().unwrap())
                    },
                    Value::Expr(ExprResult::String(s)) => {
                        format!("{} equ {}", k.value(), s)
                    },
                    Value::Expr(l @ ExprResult::List(_)) => {
                        format!("{} equ {}", k.value(), l)
                    },
                    Value::Expr(m @ ExprResult::Matrix { .. }) => {
                        format!("{} equ {}", k.value(), m)
                    },

                    _ => unimplemented!("{:?}", v)
                }
            },
            SymbolOutputFormat::Winape => {
                match v {
                    Value::Address(a) => {
                        format!("{} #{:X}", k.value(), a.address())
                    },
                    Value::Expr(ExprResult::Value(i)) => {
                        format!("{} #{:X}", k.value(), i)
                    },
                    Value::Expr(ExprResult::Bool(b)) => {
                        format!("{} {}", k.value(), *b)
                    },
                    Value::Expr(e @ ExprResult::Float(_f)) => {
                        format!("{} #{:X}", k.value(), e.int().unwrap())
                    },
                    Value::Expr(ExprResult::String(_s)) => {
                        "".to_owned() // ignored by winape
                    },
                    Value::Expr(_l @ ExprResult::List(_)) => {
                        "".to_owned() // ignored by winape
                    },
                    Value::Expr(_m @ ExprResult::Matrix { .. }) => {
                        "".to_owned() // ignored by winape
                    },

                    _ => unimplemented!("{:?}", v)
                }
            }
        }
    }
}

impl FromStr for SymbolOutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "basm" => Ok(Self::Basm),
            "winape" => Ok(Self::Winape),
            _ => Err(format!("Wrong symbol format {s}"))
        }
    }
}

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
    pub fn build_ace_snapshot_chunk(&self, symbs: &impl SymbolsTableTrait) -> AceSymbolChunk {
        let mut symbols = Vec::new();

        for (k, v) in symbs
            .expression_symbol()
            .iter()
            .filter(|(s, _v)| self.keep_symbol(s))
        //    .sorted_by_key(|(s, _v)| s.to_string().to_ascii_lowercase())
        {
            // Get the symbol
            let k = k.value();

            // get a possible value when using u16
            let v = match v {
                Value::Address(a) => Some(a.address()),
                Value::Expr(ExprResult::Value(i)) => Some(*i as u16),
                Value::Expr(ExprResult::Bool(b)) => Some(*b as u16),
                Value::Expr(_e @ ExprResult::Float(_f)) => None,
                Value::Expr(ExprResult::String(_s)) => None,
                Value::Expr(_l @ ExprResult::List(_)) => None,
                Value::Expr(_m @ ExprResult::Matrix { .. }) => None,

                _ => None
            };

            // TODO properly create the value by specifying a correct mem map type and symb type
            // store if we have a representation
            if let Some(v) = v {
                let symb = AceSymbol::new(&k, v, cpclib_sna::AceMemMapType::Undefined, AceSymbolType::Absolute);
                symbols.push(symb);
            }
        }


        let mut chunk = AceSymbolChunk::empty();
        chunk.add_symbols(symbols.into_iter());
        chunk
    }

    /// Generate the symbol table in w
    pub fn generate<W: Write>(
        &self,
        w: &mut W,
        symbs: &impl SymbolsTableTrait,
        format: SymbolOutputFormat
    ) -> std::io::Result<()> {
        for (k, v) in symbs
            .expression_symbol()
            .iter()
            .filter(|(s, _v)| self.keep_symbol(s))
            .sorted_by_key(|(s, _v)| s.to_string().to_ascii_lowercase())
        {
            writeln!(w, "{}", format.format(k, v))?;
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
