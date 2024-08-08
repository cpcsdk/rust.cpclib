use cpclib_asm::error::AssemblerError;
use cpclib_asm::preamble::*;
use cpclib_sna::Snapshot;
use wasm_bindgen::prelude::*;
use web_sys::console;

use crate::sna::JsSnapshot;

#[wasm_bindgen]
pub struct AsmParserConfig {
    dotted_directive: bool,
    case_sensitive: bool,
    file_name: String
}

#[wasm_bindgen]
pub fn asm_create_parser_config(file_name: &str) -> AsmParserConfig {
    AsmParserConfig {
        dotted_directive: false,
        case_sensitive: true,
        file_name: file_name.to_owned()
    }
}

impl From<&AsmParserConfig> for ParserContextBuilder {
    fn from(val: &AsmParserConfig) -> Self {
        let options: ParserOptions = val.into();
        options
            .context_builder()
            .set_current_filename(val.file_name.clone())
    }
}

impl From<&AsmParserConfig> for ParserOptions {
    fn from(val: &AsmParserConfig) -> Self {
        let mut ctx = ParserOptions::default();
        ctx.set_dotted_directives(val.dotted_directive);
        ctx
    }
}

impl From<&AsmParserConfig> for AssemblingOptions {
    fn from(val: &AsmParserConfig) -> Self {
        let mut options = AssemblingOptions::default();
        options.set_case_sensitive(val.case_sensitive);
        options.set_snapshot_model(Snapshot::new_6128_v2().expect("Unable to create a snapshot"));
        // TODO add specific symbols to recognize the wasm way of life
        options
    }
}

impl From<&AsmParserConfig> for EnvOptions {
    fn from(val: &AsmParserConfig) -> Self {
        let mut assemble_options: AssemblingOptions = val.into();
        assemble_options
            .symbols_mut()
            .assign_symbol_to_value(Symbol::from("__CPC_PLAYGROUND__"), Value::from(true))
            .unwrap();

        let parse_options: ParserOptions = val.into();

        EnvOptions::new(parse_options, assemble_options)
    }
}

#[wasm_bindgen]
pub struct JsAsmListing {
    listing: LocatedListing
}

impl From<LocatedListing> for JsAsmListing {
    fn from(listing: LocatedListing) -> Self {
        Self { listing }
    }
}

#[wasm_bindgen]
#[derive(Debug)]
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
        self.errors.clone()
    }
}

#[wasm_bindgen(catch)]
pub fn asm_assemble_snapshot(
    code: &str,
    conf: &AsmParserConfig
) -> Result<JsSnapshot, JsAssemblerError> {
    console::log_1(&"assemble_snapshot".into());

    asm_parse_source(code, conf)
        .inspect_err(|e| {
            console::log_1(&"Parse NOK".into());
        })
        .and_then(|JsAsmListing { listing }| {
            console::log_1(&"Parse OK".into());

            let options: EnvOptions = conf.into();
            console::log_1(&"Assemble options".into());

            visit_tokens_all_passes_with_options(&listing, options)
                .map_err(|(_t_, _env, e)| {
                    let e = AssemblerError::AlreadyRenderedError(e.to_string());
                    console::log_1(&"ASM error".into());
                    console::log_1(&e.to_string().into());
                    JsAssemblerError::from(e)
                })
                .map(|(_, env)| {
                    console::log_1(&"SNA OK".into());
                    let sna = env.sna();
                    let mut sna = sna.clone();
                    sna.unwrap_memory_chunks();
                    sna.into()
                })
        })
}

/// Parse the source and return the tokens.
/// Mainly usefull for acquiring syntax error when editing the file.
#[wasm_bindgen(catch)]
pub fn asm_parse_source(
    code: &str,
    conf: &AsmParserConfig
) -> Result<JsAsmListing, JsAssemblerError> {
    let res = parse_z80_with_context_builder(code, conf.into());

    res.map(|l| l.into()).map_err(|e| e.into())
}
