# Claim 12: Vault immediate duplicate packets

Upload `switch.wasm` and `detect.wasm` together. Do not upload the manifest,
this README, or the folder itself.

This claim observes the 500-byte TCP source attached to S3. It samples only
nonzero sequence numbers divisible by 256 and returns true only when two
distinct sequences each arrive twice for the same flow within 750 microseconds.
That matches the simulator's explicit `echo` behavior: the same numbered packet
is emitted twice immediately with a new packet ID. Normal retransmissions occur
later and are rejected.

Exact files:

- `switch.wasm`: 129,954 bytes; SHA-256
  `98ffd2dfbd1adb5eb516b2558d0b1f92b132acaf318ea986e2520cf82b0a5793`
- `detect.wasm`: 60,259 bytes; SHA-256
  `4d66d05f9102e008765d89755e413141ec679430f379b60502e5ee139525a972`

Validation: 480/480 adversarial network runs, 13/13 exact-record checks,
9/9 practice-truth/load negative runs, and 158/158 repository tests passed.
The largest adversarial claim log was 8,210 of 24,576 bytes.
