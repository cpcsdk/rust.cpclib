/// ! Z80 emulator
/// ! This should be deprecated in favor of a real emulator (WIP in another repo)
pub mod emul;
mod preamble;
mod z80;

use cpclib_asm::preamble::*;

pub use self::z80::{HasValue, Z80};

/// Result on the listing execution
#[derive(Default, Debug, Copy, Clone)]
pub struct ListingExecution {
    /// Number of nops needed to execute the listing
    nops: usize
}

impl ListingExecution {
    /// Emulated duration of the executed listing
    pub fn duration(&self) -> usize {
        self.nops
    }
}

/// Execute the Listing as soon as there is no jumps except for djnz/jp $ (i.e. instructions are taken from the stream)
pub fn execute_dummy_listing(lst: &Listing) -> Result<ListingExecution, String> {
    let mut res = ListingExecution::default();
    let mut z80 = z80::Z80::default();

    // We assume the PC starts at 0 (as well as any other register...)
    // TODO setup properly the context
    for token in lst.listing().iter() {
        loop {
            let old_pc = z80.pc().value();
            let nops = z80.execute(token);
            let new_pc = z80.pc().value();

            res.nops += nops;

            if old_pc != new_pc {
                // execution flow is broken / it is not compatible with our case
                if new_pc != old_pc + token.number_of_bytes().unwrap() as u16 {
                    return Err(format!("{token} does not allow a sequential execution"));
                }
                // break the loop if we need to go to the next instruction
                else {
                    break;
                }
            }
        }
    }

    Ok(res)
}
