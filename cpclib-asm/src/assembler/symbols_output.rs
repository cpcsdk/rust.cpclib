use std::io::Write;

use cpclib_tokens::symbols::{SymbolsTableTrait, Symbol};

/// Manage the generation of the symbols output.
/// Could be parametrize by some directives
#[derive(Clone)]
pub struct SymbolOutputGenerator {
	forbidden: Vec<Symbol>,
	allowed: Vec<Symbol>,

	all_forbidden: bool,
	all_allowed: bool,
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
	pub fn generate<W: Write>(&self, w: &mut W, symbs: & impl SymbolsTableTrait) -> std::io::Result<()> {

		for &k in symbs.integer_symbols()
					.iter()
					.filter(|s| self.keep_symbol(s))  {
			
			// TODO add some filtering stuffs based on the directives
			writeln!(
				w,
				"{} equ 0x{:04X}",
				k.value(),
				symbs.int_value(k.clone()).unwrap().unwrap()
			)?;
		}

		Ok(())
	}

	/// Returns true if the symbol needs to be printed
	pub fn keep_symbol(&self, sym: &Symbol) -> bool {
		assert!(self.all_allowed ^ self.all_forbidden);
		
		if sym.value() == "$" {return false}
		if sym.value() == "$$" {return false}
		else if self.all_allowed {
			!Self::is_included(&self.forbidden, sym)
		}
		else /*if self.all_forbidden*/ {
			Self::is_included(&self.allowed, sym)
		}

	}


	fn is_included(list: &[Symbol], sym: &Symbol) -> bool {
		list.iter()
		.find(|s2| {
			if **s2 == *sym {return true;}
			//if !s2.value().contains(".") {return false;}
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

	pub fn forbid_symbol<S:Into<Symbol>>(&mut self, s: S) {
		self.forbidden.push(s.into());
	}

	pub fn allow_symbol<S:Into<Symbol>>(&mut self, s: S)  {
		self.allowed.push(s.into());
	}
}