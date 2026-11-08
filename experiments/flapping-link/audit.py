#!/usr/bin/env python3
"""Adversarial whole-network audit for repeated physical flaps on link 1-2."""

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


def failure(a: int, b: int, start: int, duration: int | None) -> dict[str, object]:
    row: dict[str, object] = {"kind": "fail", "a": a, "b": b, "at_ms": start}
    if duration is not None:
        row["for_ms"] = duration
    return row


def background() -> list[dict[str, object]]:
    """Keep every already-understood link/failure family active."""
    return [
        {"kind": "link_delay", "a": 1, "b": 2, "a_to_b_us": 2000, "b_to_a_us": 30000},
        {"kind": "link_delay", "a": 13, "b": 14, "a_to_b_us": 25000, "b_to_a_us": 25000},
        {"kind": "wire_loss", "a": 11, "b": 12, "loss_pct": 7.0, "start_ms": 4000, "stop_ms": 39000},
        {"kind": "link_capacity", "a": 6, "b": 7, "steps": [[0, 10], [10000, 4], [27000, 10]]},
        failure(0, 1, 11000, 6000),
        failure(10, 11, 11000, 6000),
        failure(0, 5, 31000, 5000),
        failure(5, 6, 31000, 5000),
        failure(5, 10, 31000, 5000),
    ]


def case(name: str) -> tuple[list[dict[str, object]], list[tuple[int, int, int, int]], bool, bool]:
    plants = background()
    gaps: list[tuple[int, int, int, int]] = []
    congestion = False
    if name == "positive-two-flaps":
        plants += [failure(1, 2, 4000, 3500), failure(1, 2, 21000, 3500)]
        return plants, gaps, True, congestion
    if name == "positive-three-flaps":
        plants += [failure(1, 2, 4000, 3000), failure(1, 2, 19000, 3500), failure(1, 2, 26000, 3000)]
        return plants, gaps, True, congestion
    if name == "off":
        return plants, gaps, False, congestion
    if name == "single-recovered-outage":
        plants += [failure(1, 2, 19000, 5000)]
        return plants, gaps, False, congestion
    if name == "permanent-outage":
        plants += [failure(1, 2, 19000, None)]
        return plants, gaps, False, congestion
    if name == "one-long-outage":
        plants += [failure(1, 2, 4000, 22000)]
        return plants, gaps, False, congestion
    if name == "too-short-twice":
        plants += [failure(1, 2, 5000, 1200), failure(1, 2, 22000, 1200)]
        return plants, gaps, False, congestion
    if name == "wrong-link-flaps":
        plants += [failure(2, 3, 4000, 3500), failure(2, 3, 21000, 3500)]
        return plants, gaps, False, congestion
    if name == "alternating-links":
        plants += [failure(1, 2, 4000, 3500), failure(2, 3, 21000, 3500)]
        return plants, gaps, False, congestion
    if name == "keepalive-gaps-only":
        gaps = [(1, 2, 4000, 7500), (2, 1, 4000, 7500), (1, 2, 21000, 24500), (2, 1, 21000, 24500)]
        return plants, gaps, False, congestion
    if name == "congestion-only":
        congestion = True
        return plants, gaps, False, congestion
    raise ValueError(name)


def world(seed: int, plants: list[dict[str, object]], gaps: list[tuple[int, int, int, int]], congestion: bool) -> str:
    lines = [
        "[world]", 'name="flapping-link-audit"', f"seed={seed}", 'family="harness"',
        "version=1", "[params]", "link_capacity_mbps=10", "link_latency_us=500",
        "queue_capacity_bytes=40000", "warmup_ms=3000", "duration_ms=40000",
        "delivery_floor_pct=0.0", "packet_size_bytes=1000", "control_delay_us=2000",
    ]
    for switch in range(15):
        lines += ["[[switch]]", f"id={switch}"]
    for a, b in EDGES:
        lines += ["[[link]]", f"a={a}", f"b={b}"]
    apps = [
        {"id": 100, "switch": 2, "prefix": "10.1.0.0/24", "name": "Background A", "preset": "fortnite", "peer": 101, "size_bytes": 200, "rate_mbps": 0.4, "proto": "udp"},
        {"id": 101, "switch": 13, "prefix": "10.2.0.0/24", "name": "Background B"},
    ]
    if congestion:
        apps += [
            {"id": 102, "switch": 1, "prefix": "10.3.0.0/24", "name": "Busy sender", "preset": "netflix", "peer": 103, "size_bytes": 1000, "rate_mbps": 9.8, "proto": "tcp"},
            {"id": 103, "switch": 2, "prefix": "10.4.0.0/24", "name": "Busy receiver"},
        ]
    for app in apps:
        lines += ["[[app]]"] + [f"{key}={q(value)}" for key, value in app.items()]
    lines += [
        "[workload]", 'kind="mixed"', "size_bytes=1000", "[observe]",
        "punt_events_per_s=80", "punt_backlog=64", "export_bytes_per_s=16000",
        "export_burst_bytes=8000", "[harness]", "keepalive_ms=100",
        "log_cap_bytes=24576",
    ]
    for plant in plants:
        lines += ["[[plant]]"] + [f"{key}={q(value)}" for key, value in plant.items()]
    for switch, toward, start, end in gaps:
        lines += [
            "[[harness.keepalive_gap]]", f"switch={switch}", f"toward={toward}",
            f"start_ms={start}", f"end_ms={end}",
        ]
    return "\n".join(lines) + "\n"


def digest(path: Path) -> dict[str, object]:
    data = path.read_bytes()
    return {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}


def main() -> None:
    OUT.mkdir(exist_ok=True)
    names = [
        "positive-two-flaps", "positive-three-flaps", "off",
        "single-recovered-outage", "permanent-outage", "one-long-outage",
        "too-short-twice", "wrong-link-flaps", "alternating-links",
        "keepalive-gaps-only", "congestion-only",
    ]
    results = []
    for seed in range(5101, 5113):
        for load in (1.0, 1.25, 1.5):
            for name in names:
                plants, gaps, expected, congestion = case(name)
                stem = f"{name}-{seed}-{load}"
                world_path = OUT / f"{stem}.toml"
                log = OUT / f"{stem}.team"
                world_path.write_text(world(seed, plants, gaps, congestion))
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
