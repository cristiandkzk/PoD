"""Validación del esquema cerrado del WorkOrder — SPEC.md §6 y §7.1 pasos 2 y 3.

El recorrido de cada objeto es una sola pasada sobre la unión de claves presentes y
declaradas, en orden canónico ascendente (SPEC §7.1.2). Las uniones etiquetadas resuelven
`kind` antes de recorrer (SPEC §6.11).
"""

import re

from .errors import SpecError

SCALE = 18

_I64_MAX = 2**63 - 1
_TEN18 = 10**SCALE

_SHA256_RE = re.compile(r"sha256:[0-9a-f]{64}\Z")
_HEX40_RE = re.compile(r"[0-9a-f]{40}\Z")
_BASE58_RE = re.compile(r"[1-9A-HJ-NP-Za-km-z]{32,44}\Z")
_TOOLCHAIN_RE = re.compile(r"[A-Za-z0-9][A-Za-z0-9._-]{0,63}\Z")
_DECIMAL_RE = re.compile(r"-?(0|[1-9][0-9]*)(\.[0-9]*[1-9])?\Z")

_METRICS = frozenset(
    {
        "gross_loss_lamports",
        "gross_win_lamports",
        "n_trades",
        "net_lamports",
        "unclosed",
        "wins",
    }
)


# --------------------------------------------------------------------------- primitivas


def _is_int(v) -> bool:
    return isinstance(v, int) and not isinstance(v, bool)


def _is_str(v) -> bool:
    return isinstance(v, str)


def _walk(obj, path: str, fields: dict) -> None:
    """Una sola pasada sobre la unión de claves, en orden canónico (SPEC §7.1.2)."""
    if not isinstance(obj, dict):
        raise SpecError("E_TYPE", path)
    declared = set(fields)
    present = set(obj)
    for key in sorted(declared | present, key=lambda k: k.encode("utf-8")):
        sub = f"{path}.{key}"
        if key not in declared:
            raise SpecError("E_UNKNOWN_FIELD", sub)
        if key not in present:
            raise SpecError("E_MISSING_FIELD", sub)
        fields[key](obj[key], sub)


def _walk_tagged(obj, path: str, table: dict) -> str:
    """Unión etiquetada: `kind` primero, después el resto en orden canónico (SPEC §6.11)."""
    if not isinstance(obj, dict):
        raise SpecError("E_TYPE", path)
    if "kind" not in obj:
        raise SpecError("E_MISSING_FIELD", f"{path}.kind")
    kind = obj["kind"]
    if not _is_str(kind):
        raise SpecError("E_TYPE", f"{path}.kind")
    if kind not in table:
        raise SpecError("E_ENUM", f"{path}.kind")
    fields = {"kind": _v_noop}
    fields.update(table[kind])
    _walk(obj, path, fields)
    return kind


def _v_noop(value, path: str) -> None:
    return None


def v_int(lo: int, hi: int):
    def check(value, path: str) -> None:
        if not _is_int(value):
            raise SpecError("E_TYPE", path)
        if value < lo or value > hi:
            raise SpecError("E_INT_RANGE", path)

    return check


def v_exact_int(expected: int):
    def check(value, path: str) -> None:
        if not _is_int(value):
            raise SpecError("E_TYPE", path)
        if value != expected:
            raise SpecError("E_INT_RANGE", path)

    return check


def v_bool(value, path: str) -> None:
    if not isinstance(value, bool):
        raise SpecError("E_TYPE", path)


def v_enum(allowed):
    allowed = frozenset(allowed)

    def check(value, path: str) -> None:
        if not _is_str(value):
            raise SpecError("E_TYPE", path)
        if value not in allowed:
            raise SpecError("E_ENUM", path)

    return check


def v_str_re(pattern: re.Pattern):
    def check(value, path: str) -> None:
        if not _is_str(value):
            raise SpecError("E_TYPE", path)
        if not pattern.match(value):
            raise SpecError("E_STRING_FORMAT", path)

    return check


def v_str_any(value, path: str) -> None:
    if not _is_str(value):
        raise SpecError("E_TYPE", path)


def v_str_array(item_check):
    def check(value, path: str) -> None:
        if not isinstance(value, list):
            raise SpecError("E_TYPE", path)
        for i, item in enumerate(value):
            item_check(item, f"{path}[{i}]")

    return check


def v_metrics(value, path: str) -> None:
    if not isinstance(value, list):
        raise SpecError("E_TYPE", path)
    for i, item in enumerate(value):
        if not _is_str(item):
            raise SpecError("E_TYPE", f"{path}[{i}]")
        if item not in _METRICS:
            raise SpecError("E_ENUM", f"{path}[{i}]")
    for i in range(1, len(value)):
        if value[i - 1].encode("utf-8") >= value[i].encode("utf-8"):
            raise SpecError("E_NOT_SORTED", path)


def decimal_scaled(text: str, path: str) -> int:
    """Decimal -> entero escalado por 10^18, para comparar sin punto flotante (SPEC §3.4)."""
    if not _is_str(text):
        raise SpecError("E_TYPE", path)
    if not _DECIMAL_RE.match(text):
        raise SpecError("E_DECIMAL_FORMAT", path)
    negative = text[0] == "-"
    body = text[1:] if negative else text
    int_part, _, frac_part = body.partition(".")
    if len(int_part) > 18 or len(frac_part) > 18:
        raise SpecError("E_DECIMAL_FORMAT", path)
    value = int(int_part + frac_part.ljust(SCALE, "0"))
    if negative:
        if value == 0:
            raise SpecError("E_DECIMAL_FORMAT", path)
        value = -value
    return value


# ------------------------------------------------------------- rejillas de barrido §3.5


def grid_int(lo: int, hi: int):
    """Rejilla de enteros: >=1 elemento, estrictamente ascendente (SPEC §3.5)."""
    item = v_int(lo, hi)

    def check(value, path: str) -> None:
        if not isinstance(value, list):
            raise SpecError("E_TYPE", path)
        for i, v in enumerate(value):
            item(v, f"{path}[{i}]")
        for i in range(1, len(value)):
            if value[i - 1] >= value[i]:
                raise SpecError("E_NOT_SORTED", path)

    return check


def grid_decimal(lo: str, hi: str, *, lo_inclusive: bool, hi_inclusive: bool):
    """Rejilla de decimales: >=1 elemento, ascendente por valor escalado (SPEC §3.5)."""
    lo_s = decimal_scaled(lo, "$")
    hi_s = decimal_scaled(hi, "$")

    def check(value, path: str) -> None:
        if not isinstance(value, list):
            raise SpecError("E_TYPE", path)
        scaled = []
        for i, v in enumerate(value):
            sub = f"{path}[{i}]"
            s = decimal_scaled(v, sub)
            if s < lo_s or (s == lo_s and not lo_inclusive):
                raise SpecError("E_DECIMAL_RANGE", sub)
            if s > hi_s or (s == hi_s and not hi_inclusive):
                raise SpecError("E_DECIMAL_RANGE", sub)
            scaled.append(s)
        for i in range(1, len(scaled)):
            if scaled[i - 1] >= scaled[i]:
                raise SpecError("E_NOT_SORTED", path)

    return check


def _grid_max_int(value) -> int:
    """Máximo de una rejilla de enteros ya validada (viene ascendente)."""
    return value[-1]


# ------------------------------------------------------------------------ subobjetos §6

_UNLIMITED_DEC = "999999999999999999"


def _v_dataset(value, path: str) -> None:
    _walk(
        value,
        path,
        {
            "days": v_int(1, _I64_MAX),
            "first_unix_ms": v_int(0, _I64_MAX),
            "format": v_enum({"ticks.jsonl.v1"}),
            "hash": v_str_re(_SHA256_RE),
            "last_unix_ms": v_int(0, _I64_MAX),
            "records": v_int(1, _I64_MAX),
            "tokens": v_int(1, _I64_MAX),
        },
    )
    if value["last_unix_ms"] <= value["first_unix_ms"]:
        raise SpecError("E_CONSTRAINT", f"{path}.last_unix_ms")


def _v_split(value, path: str) -> str:
    return _walk_tagged(value, path, {"last_day_holdout.v1": {}, "none.v1": {}})


def _v_params_survivor(value, path: str) -> None:
    _walk(
        value,
        path,
        {
            "max_age_min": grid_int(1, 1440),
            "min_age_min": grid_int(0, 1440),
            "min_buyers": grid_int(1, 100000),
            "min_growth": grid_decimal("0", _UNLIMITED_DEC, lo_inclusive=False, hi_inclusive=True),
            "min_mcap": grid_decimal("0", _UNLIMITED_DEC, lo_inclusive=False, hi_inclusive=True),
            "min_ratio": grid_decimal("0", _UNLIMITED_DEC, lo_inclusive=False, hi_inclusive=True),
        },
    )
    if _grid_max_int(value["min_age_min"]) >= _grid_max_int(value["max_age_min"]):
        raise SpecError("E_CONSTRAINT", f"{path}.max_age_min")


def _v_params_sniper(value, path: str) -> None:
    _walk(
        value,
        path,
        {
            "max_dev_lamports": grid_int(0, _I64_MAX),
            "min_dev_lamports": grid_int(0, _I64_MAX),
            "panic_lamports": grid_int(0, _I64_MAX),
            "stall_s": grid_int(1, 86400),
        },
    )
    if _grid_max_int(value["min_dev_lamports"]) > _grid_max_int(value["max_dev_lamports"]):
        raise SpecError("E_CONSTRAINT", f"{path}.max_dev_lamports")


def _v_params_graduacion(value, path: str) -> None:
    _walk(
        value,
        path,
        {
            "abort_bps": grid_int(0, 10000),
            "dip_bps": grid_int(0, 10000),
            "timeout_min": grid_int(1, 1440),
        },
    )


def _v_strategy(value, path: str) -> None:
    _walk_tagged(
        value,
        path,
        {
            "survivor.v1": {"params": _v_params_survivor},
            "sniper.v1": {"params": _v_params_sniper},
            "graduacion.v1": {"params": _v_params_graduacion},
        },
    )


def _v_exit_policy(value, path: str) -> None:
    _walk(
        value,
        path,
        {
            "hard_stop_bps": grid_int(1, 10000),
            "time_stop_min": grid_int(1, 1440),
            "tp_mult": grid_decimal("1", _UNLIMITED_DEC, lo_inclusive=False, hi_inclusive=True),
            "tp_sell_bps": grid_int(1, 10000),
            "trail_always": v_bool,
            "trail_arm_bps": grid_int(0, 100000),
            "trail_bps": grid_int(1, 10000),
            "trail_sell_bps": grid_int(1, 10000),
        },
    )


def _v_portfolio(value, path: str) -> None:
    _walk(
        value,
        path,
        {
            "max_open_positions": v_int(1, 10000),
            "notional_lamports": v_int(1, _I64_MAX),
        },
    )


def _v_costs(value, path: str) -> None:
    _walk(
        value,
        path,
        {
            "fee_lamports_per_tx": v_int(0, _I64_MAX),
            "slippage_bps_per_side": v_int(0, 10000),
        },
    )


def _v_inputs_sweep(value, path: str) -> None:
    _walk(
        value,
        path,
        {
            "costs": _v_costs,
            "dataset": _v_dataset,
            "exit_policy": _v_exit_policy,
            "portfolio": _v_portfolio,
            "split": _v_split,
            "strategy": _v_strategy,
        },
    )
    if value["split"]["kind"] == "last_day_holdout.v1" and value["dataset"]["days"] < 2:
        raise SpecError("E_CONSTRAINT", f"{path}.split")


def _v_output_shape(value, path: str) -> None:
    _walk(
        value,
        path,
        {
            "format": v_enum({"sweep_top.v1"}),
            "metrics": v_metrics,
            "rounding": v_enum({"trunc_to_lamports.v1"}),
            "top_n": v_int(1, 1000),
        },
    )


def _v_runner(value, path: str) -> None:
    _walk(
        value,
        path,
        {
            "commit": v_str_re(_HEX40_RE),
            "entrypoint": v_str_array(v_str_any),
            "image_digest": v_str_re(_SHA256_RE),
            "image_ref": v_str_any,
            "toolchain": v_str_re(_TOOLCHAIN_RE),
        },
    )


def _v_limits(value, path: str) -> None:
    _walk(
        value,
        path,
        {
            "cpu_count": v_int(1, 64),
            "memory_bytes": v_int(1, 2**40),
            "wall_time_s": v_int(1, 86400),
        },
    )


def _v_deadline(value, path: str) -> None:
    _walk(
        value,
        path,
        {
            "accept_by_unix_s": v_int(1, _I64_MAX),
            "deliver_within_s": v_int(1, 2592000),
        },
    )


def _v_payment(value, path: str) -> None:
    _walk(
        value,
        path,
        {
            "amount_base_units": v_int(1, _I64_MAX),
            "bond_base_units": v_int(0, _I64_MAX),
            "mint": v_str_re(_BASE58_RE),
            "mint_decimals": v_int(0, 18),
        },
    )


def _v_proof_mode(value, path: str) -> None:
    _walk_tagged(
        value,
        path,
        {
            "optimistic": {"challenge_window_s": v_int(1, 2592000)},
            "zk": {"verifier_key": v_str_re(_SHA256_RE)},
        },
    )


_INPUTS_BY_CLASS = {"backtest.sweep.v1": _v_inputs_sweep}


# ------------------------------------------------------------------------------- raíz


def validate(order) -> None:
    """Valida un WorkOrder ya parseado. Lanza SpecError con el código de SPEC §7."""
    if not isinstance(order, dict):
        raise SpecError("E_NOT_OBJECT", "$")

    def v_inputs(value, path: str) -> None:
        # `class` ordena antes que `inputs`, así que ya fue validada por el recorrido
        # canónico de _walk cuando se llega acá (SPEC §6.11, nota final).
        _INPUTS_BY_CLASS[order["class"]](value, path)

    _walk(
        order,
        "$",
        {
            "class": v_enum(_INPUTS_BY_CLASS),
            "deadline": _v_deadline,
            "inputs": v_inputs,
            "limits": _v_limits,
            "output_shape": _v_output_shape,
            "payment": _v_payment,
            "proof_mode": _v_proof_mode,
            "runner": _v_runner,
            "schema_version": v_exact_int(1),
        },
    )
