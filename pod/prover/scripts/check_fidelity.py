"""Fidelidad del port: Python (runner) contra Node (simulador original de backtest.mjs).

Un `output_hash` reproducible no dice que el resultado sea el correcto — solo que es el
mismo. Este chequeo ataca la otra mitad: que el runner compute *el mismo trabajo* que el
backtester real. Se comparan las métricas de **todas** las combinaciones, no solo del top.
"""

import json
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

from podprover import dataset, sim, sweep  # noqa: E402  (su __init__ pone podspec en el path)

import podspec  # noqa: E402

DATA = ROOT.parent.parent.parent / "memebot" / "data"
FILES = [DATA / "ticks-2026-06-10.jsonl", DATA / "ticks-2026-06-15.jsonl"]
SCRATCH = ROOT / "fidelity" / ".tmp"


def compare(order_path: pathlib.Path, which_set: str = "train") -> tuple[int, int]:
    order = podspec.parse(order_path.read_bytes())
    podspec.validate(order)
    inputs = order["inputs"]

    data = dataset.dataset_bytes(FILES)
    tokens = dataset.load(data, inputs["dataset"]["hash"])
    if inputs["split"]["kind"] == "last_day_holdout.v1":
        train, test, _ = dataset.split_last_day(tokens)
        pool = test if which_set == "test" else train
    else:
        pool = tokens

    axes = sweep.swept(order)
    combos = sweep.total_combos(axes)
    ctx = sim.Ctx(
        inputs["costs"]["slippage_bps_per_side"],
        inputs["costs"]["fee_lamports_per_tx"],
        inputs["portfolio"]["notional_lamports"],
        inputs["portfolio"]["max_open_positions"],
    )

    combo_list = []
    mine = []
    for index in range(combos):
        met, exit_pick, strat_pick = sweep.evaluate(order, pool, axes, index, ctx)
        fixed_exit = {k: v for k, v in inputs["exit_policy"].items() if not isinstance(v, list)}
        combo_list.append(
            {"exit_policy": {**fixed_exit, **exit_pick}, "strategy": strat_pick}
        )
        mine.append(met)

    SCRATCH.mkdir(parents=True, exist_ok=True)
    payload = SCRATCH / f"combos-{order_path.stem}-{which_set}.json"
    payload.write_text(
        json.dumps(
            {
                "kind": inputs["strategy"]["kind"],
                "split": inputs["split"]["kind"],
                "set": which_set,
                "trail_always": inputs["exit_policy"]["trail_always"],
                "slippage_bps_per_side": inputs["costs"]["slippage_bps_per_side"],
                "fee_lamports_per_tx": inputs["costs"]["fee_lamports_per_tx"],
                "notional_lamports": inputs["portfolio"]["notional_lamports"],
                "max_open_positions": inputs["portfolio"]["max_open_positions"],
                "combos": combo_list,
            }
        ),
        encoding="utf-8",
    )

    proc = subprocess.run(
        ["node", str(ROOT / "fidelity" / "harness.mjs"), str(payload), *[str(f) for f in FILES]],
        capture_output=True,
        check=False,
    )
    if proc.returncode != 0:
        print("  el harness de Node fallo:", proc.stderr.decode()[:500])
        return 0, combos
    theirs = json.loads(proc.stdout.decode())

    same = 0
    for index, (a, b) in enumerate(zip(mine, theirs)):
        if a == b:
            same += 1
        elif same + 3 > index:  # muestra las primeras diferencias, no las 144
            print(f"    combo {index}: python={a}")
            print(f"    combo {index}: node   ={b}")
    return same, combos


def main() -> int:
    total_same = total = 0
    cases = [
        (ROOT / "orders" / "order-graduacion.json", "train"),
        (ROOT / "orders" / "order-sniper.json", "train"),
        (ROOT / "orders" / "order-survivor.json", "train"),
        (ROOT / "orders" / "order-survivor.json", "test"),
    ]
    for order_path, which in cases:
        if not order_path.exists():
            continue
        same, combos = compare(order_path, which)
        total_same += same
        total += combos
        mark = "ok " if same == combos else "FALLA"
        print(f"  [{mark}] {order_path.name:26s} {which:5s}  {same}/{combos} combinaciones iguales")
    print(f"\n{total_same}/{total} combinaciones coinciden entre Python y el simulador original")
    return 0 if total_same == total and total else 1


if __name__ == "__main__":
    raise SystemExit(main())
