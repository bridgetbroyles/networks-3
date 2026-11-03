#!/usr/bin/env python3
"""Print source-counter time series from an A3 truth log."""

from __future__ import annotations

import argparse
import ipaddress
from collections import defaultdict
from pathlib import Path


def u16(data: bytes, at: int) -> int:
    return int.from_bytes(data[at : at + 2], "little")


def u32(data: bytes, at: int) -> int:
    return int.from_bytes(data[at : at + 4], "little")


def records(data: bytes):
    at = 0
    while at + 2 <= len(data):
        tag, size = data[at], data[at + 1]
        at += 2
        if at + size > len(data):
            return
        yield tag, data[at : at + size]
        at += size


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("team_log", type=Path)
    parser.add_argument("customers", type=Path)
    args = parser.parse_args()

    names = {}
    for line in args.customers.read_text().splitlines():
        prefix, name = line.split(maxsplit=1)
        names[str(ipaddress.ip_network(prefix))] = name

    prefixes: dict[int, str] = {}
    series = defaultdict(list)
    for tag, body in records(args.team_log.read_bytes()):
        if tag == 1 and len(body) >= 2:
            if body[0] == 1:
                start, count, offset = 1, body[1], 2
            elif body[0] == 2 and len(body) >= 3:
                start, count, offset = body[1], body[2], 3
            else:
                continue
            for index in range(count):
                base = offset + 5 * index
                if base + 5 <= len(body):
                    prefixes[start + index] = str(
                        ipaddress.ip_network((u32(body, base), body[base + 4]), strict=False)
                    )
        elif tag == 2 and len(body) in (10, 12):
            packets = u32(body, 8) if len(body) == 12 else u16(body, 8)
            series[(body[1], body[0])].append((u16(body, 2), u32(body, 4), packets))

    for (customer, switch), values in sorted(series.items()):
        values.sort()
        prefix = prefixes.get(customer, "unknown")
        label = names.get(prefix, prefix)
        times = [value[0] / 10 for value in values]
        gaps = [round(times[i + 1] - times[i], 1) for i in range(len(times) - 1)]
        sizes = [round(byte_count / packets, 1) for _, byte_count, packets in values if packets]
        counts = [packets for _, _, packets in values]
        print(f"C{customer:02d} S{switch:02d} {label}")
        print(f"  times_s={times}")
        print(f"  gaps_s={gaps}")
        print(f"  packets={counts}")
        print(f"  avg_sizes={sizes}")


if __name__ == "__main__":
    main()
