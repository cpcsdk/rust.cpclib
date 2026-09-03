// wasm-bindgen's macro expansion for `#[wasm_bindgen(catch)]` triggers a
// spurious unused_variables lint anchored on the attribute itself (the lint
// targets code the macro generates, which a fn-level #[allow] cannot reach);
// see https://github.com/rustwasm/wasm-bindgen/issues/3946
#![allow(unused_variables)]

use cpclib_basic::*;
use wasm_bindgen::prelude::*;
use web_sys::console;

use crate::sna::JsSnapshot;

#[wasm_bindgen]
#[derive(Debug)]
pub struct JsBasicError(BasicError);

impl From<BasicError> for JsBasicError {
    fn from(error: BasicError) -> JsBasicError {
        JsBasicError(error)
    }
}

#[wasm_bindgen]
impl JsBasicError {
    #[wasm_bindgen(getter)]
    pub fn msg(&self) -> String {
        self.0.to_string()
    }
}

#[wasm_bindgen]
pub struct JsBasicProgram(BasicProgram);

impl From<BasicProgram> for JsBasicProgram {
    fn from(prog: BasicProgram) -> JsBasicProgram {
        JsBasicProgram(prog)
    }
}

#[wasm_bindgen]
impl JsBasicProgram {
    #[wasm_bindgen(catch)]
    pub fn sna(&self) -> Result<JsSnapshot, String> {
        self.0.as_sna().map(|sna| sna.into())
    }
}

#[wasm_bindgen(catch)]
pub fn basic_parse_program(src: &str) -> Result<JsBasicProgram, JsBasicError> {
    BasicProgram::parse(src)
        .map_err(|e| {
            console::error_1(&e.to_string().into());
            e.into()
        })
        .map(|b| b.into())
}

#[wasm_bindgen(catch)]
pub fn basic_snapshot(src: &str) -> Result<JsSnapshot, String> {
    basic_parse_program(src).map_err(|e| e.0.to_string())?.sna()
}
