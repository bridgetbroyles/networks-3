# A3 Claim 13 submission

Claim 13 targets the simulator's explicit customer `ttl_ladder` behavior. The
named Mapper customer is a 64-byte UDP source at S4. It appears in the retained
truth runs; the TTL-capable truth log sampled its packets at low TTL, and an
earlier broad run captured its short 64-byte burst. The simulator source shows
that `ttl_ladder = 8` cycles packet starting TTLs from 1 through 8.

The retained TTL-capable run sampled Mapper sequences 152, 912, and 1064. All
three are divisible by eight and all appeared at TTL 1. The old observer sampled
every eighth sequence, so it aliased an eight-rung ladder onto one rung; this
explains the apparently constant truth evidence instead of contradicting the
ladder hypothesis.

The harness decrements TTL before the observation pipeline, so the source
switch sees rungs 0 through 7. The observer filters at S4 for 64-byte UDP and
punts only TTL values below 16. It retains at most one record for each exact
source/destination/TTL per second. The detector requires six distinct low TTL
rungs for one flow, including TTL 0 or 1.

The recreated false worlds include ordinary constant TTL, constant low TTL,
four- and five-rung short ladders, the eight-rung ladder at another switch,
wrong packet size, wrong protocol, an unrelated low-TTL flow, and insufficient
evidence. The normal practice customers are also false at all three practice
truth worlds and all three tested loads.

The exact package passed 528 whole-network cases, 13 encoded-record cases, all
nine practice-truth/load checks, and all 158 repository tests. The maximum
claim log was 7,435 of 24,576 bytes.

Upload only the two Wasm files under
`submission/next/claim-13-mapper-ttl-ladder/`, together in one claim.
