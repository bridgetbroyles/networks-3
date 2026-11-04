# A3 Claim 18 result

## Server result

Claim 18 won **no new egg**. The confirmed total remains **68/200** after
eighteen claims, leaving twelve attempts. The exact pair is archived under
`submission/history/failed-claim-18-fastly-destination-fanout/` and must not be
submitted again.

The result disproves the causal hypothesis, not the packet measurements. The
S7 sequence-zero fanout is real and the detector implemented its rule
correctly, but that rule did not classify all ten cases for any unwon egg. It
may be normal workload behavior, an effect of already-won Egg 10, or a feature
whose hidden near misses differ from the proposed 50-address/five-second rule.

## Hypothesis

One remaining traffic egg makes a single source at S7 fan out unnumbered
200-byte TCP packets to at least 50 addresses in one destination `/24` within
five seconds.

This is not the three-destination Login behavior from failed Claim 15. It is
also independent of the 1,000-byte Cloudflare one-to-one shuffle, which is the
strongest packet-level explanation for already-won Egg 10.

## Direct truth evidence

The same exact source and destination range appears in all three returned
truth logs:

| Run | Retained samples | Distinct destinations | Retained span | Sequence / size |
| --- | ---: | ---: | --- | --- |
| Truth 1 | 186 | 121 | 1.2-38.8 s | 0 / 200 B |
| Truth 2 | 259 | 128 | 1.0-38.7 s | 0 / 200 B |
| Truth 3 | 94 | 69 | 1.2-3.8 s | 0 / 200 B TCP |

Truth Runs 1 and 2 used a sparse sampler, so they prove persistence and address
diversity but do not retain every event. Truth Run 3 used the corrected packet
format and proves the threshold directly: one exact source reached 69 distinct
destinations in 2.6 seconds. The next-largest 200-byte sequence-zero source in
that log reached only five destinations in any five-second window.

Normal mixed-workload data is numbered. The focused observer checks packet
kind, size, protocol, ingress type, switch, and sequence in the data plane, so
ordinary numbered 200-byte customer traffic never enters the controller queue.

## Exact rule

- observe only at switch 7;
- require data kind, customer-facing ingress, TCP, 200 bytes, sequence zero;
- retain each exact `(source, destination)` pair once;
- group by exact source and destination `/24`;
- return true only if one group contains at least 50 distinct destinations in
  a five-second window.

The exact source address and prefix are not hard-coded, so address reshuffling
does not affect the detector.

## Verification

- 96/96 synthetic whole-network cases passed across four seeds and loads 1.0,
  1.25, and 1.5.
- The negatives include no fanout, 49 destinations, 50 destinations spread too
  slowly, 50 destinations split across two sources, the wrong switch, and
  199- and 201-byte near misses.
- 12/12 direct encoded-log tests passed, including malformed input, duplicate
  destinations, UDP, numbered packets, and wrong-switch records.
- The exact detector returns true on Truth Run 3. It returns false on the older
  sparse Truth 1/2 logs because those observers intentionally discarded too
  many packets inside each five-second window; that is an observer difference,
  not contradictory traffic evidence.
- The exact focused observer/detector pair loaded and passed every generated
  network case. The largest log was 1,280 of 24,576 bytes.
- The full Rust workspace passed all tests (114 simulator tests plus every
  additional crate/doctest group).

## Confidence and remaining uncertainty

This is the strongest unclaimed candidate after Claim 17 because it is a
packet-level mechanism, not a rate correlation or a queue side effect. The
truth evidence establishes the exact positive predicate and clean separation
from other S7 traffic.

No local test can prove the server's private causal label or its ten hidden
near misses. The remaining risk is that this fanout is ordinary baseline
behavior or belongs to already-won Egg 10. That risk is lower than for Claims
14-17: Egg 10's name and the Cloudflare 8-by-8 fixed permutation fit the
1,000-byte shuffle much more directly, while this source reaches over half of
an entire `/24` in seconds.

## Retired files

```text
submission/history/failed-claim-18-fastly-destination-fanout/switch.wasm
submission/history/failed-claim-18-fastly-destination-fanout/detect.wasm
```

Hashes:

```text
cf09a35036abe004e8e7c71a9152090b14d34fb4ba022e0b1f78587e157812aa  switch.wasm
fbce6d0e67bf97e848988473e3bce6dc93996d58feb81a4b34ea1ce0e400a3b7  detect.wasm
```
