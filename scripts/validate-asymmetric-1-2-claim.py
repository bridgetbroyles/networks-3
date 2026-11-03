#!/usr/bin/env python3
"""Adversarial local checks for the graded asymmetric link-1--2 claim."""

from __future__ import annotations

import importlib.util
import subprocess
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SIM = ROOT / "target/release/competitive_net_sim"
SWITCH = ROOT / "a3_switch/target/wasm32-unknown-unknown/release/a3_switch.wasm"
DETECT = ROOT / "a3_detect_asymmetric_1_2/target/wasm32-unknown-unknown/release/a3_detect_asymmetric_1_2.wasm"


def load_grid_generator():
    path = ROOT / "scripts/a3-grid-validation.py"
    spec = importlib.util.spec_from_file_location("a3_grid_validation", path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def delay(a: int, b: int, a_to_b_us: int, b_to_a_us: int) -> str:
    return f"""
[[plant]]
kind = "link_delay"
a = {a}
b = {b}
a_to_b_us = {a_to_b_us}
b_to_a_us = {b_to_a_us}
"""


def main() -> int:
    grid = load_grid_generator()
    baseline = grid.world_text("baseline")
    twenty = grid.world_text("twenty-plants")
    signature = delay(1, 2, 2_000, 30_000)
    cases = {
        "true-asymmetric-target": (baseline + signature, "true"),
        "true-with-all-other-plants": (twenty + signature, "true"),
        "false-normal": (baseline, "false"),
        "false-only-slow-side": (baseline + delay(1, 2, 500, 30_000), "false"),
        "false-symmetric-slow": (baseline + delay(1, 2, 30_000, 30_000), "false"),
        "false-reversed": (baseline + delay(1, 2, 30_000, 2_000), "false"),
        "false-other-link": (baseline + delay(5, 6, 2_000, 30_000), "false"),
        "false-sub-threshold": (baseline + delay(1, 2, 2_000, 15_000), "false"),
    }

    with tempfile.TemporaryDirectory(prefix="a3-claim-asymmetric-link-") as raw:
        temp = Path(raw)
        for name, (world_text, expected) in cases.items():
            for load in (1.0, 1.25, 1.5):
                suffix = str(load).replace(".", "-")
                world = temp / f"{name}-{suffix}.toml"
                log = temp / f"{name}-{suffix}.team"
                world.write_text(world_text)
                subprocess.run(
                    [
                        str(SIM),
                        "run-world",
                        str(world),
                        "--program",
                        str(SWITCH),
                        "--team-log",
                        str(log),
                        "--load-scale",
                        str(load),
                    ],
                    cwd=ROOT,
                    check=True,
                    stdout=subprocess.DEVNULL,
                )
                answer = subprocess.run(
                    [str(SIM), "detect", str(DETECT), str(log)],
                    cwd=ROOT,
                    check=True,
                    capture_output=True,
                    text=True,
                ).stdout.strip()
                if answer != expected:
                    raise SystemExit(f"{name} at {load}x: expected {expected}, got {answer}")
                if log.stat().st_size >= 24_576:
                    raise SystemExit(
                        f"{name} at {load}x: claim log is too large: {log.stat().st_size} bytes"
                    )
                print(f"PASS {name} at {load}x: {answer}, {log.stat().st_size} log bytes")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
