# A3 Truth Run 2 analysis

## Bottom line

Truth Run 2 returned a valid 53,574-byte log with 2,874 complete records, but
it did **not** provide complete network-wide evidence. The log stayed below the
65,536-byte cap; the limiting resource was instead the logging switch's
80-event/s controller budget.

The exact submitted observer is preserved at
`submission/truth-run2/switch.wasm` with SHA-256:

```text
9cb5ea50346bae90feb8db6eeab88b26ab268ccd1d812d02017970097fd019b0
```

The returned `customers.txt` is byte-for-byte identical to Truth Run 1. This
means the customer map stayed fixed across the first two truth attempts.

## What the log contains

| Record | Count |
|---|---:|
| Header | 1 |
| Source statistics | 116 |
| Destination statistics | 0 received |
| Ordinary packet samples | 2,614 |
| Small-packet samples | 17 |
| Link summaries | 91 |
| Link silent / heard | 18 / 15 |
| Port-drop summaries | 2 |

Only 91 of roughly 836 expected two-second directed-link summaries reached
the logger. S7 contributed all 19 periods for its four links, but S8
contributed only two periods, S9 one, S14 two, and the remaining remote
switches contributed none. The sparse source statistics and absent
destination statistics have the same cause.

## Why Run 2 became incomplete

The dominant sampled flow was Cloudflare to Cloudflare users: 2,337 samples
from 1.0 through 38.8 seconds, all 1,000 bytes. Sampling one packet in 1,024
therefore produced about 62 controller events per second at S7. S7 also had to
process roughly 40 local keepalives per second plus incoming reports. Those
loads exceed the 80-event/s limit even though the final byte log is not full.

This explains the repeated S7 SILENT/HEARD events and missing remote reports.
They are observer-overload artifacts, not dependable physical-link evidence.
The isolated `S0->S5 SILENT` event at 33.1 seconds might be real, but one event
without the opposite direction or recovery is not enough for a claim.

## Evidence that is still useful

- S14 receiving from S13 measured 25,051 microseconds in both returned truth
  runs. Normal directions are about 551 microseconds. This is a strong
  one-way-delay candidate.
- Truth Run 1 also measured S1 receiving from S2 at 30,051 microseconds. Run 2
  did not receive S1's summaries, so the candidate remains unconfirmed.
- S7 receiving from S6 peaked at 15,051 microseconds, but the same port was
  heavily affected by controller starvation. It needs clean evidence.
- S12 port 2 accumulated 51 drops. A drop count alone does not distinguish a
  capacity plant from ordinary congestion.
- Stable named flows include Cloudflare to Cloudflare users (1,000-byte),
  Fastly to Home users (200-byte), Login to Login (100-byte), and Akamai to
  Akamai users (200-byte).
- Steam was steady at about 0.63 Mb/s in the received 1.5-second windows.
  The two Amazon shopper sources showed long inactive gaps of roughly 7.5 and
  6 seconds, consistent with normal on/off behavior rather than automatically
  proving a pause egg.

## Final truth-run correction

The final `focused` observer keeps source and destination byte and packet
counters, two-second link summaries, drop records, and packet examples. Source
deltas are emitted every 1.5 seconds and destination deltas every four seconds.
Packet fields are now 32-bit so the high-rate customer cannot saturate a
reporting window.
It now logs at S2 rather than S7. S2 remains near the grid center but avoids the
measured high-rate Cloudflare attachment that exhausted S7 in Run 2.

Open-loop planted packets expose sequence zero, so a sequence-only first-two
test would punt every packet. The final build hashes source, destination, size,
and protocol into a data-plane register. It keeps the first two packets for a
new flow signature, then samples medium packets at 1/1,024 and large packets at
only 1/65,536. Small source/destination pairs are retained at most once per five
seconds. This identifies new planted flows without recreating the event-budget
failure from the known 1,000-byte hotspot.

Routine reports use one decreasing-distance path toward S2. A silent-link
record is repeated every timer tick while the link remains silent. Both SILENT
and HEARD transition chunks use duplicate-suppressed flooding. That special
treatment is necessary because a failed link can destroy the normal reporting
path, and the recovering adjacency may not immediately restore that path. It
costs nothing in a healthy run and retained every transition during overlapping
failure tests.

Final eleven-scenario, 1.5x-load full-grid validation retained:

- records from all 15 switches and all 44 directed link views;
- all 836 expected summaries in every healthy stress scenario;
- all six failed directions and all six recoveries across three overlapping
  bidirectional failures;
- all 50 targets from a 50-address scan under a simultaneous high-rate source;
- wire-loss gaps, a 30,051-microsecond delay peak, capacity/drop evidence, and
  ordinary side-flow packet evidence in the combined-plant scenario;
- no false SILENT/HEARD transitions in healthy scenarios;
- a largest observed log of 51,398 bytes (78.4% of the 65,536-byte cap).

The added adversarial cases cover an extreme source directly on logger S2,
three simultaneous lossy links, three permanent failures, a scan that sends
four packets per target, and a dedicated capacity bottleneck. The capacity case
recorded 10,818 drops plus two exact 1,000-byte side-flow samples. The
permanent-failure case produced all six SILENT directions and no false HEARD
record; the recovery case produced all six of each. The scan-multiplicity case
retained all 50 distinct targets.

The exact candidate for Truth Run 3 is `submission/truth/switch.wasm`,
SHA-256:

```text
deed7bef8ddb8b772620a11fbfea6c979510135acd415cc8edb789f2d60d506d
```

## What to do next

Use the final truth attempt on the focused observer. The highest-value missing
evidence is a complete time series for every link and every customer counter,
not more Cloudflare packet examples. After the returned files arrive, compare
the three logs, write explicit hypotheses, and create narrow claim-specific
observers and detectors. Do not claim the repeated S7 silence artifacts.
