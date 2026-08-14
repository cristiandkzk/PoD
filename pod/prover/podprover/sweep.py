"""Barrido cartesiano, ranking y armado del resultado — SPEC-RUNNER.md §3."""

from . import sim
from .dataset import split_last_day


def swept(order: dict) -> list[tuple[str, str, str, list]]:
    """Parámetros barridos, en orden de ruta canónica (§3.1).

    Devuelve (ruta, grupo, clave, valores). El orden por ruta hace que `inputs.exit_policy.…`
    venga antes que `inputs.strategy.params.…` sin que haya que decidirlo a mano.
    """
    inputs = order["inputs"]
    out = []
    for key, value in inputs["exit_policy"].items():
        if isinstance(value, list):
            out.append((f"inputs.exit_policy.{key}", "exit_policy", key, value))
    for key, value in inputs["strategy"]["params"].items():
        if isinstance(value, list):
            out.append((f"inputs.strategy.params.{key}", "strategy", key, value))
    out.sort(key=lambda entry: entry[0].encode("utf-8"))
    return out


def total_combos(axes) -> int:
    total = 1
    for _, _, _, values in axes:
        total *= len(values)
    return total


def combo_at(axes, index: int) -> tuple[dict, dict]:
    """Cuentakilómetros: el último eje es el que varía más rápido (§3.1)."""
    exit_pick: dict = {}
    strat_pick: dict = {}
    rem = index
    for _, group, key, values in reversed(axes):
        value = values[rem % len(values)]
        rem //= len(values)
        (exit_pick if group == "exit_policy" else strat_pick)[key] = value
    return exit_pick, strat_pick


def _fixed(block: dict) -> dict:
    """Campos del bloque que no son rejilla (hoy solo `trail_always`)."""
    return {k: v for k, v in block.items() if not isinstance(v, list)}


def evaluate(order: dict, tokens, axes, index: int, ctx: sim.Ctx) -> tuple[dict, dict, dict]:
    """Corre una combinación. Devuelve (metricas, exit_pick, strat_pick)."""
    exit_pick, strat_pick = combo_at(axes, index)
    fixed_exit = _fixed(order["inputs"]["exit_policy"])
    fixed_strat = _fixed(order["inputs"]["strategy"]["params"])

    exit_params = {**fixed_exit, **exit_pick}
    entry_params = {**fixed_strat, **strat_pick}

    ex = sim.Exit(exit_params, bool(exit_params["trail_always"]))
    runner = sim.RUNNERS[order["inputs"]["strategy"]["kind"]]
    trades = sim.apply_cap(runner(tokens, entry_params, ex, ctx), ctx.max_open)
    return sim.metrics(trades), exit_pick, strat_pick


def run(order: dict, tokens, spec_hash: str) -> dict:
    """Ejecuta el pedido completo y devuelve el documento `sweep_top.v1` (§3.3).

    `spec_hash` llega como argumento y no se lee del pedido: meter una clave de más en el
    dict del pedido cambiaría su forma canónica y con ella su propio hash.
    """
    inputs = order["inputs"]
    ctx = sim.Ctx(
        slippage_bps=inputs["costs"]["slippage_bps_per_side"],
        fee_lamports=inputs["costs"]["fee_lamports_per_tx"],
        notional_lamports=inputs["portfolio"]["notional_lamports"],
        max_open=inputs["portfolio"]["max_open_positions"],
    )

    if inputs["split"]["kind"] == "last_day_holdout.v1":
        train, test, _ = split_last_day(tokens)
    else:
        train, test = tokens, []

    axes = swept(order)
    combos = total_combos(axes)
    wanted = order["output_shape"]["metrics"]

    scored = []
    for index in range(combos):
        met, exit_pick, strat_pick = evaluate(order, train, axes, index, ctx)
        scored.append((met["net_lamports"], index, met, exit_pick, strat_pick))

    # Orden por neto descendente; empate por combo_index ascendente (§3.2).
    scored.sort(key=lambda row: (-row[0], row[1]))

    top_n = order["output_shape"]["top_n"]
    rows = []
    for rank, (_, index, met, exit_pick, strat_pick) in enumerate(scored[:top_n], start=1):
        rows.append(
            {
                "rank": rank,
                "combo_index": index,
                "params": {"exit_policy": exit_pick, "strategy": strat_pick},
                "metrics": {k: met[k] for k in wanted},
            }
        )

    if inputs["split"]["kind"] == "last_day_holdout.v1":
        best_index = scored[0][1] if scored else 0
        met, _, _ = evaluate(order, test, axes, best_index, ctx)
        validation = {
            "kind": "last_day_holdout.v1",
            "evaluated": len(test),
            "metrics": {k: met[k] for k in wanted},
        }
    else:
        validation = {"kind": "none.v1"}

    return {
        "schema_version": 1,
        "format": "sweep_top.v1",
        "spec_hash": spec_hash,
        "combos": combos,
        "evaluated": len(train),
        "rows": rows,
        "validation": validation,
    }
