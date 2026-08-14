"""Evidencia del gate de la subfase 1.2 — ..._FASE1.md §3.

    python scripts/gate.py

El gate: un tercero, en otra máquina y otro sistema operativo, con el pedido, el dataset y
el recibo, obtiene el mismo `output_hash`.

Este script corre las dos plataformas que puede alcanzar (Windows y Ubuntu bajo WSL2) y
compara contra `EXPECTED_OUTPUT.tsv`. La tercera —Android/ARM64 bajo Termux, que es la que
cierra el eje de arquitectura— se corrio a mano y esta asentada en `PLATFORMS.tsv`.
"""

import json
import os
import pathlib
import random
import shutil
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

from podprover import receipt as receipt_mod  # noqa: E402

import podspec  # noqa: E402

DATA_WIN = ROOT.parent.parent.parent / "memebot" / "data"
FILES = ["ticks-2026-06-10.jsonl", "ticks-2026-06-15.jsonl"]
ORDERS = ["order-graduacion.json", "order-sniper.json", "order-survivor.json"]
FAST_ORDER = "order-graduacion.json"

WSL_DISTRO = "Ubuntu-24.04"
WSL_ROOT = "/mnt/c/Users/Cristian/Documents/apps/Chain/pod/prover"
WSL_DATA = "/mnt/c/Users/Cristian/Documents/apps/memebot/data"

SCRATCH = ROOT / "fidelity" / ".tmp"
failures: list[str] = []


def head(title: str) -> None:
    print(f"\n{'=' * 74}\n{title}\n{'=' * 74}")


def check(ok: bool, label: str) -> None:
    print(f"  [{'ok ' if ok else 'FALLA'}] {label}")
    if not ok:
        failures.append(label)


def win_run(order: str, env_extra: dict | None = None, cwd: pathlib.Path | None = None):
    env = {**os.environ, **(env_extra or {})}
    args = [
        sys.executable,
        "-m",
        "podprover",
        "run",
        "--order",
        str(ROOT / "orders" / order),
        "--dataset",
        *[str(DATA_WIN / f) for f in FILES],
    ]
    return subprocess.run(
        args, capture_output=True, env=env, cwd=str(cwd or ROOT), check=False
    )


def wsl_run(command: str):
    return subprocess.run(
        ["wsl", "-d", WSL_DISTRO, "--", "bash", "-lc", command],
        capture_output=True,
        check=False,
    )


def wsl_out(proc) -> str:
    return proc.stdout.decode("utf-8", "replace").replace("\x00", "").strip()


# ------------------------------------------------------------------------------ 1
def cross_os() -> dict:
    head("1. GATE — mismo output_hash en otro sistema operativo y otro interprete")
    print(f"  A: Windows  CPython {sys.version.split()[0]}")
    ver = wsl_out(wsl_run("python3 --version"))
    distro = wsl_out(wsl_run("grep PRETTY_NAME /etc/os-release | cut -d= -f2 | tr -d '\"'"))
    print(f"  B: {distro}  (WSL2)  {ver}")

    frozen = {}
    for line in (ROOT / "EXPECTED_OUTPUT.tsv").read_text(encoding="utf-8").splitlines():
        if line and not line.startswith("#"):
            name, value = line.split("\t")
            frozen[name] = value

    hashes = {}
    for order in ORDERS:
        a = win_run(order)
        hash_a = a.stdout.decode().strip()
        cmd = (
            f"cd {WSL_ROOT} && python3 -m podprover run --order orders/{order} "
            f"--dataset " + " ".join(f"{WSL_DATA}/{f}" for f in FILES)
        )
        hash_b = wsl_out(wsl_run(cmd))
        hashes[order] = hash_a
        same = hash_a == hash_b == frozen[order]
        print(f"    {order:26s} A={hash_a[:20]}… B={hash_b[:20]}…")
        check(same, f"{order}: los dos sistemas dan el hash congelado")
    return hashes


# ------------------------------------------------------------------------------ 2
def replay_from_receipt(hashes: dict) -> None:
    head("2. REPLAY — el tercero verifica con el pedido, el dataset y el recibo")
    SCRATCH.mkdir(parents=True, exist_ok=True)
    key = SCRATCH / "runner.key"
    rec_path = SCRATCH / "receipt.json"

    subprocess.run(
        [sys.executable, "-m", "podprover", "keygen", "--out", str(key)],
        capture_output=True,
        cwd=str(ROOT),
        check=True,
    )
    proc = subprocess.run(
        [
            sys.executable, "-m", "podprover", "run",
            "--order", str(ROOT / "orders" / FAST_ORDER),
            "--dataset", *[str(DATA_WIN / f) for f in FILES],
            "--key", str(key), "--out", str(rec_path),
        ],
        capture_output=True,
        cwd=str(ROOT),
        check=False,
    )
    print(f"  worker firma el recibo  ({proc.stderr.decode().strip()})")
    rec = podspec.parse(rec_path.read_bytes())
    print(f"  runner_id   {rec['runner_id']}")
    print(f"  output_hash {rec['output_hash']}")
    check(not receipt_mod.check(rec), "firma valida y el output del recibo hashea a su output_hash")
    check(rec["output_hash"] == hashes[FAST_ORDER], "el recibo trae el hash que dio la corrida")

    cmd = (
        f"cd {WSL_ROOT} && python3 -m podprover replay --order orders/{FAST_ORDER} "
        f"--dataset " + " ".join(f"{WSL_DATA}/{f}" for f in FILES)
        + f" --receipt fidelity/.tmp/receipt.json"
    )
    proc = wsl_run(cmd)
    check(
        proc.returncode == 0 and wsl_out(proc) == rec["output_hash"],
        "replay en el otro sistema operativo sale 0 y reproduce el hash del recibo",
    )


# ------------------------------------------------------------------------------ 3
def tampering() -> None:
    head("3. MANIPULACION — un recibo alterado tiene que fallar")
    original = podspec.parse((SCRATCH / "receipt.json").read_bytes())

    def replay_local(rec: dict) -> int:
        path = SCRATCH / "tampered.json"
        path.write_bytes(podspec.canonical_bytes(rec))
        proc = subprocess.run(
            [
                sys.executable, "-m", "podprover", "replay",
                "--order", str(ROOT / "orders" / FAST_ORDER),
                "--dataset", *[str(DATA_WIN / f) for f in FILES],
                "--receipt", str(path),
            ],
            capture_output=True,
            cwd=str(ROOT),
            check=False,
        )
        return proc.returncode

    check(replay_local(original) == 0, "el recibo intacto pasa el replay")

    inflated = json.loads(json.dumps(original))
    inflated["output"]["rows"][0]["metrics"]["net_lamports"] += 1
    check(replay_local(inflated) == 3, "cambiar una metrica del output rompe el replay")

    rehashed = json.loads(json.dumps(inflated))
    rehashed["output_hash"] = receipt_mod.output_hash(rehashed["output"])
    check(
        replay_local(rehashed) == 3,
        "recalcular el output_hash no alcanza: la firma deja de verificar",
    )

    other_order = json.loads(json.dumps(original))
    other_order["spec_hash"] = "0" * 64
    check(replay_local(other_order) == 3, "un recibo de otro pedido se rechaza")

    # Dataset equivocado: el runner tiene que abortar, no ejecutar igual.
    proc = subprocess.run(
        [
            sys.executable, "-m", "podprover", "run",
            "--order", str(ROOT / "orders" / FAST_ORDER),
            "--dataset", str(DATA_WIN / FILES[0]),
        ],
        capture_output=True,
        cwd=str(ROOT),
        check=False,
    )
    check(proc.returncode == 2, "con un dataset que no es el del pedido, el runner aborta")


# ------------------------------------------------------------------------------ 4
def environment(hashes: dict) -> None:
    head("4. ENTORNO — el hash no depende de nada del ambiente")
    expected = hashes[FAST_ORDER]
    variants = [
        ("PYTHONHASHSEED aleatorio", {"PYTHONHASHSEED": str(random.randint(0, 2**32 - 1))}, None),
        ("TZ=Pacific/Kiritimati", {"TZ": "Pacific/Kiritimati"}, None),
        ("LC_ALL=tr_TR.UTF-8", {"LC_ALL": "tr_TR.UTF-8", "LANG": "tr_TR.UTF-8"}, None),
        ("PYTHONUTF8=0", {"PYTHONUTF8": "0"}, None),
        ("otro directorio de trabajo", {}, ROOT.parent),
    ]
    for label, env_extra, cwd in variants:
        env = {**env_extra}
        if cwd is not None:
            env["PYTHONPATH"] = str(ROOT)
        got = win_run(FAST_ORDER, env, cwd).stdout.decode().strip()
        check(got == expected, f"{label} -> mismo hash")

    # Orden de los archivos en la linea de comandos: la concatenacion es por nombre (§2.1.1).
    args = [
        sys.executable, "-m", "podprover", "run",
        "--order", str(ROOT / "orders" / FAST_ORDER),
        "--dataset", *[str(DATA_WIN / f) for f in reversed(FILES)],
    ]
    got = subprocess.run(args, capture_output=True, cwd=str(ROOT), check=False)
    check(got.stdout.decode().strip() == expected, "invertir el orden de los archivos -> mismo hash")


# ------------------------------------------------------------------------------ 5
def delegated() -> None:
    head("5. CHEQUEOS DELEGADOS")
    for script, label in (
        ("check_ed25519.py", "Ed25519 propio contra OpenSSL"),
        ("check_fidelity.py", "fidelidad contra el simulador original de backtest.mjs"),
    ):
        proc = subprocess.run(
            [sys.executable, str(ROOT / "scripts" / script)],
            capture_output=True,
            cwd=str(ROOT),
            check=False,
        )
        tail = [ln for ln in proc.stdout.decode("utf-8", "replace").splitlines() if ln.strip()]
        print(f"  {label}: {tail[-1] if tail else '(sin salida)'}")
        check(proc.returncode == 0, label)


def main() -> int:
    if shutil.which("wsl") is None:
        print("no hay `wsl`: el gate cross-OS no se puede correr en esta maquina")
        return 1
    hashes = cross_os()
    replay_from_receipt(hashes)
    tampering()
    environment(hashes)
    delegated()

    head("6. PLATAFORMAS QUE REPRODUJERON LOS HASHES CONGELADOS")
    print("  Las dos primeras las acaba de correr este script. La tercera se corrio a mano")
    print("  en un telefono: el script no puede alcanzarla, y por eso queda asentada.")
    for line in (ROOT / "PLATFORMS.tsv").read_text(encoding="utf-8").splitlines():
        if line and not line.startswith("#"):
            name, arch, system, interp, date, result = line.split("\t")
            print(f"    {name:16s} {arch:9s} {system:22s} {interp:16s} {date}  {result}")

    print(f"\n{'=' * 74}")
    if failures:
        print(f"RESULTADO: {len(failures)} falla(s)")
        for f in failures:
            print(f"  - {f}")
        return 1
    print("RESULTADO: todo pasa. Gate cumplido en las tres plataformas de PLATFORMS.tsv")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
