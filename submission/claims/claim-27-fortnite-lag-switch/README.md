# Claim 27 candidate: Fortnite cheater, a lag switch (Egg 13)

Upload these two files together:

```text
switch.wasm  112,539 bytes  SHA-256 4745cd7e61028e583bb868eb97886a0fe9f195ac00db2afb9982df1cea4100d8
detect.wasm   49,257 bytes  SHA-256 d05c75d93d135417a96694bcd1c65625d350cd6761f7e88cd63007ca7359604a
```

Sources: `a3_lag_switch/` and `a3_detect_lag/`.

## Evidence (truth run C, logger S2 — the player's switch, complete counts)

Fortnite's /24 sent a metronome-steady 494 packets every 1.5 s (±1). Its
nested /32, the Fortnite player, sent 84–85 per window except:

| Windows | Player packets | Sum |
|---|---|---|
| 11.0 → 12.5 s | 26 → 143 | 169 |
| 15.5 → 17.0 s | 53 → 117 | 170 |
| 26.1 → 27.6 s | 23 → 147 | 170 |
| 30.6 → 32.1 s | 50 → 120 | 170 |

Nothing is lost: packets are held back for ~0.6–1.1 s and released in a
burst, again and again — a lag switch. Counts are at the player's ingress
port, so it is the player's own sending. Another team's card confirms Egg 13
"Traffic · Fortnite cheater".

## Rule

- Identity: the /32 nested in a shorter range where both send ~200-byte
  packets (Fortnite). Prime member and Netflix live have 1000-byte parents.
- Lag event: ≥ 0.5 s (5 × 100 ms ticks) with no packet from a player whose
  median tick is ≥ 2, followed within 2 s by packets ≥ the normal amount plus
  40% of what was held.
- True if ≥ 2 lag events.

## Switch

Only S2 does anything: data-plane packet and byte counters per source range,
read every 100 ms; once a second it writes ten 100 ms counts for each nested
range that sent anything, plus all ranges once at start. No punts, no
reports. Log ≈ 3 KB.

## Validation

Patched simulator copy allowing nested /32s (as for Claim 24). Lag switch
emulated as a pause plus a burst of the held packets; it reproduces run C's
windows (35 → 137, 31 → 141, 33 → 139, 30 → 142). Loads 1.0 / 1.25 / 1.5:
**33 of 33 right**.

| Scenario | Expected |
|---|---|
| 4 lags like run C; 2 lags of 0.6 s; 4 lags with other eggs' faults and the flood | true |
| off; off + other eggs | false |
| dropouts without release; bursts without holds; a single lag | false |
| the Fortnite /24 lags; Prime member lags; Jane Street lags | false |

Also right with the player slowed to ~37 packets/s (the real load-1.0 rate if
run C was load 1.5). On the unmodified simulator the switch completes all
practice truth worlds without failure; the detector said no on all 30
practice runs.

## Remaining risks

- Evidence is from run C only (S2 was not the logger in runs A and B).
- If the egg's near miss is "the player lags only once or twice", the
  2-event minimum could misjudge it; run C showed at least four.
