"""Simulador — SPEC-RUNNER.md §2.3 a §2.6.

Port del simulador de `memebot/backtest.mjs`. Se conservan sus rarezas a propósito (están
marcadas): el objetivo de esta subfase es reproducir el trabajo real, no mejorarlo. Cualquier
cambio de semántica acá cambia el `output_hash` de todos los pedidos ya liquidados.

Aritmética: solo `+ - * /` y comparaciones sobre binario64 (§1). Ninguna función
trascendental.
"""

LAMPORTS_PER_SOL = 1000000000


class Ctx:
    """Costos y portfolio, ya convertidos al modelo numérico de §1."""

    __slots__ = ("slippage", "fee_sol", "notional_sol", "max_open")

    def __init__(self, slippage_bps: int, fee_lamports: int, notional_lamports: int, max_open: int):
        # bps -> porcentaje -> fraccion, dos divisiones, igual que el backtester de
        # referencia (que hace `Number(pct) / 100` una sola vez sobre un porcentaje).
        self.slippage = (slippage_bps / 100) / 100
        self.fee_sol = fee_lamports / LAMPORTS_PER_SOL
        self.notional_sol = notional_lamports / LAMPORTS_PER_SOL
        self.max_open = max_open


class Exit:
    """Política de salida de una combinación concreta del barrido."""

    __slots__ = (
        "tp_mult",
        "tp_sell_pct",
        "trail_pct",
        "trail_arm_pct",
        "trail_sell_pct",
        "hard_stop_pct",
        "time_stop_min",
        "trail_always",
    )

    def __init__(self, combo: dict, trail_always: bool):
        self.tp_mult = float(combo["tp_mult"])
        self.tp_sell_pct = combo["tp_sell_bps"] / 100
        self.trail_pct = combo["trail_bps"] / 100
        self.trail_arm_pct = combo["trail_arm_bps"] / 100
        self.trail_sell_pct = combo["trail_sell_bps"] / 100
        self.hard_stop_pct = combo["hard_stop_bps"] / 100
        self.time_stop_min = combo["time_stop_min"]
        self.trail_always = trail_always


class Trade:
    __slots__ = ("mint", "entry_t", "exit_t", "pnl", "reason")

    def __init__(self, mint: str, entry_t: int, exit_t: int, pnl: float, reason: str):
        self.mint = mint
        self.entry_t = entry_t
        self.exit_t = exit_t
        self.pnl = pnl
        self.reason = reason


def _positive(value) -> bool:
    """Réplica de `x > 0` de JS: un campo ausente da False sin explotar."""
    return isinstance(value, (int, float)) and not isinstance(value, bool) and value > 0


# ------------------------------------------------------------------ salida compartida


def sim_exit(series, start_idx, entry_t, entry_p, ex: Exit, ctx: Ctx):
    """§2.4. Devuelve (pnl_sol, exit_t, motivo)."""
    slippage = ctx.slippage
    fee = ctx.fee_sol
    monto = ctx.notional_sol

    eff_entry = entry_p * (1 + slippage)
    state = {"remaining": 100.0, "pnl": -fee}
    tp1_done = False
    trail_sold = False
    peak = entry_p
    last_p, last_t = entry_p, entry_t

    def sell(p, pct):
        mult_eff = (p * (1 - slippage)) / eff_entry
        state["pnl"] += monto * (state["remaining"] / 100) * (pct / 100) * (mult_eff - 1) - fee
        state["remaining"] *= 1 - pct / 100

    for i in range(start_idx, len(series)):
        rec = series[i]
        p = rec.get("p")
        if not _positive(p):
            continue
        t = rec.get("t")
        last_p, last_t = p, t
        if p > peak:
            peak = p
        mult = p / entry_p
        age_min = (t - entry_t) / 60000
        off_peak = (1 - p / peak) * 100

        armed = ex.trail_arm_pct > 0 and peak >= entry_p * (1 + ex.trail_arm_pct / 100)
        if mult <= 1 - ex.hard_stop_pct / 100:
            sell(p, 100)
            return state["pnl"], t, "stop"
        if age_min >= ex.time_stop_min:
            sell(p, 100)
            return state["pnl"], t, "tiempo"
        if not tp1_done and mult >= ex.tp_mult:
            sell(p, ex.tp_sell_pct)
            tp1_done = True
            if state["remaining"] <= 0.5:
                return state["pnl"], t, "tp"
        elif (tp1_done or ex.trail_always or armed) and off_peak >= ex.trail_pct:
            # El primer disparo previo al TP puede asegurar una parte y rearmar el pico.
            parcial = (
                not trail_sold
                and not tp1_done
                and not ex.trail_always
                and ex.trail_sell_pct < 100
            )
            if parcial:
                sell(p, ex.trail_sell_pct)
                trail_sold = True
                peak = p
            else:
                sell(p, 100)
                return state["pnl"], t, "trailing"

    sell(last_p, 100)
    return state["pnl"], last_t, "finDatos"


# ----------------------------------------------------------------------- estrategias


def _entry_survivor(tok, en: dict):
    create_t = tok.create.get("t") if tok.create is not None else tok.ticks[0].get("t")
    buys = 0
    sells = 0
    buyers = set()
    first_mcap = (tok.create.get("mc") or 0) if tok.create is not None else 0
    last_mcap = first_mcap

    min_age = en["min_age_min"]
    max_age = en["max_age_min"]
    min_buyers = en["min_buyers"]
    min_mcap = float(en["min_mcap"])
    min_growth = float(en["min_growth"])
    min_ratio = float(en["min_ratio"])

    for i, tick in enumerate(tok.ticks):
        if tick.get("y") == "b":
            buys += 1
            w = tick.get("w")
            if w:
                buyers.add(w)
        else:
            sells += 1
        mc = tick.get("mc")
        if _positive(mc):
            if not first_mcap:
                first_mcap = mc
            last_mcap = mc
        age_min = (tick.get("t") - create_t) / 60000
        if age_min < min_age:
            continue
        if age_min > max_age:
            return None
        growth = last_mcap / first_mcap if first_mcap > 0 else 0
        ratio = buys / sells if sells > 0 else buys
        if (
            len(buyers) >= min_buyers
            and last_mcap >= min_mcap
            and growth >= min_growth
            and ratio >= min_ratio
        ):
            return i + 1, tick.get("t"), tick.get("p")
    return None


def run_survivor(tokens, en: dict, ex: Exit, ctx: Ctx) -> list[Trade]:
    trades = []
    for tok in tokens:
        if len(tok.ticks) < 3:
            continue
        entry = _entry_survivor(tok, en)
        if entry is None or not _positive(entry[2]):
            continue
        idx, entry_t, entry_p = entry
        pnl, exit_t, reason = sim_exit(tok.ticks, idx, entry_t, entry_p, ex, ctx)
        trades.append(Trade(tok.mint, entry_t, exit_t, pnl, reason))
    return trades


def run_sniper(tokens, en: dict, ex: Exit, ctx: Ctx) -> list[Trade]:
    """El sniper tiene su propio bucle de salida en el original (muerto / panico)."""
    slippage = ctx.slippage
    fee = ctx.fee_sol
    monto = ctx.notional_sol
    min_dev = en["min_dev_lamports"] / LAMPORTS_PER_SOL
    max_dev = en["max_dev_lamports"] / LAMPORTS_PER_SOL
    stall_ms = en["stall_s"] * 1000
    panic_sol = en["panic_lamports"] / LAMPORTS_PER_SOL

    trades = []
    for tok in tokens:
        c = tok.create
        if c is None or not _positive(c.get("p")) or len(tok.ticks) < 2:
            continue
        sol = c.get("sol")
        # Rareza conservada: en el original, `c.sol` ausente hace falsas las dos
        # comparaciones y el token NO se filtra. Se replica.
        if isinstance(sol, (int, float)) and not isinstance(sol, bool):
            if sol < min_dev or sol > max_dev:
                continue

        entry_p = c.get("p")
        entry_t = c.get("t")
        eff_entry = entry_p * (1 + slippage)
        state = {"remaining": 100.0, "pnl": -fee}
        peak = entry_p
        last_p, last_t = entry_p, entry_t
        done = None

        def sell(p, pct, _s=state):
            mult_eff = (p * (1 - slippage)) / eff_entry
            _s["pnl"] += monto * (_s["remaining"] / 100) * (pct / 100) * (mult_eff - 1) - fee
            _s["remaining"] *= 1 - pct / 100

        for tick in tok.ticks:
            tp = tick.get("p")
            if not _positive(tp):
                continue
            tt = tick.get("t")
            if tt - last_t > stall_ms:
                sell(last_p, 100)
                done = (last_t + stall_ms, "muerto")
                break
            last_p, last_t = tp, tt
            if tp > peak:
                peak = tp
            tsol = tick.get("sol")
            panic_size = (
                isinstance(tsol, (int, float))
                and not isinstance(tsol, bool)
                and tsol >= panic_sol
            )
            if tick.get("y") == "s" and (tick.get("w") == c.get("w") or panic_size):
                sell(tp, 100)
                done = (tt, "panico")
                break
            mult = tp / entry_p
            age_min = (tt - entry_t) / 60000
            off_peak = (1 - tp / peak) * 100
            if mult <= 1 - ex.hard_stop_pct / 100:
                sell(tp, 100)
                done = (tt, "stop")
                break
            if age_min >= ex.time_stop_min:
                sell(tp, 100)
                done = (tt, "tiempo")
                break
            if mult >= ex.tp_mult:
                sell(tp, 100)
                done = (tt, "tp")
                break
            if off_peak >= ex.trail_pct:
                sell(tp, 100)
                done = (tt, "trailing")
                break
        if done is None:
            sell(last_p, 100)
            done = (last_t, "finDatos")
        trades.append(Trade(tok.mint, entry_t, done[0], state["pnl"], done[1]))
    return trades


def run_graduacion(tokens, en: dict, ex: Exit, ctx: Ctx) -> list[Trade]:
    dip_pct = en["dip_bps"] / 100
    abort_pct = en["abort_bps"] / 100
    timeout_min = en["timeout_min"]

    trades = []
    for tok in tokens:
        if not tok.migrate_at or len(tok.grad) < 2:
            continue
        entry = None
        if dip_pct == 0:
            first = tok.grad[0]
            if _positive(first.get("p")):
                entry = (1, first.get("t"), first.get("p"))
        else:
            high = None
            t_start = tok.grad[0].get("t")
            for i, tk in enumerate(tok.grad):
                p = tk.get("p")
                if not _positive(p):
                    continue
                if (tk.get("t") - t_start) / 60000 > timeout_min:
                    break
                if high is None or p > high:
                    high = p
                    continue
                drop = (1 - p / high) * 100
                if drop >= abort_pct:
                    break
                if drop >= dip_pct:
                    entry = (i + 1, tk.get("t"), p)
                    break
        if entry is None:
            continue
        idx, entry_t, entry_p = entry
        pnl, exit_t, reason = sim_exit(tok.grad, idx, entry_t, entry_p, ex, ctx)
        trades.append(Trade(tok.mint, entry_t, exit_t, pnl, reason))
    return trades


RUNNERS = {
    "survivor.v1": run_survivor,
    "sniper.v1": run_sniper,
    "graduacion.v1": run_graduacion,
}


# ------------------------------------------------------------ tope y cuantizacion


def apply_cap(trades: list[Trade], max_open: int) -> list[Trade]:
    """§2.5. Orden total `(entry_t, mint)`: sin el mint, dos entradas en el mismo
    milisegundo se resolverian por orden de iteracion y eso no es reproducible."""
    ordered = sorted(trades, key=lambda tr: (tr.entry_t, tr.mint))
    open_exits: list[int] = []
    out: list[Trade] = []
    for tr in ordered:
        while open_exits and open_exits[0] <= tr.entry_t:
            open_exits.pop(0)
        if len(open_exits) >= max_open:
            continue
        open_exits.append(tr.exit_t)
        open_exits.sort()
        out.append(tr)
    return out


def to_lamports(pnl_sol: float) -> int:
    """§2.6. Truncamiento hacia cero; `int()` de Python trunca hacia cero."""
    return int(pnl_sol * LAMPORTS_PER_SOL)


def metrics(trades: list[Trade]) -> dict:
    """Todas las metricas son enteras y aditivas (SPEC §6.8)."""
    net = 0
    wins = 0
    gross_win = 0
    gross_loss = 0
    unclosed = 0
    for tr in trades:
        lam = to_lamports(tr.pnl)
        net += lam
        if lam > 0:
            wins += 1
            gross_win += lam
        else:
            gross_loss += lam
        if tr.reason == "finDatos":
            unclosed += 1
    return {
        "gross_loss_lamports": gross_loss,
        "gross_win_lamports": gross_win,
        "n_trades": len(trades),
        "net_lamports": net,
        "unclosed": unclosed,
        "wins": wins,
    }
