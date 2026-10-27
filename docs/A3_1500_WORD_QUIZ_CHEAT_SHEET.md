# Assignment 3 Quiz Cheat Sheet — Our Program

## Evidence first

A3 questions are about what our submitted observer and detector literally do. Separate: the real network event, what the data plane counted, which packets were punted, which controller events survived the 80-event/s budget, which reports survived export and queueing, what reached the logger, what fit in the log, and what the detector inferred. An event can happen without appearing in the log. The detector cannot use the topology, customer names, full simulator trace, or facts we forgot to encode.

For a shown moment, use only records already written. `arrived_ns` is when an observed packet reached a switch; `now_ns` may be later because the controller event waited. Reports then need controller action delay, link queueing, propagation, another punt, and possibly several hops before the logger writes them.

## Actual architecture

The harness forwards customer traffic; our routes and queues do not control it. Our five-stage data plane does observation:

1. classify ingress as switch-facing if that port has produced a keepalive;
2. on customer-facing ingress, map source prefix to internal customer ID;
3. add packet size to that customer’s byte counter;
4. add one to its packet counter;
5. punt keepalives, our reports, all data packets smaller than 128 bytes, and in truth mode one ordinary source packet in 64.

Counting only at customer ingress avoids counting one packet at every hop. Before keepalives teach switch ports, transit traffic could look local, so our controller ignores customer statistics before one second. There are 16 counter slots: slot 0 is unclassified and customer IDs use 1–15.

Every 500 ms the controller reads cumulative counters and reports deltas. The local practice build uses switch 0 as logger. Both graded builds enable the `hidden` feature and use switch 7, the center of the 3x5 hidden grid. Other switches batch observations into at most 800-byte report payloads and flood them over known switch links. Reports carry `(origin,sequence)`; every switch forwards each pair once, so flooding does not create duplicate log records. Flood fanout is bounded to respect the 4,096-byte action-output ceiling.

Every stored record is `[tag u8][length u8][body]`. Tags identify header, customer statistic, small packet, link silent, link heard, link summary, ordinary packet sample, or drops. Claim mode logs the first five. Truth mode logs all eight.

## Link failure questions

The harness sends a keepalive every 100 ms on every switch-to-switch direction. Our switch remembers the last arrival and calls a port silent after ten missing periods:

`quiet threshold = 10 × 100 ms = 1000 ms`.

Silence is checked only on a 500 ms timer. A rough first local detection bound after the last received keepalive is:

`1000 ms quiet + up to 500 ms timer alignment`.

Then add report action delay (practice value 2 ms), serialization, link propagation (500 µs per practice hop), customer-induced report queueing, and controller-event service at every reporting hop. In the measured truth run, link 1–2 failed at 17.2 s and both directions appeared silent around 18.5 s: approximately 1.3 s later. Do not automatically answer exactly 1 s.

The broken-link detector tracks the final record for each directed neighbor pair. It returns true only if both `(1,2)` and `(2,1)` end silent. If 1–2 goes down and returns, later HEARD records make the answer false. If another link fails, the target pairs are absent or heard, so false. One missing keepalive is not enough.

If asked what could cause a false positive, name prolonged keepalive loss or extreme low-priority report delay in both directions. Requiring both endpoints and ten periods reduces that risk but does not make measurement perfect.

## Customer activity and pause questions

A statistic record contains one 500 ms interval’s source bytes and packets. Compute active rate as:

`rate_bps = bytes × 8 / 0.5 = bytes × 16`.

Average packet size is:

`average bytes/packet = bytes / packets`.

Example: 105,000 bytes and 105 packets in one record means 1000-byte packets and `105,000 × 16 = 1.68 Mb/s`.

The long-pause detector must find Spotify without names because prefixes and customer order are shuffled. It accepts a customer identity candidate when average packet size is at least 800 bytes, median active rate is 0.7–4 Mb/s, and activity outside the largest gap is at least 82%. These features reject 200-byte Fortnite/Jane Street, high-rate unlimited Steam, and Amazon’s repeated eight-second off periods.

It then looks for a step of at least 100 units of 100 ms between active records. Because records normally end about 500 ms apart, this represents roughly 9.5 seconds with no complete active window, consistent with a ten-second pause at arbitrary phase. A two-second Spotify pause is too short. A ten-second Steam pause has the duration but fails the Spotify identity test. True practice pauses last 12 seconds and produced about an 11.6-second measured gap.

If an event begins or ends within a 500 ms interval, that interval may contain some bytes. This is why measured missing time need not equal the planted duration exactly.

## Scanner questions

The scanner packets are 60-byte data, below our 128-byte threshold. They are punted only at source ingress, preventing per-hop duplicates. A record stores source, destination, time in 100 ms units, and size.

The detector groups records by source and slides a five-second window. It counts distinct destinations, not packets. It returns true at 50 distinct destinations.

- 50 destinations over four seconds: true.
- 60 packets to only three destinations: false.
- 50 destinations over 36 seconds: false.
- 49 destinations within five seconds: false.

The practice true scan rate is `50/4 = 12.5 packets/s`, below the controller’s 80-event/s limit at the source. The scanner is still vulnerable if competing punts fill the 64-event backlog or if reports are lost. Our source filtering and counter-based normal monitoring reserve punt capacity for detailed evidence.

## Event-budget calculations

At 80 controller events/s, service spacing is:

`1/80 s = 12.5 ms per event`.

The backlog holds 64 pending events. If 64 arrive simultaneously, the last admitted event can wait roughly:

`63 × 12.5 ms = 787.5 ms`, plus normal pipeline latency.

The 65th pending event is lost. This does not drop the original customer packet; only its observation copy disappears. Counters remain more reliable because they run in the data plane on every packet.

## Export and report calculations

Practice export limits are 16,000 bytes/s sustained and 8,000 bytes burst. A report is charged at least a 40-byte header plus payload. An 800-byte payload costs about 840 bytes. Starting from a full bucket, at most `floor(8000/840)=9` such reports fit immediately; tokens then refill at 16 bytes/ms.

Reports use the low-priority report queue and never delay customer data. On a busy link, they may be late or dropped when the four-packet report queue fills. Flooding provides alternative paths, but a record generated in the last moments may still miss the logger before the 40-second run ends.

## Log-budget calculations

The local practice claim cap is 16,384 bytes, the graded claim cap is 24,576 bytes, and the truth cap is 65,536 bytes. A write that would cross the cap is lost entirely; earlier bytes remain. Our compact all-eggs practice log measured 5,864 bytes: 10,520 bytes remain under the practice cap and 18,712 under the graded claim cap. The truth log measured 41,655 bytes, leaving 23,881 bytes.

For fixed record size `R` and cap `C`, maximum complete records is `floor(C/R)`, but include the two-byte tag/length header and initial customer header. Do not assume a truncated final record: writes are atomic, so an oversized write is absent.

## Detector and claim reasoning

A detector returns one bit. Winning requires all ten runs correct; 9/10 earns nothing. False runs include near misses, so a good answer must state both the positive condition and what rejects alternatives.

Broken link: correct endpoints, both directions, final state silent. Long pause: correct behavioral customer identity, long duration, traffic before and after. Scanner: one source, 50 distinct destinations, five-second window. Our final practice results are 10/10 for each egg.

Truth mode is not itself a universal hidden-egg detector. It gathers broad evidence: rates, packet sizes, source/destination samples, link delay, keepalive gaps, drops, and state changes. After observing a hidden pattern, the competitive strategy is to formulate likely near misses, retain only necessary records, and create one narrow detector. A detector that always returns true loses every false run.

## Likely “what if” questions

If the timer changes from 500 to 100 ms, pause timing is more precise but there are five times as many reports and records. If the silence threshold changes from ten to three periods, failure detection is about 700 ms faster but congestion causes more false positives. If the hidden logger were switch 0 instead of central switch 7, reports would travel farther on average and face more low-priority queues. If the log cap halves, truth mode may overflow but claim mode still fits the measured practice case. If the report path breaks, flooding may use another path; if the logger itself is isolated, remote evidence cannot arrive. If packet sampling changes from 1/64 to 1/16, visibility improves fourfold but so do punt, export, and log pressure.

## Final answer checklist

Identify the real event; identify the measurement mechanism; apply event, export, timer, and log limits; state the exact record the detector needs; apply the detector threshold; test the near miss; and distinguish “the event happened” from “our log contains enough evidence to prove it.”
