# A3 final truth-run coverage audit

## Conclusion

The final truth observer is ready for the third truth run. It cannot guarantee
150/200 by itself—the returned evidence still has to be converted into narrow,
correct claim detectors—but it now records evidence for every plant family the
simulator can create. The exact upload is:

```text
submission/truth/switch.wasm
SHA-256: deed7bef8ddb8b772620a11fbfea6c979510135acd415cc8edb789f2d60d506d
```

Do not rebuild or rename a different artifact before uploading. Upload this
file as `switch.wasm` to **A3 — Truth run**.

## What the third run will reveal

| Hidden behavior | Evidence in the returned log | Resolution |
|---|---|---|
| Customer identity and attachment | Segmented prefix header plus source records carrying switch ID and customer ID; combine with returned `customers.txt` | Complete prefix map; 1.5 s source windows |
| Normal rate, packet size, bursts, and pauses | Source bytes and 32-bit packet counts | 1.5 s windows |
| Destination activity | Destination bytes and 32-bit packet counts at each source-side switch | 4 s windows |
| Short scan | Small-packet source and exact destination address | One retained record per source/destination pair per 5 s |
| New side flow under 800 bytes | Exact source, destination, size, protocol, TTL, switch, and time | First two per flow signature; then 1/1,024 |
| New side flow at least 800 bytes | Same exact packet fields | First two per flow signature; then 1/65,536 |
| One-way link delay | Per-direction average and maximum keepalive delay | 2 s summaries |
| Link packet loss | Missing keepalive sequence count per direction | 2 s summaries |
| Link failure | Repeated `SILENT` records with both endpoints and local port | Detected after about 2 s without a keepalive |
| Link recovery | `HEARD` transition | First received keepalive after silence |
| Capacity/congestion | Per-port drop deltas, link delay, and keepalive gaps | Drop deltas every 0.5 s |

The source and destination records use 32-bit packet counts. This matters for
the known high-rate customer: a 16-bit count could saturate and make average
packet size or packet rate misleading.

## Why the packet sampling is safe

Open-loop planted packets have no sequence number, so the SDK exposes their
sequence as zero. A sequence-based “first two packets” test therefore punts
every open-loop packet. The final build instead hashes
`source XOR destination XOR size XOR protocol` into a 4,096-entry register.
Each hash bucket keeps a packet count:

- packets smaller than 128 bytes use the scan-specific rule;
- 128–799-byte flows keep their first two packets and 1/1,024 afterward;
- flows of 800 bytes or more keep their first two packets and 1/65,536 afterward.

This preserves exact endpoints for a newly planted flow while preventing the
known 1,000-byte hotspot from consuming the 80-event/s controller budget.
Hash collisions are possible, but they only suppress the first-two guarantee;
the periodic sample and the unsampled byte/packet counters still remain.

## Exact validation results

The final source uses eight TinyVM stages, four 64-entry counter arrays, and
one 4,096-entry register array. Unit tests confirm that the setup and timer
payload fit the 4,096-byte ABI limit.

The full eleven-scenario 15-switch grid suite passed at 1.5× load against the
final source. It covered baseline traffic, overlapping recovered failures,
small traffic, the known high-rate source, a hotspot directly on the logger,
three lossy links, permanent failures, scan multiplicity, all plant families
together, a dedicated capacity bottleneck, and the exact 127/128/799/800-byte
sampling boundaries.

Key results:

- records arrived from all 15 switches in every scenario;
- all 44 directed link views were present;
- every healthy stress run retained all 836 expected link summaries;
- the failure test retained all six failed directions and all six recoveries;
- the permanent-failure test retained all six failed directions and no false
  recovery;
- the loss storm recorded 542 missing keepalive sequences;
- the mixed test recorded a 30,199 µs delay peak, 67 gaps, all 50 scan
  destinations, and two exact 500-byte side-flow samples;
- the capacity test recorded 10,818 drops and two exact 1,000-byte side-flow
  samples;
- the largest 1.5× log was 51,398/65,536 bytes (78.4%);
- a separate 4× logger-hotspot test used 57,261 bytes (87.4%), retained all
  836 link summaries and all 50 scan destinations, and produced no false link
  failure.
- short 127-, 128-, 799-, and 800-byte planted flows were all retained on the
  correct side of both sampling boundaries at 0.25×, 1.5×, and 4× load;
- the exact upload loaded and completed all three official practice truth
  worlds, producing a valid 6,889-byte log in each.

## Remaining uncertainty

No local test can prove a hidden claim score. The important remaining risks
are interpretive rather than missing instrumentation:

- several eggs may create similar symptoms, such as loss, congestion, and a
  capacity reduction all increasing delay or drops;
- a flow-signature hash collision can reduce packet examples;
- an uncongested capacity increase may not create drops, so it must be inferred
  from keepalive serialization delay or compared with other link periods;
- source and destination counters are separate aggregates, so packet examples
  are the strongest evidence when several customers on one switch change at
  once;
- a correct detector must still reject all false and near-miss runs.

After the truth files return, decode the log, compare it with both earlier
truth logs, and write one explicit hypothesis per candidate egg. Claims should
be narrow and should test the exact switch/customer/link, time pattern, and
threshold supported by the evidence. Do not turn every unusual value into a
claim without a stable baseline or a matching transition.
