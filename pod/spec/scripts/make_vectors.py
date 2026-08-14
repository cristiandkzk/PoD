"""Genera los vectores congelados de testvectors/ — SPEC.md §8.

Los `.json` que produce son el artefacto: se versionan y no se regeneran salvo que cambie
el formato (y en ese caso cambia `schema_version` y el dominio del hash). Este script
existe para que los casos hostiles tengan bytes exactos y deliberados, no para que los
vectores se rehagan cada vez.

No importa `podspec`: si el generador usara el canonicalizador, los vectores validarían
contra sí mismos.

El pedido base es un barrido real de `memebot/backtest.mjs`: las rejillas salen de
`gridEntries()` y `gridExits()`, los costos de los defaults del CLI, y el dataset es el
SHA-256 de dos archivos `data/ticks-*.jsonl` reales concatenados por orden de nombre.
"""

import copy
import json
import pathlib
import unicodedata

ROOT = pathlib.Path(__file__).resolve().parent.parent
VALID = ROOT / "testvectors" / "valid"
REJECT = ROOT / "testvectors" / "reject"

# Dataset real: ticks-2026-06-17.jsonl + ticks-2026-07-16.jsonl, en ese orden.
DATASET = {
    "hash": "sha256:067e277c1dac59fb8a7ca383f5f9aac087221c66a512316e7e290b72c7a993fc",
    "format": "ticks.jsonl.v1",
    "records": 24105,
    "tokens": 8407,
    "days": 2,
    "first_unix_ms": 1781734633178,
    "last_unix_ms": 1784187305956,
}

IMAGE_DIGEST = "sha256:" + "a7d4e019" * 8
COMMIT = "9c1d4f7a" * 5
USDC = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"

RUNNER = {
    "image_ref": "ghcr.io/memebot/backtest-runner",
    "image_digest": IMAGE_DIGEST,
    "commit": COMMIT,
    "toolchain": "node-22.22.3",
    "entrypoint": ["/usr/local/bin/node", "/app/backtest.mjs", "--json"],
}

# gridExits() de backtest.mjs, con los porcentajes pasados a puntos basicos.
EXIT_POLICY = {
    "tp_mult": ["1.5", "2", "3"],
    "tp_sell_bps": [5000, 10000],
    "trail_bps": [2000, 3000, 5000],
    "trail_arm_bps": [0, 3000, 5000],
    "trail_sell_bps": [5000, 10000],
    "hard_stop_bps": [2500, 4000, 6000],
    "time_stop_min": [15, 30, 60],
    "trail_always": False,
}

ALL_METRICS = [
    "gross_loss_lamports",
    "gross_win_lamports",
    "n_trades",
    "net_lamports",
    "unclosed",
    "wins",
]

# Barrido SUPERVIVIENTE: gridEntries() x gridExits() = 81 x 972 combinaciones.
BASE = {
    "schema_version": 1,
    "class": "backtest.sweep.v1",
    "inputs": {
        "dataset": DATASET,
        "split": {"kind": "last_day_holdout.v1"},
        "strategy": {
            "kind": "survivor.v1",
            "params": {
                "min_age_min": [8],
                "max_age_min": [35],
                "min_buyers": [25, 40, 60],
                "min_mcap": ["30", "40", "60"],
                "min_growth": ["1.5", "1.8", "2.5"],
                "min_ratio": ["1.2", "1.4", "1.8"],
            },
        },
        "exit_policy": EXIT_POLICY,
        "portfolio": {"max_open_positions": 4, "notional_lamports": 20000000},
        "costs": {"slippage_bps_per_side": 300, "fee_lamports_per_tx": 1000000},
    },
    "runner": RUNNER,
    "limits": {"wall_time_s": 3600, "memory_bytes": 4294967296, "cpu_count": 2},
    "output_shape": {
        "format": "sweep_top.v1",
        "top_n": 8,
        "metrics": ALL_METRICS,
        "rounding": "trunc_to_lamports.v1",
    },
    "deadline": {"accept_by_unix_s": 1789000000, "deliver_within_s": 7200},
    "payment": {
        "mint": USDC,
        "mint_decimals": 6,
        "amount_base_units": 5000000,
        "bond_base_units": 10000000,
    },
    "proof_mode": {"kind": "optimistic", "challenge_window_s": 3600},
}


def deep_reverse(value):
    """Mismo objeto con el orden de claves invertido en cada nivel."""
    if isinstance(value, dict):
        return {k: deep_reverse(value[k]) for k in reversed(list(value))}
    if isinstance(value, list):
        return [deep_reverse(v) for v in value]
    return value


def rotate_keys(value, shift: int = 3):
    """Mismo objeto con las claves rotadas en cada nivel — tercera permutación."""
    if isinstance(value, dict):
        keys = list(value)
        n = len(keys)
        order = [keys[(i + shift) % n] for i in range(n)]
        return {k: rotate_keys(value[k], shift) for k in order}
    if isinstance(value, list):
        return [rotate_keys(v, shift) for v in value]
    return value


def mutate(path: str, new_value):
    """Copia de BASE con un campo cambiado. `path` va con puntos."""
    out = copy.deepcopy(BASE)
    parts = path.split(".")
    node = out
    for p in parts[:-1]:
        node = node[p]
    node[parts[-1]] = new_value
    return out


def drop(path: str):
    out = copy.deepcopy(BASE)
    parts = path.split(".")
    node = out
    for p in parts[:-1]:
        node = node[p]
    del node[parts[-1]]
    return out


def sniper_order():
    """gridSniper() de backtest.mjs: SOL -> lamports."""
    o = copy.deepcopy(BASE)
    o["inputs"]["strategy"] = {
        "kind": "sniper.v1",
        "params": {
            "min_dev_lamports": [100000000, 500000000, 1000000000],
            "max_dev_lamports": [5000000000],
            "stall_s": [45],
            "panic_lamports": [1500000000],
        },
    }
    o["inputs"]["exit_policy"] = {**EXIT_POLICY, "trail_always": True}
    o["inputs"]["split"] = {"kind": "none.v1"}
    o["proof_mode"] = {"kind": "zk", "verifier_key": "sha256:" + "0f5e2b91" * 8}
    return o


def graduacion_order():
    """Barrido de una sola combinacion: la clase cubre el backtest suelto (SPEC §3.5)."""
    o = copy.deepcopy(BASE)
    o["inputs"]["strategy"] = {
        "kind": "graduacion.v1",
        "params": {"dip_bps": [1000], "timeout_min": [10], "abort_bps": [5000]},
    }
    o["inputs"]["exit_policy"] = {
        "tp_mult": ["2"],
        "tp_sell_bps": [5000],
        "trail_bps": [3000],
        "trail_arm_bps": [0],
        "trail_sell_bps": [10000],
        "hard_stop_bps": [4000],
        "time_stop_min": [30],
        "trail_always": False,
    }
    o["payment"]["bond_base_units"] = 0
    o["output_shape"]["top_n"] = 1
    o["output_shape"]["metrics"] = ["net_lamports"]
    return o


def write(directory: pathlib.Path, name: str, text: str) -> None:
    directory.mkdir(parents=True, exist_ok=True)
    (directory / name).write_bytes(text.encode("utf-8"))


def dump(value, **kw) -> str:
    kw.setdefault("ensure_ascii", False)
    return json.dumps(value, **kw)


def main() -> None:
    for old in list(VALID.glob("*.json")) + list(REJECT.glob("*.json")):
        old.unlink()

    # ---------------------------------------------------------------- válidos
    # 01-05 DEBEN producir el mismo spec_hash. Es el gate 1.
    write(VALID, "01_survivor_indent.json", dump(BASE, indent=2) + "\n")
    write(
        VALID,
        "02_survivor_sorted_compact.json",
        dump(BASE, sort_keys=True, separators=(",", ":")),
    )
    write(VALID, "03_survivor_reversed.json", dump(deep_reverse(BASE), indent=4))
    write(
        VALID,
        "04_survivor_rotated_tabs.json",
        dump(rotate_keys(BASE), indent="\t").replace("\n", "\r\n") + "\r\n\r\n",
    )
    # Escapes \u: mismo texto decodificado, bytes de entrada distintos (SPEC §3.2).
    escaped = dump(BASE, sort_keys=True, separators=(",", ":"))
    escaped = escaped.replace('"ticks.jsonl.v1"', '"\\u0074icks.jsonl.v1"').replace(
        '"survivor.v1"', '"survivor.v\\u0031"'
    )
    write(VALID, "05_survivor_escaped.json", escaped)

    # 06 cambia un solo valor de una rejilla: el hash DEBE diferir. Es el gate 2.
    write(
        VALID,
        "06_survivor_grid_changed.json",
        dump(mutate("inputs.strategy.params.min_buyers", [25, 40, 61]), indent=2) + "\n",
    )
    write(VALID, "07_sniper_zk_nosplit.json", dump(sniper_order(), indent=2) + "\n")
    write(VALID, "08_graduacion_single.json", dump(graduacion_order(), indent=2) + "\n")

    # ---------------------------------------------------------------- rechazos
    r = {}
    base_txt = dump(BASE, indent=2)
    r["r01_null.json"] = dump(mutate("payment.bond_base_units", None), indent=2)
    r["r02_float.json"] = base_txt.replace('"top_n": 8,', '"top_n": 8.0,')
    r["r03_float_exponent.json"] = base_txt.replace('"top_n": 8,', '"top_n": 8e0,')
    r["r04_dup_key.json"] = base_txt.replace('"top_n": 8,', '"top_n": 8,\n    "top_n": 9,')
    r["r05_key_charset.json"] = base_txt.replace('"top_n"', '"Top_n"')
    r["r06_string_charset_nfc.json"] = dump(
        mutate("output_shape.rounding", "trunc_ó"), indent=2
    )
    r["r07_string_charset_nfd.json"] = dump(
        mutate("output_shape.rounding", unicodedata.normalize("NFD", "trunc_ó")), indent=2
    )
    r["r08_int_leading_zero.json"] = base_txt.replace('"top_n": 8,', '"top_n": 08,')
    r["r09_negative_zero.json"] = base_txt.replace('"top_n": 8,', '"top_n": -0,')
    r["r10_decimal_trailing_zero.json"] = dump(
        mutate("inputs.exit_policy.tp_mult", ["1.50", "2", "3"]), indent=2
    )
    r["r11_decimal_range.json"] = dump(
        mutate("inputs.exit_policy.tp_mult", ["0.5", "2", "3"]), indent=2
    )
    r["r12_empty_array.json"] = dump(mutate("output_shape.metrics", []), indent=2)
    r["r13_unknown_field.json"] = dump(mutate("extra", 1), indent=2)
    r["r14_missing_field.json"] = dump(drop("payment"), indent=2)
    r["r15_metrics_unsorted.json"] = dump(
        mutate("output_shape.metrics", ["wins", "n_trades"]), indent=2
    )
    r["r16_metrics_duplicate.json"] = dump(
        mutate("output_shape.metrics", ["wins", "wins"]), indent=2
    )
    r["r17_constraint_age.json"] = dump(
        mutate("inputs.strategy.params.min_age_min", [40]), indent=2
    )
    r["r18_constraint_dates.json"] = dump(
        mutate("inputs.dataset.last_unix_ms", DATASET["first_unix_ms"]), indent=2
    )
    r["r19_enum_class.json"] = dump(mutate("class", "backtest.sweep.v2"), indent=2)
    r["r20_type.json"] = dump(mutate("output_shape.top_n", "8"), indent=2)
    r["r21_not_object.json"] = "[1,2]"
    r["r22_syntax_trailing_comma.json"] = base_txt.replace('"top_n": 8,', '"top_n": 8,,')
    r["r23_int_range.json"] = dump(mutate("limits.cpu_count", 65), indent=2)
    r["r24_string_format_sha.json"] = dump(
        mutate("inputs.dataset.hash", "sha256:ZZZZ" + "067e277c" * 7 + "1dac59"), indent=2
    )
    r["r25_depth.json"] = '{"a":{"b":{"c":{"d":{"e":{"f":{"g":{"h":{"i":1}}}}}}}}}'
    r["r26_union_wrong_key.json"] = dump(
        mutate(
            "proof_mode",
            {
                "kind": "optimistic",
                "challenge_window_s": 3600,
                "verifier_key": "sha256:" + "0f5e2b91" * 8,
            },
        ),
        indent=2,
    )
    r["r27_bom.json"] = "﻿" + base_txt
    r["r28_trailing_garbage.json"] = base_txt + " x"
    r["r29_int_over_i64.json"] = base_txt.replace(
        '"records": 24105', '"records": 9223372036854775808'
    )
    # Reglas de rejilla (SPEC §3.5) — no existian en la version anterior del esquema.
    r["r30_grid_unsorted_int.json"] = dump(
        mutate("inputs.strategy.params.min_buyers", [60, 25, 40]), indent=2
    )
    r["r31_grid_duplicate_int.json"] = dump(
        mutate("inputs.strategy.params.min_buyers", [25, 25]), indent=2
    )
    r["r32_grid_unsorted_decimal.json"] = dump(
        mutate("inputs.exit_policy.tp_mult", ["3", "2"]), indent=2
    )
    r["r33_grid_scalar.json"] = dump(
        mutate("inputs.strategy.params.min_buyers", 40), indent=2
    )
    r["r34_constraint_split_days.json"] = dump(mutate("inputs.dataset.days", 1), indent=2)
    sniper_bad = sniper_order()
    sniper_bad["inputs"]["strategy"]["params"]["min_dev_lamports"] = [6000000000]
    r["r35_constraint_dev.json"] = dump(sniper_bad, indent=2)

    for name, text in r.items():
        write(REJECT, name, text)

    print(f"escritos {len(list(VALID.glob('*.json')))} válidos y {len(r)} de rechazo")


if __name__ == "__main__":
    main()
