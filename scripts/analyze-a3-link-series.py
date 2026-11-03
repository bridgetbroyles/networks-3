#!/usr/bin/env python3
"""Print per-period link summaries and transitions from an A3 truth log."""

from __future__ import annotations

import argparse
from collections import defaultdict
from pathlib import Path


def u16(data: bytes, at: int) -> int:
    return int.from_bytes(data[at : at + 2], "little")


def u32(data: bytes, at: int) -> int:
    return int.from_bytes(data[at : at + 4], "little")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("team_log", type=Path)
    parser.add_argument("--edge", required=True, help="receiver-sender, for example 2-1")
    args = parser.parse_args()
    receiver, sender = (int(value) for value in args.edge.split("-"))
    summaries = []
    transitions = []
    data = args.team_log.read_bytes()
    at = 0
    while at + 2 <= len(data):
        tag, size = data[at], data[at + 1]
        at += 2
        if at + size > len(data):
            break
        body = data[at : at + size]
        at += size
        if tag == 6 and len(body) == 18 and body[0] == receiver and body[1] == sender:
            summaries.append(
                (u16(body, 4) / 10, u16(body, 6), u16(body, 8), u32(body, 10), u32(body, 14))
            )
        elif tag in (4, 5) and len(body) == 6 and body[0] == receiver and body[1] == sender:
            transitions.append((u16(body, 4) / 10, "SILENT" if tag == 4 else "HEARD"))
    print(f"S{receiver} receiving from S{sender}")
    print("summaries: time_s received gaps mean_us peak_us")
    for row in sorted(set(summaries)):
        print(" ", *row)
    print("transitions:")
    for row in sorted(set(transitions)):
        print(" ", *row)


if __name__ == "__main__":
    main()
