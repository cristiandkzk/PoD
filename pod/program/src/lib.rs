//! PoD escrow — subfase 1.3. Normativa: `SPEC-PROGRAM.md`.
//!
//! El dinero se mueve antes de que exista cualquier prueba. No hay `deliver`, no hay
//! `challenge`, no hay `settle`: son 1.4. El `spec_hash` entra como 32 bytes opacos y el
//! programa no lo interpreta.

pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;

use solana_program::pubkey::Pubkey;

// Pubkey de `devnet/program-keypair.json`. Ver SPEC-PROGRAM.md §2: el id derivado del
// dominio no sobrevivio al despliegue, porque nadie tiene su clave privada.
solana_program::declare_id!("PoDJWDugBecU1jjXJtvPTQgUQKVv9rBcNpK1hCfpmS1");

pub use processor::process_instruction;

/// El entrypoint solo se compila para el target de Solana. Nativamente —tests y
/// `solana-program-test`— se llama a `process_instruction` directo, sin ABI de por medio.
#[cfg(all(target_os = "solana", not(feature = "no-entrypoint")))]
solana_program::entrypoint!(process_instruction);

/// Reexport para que los tests no dependan del path interno.
pub fn program_id() -> Pubkey {
    ID
}
