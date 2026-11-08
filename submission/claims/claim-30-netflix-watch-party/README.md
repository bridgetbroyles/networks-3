# Claim 30: Netflix live pauses — the watch party (Egg 5) — MISSED

Server result, Nov 8 2:49am: no new egg won. Final score 144 of 200.
So "Netflix live pauses at least twice" is not how any unwon egg's ten
runs split; the truth-run silences were either not the egg, or the egg's
runs differ in a way this rule does not follow (one pause per run, say).

Upload these two files together:

```text
switch.wasm  112,550 bytes  SHA-256 9a5b087fc7fd28fb70b5f3364f9f8228d513724a1b88c740773b9bd17ee5d572
detect.wasm   36,998 bytes  SHA-256 2d2e6be2ab745ea56af1c6a05aa0e615b0463843eeec5a81d064b93ae2a0fee7
```

Sources: `a3_party_switch/` and `a3_detect_party/`.

## Why Egg 5

- **It is worth 16 and it can be won.** The other team's card held exactly
  our eggs plus Egg 5 and scored 160 to our 144.
- **Its vehicle is in the customer list.** The hidden network has three /32s
  nested inside /24s, and each looks like an egg's vehicle. "Fortnite
  player" was the Fortnite cheater (Egg 13, won). "Netflix live"
  (10.17.72.200/32, inside Netflix's /24, with its own "Netflix live's
  viewers" /24) is the obvious one for a Netflix watch party.

## Evidence (truth runs; every egg on)

Netflix live's packets per 1.5 s window at S3, run C (load 1.5), next to
Netflix's own /24 at the same switch:

| Window ends | 9.5 | 11.0 | 12.5 | 14.0 | 21.5 | 32.1 | 35.1 | 38.1 |
|---|---|---|---|---|---|---|---|---|
| Netflix live | 528 | **163** | **193** | 509 | 528 | **45** | **129** | 475 |
| Netflix /24 | 748 | 748 | 749 | 748 | 748 | 748 | 748 | 749 |

- 528 is its full rate, which matches run A (118 per 0.5 s at load 1.0).
  It never goes above that, so there is no catch-up burst like the
  Fortnite lag switch has.
- The 11.0 and 12.5 windows fit one silence of **1.98 s**, from about 9.97 s
  to 11.95 s. The 32.1 and 35.1 windows hold at least 1.37 s and 1.13 s more
  of silence. Run B's 6.5 s window lost about 0.5 s.
- Netflix's /24 never misses a beat, so S3 itself is fine.
- No link outage lines up with these silences in any truth log. The only
  keepalive losses at those times are at S2, from several neighbours at
  once, which is an artifact of our truth build at S2.
- The simulator's TCP engine has no seconds-long timeouts: its RTO is 2×
  the smoothed RTT, with a 1 ms minimum. So silences this long are planted
  pauses, not congestion.

## Rule

- **Switch.** Only S3, the logger, does anything. Its data plane counts
  packets and bytes by source range. Every 100 ms it checks the nested
  ranges (each /24 holding a /32, and each such /32). When one sends again
  after more than 1 s of silence, and had already sent at least 10 packets
  (so a hello packet followed by a late start doesn't count), it writes
  that pause. Every 5 s it writes running totals, and at start it writes
  the range list. The log is about 0.7 KB.
- **Detector.** Netflix live is the /32 inside a shorter range whose packets
  average at least 900 B (Amazon's and Fortnite's /32s send 200 B). True if
  it has **at least 2 recovered pauses of at least 1 s**. A pause doesn't
  count if Netflix's /24 was silent at the same time.

## Validation

The bench ran on a replica of the hidden network (patched simulator copy)
with every won egg switched on in every run:
- Fortnite lag switch, Netflix side channel, Steam surges, Home-users flood
- 1-2 one-way delay, 13-14 delay, leaky 11-12, breathing 12-13
- SRLG 0-1 / 10-11, S5 reboot, run C's 1→2 keepalive silences

Loads 1.0 / 1.25 / 1.5: **48 of 48 right.**

| Scenario | Expected |
|---|---|
| truth-like: pauses of 2 s and 4 s | true |
| two pauses of 1.2 s; three pauses of 2–3 s | true |
| truth-like + flaps on Netflix live's path (3-8, 8-13) | true |
| truth-like with Netflix live as TCP; after a late start | true |
| everything else only; + flaps on its path; Netflix live as TCP | false |
| one 4 s pause; one 10 s pause; four 0.5 s hiccups | false |
| Netflix's /24 pauses twice; Prime member pauses twice | false |
| Netflix live stops for good; late start + one pause | false |

Extra check, not a promise: Netflix live as TCP with flaps on its path, and
no pause planted, still says false. Rerouting keeps it from going quiet for
a second.

Unit tests: 3 of 3 (switch), 2 of 2 (detector). The switch completes all
practice worlds; the detector says no on all 30 practice runs.

## Remaining risk

- **The egg may not be the pauses.** This rests on reading the truth-run
  silences as the watch party. If the egg is something else on Netflix live
  (more viewers, extra copies of the stream), this misses.
- **One pause per run would miss.** The truth runs show at least two
  pauses, so this needs two. If each true run has only one, this says no on
  them.
- **The silences may come from another egg.** If another egg that stays on
  in claim runs causes them, the off runs would show them too.
- If the pauses belong to a different Customer egg on Netflix live, a match
  would win that egg instead (4–16 points).
