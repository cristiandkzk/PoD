"""Verifica la implementación pura de Ed25519 contra OpenSSL, vía Node.

No alcanza con firmar y verificar contra uno mismo: una implementación consistentemente
equivocada pasa esa prueba. Se compara clave pública y firma, byte a byte, contra las que
produce OpenSSL para la misma semilla y el mismo mensaje.
"""

import json
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

from podprover import ed25519  # noqa: E402

# Semillas y mensajes fijos: si alguna vez cambian, cambia la evidencia.
CASES = [
    ("00" * 32, ""),
    ("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60", ""),
    ("4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb", "72"),
    ("f5e5767cf153319517630f226876b86c8160cc583bc013744c6bf255f5cc0ee5", "af82c0ffee"),
    ("833fe62409237b9d62ec77587520911e9a759cec1d19755b7da901b96dca3d42", "0102030405060708"),
]

NODE = r"""
const crypto = require('node:crypto');
const cases = JSON.parse(process.argv[1]);
const PKCS8 = Buffer.from('302e020100300506032b657004220420', 'hex');
const out = cases.map(([seedHex, msgHex]) => {
  const der = Buffer.concat([PKCS8, Buffer.from(seedHex, 'hex')]);
  const priv = crypto.createPrivateKey({ key: der, format: 'der', type: 'pkcs8' });
  const spki = crypto.createPublicKey(priv).export({ format: 'der', type: 'spki' });
  const pub = spki.subarray(spki.length - 32);
  const sig = crypto.sign(null, Buffer.from(msgHex, 'hex'), priv);
  return { pub: pub.toString('hex'), sig: sig.toString('hex') };
});
process.stdout.write(JSON.stringify(out));
"""


def main() -> int:
    proc = subprocess.run(
        ["node", "-e", NODE, json.dumps(CASES)],
        capture_output=True,
        check=False,
    )
    if proc.returncode != 0:
        print("no se pudo correr Node como oraculo:", proc.stderr.decode()[:400])
        return 1
    expected = json.loads(proc.stdout.decode())

    failures = 0
    for (seed_hex, msg_hex), want in zip(CASES, expected):
        seed = bytes.fromhex(seed_hex)
        msg = bytes.fromhex(msg_hex)
        pub = ed25519.public_key(seed).hex()
        sig = ed25519.sign(seed, msg).hex()
        ok_pub = pub == want["pub"]
        ok_sig = sig == want["sig"]
        ok_ver = ed25519.verify(bytes.fromhex(want["pub"]), msg, bytes.fromhex(want["sig"]))
        tampered = bytearray(bytes.fromhex(want["sig"]))
        tampered[0] ^= 1
        ok_rej = not ed25519.verify(bytes.fromhex(want["pub"]), msg, bytes(tampered))
        good = ok_pub and ok_sig and ok_ver and ok_rej
        failures += 0 if good else 1
        print(
            f"  [{'ok ' if good else 'FALLA'}] semilla {seed_hex[:16]}… msg={len(msg)}B  "
            f"pub={'=' if ok_pub else 'X'} firma={'=' if ok_sig else 'X'} "
            f"verifica={'si' if ok_ver else 'NO'} rechaza-alterada={'si' if ok_rej else 'NO'}"
        )

    print(f"\n{len(CASES) - failures}/{len(CASES)} casos coinciden con OpenSSL")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
