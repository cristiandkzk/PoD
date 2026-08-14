//! Gate 1.3, puntos 2, 3 y 4 — SPEC-PROGRAM.md §5, invariantes I4, I5, I6 e I7.
//!
//!   2. No se puede aceptar sin bond.
//!   3. El pagador no puede cancelar despues de ACEPTADA.
//!   4. No existe ninguna ruta de retiro del escrow fuera de las declaradas.
//!
//! Un gate negativo no se prueba mostrando que algo anda: se prueba intentando romperlo.

mod common;

use common::*;
use pod_escrow::error::PodError;
use pod_escrow::instruction as ix;
use pod_escrow::state::STATE_CREADA;
use solana_sdk::instruction::{AccountMeta, Instruction, InstructionError};
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::TransactionError;

fn err(e: PodError) -> TransactionError {
    TransactionError::InstructionError(0, InstructionError::Custom(e as u32))
}

#[tokio::test]
async fn i4_un_worker_sin_bond_no_puede_aceptar() {
    // El worker arranca con menos de lo que exige el bond.
    let mut e = setup_with(5 * SOL, BOND - 1).await;
    let (order, _) = ix::order_address(&e.pid, &e.payer.pubkey(), &SPEC_HASH, 1);
    e.send(&[create_ix(&e, 1)], &[&e.payer.insecure_clone()]).await.unwrap();

    let escrow_antes = e.balance(&order).await;
    let worker_antes = e.balance(&e.worker.pubkey()).await;

    let a = ix::accept_order(&e.pid, &e.worker.pubkey(), &order);
    let r = e.send(&[a], &[&e.worker.insecure_clone()]).await;
    assert!(r.is_err(), "I4: acepto sin tener el bond");

    // Mas importante que el error: nada se movio y el estado no avanzo.
    assert_eq!(e.balance(&order).await, escrow_antes);
    assert_eq!(e.balance(&e.worker.pubkey()).await, worker_antes);
    assert_eq!(e.order(&order).await.unwrap().state, STATE_CREADA, "I4: quedo ACEPTADA sin bond");
}

#[tokio::test]
async fn i5_el_pagador_no_puede_cancelar_despues_de_aceptada() {
    let mut e = setup().await;
    let (order, _) = ix::order_address(&e.pid, &e.payer.pubkey(), &SPEC_HASH, 1);
    e.send(&[create_ix(&e, 1)], &[&e.payer.insecure_clone()]).await.unwrap();
    let a = ix::accept_order(&e.pid, &e.worker.pubkey(), &order);
    e.send(&[a], &[&e.worker.insecure_clone()]).await.unwrap();

    // El plazo de aceptacion ya vencio, el de entrega no. Es la ventana en la que un
    // pagador arrepentido querria salirse.
    e.warp(i64::from(ACCEPT_W) + 1).await;

    // (a) La forma "CREADA" de la instruccion, con dos cuentas.
    let x = ix::cancel_expired(&e.pid, &order, &e.payer.pubkey(), None, None);
    assert_eq!(e.send(&[x], &[]).await.unwrap_err(), err(PodError::BadAccounts));

    // (b) La forma completa, antes del vencimiento de entrega.
    let x = ix::cancel_expired(&e.pid, &order, &e.payer.pubkey(), Some(&e.worker.pubkey()), None);
    assert_eq!(e.send(&[x], &[]).await.unwrap_err(), err(PodError::NotExpired));

    // (c) Firmar no cambia nada: la instruccion no mira firmas, mira el estado.
    let mut x = ix::cancel_expired(&e.pid, &order, &e.payer.pubkey(), Some(&e.worker.pubkey()), None);
    x.accounts[1] = AccountMeta::new(e.payer.pubkey(), true);
    assert_eq!(
        e.send(&[x], &[&e.payer.insecure_clone()]).await.unwrap_err(),
        err(PodError::NotExpired),
        "I5: la firma del pagador no puede abrir una puerta que el estado cierra"
    );

    let o = e.order(&order).await.unwrap();
    assert_eq!(e.balance(&order).await, o.rent_lamports + REWARD + BOND);
}

#[tokio::test]
async fn i6_el_crank_es_permissionless_pero_no_cobra() {
    let mut e = setup().await;
    let (order, _) = ix::order_address(&e.pid, &e.payer.pubkey(), &SPEC_HASH, 1);
    let antes = (e.balance(&e.payer.pubkey()).await, e.balance(&e.worker.pubkey()).await);

    e.send(&[create_ix(&e, 1)], &[&e.payer.insecure_clone()]).await.unwrap();
    let a = ix::accept_order(&e.pid, &e.worker.pubkey(), &order);
    e.send(&[a], &[&e.worker.insecure_clone()]).await.unwrap();
    e.warp(i64::from(DELIVER_W) + 1).await;

    // La manda un tercero: no firma nada y no tiene relacion con la orden.
    let x = ix::cancel_expired(&e.pid, &order, &e.payer.pubkey(), Some(&e.worker.pubkey()), None);
    e.send(&[x], &[]).await.unwrap();

    // Sale bien, y el que la mando no se lleva nada: los destinos son los pubkeys grabados
    // en la cuenta, no un parametro de la instruccion.
    assert_eq!(e.balance(&e.payer.pubkey()).await, antes.0);
    assert_eq!(e.balance(&e.worker.pubkey()).await, antes.1);
    assert!(e.is_gone(&order).await);
}

#[tokio::test]
async fn i6_no_se_puede_desviar_el_reembolso() {
    let mut e = setup().await;
    let (order, _) = ix::order_address(&e.pid, &e.payer.pubkey(), &SPEC_HASH, 1);
    e.send(&[create_ix(&e, 1)], &[&e.payer.insecure_clone()]).await.unwrap();
    let a = ix::accept_order(&e.pid, &e.worker.pubkey(), &order);
    e.send(&[a], &[&e.worker.insecure_clone()]).await.unwrap();
    e.warp(i64::from(DELIVER_W) + 1).await;

    let ladron = Keypair::new().pubkey();

    let x = ix::cancel_expired(&e.pid, &order, &ladron, Some(&e.worker.pubkey()), None);
    assert_eq!(e.send(&[x], &[]).await.unwrap_err(), err(PodError::WrongPayer));

    let x = ix::cancel_expired(&e.pid, &order, &e.payer.pubkey(), Some(&ladron), None);
    assert_eq!(e.send(&[x], &[]).await.unwrap_err(), err(PodError::WrongWorker));

    // El escrow sigue intacto despues de los dos intentos.
    let o = e.order(&order).await.unwrap();
    assert_eq!(e.balance(&order).await, o.rent_lamports + REWARD + BOND);
}

#[tokio::test]
async fn i7_no_hay_una_octava_instruccion() {
    let mut e = setup().await;
    let (order, _) = ix::order_address(&e.pid, &e.payer.pubkey(), &SPEC_HASH, 1);
    e.send(&[create_ix(&e, 1)], &[&e.payer.insecure_clone()]).await.unwrap();

    let cuentas = vec![
        AccountMeta::new(order, false),
        AccountMeta::new(e.payer.pubkey(), true),
        AccountMeta::new_readonly(solana_system_interface::program::ID, false),
    ];
    // 0..=6 existen (§4). 7 en adelante, no.
    for disc in [7u8, 8, 9, 42, 0x80, 0xff] {
        let bad = Instruction { program_id: e.pid, accounts: cuentas.clone(), data: vec![disc] };
        assert_eq!(
            e.send(&[bad], &[&e.payer.insecure_clone()]).await.unwrap_err(),
            err(PodError::BadInstruction),
            "I7: el discriminante {disc} hizo algo"
        );
    }
    // Longitudes que no son la exacta, con discriminantes que si existen.
    // Longitudes pegadas a la exacta, con discriminantes que si existen.
    for (disc, len) in [(0u8, 108usize), (0, 110), (1, 2), (2, 0), (2, 5),
                        (3, 32), (3, 34), (4, 32), (5, 1), (5, 3), (6, 2)] {
        let mut data = vec![0u8; len];
        if len > 0 {
            data[0] = disc;
        }
        let bad = Instruction { program_id: e.pid, accounts: cuentas.clone(), data };
        assert_eq!(
            e.send(&[bad], &[&e.payer.insecure_clone()]).await.unwrap_err(),
            err(PodError::BadInstruction),
            "I7: disc {disc} con {len} bytes paso"
        );
    }
}

#[tokio::test]
async fn i7_una_cuenta_ajena_no_se_hace_pasar_por_orden() {
    let mut e = setup().await;
    let otro = Keypair::new();
    e.send(&[create_ix(&e, 1)], &[&e.payer.insecure_clone()]).await.unwrap();

    // (a) Un PDA calculado para otro pagador no es el de esta orden.
    let (ajeno, _) = ix::order_address(&e.pid, &otro.pubkey(), &SPEC_HASH, 1);
    let a = ix::accept_order(&e.pid, &e.worker.pubkey(), &ajeno);
    assert_eq!(e.send(&[a], &[&e.worker.insecure_clone()]).await.unwrap_err(), err(PodError::BadPda));

    // (b) Una cuenta cualquiera del sistema tampoco.
    let a = ix::accept_order(&e.pid, &e.worker.pubkey(), &e.outsider.pubkey());
    assert_eq!(e.send(&[a], &[&e.worker.insecure_clone()]).await.unwrap_err(), err(PodError::BadPda));

    // (c) Crear dos veces la misma orden no reabre nada.
    assert_eq!(
        e.send(&[create_ix(&e, 1)], &[&e.payer.insecure_clone()]).await.unwrap_err(),
        err(PodError::AlreadyExists)
    );
}

#[tokio::test]
async fn i7_una_orden_cerrada_no_revive() {
    let mut e = setup().await;
    let (order, _) = ix::order_address(&e.pid, &e.payer.pubkey(), &SPEC_HASH, 1);
    e.send(&[create_ix(&e, 1)], &[&e.payer.insecure_clone()]).await.unwrap();
    e.warp(i64::from(ACCEPT_W) + 1).await;
    let x = ix::cancel_expired(&e.pid, &order, &e.payer.pubkey(), None, None);
    e.send(&[x], &[]).await.unwrap();

    // Segundo cobro sobre la misma cuenta: ya no es del programa.
    let x = ix::cancel_expired(&e.pid, &order, &e.payer.pubkey(), None, None);
    assert_eq!(e.send(&[x], &[]).await.unwrap_err(), err(PodError::BadPda));

    let a = ix::accept_order(&e.pid, &e.worker.pubkey(), &order);
    assert_eq!(e.send(&[a], &[&e.worker.insecure_clone()]).await.unwrap_err(), err(PodError::BadPda));
}
