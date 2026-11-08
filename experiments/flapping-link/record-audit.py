#!/usr/bin/env python3
"""Check the flapping detector on encoded, incomplete, and malformed logs."""

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


def series(
    a: int,
    b: int,
    quiet: list[tuple[int, int]],
    *,
    recover_last: bool = True,
    live_during_quiet: bool = False,
    include_ingress: bool = True,
) -> bytes:
    data = b""
    for index, (start, end) in enumerate(quiet):
        data += link(4, a, b, start)
        if index + 1 < len(quiet) or recover_last:
            data += link(5, a, b, end)
    for at in range(20, 191, 10):
        silent = any(start <= at < end for start, end in quiet)
        data += probe(a, b, at, 0 if silent else 8)
        if include_ingress:
            data += ingress(a, b, at, 100 if silent and live_during_quiet else (0 if silent else 20))
    return data


def solve(data: bytes, name: str) -> bool:
    path = OUT / f"{name}.team"
    path.write_bytes(data)
    return subprocess.check_output([str(SIM), "detect", str(DETECT), str(path)], text=True).strip() == "true"


def both(quiet: list[tuple[int, int]], **kwargs: object) -> bytes:
    return series(1, 2, quiet, **kwargs) + series(2, 1, quiet, **kwargs)


def main() -> None:
    OUT.mkdir(exist_ok=True)
    positive = both([(50, 80), (120, 150)])
    cases = {
        "positive-two-flaps": (positive, True),
        "positive-three-flaps": (both([(30, 55), (80, 110), (140, 170)]), True),
        "one-flap": (both([(50, 90)]), False),
        "one-direction-only": (series(1, 2, [(50, 80), (120, 150)]), False),
        "second-never-recovers": (both([(50, 80), (120, 190)], recover_last=False), False),
        "live-port-traffic": (both([(50, 80), (120, 150)], live_during_quiet=True), False),
        "missing-port-observations": (both([(50, 80), (120, 150)], include_ingress=False), False),
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
