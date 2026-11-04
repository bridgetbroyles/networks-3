# S3 Vault echo experiment

This experiment tests whether the 500-byte TCP source at S3 sends each
numbered packet twice. The data plane samples only nonzero sequence numbers
divisible by 256, keeping multiple echo pairs while sharply reducing controller
events from unrelated high-rate 500-byte traffic.
The detector requires two distinct sequence numbers whose two copies
arrive no more than 750 microseconds apart.

Immediate same-sequence copies are different from normal TCP retransmissions,
which arrive much later. The rule also fixes the source switch, packet size,
protocol, and flow identity.
