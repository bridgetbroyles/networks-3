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
    9: "dest-stat",
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
    ap.add_argument("--customers", type=Path, help="truth-run customers.txt")
    args = ap.parse_args()
    data = args.team_log.read_bytes()
    parsed = list(records(data))
    counts = collections.Counter(TAGS.get(tag, f"unknown-{tag}") for tag, _ in parsed)
    print(f"{args.team_log}: {len(data)} bytes, {len(parsed)} records")
    print("records:", ", ".join(f"{name}={n}" for name, n in sorted(counts.items())))

    prefixes: dict[int, str] = {}
    names_by_prefix: dict[str, str] = {}
    customer_ranges: list[tuple[ipaddress.IPv4Network, str]] = []
    if args.customers:
        for line in args.customers.read_text().splitlines():
            if not line.strip():
                continue
            prefix, name = line.split(maxsplit=1)
            network = ipaddress.ip_network(prefix)
            names_by_prefix[str(network)] = name
            customer_ranges.append((network, name))

    def address_name(address: int) -> str:
        ip = ipaddress.ip_address(address)
        matches = [(network.prefixlen, name) for network, name in customer_ranges if ip in network]
        return max(matches)[1] if matches else str(ip)

    stats: dict[int, dict[int, tuple[int, int]]] = collections.defaultdict(dict)
    dest_stats: dict[int, dict[int, tuple[int, int]]] = collections.defaultdict(dict)
    small: dict[int, list[tuple[int, int]]] = collections.defaultdict(list)
    samples: dict[tuple[str, str], list[tuple[int, int, int, int, int]]] = collections.defaultdict(list)
    link_events: list[str] = []
    link_quality: dict[tuple[int, int], list[tuple[int, int, int, int, int]]] = collections.defaultdict(list)
    drops = collections.Counter()

    for tag, body in parsed:
        if tag == 1 and len(body) >= 2:
            if body[0] == 1:
                start, count, offset = 1, body[1], 2
            elif body[0] == 2 and len(body) >= 3:
                start, count, offset = body[1], body[2], 3
            else:
                continue
            for i in range(count):
                base = offset + i * 5
                if base + 5 <= len(body):
                    addr, plen = u32(body, base), body[base + 4]
                    prefixes[start + i] = str(ipaddress.ip_network((addr, plen), strict=False))
        elif tag == 2 and len(body) in (10, 12):
            customer, at = body[1], u16(body, 2)
            old_b, old_p = stats[customer].get(at, (0, 0))
            packet_count = u32(body, 8) if len(body) == 12 else u16(body, 8)
            stats[customer][at] = (old_b + u32(body, 4), old_p + packet_count)
        elif tag == 3 and len(body) in (13, 19):
            at, src, dst = u16(body, 1), u32(body, 3), u32(body, 7)
            small[src].append((at, dst))
            seq = u32(body, 13) if len(body) >= 17 else 0
            proto = body[17] if len(body) >= 19 else -1
            ttl = body[18] if len(body) >= 19 else -1
            samples[(address_name(src), address_name(dst))].append(
                (at, u16(body, 11), seq, proto, ttl)
            )
        elif tag in (4, 5) and len(body) == 6:
            state = "SILENT" if tag == 4 else "HEARD"
            link_events.append(f"{u16(body, 4) / 10:6.1f}s  S{body[0]}->S{body[1]} {state}")
        elif tag == 6 and len(body) == 18:
            link_quality[(body[0], body[1])].append(
                (u16(body, 4), u16(body, 6), u16(body, 8), u32(body, 10), u32(body, 14))
            )
        elif tag == 8 and len(body) == 9:
            drops[(body[0], u16(body, 3))] += u32(body, 5)
        elif tag == 9 and len(body) in (10, 12):
            destination, at = body[1], u16(body, 2)
            byte_count = u32(body, 4)
            packet_count = u32(body, 8) if len(body) == 12 else u16(body, 8)
            old_b, old_p = dest_stats[destination].get(at, (0, 0))
            dest_stats[destination][at] = (old_b + byte_count, old_p + packet_count)
        elif tag == 7 and len(body) in (17, 19):
            at, src, dst = u16(body, 1), u32(body, 3), u32(body, 7)
            proto = body[17] if len(body) >= 19 else -1
            ttl = body[18] if len(body) >= 19 else -1
            samples[(address_name(src), address_name(dst))].append(
                (at, u16(body, 11), u32(body, 13), proto, ttl)
            )

    if prefixes:
        print("\ncustomer ranges")
        for customer, prefix in prefixes.items():
            name = names_by_prefix.get(prefix, "")
            print(f"  C{customer}: {prefix}" + (f"  {name}" if name else ""))

    if stats:
        print("\ncustomer activity (counter records)")
        for customer, windows in sorted(stats.items()):
            values = list(windows.values())
            total_b = sum(v[0] for v in values)
            total_p = sum(v[1] for v in values)
            times = sorted(windows)
            steps = [b - a for a, b in zip(times, times[1:])]
            nominal_ticks = statistics.median(steps) if steps else 5
            interval_s = nominal_ticks / 10
            rates = [v[0] * 8 / interval_s / 1_000_000 for v in values]
            gap_ms = max((step - nominal_ticks) * 100 for step in steps) if steps else 0
            avg_size = total_b / total_p if total_p else 0
            print(
                f"  C{customer:<2} {prefixes.get(customer, '?'):<18} "
                f"{names_by_prefix.get(prefixes.get(customer, ''), ''):<24} "
                f"active={len(values):>2} interval~{interval_s:3.1f}s "
                f"median={statistics.median(rates):5.2f} Mb/s "
                f"avg-packet={avg_size:6.1f} B longest-gap~{max(gap_ms, 0):>5} ms"
            )

    if dest_stats:
        print("\ndestination activity")
        for destination, windows in sorted(dest_stats.items()):
            byte_count = sum(v[0] for v in windows.values())
            packet_count = sum(v[1] for v in windows.values())
            prefix = prefixes.get(destination, "unknown")
            name = names_by_prefix.get(prefix, prefix)
            avg = byte_count / packet_count if packet_count else 0
            print(
                f"  C{destination}: {name}; active={len(windows)}, "
                f"bytes={byte_count}, packets={packet_count}, avg={avg:.1f} B"
            )

    if link_events:
        print("\nlink state changes")
        for line in link_events:
            print(" ", line)

    if link_quality:
        print("\ntruth-mode link quality")
        for (a, b), link_samples in sorted(link_quality.items()):
            received = sum(v[1] for v in link_samples)
            gaps = sum(v[2] for v in link_samples)
            avg = statistics.mean(v[3] for v in link_samples if v[1] > 0) if received else 0
            peak = max(v[4] for v in link_samples)
            zeros = sum(v[1] == 0 for v in link_samples)
            print(
                f"  S{a}->S{b}: periods={len(link_samples)}, keepalives={received}, gaps={gaps}, "
                f"zero-periods={zeros}, mean-delay={avg:.0f} us, peak={peak} us"
            )

    if small:
        print("\nsmall-packet destination diversity")
        for src, events in sorted(small.items()):
            best = 0
            events.sort()
            for i, (start, _) in enumerate(events):
                best = max(best, len({dst for at, dst in events[i:] if at - start <= 50}))
            print(f"  {ipaddress.ip_address(src)}: {len(events)} packets, max {best} distinct targets in 5 s")

    if samples and args.customers:
        print("\nsampled named flows")
        for (source, destination), events in sorted(samples.items(), key=lambda item: -len(item[1])):
            times = [event[0] for event in events]
            sizes = [event[1] for event in events]
            protocols = sorted({event[3] for event in events if event[3] >= 0})
            ttls = sorted({event[4] for event in events if event[4] >= 0})
            detail = ""
            if protocols:
                detail += f", proto={protocols}"
            if ttls:
                detail += f", ttl={min(ttls)}-{max(ttls)}"
            print(
                f"  {source} -> {destination}: samples={len(events)}, "
                f"time={min(times)/10:.1f}-{max(times)/10:.1f}s, "
                f"size={min(sizes)}-{max(sizes)} B{detail}"
            )

    if drops:
        print("\nport drops")
        for (switch, port), count in sorted(drops.items()):
            print(f"  S{switch} port {port}: {count}")


if __name__ == "__main__":
    main()
