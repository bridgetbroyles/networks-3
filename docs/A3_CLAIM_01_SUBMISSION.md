# A3 Claim 1 submission

## Upload exactly these two files

Upload both files together to the Canvas assignment **A3 - Claim**:

```text
submission/claims/claim-01-slow-13-14/switch.wasm
submission/claims/claim-01-slow-13-14/detect.wasm
```

Their upload names are already exactly `switch.wasm` and `detect.wasm`. Do not
upload the directory, source files, truth-run switch, or a ZIP archive.

## Hypothesis

Link 13-14 has sustained delay above 20 ms in both directions. The switch logs
only two-second summaries for that physical link. The detector requires two
or more nonempty periods per direction and tests the median average, not a
single maximum.

## Exact artifact checks

```text
switch.wasm
  bytes: 117472
  SHA-256: 7de0c5ece9bcfac7936c5d7c6346261aa322d3bbd6678fd2f86d2ba3a5bc5b58

detect.wasm
  bytes: 25418
  SHA-256: 8b05c3651985fc396b91bbbfd5eb64bcf52661ecb7cb4b3a6986d91aa6caedcb
```

The claim log was 1,040 bytes in every local case, compared with the 24,576-byte
server cap.

## Verification performed

- Detector unit tests: 4/4 passed.
- Observer unit tests: 4/4 passed.
- Exact packaged Wasm files load in the simulator.
- The packaged detector returns true on the actual Truth Run 3 log.
- The exact packaged pair returns true on a bidirectional 25 ms target link.
- It returns false on a normal target link, a one-way target delay, a slow
  different link, a 15 ms bidirectional near miss, a single-spike synthetic
  log, missing-direction evidence, malformed records, and empty periods.
- The true and false cases were tested at 1.0x, 1.25x, and 1.5x load.
- The positive and one-way-negative cases were also tested while twenty other
  simultaneous failure, loss, capacity, pause, scan, and side-flow plants were
  active.

## Submission procedure

1. Open **A3 - Claim** in Canvas.
2. Attach both files from the claim directory in one submission.
3. Confirm Canvas shows exactly `switch.wasm` and `detect.wasm`.
4. Submit once and wait for the server comment before sending another claim.
5. Save the returned card. Do not resend this exact pair; identical claims are
   ignored by the server.

This is the best first claim from the available evidence, but hidden near-miss
definitions cannot be proven locally. The returned card is still needed before
choosing the next claim.
