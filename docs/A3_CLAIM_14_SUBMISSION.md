# A3 Claim 14 submission

Claim 14 targets a reverse stream from the normally receiving Akamai-users
range at S2. Truth Run 3 contains 26 source windows for this customer, all
averaging exactly 299 bytes per packet, plus four exact 299-byte TCP samples
back toward Akamai. The sample sequence numbers 8, 15, 33, and 93 are
consistent with the simulator's unnumbered `side_flow` packets being exposed as
an ordinary sequence at observation time.

The earlier truth observers did not provide a contradiction: Truth Run 1
installed counters for only the first 15 of 54 ranges, while Akamai users is
range 26; Truth Run 2 lost most remote source statistics to controller
overload. The final observer is the first one that measured this customer
continuously.

The recreated ON worlds use the simulator's explicit `side_flow` plant from a
receiver at S2. OFF worlds keep the ordinary 200-byte forward customer traffic
but omit the reverse stream. Near misses use 298 or 300 bytes, UDP, another
source switch, fewer than three retained packets, a sub-second burst, split
flows, and malformed or incomplete evidence. All normal practice customers are
negative across the three practice worlds and three loads.

The observer filters locally at S2 for exactly 299-byte TCP and retains one
record per exact flow per half second. The detector requires three distinct
sequence numbers in three time windows spanning at least one second. The exact
package passed 480 whole-network cases, 14 encoded-record cases, all nine
practice-truth/load checks, and all 158 repository tests. Maximum claim-log use
was 1,980 of 24,576 bytes.

This is a strong causal candidate, but unlike Echo and Traceroute the returned
log does not expose a flag proving that the packets came from a `side_flow` row.
The confidence is therefore moderate-high rather than absolute.

Upload only the two Wasm files under
`submission/next/claim-14-akamai-299-side-flow/`, together in one claim.
