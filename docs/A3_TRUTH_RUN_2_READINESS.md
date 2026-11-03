# Truth Run 2 readiness

> Historical pre-submission record: Truth Run 2 has now returned. Its packet
> sampling overloaded S7's event budget and suppressed most remote summaries.
> The exact artifact is preserved in `submission/evidence/truth-run2/`. See
> `A3_TRUTH_RUN_2_ANALYSIS.md` for the measured result and the corrected Truth
> Run 3 observer. Do not use the upload recommendation or SHA below now.

## Decision

`submission/truth/switch.wasm` is ready for the second truth-run upload.
It is substantially safer and more informative than the program used for
Truth Run 1.

This is a readiness conclusion, not a guarantee of a full course score. The
hidden egg definitions and their near-miss boundaries are intentionally not
available locally. The purpose of Truth Run 2 is to obtain that missing
evidence. What local testing can establish—and now does establish—is that the
observer loads, covers the whole published topology, survives representative
failures and traffic stress, and stays below the log cap.

## Exact file

Upload exactly:

```text
submission/truth/switch.wasm
```

Canvas requires the uploaded filename to be `switch.wasm`. SHA-256:

```text
9cb5ea50346bae90feb8db6eeab88b26ab268ccd1d812d02017970097fd019b0
```

Do not upload `submission/evidence/truth-run1/switch.wasm`; that is the preserved old
observer. Do not upload `submission/templates/observer/switch.wasm` to a truth run.

## What the validation reproduced

`scripts/a3-grid-validation.py` creates the published 15-switch 3x5 grid,
uses central logger S7, installs 54 sending customer ranges, and runs for 40
seconds under the official observation and 65,536-byte log limits. It checks
for evidence from all 15 switches and all 44 directed link views.

The exact upload was tested at load 1.5:

| Scenario | Log bytes | Cap used | Result |
|---|---:|---:|---|
| Baseline | 46,160 | 70.4% | All switches and links; no false failures |
| Three overlapping failures | 40,292 | 61.5% | 6 SILENT and 6 HEARD records; reporting rerouted |
| Sustained 64-byte traffic | 47,214 | 72.0% | All switches and links; no false failures |
| High-rate 200-byte load | 49,238 | 75.1% | All switches and links; no false failures |
| Mixed plants | 49,525 | 75.6% | Delay, loss, capacity, pause, scan and side-flow evidence retained |

The worst case left 16,011 bytes of headroom. The mixed case deliberately
stacked all locally available plant families because a real truth run has
every egg enabled.

The practice truth worlds produced 9,945, 10,097, and 10,230 bytes at loads
1.0, 1.25, and 1.5. The compact observer still won all 10 runs for each of
practice eggs 21, 22, and 23. The full workspace test suite passed.

## Defects found and fixed before this recommendation

The first full-grid baseline nearly filled the log: 65,480 of 65,536 bytes.
The first small-packet stress test filled it and caused false silent-link
records. Those results were not accepted.

The final observer now:

- records source counters every 1.5 seconds and destination counters every
  two seconds;
- samples every eighth small-packet sequence number, while preserving the
  first two ordinary samples and the one-in-1,024 long-flow sample;
- logs a repeated small source/destination pair at most once per five seconds;
- uses a two-second quiet threshold so controller contention is not mistaken
  for a failed link;
- routes one report toward S7 and avoids locally silent next hops;
- keeps segmented headers for all 54 observed customer ranges.

The synthetic 50-address scan still retained eight distinct targets, while a
steady small side flow retained only one target per deduplication period. The
source counters preserve the total rate even when individual packets are not
logged.

## What Truth Run 2 must return

After Canvas finishes, save both returned files together, unchanged, for
example:

```text
truthrun2/team.log
truthrun2/customers.txt
```

Then decode them with:

```sh
scripts/decode-team-log.py truthrun2/team.log --customers truthrun2/customers.txt
```

Before spending Truth Run 3, compare runs 1 and 2. Separate repeatable plant
signatures from ordinary customer behavior, write explicit egg hypotheses,
and identify which questions remain unresolved. Truth Run 3 should be changed
only to target those remaining questions; it should not automatically repeat
the same observer.

## Honest remaining uncertainty

No local test can guarantee that one truth log reveals every hidden egg or
its exact ten-run near misses. In particular, aggregate source and destination
counters do not preserve every packet pair, and packet sampling is necessarily
bounded by the 80-event/s controller limit. Exact detector thresholds still
have to be inferred from returned truth evidence.

The final local result is therefore: **go for Truth Run 2**, with strong
evidence that the run will be usable and no known observer defect that justifies
spending more time before the upload. It is not an honest basis for promising
full credit before the returned files are analyzed.
