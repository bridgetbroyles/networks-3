#!/usr/bin/env python3
"""Exact-record checks for the S1 synchronized late-start detector."""

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


def stat(sw: int, customer: int, tick: int, packets: int, size: int = 1000) -> bytes:
    return record(2, struct.pack("<BBHII", sw, customer, tick,
                                 packets * size, packets))


def rows(*values: tuple[int, int, int, int, int]) -> bytes:
    return b"".join(stat(*value) for value in values)


def solve(data: bytes, name: str) -> bool:
    path = OUT / f"{name}.team"
    path.write_bytes(data)
    return subprocess.check_output(
        [str(SIM), "detect", str(DETECT), str(path)], text=True
    ).strip() == "true"


def main() -> None:
    OUT.mkdir(exist_ok=True)
    truth_pair = rows((1, 1, 306, 548, 1000), (1, 2, 336, 61, 1000),
                      (1, 3, 336, 55, 1000))
    cases = {
        "truth-shaped-positive": (truth_pair, True),
        "positive-extra-later-records": (
            truth_pair + rows((1, 2, 351, 60, 1000), (1, 3, 351, 56, 1000)), True),
        "one-source": (rows((1, 1, 336, 61, 1000)), False),
        "staggered": (rows((1, 1, 320, 61, 1000), (1, 2, 340, 55, 1000)), False),
        "unequal-rates": (rows((1, 1, 336, 61, 1000), (1, 2, 336, 30, 1000)), False),
        "two-high-rate-sources": (rows((1, 1, 336, 300, 1000), (1, 2, 336, 310, 1000)), False),
        "startup-twins": (rows((1, 1, 20, 61, 1000), (1, 2, 20, 55, 1000)), False),
        "wrong-switch": (rows((2, 1, 336, 61, 1000), (2, 2, 336, 55, 1000)), False),
        "wrong-size": (rows((1, 1, 336, 61, 500), (1, 2, 336, 55, 500)), False),
        "zero-count": (rows((1, 1, 336, 0, 1000), (1, 2, 336, 0, 1000)), False),
        "truncated": (truth_pair + bytes([2, 12, 1]), False),
        "malformed": (bytes([2, 12, 1]), False),
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
