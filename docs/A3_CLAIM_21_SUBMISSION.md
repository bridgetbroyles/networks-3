# A3 Claim 21: the microburst on link 6→7 (Egg 9)

## Upload exactly these two files

```text
submission/claims/claim-21-microburst-6-7/switch.wasm
submission/claims/claim-21-microburst-6-7/detect.wasm
```

```text
switch.wasm  123,982 bytes  SHA-256 b1689a5ce0fe772346e09b13d0b9dd568a9961c4482944ab00dfcc6fa58a6e20
detect.wasm   31,274 bytes  SHA-256 48f9a0c953aa4d9c36dc5bdd0c73756dcf88224f16eb383af5e9081c59b38795
```

Sources: `a3_burst_switch/` and `a3_detect_burst/`.

## Evidence (S7 hearing S6)

| Run | What the log shows |
|---|---|
| A | Lost keepalives on 6→7; S6 dropped 168 packets on its port toward S7 (9–18.5 s). S7 was overloaded, so few delay summaries |
| B | Peaks of exactly 15,051 µs in windows at 14, 22, 30 and 36 s, otherwise 551 µs |
| C | A 15,051 µs peak in every reported window (6, 12, 30, 38 s); window means 1.3–3.0 ms; 2 lost keepalives per window; S6 dropped 728 packets toward S7 (9.5–37.1 s) |

The reverse direction 7→6 stays at 551 µs. Another team's card confirms Egg 9
"Traffic · The microburst" exists.

## Rule (S6 → S7 only)

1. at least 4 keepalives at least 8 ms slower than the link's fastest, in at
   least 3 different 5-second periods;
2. in at least 60% of 2-second windows (with ≥ 5 keepalives), the mean is
   within 4 ms of the fastest;
3. S6 reports at least 10 drops on its port toward S7.

Applied by hand to run C's numbers it passes: 4 spikes in 4 periods, every
mean within 2.4 ms, 728 drops.

## What the switch logs

Logger S7 punts every keepalive and logs each one at least 5 ms slower than
the link's fastest (`SPIKE`, at most 20 per link per 2 s) and a 2-second
summary per link. Every switch reports its per-port drops each second with
the neighbour's id. Reports go downhill toward S7 with duplicate
suppression. Nothing else is punted.

## Local validation

Microbursts emulated as three synchronised senders (two at S6, one at S5)
bursting at line rate into 6→7, at varying points of the keepalive cycle.
Loads 1.0 / 1.25 / 1.5: **30 of 30 right**.

| Scenario | Expected |
|---|---|
| bursts every 2 s; every 4 s; every 2 s with the other eggs' faults, the Home-users flood and the polite Steam | true |
| off; off + other faults | false |
| the same average sent evenly | false |
| 6→7 congested all the time (mean ~32 ms) | false |
| one burst | false |
| small bursts that never overflow (10–14 ms spikes, no drops) | false |
| the same bursts on 8→7 | false |

Logs 1.7–5.4 KB. The switch completes all three practice truth worlds at all
three loads without a program failure; the detector said no on all 30
practice runs (eggs 21–23).

## Known risks

- If the server's near miss is "a microburst on 6→7 that does not overflow",
  rule 3 handles it; if its true runs ever lack drops, rule 3 misses them.
- If the egg is defined on a different link than 6→7, this misses.
- One of claims 3–18 may already have tested 6-7 (the old switch had a
  `claim_bottleneck_6_7` setting); this rule differs from a plain
  bottleneck test by requiring a low average.
