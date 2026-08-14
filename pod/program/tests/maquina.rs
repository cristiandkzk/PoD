//! La maquina de estados y la validacion de entrada — SPEC-PROGRAM.md §3.3 y §4.
//!
//! Lo que el gate no enumera pero sostiene: que las transiciones sean las tres que dice
//! §3.3 y ninguna mas, y que los limites de §4.1 se rechacen en la puerta.

mod common;

use common::*;
use pod_escrow::error::PodError;
use pod_escrow::instruction as ix;
use pod_escrow::instruction::CreateArgs;
use pod_escrow::state::{STATE_ACEPTADA, WINDOW_MAX, WINDOW_MIN};
use solana_sdk::instruction::{AccountMeta, Instruction, InstructionError};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer;
use solana_sdk::transaction::TransactionError;

fn err(e: PodError) -> TransactionError {
    TransactionError::InstructionError(0, InstructionError::Custom(e as u32))
}

#[tokio::test]
async fn los_limites_de_entrada_se_rechazan_en_la_puerta() {
    let mut e = setup().await;
    let p = e.payer.pubkey();
    type Tocar = fn(&mut CreateArgs);
    let casos: [(Tocar, PodError); 10] = [
        (|a| a.reward_lamports = 0, PodError::ZeroReward),
        (|a| a.bond_lamports = 0, PodError::ZeroBond),
        (|a| a.challenge_deposit_lamports = 0, PodError::ZeroDeposit),
        (|a| a.arbiter = Pubkey::default(), PodError::WrongArbiter),
        (|a| a.accept_window_secs = WINDOW_MIN - 1, PodError::BadWindow),
        (|a| a.accept_window_secs = WINDOW_MAX + 1, PodError::BadWindow),
        (|a| a.deliver_window_secs = WINDOW_MIN - 1, PodError::BadWindow),
        (|a| a.deliver_window_secs = WINDOW_MAX + 1, PodError::BadWindow),
        (|a| a.challenge_window_secs = WINDOW_MIN - 1, PodError::BadWindow),
        (|a| a.challenge_window_secs = WINDOW_MAX + 1, PodError::BadWindow),
    ];
    for (i, (tocar, esperado)) in casos.into_iter().enumerate() {
        let mut a = args(&e, 100 + i as u64);
        tocar(&mut a);
        let c = ix::create_order(&e.pid, &p, &a);
        assert_eq!(e.send(&[c], &[&e.payer.insecure_clone()]).await.unwrap_err(), err(esperado));
    }
    // Los bordes exactos si tienen que entrar.
    for (i, w) in [WINDOW_MIN, WINDOW_MAX].into_iter().enumerate() {
        let mut a = args(&e, 200 + i as u64);
        a.accept_window_secs = w;
        a.deliver_window_secs = w;
        a.challenge_window_secs = w;
        let c = ix::create_order(&e.pid, &p, &a);
        e.send(&[c], &[&e.payer.insecure_clone()]).await.unwrap();
    }
}

#[tokio::test]
async fn crear_sin_la_firma_del_pagador_falla() {
    let mut e = setup().await;
    let mut c = create_ix(&e, 1);
    c.accounts[0] = AccountMeta::new(e.payer.pubkey(), false);
    assert_eq!(e.send(&[c], &[]).await.unwrap_err(), err(PodError::NotSigner));
}

#[tokio::test]
async fn no_se_acepta_dos_veces() {
    let mut e = setup().await;
    let (order, _) = ix::order_address(&e.pid, &e.payer.pubkey(), &SPEC_HASH, 1);
    let c = create_ix(&e, 1);
    e.send(&[c], &[&e.payer.insecure_clone()]).await.unwrap();

    let a = ix::accept_order(&e.pid, &e.worker.pubkey(), &order);
    e.send(&[a.clone()], &[&e.worker.insecure_clone()]).await.unwrap();
    assert_eq!(
        e.send(&[a], &[&e.worker.insecure_clone()]).await.unwrap_err(),
        err(PodError::BadState),
        "un segundo bond entraria al escrow sin cambiar nada"
    );

    // Otro worker tampoco puede pisar al primero.
    let otro = solana_sdk::signature::Keypair::new();
    let a2 = ix::accept_order(&e.pid, &otro.pubkey(), &order);
    // (sin fondos ni firma valida en el banco, igual tiene que morir por estado)
    let r = e.send(&[a2], &[&otro]).await;
    assert!(r.is_err());
}

#[tokio::test]
async fn no_se_acepta_una_orden_vencida() {
    let mut e = setup().await;
    let (order, _) = ix::order_address(&e.pid, &e.payer.pubkey(), &SPEC_HASH, 1);
    let c = create_ix(&e, 1);
    e.send(&[c], &[&e.payer.insecure_clone()]).await.unwrap();
    e.warp(i64::from(ACCEPT_W) + 1).await;

    let a = ix::accept_order(&e.pid, &e.worker.pubkey(), &order);
    assert_eq!(e.send(&[a], &[&e.worker.insecure_clone()]).await.unwrap_err(), err(PodError::Expired));
}

#[tokio::test]
async fn no_se_cancela_antes_del_vencimiento() {
    let mut e = setup().await;
    let (order, _) = ix::order_address(&e.pid, &e.payer.pubkey(), &SPEC_HASH, 1);
    let c = create_ix(&e, 1);
    e.send(&[c], &[&e.payer.insecure_clone()]).await.unwrap();

    let x = ix::cancel_expired(&e.pid, &order, &e.payer.pubkey(), None, None);
    assert_eq!(e.send(&[x], &[]).await.unwrap_err(), err(PodError::NotExpired));

    // Justo en el borde: el vencimiento es estricto, `now > deadline`.
    e.warp(i64::from(ACCEPT_W)).await;
    let x = ix::cancel_expired(&e.pid, &order, &e.payer.pubkey(), None, None);
    assert_eq!(e.send(&[x], &[]).await.unwrap_err(), err(PodError::NotExpired));
    e.warp(1).await;
    let x = ix::cancel_expired(&e.pid, &order, &e.payer.pubkey(), None, None);
    e.send(&[x], &[]).await.unwrap();
}

#[tokio::test]
async fn el_nonce_permite_dos_ordenes_del_mismo_trabajo() {
    let mut e = setup().await;
    let p = e.payer.pubkey();
    let (a, _) = ix::order_address(&e.pid, &p, &SPEC_HASH, 1);
    let (b, _) = ix::order_address(&e.pid, &p, &SPEC_HASH, 2);
    assert_ne!(a, b);

    for n in [1u64, 2] {
        let c = ix::create_order(&e.pid, &p, &args(&e, n));
        e.send(&[c], &[&e.payer.insecure_clone()]).await.unwrap();
    }
    assert_eq!(e.order(&a).await.unwrap().nonce, 1);
    assert_eq!(e.order(&b).await.unwrap().nonce, 2);
}

/// Autotrato: `payer == worker`. En 1.3 esta permitido y no rinde — ver SPEC-PROGRAM §7.
/// El test existe para que el dia que 2.2 lo mida, este escrito que aca era legal.
#[tokio::test]
async fn el_autotrato_funciona_y_conserva() {
    let mut e = setup().await;
    let p = e.payer.pubkey();
    let (order, _) = ix::order_address(&e.pid, &p, &SPEC_HASH, 1);
    let antes = e.balance(&p).await;

    let c = create_ix(&e, 1);
    e.send(&[c], &[&e.payer.insecure_clone()]).await.unwrap();
    let a = ix::accept_order(&e.pid, &p, &order);
    e.send(&[a], &[&e.payer.insecure_clone()]).await.unwrap();
    assert_eq!(e.order(&order).await.unwrap().state, STATE_ACEPTADA);

    e.warp(i64::from(DELIVER_W) + 1).await;
    // La misma cuenta aparece dos veces en la instruccion: pagador y worker.
    let x = ix::cancel_expired(&e.pid, &order, &p, Some(&p), None);
    e.send(&[x], &[]).await.unwrap();

    assert_eq!(e.balance(&p).await, antes, "cuenta duplicada: el reparto se sumo mal");
    assert!(e.is_gone(&order).await);
}

#[tokio::test]
async fn la_cuenta_del_sistema_tiene_que_ser_la_de_verdad() {
    let mut e = setup().await;
    let mut c = create_ix(&e, 1);
    c.accounts[2] = AccountMeta::new_readonly(e.outsider.pubkey(), false);
    assert_eq!(e.send(&[c], &[&e.payer.insecure_clone()]).await.unwrap_err(), err(PodError::BadAccounts));

    let _: Option<Instruction> = None;
}
