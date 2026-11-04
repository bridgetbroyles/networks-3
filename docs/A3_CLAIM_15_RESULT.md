# A3 Claim 15 server result

Claim 15 won no new egg. The score remains 68 points with fifteen of thirty
claim attempts used and fifteen remaining.

The submitted pair correctly detected the repeated Login fanout that it was
designed to detect. Its local validation therefore established implementation
correctness, but not hidden-egg causality. The server result shows that toggling
an unwon egg did not produce the required detector difference. The fanout may
be ordinary customer behavior or may be unrelated to the hidden mechanism that
the grader toggled.

The exact pair is archived under
`submission/history/failed-claim-15-login-three-destination-fanout/` and must
not be submitted again. Claim 16 remains independent: it examines synchronized
positive packet-rate changes between two sources at S6 rather than destination
fanout at S8.
