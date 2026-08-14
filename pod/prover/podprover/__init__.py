"""Runner determinístico de PoD — subfase 1.2. Ver SPEC-RUNNER.md.

Reusa la canonicalización de `pod/spec/python/podspec`: es la única fuente de verdad de
SPEC §4 y §5, y tener una segunda copia acá seria garantizar que se desincronicen.
"""

import pathlib
import sys

_SPEC_PY = pathlib.Path(__file__).resolve().parent.parent.parent / "spec" / "python"
if str(_SPEC_PY) not in sys.path:
    sys.path.insert(0, str(_SPEC_PY))

__all__ = ["dataset", "ed25519", "receipt", "sim", "sweep"]
