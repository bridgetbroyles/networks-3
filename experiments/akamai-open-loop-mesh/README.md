# Research candidate: S7 open-loop many-to-many traffic

This experiment tests the strongest packet-level behavior not covered by
Claims 1-18. It is **not yet an approved Canvas upload**.

Truth Run 3 retained a 200-byte TCP, sequence-zero flow entering S7 with 24
source hosts in one `/24` and eight destination hosts in another `/24`. The
named ranges identify the direction as Akamai to Akamai users. The complete
run separates it from the other S7 sequence-zero families:

- Cloudflare: eight sources to eight fixed destinations using 1,000-byte
  packets; this is the best explanation for already-won Egg 10, The shuffle.
- Fastly: one source to 69 retained Home-user destinations using 200-byte
  packets; a focused claim for that behavior already failed.
- Akamai: 24 retained sources to eight destinations using 200-byte packets;
  this direction has not previously been claimed.

The focused detector groups records by source and destination `/24` and
requires at least 12 samples, eight source hosts, and eight destination hosts.
It also requires S7, TCP, 200 bytes, sequence zero, and customer-facing
ingress. Local worlds vary one dimension at a time: source multiplicity,
destination multiplicity, switch, size, protocol, and sample count.

The evidence proves that the observed pattern is real and that the proposed
observer can distinguish its structure. It does not prove whether the private
grader treats the pattern as an unwon egg, an effect of already-won Shuffle,
or background traffic. Packaging requires the audit and repository checks to
pass and a final causal-risk review.
