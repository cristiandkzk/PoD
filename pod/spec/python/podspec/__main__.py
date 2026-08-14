"""CLI de la implementación A — contrato de salida compartido con la de Rust.

    python -m podspec hash  <archivo>   -> 64 hex en stdout
    python -m podspec canon <archivo>   -> bytes canónicos en stdout, sin salto final

En caso de rechazo: "<CODE>\t<path>" en stderr y código de salida 2. El cross-check del
gate 3 compara exactamente estas dos salidas entre las dos implementaciones.
"""

import sys

from .canonical import canonical_bytes, spec_hash
from .errors import SpecError
from .parse import parse
from .schema import validate


def main(argv: list[str]) -> int:
    if len(argv) != 3 or argv[1] not in ("hash", "canon"):
        sys.stderr.write("uso: python -m podspec <hash|canon> <archivo>\n")
        return 64
    try:
        with open(argv[2], "rb") as fh:
            data = fh.read()
    except OSError as exc:
        sys.stderr.write(f"E_IO\t{exc}\n")
        return 66

    try:
        value = parse(data)
        validate(value)
    except SpecError as exc:
        sys.stderr.write(f"{exc.code}\t{exc.path}\n")
        return 2

    if argv[1] == "hash":
        sys.stdout.write(spec_hash(value))
    else:
        sys.stdout.buffer.write(canonical_bytes(value))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
