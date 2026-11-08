# A3 Claim 20: Steam polite downloader (Egg 6)

## Upload exactly these two files

Upload both together to **A3 — Claim**, after Claim 19's reply has come
back, and after checking the server still accepts claims:

```text
submission/claims/claim-20-steam-polite/switch.wasm
submission/claims/claim-20-steam-polite/detect.wasm
```

```text
switch.wasm  121,819 bytes  SHA-256 25a084dbfc9dab7d10c221051d477039320c62be692514b9665cca26ad720e30
detect.wasm   61,572 bytes  SHA-256 145eeb7f0d6f2c67fd4d74fc56d19559a85f5afcb4af1618c25e84a016ec4ba7
```

Sources: `a3_steam_switch/` and `a3_detect_steam/`.

## Evidence

Truth run B (logger at S7, where Steam and both Amazon
shopper sources attach) shows what "polite" means here:

| Time | Amazon shoppers A / B (Mb/s) | Steam (Mb/s) |
|---|---|---|
| 2–11 s | 1.28 / 0–0.97 | 0.63 |
| 12.5–17 s | both off | 4.0 → 10.0 → 3.4 |
| 18.5–27.6 s | 1.28 / 0.97 | 0.63 |
| 29.1–32.1 s | both off | 7.6 → 10.0 → 9.3 |
| 33.6–39.6 s | 1.28 / 0–0.97 | 0.63 |

Amazon uses at most 2.3 Mb/s, yet Steam stays at 0.63 Mb/s whenever either
source sends, and fills its 10 Mb/s only while both are silent. Run A's
Steam samples (12.4–15.6 s and 28.6–30.7 s, about 1,300 packets/s) and run
C's 4.1 Mb/s at 12.5 s match. On run B's numbers the rule below gives:
5 high windows, all with competitors quiet (median 0.10 Mb/s); 19 low
windows with competitors at a median of 1.43 Mb/s and the total at 2.06
Mb/s. No other bulk customer in runs B or C has both high and low phases.

## Rule

For each bulk flow (average packet ≥ 900 B) per switch and egress port, in
one-second buckets:

- at least 3 seconds at ≥ 5 Mb/s and at least 8 at ≤ 1.5 Mb/s;
- in ≥ 70% of high seconds, competitors send ≤ 0.8 Mb/s;
- in low seconds, competitors' median ≥ 0.5 Mb/s, and flow + competitors
  ≤ 6 Mb/s (it backs off while there is room).

Competitors are the other customers leaving the same switch by the same
egress port, plus those on other ports that pause at some point. So
Cloudflare's constant 33 Mb/s at S7, on another port, is not a competitor,
but a constant flow sharing Steam's port is.

## What the switch logs

Logger S7. Every switch counts bytes and packets of customer data in the
data plane per (source customer, harness egress port), and once a second
reports each pair that sent ≥ 0.4 Mb/s. Reports travel to every neighbour
closer to S7 on the published grid (plus same-distance neighbours when only
one is closer), with duplicate suppression; switches stagger their timers.
S7 stops accepting remote reports past 20 KB, keeping the rest of the cap
for its own records.

## Local validation

Grid replica with a Steam at S7 (0.63 Mb/s plus 10 Mb/s bursts when both
Amazon-like on/off sources are idle), Cloudflare-like constant 4 Mb/s on
another port, Home users / Fastly, and 40 filler customers. Loads 1.0 /
1.25 / 1.5: **24 of 24 right**.

| Scenario | Expected |
|---|---|
| polite Steam | true |
| polite Steam + loss 1-2, slow 13-14 and 1-2, SRLG, narrow 12-13, Home-users flood | true |
| polite Steam, Amazon on a different egress port | true |
| real `steam` preset (takes 10 Mb/s throughout) | false |
| no Steam | false |
| Steam never surges | false |
| Steam surges only while Amazon is sending | false |
| Steam bursts while a constant 7.6 Mb/s flow fills its port | false |

Logs 2.3–3.1 KB locally; about 11 KB expected on the hidden network (cap
24 KB). The switch completes all three practice truth worlds at all three
loads; the practice checker loads both files, and the detector answered no
on all 30 practice runs (eggs 21–23), including the practice network's
ordinary Steam.

## Known risks

- The polite behaviour was emulated with planted bursts; the hidden
  mechanism may differ (for example, a Steam that is only somewhat slower
  under competition, not 16× slower).
- The detector does not check that the polite flow is Steam: a different
  polite bulk customer in a near miss would fool it.
- If claim runs' Steam surge is shorter than 3 seconds above 5 Mb/s, it
  is missed. Truth runs showed about 7 such seconds.
