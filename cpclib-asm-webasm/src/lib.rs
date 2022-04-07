mod utils;

use cpclib_sna::SnapshotVersion;
use wasm_bindgen::{prelude::*};
use web_sys::console;

use cpclib_asm::{preamble::*, error::AssemblerError};


#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}


#[wasm_bindgen]

pub struct ParserConfig {
    dotted_directive: bool,
    case_sensitive: bool,
    file_name: String
}

#[wasm_bindgen]
pub fn create_parser_config(title: &str) -> ParserConfig {
    ParserConfig {
        dotted_directive: false,
        case_sensitive: true,
        file_name: title.to_owned()
    }
}

impl Into<ParserContext> for &ParserConfig {
    fn into(self) -> ParserContext {
        let mut ctx = ParserContext::default();
        ctx.set_dotted_directives(self.dotted_directive);
        ctx.set_current_filename(self.file_name.clone());
        ctx
    }
}

impl Into<AssemblingOptions> for &ParserConfig {
    fn into(self) -> AssemblingOptions {
        let mut options = AssemblingOptions::default();
        options.set_case_sensitive(self.case_sensitive);
        // TODO add specific symbols to recognize the wasm way of life
        options
    }
}

#[wasm_bindgen]
pub struct JsLocatedListing {
    listing: LocatedListing
}

impl From<LocatedListing> for JsLocatedListing {
    fn from(listing: LocatedListing) -> Self {
        Self {
            listing
        }
    }
}




#[wasm_bindgen]
pub struct JsAssemblerError {
    errors: String
}

impl From<AssemblerError> for JsAssemblerError {
    fn from(error: AssemblerError) -> Self {
        Self {
            errors: error.to_string()
        }
    }
}


#[wasm_bindgen]
impl JsAssemblerError {
    #[wasm_bindgen(getter)]
    pub fn msg(&self) -> String {
       self.errors.to_owned()
    }
}


#[wasm_bindgen]
pub struct JsSnapshot {
   // content: Vec<u8>
}

impl Into<JsSnapshot> for &cpclib_sna::Snapshot {
    fn into(self) -> JsSnapshot {
        let mut content = Vec::new();
        self.write(&mut content, SnapshotVersion::V3);

        JsSnapshot { /*content*/ }
    }
}


#[wasm_bindgen]
extern {
    fn alert(s: &str);
}



#[wasm_bindgen]
pub fn assemble_file(s: String) {

}

#[wasm_bindgen(catch)]
pub fn assemble_snapshot(code: &str, conf: &ParserConfig) 
    -> Result<JsSnapshot, JsAssemblerError> {
        console::log_1(&"assemble_snapshot".into());


    parse_source(code, conf)
        .map_err(|e| {
            console::log_1(&"Parse NOK".into());
            e
        })
        .and_then(|JsLocatedListing { listing }| {

            console::log_1(&"Parse OK".into());

            let mut options = AssemblingOptions::default();
            options.set_case_sensitive(conf.case_sensitive);
            options
                .symbols_mut()
                .assign_symbol_to_value(
                    Symbol::from("__CPC_PLAYGROUND__"), 
                    Value::from(true));

            console::log_1(&"Assemble options".into());


            visit_tokens_all_passes_with_options(&listing, &options, listing.ctx())
                .map_err(|e| {
                    console::log_1(&"ASM error".into());
                    JsAssemblerError::from(e)
                })
                .map(|env| {
                    console::log_1(&"SNA OK".into());
                    env.sna().into()
                })
            })
}

/// Parse the source and return the tokens.
/// Mainly usefull for acquiring syntax error when editing the file.
#[wasm_bindgen(catch)]
pub fn parse_source(code: &str, conf: &ParserConfig) -> Result<JsLocatedListing, JsAssemblerError> {
    let ctx:ParserContext = conf.into();

    let res = parse_z80_str_with_context(code, ctx);

    res
        .map(|l| l.into())
        .map_err(|e| e.into())
} 