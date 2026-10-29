# Assignment 3 Quiz Cheat Sheet — Our Program

## Evidence first

A3 questions are about what our submitted programs literally do. Separate the
real event, data-plane counts, punts surviving the 80-event/s budget, reports
reaching the logger, bytes fitting the cap, and the detector’s inference. An
event can happen without appearing in the log. The detector uses only that log.

`arrived_ns` is packet arrival; `now_ns` may be later because its event waited.
Reports then incur action delay, queueing, propagation, and another punt before
the logger writes them.

## Actual architecture

The harness forwards customer traffic; our routes and queues do not control it. Our seven-stage data plane does observation:

1. classify ingress as switch-facing if that port has produced a keepalive;
2. on customer-facing ingress, map source prefix to internal customer ID;
3. map the destination prefix independently;
4. count source bytes;
5. count source packets;
6. count destination bytes;
7. punt keepalives and reports. Compact mode punts every packet smaller than
   128 bytes. The final focused truth mode samples every eighth small-packet
   sequence number, keeps the first two examples below 800 bytes, and does not
   punt packets of 800 bytes or more. Large traffic is still fully counted.

Counting only at customer ingress avoids counting one packet at every hop. Before keepalives teach switch ports, transit traffic could look local, so our controller ignores customer statistics before one second. Each counter has 64 slots: slot 0 is unclassified and customer IDs use 1–63.

Every 500 ms the controller reads cumulative counters. Compact mode reports
source deltas every 500 ms; truth mode reports source deltas every 1.5 seconds
and destination byte deltas every four seconds. The local practice build uses switch
0 as logger. The final truth build uses switch 2, near the center of the 3x5 grid but away from the measured high-rate customer. On that
grid, each routine report follows one decreasing-distance next hop. Sequence
numbers spread reports across equal shortest parents, and locally silent links
are avoided. Repeated silent-link reports and one-time recovery reports use
duplicate-suppressed flooding so a changed link cannot erase its transition
evidence. Practice mode floods all reports.

Every stored record is `[tag u8][length u8][body]`. Tags identify segmented headers, source statistics, small packets, link silent/heard, link summaries, ordinary packet samples, drops, and destination statistics.

## Link failure questions

The harness sends a keepalive every 100 ms on every switch-to-switch direction. Our switch remembers the last arrival and calls a port silent after twenty missing periods:

`quiet threshold = 20 × 100 ms = 2000 ms`.

Silence is checked only on a 500 ms timer. A rough first local detection bound after the last received keepalive is:

`2000 ms quiet + up to 500 ms timer alignment`.

Then add report action delay (practice value 2 ms), serialization, link propagation (500 µs per practice hop), customer-induced report queueing, and controller-event service at every reporting hop. In the final measured truth replay, link 1–2 failed at 17.2 s and both directions appeared silent around 19.5 s: approximately 2.3 s later. Do not automatically answer exactly 2 s.

The broken-link detector tracks the final record for each directed neighbor pair. It returns true only if both `(1,2)` and `(2,1)` end silent. If 1–2 goes down and returns, later HEARD records make the answer false. If another link fails, the target pairs are absent or heard, so false. One missing keepalive is not enough.

If asked what could cause a false positive, name prolonged keepalive loss or extreme controller contention in both directions. Requiring both endpoints and twenty periods reduces that risk but does not make measurement perfect.

## Customer activity and pause questions

A compact claim statistic contains one 500 ms interval’s source bytes and packets. Compute active rate as:

`rate_bps = bytes × 8 / 0.5 = bytes × 16`.

Average packet size is:

`average bytes/packet = bytes / packets`.

Example: 105,000 bytes and 105 packets in one record means 1000-byte packets and `105,000 × 16 = 1.68 Mb/s`.

The long-pause detector must find Spotify without names because prefixes and customer order are shuffled. It accepts a customer identity candidate when average packet size is at least 800 bytes, median active rate is 0.7–4 Mb/s, and activity outside the largest gap is at least 82%. These features reject 200-byte Fortnite/Jane Street, high-rate unlimited Steam, and Amazon’s repeated eight-second off periods.

It then looks for a step of at least 100 units of 100 ms between active records. Because records normally end about 500 ms apart, this represents roughly 9.5 seconds with no complete active window, consistent with a ten-second pause at arbitrary phase. A two-second Spotify pause is too short. A ten-second Steam pause has the duration but fails the Spotify identity test. True practice pauses last 12 seconds and produced about an 11.6-second measured gap.

Truth records aggregate 1.5 seconds instead, so use
`rate_bps = bytes × 8 / 1.5`. If an event begins or ends inside a measurement
interval, that interval may contain some bytes. This is why measured missing
time need not equal the planted duration exactly.

## Scanner questions

The scanner packets are 60-byte data, below our 128-byte threshold. Compact
claim mode punts them only at source ingress, preventing per-hop duplicates. A
record stores source, destination, time in 100 ms units, and size. Truth mode
retains every eighth sequence number and writes the same source/destination
pair at most once per five seconds; a 50-address synthetic scan retained eight
different targets.

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

Practice export limits are 16,000 bytes/s sustained and 8,000 bytes burst. An
800-byte payload costs about 840 bytes including headers, so a full bucket fits
`floor(8000/840)=9`; tokens refill at 16 bytes/ms.

Reports use a low-priority four-packet queue. They never delay customer data,
but may be late or dropped, especially near the run’s end.

## Log-budget calculations

The local practice claim cap is 16,384 bytes, the graded claim cap is 24,576 bytes, and the truth cap is 65,536 bytes. A write that would cross the cap is lost entirely; earlier bytes remain. The exact final hidden artifact produced 5,623-byte logs on each practice truth world. Its largest measured 15-switch stress log was 47,618 bytes, leaving 17,918 bytes.

For fixed record size `R` and cap `C`, maximum complete records is `floor(C/R)`, but include the two-byte tag/length header and initial customer header. Do not assume a truncated final record: writes are atomic, so an oversized write is absent.

## Detector and claim reasoning

A detector returns one bit. Winning requires 10/10; near misses mean a good
rule states both the positive condition and what rejects alternatives.

Broken link: correct endpoints, both directions, final state silent. Long pause: correct behavioral customer identity, long duration, traffic before and after. Scanner: one source, 50 distinct destinations, five-second window. Our final practice results are 10/10 for each egg.

Truth mode gathers broad evidence, not universal answers. After identifying a
pattern, predict near misses, retain only needed records, and build one narrow
detector. Always-true loses every false run.

## Likely “what if” questions

If the timer changes from 500 to 100 ms, pause timing is more precise but there are five times as many reports and records. If the silence threshold changes from twenty to ten periods, failure detection is about one second faster but stress testing showed congestion can create false failures. If the hidden logger were switch 0 instead of near-central switch 2, reports would travel farther. If a selected report link dies, routine reports avoid it after silence detection; repeated failure records are flooded over the remaining links. If the logger is isolated, remote evidence cannot arrive. Sampling more frequently improves packet detail but can consume the 80-event/s budget and fill the log; both truth run 1 and the first full-grid stress test demonstrated that failure directly.

## Final answer checklist

Identify the real event; identify the measurement mechanism; apply event, export, timer, and log limits; state the exact record the detector needs; apply the detector threshold; test the near miss; and distinguish “the event happened” from “our log contains enough evidence to prove it.”
