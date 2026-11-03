# A3 Claim 2 submission

## Upload exactly these two files

Upload both files together to **A3 - Claim**:

```text
submission/completed/claim-02-asymmetric-1-2/switch.wasm
submission/completed/claim-02-asymmetric-1-2/detect.wasm
```

Their upload names are already exactly `switch.wasm` and `detect.wasm`. Do not
upload the directory, source files, Claim 1 files, or a ZIP archive.

## Hypothesis

Physical link 1-2 has a stable directional timing signature:

- packets from S2 received at S1 take about 30,051 microseconds;
- packets from S1 received at S2 take about 2,051 microseconds;
- ordinary links take about 551 microseconds.

The detector requires at least two nonempty periods in each direction. The
median S2-to-S1 direction must be at least 20 ms, while the reverse median must
be between 1 and 5 ms. This distinguishes the observed signature from a normal
link, a one-way-only delay, symmetric delay, reversed asymmetry, another slow
link, and transient peaks.

## Exact artifacts

```text
switch.wasm
  bytes: 117462
  SHA-256: c79ee8b696487057419ba329de019fdf2e30aeda4f8f09291b4791e77a6e0645

detect.wasm
  bytes: 25430
  SHA-256: e3b65f2385454372a3f8e0cc149246fd683352221de02636eed8b5f70ac10d93
```

Every local claim log was 1,040 bytes, compared with the 24,576-byte cap.

## Verification

- Detector unit tests: 3/3 passed.
- Observer unit tests: 4/4 passed.
- The detector returns true on the actual Truth Run 3 log.
- Positive cases passed at 1.0x, 1.25x, and 1.5x load.
- Positive cases also passed with twenty simultaneous unrelated plants.
- Negative cases passed for a normal link, one delayed direction with a normal
  reverse direction, symmetric delay, reversed asymmetry, another affected
  link, a sub-threshold delay, isolated peak spikes, missing evidence, and
  empty summary periods.

## Submission procedure

1. Wait until Canvas has finished Claim 1. It already has.
2. Attach the two files above together to **A3 - Claim**.
3. Confirm Canvas shows exactly `switch.wasm` and `detect.wasm`.
4. Submit once and wait for the new comment before Claim 3.
5. Save the returned card and report it before submitting another pair.
