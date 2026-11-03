#!/usr/bin/env python3
"""Adversarial whole-network audit for the S5 physical-interruption pair."""

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


def base_world(seed: int, plants: list[dict[str, object]], gaps: list[tuple[int, int]] = []) -> str:
    lines = [
        "[world]", 'name="hub-outage-audit"', f"seed={seed}", 'family="harness"',
        "version=1", "[params]", "link_capacity_mbps=10", "link_latency_us=500",
        "queue_capacity_bytes=40000", "warmup_ms=3000", "duration_ms=40000",
        "delivery_floor_pct=0.0", "packet_size_bytes=1000", "control_delay_us=2000",
    ]
    for switch in range(15):
        lines += ["[[switch]]", f"id={switch}"]
    for a, b in EDGES:
        lines += ["[[link]]", f"a={a}", f"b={b}"]
    # Background traffic exercises the same physical ports and report paths.
    apps = [
        {"id": 100, "switch": 2, "prefix": "10.1.0.0/24", "name": "Background A", "preset": "fortnite", "peer": 101, "size_bytes": 200, "rate_mbps": 0.4, "proto": "udp"},
        {"id": 101, "switch": 13, "prefix": "10.2.0.0/24", "name": "Background B"},
        {"id": 102, "switch": 5, "prefix": "10.3.0.0/24", "name": "S5 traffic", "preset": "fortnite", "peer": 103, "size_bytes": 1000, "rate_mbps": 0.8, "proto": "tcp"},
        {"id": 103, "switch": 9, "prefix": "10.4.0.0/24", "name": "S5 peer"},
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
    for switch, toward in gaps:
        lines += [
            "[[harness.keepalive_gap]]", f"switch={switch}", f"toward={toward}",
            "start_ms=31000", "end_ms=36000",
        ]
    return "\n".join(lines) + "\n"


def failures(edges: list[tuple[int, int]], start: int, duration: int | None) -> list[dict[str, object]]:
    rows = []
    for a, b in edges:
        row: dict[str, object] = {"kind": "fail", "a": a, "b": b, "at_ms": start}
        if duration is not None:
            row["for_ms"] = duration
        rows.append(row)
    return rows


def case(name: str) -> tuple[list[dict[str, object]], list[tuple[int, int]], bool]:
    target = [(0, 5), (5, 6), (5, 10)]
    other_eggs = [
        {"kind": "link_delay", "a": 1, "b": 2, "a_to_b_us": 2000, "b_to_a_us": 30000},
        {"kind": "link_delay", "a": 13, "b": 14, "a_to_b_us": 25000, "b_to_a_us": 25000},
        {"kind": "wire_loss", "a": 11, "b": 12, "loss_pct": 7.0, "start_ms": 4000, "stop_ms": 39000},
        {"kind": "link_capacity", "a": 12, "b": 13, "steps": [[0, 10], [10000, 4], [27000, 10]]},
    ]
    if name == "positive-5s":
        return other_eggs + failures(target, 31000, 5000), [], True
    if name == "positive-6s":
        return other_eggs + failures(target, 30000, 6000), [], True
    if name == "off":
        return other_eggs, [], False
    if name == "one-edge":
        return other_eggs + failures(target[:1], 31000, 5000), [], False
    if name == "two-edges":
        return other_eggs + failures(target[:2], 31000, 5000), [], False
    if name == "short-2s":
        return other_eggs + failures(target, 33000, 2000), [], False
    if name == "short-3s":
        return other_eggs + failures(target, 32000, 3000), [], False
    if name == "permanent":
        return other_eggs + failures(target, 31000, None), [], False
    if name == "staggered":
        rows = other_eggs
        rows += failures([(0, 5)], 28000, 4000)
        rows += failures([(5, 6)], 31000, 4000)
        rows += failures([(5, 10)], 34000, 4000)
        return rows, [], False
    if name == "wrong-hub":
        return other_eggs + failures([(2, 7), (6, 7), (7, 8), (7, 12)], 31000, 5000), [], False
    if name == "keepalive-only":
        return other_eggs, [(0, 5), (6, 5), (10, 5)], False
    raise ValueError(name)


def digest(path: Path) -> dict[str, object]:
    data = path.read_bytes()
    return {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}


def main() -> None:
    OUT.mkdir(exist_ok=True)
    names = [
        "positive-5s", "positive-6s", "off", "one-edge", "two-edges",
        "short-2s", "short-3s", "permanent", "staggered", "wrong-hub",
        "keepalive-only",
    ]
    results = []
    for seed in range(4101, 4113):
        for load in (1.0, 1.25, 1.5):
            for name in names:
                plants, gaps, expected = case(name)
                stem = f"{name}-{seed}-{load}"
                world = OUT / f"{stem}.toml"
                log = OUT / f"{stem}.team"
                world.write_text(base_world(seed, plants, gaps))
                run = subprocess.run(
                    [str(SIM), "run-world", str(world), "--program", str(SWITCH),
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
