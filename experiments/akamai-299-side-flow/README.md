# S2 Akamai-users 299-byte side-flow experiment

This experiment recreates a `side_flow` from a normally receiving customer at
S2 back toward its sender. The observed truth signature is a steady TCP stream
of exactly 299-byte packets. Truth Run 3 contains 26 source windows averaging
exactly 299 bytes and four exact packet samples; earlier observers did not have
complete counters for this customer.

The observer filters locally at S2, then retains at most one exact-flow sample
per half second. The detector requires three distinct sequence numbers in three
time windows spanning at least one second. Near misses cover adjacent sizes,
UDP, another site, brief traffic, split flows, and ordinary baseline traffic.
