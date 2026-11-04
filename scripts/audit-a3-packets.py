#!/usr/bin/env python3
"""Audit exact packet samples in an A3 truth log by named customer range."""

from __future__ import annotations

import argparse
import collections
import ipaddress
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

    ranges: list[tuple[ipaddress.IPv4Network, str]] = []
    for line in args.customers.read_text().splitlines():
        if line.strip():
            prefix, name = line.split(maxsplit=1)
            ranges.append((ipaddress.ip_network(prefix), name))

    def identity(raw: int) -> str:
        address = ipaddress.ip_address(raw)
        matches = [(network.prefixlen, f"{name} [{network}]") for network, name in ranges if address in network]
        return max(matches)[1] if matches else str(address)

    samples = collections.defaultdict(list)
    for tag, body in records(args.team_log.read_bytes()):
        if tag not in (3, 7):
            continue
        minimum = 13 if tag == 3 else 17
        if len(body) < minimum:
            continue
        switch = body[0]
        tick = u16(body, 1)
        source = u32(body, 3)
        destination = u32(body, 7)
        size = u16(body, 11)
        sequence = u32(body, 13) if len(body) >= 17 else 0
        protocol = body[17] if len(body) >= 19 else -1
        ttl = body[18] if len(body) >= 19 else -1
        samples[(identity(source), identity(destination))].append(
            (tick, switch, source, destination, size, sequence, protocol, ttl, tag)
        )

    for (source_name, destination_name), values in sorted(
        samples.items(), key=lambda item: (-len(item[1]), item[0])
    ):
        values.sort()
        source_addresses = sorted({v[2] for v in values})
        destination_addresses = sorted({v[3] for v in values})
        sequences = [v[5] for v in values]
        duplicates = len(sequences) - len(set(sequences))
        exact = collections.Counter((v[2], v[3], v[4], v[5], v[6]) for v in values)
        repeated_tuples = sum(count - 1 for count in exact.values())
        print(f"{source_name} -> {destination_name}")
        print(
            "  "
            f"samples={len(values)} time={values[0][0]/10:.1f}-{values[-1][0]/10:.1f}s "
            f"switches={sorted({v[1] for v in values})} tags={sorted({v[8] for v in values})}"
        )
        print(
            "  "
            f"src-hosts={len(source_addresses)} dst-hosts={len(destination_addresses)} "
            f"sizes={sorted({v[4] for v in values})} "
            f"proto={sorted({v[6] for v in values if v[6] >= 0}) or ['not logged']} "
            f"ttl={min((v[7] for v in values if v[7] >= 0), default='not logged')}-"
            f"{max((v[7] for v in values if v[7] >= 0), default='not logged')}"
        )
        print(
            "  "
            f"seq-zero={sum(seq == 0 for seq in sequences)}/{len(sequences)} "
            f"duplicate-seq={duplicates} repeated-exact-tuples={repeated_tuples}"
        )
        print(
            "  src=" + ",".join(str(ipaddress.ip_address(value)) for value in source_addresses[:12])
            + ("..." if len(source_addresses) > 12 else "")
        )
        print(
            "  dst=" + ",".join(str(ipaddress.ip_address(value)) for value in destination_addresses[:12])
            + ("..." if len(destination_addresses) > 12 else "")
        )
        compact = [
            f"{tick/10:.1f}:{ipaddress.ip_address(src)}>{ipaddress.ip_address(dst)}"
            f"/{size}/q{seq}/p{proto}/t{ttl}/S{switch}"
            for tick, switch, src, dst, size, seq, proto, ttl, _ in values[:24]
        ]
        print("  first=" + " ".join(compact))


if __name__ == "__main__":
    main()
