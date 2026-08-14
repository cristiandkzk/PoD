// Harness de fidelidad: corre el simulador ORIGINAL de memebot/backtest.mjs contra las
// mismas combinaciones que el runner en Python, y emite las mismas metricas.
//
//   node harness.mjs <combos.json> <dataset.jsonl>...
//
// Las funciones simExit / entradaSurvivor / simSniper / simGrad estan copiadas de
// backtest.mjs. Los unicos cambios son los que SPEC-RUNNER.md declara y estan marcados
// con "// [PoD]": el orden total de tokens y del tope de posiciones, y la cuantizacion.
// Si estas metricas no coinciden con las de Python, el port cambio la semantica del
// trabajo, que es peor que un bug: seria computar mal de forma reproducible.
import fs from 'node:fs';

const [combosPath, ...files] = process.argv.slice(2);
const cfg = JSON.parse(fs.readFileSync(combosPath, 'utf8'));

const SLIPPAGE = cfg.slippage_bps_per_side / 100 / 100;
const FEE = cfg.fee_lamports_per_tx / 1000000000;
const MONTO = cfg.notional_lamports / 1000000000;
const MAXPOS = cfg.max_open_positions;

// ---------- carga (misma agrupacion que backtest.mjs) ----------
const tokens = new Map();
function tok(m) {
  let t = tokens.get(m);
  if (!t) { t = { mint: m, create: null, ticks: [], migrateAt: null, grad: [] }; tokens.set(m, t); }
  return t;
}
for (const file of files.sort((a, b) => (a.split(/[\\/]/).pop() < b.split(/[\\/]/).pop() ? -1 : 1))) {
  for (const line of fs.readFileSync(file, 'utf8').split('\n')) {
    if (!line) continue;
    let r; try { r = JSON.parse(line); } catch { continue; }
    if (typeof r.m !== 'string') continue;
    if (r.k === 'c') tok(r.m).create = r;
    else if (r.k === 't') tok(r.m).ticks.push(r);
    else if (r.k === 'm') tok(r.m).migrateAt = r.t;
    else if (r.k === 'g') tok(r.m).grad.push(r);
  }
}
// [PoD] recorrido en orden ascendente de mint, no de insercion (SPEC-RUNNER 2.1.6)
const all = [...tokens.values()].sort((a, b) => (a.mint < b.mint ? -1 : a.mint > b.mint ? 1 : 0));
for (const t of all) {
  t.ticks.sort((a, b) => a.t - b.t);
  t.grad.sort((a, b) => a.t - b.t);
  const t0 = t.create?.t ?? t.ticks[0]?.t ?? t.migrateAt;
  t.day = t0 ? new Date(Math.floor(t0 / 1000) * 1000).toISOString().slice(0, 10) : null;
}

let pool = all;
if (cfg.split === 'last_day_holdout.v1') {
  const dias = [...new Set(all.map((t) => t.day).filter(Boolean))].sort();
  const last = dias[dias.length - 1];
  pool = cfg.set === 'test' ? all.filter((t) => t.day === last) : all.filter((t) => t.day !== last);
}

// ---------- simulador de salidas (verbatim de backtest.mjs) ----------
function simExit(series, startIdx, entryT, entryP, ex) {
  const effEntry = entryP * (1 + SLIPPAGE);
  let remaining = 100, tp1Done = false, trailSold = false, peak = entryP, pnl = -FEE;
  let lastP = entryP, lastT = entryT;

  const sell = (p, pct) => {
    const multEff = (p * (1 - SLIPPAGE)) / effEntry;
    pnl += MONTO * (remaining / 100) * (pct / 100) * (multEff - 1) - FEE;
    remaining *= 1 - pct / 100;
  };

  for (let i = startIdx; i < series.length; i++) {
    const { t, p } = series[i];
    if (!(p > 0)) continue;
    lastP = p; lastT = t;
    if (p > peak) peak = p;
    const mult = p / entryP;
    const ageMin = (t - entryT) / 60000;
    const offPeak = (1 - p / peak) * 100;

    const armed = ex.trailArmPct > 0 && peak >= entryP * (1 + ex.trailArmPct / 100);
    if (mult <= 1 - ex.hardStopPct / 100) { sell(p, 100); return { pnl, exitT: t, by: 'stop' }; }
    if (ageMin >= ex.timeStopMin) { sell(p, 100); return { pnl, exitT: t, by: 'tiempo' }; }
    if (!tp1Done && mult >= ex.tpMult) { sell(p, ex.tpSellPct); tp1Done = true; if (remaining <= 0.5) return { pnl, exitT: t, by: 'tp' }; }
    else if ((tp1Done || ex.trailAlways || armed) && offPeak >= ex.trailPct) {
      const parcial = !trailSold && !tp1Done && !ex.trailAlways && (ex.trailSellPct ?? 100) < 100;
      if (parcial) { sell(p, ex.trailSellPct); trailSold = true; peak = p; }
      else { sell(p, 100); return { pnl, exitT: t, by: 'trailing' }; }
    }
  }
  sell(lastP, 100);
  return { pnl, exitT: lastT, by: 'finDatos' };
}

function aplicarTope(trades) {
  // [PoD] orden total (entryT, mint) en lugar de solo entryT (SPEC-RUNNER 2.5)
  trades.sort((a, b) => (a.entryT - b.entryT) || (a.mint < b.mint ? -1 : a.mint > b.mint ? 1 : 0));
  const open = [];
  const out = [];
  for (const tr of trades) {
    while (open.length && open[0] <= tr.entryT) open.shift();
    if (open.length >= MAXPOS) continue;
    open.push(tr.exitT);
    open.sort((a, b) => a - b);
    out.push(tr);
  }
  return out;
}

// ---------- entradas (verbatim) ----------
function entradaSurvivor(token, en) {
  const createT = token.create?.t ?? token.ticks[0].t;
  let buys = 0, sells = 0;
  const buyers = new Set();
  let firstMcap = token.create?.mc || 0, lastMcap = firstMcap;
  for (let i = 0; i < token.ticks.length; i++) {
    const tick = token.ticks[i];
    if (tick.y === 'b') { buys++; if (tick.w) buyers.add(tick.w); } else sells++;
    if (tick.mc > 0) { if (!firstMcap) firstMcap = tick.mc; lastMcap = tick.mc; }
    const ageMin = (tick.t - createT) / 60000;
    if (ageMin < en.minAgeMin) continue;
    if (ageMin > en.maxAgeMin) return null;
    const growth = firstMcap > 0 ? lastMcap / firstMcap : 0;
    const ratio = sells > 0 ? buys / sells : buys;
    if (buyers.size >= en.minBuyers && lastMcap >= en.minMcap && growth >= en.minGrowth && ratio >= en.minRatio) {
      return { idx: i + 1, t: tick.t, p: tick.p };
    }
  }
  return null;
}

function simSurvivor(en, ex, toks) {
  const trades = [];
  for (const token of toks) {
    if (token.ticks.length < 3) continue;
    const entry = entradaSurvivor(token, en);
    if (!entry || !(entry.p > 0)) continue;
    const r = simExit(token.ticks, entry.idx, entry.t, entry.p, ex);
    trades.push({ mint: token.mint, entryT: entry.t, ...r });
  }
  return trades;
}

function simSniper(en, ex, toks) {
  const trades = [];
  for (const token of toks) {
    const c = token.create;
    if (!c || !(c.p > 0) || token.ticks.length < 2) continue;
    if (c.sol < en.minDev || c.sol > en.maxDev) continue;
    const effEntry = c.p * (1 + SLIPPAGE);
    let remaining = 100, peak = c.p, pnl = -FEE, lastP = c.p, lastT = c.t, done = null;
    const sell = (p, pct) => {
      pnl += MONTO * (remaining / 100) * (pct / 100) * ((p * (1 - SLIPPAGE)) / effEntry - 1) - FEE;
      remaining *= 1 - pct / 100;
    };
    for (const tick of token.ticks) {
      if (!(tick.p > 0)) continue;
      if (tick.t - lastT > en.stallSec * 1000) { sell(lastP, 100); done = { exitT: lastT + en.stallSec * 1000, by: 'muerto' }; break; }
      lastP = tick.p; lastT = tick.t;
      if (tick.p > peak) peak = tick.p;
      if (tick.y === 's' && (tick.w === c.w || tick.sol >= en.panicSol)) { sell(tick.p, 100); done = { exitT: tick.t, by: 'panico' }; break; }
      const mult = tick.p / c.p;
      const ageMin = (tick.t - c.t) / 60000;
      const offPeak = (1 - tick.p / peak) * 100;
      if (mult <= 1 - ex.hardStopPct / 100) { sell(tick.p, 100); done = { exitT: tick.t, by: 'stop' }; break; }
      if (ageMin >= ex.timeStopMin) { sell(tick.p, 100); done = { exitT: tick.t, by: 'tiempo' }; break; }
      if (mult >= ex.tpMult) { sell(tick.p, 100); done = { exitT: tick.t, by: 'tp' }; break; }
      if (offPeak >= ex.trailPct) { sell(tick.p, 100); done = { exitT: tick.t, by: 'trailing' }; break; }
    }
    if (!done) { sell(lastP, 100); done = { exitT: lastT, by: 'finDatos' }; }
    trades.push({ mint: token.mint, entryT: c.t, pnl, ...done });
  }
  return trades;
}

function simGrad(dip, ex, toks) {
  const trades = [];
  for (const token of toks) {
    if (!token.migrateAt || token.grad.length < 2) continue;
    let entry = null;
    if (!dip.dipPct) {
      const t0 = token.grad[0];
      if (t0.p > 0) entry = { idx: 1, t: t0.t, p: t0.p };
    } else {
      let high = null;
      for (let i = 0; i < token.grad.length; i++) {
        const tk = token.grad[i];
        if (!(tk.p > 0)) continue;
        if ((tk.t - token.grad[0].t) / 60000 > dip.timeoutMin) break;
        if (high === null || tk.p > high) { high = tk.p; continue; }
        const drop = (1 - tk.p / high) * 100;
        if (drop >= dip.abortPct) break;
        if (drop >= dip.dipPct) { entry = { idx: i + 1, t: tk.t, p: tk.p }; break; }
      }
    }
    if (!entry) continue;
    const r = simExit(token.grad, entry.idx, entry.t, entry.p, ex);
    trades.push({ mint: token.mint, entryT: entry.t, ...r });
  }
  return trades;
}

// ---------- metricas ----------
function metrics(trades) {
  const t = aplicarTope(trades);
  let net = 0, wins = 0, gw = 0, gl = 0, unclosed = 0;
  for (const tr of t) {
    const lam = Math.trunc(tr.pnl * 1000000000); // [PoD] cuantizacion por operacion
    net += lam;
    if (lam > 0) { wins++; gw += lam; } else { gl += lam; }
    if (tr.by === 'finDatos') unclosed++;
  }
  return { gross_loss_lamports: gl, gross_win_lamports: gw, n_trades: t.length, net_lamports: net, unclosed, wins };
}

// ---------- barrido ----------
const out = cfg.combos.map((combo) => {
  const e = combo.exit_policy;
  const ex = {
    tpMult: Number(e.tp_mult),
    tpSellPct: e.tp_sell_bps / 100,
    trailPct: e.trail_bps / 100,
    trailArmPct: e.trail_arm_bps / 100,
    trailSellPct: e.trail_sell_bps / 100,
    hardStopPct: e.hard_stop_bps / 100,
    timeStopMin: e.time_stop_min,
    trailAlways: cfg.trail_always,
  };
  const s = combo.strategy;
  if (cfg.kind === 'survivor.v1') {
    return metrics(simSurvivor({
      minAgeMin: s.min_age_min, maxAgeMin: s.max_age_min, minBuyers: s.min_buyers,
      minMcap: Number(s.min_mcap), minGrowth: Number(s.min_growth), minRatio: Number(s.min_ratio),
    }, ex, pool));
  }
  if (cfg.kind === 'sniper.v1') {
    return metrics(simSniper({
      minDev: s.min_dev_lamports / 1000000000, maxDev: s.max_dev_lamports / 1000000000,
      stallSec: s.stall_s, panicSol: s.panic_lamports / 1000000000,
    }, ex, pool));
  }
  return metrics(simGrad({
    dipPct: s.dip_bps / 100, timeoutMin: s.timeout_min, abortPct: s.abort_bps / 100,
  }, ex, pool));
});

process.stdout.write(JSON.stringify(out));
