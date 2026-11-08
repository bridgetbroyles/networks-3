# Claim 29: the flapping link on any link (Egg 18) — WON

Server result, Nov 8 2:11am: won Egg 18 · Failure · The flapping link.
Score 136 → 144 of 200.

Upload these two files together:

```text
switch.wasm  105,533 bytes  SHA-256 fba5203e585bae6e1ac39ed8fe0b3407420000bdf6e90146a9bf4d0fd6edc789
detect.wasm   45,866 bytes  SHA-256 a0d64cefc7f45ba595dc042186cfd1eeb1e7c607644757f0c9416d6f7cfd87e4
```

Sources: `a3_anyflap_switch/` and `a3_detect_anyflap/`.

## Why this shape

No truth log shows which link flaps (coverage of many links was partial),
so — as with Claim 28, which won the microburst by watching every sender —
this watches every link. It differs from both earlier Egg 18 attempts:
Claim 25 watched one direction's keepalives on 1-2, and
`submission/next/egg18-attempt` watches only link 1-2, which truth run C
contradicts (2→1 stayed perfect while 1→2 was silent).

## Rule

Every switch punts every keepalive and reports each recovered silence of at
least 0.35 s (who, which neighbour, start and end in ms), flooded to S7 with
duplicate suppression.

* **Link outage:** both ends of one link silent at once for at least 0.2 s
  (outages closer than 0.3 s merge).
* **Shared event:** an outage that coincides with a *clean* outage of
  another link (both ends of that link went silent and came back within
  0.5 s of each other) is an SRLG or a rebooting switch and does not count.
  An unclean overlap (a long one-way silence crossing a brief gap the other
  way) never excuses another link's outage.
* **True** if one link has at least 2 outages of its own.

## Validation

Patched simulator copy, everything switched on in every scenario: the won
eggs (Fortnite lag switch, Steam surges, Home-users flood, 1-2 delay, leaky
11-12, SRLG 0-1/10-11, breathing 12-13, camera microbursts), an emulated S5
reboot, and run C's one-way 1→2 keepalive silences.

**Exhaustive sweep: all 22 grid links × 3 patterns × loads 1.0 and 1.5 —
132 of 132 right.**

| Pattern on each link | Runs | Right | Outages counted on the planted link |
|---|---|---|---|
| down 2× for 1 s (8 s, 18 s) — true | 44 | 44 | exactly 2 every time |
| down 4× for 0.5 s (5, 11, 17, 21 s) — true | 44 | 44 | exactly 4 every time |
| down once for 3 s (10 s) — false | 44 | 44 | 1; no phantom outage on any other link |

This includes the hard cases: the SRLG links 0-1 / 10-11 and the reboot
links 0-5 / 5-6 / 5-10, which already have one outage each before the
planted one; 1-2 (one-way keepalive silences) and 12-13 (one-way congestion
silences), where a real flap merges into a long one-way silence; and 2-7,
whose flap made a fake overlap on 1-2.

**Original bench, loads 1.0 / 1.25 / 1.5 — 21 of 21 right:**

| Scenario | Expected |
|---|---|
| 3-8 down 3× for 2 s | true |
| 9-14 down 2× for 1.5 s | true |
| 6-11 down 4× for 0.8 s | true |
| everything else only | false |
| 3-8 down once and back | false |
| 3-8 down for good (never recovered → never reported) | false |
| 3→8 keepalives only, 3× (one-way) | false |

Unit tests: 6 of 6 pass.

The switch completes all practice worlds without failure; the detector says
no on all 30 practice runs (eggs 21–23, none of them a flapping link —
including Egg 21's one-time recovered outage, which correctly stays no).

## Remaining risk

If the egg's near miss is "a different link flaps", a watch-every-link rule
says true on it and misses. No rule that watches every link can guard
against that; only knowing the link could.
