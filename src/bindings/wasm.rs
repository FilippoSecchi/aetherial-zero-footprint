use wasm_bindgen::prelude::*;
use zeroize::Zeroize;

#[wasm_bindgen]
pub struct WasmEphemeralBuffer {
    data: Vec<u8>,
}

#[wasm_bindgen]
impl WasmEphemeralBuffer {
    #[wasm_bindgen(constructor)]
    pub fn new(mut secret: Vec<u8>) -> Self {
        let copy = secret.clone();
        secret.zeroize();
        Self { data: copy }
    }

    #[wasm_bindgen]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    #[wasm_bindgen]
    pub fn read(&self) -> Vec<u8> {
        self.data.clone()
    }

    #[wasm_bindgen]
    pub fn destroy(mut self) {
        self.data.zeroize();
    }
}

impl Drop for WasmEphemeralBuffer {
    fn drop(&mut self) {
        self.data.zeroize();
    }
}
