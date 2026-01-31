use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn decode_payload(input: &str) -> Result<String, JsValue> {
    let mut bytes = input.as_bytes().to_vec();
    match simd_json::to_string_pretty(&simd_json::from_slice::<simd_json::OwnedValue>(&mut bytes).map_err(|e| e.to_string())?) {
        Ok(s) => Ok(s),
        Err(e) => Err(JsValue::from_str(&e.to_string())),
    }
}

#[wasm_bindgen]
pub fn validate_json(input: &str) -> bool {
    let mut bytes = input.as_bytes().to_vec();
    simd_json::from_slice::<simd_json::OwnedValue>(&mut bytes).is_ok()
}
