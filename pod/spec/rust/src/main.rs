//! CLI de la implementación B — mismo contrato de salida que `python -m podspec`.
//!
//!     podspec hash  <archivo>   -> 64 hex en stdout
//!     podspec canon <archivo>   -> bytes canónicos en stdout, sin salto final
//!
//! Rechazo: "<CODE>\t<path>" en stderr y salida 2.

use std::io::Write;

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 || (args[1] != "hash" && args[1] != "canon") {
        eprintln!("uso: podspec <hash|canon> <archivo>");
        return std::process::ExitCode::from(64);
    }
    let data = match std::fs::read(&args[2]) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("E_IO\t{e}");
            return std::process::ExitCode::from(66);
        }
    };
    let value = match podspec::load(&data) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{}\t{}", e.code, e.path);
            return std::process::ExitCode::from(2);
        }
    };
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let bytes = if args[1] == "hash" {
        podspec::spec_hash(&value).into_bytes()
    } else {
        podspec::canonical_bytes(&value)
    };
    if out.write_all(&bytes).is_err() {
        return std::process::ExitCode::from(74);
    }
    std::process::ExitCode::SUCCESS
}
