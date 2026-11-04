#!/usr/bin/env python3
"""Exercise the S7 open-loop mesh detector on causal and near-miss worlds."""

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


def source(app_id: int, site: int, offset: int) -> str:
    return f'''\n[[app]]
id={app_id}
switch={site}
prefix="10.200.0.{offset * 16}/28"
name="Mesh source {app_id}"
hosts=[1]
'''


def receiver(app_id: int, offset: int) -> str:
    return f'''\n[[app]]
id={app_id}
switch=14
prefix="10.201.0.{offset * 16}/28"
name="Mesh receiver {app_id}"
'''


def side_flow(app_id: int, target: int, rate_pps: float, size: int, proto: str) -> str:
    return f'''\n[[plant]]
kind="side_flow"
app={app_id}
to={target}
start_ms=4000
stop_ms=14000
rate_pps={rate_pps}
size_bytes={size}
proto="{proto}"
'''


def additions(case: str) -> str:
    site, sources, destinations, size, proto, rate, enabled = 7, 16, 8, 200, "tcp", 2.0, True
    if case == "off":
        enabled = False
    elif case == "seven-sources":
        sources = 7
    elif case == "seven-destinations":
        destinations = 7
    elif case == "one-source":
        sources = 1
    elif case == "one-destination":
        destinations = 1
    elif case == "wrong-site":
        site = 6
    elif case == "wrong-size":
        size = 199
    elif case == "wrong-protocol":
        proto = "udp"
    elif case == "too-few-samples":
        sources = 8
    elif case != "positive":
        raise ValueError(case)

    text = ""
    for offset in range(8):
        target = 400 + offset
        text += receiver(target, offset)
    for offset in range(sources):
        app_id = 300 + offset
        target = 400 + (offset % destinations)
        text += source(app_id, site, offset)
        if enabled:
            text += side_flow(app_id, target, rate, size, proto)
    return text


CASES = {
    "positive": True,
    "off": False,
    "seven-sources": False,
    "seven-destinations": False,
    "one-source": False,
    "one-destination": False,
    "wrong-site": False,
    "wrong-size": False,
    "wrong-protocol": False,
    "too-few-samples": False,
}


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    base = BASE.read_text()
    results = []
    for seed in (8209, 8219, 8221, 8231):
        for load in (1.0, 1.25, 1.5):
            for case, expected in CASES.items():
                world = base.replace("seed=6401", f"seed={seed}") + additions(case)
                path = OUT / f"{case}-{seed}-{load}.toml"
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
                results.append({"case": case, "seed": seed, "load": load, "expected": expected, "answer": answer, "bytes": team.stat().st_size})
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
