//! Codificacion de instrucciones — SPEC-PROGRAM.md §4.
//!
//! Byte 0 = discriminante, el resto son los campos en orden, little-endian. La longitud es
//! **exacta**: sobrante o faltante se rechaza. Misma regla que `../../spec/SPEC.md` §1 — la
//! ambiguedad se rechaza, no se normaliza.

use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;
use solana_program::sysvar;
use solana_system_interface::program as system_program;

use crate::error::PodError;

pub const DISC_CREATE: u8 = 0;
pub const DISC_ACCEPT: u8 = 1;
pub const DISC_CANCEL: u8 = 2;
pub const DISC_DELIVER: u8 = 3;
pub const DISC_CHALLENGE: u8 = 4;
pub const DISC_RESOLVE: u8 = 5;
pub const DISC_SETTLE: u8 = 6;

pub const LEN_CREATE: usize = 109;
pub const LEN_ACCEPT: usize = 1;
pub const LEN_CANCEL: usize = 1;
pub const LEN_DELIVER: usize = 33;
pub const LEN_CHALLENGE: usize = 33;
pub const LEN_RESOLVE: usize = 2;
pub const LEN_SETTLE: usize = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CreateArgs {
    pub nonce: u64,
    pub spec_hash: [u8; 32],
    pub arbiter: Pubkey,
    pub reward_lamports: u64,
    pub bond_lamports: u64,
    pub challenge_deposit_lamports: u64,
    pub accept_window_secs: u32,
    pub deliver_window_secs: u32,
    pub challenge_window_secs: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PodInstruction {
    CreateOrder(CreateArgs),
    AcceptOrder,
    CancelExpired,
    Deliver { output_hash: [u8; 32] },
    Challenge { claimed_output_hash: [u8; 32] },
    Resolve { verdict: u8 },
    Settle,
}

fn u64_at(d: &[u8], off: usize) -> u64 {
    u64::from_le_bytes(d[off..off + 8].try_into().unwrap())
}

fn u32_at(d: &[u8], off: usize) -> u32 {
    u32::from_le_bytes(d[off..off + 4].try_into().unwrap())
}

impl PodInstruction {
    pub fn decode(data: &[u8]) -> Result<PodInstruction, PodError> {
        match data.first() {
            Some(&DISC_CREATE) if data.len() == LEN_CREATE => {
                Ok(PodInstruction::CreateOrder(CreateArgs {
                    nonce: u64_at(data, 1),
                    spec_hash: data[9..41].try_into().unwrap(),
                    arbiter: Pubkey::new_from_array(data[41..73].try_into().unwrap()),
                    reward_lamports: u64_at(data, 73),
                    bond_lamports: u64_at(data, 81),
                    challenge_deposit_lamports: u64_at(data, 89),
                    accept_window_secs: u32_at(data, 97),
                    deliver_window_secs: u32_at(data, 101),
                    challenge_window_secs: u32_at(data, 105),
                }))
            }
            Some(&DISC_ACCEPT) if data.len() == LEN_ACCEPT => Ok(PodInstruction::AcceptOrder),
            Some(&DISC_CANCEL) if data.len() == LEN_CANCEL => Ok(PodInstruction::CancelExpired),
            Some(&DISC_DELIVER) if data.len() == LEN_DELIVER => Ok(PodInstruction::Deliver {
                output_hash: data[1..33].try_into().unwrap(),
            }),
            Some(&DISC_CHALLENGE) if data.len() == LEN_CHALLENGE => Ok(PodInstruction::Challenge {
                claimed_output_hash: data[1..33].try_into().unwrap(),
            }),
            Some(&DISC_RESOLVE) if data.len() == LEN_RESOLVE => {
                Ok(PodInstruction::Resolve { verdict: data[1] })
            }
            Some(&DISC_SETTLE) if data.len() == LEN_SETTLE => Ok(PodInstruction::Settle),
            _ => Err(PodError::BadInstruction),
        }
    }
}

/// Direccion del PDA de una orden. Cualquiera puede calcularla sin leer la cadena (§3.1).
pub fn order_address(program_id: &Pubkey, payer: &Pubkey, spec_hash: &[u8; 32], nonce: u64) -> (Pubkey, u8) {
    let nonce_le = nonce.to_le_bytes();
    Pubkey::find_program_address(
        &[crate::state::SEED_PREFIX, payer.as_ref(), spec_hash, &nonce_le],
        program_id,
    )
}

pub fn create_order(program_id: &Pubkey, payer: &Pubkey, a: &CreateArgs) -> Instruction {
    let (order, _) = order_address(program_id, payer, &a.spec_hash, a.nonce);
    let mut data = Vec::with_capacity(LEN_CREATE);
    data.push(DISC_CREATE);
    data.extend_from_slice(&a.nonce.to_le_bytes());
    data.extend_from_slice(&a.spec_hash);
    data.extend_from_slice(a.arbiter.as_ref());
    data.extend_from_slice(&a.reward_lamports.to_le_bytes());
    data.extend_from_slice(&a.bond_lamports.to_le_bytes());
    data.extend_from_slice(&a.challenge_deposit_lamports.to_le_bytes());
    data.extend_from_slice(&a.accept_window_secs.to_le_bytes());
    data.extend_from_slice(&a.deliver_window_secs.to_le_bytes());
    data.extend_from_slice(&a.challenge_window_secs.to_le_bytes());
    debug_assert_eq!(data.len(), LEN_CREATE);
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new(order, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(sysvar::rent::ID, false),
            AccountMeta::new_readonly(sysvar::clock::ID, false),
        ],
        data,
    }
}

pub fn accept_order(program_id: &Pubkey, worker: &Pubkey, order: &Pubkey) -> Instruction {
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*worker, true),
            AccountMeta::new(*order, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(sysvar::clock::ID, false),
        ],
        data: vec![DISC_ACCEPT],
    }
}

/// `worker` va desde ACEPTADA; `challenger` solo desde DISPUTADA. Sin firmante: es un
/// crank permissionless.
pub fn cancel_expired(
    program_id: &Pubkey,
    order: &Pubkey,
    payer: &Pubkey,
    worker: Option<&Pubkey>,
    challenger: Option<&Pubkey>,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new(*order, false),
        AccountMeta::new(*payer, false),
        AccountMeta::new_readonly(sysvar::clock::ID, false),
    ];
    if let Some(w) = worker {
        accounts.push(AccountMeta::new(*w, false));
    }
    if let Some(c) = challenger {
        accounts.push(AccountMeta::new(*c, false));
    }
    Instruction { program_id: *program_id, accounts, data: vec![DISC_CANCEL] }
}

pub fn deliver(program_id: &Pubkey, worker: &Pubkey, order: &Pubkey, output_hash: &[u8; 32]) -> Instruction {
    let mut data = Vec::with_capacity(LEN_DELIVER);
    data.push(DISC_DELIVER);
    data.extend_from_slice(output_hash);
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new_readonly(*worker, true),
            AccountMeta::new(*order, false),
            AccountMeta::new_readonly(sysvar::clock::ID, false),
        ],
        data,
    }
}

pub fn challenge(
    program_id: &Pubkey,
    challenger: &Pubkey,
    order: &Pubkey,
    claimed_output_hash: &[u8; 32],
) -> Instruction {
    let mut data = Vec::with_capacity(LEN_CHALLENGE);
    data.push(DISC_CHALLENGE);
    data.extend_from_slice(claimed_output_hash);
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*challenger, true),
            AccountMeta::new(*order, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(sysvar::clock::ID, false),
        ],
        data,
    }
}

/// Lo firma el arbitro declarado en la orden. Los cuatro destinos van siempre, en este
/// orden, con o sin el veredicto que los favorezca: quien liquida no elige a quien paga.
pub fn resolve(
    program_id: &Pubkey,
    arbiter: &Pubkey,
    order: &Pubkey,
    payer: &Pubkey,
    worker: &Pubkey,
    challenger: &Pubkey,
    verdict: u8,
) -> Instruction {
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new_readonly(*arbiter, true),
            AccountMeta::new(*order, false),
            AccountMeta::new(*payer, false),
            AccountMeta::new(*worker, false),
            AccountMeta::new(*challenger, false),
            AccountMeta::new_readonly(sysvar::clock::ID, false),
        ],
        data: vec![DISC_RESOLVE, verdict],
    }
}

/// Sin firmante, como `cancel_expired`: la liquidacion no depende de que nadie este vivo.
pub fn settle(program_id: &Pubkey, order: &Pubkey, payer: &Pubkey, worker: &Pubkey) -> Instruction {
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*order, false),
            AccountMeta::new(*payer, false),
            AccountMeta::new(*worker, false),
            AccountMeta::new_readonly(sysvar::clock::ID, false),
        ],
        data: vec![DISC_SETTLE],
    }
}
