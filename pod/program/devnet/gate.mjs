// Gate 1.4 contra una cadena de verdad — ..._FASE1.md §5.
//
//   node gate.mjs [url]     por defecto http://127.0.0.1:8899
//
// Los tests de `scripts/gate.sh` corren el programa nativo, en proceso. Esto corre el .so
// compilado a SBF sobre un validador: VM real, presupuesto de computo real, fees reales y
// reloj real. Es la diferencia entre "la logica es correcta" y "el programa existe".

import { readFileSync } from "node:fs";
import { Connection, Keypair, LAMPORTS_PER_SOL, PublicKey, Transaction, sendAndConfirmTransaction } from "@solana/web3.js";
import * as ix from "./ix.mjs";

const URL = process.argv[2] ?? "http://127.0.0.1:8899";
const conn = new Connection(URL, "confirmed");
const kp = (n) => Keypair.fromSecretKey(Uint8Array.from(JSON.parse(readFileSync(`${import.meta.dirname}/${n}.json`))));

const PROGRAM = kp("program-keypair").publicKey;
const payer = kp("payer"), worker = kp("worker"), challenger = kp("challenger"), arbiter = kp("arbiter");

const REWARD = 0.2 * LAMPORTS_PER_SOL, BOND = 0.1 * LAMPORTS_PER_SOL, DEPOSIT = 0.05 * LAMPORTS_PER_SOL;
const HASH_BUENO = Buffer.alloc(32, 0x7c), HASH_FALSO = Buffer.alloc(32, 0xde);
const WINDOW = 60;

let fallas = 0;
const check = (ok, label) => { console.log(`  [${ok ? "ok " : "FALLA"}] ${label}`); if (!ok) fallas++; };
const bal = (k) => conn.getBalance(k, "confirmed");

/** La fee la paga siempre `feePayer`, que no participa de la orden: asi las cuentas que
 *  importan conservan lamports exactamente, igual que en los tests de Rust. */
const feePayer = arbiter;
async function send(instrucciones, firmantes) {
  const tx = new Transaction().add(...instrucciones);
  return sendAndConfirmTransaction(conn, tx, [feePayer, ...firmantes], { commitment: "confirmed" });
}

const args = (nonce, specHash) => ({
  nonce, specHash, arbiter: arbiter.publicKey,
  reward: REWARD, bond: BOND, deposit: DEPOSIT,
  acceptWindow: WINDOW, deliverWindow: WINDOW, challengeWindow: WINDOW,
});

async function escenario(nombre, nonce, entregado, reclamado, verdict, esperado) {
  console.log(`\n${"=".repeat(70)}\n${nombre}\n${"=".repeat(70)}`);
  const specHash = Buffer.alloc(32, nonce);
  const order = ix.orderAddress(PROGRAM, payer.publicKey, specHash, nonce);
  const antes = { p: await bal(payer.publicKey), w: await bal(worker.publicKey), c: await bal(challenger.publicKey) };

  await send([ix.createOrder(PROGRAM, payer.publicKey, args(nonce, specHash))], [payer]);
  await send([ix.acceptOrder(PROGRAM, worker.publicKey, order)], [worker]);
  await send([ix.deliver(PROGRAM, worker.publicKey, order, entregado)], [worker]);
  await send([ix.challenge(PROGRAM, challenger.publicKey, order, reclamado)], [challenger]);

  const cuenta = ix.unpackOrder((await conn.getAccountInfo(order, "confirmed")).data);
  check(cuenta.state === 4, "la orden quedo en DISPUTADA");
  check(cuenta.outputHash === entregado.toString("hex"), "el output_hash entregado es el que se grabo");
  check(cuenta.claimedOutputHash === reclamado.toString("hex"), "el challenger quedo comprometido a una respuesta");

  const sig = await send([ix.resolve(PROGRAM, arbiter.publicKey, order, payer.publicKey, worker.publicKey, challenger.publicKey, verdict)], [arbiter]);
  console.log(`  resolve: ${sig}`);

  const d = { p: await bal(payer.publicKey), w: await bal(worker.publicKey), c: await bal(challenger.publicKey) };
  const delta = { p: d.p - antes.p, w: d.w - antes.w, c: d.c - antes.c };
  console.log(`  pagador ${delta.p}  worker ${delta.w}  challenger ${delta.c}  (lamports)`);
  check(delta.p === esperado.p, `el pagador mueve ${esperado.p}`);
  check(delta.w === esperado.w, `el worker mueve ${esperado.w}`);
  check(delta.c === esperado.c, `el challenger mueve ${esperado.c}`);
  check(delta.p + delta.w + delta.c === 0, "I1: la suma de las tres cuentas se conserva");
  check((await conn.getAccountInfo(order, "confirmed")) === null, "I2: la cuenta se cerro y el rent volvio");
}

const version = await conn.getVersion();
console.log(`cadena: ${URL}  solana-core ${version["solana-core"]}`);
console.log(`programa: ${PROGRAM.toBase58()}`);
const info = await conn.getAccountInfo(PROGRAM);
if (!info || !info.executable) { console.error("el programa no esta desplegado en esa cadena"); process.exit(2); }

await escenario(
  "GATE 1.4 (1) — el worker entrega un output_hash falso y pierde el bond",
  1, HASH_FALSO, HASH_BUENO, ix.VERDICT.FRAUDE,
  { p: 0, w: -BOND, c: BOND },
);
await escenario(
  "GATE 1.4 (2) — el worker es honesto y el challenger pierde el deposito",
  2, HASH_BUENO, HASH_FALSO, ix.VERDICT.INFUNDADO,
  { p: -REWARD, w: REWARD + DEPOSIT, c: -DEPOSIT },
);

console.log(`\n${"=".repeat(70)}`);
console.log(fallas ? `RESULTADO: ${fallas} falla(s)` : "RESULTADO: los dos escenarios del gate 1.4 pasan en cadena");
process.exit(fallas ? 1 : 0);
