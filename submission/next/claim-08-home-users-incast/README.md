# Claim 8 candidate: high-volume Home-users UDP incast

Upload both `switch.wasm` and `detect.wasm` from this directory together. Do
not mix either file with another claim.

## Hypothesis and evidence

Truth Run 3 retained 64-byte UDP packets from 22 Home-user addresses converging
on one Fastly destination at S2. During the same interval, the source prefix
rose from about 155 packets per 1.5-second counter window to about 4,674, then
returned to baseline. The high-volume interval lasted roughly 15 seconds.

The claim observer works at source ingress S2 and records only the counters and
packet fields needed for this predicate. The detector joins sampled source
addresses to the shuffled prefix header and requires:

- at least ten distinct source addresses in one source prefix;
- 64-byte UDP to one exact destination;
- samples spanning at least five seconds;
- at least three contiguous high-volume counter windows;
- at least 1,000 packets in each high window;
- at least a tenfold increase over the prefix's lower-quartile baseline;
- mean packet size at most 100 bytes during the high interval.

This rejects the tested near misses of nine sources, a two-second burst, a
lower-volume distributed flow, the wrong site/size/protocol, and unrelated
prefix counters.

## Verification

The exact packaged pair passed a fresh 30-case grid sweep across two seeds and
loads 1.0, 1.25, and 1.5. Positive, off, nine-source, low-volume, and
short-burst cases all produced the expected answer. Encoded positive,
near-miss, empty, and truncated-log checks also passed. Earlier simultaneous-
anomaly and 54-sender header tests passed with these same hashes.

Largest fresh log: 8,204/24,576 bytes. Earlier combined stress maximum:
10,602/24,576 bytes.

This establishes the implemented behavior and resource safety, not the
undisclosed hidden rule. It is the strongest remaining distinct traffic
hypothesis after Eggs 10, 14, 16, and 19 were removed.

## Exact files

```text
switch.wasm
  bytes: 131891
  SHA-256: 462fd4d1a88aec73d897a0e869b336d9adc70006a024f63fc0a911bf3a1aaf89

detect.wasm
  bytes: 75592
  SHA-256: 1d3053af3a50301c8669a0b7f66646e8ebda3e436557a43c258078597e36dcfe
```

Wait for the complete server response before choosing Claim 9.
