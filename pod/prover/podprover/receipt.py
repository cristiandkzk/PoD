"""Recibo firmado — SPEC-RUNNER.md §4."""

import podspec

from . import ed25519

BODY_KEYS = (
    "schema_version",
    "spec_hash",
    "output_hash",
    "output",
    "runner_id",
    "started_unix_ms",
    "wall_ms",
    "peak_bytes",
)


def output_hash(output: dict) -> str:
    return podspec.hash_with_domain(podspec.DOMAIN_OUTPUT, output)


def _signing_bytes(body: dict) -> bytes:
    """Lo que se firma: dominio || bytes canónicos del recibo sin la firma (§4)."""
    return podspec.DOMAIN_RECEIPT + podspec.canonical_bytes(body)


def build(
    seed: bytes,
    spec_hash: str,
    output: dict,
    started_unix_ms: int,
    wall_ms: int,
    peak_bytes: int,
) -> dict:
    body = {
        "schema_version": 1,
        "spec_hash": spec_hash,
        "output_hash": output_hash(output),
        "output": output,
        "runner_id": ed25519.public_key(seed).hex(),
        "started_unix_ms": started_unix_ms,
        "wall_ms": wall_ms,
        "peak_bytes": peak_bytes,
    }
    signature = ed25519.sign(seed, _signing_bytes(body)).hex()
    return {**body, "signature": signature}


def check(receipt: dict) -> list[str]:
    """Chequeos baratos de §5, pasos 3 y 4. Devuelve la lista de problemas."""
    problems = []
    missing = [k for k in (*BODY_KEYS, "signature") if k not in receipt]
    if missing:
        return [f"faltan campos en el recibo: {', '.join(missing)}"]

    body = {k: receipt[k] for k in BODY_KEYS}
    try:
        ok = ed25519.verify(
            bytes.fromhex(receipt["runner_id"]),
            _signing_bytes(body),
            bytes.fromhex(receipt["signature"]),
        )
    except ValueError:
        ok = False
    if not ok:
        problems.append("la firma no verifica contra runner_id")

    if output_hash(receipt["output"]) != receipt["output_hash"]:
        problems.append("el output que trae el recibo no hashea a su output_hash")
    return problems
