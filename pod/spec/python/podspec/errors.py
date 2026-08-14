"""Errores del formato — SPEC.md §7."""


class SpecError(Exception):
    """Rechazo de una entrada. El `code` es parte del contrato: dos implementaciones
    tienen que coincidir en el código, no solo en el hecho de rechazar (SPEC §7)."""

    __slots__ = ("code", "path")

    def __init__(self, code: str, path: str = ""):
        self.code = code
        self.path = path
        super().__init__(f"{code} at {path}" if path else code)
