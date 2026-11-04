#!/usr/bin/env python3
"""Exercise the full Home-user surge shape and one-change near misses."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[1]
sys.path.insert(0, str(REPO / "experiments" / "wire-loss"))
from worlds import app, world  # noqa: E402

SIM = REPO / "target" / "release" / "competitive_net_sim"
SWITCH = ROOT / "generated" / "switch.wasm"
DETECT = ROOT / "generated" / "detect.wasm"
OUT = ROOT / "generated" / "audit"


def side_flow(
    *, app_id: int = 100, target: int = 101, rate: float = 3000.0,
    size: int = 64, proto: str = "udp", start: int = 15000, stop: int = 30000,
) -> dict[str, object]:
    return {
        "kind": "side_flow", "app": app_id, "to": target, "rate_pps": rate,
        "size_bytes": size, "proto": proto, "start_ms": start, "stop_ms": stop,
    }


def case_world(case: str, seed: int) -> str:
    site, hosts, normal_size = 2, list(range(1, 65)), 200
    plants: list[dict[str, object]] = []
    receivers = [app(101, 7, hosts=[8])]
    if case == "positive":
        plants.append(side_flow())
    elif case == "off":
        pass
    elif case == "short":
        plants.append(side_flow(stop=20000))
    elif case == "no-recovery":
        plants.append(side_flow(stop=39900))
    elif case == "early":
        plants.append(side_flow(start=5000, stop=20000))
    elif case == "late":
        plants.append(side_flow(start=20000, stop=35000))
    elif case == "too-long":
        plants.append(side_flow(start=8000, stop=32000))
    elif case == "low-rate":
        plants.append(side_flow(rate=400.0))
    elif case == "wrong-size":
        plants.append(side_flow(size=65))
    elif case == "wrong-protocol":
        plants.append(side_flow(proto="tcp"))
    elif case == "four-sources":
        hosts = [1, 2, 3, 4]
        plants.append(side_flow())
    elif case == "wrong-site":
        site = 3
        plants.append(side_flow())
    elif case == "wrong-baseline":
        normal_size = 500
        plants.append(side_flow())
    elif case == "five-destinations":
        for offset in range(5):
            target = 110 + offset
            receivers.append(app(target, 7, hosts=[8]))
            plants.append(side_flow(target=target, rate=600.0))
    else:
        raise ValueError(case)

    apps = [
        app(100, site, 101, size=normal_size, rate=0.16, proto="tcp", hosts=hosts),
        *receivers,
        app(120, 3, 121, size=500, rate=0.63, proto="tcp", echo=True),
        app(121, 8),
        app(122, 4, 123, size=64, rate=0.02, proto="udp", ttl_ladder=8),
        app(123, 9),
    ]
    plants.extend([
        {"kind": "wire_loss", "a": 11, "b": 12, "loss_pct": 7.0, "start_ms": 4000, "stop_ms": 39000},
        {"kind": "link_delay", "a": 1, "b": 2, "a_to_b_us": 2000, "b_to_a_us": 30000},
        {"kind": "link_delay", "a": 13, "b": 14, "a_to_b_us": 25000, "b_to_a_us": 25000},
        {"kind": "fail", "a": 10, "b": 11, "at_ms": 12000, "for_ms": 6000},
    ])
    return world(apps, plants, seed=seed)


CASES = {
    "positive": True,
    "off": False,
    "short": False,
    "no-recovery": False,
    "early": True,
    "late": True,
    "too-long": False,
    "low-rate": False,
    "wrong-size": False,
    "wrong-protocol": False,
    "four-sources": False,
    "wrong-site": False,
    "wrong-baseline": False,
    "five-destinations": False,
}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--reuse-existing", action="store_true", help="reuse logs made by the unchanged observer")
    args = parser.parse_args()
    OUT.mkdir(parents=True, exist_ok=True)
    results = []
    for seed in (9103, 9109, 9127, 9133):
        for load in (1.0, 1.25, 1.5):
            for case, expected in CASES.items():
                path = OUT / f"{case}-{seed}-{load}.toml"
                team = path.with_suffix(".team")
                path.write_text(case_world(case, seed))
                if not (args.reuse_existing and team.exists()):
                    subprocess.run(
                        [str(SIM), "run-world", str(path), "--load-scale", str(load),
                         "--program", str(SWITCH), "--team-log", str(team)],
                        cwd=REPO, check=True, stdout=subprocess.DEVNULL,
                    )
                answer = subprocess.run(
                    [str(SIM), "detect", str(DETECT), str(team)], cwd=REPO,
                    check=True, capture_output=True, text=True,
                ).stdout.strip() == "true"
                results.append({
                    "case": case, "seed": seed, "load": load, "expected": expected,
                    "answer": answer, "bytes": team.stat().st_size,
                })
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
