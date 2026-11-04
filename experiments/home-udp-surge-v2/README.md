# Research candidate: sustained Home-user UDP surge and recovery

This is a second-generation investigation of the real mixed-traffic episode
that the earlier Claim 8 detected too broadly. After the final causal review
and fresh 168-case audit, its exact binaries were approved and copied to
`submission/next/claim-19-home-udp-surge/`.

The handout says the truth worlds contain only normal customer traffic plus
the eggs. The same Home-user source range normally sends numbered 200-byte TCP
traffic, but from about 15 to 30 seconds it additionally emits unnumbered
64-byte UDP at roughly thirty times its baseline packet rate. One normal mixed
customer row cannot change size and protocol this way, while the simulator's
explicit `side_flow` plant can. This makes the episode independently planted
evidence rather than an unexplained rate correlation.

Claim 8 required only three high windows and did not require recovery. This
detector instead requires the complete shape: a 200-byte baseline, a sustained
9-18 second high-rate 64-byte interval, UDP samples from at least five
source hosts to one destination, and observed normal windows both before and
after the surge. Absolute start time is deliberately not part of the
rule because the practice claim generator moves true events between runs. The
local near misses independently change duration, recovery, rate, size,
protocol, source count, site,
baseline packet size, and destination multiplicity.

The simulator assigns planted packets a synthetic visible sequence even though
they are unacknowledged, specifically so a program cannot classify a plant by
an internal marker. The detector therefore does not use sequence zero; it uses
only behavior visible in real traffic.

The local audit establishes implementation and boundary behavior. It cannot
reveal the private egg name or prove that every hidden near miss uses the
inferred boundaries. The final review did establish a substantive difference
from failed Claim 8: the old detector also accepted no-recovery, 24-second,
wrong-baseline, and five-destination cases, while this detector rejects them.

The final fresh audit passed 168/168 generated whole-network cases over four
seeds and loads 1.0, 1.25, and 1.5. The largest log was 11,664/24,576 bytes.
The exact detector is true on Truth Run 3, where the needed packet and counter
telemetry exists, and false on all nine ordinary practice-truth/load runs.
