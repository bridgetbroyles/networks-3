#!/usr/bin/env python3
"""Exercise the Ledger-pause detector on exact and malformed records."""

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


def stat(sw: int, customer: int, at: int, byte_count: int, packets: int, size: int = 1000) -> bytes:
    if packets and not byte_count:
        byte_count = packets * size
    return record(2, struct.pack("<BBHII", sw, customer, at, byte_count, packets))


def heartbeat(sw: int, at: int) -> bytes:
    return record(10, struct.pack("<BH", sw, at))


def series(
    *, sw: int = 9, customer: int = 23, zero_times: set[int] | None = None,
    heartbeats: bool = True, byte_count: int = 210_000, size: int = 1000,
    stop_after_zero: bool = False,
) -> bytes:
    zero_times = zero_times or set()
    out = bytearray()
    for at in range(20, 291, 15):
        if stop_after_zero and at > max(zero_times, default=0):
            break
        if heartbeats:
            out += heartbeat(sw, at)
        if at in zero_times:
            out += stat(sw, customer, at, 0, 0, size)
        else:
            packets = max(1, byte_count // size)
            out += stat(sw, customer, at, packets * size, packets, size)
    return bytes(out)


def solve(data: bytes, name: str) -> bool:
    path = OUT / f"{name}.team"
    path.write_bytes(data)
    return subprocess.check_output([str(SIM), "detect", str(DETECT), str(path)], text=True).strip() == "true"


def main() -> None:
    OUT.mkdir(exist_ok=True)
    long_zeros = set(range(95, 216, 15))
    short_zeros = set(range(95, 141, 15))
    boundary_zeros = set(range(95, 186, 15))
    cases = {
        "positive": (series(zero_times=long_zeros), True),
        "normal": (series(), False),
        "short-pause": (series(zero_times=short_zeros), False),
        "twelve-second-boundary": (series(zero_times=boundary_zeros), False),
        "permanent-silence": (series(zero_times=long_zeros, stop_after_zero=True), False),
        "missing-heartbeats": (series(zero_times=long_zeros, heartbeats=False), False),
        "missing-records-not-zero": (series()[:10] + series()[100:], False),
        "wrong-switch": (series(sw=8, zero_times=long_zeros), False),
        "wrong-rate": (series(zero_times=long_zeros, byte_count=700_000), False),
        "wrong-size": (series(zero_times=long_zeros, byte_count=210_000, size=500), False),
        "empty": (b"", False),
        "malformed": (bytes([2, 12, 9]), False),
        "truncated": (series(zero_times=long_zeros) + bytes([2, 12, 9]), False),
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
