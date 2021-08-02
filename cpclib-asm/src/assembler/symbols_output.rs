use std::io::Write;

use cpclib_tokens::symbols::{SymbolsTableTrait, Symbol};

/// Manage the generation of the symbols output.
/// Could be parametrize by some directives
#[derive(Default)]
pub struct SymbolOutputGenerator {

}


impl SymbolOutputGenerator {

	/// Generate the symbol table in w
	pub fn generate<W: Write>(&self, w: &mut W, symbs: & impl SymbolsTableTrait) -> std::io::Result<()> {

		for &k in symbs.integer_symbols()
					.iter()
					.filter(|s| self.keep_symbol(s)) {
			
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
		sym.value() != "$"
	}
}