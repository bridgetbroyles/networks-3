# Claim 14: Akamai-users 299-byte reverse side flow

Upload `switch.wasm` and `detect.wasm` together. Do not upload the manifest,
this README, or the folder itself.

This claim observes the normally receiving Akamai-users customer at S2 sending
a reverse TCP stream of exactly 299-byte packets. It retains at most one sample
per exact flow every half second. The detector requires three distinct sequence
numbers in three observation windows spanning at least one second.

Exact files:

- `switch.wasm`: 133,856 bytes; SHA-256
  `049831c981290ea48916f7d117492fd6e38466ea07c7dd67e9a1f764cadc1a5c`
- `detect.wasm`: 67,191 bytes; SHA-256
  `c8dd4d29acc51d72201f21c077ac101d72125dccb84a5f3e1096e84aa64a53ca`

Validation: 480/480 adversarial network runs, 14/14 exact-record checks,
9/9 normal practice-truth/load runs, and 158/158 repository tests passed. The
largest adversarial claim log was 1,980 of 24,576 bytes.
