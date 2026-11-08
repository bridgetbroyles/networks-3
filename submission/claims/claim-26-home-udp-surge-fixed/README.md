# Claim 26 candidate: Home UDP surge v2, detector fixed for load 1.0

Upload these two files together:

```text
switch.wasm  131,923 bytes  SHA-256 25dca8a1089ea10aec5edc295584d7da879a2ea294fb666dbaaaa81ae871fb72  (unchanged from submission/next/claim-19-home-udp-surge)
detect.wasm   SHA-256 e98d78fc60ff2b859acad962f2ce9e9c385bb3ede36230e91cad62df1bceca9c  (fixed)
```

## The bug

`submission/next/claim-19-home-udp-surge` misses the true flood at load 1.0.

- The flood is a planted side flow scaled by load. Truth run C measured
  3,000 packets/s, and run C was load 1.5 (Netflix's rate in run C vs run B is
  exactly 1.5 / 1.25), so the base rate is about 2,000/s. The audit
  (`experiments/home-udp-surge-v2/audit.py`, `rate: float = 3000.0`) planted
  3,000/s at load 1.0 and so never tested the real load-1.0 rate.
- In this mode the observer samples each host pair's first two packets, then
  every 256th. With ~128 Home-user hosts, each pair carries ~15.6 packets/s at
  load 1.0, so the 256th packet would come ~16.4 s in, after the 15 s flood
  ends. Only start-of-flood samples exist.
- The detector required those samples to span at least 8 s
  (`last_packet - first_packet >= 80`) and the last one to be within 5 s of
  the surge's end (`last_packet + 50 >= v[end].t`). Both fail at load 1.0.

Measured with the unchanged pair on a 15-switch replica, load 1.0: flood at
2,000/s → false; 2,500/s → true; 3,000/s → true. At loads 1.25 and 1.5 it was
right.

## The fix (detector only; see detector.patch)

Remove those two sample-timing conditions. The switch's counters already
give the surge's start, length (9–18 s) and end; samples still decide the
senders (≥ 5) and exactly one destination, and must start with the surge.
Everything else is unchanged.

## Validation

- A rebuild of the unmodified detector from this source gives the same answer
  as the published `detect.wasm` on all 51 + 3 test logs (hashes differ only
  by toolchain/paths).
- Fixed pair, loads 1.0 / 1.25 / 1.5, 42 of 42 right: true for a 15 s flood
  (also with other eggs' faults), a 12 s flood, and Home users flooding another
  single target; false for off, a 24 s flood, a flood that never stops, 7 s,
  2 s, 300 packets/s, one host, another customer's hosts, a TCP flood.
- Truth run C: original and fixed both true (run C was load 1.5).
- Loads in the practice checker; no on all practice runs.

The flood has been tried three times (Claims 8, 20, 22), none of which
rejected a flood that lasts too long; this is the first that does and also
works at load 1.0.
