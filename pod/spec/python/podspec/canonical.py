"""Serialización canónica y hashes con dominio — SPEC.md §4, §5 y §5.1."""

import hashlib

DOMAIN = b"PoD/WorkOrder/1\x00"
DOMAIN_OUTPUT = b"PoD/Output/1\x00"
DOMAIN_RECEIPT = b"PoD/Receipt/1\x00"


def canonical_text(value) -> str:
    """Forma canónica de un valor ya validado (SPEC §4).

    No escapa nada: el conjunto de caracteres de §3.2 es ASCII imprimible sin comillas
    ni barra invertida, así que un escape sería inalcanzable.
    """
    out: list[str] = []
    _write(value, out)
    return "".join(out)


def _write(value, out: list[str]) -> None:
    if value is True:
        out.append("true")
    elif value is False:
        out.append("false")
    elif isinstance(value, int):
        out.append(str(value))
    elif isinstance(value, str):
        out.append('"')
        out.append(value)
        out.append('"')
    elif isinstance(value, list):
        out.append("[")
        for i, item in enumerate(value):
            if i:
                out.append(",")
            _write(item, out)
        out.append("]")
    elif isinstance(value, dict):
        out.append("{")
        # Orden ascendente por los bytes UTF-8 de la clave. Las claves son ASCII minúscula
        # (SPEC §3.1), así que el orden por bytes de Python coincide con el normativo.
        for i, key in enumerate(sorted(value, key=lambda k: k.encode("utf-8"))):
            if i:
                out.append(",")
            out.append('"')
            out.append(key)
            out.append('":')
            _write(value[key], out)
        out.append("}")
    else:
        raise TypeError(f"valor no canonicalizable: {type(value).__name__}")


def canonical_bytes(value) -> bytes:
    return canonical_text(value).encode("utf-8")


def hash_with_domain(domain: bytes, value) -> str:
    """SHA-256(dominio || bytes canónicos), hex minúscula (SPEC §5.1).

    Toda pieza hasheada del protocolo pasa por acá; el dominio sale del registro de §5.1.
    """
    return hashlib.sha256(domain + canonical_bytes(value)).hexdigest()


def spec_hash(value) -> str:
    """`spec_hash` de un WorkOrder (SPEC §5)."""
    return hash_with_domain(DOMAIN, value)
