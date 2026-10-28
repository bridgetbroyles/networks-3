# A3 submission guide

This is the short, repository-specific checklist. `SUBMITTING.md` is the
official source if anything here ever differs.

## Files already prepared

| Purpose | File to use | Canvas upload name |
|---|---|---|
| Practice claim validation | `submission/a3_practice_switch.wasm` | any local name |
| Practice truth validation | `submission/a3_practice_truth_switch.wasm` | any local name |
| Truth run | `submission/truth/switch.wasm` | `switch.wasm` |
| Claim observer template | `submission/claim/switch.wasm` | `switch.wasm` |
| Practice egg 21 | `submission/detect_broken_link.wasm` | `detect.wasm` |
| Practice egg 22 | `submission/detect_long_pause.wasm` | `detect.wasm` |
| Practice egg 23 | `submission/detect_scanner.wasm` | `detect.wasm` |

The three supplied detectors prove the instrumentation on practice eggs;
they are not expected to win hidden eggs. Do not upload a practice
detector as a graded claim without a specific truth-log hypothesis.

The graded observer builds use switch 7 as logger because it is the
center of the published 3x5 hidden grid. The practice build uses switch
0 because the six-switch practice network has no switch 7.

## Before every truth run

1. Run the published-grid validation against the exact upload:

   ```sh
   scripts/a3-grid-validation.py --switch submission/truth/switch.wasm --load-scale 1.5 --focused
   ```

2. Test `a3_practice_truth_switch.wasm` on all three practice truth worlds:
   `run-a` at load 1.0, `run-b` at 1.25, and `run-c` at 1.5.
3. Confirm `submission/truth/switch.wasm` loads as a Wasm switch program.
4. Upload only that file to **A3 — Truth run**, with the upload name
   exactly `switch.wasm`.
5. Wait for the Canvas comment before uploading anything else.
6. Save both returned files without editing them. Give them run-specific
   local names such as `truth-1.team.log` and `truth-1-customers.txt`.
7. Decode the log with:

   ```sh
   scripts/decode-team-log.py truth-1.team.log
   ```

There are only three truth runs. A successful returned log consumes one.
An install failure does not. The first two runtime-error truth jobs are
free, but that protection is not a reason to submit an untested build.

The current `submission/truth/switch.wasm` is the verified focused Truth Run
3 candidate. Its SHA-256 is
`e76e922fcdaebb42e9279b99d98851ce094432dfeb828ce26645c8d7c4e2cbf1`.
See `A3_TRUTH_RUN_2_ANALYSIS.md` for the evidence that motivated it. The exact
Truth Run 2 artifact is preserved in `submission/truth-run2/`.

## Before every claim

1. Start from evidence in a returned truth log and state one precise egg
   hypothesis, including likely false and near-miss cases.
2. Narrow the observer if the egg needs different evidence. Keep the log
   below the 24,576-byte claim cap.
3. Write one narrow detector for that hypothesis and test true, false,
   boundary, and near-miss cases locally.
4. Put the final pair in a fresh directory with exactly these names:

   ```text
   switch.wasm
   detect.wasm
   ```

5. Upload both together to **A3 — Claim** and wait for its comment.
6. Record the pair's hashes, hypothesis, result, and claim number. Never
   resend the exact same pair: the server ignores it rather than spending
   another claim.

There are 30 claims. Each claim can win at most one egg, and an egg is
won only if the detector answers all ten fixed runs correctly. Claims
can take minutes; a job that exceeds one hour loses and still consumes
the claim.

## Important caps and filenames

- Truth log: 65,536 bytes.
- Claim log: 24,576 bytes.
- Local practice claim log: 16,384 bytes.
- Truth upload: one file named `switch.wasm`.
- Claim upload: two files named `switch.wasm` and `detect.wasm`.
- One teammate uploads for the team.
- Send one upload at a time; a newer pending upload replaces the older.
- Deadline: end of Tuesday, November 3, Central time.

## Rebuilding

The environment may need the Rust toolchain library directory in
`DYLD_LIBRARY_PATH`, as recorded in `A3_IMPLEMENTATION_LOG.md`.

```sh
# Practice claim build: compact records, logger 0
(cd a3_switch && cargo build --release --target wasm32-unknown-unknown)

# Practice truth build: broad records, logger 0
(cd a3_switch && cargo build --release --target wasm32-unknown-unknown --features truth)

# Hidden claim build: compact records, logger 7
(cd a3_switch && cargo build --release --target wasm32-unknown-unknown --features hidden)

# Hidden truth build: broad records, logger 7
(cd a3_switch && cargo build --release --target wasm32-unknown-unknown --features "hidden truth")

# Final focused truth build: logger 7, counters/links prioritized
(cd a3_switch && cargo build --release --target wasm32-unknown-unknown --features "hidden truth focused")
```

Copy the hidden truth build to `submission/truth/switch.wasm` and the
hidden compact build to `submission/claim/switch.wasm`. Do not overwrite
one with the other when switching features.

## Design card

The design card is `design-card.txt`. Convert it to PDF only if the
separate Canvas assignment specifically requires `design-card.pdf`.
