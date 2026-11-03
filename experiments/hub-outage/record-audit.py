#!/usr/bin/env python3
"""Check detector behavior on encoded, incomplete, and malformed logs."""

from __future__ import annotations

import json
import struct
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parent
SIM = ROOT.parents[1] / "target/release/competitive_net_sim"
DETECT = ROOT / "generated/detect.wasm"
OUT = ROOT / "record-tests"


def record(tag: int, body: bytes) -> bytes:
    return bytes([tag, len(body)]) + body


def link(tag: int, a: int, b: int, at: int) -> bytes:
    return record(tag, struct.pack("<BBHH", a, b, 2, at))


def probe(a: int, b: int, at: int, count: int) -> bytes:
    return record(11, struct.pack("<BBHHHHIIHII", a, b, at, count, 0, at, 2552, 2552, 0, 0, 0))


def ingress(a: int, b: int, at: int, count: int) -> bytes:
    return record(13, struct.pack("<BBHI", a, b, at, count))


def series(a: int, b: int, quiet: tuple[int, int], recover: bool = True, live: bool = False) -> bytes:
    data = link(4, a, b, 100)
    if recover:
        data += link(5, a, b, 160)
    for at in range(20, 201, 10):
        silent = quiet[0] <= at <= quiet[1]
        data += probe(a, b, at, 0 if silent else 8)
        data += ingress(a, b, at, 100 if live and silent else (0 if silent else 20))
    return data


def solve(data: bytes, name: str) -> bool:
    path = OUT / f"{name}.team"
    path.write_bytes(data)
    return subprocess.check_output([str(SIM), "detect", str(DETECT), str(path)], text=True).strip() == "true"


def main() -> None:
    OUT.mkdir(exist_ok=True)
    positive = b"".join(series(a, 5, (80, 155)) for a in (0, 6, 10))
    cases = {
        "positive": (positive, True),
        "too-short": (b"".join(series(a, 5, (120, 145)) for a in (0, 6, 10)), False),
        "only-two-links": (b"".join(series(a, 5, (80, 155)) for a in (0, 6)), False),
        "no-recovery": (b"".join(series(a, 5, (80, 200), recover=False) for a in (0, 6, 10)), False),
        "live-port-traffic": (b"".join(series(a, 5, (80, 155), live=True) for a in (0, 6, 10)), False),
        "missing-port-observations": (b"".join(
            chunk for chunk in _chunks(positive) if chunk[0] != 13
        ), False),
        "empty": (b"", False),
        "malformed": (bytes([11, 28, 0]), False),
        "truncated": (positive + bytes([11, 28, 0]), False),
    }
    results = []
    for name, (data, expected) in cases.items():
        actual = solve(data, name)
        row = {"case": name, "expected": expected, "actual": actual, "pass": expected == actual}
        results.append(row)
        if not row["pass"]:
            raise AssertionError(row)
    (ROOT / "record-results.json").write_text(json.dumps(results, indent=2) + "\n")
    print(f"PASS {len(results)} record checks")


def _chunks(data: bytes):
    at = 0
    while at + 2 <= len(data):
        size = data[at + 1]
        end = at + 2 + size
        yield data[at:end]
        at = end


if __name__ == "__main__":
    main()
