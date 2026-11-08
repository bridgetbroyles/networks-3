# A3 Claim 25 candidate: the flapping link 1-2, seen by its keepalives (Egg 18?)

## Upload exactly these two files

```text
submission/claims/claim-25-flapping-link-1-2/switch.wasm
submission/claims/claim-25-flapping-link-1-2/detect.wasm
```

```text
switch.wasm  126,052 bytes  SHA-256 1bca08b5408fa20edc60c7efdf57ee2f1b43d692d51a05766ab8db6c5dff1b20
detect.wasm   38,090 bytes  SHA-256 34dcc7cc261124444d077d4bd010bf9e14cb16df6e2236dd1c3a51f76596d5af
```

Sources: `a3_flap_switch/` and `a3_detect_flap/`.

## How this differs from `submission/next/egg18-attempt`

That attempt requires link 1-2 to go physically down and up at least twice in
**both** directions. Truth run C contradicts that: while S2 heard nothing from
S1, S1 heard every keepalive from S2.

| Window | S2 hears S1 (1→2) | S1 hears S2 (2→1) |
|---|---|---|
| 16–18 s | 0 | 20 of 20, no gaps |
| 22–24 s | 0 | 20 of 20, no gaps |
| 28–30 s | 0 | 20 of 20, no gaps |

A link that is down loses keepalives at both ends, so 1-2 stayed up. The
egg18-attempt detector deliberately rejects this pattern ("keepalive gaps with
a working physical link").

## Evidence for this claim

Run C, S2 hearing S1: one or two keepalives per two-second window, each at
exactly 2,051 µs (not queued), and none from about 16 s to 31.6 s. The truth
observer logged S2 hearing S1 again after at least 2 s of silence six times:
6.0, 7.8, 10.8, 13.3, 31.6, 34.1 s. Already-won eggs do not explain it: the
leaky link is 11-12 (`experiments/wire-loss` observer, `peer()`), and the
one-way slow link only adds delay. The simulator has a mechanism that does
exactly this: `[[harness.keepalive_gap]]`, which stops one switch's
keepalives toward a neighbour while the link keeps working — the classic
picture of a flapping adjacency.

## Rule

- S2's view of S1 goes silent for at least 2 whole seconds and later hears S1
  again, at least 3 times;
- veto if, during those silences, S1 also heard nothing from S2 in at least
  half the seconds it reported (the link really down);
- veto if S2 also heard nothing from all its other neighbours in at least
  half of them (S2 itself failing).

## Switch

S1 and S2 punt every keepalive and count, per neighbour and per second (zeros
included). S2 is the logger; S1 floods its counts with duplicate suppression.
Other switches only learn ports and forward reports. Log ≈ 1.6 KB.

## Validation

Crowded replica (as for Claim 23) with run C's 1→2 keepalive pattern recreated
by `keepalive_gap`. Loads 1.0 / 1.25 / 1.5: **39 of 39 right**.

| Scenario | Expected | S2 hears S1 (per second) | Cycles |
|---|---|---|---|
| run C pattern; with the won eggs' faults (1-2 delay, 11-12 loss, 13-14 delay, SRLG, breathing, flood) | true | `+....++..+..+.+...............+..+...` | 5 |
| regular 3 s flaps | true | `####...###...###...` | 5 |
| off; off + won eggs | false | all heard | 0 |
| one silence that recovers | false | `#########..........###` | 1 |
| silent for good | false | | 0 |
| 1-second blips | false | | 0 |
| the other direction (2→1), another link (3→2), a far link (6→7) | false | all heard | 0 |
| link 1-2 physically flaps 4 times | false | S1 deaf too → veto | 4 |
| S2 deaf to every neighbour | false | → veto | 5 |

The switch completes all practice truth worlds at all loads without failure;
the detector said no on all 30 practice runs.

## Remaining risks

- If Egg 18 is a physical flap somewhere else, this misses (so would the
  egg18-attempt unless it is on 1-2).
- This may instead match an unnamed Link egg (17) that silences one
  direction; a claim wins whichever egg its answers match.
