//! `wasm32-unknown-unknown` bindings exposed via `wasm-bindgen`.
//!
//! Each function delegates to the conversion helpers on [`crate::ModSettings`]
//! so the format logic stays single-sourced; only the error type mapping to
//! `JsValue` happens here.
//!
//! JS mapping notes:
//! - `&[u8]` parameters arrive as JS `Uint8Array`; `Vec<u8>` returns are
//!   handed back as `Uint8Array` as well.
//! - Errors are mapped to `JsValue` strings using `{:#}` so the anyhow
//!   context chain is preserved in the message.

use wasm_bindgen::prelude::*;

/// Convert raw `mod-settings.dat` bytes to a pretty-printed JSON string.
#[wasm_bindgen]
pub fn dat_to_json(bytes: &[u8]) -> Result<String, JsValue> {
    crate::ModSettings::dat_to_json(bytes).map_err(|e| JsValue::from_str(&format!("{e:#}")))
}

/// Convert raw `mod-settings.dat` bytes to a YAML string.
#[wasm_bindgen]
pub fn dat_to_yaml(bytes: &[u8]) -> Result<String, JsValue> {
    crate::ModSettings::dat_to_yaml(bytes).map_err(|e| JsValue::from_str(&format!("{e:#}")))
}

/// Convert a JSON string (as produced by `dat_to_json`) back to raw `.dat` bytes.
#[wasm_bindgen]
pub fn json_to_dat(json: &str) -> Result<Vec<u8>, JsValue> {
    crate::ModSettings::json_to_dat(json).map_err(|e| JsValue::from_str(&format!("{e:#}")))
}

/// Convert a YAML string (as produced by `dat_to_yaml`) back to raw `.dat` bytes.
#[wasm_bindgen]
pub fn yaml_to_dat(yaml: &str) -> Result<Vec<u8>, JsValue> {
    crate::ModSettings::yaml_to_dat(yaml).map_err(|e| JsValue::from_str(&format!("{e:#}")))
}
