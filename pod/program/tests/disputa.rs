//! Gate 1.4 — ..._FASE1.md §5. Los dos escenarios que definen la subfase:
//!
//!   1. Un worker que entrega un `output_hash` falso es challengeado dentro de la ventana
//!      y **pierde el bond**.
//!   2. Un worker honesto challengeado en falso **cobra igual**, y el challenger pierde su
//!      deposito.
//!
//! Los dos se miden igual que el gate 1.3: sumando lamports antes y despues, sin ruido de
//! fees, porque la fee la paga una cuenta que no participa de la orden.

mod common;

use common::*;
use pod_escrow::error::PodError;
use pod_escrow::instruction as ix;
use pod_escrow::state::{STATE_DISPUTADA, STATE_ENTREGADA, VERDICT_FRAUDE, VERDICT_INFUNDADO, ZERO_HASH};
use solana_sdk::instruction::InstructionError;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::TransactionError;

fn err(e: PodError) -> TransactionError {
    TransactionError::InstructionError(0, InstructionError::Custom(e as u32))
}

/// create → accept → deliver(`hash`). Devuelve la direccion del PDA.
async fn hasta_entregada(e: &mut Env, hash: [u8; 32]) -> Pubkey {
    let (order, _) = ix::order_address(&e.pid, &e.payer.pubkey(), &SPEC_HASH, 1);
    e.send(&[create_ix(e, 1)], &[&e.payer.insecure_clone()]).await.unwrap();
    let a = ix::accept_order(&e.pid, &e.worker.pubkey(), &order);
    e.send(&[a], &[&e.worker.insecure_clone()]).await.unwrap();
    let d = ix::deliver(&e.pid, &e.worker.pubkey(), &order, &hash);
    e.send(&[d], &[&e.worker.insecure_clone()]).await.unwrap();
    order
}

async fn hasta_disputada(e: &mut Env, entregado: [u8; 32], reclamado: [u8; 32]) -> Pubkey {
    let order = hasta_entregada(e, entregado).await;
    let c = ix::challenge(&e.pid, &e.challenger.pubkey(), &order, &reclamado);
    e.send(&[c], &[&e.challenger.insecure_clone()]).await.unwrap();
    order
}

async fn saldos(e: &mut Env) -> (u64, u64, u64) {
    (
        e.balance(&e.payer.pubkey()).await,
        e.balance(&e.worker.pubkey()).await,
        e.balance(&e.challenger.pubkey()).await,
    )
}

// ------------------------------------------------------------------ gate 1

#[tokio::test]
async fn gate1_el_worker_fraudulento_pierde_el_bond() {
    let mut e = setup().await;
    let antes = saldos(&mut e).await;

    // El worker entrega un hash que no es el que produce el runner para este pedido.
    let order = hasta_disputada(&mut e, HASH_FALSO, HASH_BUENO).await;
    let o = e.order(&order).await.unwrap();
    assert_eq!(o.state, STATE_DISPUTADA);
    assert_eq!(o.output_hash, HASH_FALSO);
    assert_eq!(o.claimed_output_hash, HASH_BUENO, "el challenger tiene que comprometerse a una respuesta");

    let r = ix::resolve(
        &e.pid, &e.arbiter.pubkey(), &order,
        &e.payer.pubkey(), &e.worker.pubkey(), &e.challenger.pubkey(),
        VERDICT_FRAUDE,
    );
    e.send(&[r], &[&e.arbiter.insecure_clone()]).await.unwrap();

    let d = saldos(&mut e).await;
    assert_eq!(d.0, antes.0, "el pagador recupera recompensa y rent: no pago por trabajo falso");
    assert_eq!(antes.1 - d.1, BOND, "el worker pierde exactamente el bond");
    assert_eq!(d.2 - antes.2, BOND, "el challenger cobra el bond y recupera su deposito");
    assert_eq!(antes.0 + antes.1 + antes.2, d.0 + d.1 + d.2, "I1");
    assert!(e.is_gone(&order).await, "I2");
}

// ------------------------------------------------------------------ gate 2

#[tokio::test]
async fn gate2_el_challenger_infundado_pierde_el_deposito() {
    let mut e = setup().await;
    let antes = saldos(&mut e).await;

    // El worker entrega bien; alguien lo challengea igual.
    let order = hasta_disputada(&mut e, HASH_BUENO, HASH_FALSO).await;

    let r = ix::resolve(
        &e.pid, &e.arbiter.pubkey(), &order,
        &e.payer.pubkey(), &e.worker.pubkey(), &e.challenger.pubkey(),
        VERDICT_INFUNDADO,
    );
    e.send(&[r], &[&e.arbiter.insecure_clone()]).await.unwrap();

    let d = saldos(&mut e).await;
    assert_eq!(antes.0 - d.0, REWARD, "el pagador paga la recompensa: el trabajo estaba bien");
    assert_eq!(d.1 - antes.1, REWARD + DEPOSIT, "el worker cobra y ademas se queda el deposito");
    assert_eq!(antes.2 - d.2, DEPOSIT, "el challenger pierde el deposito");
    assert_eq!(antes.0 + antes.1 + antes.2, d.0 + d.1 + d.2, "I1");
    assert!(e.is_gone(&order).await, "I2");
}

// ------------------------------------------------------------------ camino feliz

#[tokio::test]
async fn sin_challenge_la_ventana_vence_y_el_worker_cobra() {
    let mut e = setup().await;
    let antes = saldos(&mut e).await;
    let order = hasta_entregada(&mut e, HASH_BUENO).await;
    assert_eq!(e.order(&order).await.unwrap().state, STATE_ENTREGADA);

    // Antes de que venza, no se liquida.
    let s = ix::settle(&e.pid, &order, &e.payer.pubkey(), &e.worker.pubkey());
    assert_eq!(e.send(&[s], &[]).await.unwrap_err(), err(PodError::NotExpired));

    e.warp(i64::from(CHALLENGE_W) + 1).await;
    let s = ix::settle(&e.pid, &order, &e.payer.pubkey(), &e.worker.pubkey());
    e.send(&[s], &[]).await.unwrap();

    let d = saldos(&mut e).await;
    assert_eq!(antes.0 - d.0, REWARD, "el pagador paga la recompensa y recupera el rent");
    assert_eq!(d.1 - antes.1, REWARD, "el worker cobra y recupera el bond");
    assert_eq!(d.2, antes.2, "el challenger no existio");
    assert_eq!(antes.0 + antes.1 + antes.2, d.0 + d.1 + d.2, "I1");
    assert!(e.is_gone(&order).await, "I2");
}

// ------------------------------------------------------------------ el arbitro

#[tokio::test]
async fn solo_el_arbitro_declarado_resuelve() {
    let mut e = setup().await;
    let order = hasta_disputada(&mut e, HASH_FALSO, HASH_BUENO).await;
    let intruso = Keypair::new();

    // Ni el pagador, ni el worker, ni un desconocido.
    for quien in [e.payer.insecure_clone(), e.worker.insecure_clone(), intruso.insecure_clone()] {
        let r = ix::resolve(
            &e.pid, &quien.pubkey(), &order,
            &e.payer.pubkey(), &e.worker.pubkey(), &e.challenger.pubkey(),
            VERDICT_FRAUDE,
        );
        let got = e.send(&[r], &[&quien]).await.unwrap_err();
        assert_eq!(got, err(PodError::WrongArbiter), "resolvio {}", quien.pubkey());
    }

    // Y el arbitro de verdad tampoco puede desviar un pago cambiando un destino.
    let ladron = Keypair::new().pubkey();
    for (p, w, c, esperado) in [
        (ladron, e.worker.pubkey(), e.challenger.pubkey(), PodError::WrongPayer),
        (e.payer.pubkey(), ladron, e.challenger.pubkey(), PodError::WrongWorker),
        (e.payer.pubkey(), e.worker.pubkey(), ladron, PodError::WrongChallenger),
    ] {
        let r = ix::resolve(&e.pid, &e.arbiter.pubkey(), &order, &p, &w, &c, VERDICT_FRAUDE);
        assert_eq!(e.send(&[r], &[&e.arbiter.insecure_clone()]).await.unwrap_err(), err(esperado));
    }
}

#[tokio::test]
async fn un_veredicto_que_no_existe_se_rechaza() {
    let mut e = setup().await;
    let order = hasta_disputada(&mut e, HASH_FALSO, HASH_BUENO).await;
    for v in [0u8, 3, 9, 255] {
        let r = ix::resolve(
            &e.pid, &e.arbiter.pubkey(), &order,
            &e.payer.pubkey(), &e.worker.pubkey(), &e.challenger.pubkey(), v,
        );
        assert_eq!(
            e.send(&[r], &[&e.arbiter.insecure_clone()]).await.unwrap_err(),
            err(PodError::BadVerdict),
            "el veredicto {v} hizo algo"
        );
    }
}

/// La frontera de la opcion (a): el arbitro es un tercero de confianza, y un tercero de
/// confianza puede desaparecer. Si no resuelve a tiempo, la disputa **no se decide**: se
/// desarma, y cada uno recupera lo suyo. Ver SPEC-PROGRAM.md §7.
#[tokio::test]
async fn un_arbitro_que_no_resuelve_a_tiempo_no_decide_nada() {
    let mut e = setup().await;
    let antes = saldos(&mut e).await;
    let order = hasta_disputada(&mut e, HASH_FALSO, HASH_BUENO).await;

    e.warp(i64::from(CHALLENGE_W) + 1).await;

    // Llegar tarde no sirve de nada, ni siquiera con el veredicto correcto.
    let r = ix::resolve(
        &e.pid, &e.arbiter.pubkey(), &order,
        &e.payer.pubkey(), &e.worker.pubkey(), &e.challenger.pubkey(),
        VERDICT_FRAUDE,
    );
    assert_eq!(e.send(&[r], &[&e.arbiter.insecure_clone()]).await.unwrap_err(), err(PodError::Expired));

    // La salida es desarmar la disputa. Cualquiera puede hacerlo.
    let x = ix::cancel_expired(
        &e.pid, &order, &e.payer.pubkey(),
        Some(&e.worker.pubkey()), Some(&e.challenger.pubkey()),
    );
    e.send(&[x], &[]).await.unwrap();

    let d = saldos(&mut e).await;
    assert_eq!(d, antes, "nadie gana ni pierde cuando el arbitro no aparece");
    assert!(e.is_gone(&order).await, "I2");
}

// ------------------------------------------------------------------ los bordes

#[tokio::test]
async fn solo_el_worker_entrega_y_una_sola_vez() {
    let mut e = setup().await;
    let (order, _) = ix::order_address(&e.pid, &e.payer.pubkey(), &SPEC_HASH, 1);
    e.send(&[create_ix(&e, 1)], &[&e.payer.insecure_clone()]).await.unwrap();

    // Sin aceptar todavia, no hay nada que entregar.
    let d = ix::deliver(&e.pid, &e.worker.pubkey(), &order, &HASH_BUENO);
    assert_eq!(e.send(&[d], &[&e.worker.insecure_clone()]).await.unwrap_err(), err(PodError::BadState));

    let a = ix::accept_order(&e.pid, &e.worker.pubkey(), &order);
    e.send(&[a], &[&e.worker.insecure_clone()]).await.unwrap();

    // Otro que no es el worker que acepto.
    let d = ix::deliver(&e.pid, &e.outsider.pubkey(), &order, &HASH_BUENO);
    assert_eq!(e.send(&[d], &[&e.outsider.insecure_clone()]).await.unwrap_err(), err(PodError::WrongWorker));

    // Un hash en ceros no se distingue de un campo sin inicializar.
    let d = ix::deliver(&e.pid, &e.worker.pubkey(), &order, &ZERO_HASH);
    assert_eq!(e.send(&[d], &[&e.worker.insecure_clone()]).await.unwrap_err(), err(PodError::ZeroHash));

    let d = ix::deliver(&e.pid, &e.worker.pubkey(), &order, &HASH_BUENO);
    e.send(&[d.clone()], &[&e.worker.insecure_clone()]).await.unwrap();
    // Y no se puede reescribir la entrega para tapar un challenge.
    assert_eq!(e.send(&[d], &[&e.worker.insecure_clone()]).await.unwrap_err(), err(PodError::BadState));
}

#[tokio::test]
async fn no_se_entrega_despues_del_plazo_de_entrega() {
    let mut e = setup().await;
    let (order, _) = ix::order_address(&e.pid, &e.payer.pubkey(), &SPEC_HASH, 1);
    e.send(&[create_ix(&e, 1)], &[&e.payer.insecure_clone()]).await.unwrap();
    let a = ix::accept_order(&e.pid, &e.worker.pubkey(), &order);
    e.send(&[a], &[&e.worker.insecure_clone()]).await.unwrap();

    e.warp(i64::from(DELIVER_W) + 1).await;
    let d = ix::deliver(&e.pid, &e.worker.pubkey(), &order, &HASH_BUENO);
    assert_eq!(e.send(&[d], &[&e.worker.insecure_clone()]).await.unwrap_err(), err(PodError::Expired));
}

#[tokio::test]
async fn el_challenge_tiene_sus_limites() {
    let mut e = setup().await;
    let order = hasta_entregada(&mut e, HASH_BUENO).await;

    // Un challenge que afirma el mismo hash no es un challenge.
    let c = ix::challenge(&e.pid, &e.challenger.pubkey(), &order, &HASH_BUENO);
    assert_eq!(e.send(&[c], &[&e.challenger.insecure_clone()]).await.unwrap_err(), err(PodError::SameHash));

    let c = ix::challenge(&e.pid, &e.challenger.pubkey(), &order, &ZERO_HASH);
    assert_eq!(e.send(&[c], &[&e.challenger.insecure_clone()]).await.unwrap_err(), err(PodError::ZeroHash));

    // Un segundo challenger no puede pisar al primero.
    let c = ix::challenge(&e.pid, &e.challenger.pubkey(), &order, &HASH_FALSO);
    e.send(&[c], &[&e.challenger.insecure_clone()]).await.unwrap();
    let c = ix::challenge(&e.pid, &e.outsider.pubkey(), &order, &[0x99; 32]);
    assert_eq!(e.send(&[c], &[&e.outsider.insecure_clone()]).await.unwrap_err(), err(PodError::BadState));
}

#[tokio::test]
async fn no_se_challengea_ni_se_cancela_fuera_de_la_ventana() {
    let mut e = setup().await;
    let order = hasta_entregada(&mut e, HASH_BUENO).await;

    // ENTREGADA no se cancela: su salida es `settle`, no un reembolso.
    let x = ix::cancel_expired(&e.pid, &order, &e.payer.pubkey(), Some(&e.worker.pubkey()), None);
    assert_eq!(e.send(&[x], &[]).await.unwrap_err(), err(PodError::BadState));

    e.warp(i64::from(CHALLENGE_W) + 1).await;
    let c = ix::challenge(&e.pid, &e.challenger.pubkey(), &order, &HASH_FALSO);
    assert_eq!(
        e.send(&[c], &[&e.challenger.insecure_clone()]).await.unwrap_err(),
        err(PodError::Expired),
        "un challenge tardio reabriria una orden ya liquidable"
    );
}

#[tokio::test]
async fn un_challenger_sin_deposito_no_puede_challengear() {
    let mut e = setup().await;
    let order = hasta_entregada(&mut e, HASH_FALSO).await;

    // Un challenger sin fondos: la transferencia falla y con ella la instruccion entera.
    let pobre = Keypair::new();
    let c = ix::challenge(&e.pid, &pobre.pubkey(), &order, &HASH_BUENO);
    assert!(e.send(&[c], &[&pobre]).await.is_err(), "challengeo gratis");
    assert_eq!(e.order(&order).await.unwrap().state, STATE_ENTREGADA, "el estado avanzo sin deposito");
}
