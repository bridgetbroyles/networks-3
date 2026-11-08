# A3 Claim 19: many-to-one small-UDP flood

## Upload exactly these two files

Upload both together to **A3 — Claim**, after checking the server still
accepts claims (SUBMITTING.md lists the deadline as Nov 3):

```text
submission/claims/claim-19-flood-to-one-address/switch.wasm
submission/claims/claim-19-flood-to-one-address/detect.wasm
```

```text
switch.wasm  120,379 bytes  SHA-256 59a9e7f3f21d18fa0bdd8708b47bef9873336b18935061b791bc2b8a68223817
detect.wasm   57,696 bytes  SHA-256 0b20f8f8bd303f1701c93a1ced63ea1c88283714b20e02f91d7e609121767d28
```

Sources: `a3_flood_switch/` and `a3_detect_flood/`.

## Evidence from the truth runs

| Run | What the log shows |
|---|---|
| A | 7 distinct Home-user hosts → 10.50.151.8 (Fastly), 64 B, 16.7–17.2 s, after 1-in-8 sampling |
| B | Not visible: logger S7 was overloaded and kept only 17 small-packet samples |
| C | 22 distinct Home-user hosts → 10.50.151.8, 64 B UDP, 15.0–25.9 s; Home users' rate 0.17 → 1.71 Mb/s and average packet 200 → 69 B from 15.5 to 30.6 s (≈3,000 packets/s) |

No normal traffic comes close: the largest small-packet fan-in is
Cloudflare users at 8 hosts → 8 addresses (TCP, 100 B). The won Traffic
eggs do not cover it (traceroute = Mapper with TTL 1, echo = Ping). Another
team's card names the Traffic eggs 7–13; this is either Egg 11 (Cloudflare
DDoS) or the unnamed Egg 12. The detector does not depend on the target's
identity, so it can win either.

## Hypothesis and rule

Many hosts send small UDP packets to one address, at high rate, for many
seconds. The detector is true only if one destination address has:

1. at least 24 distinct UDP sources of packets under 128 B, first seen
   within 5 s;
2. its range receiving at least 500 small-UDP packets/s in at least 5 of
   some 10 one-second buckets that overlap those sources' window.

## What the switch logs

Logger S2 (the flood's ingress switch in the truth runs). Every switch:

- punts only the first two packets of each small source/destination pair
  from a customer port (data-plane register), so the flood costs about
  one punt per host;
- counts every small UDP packet per destination range in the data plane and
  reports the per-second delta;
- punts keepalives only until it has learned its switch ports;
- floods reports over switch ports with duplicate suppression.

## Local validation

Grid replica (15 switches, Home-users-like 128-host customer at S2,
Fastly-like target at S7, Cloudflare-like, Ping-like and 40 filler
customers), loads 1.0 / 1.25 / 1.5: **33 of 33 right**.

| Scenario | Expected | Senders | Busy s | Peak pps |
|---|---|---|---|---|
| flood 15–30 s | true | 73 | 15 | 2,008 |
| flood + loss 1-2, slow 13-14 and 1-2, SRLG fail 0-1/10-11, narrow 12-13 | true | 73 | 15 | 2,008 |
| flood 30–39 s | true | 73 | 9 | 2,008 |
| off; off + other faults | false | 1 | 0 | 42 |
| one host floods | false | 1 | 0 | 42 |
| 2 s burst | false | 73 | 2 | 2,008 |
| 100 pps for 15 s | false | 128 | 0 | 101 |
| 3 pps over the whole run | false | 16 | 0 | 3 |
| scan of 60 addresses in 4 s | false | 1 | 0 | 42 |
| same flood over TCP | false | — | 0 | — |

Logs stayed between 1.9 and 6.2 KB (cap 24 KB). The switch completes all
three practice truth worlds at all three loads with no program failure,
and the practice checker loads both files (all ten egg-23 scanner runs
answered false, as a flood detector should).

## Known risks

- If the egg's near miss is "the same flood against a different
  customer", this generic detector says true on it and misses.
- If claim runs' flood is under ~500 packets/s or shorter than 5 s, the
  rate check rejects it. Truth run C measured about 3,000 packets/s for
  about 15 s.
- Simulated locally only; the hidden network's exact rates and other eggs
  cannot be reproduced.
