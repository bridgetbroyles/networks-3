# Current Claim 16: synchronized S6 camera rates

Claim 15 has returned with no new egg. Upload this folder's `switch.wasm` and
`detect.wasm` together to **A3 - Claim**. Do not mix either file with a file
from another claim package.

The observer makes S6 the logging switch and records source counters every 1.5
seconds. The detector requires two 1,000-byte sources to match within one
packet across at least six aligned windows, traverse at least three positive
rate levels, and vary by at least 1.7x. This rejects constant twins and merely
similar flows.

Exact files:

- `switch.wasm`: 129,242 bytes; SHA-256
  `c340506a81708461b8d5a41963297b8dbb645edbee4a31d832e6d373d586ffd5`
- `detect.wasm`: 71,863 bytes; SHA-256
  `46fdc0faa66eb072bd0e0f88fef9850e4c3251084c4ab6f1090b6420fbee9ece`

Validation: 576/576 full-network cases, 14/14 exact-record checks, nine
practice-truth/load negatives, and 158/158 repository tests. Maximum claim log
use was 883 of 24,576 bytes. The exact detector also returns true on the
returned Truth Run 3 log.
