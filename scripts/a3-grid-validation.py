#!/usr/bin/env python3
"""Generate and validate local A3 truth-run stress worlds on the hidden grid.

This is a development check, not assignment infrastructure.  It recreates the
published 15-switch 3x5 topology, runs the supplied observer, and checks that
all switches can contribute records to the central logger without overflowing
the truth-run log cap.
"""

from __future__ import annotations

import argparse
import collections
import subprocess
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SIM = ROOT / "target/release/competitive_net_sim"
DEFAULT_WASM = ROOT / "submission/truth/switch.wasm"
LOG_CAP = 65_536


def grid_edges() -> list[tuple[int, int]]:
    edges: list[tuple[int, int]] = []
    for row in range(3):
        for col in range(4):
            a = row * 5 + col
            edges.append((a, a + 1))
    for row in range(2):
        for col in range(5):
            a = row * 5 + col
            edges.append((a, a + 5))
    return edges


def world_text(scenario: str) -> str:
    if scenario not in {
        "baseline",
        "failures",
        "small-stress",
        "load-stress",
        "mixed-plants",
    }:
        raise ValueError(scenario)

    size = 64 if scenario == "small-stress" else 200
    rate = 0.02 if scenario in {"baseline", "failures", "small-stress"} else 0.10
    lines = [
        "[world]",
        f'name = "a3-grid-{scenario}"',
        "seed = 3303",
        "version = 1",
        'family = "harness"',
        "",
        "[params]",
        "link_capacity_mbps = 10",
        "link_latency_us = 500",
        "queue_capacity_bytes = 40000",
        "warmup_ms = 3000",
        "duration_ms = 40000",
        "delivery_floor_pct = 0.0",
        "packet_size_bytes = 1000",
        "control_delay_us = 2000",
        "",
    ]
    for switch in range(15):
        lines += ["[[switch]]", f"id = {switch}", ""]
    for a, b in grid_edges():
        lines += ["[[link]]", f"a = {a}", f"b = {b}", ""]

    # Fifty-four sending customers reproduce the first truth run's customer
    # count.  Pair i with i+27 so most traffic crosses several grid hops.
    for i in range(54):
        # Truth Run 2 exposed a roughly 60 kpacket/s, 1000-byte source local
        # to logger S7. Reproduce that specific controller-pressure case.
        app_rate = 4_800.0 if scenario == "load-stress" and i == 7 else rate
        peer = 122 if scenario == "load-stress" and i == 7 else 100 + ((i + 27) % 54)
        lines += [
            "[[app]]",
            f"id = {100 + i}",
            f"switch = {i % 15}",
            f'prefix = "10.{i + 1}.0.0/24"',
            f'name = "Grid customer {i + 1}"',
            'preset = "fortnite"',
            f"rate_mbps = {app_rate:.2f}",
            f"peer = {peer}",
            f"size_bytes = {size}",
            "hosts = [1]",
            "",
        ]

    lines += [
        "[workload]",
        'kind = "mixed"',
        "size_bytes = 1000",
        "",
        "[observe]",
        "punt_events_per_s = 80",
        "punt_backlog = 64",
        "export_bytes_per_s = 16000",
        "export_burst_bytes = 8000",
        "",
        "[harness]",
        "keepalive_ms = 100",
        "log_cap_bytes = 65536",
        "",
    ]

    if scenario == "failures":
        # Exercise direct links into logger S7, including a five-second period
        # when S7 has only its southern link.  Every outage later recovers so
        # HEARD records and route restoration are tested too.
        for a, b, at, duration in [
            (1, 2, 10_000, 18_000),
            (2, 3, 14_000, 12_000),
            (7, 8, 18_000, 8_000),
        ]:
            lines += [
                "[[plant]]",
                'kind = "fail"',
                f"a = {a}",
                f"b = {b}",
                f"at_ms = {at}",
                f"for_ms = {duration}",
                "",
            ]
    elif scenario == "load-stress":
        # Keep the measured high-rate numbered source at S7. Put an additional
        # open-loop side flow and scan at final logger S2 to test its budget.
        lines += [
            "[[plant]]",
            'kind = "side_flow"',
            "app = 102",
            "to = 117",
            "rate_pps = 500.0",
            "size_bytes = 500",
            "start_ms = 5000",
            "stop_ms = 35000",
            'proto = "udp"',
            "",
            "[[plant]]",
            'kind = "scan"',
            "app = 102",
            "to = 117",
            "start_ms = 15000",
            "for_ms = 4000",
            "addresses = 50",
            "size_bytes = 60",
            "",
        ]
    elif scenario == "mixed-plants":
        # One example of every non-failure hidden plant family.  These are
        # deliberately simultaneous: a truth run has every egg present.
        lines += [
            "[[plant]]",
            'kind = "link_delay"',
            "a = 0",
            "b = 1",
            "a_to_b_us = 30000",
            "b_to_a_us = 500",
            "",
            "[[plant]]",
            'kind = "wire_loss"',
            "a = 3",
            "b = 4",
            "loss_pct = 15.0",
            "start_ms = 8000",
            "stop_ms = 30000",
            "",
            "[[plant]]",
            'kind = "link_capacity"',
            "a = 11",
            "b = 12",
            "steps = [[0, 10], [10000, 1], [25000, 10]]",
            "",
            "[[plant]]",
            'kind = "pause"',
            "app = 100",
            "at_ms = 15000",
            "for_ms = 8000",
            "",
            "[[plant]]",
            'kind = "scan"',
            "app = 101",
            "to = 102",
            "start_ms = 12000",
            "for_ms = 4000",
            "addresses = 50",
            "size_bytes = 60",
            "",
            "[[plant]]",
            'kind = "side_flow"',
            "app = 103",
            "to = 104",
            "rate_pps = 30.0",
            "size_bytes = 64",
            "start_ms = 5000",
            "stop_ms = 35000",
            'proto = "udp"',
            "",
            "[[plant]]",
            'kind = "side_flow"',
            "app = 105",
            "to = 106",
            "rate_pps = 30.0",
            "size_bytes = 500",
            "start_ms = 7000",
            "stop_ms = 12000",
            'proto = "tcp"',
            "",
        ]
    return "\n".join(lines)


def records(data: bytes):
    at = 0
    while at + 2 <= len(data):
        tag, size = data[at], data[at + 1]
        at += 2
        if at + size > len(data):
            raise ValueError(f"truncated record at byte {at - 2}")
        yield tag, data[at : at + size]
        at += size
    if at != len(data):
        raise ValueError(f"trailing byte at {at}")


def summarize(data: bytes) -> tuple[dict[str, object], list[str]]:
    counts: collections.Counter[int] = collections.Counter()
    switches: set[int] = set()
    summaries: collections.Counter[tuple[int, int]] = collections.Counter()
    link_gaps = 0
    peak_delay_us = 0
    small_targets: dict[int, set[int]] = collections.defaultdict(set)
    silent: list[tuple[int, int]] = []
    heard: list[tuple[int, int]] = []
    for tag, body in records(data):
        counts[tag] += 1
        if tag != 1 and body:
            switches.add(body[0])
        if tag == 6 and len(body) == 18:
            summaries[(body[0], body[1])] += 1
            link_gaps += int.from_bytes(body[8:10], "little")
            peak_delay_us = max(peak_delay_us, int.from_bytes(body[14:18], "little"))
        elif tag == 3 and len(body) in (13, 19):
            small_targets[int.from_bytes(body[3:7], "little")].add(
                int.from_bytes(body[7:11], "little")
            )
        elif tag == 4 and len(body) == 6:
            silent.append((body[0], body[1]))
        elif tag == 5 and len(body) == 6:
            heard.append((body[0], body[1]))

    expected_directions = {(a, b) for a, b in grid_edges() for a, b in ((a, b), (b, a))}
    missing_directions = sorted(expected_directions - set(summaries))
    result: dict[str, object] = {
        "bytes": len(data),
        "utilization_pct": round(len(data) * 100 / LOG_CAP, 1),
        "records": sum(counts.values()),
        "tag_counts": dict(sorted(counts.items())),
        "switches": sorted(switches),
        "link_directions": len(summaries),
        "link_gaps": link_gaps,
        "peak_delay_us": peak_delay_us,
        "max_small_targets": max(map(len, small_targets.values()), default=0),
        "silent": silent,
        "heard": heard,
    }
    problems: list[str] = []
    if switches != set(range(15)):
        problems.append(f"records arrived from only these switches: {sorted(switches)}")
    if missing_directions:
        problems.append(f"no link summary for directions: {missing_directions}")
    if len(data) >= LOG_CAP:
        problems.append("team log reached the cap")
    if len(data) > 60_000:
        problems.append("team log has less than 5,536 bytes of safety margin")
    return result, problems


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--switch", type=Path, default=DEFAULT_WASM)
    parser.add_argument(
        "--scenario",
        action="append",
        choices=["baseline", "failures", "small-stress", "load-stress", "mixed-plants"],
        help="scenario to run (repeatable; default: all)",
    )
    parser.add_argument("--load-scale", type=float, default=1.0)
    parser.add_argument(
        "--focused",
        action="store_true",
        help="validate the low-rate final truth observer (scan diversity was already learned)",
    )
    parser.add_argument("--keep", type=Path, help="copy generated logs/worlds here")
    args = parser.parse_args()
    scenarios = args.scenario or [
        "baseline",
        "failures",
        "small-stress",
        "load-stress",
        "mixed-plants",
    ]
    if not SIM.exists():
        raise SystemExit(f"missing simulator: build it first ({SIM})")
    if not args.switch.exists():
        raise SystemExit(f"missing switch program: {args.switch}")

    failed = False
    with tempfile.TemporaryDirectory(prefix="a3-grid-") as tmp:
        temp = Path(tmp)
        for scenario in scenarios:
            world = temp / f"{scenario}.toml"
            log = temp / f"{scenario}.team"
            world.write_text(world_text(scenario))
            command = [
                str(SIM),
                "run-world",
                str(world),
                "--program",
                str(args.switch),
                "--team-log",
                str(log),
                "--load-scale",
                str(args.load_scale),
            ]
            subprocess.run(command, cwd=ROOT, check=True, stdout=subprocess.DEVNULL)
            result, problems = summarize(log.read_bytes())
            if scenario in {"baseline", "small-stress", "load-stress"}:
                if result["silent"] or result["heard"]:
                    problems.append("unexpected silent/heard transition")
                if result["tag_counts"].get(6, 0) != 44 * 19:
                    problems.append("did not retain all 836 periodic directed-link summaries")
            if scenario == "load-stress" and result["max_small_targets"] < 8:
                problems.append("logger-local scan retained too little destination evidence under high-rate load")
            if scenario == "failures":
                if len(set(result["silent"])) != 6 or len(set(result["heard"])) != 6:
                    problems.append("did not retain all 6 failure and 6 recovery directions")
            if scenario == "mixed-plants":
                # The planted wire loss also applies to controller reports, so
                # demanding every periodic summary here would reject correct
                # observation of the impairment itself.  Require complete
                # directional coverage and the impairment signatures instead.
                if result["link_directions"] != 44:
                    problems.append("mixed plants lost one or more directed-link views")
                if result["link_gaps"] == 0:
                    problems.append("wire-loss plant produced no recorded keepalive gaps")
                if result["peak_delay_us"] < 25_000:
                    problems.append("30 ms link-delay plant was not visible")
                required_targets = 2 if args.focused else 6
                if result["max_small_targets"] < required_targets:
                    problems.append("50-address scan retained too little destination diversity")
                if result["tag_counts"].get(7, 0) == 0:
                    problems.append("ordinary-size unnumbered side flow produced no packet evidence")
            print(f"{scenario}: {result}")
            for problem in problems:
                print(f"  FAIL: {problem}")
            failed |= bool(problems)
            if args.keep:
                args.keep.mkdir(parents=True, exist_ok=True)
                (args.keep / world.name).write_text(world.read_text())
                (args.keep / log.name).write_bytes(log.read_bytes())
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
