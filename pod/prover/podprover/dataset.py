"""Carga determinística del log de ticks — SPEC-RUNNER.md §2.1.

El único insumo es la secuencia de bytes que fija `inputs.dataset.hash`. Nada de esta
capa puede depender del sistema de archivos, del orden en que el SO devolvió los nombres,
ni de la localización.
"""

import datetime
import hashlib
import json
import pathlib


class DatasetError(Exception):
    pass


class Token:
    """Un mint con sus series ya ordenadas."""

    __slots__ = ("mint", "create", "ticks", "grad", "migrate_at", "day")

    def __init__(self, mint: str):
        self.mint = mint
        self.create = None
        self.ticks: list[dict] = []
        self.grad: list[dict] = []
        self.migrate_at = None
        self.day = None


def dataset_bytes(paths: list[pathlib.Path]) -> bytes:
    """Concatenación de los archivos ordenados por nombre, sin separador (§2.1.1)."""
    ordered = sorted(paths, key=lambda p: p.name)
    return b"".join(p.read_bytes() for p in ordered)


def dataset_hash(data: bytes) -> str:
    return "sha256:" + hashlib.sha256(data).hexdigest()


def load(data: bytes, expected_hash: str) -> list[Token]:
    """bytes -> tokens, en orden ascendente de mint (§2.1.6)."""
    got = dataset_hash(data)
    if got != expected_hash:
        raise DatasetError(f"dataset no coincide con el pedido: {got} != {expected_hash}")

    tokens: dict[str, Token] = {}
    for raw in data.split(b"\n"):
        if not raw:
            continue
        try:
            rec = json.loads(raw)
        except (ValueError, UnicodeDecodeError):
            # Una linea rota se descarta, igual que en el backtester de referencia (§2.1.2).
            continue
        mint = rec.get("m")
        if not isinstance(mint, str):
            continue
        tok = tokens.get(mint)
        if tok is None:
            tok = tokens[mint] = Token(mint)
        kind = rec.get("k")
        if kind == "c":
            tok.create = rec
        elif kind == "t":
            tok.ticks.append(rec)
        elif kind == "m":
            tok.migrate_at = rec.get("t")
        elif kind == "g":
            tok.grad.append(rec)

    out = []
    for mint in sorted(tokens, key=lambda m: m.encode("utf-8")):
        tok = tokens[mint]
        # Orden estable por `t`: el desempate es el orden de aparicion, que es fijo
        # porque los bytes del dataset lo son (§2.1.4).
        tok.ticks.sort(key=lambda r: r.get("t", 0))
        tok.grad.sort(key=lambda r: r.get("t", 0))
        tok.day = _day_of(tok)
        out.append(tok)
    return out


def _day_of(tok: Token) -> str | None:
    """Fecha UTC de t0, sin punto flotante (§2.1.5)."""
    t0 = None
    if tok.create is not None:
        t0 = tok.create.get("t")
    if t0 is None and tok.ticks:
        t0 = tok.ticks[0].get("t")
    if t0 is None:
        t0 = tok.migrate_at
    if not isinstance(t0, int):
        return None
    return datetime.datetime.fromtimestamp(t0 // 1000, datetime.UTC).date().isoformat()


def split_last_day(tokens: list[Token]) -> tuple[list[Token], list[Token], str | None]:
    """Devuelve (train, test, dia_de_test) segun §2.2."""
    days = sorted({t.day for t in tokens if t.day is not None})
    if len(days) < 2:
        return tokens, [], None
    last = days[-1]
    train = [t for t in tokens if t.day != last]
    test = [t for t in tokens if t.day == last]
    return train, test, last
