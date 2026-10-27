# What to submit for Assignment 3

## Important limitation

The repository's `SUBMITTING.md` still says that the real server instructions, hidden topology, and log caps will be published later. Follow the updated course version when it appears; it overrides this guide.

## Truth run

Submit the broad observer:

```text
submission/a3_truth_switch.wasm
```

Before spending a truth attempt, inspect the published hidden map. If switch 0 is a poor logging location, change `LOGGER` in `a3_switch/src/lib.rs`, rebuild with `--features truth`, and replace the stable copy.

Keep every returned team log and `customers.txt`. Run:

```sh
scripts/decode-team-log.py YOUR_RETURNED_LOG.team
```

## Practice claims

These pairs pass all disclosed practice runs:

| Practice egg | Switch file | Detector file |
|---|---|---|
| 21, broken link | `submission/a3_claim_switch.wasm` | `submission/detect_broken_link.wasm` |
| 22, long pause | `submission/a3_claim_switch.wasm` | `submission/detect_long_pause.wasm` |
| 23, scanner | `submission/a3_claim_switch.wasm` | `submission/detect_scanner.wasm` |

Practice eggs earn no points, so these pairs are for validation and quiz preparation, not useful graded claims.

## Graded hidden claim

A graded claim needs exactly:

1. A compact switch program, normally based on `submission/a3_claim_switch.wasm` but narrowed or adjusted for the hidden egg.
2. One `detect.wasm` designed for that single hidden egg.

Do not submit one of the three practice detectors expecting hidden credit. The server tests a claim against every unwon hidden egg, but a detector wins only an egg whose ten true, false, and near-miss runs it answers perfectly.

## Design card

`design-card.txt` explains the implemented observation design. Convert it to PDF only if the separate Canvas design-card assignment requires `design-card.pdf`; the A3 handout itself describes the switch and detector upload process rather than a design-card upload.
