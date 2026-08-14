//! La cuenta `Order` — SPEC-PROGRAM.md §3.
//!
//! El layout es normativo y esta escrito a mano, sin borsh y sin `#[derive]`: la fuente de
//! verdad de los offsets es la tabla de §3.2, no el orden de los campos de este struct.
//!
//! **Version 2** (subfase 1.4). La v1 de 1.3 no llego a desplegarse en ninguna parte; el
//! byte de version sube igual, porque el layout cambio y un lector que confunda uno con otro
//! leeria basura. Ver §9.

use solana_program::pubkey::Pubkey;

use crate::error::PodError;

pub const ORDER_LEN: usize = 304;
pub const VERSION: u8 = 2;

pub const STATE_CREADA: u8 = 1;
pub const STATE_ACEPTADA: u8 = 2;
pub const STATE_ENTREGADA: u8 = 3;
pub const STATE_DISPUTADA: u8 = 4;

/// Modo de prueba. `1` es el unico implementado: verificacion optimista (Nivel 1).
/// `2` queda reservado para ZK — interfaz declarada en §8, no implementada.
pub const PROOF_OPTIMISTIC: u8 = 1;
pub const PROOF_ZK: u8 = 2;

/// Veredictos de `resolve`.
pub const VERDICT_FRAUDE: u8 = 1;
pub const VERDICT_INFUNDADO: u8 = 2;

/// Semilla literal del PDA. Ver `Order::seeds`.
pub const SEED_PREFIX: &[u8] = b"order";

/// Ventanas admisibles, en segundos: de un minuto a treinta dias.
pub const WINDOW_MIN: u32 = 60;
pub const WINDOW_MAX: u32 = 2_592_000;

pub const ZERO_HASH: [u8; 32] = [0u8; 32];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Order {
    pub version: u8,
    pub state: u8,
    pub bump: u8,
    pub proof_mode: u8,
    pub deliver_window_secs: u32,
    pub challenge_window_secs: u32,
    pub nonce: u64,
    pub spec_hash: [u8; 32],
    pub payer: Pubkey,
    pub worker: Pubkey,
    pub arbiter: Pubkey,
    pub challenger: Pubkey,
    pub output_hash: [u8; 32],
    pub claimed_output_hash: [u8; 32],
    pub reward_lamports: u64,
    pub bond_lamports: u64,
    pub rent_lamports: u64,
    pub challenge_deposit_lamports: u64,
    pub accept_deadline: i64,
    pub deliver_deadline: i64,
    pub challenge_deadline: i64,
}

fn u32_at(src: &[u8], off: usize) -> u32 {
    u32::from_le_bytes(src[off..off + 4].try_into().unwrap())
}

fn u64_at(src: &[u8], off: usize) -> u64 {
    u64::from_le_bytes(src[off..off + 8].try_into().unwrap())
}

fn i64_at(src: &[u8], off: usize) -> i64 {
    i64::from_le_bytes(src[off..off + 8].try_into().unwrap())
}

fn key_at(src: &[u8], off: usize) -> Pubkey {
    Pubkey::new_from_array(src[off..off + 32].try_into().unwrap())
}

fn h32(src: &[u8], off: usize) -> [u8; 32] {
    src[off..off + 32].try_into().unwrap()
}

impl Order {
    /// Semillas del PDA, sin el bump. §3.1.
    pub fn seeds<'a>(payer: &'a [u8; 32], spec_hash: &'a [u8; 32], nonce: &'a [u8; 8]) -> [&'a [u8]; 4] {
        [SEED_PREFIX, payer, spec_hash, nonce]
    }

    pub fn unpack(src: &[u8]) -> Result<Order, PodError> {
        if src.len() != ORDER_LEN {
            return Err(PodError::BadAccountData);
        }
        if src[0] != VERSION || src[3] != PROOF_OPTIMISTIC {
            return Err(PodError::BadAccountData);
        }
        if !(STATE_CREADA..=STATE_DISPUTADA).contains(&src[1]) {
            return Err(PodError::BadAccountData);
        }
        if src[12..16] != [0u8; 4] {
            return Err(PodError::BadAccountData);
        }
        Ok(Order {
            version: src[0],
            state: src[1],
            bump: src[2],
            proof_mode: src[3],
            deliver_window_secs: u32_at(src, 4),
            challenge_window_secs: u32_at(src, 8),
            nonce: u64_at(src, 16),
            spec_hash: h32(src, 24),
            payer: key_at(src, 56),
            worker: key_at(src, 88),
            arbiter: key_at(src, 120),
            challenger: key_at(src, 152),
            output_hash: h32(src, 184),
            claimed_output_hash: h32(src, 216),
            reward_lamports: u64_at(src, 248),
            bond_lamports: u64_at(src, 256),
            rent_lamports: u64_at(src, 264),
            challenge_deposit_lamports: u64_at(src, 272),
            accept_deadline: i64_at(src, 280),
            deliver_deadline: i64_at(src, 288),
            challenge_deadline: i64_at(src, 296),
        })
    }

    pub fn pack(&self, dst: &mut [u8]) -> Result<(), PodError> {
        if dst.len() != ORDER_LEN {
            return Err(PodError::BadAccountData);
        }
        dst[0] = self.version;
        dst[1] = self.state;
        dst[2] = self.bump;
        dst[3] = self.proof_mode;
        dst[4..8].copy_from_slice(&self.deliver_window_secs.to_le_bytes());
        dst[8..12].copy_from_slice(&self.challenge_window_secs.to_le_bytes());
        dst[12..16].copy_from_slice(&[0u8; 4]);
        dst[16..24].copy_from_slice(&self.nonce.to_le_bytes());
        dst[24..56].copy_from_slice(&self.spec_hash);
        dst[56..88].copy_from_slice(self.payer.as_ref());
        dst[88..120].copy_from_slice(self.worker.as_ref());
        dst[120..152].copy_from_slice(self.arbiter.as_ref());
        dst[152..184].copy_from_slice(self.challenger.as_ref());
        dst[184..216].copy_from_slice(&self.output_hash);
        dst[216..248].copy_from_slice(&self.claimed_output_hash);
        dst[248..256].copy_from_slice(&self.reward_lamports.to_le_bytes());
        dst[256..264].copy_from_slice(&self.bond_lamports.to_le_bytes());
        dst[264..272].copy_from_slice(&self.rent_lamports.to_le_bytes());
        dst[272..280].copy_from_slice(&self.challenge_deposit_lamports.to_le_bytes());
        dst[280..288].copy_from_slice(&self.accept_deadline.to_le_bytes());
        dst[288..296].copy_from_slice(&self.deliver_deadline.to_le_bytes());
        dst[296..304].copy_from_slice(&self.challenge_deadline.to_le_bytes());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn muestra() -> Order {
        Order {
            version: VERSION,
            state: STATE_DISPUTADA,
            bump: 254,
            proof_mode: PROOF_OPTIMISTIC,
            deliver_window_secs: 7_200,
            challenge_window_secs: 3_600,
            nonce: 0x0102_0304_0506_0708,
            spec_hash: [0xab; 32],
            payer: Pubkey::new_from_array([0x11; 32]),
            worker: Pubkey::new_from_array([0x22; 32]),
            arbiter: Pubkey::new_from_array([0x33; 32]),
            challenger: Pubkey::new_from_array([0x44; 32]),
            output_hash: [0x55; 32],
            claimed_output_hash: [0x66; 32],
            reward_lamports: 200_000_000,
            bond_lamports: 100_000_000,
            rent_lamports: 3_006_720,
            challenge_deposit_lamports: 50_000_000,
            accept_deadline: 1_700_000_000,
            deliver_deadline: 1_700_007_200,
            challenge_deadline: 1_700_010_800,
        }
    }

    /// El layout es normativo (SPEC-PROGRAM.md §3.2). Este test lo fija por offset, no por
    /// orden de campos del struct: reordenar el struct no tiene que poder mover un byte.
    #[test]
    fn los_offsets_son_los_de_la_tabla() {
        let mut buf = [0u8; ORDER_LEN];
        muestra().pack(&mut buf).unwrap();

        assert_eq!(buf[0], 2, "version");
        assert_eq!(buf[1], STATE_DISPUTADA, "state");
        assert_eq!(buf[2], 254, "bump");
        assert_eq!(buf[3], PROOF_OPTIMISTIC, "proof_mode");
        assert_eq!(&buf[4..8], &7_200u32.to_le_bytes(), "deliver_window_secs");
        assert_eq!(&buf[8..12], &3_600u32.to_le_bytes(), "challenge_window_secs");
        assert_eq!(&buf[12..16], &[0u8; 4], "reservado");
        assert_eq!(&buf[16..24], &0x0102_0304_0506_0708u64.to_le_bytes(), "nonce");
        assert_eq!(&buf[24..56], &[0xab; 32], "spec_hash");
        assert_eq!(&buf[56..88], &[0x11; 32], "payer");
        assert_eq!(&buf[88..120], &[0x22; 32], "worker");
        assert_eq!(&buf[120..152], &[0x33; 32], "arbiter");
        assert_eq!(&buf[152..184], &[0x44; 32], "challenger");
        assert_eq!(&buf[184..216], &[0x55; 32], "output_hash");
        assert_eq!(&buf[216..248], &[0x66; 32], "claimed_output_hash");
        assert_eq!(&buf[248..256], &200_000_000u64.to_le_bytes(), "reward");
        assert_eq!(&buf[256..264], &100_000_000u64.to_le_bytes(), "bond");
        assert_eq!(&buf[264..272], &3_006_720u64.to_le_bytes(), "rent");
        assert_eq!(&buf[272..280], &50_000_000u64.to_le_bytes(), "deposito");
        assert_eq!(&buf[280..288], &1_700_000_000i64.to_le_bytes(), "accept_deadline");
        assert_eq!(&buf[288..296], &1_700_007_200i64.to_le_bytes(), "deliver_deadline");
        assert_eq!(&buf[296..304], &1_700_010_800i64.to_le_bytes(), "challenge_deadline");
    }

    #[test]
    fn ida_y_vuelta() {
        let mut buf = [0u8; ORDER_LEN];
        let o = muestra();
        o.pack(&mut buf).unwrap();
        assert_eq!(Order::unpack(&buf).unwrap(), o);
    }

    #[test]
    fn se_rechaza_lo_que_no_es_una_orden() {
        let mut buf = [0u8; ORDER_LEN];
        muestra().pack(&mut buf).unwrap();

        assert_eq!(Order::unpack(&buf[..ORDER_LEN - 1]), Err(PodError::BadAccountData), "largo");
        assert_eq!(Order::unpack(&[0u8; ORDER_LEN]), Err(PodError::BadAccountData), "todo ceros");

        for (i, byte, que) in [(0usize, 1u8, "la version vieja"), (0, 3, "una version futura"),
                               (1, 5, "estado inexistente"), (1, 0, "estado cero"),
                               (3, 2, "proof_mode zk, que no esta implementado"),
                               (12, 1, "reservado sucio")] {
            let mut malo = buf;
            malo[i] = byte;
            assert_eq!(Order::unpack(&malo), Err(PodError::BadAccountData), "{que}");
        }
    }
}
