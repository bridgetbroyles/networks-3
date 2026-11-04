# Failed Claim 18: sequence-zero destination fanout at S7

**Do not submit these files again.** The pair was submitted as Claim 18 and won
no new egg. The score remained 68/200.

The observer records only 200-byte TCP data arriving at S7 from a customer
port with sequence zero, once per exact source/destination pair. The detector
requires one exact source to reach at least 50 distinct addresses in a single
destination `/24` within five seconds.

Exact files:

```text
switch.wasm
  bytes: 134227
  SHA-256: cf09a35036abe004e8e7c71a9152090b14d34fb4ba022e0b1f78587e157812aa

detect.wasm
  bytes: 64301
  SHA-256: fbce6d0e67bf97e848988473e3bce6dc93996d58feb81a4b34ea1ce0e400a3b7
```

Local verification had passed 96/96 whole-network cases, 12/12 encoded-log
boundary cases, 114/114 simulator unit tests plus all other workspace groups,
a maximum claim log of 1,280 bytes, and `true` on the exact Truth Run 3 log.
The failed server result demonstrates that these checks established mechanics,
not that an unwon egg controlled the observed pattern.
