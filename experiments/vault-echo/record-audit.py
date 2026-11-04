#!/usr/bin/env python3
"""Exercise the echo detector on exact, near-miss, and malformed records."""

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


def packet(seq: int, us: int, *, sw: int = 3, size: int = 500, proto: int = 6,
           src: int = 0x0A010001, dst: int = 0x0A020001) -> bytes:
    body = struct.pack("<BHIIHIBBI", sw, us // 100_000, src, dst, size, seq, proto, 63, us)
    return record(7, body)


def pair(seq: int, start: int, separation: int = 400, **kwargs: int) -> bytes:
    return packet(seq, start, **kwargs) + packet(seq, start + separation, **kwargs)


def solve(data: bytes, name: str) -> bool:
    path = OUT / f"{name}.team"
    path.write_bytes(data)
    return subprocess.check_output([str(SIM), "detect", str(DETECT), str(path)], text=True).strip() == "true"


def main() -> None:
    OUT.mkdir(exist_ok=True)
    positive = pair(256, 1_000_000) + pair(512, 2_000_000)
    cases = {
        "positive": (positive, True),
        "normal-unique": (packet(256, 1_000_000) + packet(512, 2_000_000), False),
        "one-pair": (pair(256, 1_000_000), False),
        "retransmissions": (pair(256, 1_000_000, 50_000) + pair(512, 2_000_000, 80_000), False),
        "boundary-750us": (pair(256, 1_000_000, 750) + pair(512, 2_000_000, 750), True),
        "boundary-751us": (pair(256, 1_000_000, 751) + pair(512, 2_000_000, 751), False),
        "wrong-switch": (pair(256, 1_000_000, sw=4) + pair(512, 2_000_000, sw=4), False),
        "wrong-size": (pair(256, 1_000_000, size=1000) + pair(512, 2_000_000, size=1000), False),
        "wrong-protocol": (pair(256, 1_000_000, proto=17) + pair(512, 2_000_000, proto=17), False),
        "different-flows": (pair(256, 1_000_000) + pair(512, 2_000_000, src=0x0A010002), False),
        "empty": (b"", False),
        "malformed": (bytes([7, 23, 3]), False),
        "truncated": (positive + bytes([7, 23, 3]), False),
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
