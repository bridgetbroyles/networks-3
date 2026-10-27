#!/usr/bin/env python3
"""Summarize a3_switch's compact binary team log."""

from __future__ import annotations

import argparse
import collections
import ipaddress
import statistics
from pathlib import Path

TAGS = {
    1: "header",
    2: "stat",
    3: "small-packet",
    4: "link-silent",
    5: "link-heard",
    6: "link-summary",
    7: "packet-sample",
    8: "port-drops",
}


def u16(b: bytes, at: int) -> int:
    return int.from_bytes(b[at : at + 2], "little")


def u32(b: bytes, at: int) -> int:
    return int.from_bytes(b[at : at + 4], "little")


def records(data: bytes):
    at = 0
    while at + 2 <= len(data):
        tag, size = data[at], data[at + 1]
        at += 2
        if at + size > len(data):
            print(f"truncated record at byte {at - 2}")
            return
        yield tag, data[at : at + size]
        at += size


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("team_log", type=Path)
    args = ap.parse_args()
    data = args.team_log.read_bytes()
    parsed = list(records(data))
    counts = collections.Counter(TAGS.get(tag, f"unknown-{tag}") for tag, _ in parsed)
    print(f"{args.team_log}: {len(data)} bytes, {len(parsed)} records")
    print("records:", ", ".join(f"{name}={n}" for name, n in sorted(counts.items())))

    prefixes: dict[int, str] = {}
    stats: dict[int, dict[int, tuple[int, int]]] = collections.defaultdict(dict)
    small: dict[int, list[tuple[int, int]]] = collections.defaultdict(list)
    link_events: list[str] = []
    link_quality: dict[tuple[int, int], list[tuple[int, int, int]]] = collections.defaultdict(list)
    drops = collections.Counter()

    for tag, body in parsed:
        if tag == 1 and len(body) >= 2:
            count = body[1]
            for i in range(count):
                base = 2 + i * 5
                if base + 5 <= len(body):
                    addr, plen = u32(body, base), body[base + 4]
                    prefixes[i + 1] = str(ipaddress.ip_network((addr, plen), strict=False))
        elif tag == 2 and len(body) == 10:
            customer, at = body[1], u16(body, 2)
            old_b, old_p = stats[customer].get(at, (0, 0))
            stats[customer][at] = (old_b + u32(body, 4), old_p + u16(body, 8))
        elif tag == 3 and len(body) == 13:
            at, src, dst = u16(body, 1), u32(body, 3), u32(body, 7)
            small[src].append((at, dst))
        elif tag in (4, 5) and len(body) == 6:
            state = "SILENT" if tag == 4 else "HEARD"
            link_events.append(f"{u16(body, 4) / 10:6.1f}s  S{body[0]}->S{body[1]} {state}")
        elif tag == 6 and len(body) == 18:
            link_quality[(body[0], body[1])].append((u16(body, 6), u32(body, 10), u32(body, 14)))
        elif tag == 8 and len(body) == 9:
            drops[(body[0], u16(body, 3))] += u32(body, 5)

    if prefixes:
        print("\ncustomer ranges")
        for customer, prefix in prefixes.items():
            print(f"  C{customer}: {prefix}")

    if stats:
        print("\ncustomer activity (500 ms records)")
        for customer, windows in sorted(stats.items()):
            values = list(windows.values())
            total_b = sum(v[0] for v in values)
            total_p = sum(v[1] for v in values)
            rates = [v[0] * 16 / 1_000_000 for v in values]
            times = sorted(windows)
            gap_ms = max((b - a) * 100 - 500 for a, b in zip(times, times[1:])) if len(times) > 1 else 0
            avg_size = total_b / total_p if total_p else 0
            print(
                f"  C{customer:<2} {prefixes.get(customer, '?'):<18} "
                f"active={len(values):>2} median={statistics.median(rates):5.2f} Mb/s "
                f"avg-packet={avg_size:6.1f} B longest-gap~{max(gap_ms, 0):>5} ms"
            )

    if link_events:
        print("\nlink state changes")
        for line in link_events:
            print(" ", line)

    if link_quality:
        print("\ntruth-mode link quality")
        for (a, b), samples in sorted(link_quality.items()):
            received = sum(v[0] for v in samples)
            avg = statistics.mean(v[1] for v in samples)
            peak = max(v[2] for v in samples)
            print(f"  S{a}->S{b}: keepalives={received}, mean-delay={avg:.0f} us, peak={peak} us")

    if small:
        print("\nsmall-packet destination diversity")
        for src, events in sorted(small.items()):
            best = 0
            events.sort()
            for i, (start, _) in enumerate(events):
                best = max(best, len({dst for at, dst in events[i:] if at - start <= 50}))
            print(f"  {ipaddress.ip_address(src)}: {len(events)} packets, max {best} distinct targets in 5 s")

    if drops:
        print("\nport drops")
        for (switch, port), count in sorted(drops.items()):
            print(f"  S{switch} port {port}: {count}")


if __name__ == "__main__":
    main()
