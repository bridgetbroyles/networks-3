# S8 Login repeated three-destination fanout

This experiment recreates the truth-log behavior where one 100-byte TCP Login
source at S8 reaches three destinations in the other Login range and repeats
the same destination set. Normal practice customers may rotate source hosts,
but none has one source repeatedly cycling across three destinations.

The observer retains at most one sample per source/destination each second.
The detector requires three destinations that each appear at least twice for
the same source inside six seconds. This rejects a one-shot scan, a two-target
flow, slowly spread destinations, and three sources each using one destination.
