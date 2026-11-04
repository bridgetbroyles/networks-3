# S1 synchronized late-start experiment

Truth Run 3 contains a useful distinction that the failed camera claim did not
test. At S1, the high-rate Offsite-like 1,000-byte source is already visible at
30.6 seconds. Two other 1,000-byte sources, whose returned customer names are
both `Sync`, are absent in that same S1 snapshot and both first appear at 33.6
seconds with similar counts: 61 and 55 packets.

This claim logs locally at S1. It groups counters by the shuffled source prefix,
so no returned address or customer-list position is compiled into the rule. The
detector finds the first positive window for each 1,000-byte source and requires
two low-rate sources to first appear in the same 1.5-second observation window,
with first-window counts within 35 percent. Startup sources and high-rate late
sources are rejected.

Validation covers 360 whole-network cases over twelve seeds and three loads,
13 encoded-record cases, the nine official practice truth/load combinations,
and the complete repository test suite. The detector is true on Truth Run 3 and
false on Truth Runs 1 and 2, whose observers did not retain the decisive S1
activation records. The largest synthetic claim log was 1,252 of 24,576 bytes.

This validates the instrumentation and predicate, not the hidden causal label.
The reason to prefer this over failed Claim 16 is that it targets the distinct
late activation of the two named Sync customers, rather than congestion-driven
rate equality between two always-active cameras.
