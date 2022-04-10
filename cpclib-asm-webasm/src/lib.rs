mod utils;

pub mod asm;
pub mod sna;
pub mod basic;




use wasm_bindgen::prelude::*;



#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}








