use cpclib_basic::*;
use wasm_bindgen::prelude::*;
use web_sys::console;

use crate::sna::JsSnapshot;


#[wasm_bindgen]
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
    #[wasm_bindgen]
    pub fn sna(&self) -> JsSnapshot {
        self.0.as_sna()
            .into()
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

