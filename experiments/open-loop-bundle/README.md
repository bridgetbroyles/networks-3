# Rejected experiment: combined S7 open-loop fingerprint

**Do not submit this pair.** It was useful for separating genuine sequence-zero
traffic from numbered customer traffic, but its detector requires three
different truth-run signatures at once. Claim runs, like the disclosed
practice sets, isolate one target egg and its near misses; the other two
signatures are not guaranteed to remain present. The focused successor is
`experiments/fastly-destination-fanout/`.

Truth Runs 1 and 2 retained three distinct families of data packets entering
S7 with sequence number zero: one 1,000-byte family and two 200-byte families.
Ordinary mixed-workload customer data is sequence-numbered, and the observer
rejects acknowledgments by packet kind before sampling. The three families are
therefore direct evidence of open-loop mechanisms rather than inferred rate
correlations.

The detector groups records by source `/24`, destination `/24`, and packet
size. The observer retains each exact source/destination pair once. A group
qualifies after at least five retained pairs and at least five
distinct source or destination addresses. A positive log needs at least one
qualifying 1,000-byte group and two qualifying 200-byte groups at S7.

This deliberately treats the three structural signatures as one fingerprint.
If any represented unwon traffic egg controls one family, disabling that egg
breaks the fingerprint even though all other eggs remain active. The main
uncertainty is that some or all families could be effects of Egg 10, which is
already won; local simulation cannot resolve that server-side causality.
