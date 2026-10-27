# Prepared A3 artifacts

## Truth run

Upload this single file to **A3 — Truth run**:

```text
truth/switch.wasm
```

It already has the required upload filename and uses hidden-grid center
switch 7 as logger. SHA-256:

```text
9950f67c431e2d482ea4f90890cf95ec9b8f52e84767997fceb5258551444154
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
bdbb4724fa9a43e34c55603645a3741fde415220dc84b211180553c056d10f98
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
