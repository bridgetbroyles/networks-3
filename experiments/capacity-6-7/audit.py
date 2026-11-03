#!/usr/bin/env python3
"""Adversarial whole-network audit for the link 6-7 capacity claim."""

from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[1]
SIM = REPO / "target/release/competitive_net_sim"
SWITCH = ROOT / "generated/switch.wasm"
DETECT = ROOT / "generated/detect.wasm"
OUT = ROOT / "audit-worlds"
EDGES = [
    (0, 1), (0, 5), (1, 2), (1, 6), (2, 3), (2, 7),
    (3, 4), (3, 8), (4, 9), (5, 6), (5, 10), (6, 7),
    (6, 11), (7, 8), (7, 12), (8, 9), (8, 13), (9, 14),
    (10, 11), (11, 12), (12, 13), (13, 14),
]


def q(value: object) -> str:
    return json.dumps(value)


def app(app_id: int, switch: int, peer: int | None = None, **extra: object) -> dict[str, object]:
    row: dict[str, object] = {
        "id": app_id,
        "switch": switch,
        "prefix": f"10.{app_id - 99}.0.0/24",
        "name": f"Synthetic {app_id}",
    }
    if peer is not None:
        row.update(
            preset="amazon",
            peer=peer,
            size_bytes=1000,
            rate_mbps=6.0,
            on_ms=400,
            off_ms=600,
            start_ms=1000,
        )
    row.update(extra)
    return row


def world(seed: int, plants: list[dict[str, object]], congestion_only: bool = False) -> str:
    lines = [
        "[world]", 'name="capacity-6-7-audit"', f"seed={seed}", 'family="harness"',
        "version=1", "[params]", "link_capacity_mbps=10", "link_latency_us=500",
        "queue_capacity_bytes=40000", "warmup_ms=3000", "duration_ms=40000",
        "delivery_floor_pct=0.0", "packet_size_bytes=1000", "control_delay_us=2000",
    ]
    for switch in range(15):
        lines += ["[[switch]]", f"id={switch}"]
    for a, b in EDGES:
        lines += ["[[link]]", f"a={a}", f"b={b}"]
    apps = [app(100, 6, 101), app(101, 7)]
    if congestion_only:
        apps += [app(102, 6, 103, rate_mbps=8.0, on_ms=300, off_ms=700, start_ms=5030), app(103, 7)]
    # Extra traffic exercises report paths without identifying the target.
    apps += [app(104, 2, 105, size_bytes=200, rate_mbps=0.2, preset="fortnite", proto="udp"), app(105, 13)]
    for row in apps:
        lines += ["[[app]]"] + [f"{key}={q(value)}" for key, value in row.items()]
    lines += [
        "[workload]", 'kind="mixed"', "size_bytes=1000", "[observe]",
        "punt_events_per_s=80", "punt_backlog=64", "export_bytes_per_s=16000",
        "export_burst_bytes=8000", "[harness]", "keepalive_ms=100",
        "log_cap_bytes=24576",
    ]
    for plant in plants:
        lines += ["[[plant]]"] + [f"{key}={q(value)}" for key, value in plant.items()]
    return "\n".join(lines) + "\n"


def capacity(a: int, b: int, steps: list[list[int]]) -> dict[str, object]:
    return {"kind": "link_capacity", "a": a, "b": b, "steps": steps}


def case(name: str) -> tuple[list[dict[str, object]], bool, bool]:
    background = [
        {"kind": "link_delay", "a": 1, "b": 2, "a_to_b_us": 2000, "b_to_a_us": 30000},
        {"kind": "link_delay", "a": 13, "b": 14, "a_to_b_us": 25000, "b_to_a_us": 25000},
        {"kind": "wire_loss", "a": 11, "b": 12, "loss_pct": 7.0, "start_ms": 4000, "stop_ms": 39000},
        capacity(12, 13, [[0, 10], [10000, 4], [27000, 10]]),
        {"kind": "fail", "a": 0, "b": 1, "at_ms": 12000, "for_ms": 10000},
        {"kind": "fail", "a": 10, "b": 11, "at_ms": 12000, "for_ms": 10000},
        {"kind": "fail", "a": 0, "b": 5, "at_ms": 31000, "for_ms": 5000},
        {"kind": "fail", "a": 5, "b": 6, "at_ms": 31000, "for_ms": 5000},
        {"kind": "fail", "a": 5, "b": 10, "at_ms": 31000, "for_ms": 5000},
    ]
    positive_rates = {"positive-1mbps": 1, "positive-2mbps": 2, "positive-4mbps": 4, "positive-5mbps": 5}
    if name in positive_rates:
        return background + [capacity(6, 7, [[0, 10], [10000, positive_rates[name]], [28000, 10]])], True, False
    if name == "off" or name == "other-edge-only":
        return background, False, False
    if name == "near-8mbps":
        return background + [capacity(6, 7, [[0, 10], [10000, 8], [28000, 10]])], False, False
    if name == "constant-4mbps":
        return background + [capacity(6, 7, [[0, 4]])], False, False
    if name == "brief-4mbps":
        return background + [capacity(6, 7, [[0, 10], [20000, 4], [21500, 10]])], False, False
    if name == "delay-only":
        return background + [{"kind": "link_delay", "a": 6, "b": 7, "a_to_b_us": 15000, "b_to_a_us": 15000}], False, False
    if name == "wire-loss-only":
        return background + [{"kind": "wire_loss", "a": 6, "b": 7, "loss_pct": 10.0, "start_ms": 10000, "stop_ms": 28000}], False, False
    if name == "congestion-only":
        return background, False, True
    raise ValueError(name)


def digest(path: Path) -> dict[str, object]:
    data = path.read_bytes()
    return {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}


def main() -> None:
    OUT.mkdir(exist_ok=True)
    names = [
        "positive-1mbps", "positive-2mbps", "positive-4mbps", "positive-5mbps",
        "off", "near-8mbps", "constant-4mbps", "brief-4mbps",
        "other-edge-only", "delay-only", "wire-loss-only", "congestion-only",
    ]
    results = []
    for seed in range(4201, 4213):
        for load in (1.0, 1.25, 1.5):
            for name in names:
                plants, expected, congestion = case(name)
                stem = f"{name}-{seed}-{load}"
                world_path = OUT / f"{stem}.toml"
                log = OUT / f"{stem}.team"
                world_path.write_text(world(seed, plants, congestion))
                run = subprocess.run(
                    [str(SIM), "run-world", str(world_path), "--program", str(SWITCH),
                     "--team-log", str(log), "--load-scale", str(load)],
                    capture_output=True, text=True,
                )
                if run.returncode or "program failed" in run.stderr.lower():
                    raise RuntimeError(f"{stem}: {run.stderr}")
                actual = subprocess.check_output(
                    [str(SIM), "detect", str(DETECT), str(log)], text=True
                ).strip() == "true"
                row = {
                    "case": name, "seed": seed, "load": load,
                    "expected": expected, "actual": actual,
                    "log_bytes": log.stat().st_size,
                    "pass": actual == expected and log.stat().st_size <= 24576,
                }
                results.append(row)
                if not row["pass"]:
                    (ROOT / "audit-results.json").write_text(json.dumps(results, indent=2) + "\n")
                    raise AssertionError(row)
    summary = {
        "cases": len(results),
        "all_passed": all(row["pass"] for row in results),
        "max_log_bytes": max(row["log_bytes"] for row in results),
        "claim_log_cap_bytes": 24576,
        "switch.wasm": digest(SWITCH),
        "detect.wasm": digest(DETECT),
        "results": results,
    }
    (ROOT / "audit-results.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(f"PASS {len(results)} cases; max log {summary['max_log_bytes']} bytes")


if __name__ == "__main__":
    main()
