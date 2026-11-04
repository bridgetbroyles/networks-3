# Failed Claim 10: capacity change on S6-S7

## Result

Claim 10 won no new egg. The confirmed score remained 56/200. Do not
resubmit this pair or retry the S6-S7 capacity hypothesis with a different
threshold.

Upload exactly `switch.wasm` and `detect.wasm` from this directory together to
**A3 - Claim**. Do not ZIP them, rename them, or mix in a file from another
folder.

## Hypothesis

Physical link S6-S7 changes from its normal capacity to a sustained capacity
of at most 6 Mb/s and later returns to normal. Truth Runs 2 and 3 repeatedly
showed a distinctive 15,051-microsecond queueing peak on this edge, while the
clean reverse observations returned to the ordinary 551-microsecond baseline.
S6 port 2 also recorded 728 drops in Truth Run 3. The earlier S12-S13 guess
failed; S6-S7 is the separate edge with the repeatable capacity-shaped signal.

## Why this pair measures capacity directly

Each endpoint sends adjacent 64-byte and 800-byte on-wire probes over S6-S7.
The receiver records the arrival separation only when no other packet arrived
between the pair. That separation is the second packet's serialization time,
so propagation delay and queueing before the pair cancel out.

The detector requires at least three measurements at 8 Mb/s or faster and at
least three measurements at 6 Mb/s or slower. Requiring both regimes rejects a
permanently slow link and requiring three samples rejects a brief dip.

## Verification

- 432/432 whole-network cases passed across 12 seeds, three load levels, four
  true reduced capacities, and eight negative or near-miss cases.
- Negative cases included no change, 8 Mb/s, constant 4 Mb/s, a brief 4 Mb/s
  dip, a capacity change on S12-S13 only, delay only, wire loss only, and
  congestion only.
- In a representative 1.5x run, every true transition produced 42 normal and
  34 reduced measurements. Normal, delay, loss, and congestion cases produced
  76 normal and zero reduced measurements.
- 10/10 encoded-record, missing-data, malformed, and truncated-log checks
  passed.
- The exact pair loaded and completed all nine official practice-truth/load
  combinations.
- The repository's 158 Rust unit and integration tests passed.
- Largest adversarial claim log: 4,528/24,576 bytes.
- The packaged files match the audited files byte-for-byte.

The local tests correctly proved that the pair could measure a capacity
transition, but the server result disproved the inference that an unwon egg
caused such a transition on S6-S7. The truth-run queueing peak and drops were a
correlated symptom, not sufficient evidence of the proposed plant.

## Exact files

```text
switch.wasm
  bytes: 142670
  SHA-256: 1cd3ffe08eb46cfe4dc7bfa2ed9fa0e6bb8a06a9b993e324dfff821d38b82bf1

detect.wasm
  bytes: 46351
  SHA-256: eaf3b0c0983b832dba6074230fc851da6813f61a4c8f09bc12eee752066b2d8b
```

Wait for the server reply before submitting another claim.
