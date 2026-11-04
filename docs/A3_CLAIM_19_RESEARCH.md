# Claim 19 research and evidence gate

## Current position

Claim 18 won no new egg. The confirmed score is **68/200**, with **18 of 30
claims used** and **12 remaining**. A revised Home-user UDP-surge detector has
now passed the evidence gate and is packaged as the recommended Claim 19.

## What was rechecked

The handout's comparison rule is decisive: while one target egg is toggled,
every other egg remains on. A truth-run pattern is therefore not enough. A
claim needs an observable that changes when one particular hidden mechanism
is disabled or replaced by a near miss.

The three truth logs were decoded again at record level. The archived 28
candidate detectors were also run against all three original team logs. None
of the still-unwon candidate predicates returned true in all three logs. Most
Truth-Run-3-only positives came from wider coverage in the final observer, not
from contradictory truth behavior.

The following server failures remain strong negative evidence:

- customer-induced queueing on links 12-13 and 6-7 is not an unwon capacity
  egg;
- the broad Home-user incast rule was insufficient;
- the Ledger silence did not classify an unwon customer egg;
- the 299-byte Akamai reverse stream, Login three-target pattern, synchronized
  camera rates, synchronized Sync start, and Fastly fanout did not classify an
  unwon egg.

## Rejected Akamai mesh hypothesis

The first new lead was a 200-byte Akamai pattern at S7 with at least eight
source and eight destination addresses. It passed its local implementation
tests but was rejected before packaging. The simulator deliberately exposes
planted packets with synthetic increasing sequence numbers, while the Akamai
samples were sequence zero. The pattern is therefore consistent with ordinary
first TCP packets or retransmissions and does not map cleanly to an
independently switchable plant. The rejected experiment remains under
`experiments/akamai-open-loop-mesh/` so it is not rediscovered later.

## Mixed Home-user traffic evidence

The Home-user source at S2 shows a qualitatively different mechanism:

- normal traffic is 200-byte TCP to Fastly;
- for about 15 seconds, the same source prefix additionally emits 64-byte UDP
  to the same destination;
- the UDP component uses at least 22 source hosts;
- the source counter changes from about 155 packets per 1.5-second window to
  about 4,673 packets per window, then returns to baseline.

That combination maps cleanly to the simulator's `side_flow` mechanism: one
ordinary app row cannot independently choose two packet sizes and two
protocols. The handout says truth runs contain normal customer traffic plus
the eggs and no other planted behavior. This supplies a causal connection to
an independently toggleable mechanism rather than merely an unusual traffic
correlation.

## Why this is not a resubmission of failed Claim 8

Claim 8 targeted the same episode but its rule was materially incomplete. It
needed only three high windows and five seconds of samples; it did not require
recovery, a bounded sustained duration, the 200-byte baseline, or exactly one
destination.

The old submitted detector was run against the new whole-network audit logs.
It returned true for all twelve cases in each of these false-positive
families:

- a surge that never recovered;
- a 24-second surge;
- a surge layered over a 500-byte rather than 200-byte baseline; and
- five simultaneous destinations rather than one.

Any one of those private near-miss families would explain Claim 8's failure.
The revised detector rejects every one of those cases.

## Revised rule

The detector requires the complete observed shape:

- an S2 `/24` with ordinary 180-220-byte counter windows before and after;
- a contiguous 9-18-second high interval with at least six counter windows;
- at least 500 packets per high window and at least ten times baseline;
- high-window average size 60-100 bytes;
- sampled 64-byte UDP from at least five hosts in the source prefix;
- exactly one sampled destination; and
- packet samples spanning at least eight seconds and overlapping the high run.

The event's absolute start time is deliberately ignored because the practice
claim generator moves true events. Sequence number is also ignored because the
simulator assigns planted packets a synthetic visible sequence.

## Verification

The final exact pair passed:

- 168/168 fresh whole-network cases across four seeds and loads 1.0, 1.25,
  and 1.5;
- positive cases at ordinary, early, and late start times;
- off, five-second, no-recovery, 24-second, low-rate, wrong-size,
  wrong-protocol, four-source, wrong-site, wrong-baseline, and
  five-destination negatives;
- unrelated loss, one-way delay, symmetric delay, and a recovered link
  failure active in every generated case;
- a fresh exact positive with result `true`;
- all nine ordinary practice-truth/load runs with result `false`;
- Truth Run 3 with result `true`;
- Truth Runs 1 and 2 with result `false`, as expected because those earlier
  observers did not record the required counter and packet telemetry; and
- the 24,576-byte log limit, with a maximum of 11,664 bytes.

## Conclusion and remaining uncertainty

The revised pair is recommended for Claim 19. This is not a promise about the
private ten-run boundary: no local test can reveal that boundary. It is,
however, substantially stronger than Claim 8 because it fixes four concrete
false-positive families that the old submitted detector accepted and ties the
event to an explicit independently planted simulator mechanism.
