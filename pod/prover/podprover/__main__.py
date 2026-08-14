"""CLI del runner — SPEC-RUNNER.md §5.

    python -m podprover keygen --out clave.hex
    python -m podprover run    --order O.json --dataset D.jsonl... --key clave.hex --out r.json
    python -m podprover replay --order O.json --dataset D.jsonl... --receipt r.json

`run` imprime el `output_hash` en stdout y nada más: es lo que compara el gate.
`replay` sale 0 si el hash recalculado coincide, 3 si no.
"""

import argparse
import pathlib
import sys
import time
import tracemalloc

import podspec

from . import dataset, ed25519, receipt, sweep


def _load_order(path: pathlib.Path) -> tuple[dict, str]:
    raw = path.read_bytes()
    order = podspec.parse(raw)
    podspec.validate(order)
    return order, podspec.spec_hash(order)


def _load_tokens(order: dict, paths: list[pathlib.Path]):
    data = dataset.dataset_bytes(paths)
    return dataset.load(data, order["inputs"]["dataset"]["hash"])


def _execute(order: dict, spec_hash: str, tokens) -> dict:
    return sweep.run(order, tokens, spec_hash)


def cmd_keygen(args) -> int:
    # `os.urandom` a traves de secrets: la clave del worker no entra en ningun hash.
    import secrets

    seed = secrets.token_bytes(32)
    out = pathlib.Path(args.out)
    out.write_text(seed.hex() + "\n", encoding="utf-8")
    sys.stderr.write(f"runner_id {ed25519.public_key(seed).hex()}\n")
    return 0


def cmd_run(args) -> int:
    order, spec_hash = _load_order(pathlib.Path(args.order))
    tokens = _load_tokens(order, [pathlib.Path(p) for p in args.dataset])

    started = int(time.time() * 1000)
    tracemalloc.start()
    t0 = time.perf_counter()
    output = _execute(order, spec_hash, tokens)
    wall_ms = int((time.perf_counter() - t0) * 1000)
    peak_bytes = tracemalloc.get_traced_memory()[1]
    tracemalloc.stop()

    if args.key:
        seed = bytes.fromhex(pathlib.Path(args.key).read_text(encoding="utf-8").strip())
        rec = receipt.build(seed, spec_hash, output, started, wall_ms, peak_bytes)
        if args.out:
            pathlib.Path(args.out).write_bytes(podspec.canonical_bytes(rec))
        sys.stderr.write(f"wall_ms {wall_ms}  peak_bytes {peak_bytes}\n")
        sys.stdout.write(rec["output_hash"])
    else:
        sys.stdout.write(receipt.output_hash(output))
    return 0


def cmd_replay(args) -> int:
    order, spec_hash = _load_order(pathlib.Path(args.order))
    rec = podspec.parse(pathlib.Path(args.receipt).read_bytes())

    problems = receipt.check(rec)
    if rec.get("spec_hash") != spec_hash:
        problems.append(
            f"el recibo es de otro pedido: {rec.get('spec_hash')} != {spec_hash}"
        )
    for problem in problems:
        sys.stderr.write(f"RECIBO  {problem}\n")
    if problems:
        return 3

    tokens = _load_tokens(order, [pathlib.Path(p) for p in args.dataset])
    recomputed = receipt.output_hash(_execute(order, spec_hash, tokens))
    if recomputed != rec["output_hash"]:
        sys.stderr.write(f"REPLAY  recalculado {recomputed} != recibo {rec['output_hash']}\n")
        return 3
    sys.stdout.write(recomputed)
    return 0


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(prog="podprover", add_help=True)
    sub = parser.add_subparsers(dest="cmd", required=True)

    p = sub.add_parser("keygen")
    p.add_argument("--out", required=True)
    p.set_defaults(fn=cmd_keygen)

    p = sub.add_parser("run")
    p.add_argument("--order", required=True)
    p.add_argument("--dataset", required=True, nargs="+")
    p.add_argument("--key")
    p.add_argument("--out")
    p.set_defaults(fn=cmd_run)

    p = sub.add_parser("replay")
    p.add_argument("--order", required=True)
    p.add_argument("--dataset", required=True, nargs="+")
    p.add_argument("--receipt", required=True)
    p.set_defaults(fn=cmd_replay)

    args = parser.parse_args(argv[1:])
    try:
        return args.fn(args)
    except (podspec.SpecError, dataset.DatasetError) as exc:
        sys.stderr.write(f"ERROR  {exc}\n")
        return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
