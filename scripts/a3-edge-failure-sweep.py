#!/usr/bin/env python3
"""Fail and recover every hidden-grid edge against one exact A3 Wasm."""

from __future__ import annotations

import argparse
import importlib.util
import subprocess
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SIM = ROOT / "target/release/competitive_net_sim"
DEFAULT_WASM = ROOT / "submission/truth/switch.wasm"


def load_grid_validator():
    path = ROOT / "scripts/a3-grid-validation.py"
    spec = importlib.util.spec_from_file_location("a3_grid_validation", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot import {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--switch", type=Path, default=DEFAULT_WASM)
    parser.add_argument("--load-scale", type=float, default=1.5)
    args = parser.parse_args()

    grid = load_grid_validator()
    failed = False
    with tempfile.TemporaryDirectory(prefix="a3-edge-sweep-") as tmp:
        work = Path(tmp)
        for a, b in grid.grid_edges():
            world = work / f"edge-{a}-{b}.toml"
            log = work / f"edge-{a}-{b}.team"
            text = grid.world_text("baseline") + "\n" + "\n".join(
                [
                    "[[plant]]",
                    'kind = "fail"',
                    f"a = {a}",
                    f"b = {b}",
                    "at_ms = 8000",
                    "for_ms = 12000",
                    "",
                ]
            )
            world.write_text(text)
            run = subprocess.run(
                [
                    str(SIM),
                    "run-world",
                    str(world),
                    "--program",
                    str(args.switch.resolve()),
                    "--team-log",
                    str(log),
                    "--load-scale",
                    str(args.load_scale),
                ],
                cwd=ROOT,
                capture_output=True,
                text=True,
            )
            problems: list[str] = []
            if run.returncode != 0 or not log.exists():
                problems.append("simulation did not complete")
                result = {"bytes": 0, "link_directions": 0, "switches": []}
            else:
                result, problems = grid.summarize(log.read_bytes())
                expected = {(a, b), (b, a)}
                if set(result["silent"]) != expected:
                    problems.append(f"SILENT directions were {sorted(set(result['silent']))}")
                if set(result["heard"]) != expected:
                    problems.append(f"HEARD directions were {sorted(set(result['heard']))}")
                if result["link_directions"] != 44:
                    problems.append("did not retain all 44 directed-link views")
                if result["switches"] != list(range(15)):
                    problems.append("did not retain records from all 15 switches")
            status = "PASS" if not problems else "FAIL"
            print(
                f"{status} S{a}-S{b}: bytes={result['bytes']} "
                f"directions={result['link_directions']} switches={len(result['switches'])}"
            )
            for problem in problems:
                print(f"  {problem}")
            failed |= bool(problems)
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
