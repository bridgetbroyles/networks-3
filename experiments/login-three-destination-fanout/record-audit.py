#!/usr/bin/env python3
"""Exercise the repeated Login fanout detector on exact records."""

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


def packet(seq: int, us: int, *, sw: int = 8, size: int = 100, proto: int = 6,
           src: int = 0x0A010001, dst: int = 0x0A020001) -> bytes:
    body = struct.pack("<BHIIHIBBI", sw, us // 100_000, src, dst, size, seq, proto, 63, us)
    return record(7, body)


def repeated(spread_us: int = 3_000_000, **kwargs: int) -> bytes:
    out = b""
    for round_no, base in enumerate((1_000_000, 1_000_000 + spread_us)):
        for i in range(3):
            out += packet(round_no * 3 + i, base + i * 10_000, dst=0x0A020001 + i, **kwargs)
    return out


def solve(data: bytes, name: str) -> bool:
    path = OUT / f"{name}.team"
    path.write_bytes(data)
    return subprocess.check_output([str(SIM), "detect", str(DETECT), str(path)], text=True).strip() == "true"


def main() -> None:
    OUT.mkdir(exist_ok=True)
    positive = repeated()
    two_dest = b"".join(packet(i, 1_000_000 + i * 500_000, dst=0x0A020001 + i % 2) for i in range(6))
    one_shot = b"".join(packet(i, 1_000_000 + i * 10_000, dst=0x0A020001 + i) for i in range(3))
    split_sources = b""
    for i in range(3):
        split_sources += packet(i * 2, 1_000_000, src=0x0A010001 + i, dst=0x0A020001 + i)
        split_sources += packet(i * 2 + 1, 4_000_000, src=0x0A010001 + i, dst=0x0A020001 + i)
    cases = {
        "positive": (positive, True),
        "boundary-six-seconds": (repeated(5_900_000), True),
        "spread-over-six-seconds": (repeated(6_100_000), False),
        "only-two-destinations": (two_dest, False),
        "one-shot-three-destination-scan": (one_shot, False),
        "three-sources-one-destination-each": (split_sources, False),
        "wrong-switch": (repeated(sw=7), False),
        "wrong-size": (repeated(size=200), False),
        "wrong-protocol": (repeated(proto=17), False),
        "empty": (b"", False),
        "malformed": (bytes([7, 23, 8]), False),
        "truncated": (positive + bytes([7, 23, 8]), False),
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
