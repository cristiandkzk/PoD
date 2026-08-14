//! Los siete handlers — SPEC-PROGRAM.md §4.

use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::program::{invoke, invoke_signed};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::sysvar::{self, clock::Clock, rent::Rent, SysvarSerialize};
use solana_system_interface::instruction as system_instruction;
use solana_system_interface::program as system_program;

use crate::error::PodError;
use crate::instruction::{CreateArgs, PodInstruction};
use crate::state::*;

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> Result<(), ProgramError> {
    match PodInstruction::decode(data)? {
        PodInstruction::CreateOrder(args) => create_order(program_id, accounts, &args),
        PodInstruction::AcceptOrder => accept_order(program_id, accounts),
        PodInstruction::CancelExpired => cancel_expired(program_id, accounts),
        PodInstruction::Deliver { output_hash } => deliver(program_id, accounts, output_hash),
        PodInstruction::Challenge { claimed_output_hash } => {
            challenge(program_id, accounts, claimed_output_hash)
        }
        PodInstruction::Resolve { verdict } => resolve(program_id, accounts, verdict),
        PodInstruction::Settle => settle(program_id, accounts),
    }
}

/// Los sysvars entran como cuentas, no por `Clock::get()`. Ver SPEC-PROGRAM.md §4.0.
/// El chequeo explicito es para que un sysvar equivocado de un error del programa (§6) y no
/// el `InvalidArgument` generico de `from_account_info`.
fn check_sysvars(rent: Option<&AccountInfo>, clock: &AccountInfo) -> Result<(), ProgramError> {
    if let Some(r) = rent {
        if r.key != &sysvar::rent::ID {
            return Err(PodError::BadAccounts.into());
        }
    }
    if clock.key != &sysvar::clock::ID {
        return Err(PodError::BadAccounts.into());
    }
    Ok(())
}

fn now_from(clock: &AccountInfo) -> Result<i64, ProgramError> {
    Ok(Clock::from_account_info(clock)?.unix_timestamp)
}

/// Lee una orden y comprueba que la cuenta sea de verdad el PDA de sus propios campos.
/// Sin ese segundo chequeo, "el duenio es el programa" seria toda la defensa.
fn load(program_id: &Pubkey, order: &AccountInfo) -> Result<Order, ProgramError> {
    if order.owner != program_id {
        return Err(PodError::BadPda.into());
    }
    let ord = {
        let data = order.try_borrow_data()?;
        Order::unpack(&data[..])?
    };
    let nonce_le = ord.nonce.to_le_bytes();
    let payer_bytes = ord.payer.to_bytes();
    let expected = Pubkey::create_program_address(
        &[SEED_PREFIX, &payer_bytes, &ord.spec_hash, &nonce_le, &[ord.bump]],
        program_id,
    )
    .map_err(|_| PodError::BadPda)?;
    if &expected != order.key {
        return Err(PodError::BadPda.into());
    }
    Ok(ord)
}

fn store(order: &AccountInfo, ord: &Order) -> Result<(), ProgramError> {
    let mut data = order.try_borrow_mut_data()?;
    ord.pack(&mut data[..])?;
    Ok(())
}

fn move_lamports(from: &AccountInfo, to: &AccountInfo, amount: u64) -> Result<(), ProgramError> {
    let available = from.lamports();
    if available < amount {
        return Err(PodError::Overflow.into());
    }
    **from.try_borrow_mut_lamports()? = available - amount;
    let had = to.lamports();
    **to.try_borrow_mut_lamports()? = had.checked_add(amount).ok_or(PodError::Overflow)?;
    Ok(())
}

/// Cierre — §4.4. El chequeo de balance cero es la invariante I2/I3 comprobada **dentro**
/// del programa, no solo en los tests: si una rama futura se olvida de repartir algo, la
/// transaccion falla en vez de dejar lamports encerrados para siempre.
fn close(order: &AccountInfo) -> Result<(), ProgramError> {
    if order.lamports() != 0 {
        return Err(PodError::Overflow.into());
    }
    {
        let mut data = order.try_borrow_mut_data()?;
        for b in data.iter_mut() {
            *b = 0;
        }
    }
    order.resize(0)?;
    order.assign(&system_program::ID);
    Ok(())
}

fn require_writable(accounts: &[&AccountInfo]) -> Result<(), ProgramError> {
    for a in accounts {
        if !a.is_writable {
            return Err(PodError::BadAccounts.into());
        }
    }
    Ok(())
}

fn create_order(program_id: &Pubkey, accounts: &[AccountInfo], a: &CreateArgs) -> Result<(), ProgramError> {
    if accounts.len() != 5 {
        return Err(PodError::BadAccounts.into());
    }
    let it = &mut accounts.iter();
    let payer = next_account_info(it)?;
    let order = next_account_info(it)?;
    let system = next_account_info(it)?;
    let rent_ai = next_account_info(it)?;
    let clock_ai = next_account_info(it)?;
    check_sysvars(Some(rent_ai), clock_ai)?;

    if !payer.is_signer {
        return Err(PodError::NotSigner.into());
    }
    require_writable(&[payer, order])?;
    if system.key != &system_program::ID {
        return Err(PodError::BadAccounts.into());
    }
    if a.reward_lamports == 0 {
        return Err(PodError::ZeroReward.into());
    }
    if a.bond_lamports == 0 {
        return Err(PodError::ZeroBond.into());
    }
    if a.challenge_deposit_lamports == 0 {
        return Err(PodError::ZeroDeposit.into());
    }
    // Un arbitro en ceros haria irresoluble toda disputa de esta orden.
    if a.arbiter == Pubkey::default() {
        return Err(PodError::WrongArbiter.into());
    }
    for w in [a.accept_window_secs, a.deliver_window_secs, a.challenge_window_secs] {
        if !(WINDOW_MIN..=WINDOW_MAX).contains(&w) {
            return Err(PodError::BadWindow.into());
        }
    }

    let nonce_le = a.nonce.to_le_bytes();
    let payer_bytes = payer.key.to_bytes();
    let (expected, bump) = Pubkey::find_program_address(
        &Order::seeds(&payer_bytes, &a.spec_hash, &nonce_le),
        program_id,
    );
    if order.key != &expected {
        return Err(PodError::BadPda.into());
    }
    if !order.data_is_empty() || order.owner != &system_program::ID {
        return Err(PodError::AlreadyExists.into());
    }

    let rent_lamports = Rent::from_account_info(rent_ai)?.minimum_balance(ORDER_LEN);
    let needed = rent_lamports
        .checked_add(a.reward_lamports)
        .ok_or(PodError::Overflow)?;

    // §4.1: transfer/allocate/assign en vez de create_account. La direccion del PDA es
    // publica y calculable de antemano; con create_account, mandarle un lamport alcanzaria
    // para bloquear la creacion de la orden.
    let have = order.lamports();
    if have < needed {
        invoke(
            &system_instruction::transfer(payer.key, order.key, needed - have),
            &[payer.clone(), order.clone(), system.clone()],
        )?;
    }
    let signer: &[&[u8]] = &[SEED_PREFIX, &payer_bytes, &a.spec_hash, &nonce_le, &[bump]];
    invoke_signed(
        &system_instruction::allocate(order.key, ORDER_LEN as u64),
        &[order.clone(), system.clone()],
        &[signer],
    )?;
    invoke_signed(
        &system_instruction::assign(order.key, program_id),
        &[order.clone(), system.clone()],
        &[signer],
    )?;

    let now = now_from(clock_ai)?;
    let ord = Order {
        version: VERSION,
        state: STATE_CREADA,
        bump,
        proof_mode: PROOF_OPTIMISTIC,
        deliver_window_secs: a.deliver_window_secs,
        challenge_window_secs: a.challenge_window_secs,
        nonce: a.nonce,
        spec_hash: a.spec_hash,
        payer: *payer.key,
        worker: Pubkey::default(),
        arbiter: a.arbiter,
        challenger: Pubkey::default(),
        output_hash: ZERO_HASH,
        claimed_output_hash: ZERO_HASH,
        reward_lamports: a.reward_lamports,
        bond_lamports: a.bond_lamports,
        rent_lamports,
        challenge_deposit_lamports: a.challenge_deposit_lamports,
        accept_deadline: now
            .checked_add(i64::from(a.accept_window_secs))
            .ok_or(PodError::Overflow)?,
        deliver_deadline: 0,
        challenge_deadline: 0,
    };
    store(order, &ord)
}

fn accept_order(program_id: &Pubkey, accounts: &[AccountInfo]) -> Result<(), ProgramError> {
    if accounts.len() != 4 {
        return Err(PodError::BadAccounts.into());
    }
    let it = &mut accounts.iter();
    let worker = next_account_info(it)?;
    let order = next_account_info(it)?;
    let system = next_account_info(it)?;
    let clock_ai = next_account_info(it)?;
    check_sysvars(None, clock_ai)?;

    if !worker.is_signer {
        return Err(PodError::NotSigner.into());
    }
    require_writable(&[worker, order])?;
    if system.key != &system_program::ID {
        return Err(PodError::BadAccounts.into());
    }

    let mut ord = load(program_id, order)?;
    if ord.state != STATE_CREADA {
        return Err(PodError::BadState.into());
    }
    let now = now_from(clock_ai)?;
    if now > ord.accept_deadline {
        return Err(PodError::Expired.into());
    }

    // El bond no es un campo declarado: es este movimiento. Si falla, falla la instruccion,
    // y con ella la transaccion entera — de ahi que no exista ACEPTADA sin bond (I4).
    invoke(
        &system_instruction::transfer(worker.key, order.key, ord.bond_lamports),
        &[worker.clone(), order.clone(), system.clone()],
    )?;

    ord.state = STATE_ACEPTADA;
    ord.worker = *worker.key;
    ord.deliver_deadline = now
        .checked_add(i64::from(ord.deliver_window_secs))
        .ok_or(PodError::Overflow)?;
    store(order, &ord)
}

fn deliver(program_id: &Pubkey, accounts: &[AccountInfo], output_hash: [u8; 32]) -> Result<(), ProgramError> {
    if accounts.len() != 3 {
        return Err(PodError::BadAccounts.into());
    }
    let it = &mut accounts.iter();
    let worker = next_account_info(it)?;
    let order = next_account_info(it)?;
    let clock_ai = next_account_info(it)?;
    check_sysvars(None, clock_ai)?;

    if !worker.is_signer {
        return Err(PodError::NotSigner.into());
    }
    require_writable(&[order])?;
    // Un hash en ceros es lo mismo que ve un lector en un campo sin inicializar: aceptarlo
    // haria indistinguible "no entrego" de "entrego ceros".
    if output_hash == ZERO_HASH {
        return Err(PodError::ZeroHash.into());
    }

    let mut ord = load(program_id, order)?;
    if ord.state != STATE_ACEPTADA {
        return Err(PodError::BadState.into());
    }
    if worker.key != &ord.worker {
        return Err(PodError::WrongWorker.into());
    }
    let now = now_from(clock_ai)?;
    if now > ord.deliver_deadline {
        return Err(PodError::Expired.into());
    }

    ord.state = STATE_ENTREGADA;
    ord.output_hash = output_hash;
    ord.challenge_deadline = now
        .checked_add(i64::from(ord.challenge_window_secs))
        .ok_or(PodError::Overflow)?;
    store(order, &ord)
}

fn challenge(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    claimed: [u8; 32],
) -> Result<(), ProgramError> {
    if accounts.len() != 4 {
        return Err(PodError::BadAccounts.into());
    }
    let it = &mut accounts.iter();
    let challenger = next_account_info(it)?;
    let order = next_account_info(it)?;
    let system = next_account_info(it)?;
    let clock_ai = next_account_info(it)?;
    check_sysvars(None, clock_ai)?;

    if !challenger.is_signer {
        return Err(PodError::NotSigner.into());
    }
    require_writable(&[challenger, order])?;
    if system.key != &system_program::ID {
        return Err(PodError::BadAccounts.into());
    }
    if claimed == ZERO_HASH {
        return Err(PodError::ZeroHash.into());
    }

    let mut ord = load(program_id, order)?;
    if ord.state != STATE_ENTREGADA {
        return Err(PodError::BadState.into());
    }
    let now = now_from(clock_ai)?;
    if now > ord.challenge_deadline {
        return Err(PodError::Expired.into());
    }
    // Un challenge que afirma el mismo hash no es un challenge: no hay nada que arbitrar.
    if claimed == ord.output_hash {
        return Err(PodError::SameHash.into());
    }

    invoke(
        &system_instruction::transfer(challenger.key, order.key, ord.challenge_deposit_lamports),
        &[challenger.clone(), order.clone(), system.clone()],
    )?;

    ord.state = STATE_DISPUTADA;
    ord.challenger = *challenger.key;
    ord.claimed_output_hash = claimed;
    // A partir de aca el plazo es el del arbitro. §4.6.
    ord.challenge_deadline = now
        .checked_add(i64::from(ord.challenge_window_secs))
        .ok_or(PodError::Overflow)?;
    store(order, &ord)
}

/// §4.7. Lo firma el arbitro declarado en la orden. Las cuatro cuentas de destino van
/// siempre, con o sin el veredicto que las favorezca: quien liquida no elige a quien paga.
fn resolve(program_id: &Pubkey, accounts: &[AccountInfo], verdict: u8) -> Result<(), ProgramError> {
    if accounts.len() != 6 {
        return Err(PodError::BadAccounts.into());
    }
    let it = &mut accounts.iter();
    let arbiter = next_account_info(it)?;
    let order = next_account_info(it)?;
    let payer = next_account_info(it)?;
    let worker = next_account_info(it)?;
    let challenger = next_account_info(it)?;
    let clock_ai = next_account_info(it)?;
    check_sysvars(None, clock_ai)?;

    if !arbiter.is_signer {
        return Err(PodError::NotSigner.into());
    }
    require_writable(&[order, payer, worker, challenger])?;

    let ord = load(program_id, order)?;
    if ord.state != STATE_DISPUTADA {
        return Err(PodError::BadState.into());
    }
    if arbiter.key != &ord.arbiter {
        return Err(PodError::WrongArbiter.into());
    }
    if payer.key != &ord.payer {
        return Err(PodError::WrongPayer.into());
    }
    if worker.key != &ord.worker {
        return Err(PodError::WrongWorker.into());
    }
    if challenger.key != &ord.challenger {
        return Err(PodError::WrongChallenger.into());
    }
    // Pasado su plazo, el arbitro ya no decide: la salida es `cancel_expired`, que desarma
    // la disputa sin ganador. Un arbitro que llega tarde no puede llegar igual.
    if now_from(clock_ai)? > ord.challenge_deadline {
        return Err(PodError::Expired.into());
    }

    match verdict {
        VERDICT_FRAUDE => {
            // El challenger tenia razon: cobra el bond del worker y recupera su deposito.
            move_lamports(order, challenger, ord.bond_lamports)?;
            move_lamports(order, challenger, ord.challenge_deposit_lamports)?;
            move_lamports(order, payer, order.lamports())?;
        }
        VERDICT_INFUNDADO => {
            // El worker era honesto: cobra la recompensa, recupera el bond, y se queda con
            // el deposito del challenger como compensacion por la demora.
            move_lamports(order, worker, ord.reward_lamports)?;
            move_lamports(order, worker, ord.bond_lamports)?;
            move_lamports(order, worker, ord.challenge_deposit_lamports)?;
            move_lamports(order, payer, order.lamports())?;
        }
        _ => return Err(PodError::BadVerdict.into()),
    }
    close(order)
}

/// §4.8. Sin firmante: la liquidacion no depende de que nadie este vivo.
fn settle(program_id: &Pubkey, accounts: &[AccountInfo]) -> Result<(), ProgramError> {
    if accounts.len() != 4 {
        return Err(PodError::BadAccounts.into());
    }
    let order = &accounts[0];
    let payer = &accounts[1];
    let worker = &accounts[2];
    check_sysvars(None, &accounts[3])?;
    require_writable(&[order, payer, worker])?;

    let ord = load(program_id, order)?;
    if ord.state != STATE_ENTREGADA {
        return Err(PodError::BadState.into());
    }
    if payer.key != &ord.payer {
        return Err(PodError::WrongPayer.into());
    }
    if worker.key != &ord.worker {
        return Err(PodError::WrongWorker.into());
    }
    if now_from(&accounts[3])? <= ord.challenge_deadline {
        return Err(PodError::NotExpired.into());
    }

    move_lamports(order, worker, ord.reward_lamports)?;
    move_lamports(order, worker, ord.bond_lamports)?;
    move_lamports(order, payer, order.lamports())?;
    close(order)
}

/// §4.3. Una sola instruccion para "un vencimiento desarma el estado", con una rama por
/// estado que puede vencer. ENTREGADA no esta: su salida es `settle`, no un reembolso.
fn cancel_expired(program_id: &Pubkey, accounts: &[AccountInfo]) -> Result<(), ProgramError> {
    if !(3..=5).contains(&accounts.len()) {
        return Err(PodError::BadAccounts.into());
    }
    let order = &accounts[0];
    let payer = &accounts[1];
    check_sysvars(None, &accounts[2])?;
    require_writable(&[order, payer])?;

    let ord = load(program_id, order)?;
    if payer.key != &ord.payer {
        return Err(PodError::WrongPayer.into());
    }
    let now = now_from(&accounts[2])?;

    match ord.state {
        STATE_CREADA => {
            if accounts.len() != 3 {
                return Err(PodError::BadAccounts.into());
            }
            if now <= ord.accept_deadline {
                return Err(PodError::NotExpired.into());
            }
            // El balance completo, no `reward + rent`: lo donado por terceros se va con el
            // resto en vez de quedar huerfano (I3).
            move_lamports(order, payer, order.lamports())?;
        }
        STATE_ACEPTADA => {
            if accounts.len() != 4 {
                return Err(PodError::BadAccounts.into());
            }
            let worker = &accounts[3];
            require_writable(&[worker])?;
            if worker.key != &ord.worker {
                return Err(PodError::WrongWorker.into());
            }
            if now <= ord.deliver_deadline {
                return Err(PodError::NotExpired.into());
            }
            move_lamports(order, worker, ord.bond_lamports)?;
            move_lamports(order, payer, order.lamports())?;
        }
        STATE_DISPUTADA => {
            // El arbitro no resolvio a tiempo. La disputa no se decide: se desarma. Cada
            // uno recupera lo suyo y nadie gana. Ver §7 — es la frontera de la opcion (a).
            if accounts.len() != 5 {
                return Err(PodError::BadAccounts.into());
            }
            let worker = &accounts[3];
            let challenger = &accounts[4];
            require_writable(&[worker, challenger])?;
            if worker.key != &ord.worker {
                return Err(PodError::WrongWorker.into());
            }
            if challenger.key != &ord.challenger {
                return Err(PodError::WrongChallenger.into());
            }
            if now <= ord.challenge_deadline {
                return Err(PodError::NotExpired.into());
            }
            move_lamports(order, worker, ord.bond_lamports)?;
            move_lamports(order, challenger, ord.challenge_deposit_lamports)?;
            move_lamports(order, payer, order.lamports())?;
        }
        _ => return Err(PodError::BadState.into()),
    }
    close(order)
}
