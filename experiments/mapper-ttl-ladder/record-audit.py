#!/usr/bin/env python3
"""Exercise the TTL-ladder detector on exact and near-miss records."""

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


def packet(ttl: int, at: int, *, sw: int = 4, size: int = 64, proto: int = 17,
           src: int = 0x0A010001, dst: int = 0x0A020001, seq: int = 1) -> bytes:
    body = struct.pack("<BHIIHIBBI", sw, at // 100_000, src, dst, size, seq, proto, ttl, at)
    return record(7, body)


def ladder(ttls: range | tuple[int, ...], **kwargs: int) -> bytes:
    return b"".join(packet(ttl, 1_000_000 + i * 10_000, seq=i + 1, **kwargs)
                    for i, ttl in enumerate(ttls))


def solve(data: bytes, name: str) -> bool:
    path = OUT / f"{name}.team"
    path.write_bytes(data)
    return subprocess.check_output([str(SIM), "detect", str(DETECT), str(path)], text=True).strip() == "true"


def main() -> None:
    OUT.mkdir(exist_ok=True)
    positive = ladder(range(8))
    cases = {
        "positive-eight-rungs": (positive, True),
        "boundary-six-rungs": (ladder(range(6)), True),
        "five-rungs": (ladder(range(5)), False),
        "constant-low": (b"".join(packet(1, 1_000_000 + i * 10_000, seq=i + 1) for i in range(12)), False),
        "short-four-rung-ladder": (ladder(range(4)), False),
        "normal-ttl": (ladder((63, 63, 63, 63, 63, 63)), False),
        "wrong-switch": (ladder(range(8), sw=3), False),
        "wrong-size": (ladder(range(8), size=100), False),
        "wrong-protocol": (ladder(range(8), proto=6), False),
        "split-between-flows": (ladder(range(3)) + ladder(range(3, 6), src=0x0A010002), False),
        "empty": (b"", False),
        "malformed": (bytes([7, 23, 4]), False),
        "truncated": (positive + bytes([7, 23, 4]), False),
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
