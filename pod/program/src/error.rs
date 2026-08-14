//! Codigos de error — SPEC-PROGRAM.md §6.
//!
//! Se devuelven como `ProgramError::Custom(n)`. Los numeros son parte de la normativa:
//! un cliente los interpreta, asi que cambiarlos es cambiar la interfaz publica.

use solana_program::program_error::ProgramError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum PodError {
    BadInstruction = 1,
    BadAccounts = 2,
    BadPda = 3,
    AlreadyExists = 4,
    BadState = 5,
    NotExpired = 6,
    Expired = 7,
    NotSigner = 8,
    WrongPayer = 9,
    WrongWorker = 10,
    ZeroReward = 11,
    ZeroBond = 12,
    BadWindow = 13,
    BadAccountData = 14,
    Overflow = 15,
    WrongArbiter = 16,
    BadVerdict = 17,
    ZeroDeposit = 18,
    WrongChallenger = 19,
    SameHash = 20,
    ZeroHash = 21,
}

impl From<PodError> for ProgramError {
    fn from(e: PodError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
