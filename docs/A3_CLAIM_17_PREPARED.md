# A3 Claim 17 prepared

Claim 17 targets synchronized late activation at S1. Truth Run 3 retained an
S1 source snapshot at 30.6 seconds containing the high-rate Offsite-like source
but neither Sync source. At 33.6 seconds, two distinct 1,000-byte sources whose
returned names are both Sync appear together with 61 and 55 packets.

The claim keeps the logger at S1 and counts source prefixes locally. Prefixes
may be reshuffled without changing the rule. For every 1,000-byte source, the
detector takes its first positive counter window and requires a pair that:

- starts after the initial five seconds;
- first appears in the same 1.5-second window;
- has between 3 and 120 packets in that first window; and
- differs in initial count by no more than 35 percent.

The lower timing bound rejects startup traffic. The count ceiling rejects the
unrelated high-rate S1 source. The ratio test rejects two coincidental late
starts with materially different rates.

The exact package passed 360 whole-network cases across twelve seeds and loads
1.0, 1.25, and 1.5. It also passed 13 record checks, all nine official practice
truth/load negative checks, and all 158 repository tests. Maximum claim-log use
was 1,252 of 24,576 bytes. The detector returns true on Truth Run 3; the first
two truth observers did not retain the decisive S1 activation records.

This is the strongest unused customer hypothesis currently supported by the
returned names and counters. It is not a guaranteed hidden win: only the server
can establish that the Sync relationship is itself an egg rather than normal
customer configuration.
