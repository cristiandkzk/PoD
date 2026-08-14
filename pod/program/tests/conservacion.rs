//! Gate 1.3, punto 1 — SPEC-PROGRAM.md §5, invariantes I1, I2 e I3.
//!
//! "Para cada camino, la suma de balances se conserva; ninguna cuenta queda con fondos
//! huerfanos ni rent atrapado" (..._FASE1.md §4).
//!
//! Los tres caminos que existen en 1.3 son: CREADA→ACEPTADA, CREADA→cerrada y
//! ACEPTADA→cerrada. Cada uno tiene su test, y los tres miden lo mismo: la suma de
//! pagador + worker + PDA antes y despues.

mod common;

use common::*;
use pod_escrow::instruction as ix;
use pod_escrow::state::{STATE_ACEPTADA, STATE_CREADA};
use solana_sdk::signature::Signer;

#[tokio::test]
async fn i1_camino_aceptado_conserva_y_deja_todo_en_el_escrow() {
    let mut e = setup().await;
    let (order, _) = ix::order_address(&e.pid, &e.payer.pubkey(), &SPEC_HASH, 1);

    let antes = (e.balance(&e.payer.pubkey()).await, e.balance(&e.worker.pubkey()).await, 0u64);

    let c = create_ix(&e, 1);
    e.send(&[c], &[&e.payer.insecure_clone()]).await.unwrap();
    let o = e.order(&order).await.expect("la orden existe");
    assert_eq!(o.state, STATE_CREADA);
    assert_eq!(o.worker, solana_sdk::pubkey::Pubkey::default(), "worker en ceros mientras CREADA");
    assert_eq!(o.deliver_deadline, 0);

    let a = ix::accept_order(&e.pid, &e.worker.pubkey(), &order);
    e.send(&[a], &[&e.worker.insecure_clone()]).await.unwrap();

    let o = e.order(&order).await.expect("la orden sigue");
    assert_eq!(o.state, STATE_ACEPTADA);
    assert_eq!(o.worker, e.worker.pubkey());

    let despues = (
        e.balance(&e.payer.pubkey()).await,
        e.balance(&e.worker.pubkey()).await,
        e.balance(&order).await,
    );

    // I1: la suma de las tres cuentas es exactamente la misma. La fee la pago ctx.payer.
    assert_eq!(
        antes.0 + antes.1 + antes.2,
        despues.0 + despues.1 + despues.2,
        "I1: la suma de lamports cambio"
    );
    // Y el reparto es el declarado: el escrow tiene rent + recompensa + bond.
    assert_eq!(despues.2, o.rent_lamports + REWARD + BOND);
    assert_eq!(antes.0 - despues.0, o.rent_lamports + REWARD, "el pagador puso rent + recompensa");
    assert_eq!(antes.1 - despues.1, BOND, "el worker puso exactamente el bond");
}

#[tokio::test]
async fn i2_camino_cancelado_devuelve_todo_incluido_el_rent() {
    let mut e = setup().await;
    let (order, _) = ix::order_address(&e.pid, &e.payer.pubkey(), &SPEC_HASH, 1);
    let antes = e.balance(&e.payer.pubkey()).await;

    let c = create_ix(&e, 1);
    e.send(&[c], &[&e.payer.insecure_clone()]).await.unwrap();
    e.warp(i64::from(ACCEPT_W) + 1).await;

    let x = ix::cancel_expired(&e.pid, &order, &e.payer.pubkey(), None, None);
    e.send(&[x], &[]).await.unwrap();

    assert_eq!(e.balance(&e.payer.pubkey()).await, antes, "I2: el pagador tiene que quedar igual que al empezar");
    assert!(e.is_gone(&order).await, "I2: el PDA no se cerro — el rent quedo atrapado");
}

#[tokio::test]
async fn i2_camino_aceptado_y_vencido_devuelve_a_los_dos() {
    let mut e = setup().await;
    let (order, _) = ix::order_address(&e.pid, &e.payer.pubkey(), &SPEC_HASH, 1);
    let antes = (e.balance(&e.payer.pubkey()).await, e.balance(&e.worker.pubkey()).await);

    e.send(&[create_ix(&e, 1)], &[&e.payer.insecure_clone()]).await.unwrap();
    let a = ix::accept_order(&e.pid, &e.worker.pubkey(), &order);
    e.send(&[a], &[&e.worker.insecure_clone()]).await.unwrap();

    e.warp(i64::from(DELIVER_W) + 1).await;
    let x = ix::cancel_expired(&e.pid, &order, &e.payer.pubkey(), Some(&e.worker.pubkey()), None);
    e.send(&[x], &[]).await.unwrap();

    assert_eq!(e.balance(&e.payer.pubkey()).await, antes.0, "el pagador recupera recompensa y rent");
    assert_eq!(e.balance(&e.worker.pubkey()).await, antes.1, "el worker recupera el bond entero (no hay slashing en 1.3)");
    assert!(e.is_gone(&order).await, "I2: el PDA no se cerro");
}

#[tokio::test]
async fn i3_los_lamports_donados_al_pda_no_quedan_huerfanos() {
    let mut e = setup().await;
    let (order, _) = ix::order_address(&e.pid, &e.payer.pubkey(), &SPEC_HASH, 7);
    let donacion = 1_234_567u64;

    let payer_antes = e.balance(&e.payer.pubkey()).await;
    let outsider_antes = e.balance(&e.outsider.pubkey()).await;

    // La direccion del PDA es publica y calculable antes de que la orden exista (§3.1).
    // Un tercero le manda lamports para intentar bloquear la creacion.
    let d = solana_system_interface::instruction::transfer(&e.outsider.pubkey(), &order, donacion);
    e.send(&[d], &[&e.outsider.insecure_clone()]).await.unwrap();
    assert_eq!(e.balance(&order).await, donacion);

    // §4.1: con transfer/allocate/assign la creacion igual funciona.
    e.send(&[create_ix(&e, 7)], &[&e.payer.insecure_clone()]).await.unwrap();
    e.warp(i64::from(ACCEPT_W) + 1).await;
    let x = ix::cancel_expired(&e.pid, &order, &e.payer.pubkey(), None, None);
    e.send(&[x], &[]).await.unwrap();

    assert!(e.is_gone(&order).await, "I3: el PDA quedo con saldo");
    assert_eq!(
        e.balance(&e.payer.pubkey()).await,
        payer_antes + donacion,
        "I3: la donacion tiene que salir con el resto, hacia el pagador"
    );
    assert_eq!(e.balance(&e.outsider.pubkey()).await, outsider_antes - donacion);
}
