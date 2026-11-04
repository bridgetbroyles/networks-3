#!/usr/bin/env python3
"""Adversarial whole-network audit for the S6 synchronized-rate claim."""

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
    (0, 1), (0, 5), (1, 2), (1, 6), (2, 3), (2, 7), (3, 4), (3, 8),
    (4, 9), (5, 6), (5, 10), (6, 7), (6, 11), (7, 8), (7, 12), (8, 9),
    (8, 13), (9, 14), (10, 11), (11, 12), (12, 13), (13, 14),
]


def q(value: object) -> str:
    return json.dumps(value)


def app(app_id: int, switch: int, prefix: str, **extra: object) -> dict[str, object]:
    row: dict[str, object] = {
        "id": app_id, "switch": switch, "prefix": prefix,
        "name": f"Synthetic {app_id}",
    }
    row.update(extra)
    return row


def world(seed: int, mode: str) -> str:
    offset = seed % 80
    source_site = 5 if mode == "wrong-switch" else 6
    size = 500 if mode == "wrong-size" else 1000
    rate_a = 3.5 if mode == "positive-heavy" else 2.0
    rate_b = rate_a
    start_a = start_b = 0
    preset_a = preset_b = "amazon"
    if mode == "off":
        start_b = 2900
    elif mode == "constant-twins":
        preset_a = preset_b = "netflix"
    elif mode == "phase-shift":
        start_b = 1700
    elif mode == "unequal-rates":
        rate_b = rate_a * 1.20
    elif mode == "near-equal-rates":
        rate_b = rate_a * 1.03
    elif mode == "late-second-source":
        start_b = 33500
    apps = [
        app(100, source_site, f"10.{1 + offset}.0.0/24", preset=preset_a,
            peer=101, rate_mbps=rate_a, size_bytes=size, proto="udp",
            on_ms=4100, off_ms=2700, start_ms=start_a),
        app(101, 14, f"10.{101 + offset}.0.0/24"),
    ]
    if mode != "one-source":
        apps += [
            app(102, source_site, f"10.{2 + offset}.0.0/24", preset=preset_b,
                peer=103, rate_mbps=rate_b, size_bytes=size, proto="udp",
                on_ms=4100, off_ms=2700, start_ms=start_b),
            app(103, 0, f"10.{102 + offset}.0.0/24"),
        ]
    apps += [
        app(104, 7, f"10.{3 + offset}.0.0/24", preset="netflix", peer=105,
            rate_mbps=4.0, size_bytes=1000, proto="tcp"),
        app(105, 2, f"10.{103 + offset}.0.0/24"),
        app(106, 4, f"10.{4 + offset}.0.0/24", preset="fortnite", peer=107,
            rate_mbps=0.2, size_bytes=64, proto="udp"),
        app(107, 13, f"10.{104 + offset}.0.0/24"),
    ]
    lines = [
        "[world]", 'name="camera-sync-audit"', f"seed={seed}",
        'family="harness"', "version=1", "[params]", "link_capacity_mbps=10",
        "link_latency_us=500", "queue_capacity_bytes=40000", "warmup_ms=3000",
        "duration_ms=40000", "delivery_floor_pct=0.0", "packet_size_bytes=1000",
        "control_delay_us=2000",
    ]
    for switch in range(15):
        lines += ["[[switch]]", f"id={switch}"]
    for a, b in EDGES:
        lines += ["[[link]]", f"a={a}", f"b={b}"]
    for row in apps:
        lines += ["[[app]]"] + [f"{key}={q(value)}" for key, value in row.items()]
    lines += [
        "[workload]", 'kind="mixed"', "size_bytes=1000", "[observe]",
        "punt_events_per_s=80", "punt_backlog=64", "export_bytes_per_s=16000",
        "export_burst_bytes=8000", "[harness]", "keepalive_ms=100",
        "log_cap_bytes=24576",
    ]
    plants: list[dict[str, object]] = [
        {"kind": "link_delay", "a": 13, "b": 14,
         "a_to_b_us": 25000, "b_to_a_us": 25000},
        {"kind": "link_delay", "a": 1, "b": 2,
         "a_to_b_us": 2000, "b_to_a_us": 30000},
        {"kind": "wire_loss", "a": 11, "b": 12, "loss_pct": 7.0,
         "start_ms": 4000, "stop_ms": 39000},
        {"kind": "fail", "a": 0, "b": 1, "at_ms": 12000, "for_ms": 10000},
        {"kind": "fail", "a": 10, "b": 11, "at_ms": 12000, "for_ms": 10000},
        {"kind": "fail", "a": 0, "b": 5, "at_ms": 32000, "for_ms": 4000},
        {"kind": "fail", "a": 5, "b": 6, "at_ms": 32000, "for_ms": 4000},
        {"kind": "fail", "a": 5, "b": 10, "at_ms": 32000, "for_ms": 4000},
        {"kind": "scan", "app": 106, "to": 107, "start_ms": 5000,
         "for_ms": 4000, "addresses": 50, "each": 1, "size_bytes": 64},
    ]
    if mode == "one-mismatched-window" and mode != "one-source":
        plants.append({"kind": "pause", "app": 102,
                       "at_ms": 15000, "for_ms": 2200})
    for plant in plants:
        lines += ["[[plant]]"] + [f"{key}={q(value)}" for key, value in plant.items()]
    return "\n".join(lines) + "\n"


def digest(path: Path) -> dict[str, object]:
    data = path.read_bytes()
    return {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}


def main() -> None:
    OUT.mkdir(exist_ok=True)
    cases = {
        "positive": True,
        "positive-heavy": True,
        "off": False,
        "one-source": False,
        "constant-twins": False,
        "phase-shift": False,
        "unequal-rates": False,
        "near-equal-rates": False,
        "one-mismatched-window": False,
        "late-second-source": False,
        "wrong-switch": False,
        "wrong-size": False,
    }
    results = []
    for seed in range(6801, 6817):
        for load in (1.0, 1.25, 1.5):
            for name, expected in cases.items():
                stem = f"{name}-{seed}-{load}"
                world_path = OUT / f"{stem}.toml"
                log = OUT / f"{stem}.team"
                world_path.write_text(world(seed, name))
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
                row = {"case": name, "seed": seed, "load": load,
                       "expected": expected, "actual": actual,
                       "log_bytes": log.stat().st_size,
                       "pass": actual == expected and log.stat().st_size <= 24576}
                results.append(row)
                if not row["pass"]:
                    (ROOT / "audit-results.json").write_text(
                        json.dumps(results, indent=2) + "\n"
                    )
                    raise AssertionError(row)
    summary = {
        "cases": len(results), "all_passed": all(row["pass"] for row in results),
        "max_log_bytes": max(row["log_bytes"] for row in results),
        "claim_log_cap_bytes": 24576, "switch.wasm": digest(SWITCH),
        "detect.wasm": digest(DETECT), "results": results,
    }
    (ROOT / "audit-results.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(f"PASS {len(results)} cases; max log {summary['max_log_bytes']} bytes")


if __name__ == "__main__":
    main()
