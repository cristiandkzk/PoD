"""Evidencia de los cuatro gates de la subfase 1.1 — ..._FASE1.md §2.

    python scripts/gate.py

Sale 0 si los cuatro pasan. No imprime opiniones: imprime comandos, conteos y hashes.
"""

import pathlib
import random
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "python"))

from podspec import canonical_bytes, parse, spec_hash, validate  # noqa: E402
from podspec.errors import SpecError  # noqa: E402

RUST_BIN = ROOT / "rust" / "target" / "release" / "podspec.exe"
if not RUST_BIN.exists():
    RUST_BIN = ROOT / "rust" / "target" / "release" / "podspec"

VALID = sorted((ROOT / "testvectors" / "valid").glob("*.json"))
REJECT = sorted((ROOT / "testvectors" / "reject").glob("*.json"))

# Las cinco formas de bytes que DEBEN colapsar al mismo hash (gate 1).
SAME_HASH = [
    "01_survivor_indent.json",
    "02_survivor_sorted_compact.json",
    "03_survivor_reversed.json",
    "04_survivor_rotated_tabs.json",
    "05_survivor_escaped.json",
]

failures: list[str] = []


def head(n: int, title: str) -> None:
    print(f"\n{'=' * 72}\nGATE {n} — {title}\n{'=' * 72}")


def py_hash(path: pathlib.Path) -> str:
    value = parse(path.read_bytes())
    validate(value)
    return spec_hash(value)


def rust(args: list[str]) -> subprocess.CompletedProcess:
    return subprocess.run(
        [str(RUST_BIN), *args], capture_output=True, timeout=60, check=False
    )


def check(condition: bool, label: str) -> None:
    print(f"  [{'ok ' if condition else 'FALLA'}] {label}")
    if not condition:
        failures.append(label)


# ---------------------------------------------------------------------------- gate 1
def gate1() -> None:
    head(1, "100 corridas y 3+ permutaciones de orden de claves -> hash identico")
    paths = [ROOT / "testvectors" / "valid" / n for n in SAME_HASH]
    for p in paths:
        print(f"  forma: {p.name:30s} {len(p.read_bytes()):5d} bytes de entrada")
    distintos_bytes = {p.read_bytes() for p in paths}
    check(len(distintos_bytes) == len(paths), f"las {len(paths)} formas son bytes distintos")

    hashes = set()
    for _ in range(100):
        for p in paths:
            hashes.add(py_hash(p))
    print(f"  python: {100 * len(paths)} evaluaciones -> {len(hashes)} hash distinto(s)")
    check(len(hashes) == 1, "python colapsa las 5 formas a un unico hash en 100 corridas")

    # Procesos separados con PYTHONHASHSEED aleatorio: es lo que detecta dependencia del
    # orden de iteracion de un hash map, y no se ve corriendo todo en un solo proceso.
    seeds = [random.randint(0, 4294967295) for _ in range(10)]
    proc_hashes = set()
    for seed in seeds:
        out = subprocess.run(
            [sys.executable, "-m", "podspec", "hash", str(paths[0])],
            capture_output=True,
            cwd=str(ROOT / "python"),
            env={**__import__("os").environ, "PYTHONHASHSEED": str(seed)},
            check=True,
        )
        proc_hashes.add(out.stdout.decode())
    print(f"  python: 10 procesos con PYTHONHASHSEED aleatorio -> {len(proc_hashes)} hash(es)")
    check(len(proc_hashes) == 1, "el hash no depende de PYTHONHASHSEED")

    rust_hashes = set()
    for _ in range(20):
        for p in paths:
            r = rust(["hash", str(p)])
            rust_hashes.add(r.stdout.decode())
    print(f"  rust:   {20 * len(paths)} evaluaciones -> {len(rust_hashes)} hash distinto(s)")
    check(len(rust_hashes) == 1, "rust colapsa las 5 formas a un unico hash")
    check(hashes == rust_hashes, "python y rust coinciden en ese hash")
    print(f"  hash comun: {next(iter(hashes))}")


# ---------------------------------------------------------------------------- gate 2
def gate2() -> None:
    head(2, "un cambio semantico minimo -> hash distinto")
    base = ROOT / "testvectors" / "valid" / "01_survivor_indent.json"
    changed = ROOT / "testvectors" / "valid" / "06_survivor_grid_changed.json"
    hb, hc = py_hash(base), py_hash(changed)
    print("  cambio: inputs.strategy.params.min_buyers  [25,40,60] -> [25,40,61]")
    print(f"  {base.name:30s} {hb}")
    print(f"  {changed.name:30s} {hc}")
    check(hb != hc, "un solo parametro cambiado produce otro hash")

    todos = {p.name: py_hash(p) for p in VALID}
    unicos = set(todos.values())
    esperados = len(VALID) - len(SAME_HASH) + 1
    print(f"  {len(VALID)} vectores validos -> {len(unicos)} hashes distintos (esperado {esperados})")
    check(len(unicos) == esperados, "no hay colisiones entre vectores semanticamente distintos")


# ---------------------------------------------------------------------------- gate 3
def gate3() -> None:
    head(3, "segunda implementacion (Rust) reproduce los vectores bit a bit")
    print(f"  binario: {RUST_BIN}")
    expected = {}
    for line in (ROOT / "testvectors" / "EXPECTED.tsv").read_text(encoding="utf-8").splitlines():
        if line and not line.startswith("#"):
            name, h = line.split("\t")
            expected[name] = h

    ok_hash = ok_bytes = 0
    for p in VALID:
        r = rust(["hash", str(p)])
        got = r.stdout.decode()
        py = py_hash(p)
        if got == py == expected[p.name] and r.returncode == 0:
            ok_hash += 1
        else:
            failures.append(f"hash {p.name}: rust={got} py={py} congelado={expected[p.name]}")
            print(f"  [FALLA] {p.name}: rust={got} py={py}")
        rc = rust(["canon", str(p)])
        if rc.stdout == canonical_bytes(parse(p.read_bytes())):
            ok_bytes += 1
        else:
            failures.append(f"bytes canonicos {p.name}")
            print(f"  [FALLA] bytes canonicos difieren en {p.name}")
    check(ok_hash == len(VALID), f"{ok_hash}/{len(VALID)} hashes coinciden con el TSV congelado")
    check(ok_bytes == len(VALID), f"{ok_bytes}/{len(VALID)} bytes canonicos identicos byte a byte")

    exp_rej = {}
    for line in (
        (ROOT / "testvectors" / "EXPECTED_REJECT.tsv").read_text(encoding="utf-8").splitlines()
    ):
        if line and not line.startswith("#"):
            name, code = line.split("\t")
            exp_rej[name] = code

    ok_code = paths_iguales = 0
    for p in REJECT:
        r = rust(["hash", str(p)])
        rust_code, _, rust_path = r.stderr.decode().strip().partition("\t")
        try:
            value = parse(p.read_bytes())
            validate(value)
            py_code, py_path = "NO_RECHAZADO", ""
        except SpecError as exc:
            py_code, py_path = exc.code, exc.path
        if rust_code == py_code == exp_rej[p.name] and r.returncode == 2:
            ok_code += 1
        else:
            failures.append(f"codigo {p.name}: rust={rust_code} py={py_code}")
            print(f"  [FALLA] {p.name}: rust={rust_code} py={py_code} congelado={exp_rej[p.name]}")
        if rust_path == py_path:
            paths_iguales += 1
    check(ok_code == len(REJECT), f"{ok_code}/{len(REJECT)} codigos de rechazo coinciden")
    print(
        f"  (informativo, no normativo) paths de diagnostico iguales: "
        f"{paths_iguales}/{len(REJECT)}"
    )


# ---------------------------------------------------------------------------- gate 4
def gate4() -> None:
    head(4, "un backtest real se expresa completo, sin campos de texto libre")
    p = ROOT / "testvectors" / "valid" / "01_survivor_indent.json"
    value = parse(p.read_bytes())
    validate(value)
    print(f"  pedido: {p.name}  ->  {spec_hash(value)}")

    # Todo string del pedido, clasificado por como lo restringe el esquema.
    registro_cerrado = {
        "class", "format", "kind", "rounding", "metrics",
    }
    patron = {"hash", "commit", "image_digest", "mint", "verifier_key", "toolchain",
              "tp_mult", "min_mcap", "min_growth", "min_ratio"}
    sin_restriccion = {"image_ref", "entrypoint"}

    encontrados: dict[str, list[str]] = {"registro": [], "patron": [], "libre": []}

    def walk(node, key=None):
        if isinstance(node, dict):
            for k, v in node.items():
                walk(v, k)
        elif isinstance(node, list):
            for item in node:
                walk(item, key)
        elif isinstance(node, str):
            if key in registro_cerrado:
                encontrados["registro"].append(f"{key}={node}")
            elif key in patron:
                encontrados["patron"].append(f"{key}")
            elif key in sin_restriccion:
                encontrados["libre"].append(f"{key}={node}")
            else:
                encontrados.setdefault("desconocido", []).append(f"{key}={node}")

    walk(value)
    print(f"  strings de registro cerrado : {len(encontrados['registro'])}")
    for s in encontrados["registro"]:
        print(f"      {s}")
    print(f"  strings con patron fijo     : {len(encontrados['patron'])}")
    print(f"  strings sin registro/patron : {len(encontrados['libre'])}")
    for s in encontrados["libre"]:
        print(f"      {s}   (acotado por el charset de SPEC §3.2 y por image_digest)")
    check(
        not encontrados.get("desconocido"),
        "ningun string del pedido escapa a la clasificacion del esquema",
    )
    check(
        len(encontrados["libre"]) <= 4,
        "los unicos strings no enumerados son la identidad del runner (image_ref/entrypoint)",
    )


def main() -> int:
    gate1()
    gate2()
    gate3()
    gate4()
    print(f"\n{'=' * 72}")
    if failures:
        print(f"RESULTADO: {len(failures)} falla(s)")
        for f in failures:
            print(f"  - {f}")
        return 1
    print("RESULTADO: los cuatro gates pasan")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
