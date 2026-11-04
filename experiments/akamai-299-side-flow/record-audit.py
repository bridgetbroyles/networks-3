#!/usr/bin/env python3
"""Exercise the 299-byte reverse-side-flow detector on exact records."""

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


def packet(seq: int, us: int, *, sw: int = 2, size: int = 299, proto: int = 6,
           src: int = 0x0A010001, dst: int = 0x0A020001) -> bytes:
    body = struct.pack("<BHIIHIBBI", sw, us // 100_000, src, dst, size, seq, proto, 63, us)
    return record(7, body)


def flow(times: tuple[int, ...], **kwargs: int) -> bytes:
    return b"".join(packet(i, us, **kwargs) for i, us in enumerate(times))


def solve(data: bytes, name: str) -> bool:
    path = OUT / f"{name}.team"
    path.write_bytes(data)
    return subprocess.check_output([str(SIM), "detect", str(DETECT), str(path)], text=True).strip() == "true"


def main() -> None:
    OUT.mkdir(exist_ok=True)
    positive = flow((1_000_000, 1_500_000, 2_000_000))
    cases = {
        "positive": (positive, True),
        "boundary-one-second": (flow((1_000_000, 1_500_000, 2_000_000)), True),
        "span-under-one-second": (flow((1_000_000, 1_400_000, 1_900_000)), False),
        "only-two-samples": (flow((1_000_000, 2_000_000)), False),
        "same-sequence": (packet(7, 1_000_000) + packet(7, 1_500_000) + packet(7, 2_000_000), False),
        "same-time-window": (packet(1, 1_000_000) + packet(2, 1_010_000) + packet(3, 1_020_000), False),
        "wrong-switch": (flow((1_000_000, 1_500_000, 2_000_000), sw=3), False),
        "size-298": (flow((1_000_000, 1_500_000, 2_000_000), size=298), False),
        "size-300": (flow((1_000_000, 1_500_000, 2_000_000), size=300), False),
        "wrong-protocol": (flow((1_000_000, 1_500_000, 2_000_000), proto=17), False),
        "split-between-flows": (flow((1_000_000, 2_000_000)) + packet(2, 3_000_000, src=0x0A010002), False),
        "empty": (b"", False),
        "malformed": (bytes([7, 23, 2]), False),
        "truncated": (positive + bytes([7, 23, 2]), False),
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
