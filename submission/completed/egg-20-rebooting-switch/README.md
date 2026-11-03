# Completed Claim 9: Egg 20, The rebooting switch

## Result

Claim 9 won **Egg 20 - Failure - The rebooting switch**, worth 16 points. The
confirmed total became 56/200. These files are archived for reproducibility;
do not resubmit them.

The exact successful pair is `switch.wasm` and `detect.wasm` in this folder.

## Why this is the next claim

Truth Run 3 recorded S0, S6, and S10 all losing S5 and then all recovering at
about 36 seconds. Because the observer declared silence only after roughly two
seconds without a keepalive, the physical interruption began earlier than the
first visible SILENT record. The timing is consistent with an approximately
five-second simultaneous interruption of all three physical links.

This is distinct from Eggs 10, 14, 16, and 19 and from failed Claim 8. It maps
directly to the simulator's planted physical-link failure mechanism. It does
not rely on an ordinary customer behavior, congestion correlation, or a
signature caused by an already-won egg.

## Exact rule

The observer actively probes all three S5 links from their outside endpoints
and records unsampled physical-port arrivals. The detector requires:

- S0-S5, S6-S5, and S10-S5 to lose probes concurrently for at least 3.5 s;
- explicit zero physical-port arrivals during those intervals;
- matching link-silence evidence; and
- later probe recovery on all three links.

It rejects a failure of one or two links, a short interruption, a permanent
failure, staggered non-overlapping interruptions, isolation of a different
hub, missing telemetry, live traffic despite probe starvation, and mere
keepalive suppression.

## Verification

- 396/396 whole-network cases passed across 12 seeds and loads 1.0, 1.25,
  and 1.5, with the other known link anomalies active.
- 9/9 malformed, incomplete, and near-miss record checks passed.
- 9/9 official practice-truth world/load combinations installed and ran.
- Largest claim log: 11,723/24,576 bytes.
- The packaged files match the tested files byte-for-byte.

This is a high-confidence claim and is the strongest unsubmitted candidate.
The hidden ten-run suite is unavailable locally, so no honest process can
mathematically guarantee its result; the evidence is sufficient to recommend
submission.

## Exact files

```text
switch.wasm
  bytes: 142167
  SHA-256: 58893d22da3b31602e38ddcd5b09617a974c95dd3331fc2c20e8042cc800ad3e

detect.wasm
  bytes: 63651
  SHA-256: ae4be04257adcb55e963bded4dee3dd2f9d9f952fc41486d404260d17cdbba92
```

The server result confirmed that the simultaneous interruption and recovery
of all three physical links around S5 was the rebooting-switch mechanism.
