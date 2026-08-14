//! El dato a registrar de la subfase — ..._FASE1.md §4: "costo de rent + fees por orden".
//!
//! No es un gate y no falla por un umbral: mide y escribe. Alimenta el KPI 1 y la tabla de
//! E0.1, porque si el piso de costo por orden es comparable al valor de una tarea chica,
//! el tamaño minimo de tarea economicamente viable sube.
//!
//! La distincion que hace toda la diferencia: **el rent es un deposito, no un costo.**
//! Vuelve entero al cerrar (I2). Lo que se quema son las fees.

mod common;

use common::*;
use pod_escrow::instruction as ix;
use pod_escrow::state::ORDER_LEN;
use solana_sdk::signature::Signer;

fn sol(l: u64) -> String {
    format!("{:>13} lamports  =  {:.9} SOL", l, l as f64 / 1e9)
}

#[tokio::test]
async fn costo_por_orden() {
    let mut e = setup().await;
    let p = e.payer.pubkey();
    let (order, _) = ix::order_address(&e.pid, &p, &SPEC_HASH, 1);
    let banco = e.ctx.payer.pubkey();

    let mut fees = Vec::new();
    let mut anterior = e.balance(&banco).await;
    let mut cobrar = |ahora: u64, anterior: &mut u64| {
        let f = *anterior - ahora;
        *anterior = ahora;
        f
    };

    e.send(&[create_ix(&e, 1)], &[&e.payer.insecure_clone()]).await.unwrap();
    fees.push(("create_order", cobrar(e.balance(&banco).await, &mut anterior)));

    let a = ix::accept_order(&e.pid, &e.worker.pubkey(), &order);
    e.send(&[a], &[&e.worker.insecure_clone()]).await.unwrap();
    fees.push(("accept_order", cobrar(e.balance(&banco).await, &mut anterior)));

    let d = ix::deliver(&e.pid, &e.worker.pubkey(), &order, &HASH_BUENO);
    e.send(&[d], &[&e.worker.insecure_clone()]).await.unwrap();
    fees.push(("deliver", cobrar(e.balance(&banco).await, &mut anterior)));

    let rent = e.order(&order).await.unwrap().rent_lamports;

    e.warp(i64::from(CHALLENGE_W) + 1).await;
    let s = ix::settle(&e.pid, &order, &p, &e.worker.pubkey());
    e.send(&[s], &[]).await.unwrap();
    fees.push(("settle", cobrar(e.balance(&banco).await, &mut anterior)));

    println!("
----------------------------------------------------------------------");
    println!("COSTO POR ORDEN — {ORDER_LEN} bytes de cuenta");
    println!("----------------------------------------------------------------------");
    println!("  rent exento (DEPOSITO, vuelve entero al cerrar — I2)");
    println!("    {}", sol(rent));
    println!("  camino normal, 4 transacciones. Fees medidas por el harness, que las paga");
    println!("  con una cuenta aparte y por eso suma una firma de mas en tres de ellas:");
    for (nombre, f) in &fees {
        println!("    {nombre:14}  {}", sol(*f));
    }
    println!("----------------------------------------------------------------------");
    println!("  En produccion cada actor paga su propia transaccion: una firma, 5000 c/u.");
    println!("    camino normal  (create+accept+deliver+settle)     {}", sol(4 * 5_000));
    println!("    camino disputado (+challenge +resolve)            {}", sol(6 * 5_000));
    println!("----------------------------------------------------------------------");
    println!("  Capital inmovilizado durante la orden = rent + recompensa + bond");
    println!("    {}", sol(rent + REWARD + BOND));
    println!("  y el deposito del challenger, si challengea:  {}", sol(DEPOSIT));
    println!("  El piso de costo por orden es despreciable frente a eso. La restriccion");
    println!("  economica no es lo que cuesta abrir una orden: es el capital inmovilizado");
    println!("  durante la ventana, que 1.4 ALARGA al sumarle la ventana de challenge.");
    println!("  Eso es lo que va a la tabla de E0.1, no la fee.");
    println!("----------------------------------------------------------------------
");

    assert!(e.is_gone(&order).await);
    assert!(fees.iter().all(|(_, f)| *f > 0));
}
