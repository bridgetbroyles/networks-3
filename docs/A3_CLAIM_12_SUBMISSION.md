# A3 Claim 12 submission

Claim 12 targets the simulator's explicit customer `echo` behavior. The
500-byte TCP source at S3 is the Vault-like customer. In a retained truth run,
that flow produced repeated source/destination/size/sequence tuples. The
simulator source shows that `echo = true` emits two copies of every numbered
packet immediately, with the same sequence and a new packet ID.

The observer samples the source-facing ingress at S3 before network loss or
routing can create copies. It filters to 500-byte TCP, ignores sequence zero,
and samples sequences divisible by 256. The detector requires two distinct
sequence numbers from the same exact flow, with each pair separated by no more
than 750 microseconds. Ordinary retransmissions, incomplete evidence, wrong
sites, wrong sizes, wrong protocols, and unrelated flows are false.

The exact packaged pair passed 480 adversarial whole-network cases, 13 encoded
record tests, all nine practice-truth/load negative runs, and all 158 repository
tests. The largest adversarial log was 8,210 of 24,576 bytes.

Upload only the two Wasm files under
`submission/next/claim-12-vault-echo/`, together in one claim submission.
