// Las siete instrucciones, armadas desde SPEC-PROGRAM.md §4.
//
// Este archivo no comparte una linea con el programa: es una **segunda implementacion** de
// la normativa, escrita desde el documento, igual que `python/` y `rust/` en `pod/spec`. Si
// el documento fuera ambiguo, este cliente y el programa no se entenderian.

import { PublicKey, SystemProgram, SYSVAR_CLOCK_PUBKEY, SYSVAR_RENT_PUBKEY, TransactionInstruction } from "@solana/web3.js";

export const DISC = { CREATE: 0, ACCEPT: 1, CANCEL: 2, DELIVER: 3, CHALLENGE: 4, RESOLVE: 5, SETTLE: 6 };
export const VERDICT = { FRAUDE: 1, INFUNDADO: 2 };

const u64 = (n) => { const b = Buffer.alloc(8); b.writeBigUInt64LE(BigInt(n)); return b; };
const u32 = (n) => { const b = Buffer.alloc(4); b.writeUInt32LE(n); return b; };

const rw = (k) => ({ pubkey: k, isSigner: false, isWritable: true });
const ro = (k) => ({ pubkey: k, isSigner: false, isWritable: false });
const signer = (k, writable) => ({ pubkey: k, isSigner: true, isWritable: writable });

/** §3.1: seeds = ["order", payer, spec_hash, nonce u64 LE]. */
export function orderAddress(programId, payer, specHash, nonce) {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("order"), payer.toBuffer(), Buffer.from(specHash), u64(nonce)],
    programId,
  )[0];
}

export function createOrder(programId, payer, a) {
  const order = orderAddress(programId, payer, a.specHash, a.nonce);
  const data = Buffer.concat([
    Buffer.from([DISC.CREATE]),
    u64(a.nonce),
    Buffer.from(a.specHash),
    a.arbiter.toBuffer(),
    u64(a.reward),
    u64(a.bond),
    u64(a.deposit),
    u32(a.acceptWindow),
    u32(a.deliverWindow),
    u32(a.challengeWindow),
  ]);
  if (data.length !== 109) throw new Error(`create_order son 109 bytes, no ${data.length}`);
  return new TransactionInstruction({
    programId,
    keys: [signer(payer, true), rw(order), ro(SystemProgram.programId), ro(SYSVAR_RENT_PUBKEY), ro(SYSVAR_CLOCK_PUBKEY)],
    data,
  });
}

export const acceptOrder = (programId, worker, order) => new TransactionInstruction({
  programId,
  keys: [signer(worker, true), rw(order), ro(SystemProgram.programId), ro(SYSVAR_CLOCK_PUBKEY)],
  data: Buffer.from([DISC.ACCEPT]),
});

export const deliver = (programId, worker, order, outputHash) => new TransactionInstruction({
  programId,
  keys: [signer(worker, false), rw(order), ro(SYSVAR_CLOCK_PUBKEY)],
  data: Buffer.concat([Buffer.from([DISC.DELIVER]), Buffer.from(outputHash)]),
});

export const challenge = (programId, challenger, order, claimedHash) => new TransactionInstruction({
  programId,
  keys: [signer(challenger, true), rw(order), ro(SystemProgram.programId), ro(SYSVAR_CLOCK_PUBKEY)],
  data: Buffer.concat([Buffer.from([DISC.CHALLENGE]), Buffer.from(claimedHash)]),
});

export const resolve = (programId, arbiter, order, payer, worker, challenger, verdict) => new TransactionInstruction({
  programId,
  keys: [signer(arbiter, false), rw(order), rw(payer), rw(worker), rw(challenger), ro(SYSVAR_CLOCK_PUBKEY)],
  data: Buffer.from([DISC.RESOLVE, verdict]),
});

export const settle = (programId, order, payer, worker) => new TransactionInstruction({
  programId,
  keys: [rw(order), rw(payer), rw(worker), ro(SYSVAR_CLOCK_PUBKEY)],
  data: Buffer.from([DISC.SETTLE]),
});

export const cancelExpired = (programId, order, payer, worker, challenger) => new TransactionInstruction({
  programId,
  keys: [rw(order), rw(payer), ro(SYSVAR_CLOCK_PUBKEY), ...(worker ? [rw(worker)] : []), ...(challenger ? [rw(challenger)] : [])],
  data: Buffer.from([DISC.CANCEL]),
});

/** §3.2: lee los campos que el gate necesita mirar. */
export function unpackOrder(buf) {
  if (buf.length !== 304) throw new Error(`la cuenta son 304 bytes, no ${buf.length}`);
  return {
    version: buf[0],
    state: buf[1],
    proofMode: buf[3],
    worker: new PublicKey(buf.subarray(88, 120)),
    arbiter: new PublicKey(buf.subarray(120, 152)),
    challenger: new PublicKey(buf.subarray(152, 184)),
    outputHash: Buffer.from(buf.subarray(184, 216)).toString("hex"),
    claimedOutputHash: Buffer.from(buf.subarray(216, 248)).toString("hex"),
    rentLamports: buf.readBigUInt64LE(264),
    challengeDeadline: buf.readBigInt64LE(296),
  };
}
