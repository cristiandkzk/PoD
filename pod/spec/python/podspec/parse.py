"""Parser estricto del subconjunto de JSON aceptado — SPEC.md §3 y §7.1 paso 1.

No se usa `json` de la stdlib a propósito: acepta floats, colapsa claves duplicadas en
silencio y no distingue `null` de ausente. Las tres cosas son errores en este formato, y
un parser que las tapa no puede reportar el código de error que exige SPEC §7.
"""

import re

from .errors import SpecError

MAX_DEPTH = 8
MAX_STRING_LEN = 256

_WS = " \t\n\r"
_STRING_CHARS = frozenset(
    "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
    "abcdefghijklmnopqrstuvwxyz"
    "0123456789"
    "-_.:/@+="
)
_KEY_RE = re.compile(r"[a-z][a-z0-9_]{0,63}\Z")
_INT_RE = re.compile(r"-?(0|[1-9][0-9]*)\Z")
_NUM_CHARS = frozenset("+-0123456789.eE")

_I64_MIN = -(2**63)
_I64_MAX = 2**63 - 1

_SIMPLE_ESCAPES = {
    '"': '"',
    "\\": "\\",
    "/": "/",
    "b": "\b",
    "f": "\f",
    "n": "\n",
    "r": "\r",
    "t": "\t",
}


def parse(data: bytes):
    """bytes -> valor. La raíz tiene que ser un objeto (SPEC §7.1 paso 1)."""
    if not isinstance(data, (bytes, bytearray)):
        raise TypeError("parse() espera bytes")
    try:
        text = data.decode("utf-8", errors="strict")
    except UnicodeDecodeError:
        raise SpecError("E_SYNTAX", "$") from None
    if text.startswith("﻿"):
        raise SpecError("E_SYNTAX", "$")

    p = _Parser(text)
    value = p.value(depth=0, path="$")
    p.skip_ws()
    if p.i != len(p.s):
        raise SpecError("E_SYNTAX", "$")
    if not isinstance(value, dict):
        raise SpecError("E_NOT_OBJECT", "$")
    return value


class _Parser:
    __slots__ = ("s", "i")

    def __init__(self, s: str):
        self.s = s
        self.i = 0

    def skip_ws(self) -> None:
        s, n = self.s, len(self.s)
        while self.i < n and s[self.i] in _WS:
            self.i += 1

    def _peek(self, path: str) -> str:
        if self.i >= len(self.s):
            raise SpecError("E_SYNTAX", path)
        return self.s[self.i]

    def _literal(self, word: str) -> bool:
        if self.s.startswith(word, self.i):
            self.i += len(word)
            return True
        return False

    def value(self, depth: int, path: str):
        self.skip_ws()
        c = self._peek(path)
        if c == "{":
            return self.obj(depth + 1, path)
        if c == "[":
            return self.arr(depth + 1, path)
        if c == '"':
            text = self.string(path)
            if len(text) < 1 or len(text) > MAX_STRING_LEN:
                raise SpecError("E_STRING_CHARSET", path)
            if not _STRING_CHARS.issuperset(text):
                raise SpecError("E_STRING_CHARSET", path)
            return text
        if c == "t":
            if self._literal("true"):
                return True
            raise SpecError("E_SYNTAX", path)
        if c == "f":
            if self._literal("false"):
                return False
            raise SpecError("E_SYNTAX", path)
        if c == "n":
            if self._literal("null"):
                raise SpecError("E_NULL", path)
            raise SpecError("E_SYNTAX", path)
        if c == "N":
            if self._literal("NaN"):
                raise SpecError("E_FLOAT", path)
            raise SpecError("E_SYNTAX", path)
        if c == "I":
            if self._literal("Infinity"):
                raise SpecError("E_FLOAT", path)
            raise SpecError("E_SYNTAX", path)
        if c == "-" and self.s.startswith("-Infinity", self.i):
            raise SpecError("E_FLOAT", path)
        if c == "-" or c.isdigit():
            return self.number(path)
        raise SpecError("E_SYNTAX", path)

    def obj(self, depth: int, path: str):
        if depth > MAX_DEPTH:
            raise SpecError("E_DEPTH", path)
        self.i += 1  # '{'
        out: dict[str, object] = {}
        self.skip_ws()
        if self._peek(path) == "}":
            self.i += 1
            return out
        while True:
            self.skip_ws()
            if self._peek(path) != '"':
                raise SpecError("E_SYNTAX", path)
            key = self.string(path)
            if not _KEY_RE.match(key):
                raise SpecError("E_KEY_CHARSET", f"{path}.{key}")
            if key in out:
                raise SpecError("E_DUP_KEY", f"{path}.{key}")
            self.skip_ws()
            if self._peek(path) != ":":
                raise SpecError("E_SYNTAX", path)
            self.i += 1
            out[key] = self.value(depth, f"{path}.{key}")
            self.skip_ws()
            c = self._peek(path)
            if c == ",":
                self.i += 1
                continue
            if c == "}":
                self.i += 1
                return out
            raise SpecError("E_SYNTAX", path)

    def arr(self, depth: int, path: str):
        if depth > MAX_DEPTH:
            raise SpecError("E_DEPTH", path)
        self.i += 1  # '['
        out: list[object] = []
        self.skip_ws()
        if self._peek(path) == "]":
            raise SpecError("E_EMPTY_ARRAY", path)
        while True:
            out.append(self.value(depth, f"{path}[{len(out)}]"))
            self.skip_ws()
            c = self._peek(path)
            if c == ",":
                self.i += 1
                continue
            if c == "]":
                self.i += 1
                return out
            raise SpecError("E_SYNTAX", path)

    def string(self, path: str) -> str:
        """Decodifica un string JSON con sus escapes. No valida el conjunto de caracteres:
        eso lo hace quien llama, porque el código difiere entre clave y valor (SPEC §7.1)."""
        self.i += 1  # '"'
        s, n = self.s, len(self.s)
        out: list[str] = []
        while True:
            if self.i >= n:
                raise SpecError("E_SYNTAX", path)
            c = s[self.i]
            if c == '"':
                self.i += 1
                return "".join(out)
            if c == "\\":
                self.i += 1
                if self.i >= n:
                    raise SpecError("E_SYNTAX", path)
                e = s[self.i]
                if e in _SIMPLE_ESCAPES:
                    out.append(_SIMPLE_ESCAPES[e])
                    self.i += 1
                elif e == "u":
                    out.append(self._unicode_escape(path))
                else:
                    raise SpecError("E_SYNTAX", path)
                continue
            if ord(c) < 0x20:
                raise SpecError("E_SYNTAX", path)
            out.append(c)
            self.i += 1

    def _unicode_escape(self, path: str) -> str:
        cp = self._hex4(path)
        if 0xD800 <= cp <= 0xDBFF:
            if not self.s.startswith("\\u", self.i):
                raise SpecError("E_SYNTAX", path)
            self.i += 1
            low = self._hex4(path)
            if not 0xDC00 <= low <= 0xDFFF:
                raise SpecError("E_SYNTAX", path)
            cp = 0x10000 + ((cp - 0xD800) << 10) + (low - 0xDC00)
        elif 0xDC00 <= cp <= 0xDFFF:
            raise SpecError("E_SYNTAX", path)
        return chr(cp)

    def _hex4(self, path: str) -> int:
        self.i += 1  # 'u'
        digits = self.s[self.i : self.i + 4]
        if len(digits) != 4 or any(d not in "0123456789abcdefABCDEF" for d in digits):
            raise SpecError("E_SYNTAX", path)
        self.i += 4
        return int(digits, 16)

    def number(self, path: str) -> int:
        start = self.i
        s, n = self.s, len(self.s)
        while self.i < n and s[self.i] in _NUM_CHARS:
            self.i += 1
        token = s[start : self.i]
        if not token:
            raise SpecError("E_SYNTAX", path)
        if any(ch in token for ch in ".eE"):
            raise SpecError("E_FLOAT", path)
        if token == "-0":
            raise SpecError("E_INT_FORMAT", path)
        if not _INT_RE.match(token):
            raise SpecError("E_INT_FORMAT", path)
        value = int(token)
        if value < _I64_MIN or value > _I64_MAX:
            raise SpecError("E_INT_RANGE", path)
        return value
