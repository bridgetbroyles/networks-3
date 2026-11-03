# Prepared A3 artifacts

## Final graded Claim 1 pair

Upload these two files together to **A3 - Claim**:

```text
claims/claim-01-slow-13-14/switch.wasm
claims/claim-01-slow-13-14/detect.wasm
```

This pair tests sustained bidirectional high delay on hidden-grid link 13-14.
See `docs/A3_CLAIM_01_SUBMISSION.md` for hashes and validation.

## Truth run

Upload this single file to **A3 — Truth run**:

```text
truth/switch.wasm
```

It already has the required upload filename and uses switch 2 as logger. This
keeps the logger near the center while avoiding the high-rate customer observed
at switch 7 in the first two truth runs. SHA-256:

```text
cdd754ec8f588d9cf723c84e3313744f35108bb20c47dc962bb0661778574a67
```

## Claim

`claim/switch.wasm` is the compact hidden-grid observer template. A real
claim also requires a detector written for one hypothesis from a truth
log. Put those two files together with exactly these names:

```text
switch.wasm
detect.wasm
```

Current compact observer SHA-256:

```text
2c89e1ec9127ee94163bbbe98bb74fed3a14fa925115147e8ac955583545de17
```

Do not submit one of the practice detectors merely because it is already
compiled. They validate the observer locally but are not hidden-egg
claims.

## Local validation only

- `a3_practice_switch.wasm`: compact logger-0 build for practice claims.
- `a3_practice_truth_switch.wasm`: broad logger-0 build for the three
  practice truth worlds.
- `detect_broken_link.wasm`, `detect_long_pause.wasm`, and
  `detect_scanner.wasm`: disclosed practice-egg detectors.

The duplicate top-level `a3_claim_switch.wasm` and
`a3_truth_switch.wasm` are retained for compatibility with earlier docs.
Use the exact-name files in `claim/` and `truth/` for Canvas preparation.

`truth-run1/switch.wasm` preserves the exact first truth-run observer.
It is evidence for reproducibility, not the observer recommended for the
second truth attempt.

`truth-run2/switch.wasm` preserves the exact second truth-run observer. Its
1/1024 sampling overloaded S7's event budget. The current `truth/switch.wasm`
uses the final focused build: small packets are sampled every eighth sequence,
medium packets below 800 bytes retain their first two identifying examples, and
packets of 800 bytes or more are counted but not punted. Link-silence evidence
is repeated, and both SILENT and HEARD transition chunks use
duplicate-suppressed flooding so failures do not erase their own reports.
