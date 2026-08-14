"""Ed25519 (RFC 8032) en Python puro, sin dependencias.

Ni `cryptography` ni `pynacl` están instalados, y agregarlos sería instalar software en la
máquina de otro para correr un gate. Además, una implementación en enteros de Python es
bit-idéntica en cualquier plataforma, que es justo lo que esta subfase necesita.

**No es constant-time y no debe usarse para claves de valor real.** Firma recibos de un
devnet; ver SPEC-RUNNER §6. La correctitud se verifica contra OpenSSL (vía Node) en
`scripts/check_ed25519.py`.
"""

import hashlib

_P = 2**255 - 19
_L = 2**252 + 27742317777372353535851937790883648493
_D = (-121665 * pow(121666, _P - 2, _P)) % _P
_SQRT_M1 = pow(2, (_P - 1) // 4, _P)


def _recover_x(y: int, sign: int):
    if y >= _P:
        return None
    xx = (y * y - 1) * pow(_D * y * y + 1, _P - 2, _P)
    x = pow(xx, (_P + 3) // 8, _P)
    if (x * x - xx) % _P != 0:
        x = (x * _SQRT_M1) % _P
    if (x * x - xx) % _P != 0:
        return None
    if x % 2 != sign:
        x = _P - x
    return x


_BY = (4 * pow(5, _P - 2, _P)) % _P
_BX = _recover_x(_BY, 0)
_B = (_BX, _BY, 1, (_BX * _BY) % _P)
_ZERO = (0, 1, 1, 0)


def _add(p, q):
    """Suma en coordenadas extendidas para la curva torcida con a = -1."""
    x1, y1, z1, t1 = p
    x2, y2, z2, t2 = q
    a = ((y1 - x1) * (y2 - x2)) % _P
    b = ((y1 + x1) * (y2 + x2)) % _P
    c = (2 * t1 * _D * t2) % _P
    d = (2 * z1 * z2) % _P
    e, f, g, h = b - a, d - c, d + c, b + a
    return ((e * f) % _P, (g * h) % _P, (f * g) % _P, (e * h) % _P)


def _mul(p, scalar: int):
    result = _ZERO
    base = p
    while scalar > 0:
        if scalar & 1:
            result = _add(result, base)
        base = _add(base, base)
        scalar >>= 1
    return result


def _encode(p) -> bytes:
    x, y, z, _ = p
    zi = pow(z, _P - 2, _P)
    x = (x * zi) % _P
    y = (y * zi) % _P
    return int.to_bytes(y | ((x & 1) << 255), 32, "little")


def _decode(data: bytes):
    if len(data) != 32:
        return None
    y = int.from_bytes(data, "little")
    sign = y >> 255
    y &= (1 << 255) - 1
    x = _recover_x(y, sign)
    if x is None:
        return None
    return (x, y, 1, (x * y) % _P)


def _expand(seed: bytes) -> tuple[int, bytes]:
    if len(seed) != 32:
        raise ValueError("la semilla Ed25519 son 32 bytes")
    h = hashlib.sha512(seed).digest()
    a = int.from_bytes(h[:32], "little")
    a &= (1 << 254) - 8
    a |= 1 << 254
    return a, h[32:]


def public_key(seed: bytes) -> bytes:
    a, _ = _expand(seed)
    return _encode(_mul(_B, a))


def sign(seed: bytes, message: bytes) -> bytes:
    a, prefix = _expand(seed)
    pub = _encode(_mul(_B, a))
    r = int.from_bytes(hashlib.sha512(prefix + message).digest(), "little") % _L
    big_r = _encode(_mul(_B, r))
    k = int.from_bytes(hashlib.sha512(big_r + pub + message).digest(), "little") % _L
    s = (r + k * a) % _L
    return big_r + int.to_bytes(s, 32, "little")


def verify(pub: bytes, message: bytes, signature: bytes) -> bool:
    if len(signature) != 64 or len(pub) != 32:
        return False
    big_r = _decode(signature[:32])
    point_a = _decode(pub)
    if big_r is None or point_a is None:
        return False
    s = int.from_bytes(signature[32:], "little")
    if s >= _L:
        return False
    k = int.from_bytes(hashlib.sha512(signature[:32] + pub + message).digest(), "little") % _L
    return _encode(_mul(_B, s)) == _encode(_add(big_r, _mul(point_a, k)))
