#!/usr/bin/env python3
"""Check capacity detector behavior on encoded and malformed logs."""

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


def probe(a: int, b: int, at: int, pairs: int, delta_us: int) -> bytes:
    return record(11, struct.pack("<BBHHHHIIHII", a, b, at, 8, 0, at, 2552, 2552, pairs, delta_us, delta_us))


def sequence(slow_delta: int | None, reverse: bool = False) -> bytes:
    a, b = (7, 6) if reverse else (6, 7)
    data = b""
    for at in range(20, 201, 5):
        delta = 640 if slow_delta is None or at < 80 or at > 150 else slow_delta
        data += probe(a, b, at, 4, delta)
    return data


def solve(data: bytes, name: str) -> bool:
    path = OUT / f"{name}.team"
    path.write_bytes(data)
    return subprocess.check_output([str(SIM), "detect", str(DETECT), str(path)], text=True).strip() == "true"


def main() -> None:
    OUT.mkdir(exist_ok=True)
    positive = sequence(1600) + sequence(1600, reverse=True)
    cases = {
        "positive": (positive, True),
        "normal": (sequence(None) + sequence(None, reverse=True), False),
        "near-8mbps": (sequence(800) + sequence(800, reverse=True), False),
        "constant-slow": (b"".join(probe(6, 7, at, 4, 1600) for at in range(20, 201, 5)), False),
        "one-slow-sample": (sequence(None) + probe(6, 7, 100, 4, 1600), False),
        "wrong-edge": (b"".join(probe(12, 13, at, 4, 1600 if 80 <= at <= 150 else 640) for at in range(20, 201, 5)), False),
        "missing-pairs": (b"".join(probe(6, 7, at, 0, 0) for at in range(20, 201, 5)), False),
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


if __name__ == "__main__":
    main()
