#!/usr/bin/env python3
"""Test detector boundaries directly with compact framed packet logs."""

from __future__ import annotations

import json
import struct
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[1]
SIM = REPO / "target" / "release" / "competitive_net_sim"
DETECT = ROOT / "generated" / "detect.wasm"
OUT = ROOT / "generated" / "records"


def packet(tick: int, source: int, destination: int, *, switch: int = 7, size: int = 200, sequence: int = 0, proto: int = 6) -> bytes:
    body = struct.pack("<BHIIHIBBI", switch, tick, source, destination, size, sequence, proto, 63, tick * 100_000)
    return bytes((7, len(body))) + body


def fanout(count: int, *, spacing: int = 1, source: int = 0x0A3C0101, switch: int = 7, size: int = 200, sequence: int = 0, proto: int = 6) -> bytes:
    return b"".join(packet(10 + i * spacing, source, 0x0A3D0001 + i, switch=switch, size=size, sequence=sequence, proto=proto) for i in range(count))


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    cases = {
        "positive-50": (fanout(50), True),
        "positive-60-with-noise": (fanout(60) + packet(1, 1, 2, switch=2, size=1000), True),
        "only-49": (fanout(49), False),
        "fifty-too-slow": (fanout(50, spacing=2), False),
        "wrong-switch": (fanout(50, switch=6), False),
        "wrong-size": (fanout(50, size=199), False),
        "numbered": (fanout(50, sequence=1), False),
        "udp": (fanout(50, proto=17), False),
        "split-sources": (fanout(25) + fanout(25, source=0x0A3C0201), False),
        "same-destination": (b"".join(packet(10 + i, 0x0A3C0101, 0x0A3D0001) for i in range(60)), False),
        "malformed": (b"\x07\x17\x00", False),
        "empty": (b"", False),
    }
    results = []
    for name, (data, expected) in cases.items():
        path = OUT / f"{name}.team"
        path.write_bytes(data)
        answer = subprocess.run([str(SIM), "detect", str(DETECT), str(path)], cwd=REPO, check=True, capture_output=True, text=True).stdout.strip() == "true"
        results.append({"case": name, "expected": expected, "answer": answer, "bytes": len(data)})
    summary = {"cases": len(results), "passed": sum(r["expected"] == r["answer"] for r in results), "results": results}
    (ROOT / "record-results.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps({k: summary[k] for k in ("cases", "passed")}, indent=2))
    if summary["passed"] != summary["cases"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
