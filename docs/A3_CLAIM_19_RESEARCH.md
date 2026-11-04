# Claim 19 research and evidence gate

## Current position

Claim 18 won no new egg. The confirmed score is **68/200**, with **18 of 30
claims used** and **12 remaining**. There is deliberately no upload pair in
`submission/next/`: no candidate has yet met the evidence standard needed to
spend Claim 19.

## What was rechecked

The handout's comparison rule is decisive: while one target egg is toggled,
every other egg remains on. A truth-run pattern is therefore not enough. A
claim needs an observable that changes when one particular hidden mechanism
is disabled or replaced by a near miss.

The three truth logs were decoded again at record level. The archived 28
candidate detectors were also run against all three original team logs. None
of the still-unwon candidate predicates returned true in all three logs. Most
truth-run-3-only positives came from wider coverage in the final observer, not
from three independent confirmations.

The following server failures remain strong negative evidence:

- customer-induced queueing on links 12-13 and 6-7 is not an unwon capacity
  egg;
- Home-user high-volume UDP incast did not classify an unwon egg;
- the Ledger silence did not classify an unwon customer egg;
- the 299-byte Akamai reverse stream, Login three-target pattern, synchronized
  camera rates, synchronized Sync start, and Fastly fanout did not classify an
  unwon egg.

## Nested-customer audit

The truth contract contains three unusual nested source ranges:

- Netflix live `/32` inside Netflix `/24`;
- Amazon Prime member `/32` inside Amazon shoppers `/24`;
- Fortnite player `/32` inside Fortnite `/24`.

The nesting is stable, but nesting alone cannot be a claim: it would remain
present in both the egg-on and egg-off runs. The old selective-pause detectors
were correctly held back because they returned false on all three truth logs.
The visible rate dips are also plausibly explained by congestion control and
lost observer records. No nested-prefix pair is ready to submit.

## Mixed Home-user traffic audit

The most promising newly examined mechanism is the Home-user source at S2:

- normal traffic is 200-byte TCP to Fastly;
- from about 15 to 30 seconds in Truth Run 3, the same source prefix also
  emits 64-byte UDP to the same destination;
- the UDP component uses at least 22 source hosts;
- the source counter changes from about 155 packets per 1.5-second window to
  about 4,673 packets per window, then returns to baseline.

That combination maps cleanly to the simulator's `side_flow` mechanism: one
ordinary app row cannot independently choose two packet sizes and two
protocols. However, Claim 8 already targeted this episode and won nothing.
The old detector's thresholds do appear over-specified, but the server result
also permits the episode to belong to already-won Egg 10, *The shuffle*.
Submitting a weaker variant now would repeat an inadequately distinguished
hypothesis, so it is not promoted.

## Candidate gate for Claim 19

A candidate will be packaged only if all of the following hold:

1. The exact detector is true on retained truth evidence or the missing
   evidence is explained by a specific observer limitation.
2. The behavior maps to an independently toggleable simulator/app mechanism.
3. A focused local world makes the behavior appear and an otherwise identical
   world with only that mechanism removed makes it disappear.
4. Near misses change one dimension at a time: customer/site, direction,
   packet size, protocol, duration, multiplicity, or threshold.
5. Already-won eggs and known background behavior remain active in both local
   sides.
6. The focused observer stays under the 24,576-byte claim cap and rejects
   missing telemetry rather than interpreting it as silence.
7. The exact packaged WebAssembly files pass loading, workspace tests, the
   practice checker, and hash verification.

## Current conclusion

No Claim 19 upload is recommended yet. The investigation has ruled out the
old nested-prefix detectors and has shown that a simplified Home-user
mixed-traffic detector would be a risky retry of Claim 8 or an attempt to
rediscover already-won Egg 10. The next work should focus on an observable not
already covered by Claims 1-18, with particular attention to explicit
protocol/sequence changes and physical-link behavior that can be measured
directly rather than inferred from queueing.
