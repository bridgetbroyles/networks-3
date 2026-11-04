#!/usr/bin/env python3
"""Exact-record checks for the S6 synchronized-rate detector."""

from __future__ import annotations

import json
import struct
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parent
SIM = ROOT.parents[1] / "target/release/competitive_net_sim"
DETECT = ROOT / "generated/detect.wasm"
OUT = ROOT / "record-tests"
LEVELS = [194, 164, 210, 328, 328, 164, 328, 164]


def record(tag: int, body: bytes) -> bytes:
    return bytes([tag, len(body)]) + body


def stat(sw: int, customer: int, tick: int, packets: int, size: int = 1000) -> bytes:
    return record(2, struct.pack("<BBHII", sw, customer, tick,
                                 packets * size, packets))


def pair(left: list[int], right: list[int], *, sw: int = 6,
         size: int = 1000, reverse: bool = False) -> bytes:
    rows = []
    for i, (a, b) in enumerate(zip(left, right), start=1):
        rows += [stat(sw, 1, i * 15, a, size), stat(sw, 2, i * 15, b, size)]
    if reverse:
        rows.reverse()
    return b"".join(rows)


def solve(data: bytes, name: str) -> bool:
    path = OUT / f"{name}.team"
    path.write_bytes(data)
    return subprocess.check_output(
        [str(SIM), "detect", str(DETECT), str(path)], text=True
    ).strip() == "true"


def main() -> None:
    OUT.mkdir(exist_ok=True)
    mismatch = LEVELS.copy()
    mismatch[4] += 10
    cases = {
        "positive": (pair(LEVELS, LEVELS), True),
        "positive-out-of-order": (pair(LEVELS, LEVELS, reverse=True), True),
        "within-one-packet": (pair(LEVELS, [n + 1 for n in LEVELS]), True),
        "only-one-source": (b"".join(
            stat(6, 1, (i + 1) * 15, n) for i, n in enumerate(LEVELS)
        ), False),
        "constant-twins": (pair([200] * 8, [200] * 8), False),
        "two-level-twins": (pair([160, 300] * 4, [160, 300] * 4), False),
        "insufficient-variation": (pair([200, 220, 240] * 3, [200, 220, 240] * 3), False),
        "one-mismatched-window": (pair(LEVELS, mismatch), False),
        "only-five-windows": (pair(LEVELS[:5], LEVELS[:5]), False),
        "wrong-switch": (pair(LEVELS, LEVELS, sw=5), False),
        "wrong-size": (pair(LEVELS, LEVELS, size=500), False),
        "truncated": (pair(LEVELS, LEVELS) + bytes([2, 12, 6]), False),
        "malformed": (bytes([2, 12, 6]), False),
        "empty": (b"", False),
    }
    results = []
    for name, (data, expected) in cases.items():
        actual = solve(data, name)
        row = {"case": name, "expected": expected, "actual": actual,
               "pass": actual == expected}
        results.append(row)
        if not row["pass"]:
            raise AssertionError(row)
    (ROOT / "record-results.json").write_text(json.dumps(results, indent=2) + "\n")
    print(f"PASS {len(results)} record checks")


if __name__ == "__main__":
    main()
