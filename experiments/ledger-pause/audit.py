#!/usr/bin/env python3
"""Adversarial whole-network audit for the S9 Ledger pause claim."""

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


def app(app_id: int, switch: int, prefix: str, **extra: object) -> dict[str, object]:
    row: dict[str, object] = {"id": app_id, "switch": switch, "prefix": prefix, "name": f"Synthetic {app_id}"}
    row.update(extra)
    return row


def world(seed: int, plants: list[dict[str, object]], natural_onoff: bool = False) -> str:
    lines = [
        "[world]", 'name="ledger-pause-audit"', f"seed={seed}", 'family="harness"', "version=1",
        "[params]", "link_capacity_mbps=10", "link_latency_us=500", "queue_capacity_bytes=40000",
        "warmup_ms=3000", "duration_ms=40000", "delivery_floor_pct=0.0", "packet_size_bytes=1000",
        "control_delay_us=2000",
    ]
    for switch in range(15):
        lines += ["[[switch]]", f"id={switch}"]
    for a, b in EDGES:
        lines += ["[[link]]", f"a={a}", f"b={b}"]
    apps = [
        app(100, 9, "10.1.0.0/24", preset="netflix", peer=101, rate_mbps=0.75, size_bytes=1000),
        app(101, 2, "10.2.0.0/24"),
        app(102, 9, "10.3.0.0/24", preset="netflix", peer=103, rate_mbps=4.0, size_bytes=1000),
        app(103, 4, "10.4.0.0/24"),
        app(104, 8, "10.5.0.0/24", preset="netflix", peer=105, rate_mbps=0.75, size_bytes=1000),
        app(105, 3, "10.6.0.0/24"),
        app(106, 9, "10.7.0.0/24", preset="netflix", peer=107, rate_mbps=0.75, size_bytes=500),
        app(107, 14, "10.8.0.0/24"),
        app(108, 7, "10.9.0.0/24", preset="fortnite", peer=109, rate_mbps=0.2, size_bytes=200, proto="udp"),
        app(109, 12, "10.10.0.0/24"),
    ]
    if natural_onoff:
        apps += [
            app(110, 9, "10.11.0.0/24", preset="amazon", peer=111, rate_mbps=0.75,
                size_bytes=1000, on_ms=8000, off_ms=8000, start_ms=1000),
            app(111, 13, "10.12.0.0/24"),
        ]
    for row in apps:
        lines += ["[[app]]"] + [f"{key}={q(value)}" for key, value in row.items()]
    lines += [
        "[workload]", 'kind="mixed"', "size_bytes=1000", "[observe]", "punt_events_per_s=80",
        "punt_backlog=64", "export_bytes_per_s=16000", "export_burst_bytes=8000", "[harness]",
        "keepalive_ms=100", "log_cap_bytes=24576",
    ]
    for plant in plants:
        lines += ["[[plant]]"] + [f"{key}={q(value)}" for key, value in plant.items()]
    return "\n".join(lines) + "\n"


def case(name: str) -> tuple[list[dict[str, object]], bool, bool]:
    background = [
        {"kind": "link_delay", "a": 1, "b": 2, "a_to_b_us": 2000, "b_to_a_us": 30000},
        {"kind": "wire_loss", "a": 11, "b": 12, "loss_pct": 7.0, "start_ms": 4000, "stop_ms": 39000},
        {"kind": "fail", "a": 0, "b": 1, "at_ms": 12000, "for_ms": 10000},
        {"kind": "fail", "a": 10, "b": 11, "at_ms": 12000, "for_ms": 10000},
        {"kind": "fail", "a": 0, "b": 5, "at_ms": 31000, "for_ms": 5000},
        {"kind": "fail", "a": 5, "b": 6, "at_ms": 31000, "for_ms": 5000},
        {"kind": "fail", "a": 5, "b": 10, "at_ms": 31000, "for_ms": 5000},
        {"kind": "scan", "app": 108, "to": 109, "start_ms": 5000, "for_ms": 4000,
         "addresses": 50, "each": 1, "size_bytes": 64},
    ]
    positive = {
        "positive-14s": (10000, 14000),
        "positive-14s-shifted": (10750, 14000),
        "positive-20s": (11250, 20000),
        "positive-24s": (10000, 24000),
    }
    if name in positive:
        start, duration = positive[name]
        return background + [{"kind": "pause", "app": 100, "at_ms": start, "for_ms": duration}], True, False
    if name == "off":
        return background, False, False
    if name == "short-2s":
        return background + [{"kind": "pause", "app": 100, "at_ms": 10000, "for_ms": 2000}], False, False
    if name == "short-6s":
        return background + [{"kind": "pause", "app": 100, "at_ms": 10000, "for_ms": 6000}], False, False
    if name == "near-10s":
        return background + [{"kind": "pause", "app": 100, "at_ms": 10750, "for_ms": 10000}], False, False
    if name == "near-12s":
        return background + [{"kind": "pause", "app": 100, "at_ms": 10750, "for_ms": 12000}], False, False
    if name == "permanent":
        return background + [{"kind": "pause", "app": 100, "at_ms": 10000, "for_ms": 40000}], False, False
    if name == "wrong-switch":
        return background + [{"kind": "pause", "app": 104, "at_ms": 10000, "for_ms": 16000}], False, False
    if name == "wrong-rate":
        return background + [{"kind": "pause", "app": 102, "at_ms": 10000, "for_ms": 16000}], False, False
    if name == "wrong-size":
        return background + [{"kind": "pause", "app": 106, "at_ms": 10000, "for_ms": 16000}], False, False
    if name == "natural-onoff":
        return background, False, True
    raise ValueError(name)


def digest(path: Path) -> dict[str, object]:
    data = path.read_bytes()
    return {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}


def main() -> None:
    OUT.mkdir(exist_ok=True)
    names = [
        "positive-14s", "positive-14s-shifted", "positive-20s", "positive-24s", "off", "short-2s",
        "short-6s", "near-10s", "near-12s", "permanent", "wrong-switch", "wrong-rate", "wrong-size",
        "natural-onoff",
    ]
    results = []
    for seed in range(5101, 5113):
        for load in (1.0, 1.25, 1.5):
            for name in names:
                plants, expected, natural = case(name)
                stem = f"{name}-{seed}-{load}"
                world_path = OUT / f"{stem}.toml"
                log = OUT / f"{stem}.team"
                world_path.write_text(world(seed, plants, natural))
                run = subprocess.run(
                    [str(SIM), "run-world", str(world_path), "--program", str(SWITCH),
                     "--team-log", str(log), "--load-scale", str(load)],
                    capture_output=True, text=True,
                )
                if run.returncode or "program failed" in run.stderr.lower():
                    raise RuntimeError(f"{stem}: {run.stderr}")
                actual = subprocess.check_output([str(SIM), "detect", str(DETECT), str(log)], text=True).strip() == "true"
                row = {"case": name, "seed": seed, "load": load, "expected": expected, "actual": actual,
                       "log_bytes": log.stat().st_size, "pass": actual == expected and log.stat().st_size <= 24576}
                results.append(row)
                if not row["pass"]:
                    (ROOT / "audit-results.json").write_text(json.dumps(results, indent=2) + "\n")
                    raise AssertionError(row)
    summary = {
        "cases": len(results), "all_passed": all(row["pass"] for row in results),
        "max_log_bytes": max(row["log_bytes"] for row in results), "claim_log_cap_bytes": 24576,
        "switch.wasm": digest(SWITCH), "detect.wasm": digest(DETECT), "results": results,
    }
    (ROOT / "audit-results.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(f"PASS {len(results)} cases; max log {summary['max_log_bytes']} bytes")


if __name__ == "__main__":
    main()
