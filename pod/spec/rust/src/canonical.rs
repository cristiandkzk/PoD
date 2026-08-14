//! Serialización canónica y spec_hash — SPEC.md §4 y §5.

use crate::json::Value;
use crate::sha256::{hex_lower, sha256};

pub const DOMAIN: &[u8] = b"PoD/WorkOrder/1\x00";

pub fn canonical_text(value: &Value) -> String {
    let mut out = String::new();
    write(value, &mut out);
    out
}

fn write(value: &Value, out: &mut String) {
    match value {
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Int(n) => out.push_str(&n.to_string()),
        // Sin escapes: el conjunto de caracteres de SPEC §3.2 no contiene comillas ni
        // barra invertida, así que ningún escape es alcanzable.
        Value::Str(s) => {
            out.push('"');
            out.push_str(s);
            out.push('"');
        }
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write(item, out);
            }
            out.push(']');
        }
        // BTreeMap ya itera en orden ascendente por bytes de la clave (SPEC §4.1).
        Value::Object(map) => {
            out.push('{');
            for (i, (key, val)) in map.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push('"');
                out.push_str(key);
                out.push_str("\":");
                write(val, out);
            }
            out.push('}');
        }
    }
}

pub fn canonical_bytes(value: &Value) -> Vec<u8> {
    canonical_text(value).into_bytes()
}

pub fn spec_hash(value: &Value) -> String {
    let mut buf = Vec::from(DOMAIN);
    buf.extend_from_slice(&canonical_bytes(value));
    hex_lower(&sha256(&buf))
}
