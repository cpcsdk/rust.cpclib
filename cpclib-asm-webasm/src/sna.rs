use std::ops::Deref;

use js_sys::{Uint8Array, Array};
use web_sys::{Blob, BlobPropertyBag, Window, Url, HtmlAnchorElement};

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use cpclib_sna::*;


#[wasm_bindgen]
pub struct JsSnapshot(Snapshot);

impl Into<JsSnapshot> for Snapshot {
    fn into(self) -> JsSnapshot {
        JsSnapshot(self)
    }
}

impl Deref for JsSnapshot {
    type Target = Snapshot;
    
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
#[wasm_bindgen]
impl JsSnapshot {
    #[wasm_bindgen]
    pub fn get_byte(&self, address: u32) -> u8 {
        self.0.get_byte(address)
    }
}

#[wasm_bindgen]
impl JsSnapshot {

    /// Returns the snapshot as a V2 format (as soon as tiny8bit emulator does not accept v3 format)
    #[wasm_bindgen(getter)]
    pub fn bytes(&self) -> Uint8Array {
        let mut content = Vec::new();
        self.0.write(&mut content, SnapshotVersion::V2).unwrap();

        Uint8Array::from(content.as_slice())
            .to_owned()
    }


    #[wasm_bindgen]
    pub fn download(&self, fname: &str) {
        let window = web_sys::window().unwrap();

        let bytes = self.bytes();
        let container = Array::new();
        container.set(0, bytes.into());
        let mut property = BlobPropertyBag::new();
        property.type_("application/octet-stream");
        
        //let blob = Array::new_with_length(1);
        //blob.set(0, bytes.into());
        let blob = Blob::new_with_blob_sequence_and_options(&container, &property).unwrap();

        let url = Url::create_object_url_with_blob(&blob).unwrap();

        let link: HtmlAnchorElement = window.document().unwrap()
            .create_element("a")
            .unwrap()
            .dyn_into()
            .unwrap() ;
        link.set_download(fname);
        link.set_href(&url);
        link.click();
        Url::revoke_object_url(&url).unwrap();

    }
}
