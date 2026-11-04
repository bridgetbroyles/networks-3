#!/usr/bin/env python3
"""Exercise the open-loop bundle detector on positive and near-miss worlds."""

from __future__ import annotations

import json
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[1]
BASE = REPO / "experiments" / "login-three-destination-fanout" / "audit-worlds" / "positive-fast-6401-1.0.toml"
SIM = REPO / "target" / "release" / "competitive_net_sim"
SWITCH = ROOT / "generated" / "switch.wasm"
DETECT = ROOT / "generated" / "detect.wasm"
OUT = ROOT / "generated" / "audit"


def app(app_id: int, switch: int, prefix: str, hosts: int, peer: int, size: int) -> str:
    numbers = ", ".join(str(i) for i in range(1, hosts + 1))
    return f'''\n[[app]]
id={app_id}
switch={switch}
prefix="{prefix}"
name="Open-loop source {app_id}"
hosts=[{numbers}]
preset="jane_street"
rate_mbps=0.1
peer={peer}
size_bytes={size}
proto="tcp"
'''


def receiver(app_id: int, switch: int, prefix: str) -> str:
    return f'''\n[[app]]
id={app_id}
switch={switch}
prefix="{prefix}"
name="Open-loop receiver {app_id}"
'''


def flow(app_id: int, target: int, size: int) -> str:
    return f'''\n[[plant]]
kind="side_flow"
app={app_id}
to={target}
start_ms=4000
stop_ms=9000
rate_pps=2.0
size_bytes={size}
proto="tcp"
'''


def additions(large: bool, medium_a: bool, medium_b: bool, hosts_b: int = 6, site_b: int = 7) -> str:
    text = ""
    specs = [
        (200, 201, 1000, 6, 6, 7, large),
        (210, 211, 200, 1, 6, 7, medium_a),
        (220, 221, 200, hosts_b, hosts_b, site_b, medium_b),
    ]
    for source, target_base, size, hosts, flow_count, site, enabled in specs:
        # Six /32 receivers share one /24 detector identity.  Each side-flow
        # plant contributes a sequence-zero first packet, matching the exact
        # field that distinguished the retained truth-run traffic from normal
        # numbered mixed-workload packets.  This is intentionally not a scan:
        # scans acquire increasing sequence values in harness worlds.
        text += app(source, site, f"10.{source}.0.0/24", hosts, target_base, size)
        for offset in range(6):
            target = target_base + offset
            text += receiver(target, 14, f"10.{target_base}.0.{offset * 4}/30")
            if enabled and offset < flow_count:
                text += flow(source, target, size)
    return text


CASES = {
    "positive": (True, True, True, 6, 7, True),
    "off-large": (False, True, True, 6, 7, False),
    "off-medium": (True, True, False, 6, 7, False),
    "four-host-near-miss": (True, True, True, 4, 7, False),
    "wrong-site-near-miss": (True, True, True, 6, 6, False),
}


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    base = BASE.read_text()
    results = []
    for seed in (7341, 7349, 7357, 7369):
        for load in (1.0, 1.25, 1.5):
            for name, (large, a, b, hosts_b, site_b, expected) in CASES.items():
                world = base.replace("seed=6401", f"seed={seed}") + additions(large, a, b, hosts_b, site_b)
                path = OUT / f"{name}-{seed}-{load}.toml"
                team = path.with_suffix(".team")
                path.write_text(world)
                subprocess.run(
                    [str(SIM), "run-world", str(path), "--load-scale", str(load), "--program", str(SWITCH), "--team-log", str(team)],
                    cwd=REPO,
                    check=True,
                    stdout=subprocess.DEVNULL,
                )
                answer = subprocess.run(
                    [str(SIM), "detect", str(DETECT), str(team)],
                    cwd=REPO,
                    check=True,
                    capture_output=True,
                    text=True,
                ).stdout.strip() == "true"
                results.append({"case": name, "seed": seed, "load": load, "expected": expected, "answer": answer, "bytes": team.stat().st_size})
    summary = {
        "cases": len(results),
        "passed": sum(row["expected"] == row["answer"] for row in results),
        "largest_log_bytes": max(row["bytes"] for row in results),
        "results": results,
    }
    (ROOT / "audit-results.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps({key: summary[key] for key in ("cases", "passed", "largest_log_bytes")}, indent=2))
    if summary["passed"] != summary["cases"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
