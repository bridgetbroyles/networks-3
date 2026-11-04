# A3 Claim 11 submission

> Historical only: the server returned no new egg. Do not resubmit this pair.

Claim 11 tests a long pause and recovery of the Ledger-like 1,000-byte source
at S9. Truth Run 3 recorded stable traffic through 12.5 seconds and the same
stable rate again at 38.1 seconds, while S9 continued supplying healthy link
records during the intervening source silence. Truth Run 2 also retained an S9
link report during the Ledger gap.

The claim logs locally at S9 and writes explicit zero-counter windows with
heartbeats. Its detector requires positive traffic, at least 11 observed
seconds of explicit silence, and positive traffic afterward. It rejects
missing telemetry and permanent silence.

Verification passed 504 whole-network cases, 13 record cases, a timer-phase
sweep, all nine practice-truth/load runs, and all 158 repository tests. The
largest claim log was 1,621 of 24,576 bytes.

The exact upload pair is under
`submission/next/claim-11-ledger-long-pause/`. Submit only its `switch.wasm`
and `detect.wasm` together, then wait for the complete result.
