# S4 Mapper TTL-ladder experiment

This experiment tests whether the 64-byte UDP Mapper source at S4 cycles
through low starting TTLs. The harness decrements TTL before the observation
pipeline, so an eight-rung application ladder appears locally as TTL 0 through
7. The data plane punts only TTLs below 16, and the controller retains at most
one record per source, destination, and TTL per second.

The detector requires at least six distinct low TTL values for one exact flow,
including TTL 0 or 1. This tolerates limited observation loss while rejecting a
constant low TTL and short ladders of four or fewer rungs.
