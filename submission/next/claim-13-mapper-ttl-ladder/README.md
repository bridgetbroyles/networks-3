# Claim 13: Mapper TTL ladder

Upload `switch.wasm` and `detect.wasm` together. Do not upload the manifest,
this README, or the folder itself.

This claim observes the 64-byte UDP Mapper source attached to S4. The simulator
decrements TTL before the observation pipeline, so an application ladder of
TTL 1 through 8 appears at the source switch as TTL 0 through 7. The detector
requires at least six distinct low TTL values for one exact source/destination
flow, including TTL 0 or 1.

The retained truth samples were sequences 152, 912, and 1064, all divisible by
eight and all observed at TTL 1. The previous every-eighth-sequence sampler
therefore aliased an eight-rung ladder onto one rung.

Exact files:

- `switch.wasm`: 134,006 bytes; SHA-256
  `5b6a76b461a3e0de9c4cb589f85639aea6d60dca8f19e8087f0ce66c1096ba99`
- `detect.wasm`: 55,271 bytes; SHA-256
  `b6f0f54e5cc3c80d86490620ae019d74c287cb7091f2bfee7f90ea3cc4a94f49`

Validation: 528/528 adversarial network runs, 13/13 exact-record checks,
9/9 normal practice-truth/load runs, and 158/158 repository tests passed. The
largest adversarial claim log was 7,435 of 24,576 bytes.
