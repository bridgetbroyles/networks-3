# Claim 17: synchronized late starts at S1

Upload `switch.wasm` and `detect.wasm` from this directory together to
**A3 - Claim**. Do not upload the directory, manifest, README, or a ZIP.

The detector asks whether two low-rate 1,000-byte sources at S1 first become
active in the same 1.5-second observation window with similar initial packet
counts. This matches the two returned customers named Sync in Truth Run 3: both
first appear at 33.6 seconds with 61 and 55 packets, after an S1 report at 30.6
seconds showed the unrelated high-rate source but neither Sync source.

Checks completed on these exact Wasm files:

- 360/360 whole-network ON, OFF, and near-miss simulations;
- 13/13 encoded-record and malformed-input checks;
- true on the decisive Truth Run 3 log;
- false on all nine official practice truth/load combinations;
- 158/158 repository tests; and
- maximum test log 1,252 of 24,576 bytes.

The whole-network negatives include one source, staggered starts, adjacent
observation windows, unequal initial rates, two high-rate sources, startup
sources, the wrong switch, and the wrong packet size. Other known delay, loss,
failure, and scanner mechanisms remain active throughout the audit.

Local testing proves the submitted programs implement this rule reliably. The
server still provides the only causal test of whether this exact Sync behavior
is an unwon egg.
