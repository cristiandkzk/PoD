//! Implementación B (Rust) del formato canónico de WorkOrder — ver ../SPEC.md.
//!
//! Escrita contra SPEC.md, no contra la implementación de Python. Es la segunda
//! implementación que exige el gate 3 de la subfase 1.1, y la que después vive al lado del
//! programa de Anchor.

pub mod canonical;
pub mod error;
pub mod json;
pub mod schema;
pub mod sha256;

pub use canonical::{canonical_bytes, canonical_text, spec_hash, DOMAIN};
pub use error::SpecError;
pub use json::{parse, Value};
pub use schema::validate;

/// bytes -> WorkOrder validado.
pub fn load(data: &[u8]) -> Result<Value, SpecError> {
    let value = parse(data)?;
    validate(&value)?;
    Ok(value)
}
