#!/usr/bin/env python3
"""Run the focused fanout pair on positive and adversarial network worlds."""

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


def source(app_id: int, switch: int, prefix: str, peer: int, size: int = 200) -> str:
    return f'''\n[[app]]
id={app_id}
switch={switch}
prefix="{prefix}"
name="Fanout source {app_id}"
hosts=[1]
preset="jane_street"
rate_mbps=0.1
peer={peer}
size_bytes={size}
proto="tcp"
'''


def receiver(app_id: int, offset: int) -> str:
    return f'''\n[[app]]
id={app_id}
switch=14
prefix="10.61.0.{offset * 4}/30"
name="Fanout receiver {app_id}"
'''


def side_flow(app_id: int, target: int, at_ms: int, size: int) -> str:
    return f'''\n[[plant]]
kind="side_flow"
app={app_id}
to={target}
start_ms={at_ms}
stop_ms={at_ms + 1000}
rate_pps=2.0
size_bytes={size}
proto="tcp"
'''


def additions(case: str) -> str:
    count = 50
    site = 7
    size = 200
    spacing = 80
    split = False
    enabled = True
    if case == "off":
        enabled = False
    elif case == "forty-nine":
        count = 49
    elif case == "slow-fifty":
        spacing = 200
    elif case == "wrong-size-199":
        size = 199
    elif case == "wrong-size-201":
        size = 201
    elif case == "wrong-site":
        site = 6
    elif case == "split-sources":
        split = True
    elif case != "positive":
        raise ValueError(case)

    target_ids = list(range(301, 351))
    text = source(300, site, "10.60.1.0/24", target_ids[0], size)
    if split:
        text += source(360, site, "10.60.2.0/24", target_ids[25], size)
    for offset, target in enumerate(target_ids):
        text += receiver(target, offset)
        if enabled and offset < count:
            sender = 360 if split and offset >= 25 else 300
            text += side_flow(sender, target, 4000 + spacing * offset, size)
    return text


CASES = {
    "positive": True,
    "off": False,
    "forty-nine": False,
    "slow-fifty": False,
    "wrong-size-199": False,
    "wrong-size-201": False,
    "wrong-site": False,
    "split-sources": False,
}


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    base = BASE.read_text()
    results = []
    for seed in (8101, 8111, 8123, 8147):
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
        "passed": sum(r["expected"] == r["answer"] for r in results),
        "largest_log_bytes": max(r["bytes"] for r in results),
        "results": results,
    }
    (ROOT / "audit-results.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps({k: summary[k] for k in ("cases", "passed", "largest_log_bytes")}, indent=2))
    if summary["passed"] != summary["cases"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
