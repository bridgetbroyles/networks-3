# A3 Claim 15 result: no new egg

Claim 15 was submitted after Claim 14 returned. The server reported **no new
egg won**. It was independent of Claim 14 mechanically: Claim 14 targeted a
299-byte reverse side flow, while Claim 15 targeted repeated destination
cycling by the Login customer. However, a mechanically distinct rule was not
enough to establish a distinct hidden cause.

## Truth evidence

The Login ranges are stable across the retained truth runs. At S8, one source
address sends 100-byte traffic to three addresses in the other Login range.

- Truth Run 1 retained 11 packets from that one source to the three
  destinations between 13.8 and 14.2 seconds. Each destination appeared more
  than once.
- Truth Run 3 retained the same three-destination set near 5.0 seconds and the
  full set again near 10.0 seconds.
- Truth Run 2 mainly retained the reverse direction: three Login source hosts
  at S7 each contacted the single S8 address. That is consistent with the same
  three-host customer relationship and does not contradict the S8 fanout.

The public practice customers demonstrate the normal contrast: Jane Street
rotates among three source hosts toward one destination. No practice customer
has one source repeatedly cycle across three destinations.

## Detector rule and near misses

The observer runs locally at S8 and filters to 100-byte TCP. It retains at most
one sample per source/destination pair per second. The detector requires one
source to have at least three destinations, with at least two retained samples
per destination, all inside a six-second window.

The recreated negative cases include:

- only one or two destinations;
- three destinations spread too slowly;
- three source hosts that each use only one destination;
- a one-shot three-address scan;
- the same behavior at another switch;
- wrong packet size or protocol;
- missing, truncated, or malformed evidence.

The exact package passed 528 adversarial 15-switch runs, 12 record tests, all
158 repository tests, and stayed at or below 2,870 of 24,576 log bytes. The
nine disclosed practice runs returned false, as expected, but their six-switch
topology has no S8 and therefore cannot exercise the positive logger.

The exact failed pair is preserved under
`submission/history/failed-claim-15-login-three-destination-fanout/`. Do not
resubmit it. Its failure means the real Login fanout did not distinguish an
unwon egg from the grader's comparison run; it may be ordinary customer
behavior or behavior unaffected by the hidden egg toggle.
