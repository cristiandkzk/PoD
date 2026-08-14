//! Andamio comun de los tests del gate 1.3.
//!
//! Decision de diseño que hace legibles a todos los tests: **la fee la paga siempre
//! `ctx.payer`**, que nunca es ni el pagador ni el worker de la orden. Asi las tres cuentas
//! que importan —pagador, worker y el PDA— conservan lamports **exactamente**, sin ruido de
//! fees, y la invariante I1 se puede escribir como una igualdad y no como una aproximacion.

#![allow(dead_code)]

use pod_escrow::instruction::{self as ix, CreateArgs};
use pod_escrow::state::Order;
use solana_program_test::{processor, ProgramTest, ProgramTestContext};
use solana_sdk::account::Account;
use solana_sdk::clock::Clock;
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::{Transaction, TransactionError};

pub const SOL: u64 = 1_000_000_000;
pub const REWARD: u64 = 200_000_000;
pub const BOND: u64 = 100_000_000;
pub const DEPOSIT: u64 = 50_000_000;
pub const ACCEPT_W: u32 = 3_600;
pub const DELIVER_W: u32 = 7_200;
pub const CHALLENGE_W: u32 = 1_800;
pub const SPEC_HASH: [u8; 32] = [0x5a; 32];
/// El `output_hash` que produce el runner de la subfase 1.2 para el pedido de referencia.
pub const HASH_BUENO: [u8; 32] = [0x7c; 32];
/// Cualquier otro. Un worker fraudulento entrega esto.
pub const HASH_FALSO: [u8; 32] = [0xde; 32];

pub struct Env {
    pub ctx: ProgramTestContext,
    pub pid: Pubkey,
    pub payer: Keypair,
    pub worker: Keypair,
    pub arbiter: Keypair,
    pub challenger: Keypair,
    pub outsider: Keypair,
}

pub async fn setup() -> Env {
    setup_with(5 * SOL, 5 * SOL).await
}

pub async fn setup_with(payer_lamports: u64, worker_lamports: u64) -> Env {
    let pid = pod_escrow::id();
    let mut pt = ProgramTest::new("pod_escrow", pid, processor!(pod_escrow::process_instruction));
    let payer = Keypair::new();
    let worker = Keypair::new();
    let arbiter = Keypair::new();
    let challenger = Keypair::new();
    let outsider = Keypair::new();
    for (k, l) in [
        (payer.pubkey(), payer_lamports),
        (worker.pubkey(), worker_lamports),
        (arbiter.pubkey(), 5 * SOL),
        (challenger.pubkey(), 5 * SOL),
        (outsider.pubkey(), 5 * SOL),
    ] {
        pt.add_account(k, Account { lamports: l, ..Account::default() });
    }
    let ctx = pt.start_with_context().await;
    Env { ctx, pid, payer, worker, arbiter, challenger, outsider }
}

/// Argumentos de `create_order` por default. Los tests que prueban un limite cambian el
/// campo que les interesa y dejan el resto igual.
pub fn args(e: &Env, nonce: u64) -> CreateArgs {
    CreateArgs {
        nonce,
        spec_hash: SPEC_HASH,
        arbiter: e.arbiter.pubkey(),
        reward_lamports: REWARD,
        bond_lamports: BOND,
        challenge_deposit_lamports: DEPOSIT,
        accept_window_secs: ACCEPT_W,
        deliver_window_secs: DELIVER_W,
        challenge_window_secs: CHALLENGE_W,
    }
}

pub fn create_ix(e: &Env, nonce: u64) -> Instruction {
    ix::create_order(&e.pid, &e.payer.pubkey(), &args(e, nonce))
}

impl Env {
    /// Manda una transaccion. La fee sale siempre de `ctx.payer`, nunca de las cuentas del test.
    pub async fn send(&mut self, ixs: &[Instruction], signers: &[&Keypair]) -> Result<(), TransactionError> {
        let fee_payer = self.ctx.payer.insecure_clone();
        let anterior = self.ctx.last_blockhash;
        let blockhash = self.ctx.get_new_latest_blockhash().await.unwrap_or(anterior);
        let mut all: Vec<&Keypair> = vec![&fee_payer];
        all.extend_from_slice(signers);
        let tx = Transaction::new_signed_with_payer(ixs, Some(&fee_payer.pubkey()), &all, blockhash);
        self.ctx
            .banks_client
            .process_transaction(tx)
            .await
            .map_err(|e| match e {
                solana_program_test::BanksClientError::TransactionError(te) => te,
                solana_program_test::BanksClientError::SimulationError { err, .. } => err,
                other => panic!("error del banco que no es de la transaccion: {other:?}"),
            })
    }

    pub async fn balance(&mut self, k: &Pubkey) -> u64 {
        self.ctx.banks_client.get_balance(*k).await.unwrap()
    }

    pub async fn clock(&mut self) -> i64 {
        let c: Clock = self.ctx.banks_client.get_sysvar().await.unwrap();
        c.unix_timestamp
    }

    /// Adelanta el reloj escribiendo el sysvar. No hay forma de esperar una hora real.
    pub async fn warp(&mut self, secs: i64) {
        let mut c: Clock = self.ctx.banks_client.get_sysvar().await.unwrap();
        c.unix_timestamp += secs;
        self.ctx.set_sysvar(&c);
    }

    pub async fn order(&mut self, k: &Pubkey) -> Option<Order> {
        let acc = self.ctx.banks_client.get_account(*k).await.unwrap()?;
        if acc.data.is_empty() {
            return None;
        }
        Some(Order::unpack(&acc.data).expect("la cuenta existe pero no parsea como Order"))
    }

    /// La cuenta desaparecio: sin lamports y sin datos.
    pub async fn is_gone(&mut self, k: &Pubkey) -> bool {
        match self.ctx.banks_client.get_account(*k).await.unwrap() {
            None => true,
            Some(a) => a.lamports == 0 && a.data.is_empty(),
        }
    }
}
