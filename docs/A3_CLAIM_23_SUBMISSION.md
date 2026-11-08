# A3 Claim 23: the breathing link 12-13 (Egg 15)

## Upload exactly these two files

```text
submission/claims/claim-23-breathing-link-12-13/switch.wasm
submission/claims/claim-23-breathing-link-12-13/detect.wasm
```

```text
switch.wasm  106,872 bytes  SHA-256 c8022030819d26571125fc074d43290c077b1495acff8cd4c435daf756020fc7
detect.wasm   33,708 bytes  SHA-256 579ee5ac22fb345f4bd9721962614f420cd6cec22e979638375d636809e45f70
```

Sources: `a3_breath_switch/` and `a3_detect_breath/`.

## Evidence

On a quiet direction a keepalive's delay is the link latency (500 µs) plus
64 bytes at the link's capacity: 551 µs at 10 Mb/s, exactly 628 µs at
4 Mb/s. The simulator's capacity steps are whole Mb/s.

Truth run C, S12 hearing S13 (13→12, the quiet direction):

| Window | Mean / max | Capacity |
|---|---|---|
| 0–2 s | 551 / 551 | 10 Mb/s |
| 4–6 s | 628 / 628 (all 20 keepalives) | 4 Mb/s |
| 6–8 s | 587 / 628 | half each |
| 8–10 s | 552 / 575 | 10 Mb/s |
| 22–24 s | 585 / 628 | half each |
| 28–30 s | 631 / 664 | 4 Mb/s |

The busy direction 12→13 peaks at 80 ms (a full 40 KB queue at 4 Mb/s)
except in the 8–10 s window (19 ms). No other link in any truth run shows
an exact capacity shift. Another team's card confirms Egg 15 "Link · The
breathing link" exists.

## Rule

Per second, the fastest keepalive S12 heard from S13:

- fast if within 12 µs of the fastest S12 heard from anyone all run;
- slow only if within ±3 µs of latency + 512/C µs for a whole C of 1–9
  Mb/s (628 at 4, 756 at 2, …) — queueing lands on arbitrary values and is
  ignored;
- one-second runs dropped as noise;
- true if two separate slow runs remain with a fast run between them.

## Switch

Only S12 (the logger) does anything: it punts every keepalive and writes,
per neighbour per second, the count and fastest and slowest delay. Every
other switch runs an empty pipeline and sends nothing. Log: 1,638 bytes.

## Validation

Crowded replica of the hidden network (~35 named customers at their truth
switches and rates, Backup's 8 Mb/s across 12→13, return traffic across
13→12, 20 fillers). Loads 1.0 / 1.25 / 1.5: **55 of 60 right**.

| Scenario | Expected | Result |
|---|---|---|
| breathing like run C; regular 5 s; down to 2 Mb/s; with other eggs' faults and the Home-users flood | true | right at all loads |
| breathing + on/off congestion on the quiet direction | true | right at 1.25; missed at 1.0, 1.5 |
| breathing + heavy constant traffic on the quiet direction | true | missed at all loads |
| off; off + faults; off + heavy reverse traffic | false | right |
| narrow all run; narrows once; dips once and recovers | false | right |
| breathing on 11-12, 7-12, 8-13, 13-14 | false | right |
| on/off congestion (heavy, moderate, light) either direction, constant capacity | false | right |

The five misses are stress cases where the quiet direction is saturated.
That does not match the hidden network: run C (every egg on) measured that
direction at exactly 551 and 628 µs. The switch completes all practice truth
worlds at all loads without failure; the detector said no on all 30
practice runs.

## Remaining risks

- If the claim runs' breathing has only one low period, this misses.
- If the server's near miss narrows 12-13 twice (e.g. "breathing too
  slowly"), this says true.
