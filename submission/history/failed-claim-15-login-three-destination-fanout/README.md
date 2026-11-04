# Failed Claim 15: Login repeated three-destination fanout

Server result: **no new egg won**. Do not submit this pair again. The observed
fanout was real, but the result proves that this rule did not isolate an unwon
egg under the grader's ON/OFF comparison.

This claim observes one 100-byte TCP Login source at S8 repeatedly reaching
three destinations. Truth Run 1 captured 11 packets from the same source to
three destinations in 0.4 seconds. Truth Run 3 captured all three destinations
near 5 seconds and all three again near 10 seconds. The detector requires three
destinations that each occur twice for one source within six seconds.

Exact files:

- `switch.wasm`: 133,868 bytes; SHA-256
  `c8111e5eb16df02e6ba286a2748481bb0026972e21eea849008fad151baba0ad`
- `detect.wasm`: 55,183 bytes; SHA-256
  `da1dbaeb3bd1708f5a5b1e05e1c058f38a306af93dc2120b462f9f681bcf882d`

Validation: 528/528 adversarial 15-switch runs, 12/12 exact-record checks,
9/9 six-switch practice runs returning false, and 158/158 repository tests.
The largest adversarial claim log was 2,870 of 24,576 bytes.
