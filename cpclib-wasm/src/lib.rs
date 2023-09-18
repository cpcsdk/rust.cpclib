mod utils;

pub mod asm;
pub mod basic;
pub mod sna;

use wasm_bindgen::prelude::*;


#[wasm_bindgen]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}
