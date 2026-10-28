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

The new `focused` observer keeps the same complete source counters,
destination counters, link summaries, drop records, and report routing. It
changes only packet sampling: keep the first two packets of a flow, then one
packet in 8,192. On the observed Cloudflare rate this reduces ordinary punt
pressure by about eight times.

Local validation now checks all 836 expected two-second link summaries, not
merely whether each direction appeared once. It also includes a high-rate
source attached directly to S7, reproducing the Run 2 controller-pressure
pattern. The final candidate retained:

- records from all 15 switches;
- all 44 directed link views and all 836 expected summaries;
- no false SILENT/HEARD transitions;
- complete source and destination counter windows;
- a 47,631-byte log (72.7% of the truth cap) in the final 1.5x high-rate S7 test.

The exact candidate for Truth Run 3 is `submission/truth/switch.wasm`,
SHA-256:

```text
e76e922fcdaebb42e9279b99d98851ce094432dfeb828ce26645c8d7c4e2cbf1
```

## What to do next

Use the final truth attempt on the focused observer. The highest-value missing
evidence is a complete time series for every link and every customer counter,
not more Cloudflare packet examples. After the returned files arrive, compare
the three logs, write explicit hypotheses, and create narrow claim-specific
observers and detectors. Do not claim the repeated S7 silence artifacts.
