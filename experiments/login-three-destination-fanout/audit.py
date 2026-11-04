#!/usr/bin/env python3
"""Adversarial audit for the S8 repeated three-destination Login fanout."""

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
    row: dict[str, object] = {"id": app_id, "switch": switch, "prefix": prefix, "name": f"Synthetic {app_id}"}
    row.update(extra)
    return row


def world(seed: int, mode: str) -> str:
    target_rate = {"positive-fast": 0.05, "positive-normal": 0.02, "positive-slow": 0.002}.get(mode, 0.02)
    peer_hosts: object = [11, 22, 33] if mode.startswith("positive") else [11, 22] if mode == "two-destinations" else [11]
    sender_hosts: object = [1, 2, 3] if mode == "multiple-sources" else [1]
    if mode == "slow-spread":
        peer_hosts = [11, 22, 33]
        target_rate = 0.0002
    target_size = 200 if mode == "wrong-size" else 100
    target_proto = "udp" if mode == "wrong-protocol" else "tcp"
    target_switch = 7 if mode == "wrong-switch" else 8
    offset = seed % 100
    apps = [
        app(100, target_switch, f"10.{1 + offset}.0.0/24", preset="netflix", peer=101,
            rate_mbps=target_rate, size_bytes=target_size, proto=target_proto, hosts=sender_hosts),
        app(101, 7 if target_switch == 8 else 8, f"10.{101 + offset}.0.0/24", hosts=peer_hosts),
        app(102, 12, f"10.{2 + offset}.0.0/24", hosts=[41, 42, 43, 44]),
        app(103, 2, f"10.{102 + offset}.0.0/24", preset="netflix", peer=102,
            rate_mbps=0.2, size_bytes=200, proto="tcp"),
        app(104, 4, f"10.{3 + offset}.0.0/24", preset="fortnite", peer=105,
            rate_mbps=0.2, size_bytes=200, proto="udp"),
        app(105, 14, f"10.{103 + offset}.0.0/24"),
    ]
    lines = [
        "[world]", 'name="login-fanout-audit"', f"seed={seed}", 'family="harness"', "version=1",
        "[params]", "link_capacity_mbps=10", "link_latency_us=500", "queue_capacity_bytes=40000",
        "warmup_ms=3000", "duration_ms=40000", "delivery_floor_pct=0.0", "packet_size_bytes=1000",
        "control_delay_us=2000",
    ]
    for switch in range(15):
        lines += ["[[switch]]", f"id={switch}"]
    for a, b in EDGES:
        lines += ["[[link]]", f"a={a}", f"b={b}"]
    for row in apps:
        lines += ["[[app]]"] + [f"{key}={q(value)}" for key, value in row.items()]
    lines += [
        "[workload]", 'kind="mixed"', "size_bytes=1000", "[observe]", "punt_events_per_s=80",
        "punt_backlog=64", "export_bytes_per_s=16000", "export_burst_bytes=8000", "[harness]",
        "keepalive_ms=100", "log_cap_bytes=24576",
    ]
    plants: list[dict[str, object]] = [
        {"kind": "link_delay", "a": 1, "b": 2, "a_to_b_us": 2000, "b_to_a_us": 30000},
        {"kind": "wire_loss", "a": 11, "b": 12, "loss_pct": 12.0, "start_ms": 4000, "stop_ms": 39000},
        {"kind": "fail", "a": 0, "b": 1, "at_ms": 12000, "for_ms": 10000},
        {"kind": "fail", "a": 10, "b": 11, "at_ms": 12000, "for_ms": 10000},
        {"kind": "fail", "a": 0, "b": 5, "at_ms": 31000, "for_ms": 5000},
        {"kind": "fail", "a": 5, "b": 6, "at_ms": 31000, "for_ms": 5000},
        {"kind": "fail", "a": 5, "b": 10, "at_ms": 31000, "for_ms": 5000},
        {"kind": "scan", "app": 104, "to": 105, "start_ms": 5000, "for_ms": 4000,
         "addresses": 50, "each": 1, "size_bytes": 64},
    ]
    if mode == "one-shot-three-destination-scan":
        plants.append({"kind": "scan", "app": 100, "to": 102, "start_ms": 10000,
                       "for_ms": 1000, "addresses": 3, "each": 1, "size_bytes": 100})
    for plant in plants:
        lines += ["[[plant]]"] + [f"{key}={q(value)}" for key, value in plant.items()]
    return "\n".join(lines) + "\n"


def digest(path: Path) -> dict[str, object]:
    data = path.read_bytes()
    return {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}


def main() -> None:
    OUT.mkdir(exist_ok=True)
    cases = {
        "positive-fast": True, "positive-normal": True, "positive-slow": True,
        "off": False, "two-destinations": False, "slow-spread": False,
        "multiple-sources": False, "one-shot-three-destination-scan": False,
        "wrong-switch": False, "wrong-size": False, "wrong-protocol": False,
    }
    results = []
    for seed in range(6401, 6417):
        for load in (1.0, 1.25, 1.5):
            for name, expected in cases.items():
                stem = f"{name}-{seed}-{load}"
                world_path = OUT / f"{stem}.toml"
                log = OUT / f"{stem}.team"
                world_path.write_text(world(seed, name))
                run = subprocess.run(
                    [str(SIM), "run-world", str(world_path), "--program", str(SWITCH),
                     "--team-log", str(log), "--load-scale", str(load)], capture_output=True, text=True,
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
