# Prepared A3 artifacts

## Truth run

Upload this single file to **A3 — Truth run**:

```text
truth/switch.wasm
```

It already has the required upload filename and uses hidden-grid center
switch 7 as logger. SHA-256:

```text
9cb5ea50346bae90feb8db6eeab88b26ab268ccd1d812d02017970097fd019b0
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
