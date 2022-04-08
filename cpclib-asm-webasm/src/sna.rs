use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;

use cpclib_sna::*;


#[wasm_bindgen]
pub struct JsSnapshot(Snapshot);

impl Into<JsSnapshot> for Snapshot {
    fn into(self) -> JsSnapshot {
        JsSnapshot(self)
    }
}



#[wasm_bindgen]
impl JsSnapshot {

    #[wasm_bindgen(getter)]
    pub fn bytes(&self) -> Uint8Array {
        let mut content = Vec::new();
        self.0.write(&mut content, SnapshotVersion::V3);

        Uint8Array::from(content.as_slice())
            .to_owned()
    }
}
