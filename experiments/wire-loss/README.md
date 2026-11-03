# Wire-loss candidate: source and validation

The only upload pair is [submission/next/claim-07-wire-loss](../../submission/next/claim-07-wire-loss/README.md). Its tested hashes are recorded in manifest.json and the result files here. Hidden grading remains unverified.

- source/observer and source/detector contain the candidate implementations and use the shared SDK at the repository root.
- audit-results.json: 64 isolated network cases.
- boundary-results.json: 48 loss-boundary and transient cases.
- combined-results.json: 25 simultaneous-fault/header cases.
- record-results.json: 11 detector record checks.
- old-*-findings.json: evidence for rejecting earlier revisions.

From the repository root, build the simulator with `cargo build --release`, then run `python3 experiments/wire-loss/records.py`, `python3 experiments/wire-loss/audit.py`, and `python3 experiments/wire-loss/combined.py`. The scripts regenerate ignored synthetic worlds and logs. To reproduce the additional boundary sweep, run:

```sh
LOSS_SEEDS=3401,3402,3403,3404,3405,3406,3407,3408,3409,3410,3411,3412 LOSS_LOADS=1.0 LOSS_CASES=near-two-percent,boundary-2.5-percent,ten-seconds-five-percent,ten-seconds LOSS_RESULT=boundary-results.json python3 experiments/wire-loss/audit.py
```

`python3 experiments/wire-loss/build.py` rebuilds programs into ignored generated/ without changing the upload pair, then reports whether hashes match. The Rust wasm target must already be installed. Synthetic validation does not prove the hidden egg definition or guarantee points.

After relocating to the shared SDK, the detector rebuild is byte-identical; the observer rebuild has a different hash. The original tested upload files remain unchanged. Generated builds stay separate and must be validated before replacing an upload pair.
