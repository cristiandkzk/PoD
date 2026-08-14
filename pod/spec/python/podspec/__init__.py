"""Implementación A (Python) del formato canónico de WorkOrder — SPEC.md.

Escrita contra SPEC.md, no contra la implementación de Rust. Ver subfase 1.1, gate 3.
"""

from .canonical import (
    DOMAIN,
    DOMAIN_OUTPUT,
    DOMAIN_RECEIPT,
    canonical_bytes,
    canonical_text,
    hash_with_domain,
    spec_hash,
)
from .errors import SpecError
from .parse import parse
from .schema import validate

__all__ = [
    "DOMAIN",
    "DOMAIN_OUTPUT",
    "DOMAIN_RECEIPT",
    "hash_with_domain",
    "SpecError",
    "canonical_bytes",
    "canonical_text",
    "parse",
    "spec_hash",
    "validate",
    "load",
    "hash_bytes",
]


def load(data: bytes):
    """bytes -> WorkOrder validado. Lanza SpecError."""
    value = parse(data)
    validate(value)
    return value


def hash_bytes(data: bytes) -> str:
    """bytes -> spec_hash. Lanza SpecError si la entrada no es un WorkOrder válido."""
    return spec_hash(load(data))
