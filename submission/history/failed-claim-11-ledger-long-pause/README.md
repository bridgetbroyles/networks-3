# Claim 11: Ledger-like source at S9 pauses and resumes

Upload exactly `switch.wasm` and `detect.wasm` from this directory together to
**A3 - Claim**. Do not ZIP them, rename them, or mix in files from another
folder.

## Evidence and hypothesis

In Truth Run 3, the 1,000-byte Ledger source at S9 sent a stable 209-211
packets per retained window through 12.5 seconds, disappeared during the
middle of the run, and resumed at exactly 210 packets by 38.1 seconds. S9
still delivered healthy link summaries at 24.0 and 30.1 seconds during the
gap. Truth Run 2 likewise delivered healthy S9 link summaries at 18.0 seconds
without a Ledger source record. This makes the silence source-specific rather
than a silent switch or failed physical link.

The claim hypothesis is that this Ledger-like source pauses for a long
interval and later resumes.

## Exact rule

S9 is both the observation site and logging switch, so no network report can
be mistaken for customer silence. Every 1.5 seconds it records a heartbeat and
the source counters, including explicit zero deltas.

The detector identifies S9's steady 1,000-byte source by its normal per-window
volume. It requires:

- positive traffic before the gap;
- at least 11 observed seconds of consecutive, heartbeat-backed zero windows;
- positive traffic afterward; and
- the recovered source to retain the expected packet-size and rate profile.

Missing records are not zeros, and permanent silence does not count because
recovery is required.

## Verification

- 504/504 whole-network cases passed across 12 seeds and loads 1.0, 1.25,
  and 1.5.
- True cases used 14-, 20-, and 24-second pauses at different timer phases.
- Negative cases included no pause; 2-, 6-, 10-, and 12-second pauses;
  permanent silence; natural 8-second on/off traffic; and a long pause on the
  wrong switch, wrong rate, or wrong packet size.
- Representative delay, loss, SRLG, reboot, and scan/traffic mechanisms were
  active together in the audit background.
- A separate phase sweep rejected every tested pause through 12 seconds and
  accepted every 14- and 20-second pause.
- 13/13 encoded-record, missing-data, malformed, and truncated-log checks
  passed.
- All nine official practice-truth/load runs completed.
- All 158 repository tests passed.
- Largest adversarial claim log: 1,621/24,576 bytes.
- A clean rebuild matches the packaged files byte-for-byte.

The truth evidence for this claim is materially stronger than Claim 10: it
shows a source-specific stop and recovery while the same switch remained
observable. The hidden claim worlds remain unavailable, so this is a
well-supported claim rather than a guarantee.

## Exact files

```text
switch.wasm
  bytes: 129226
  SHA-256: 72d4a8295fee435fe13d07b510fd02371db70f84274229cf17db54d2cb5446a1

detect.wasm
  bytes: 62858
  SHA-256: 80f797f52ccddfebf614eb477fd4a6a0a6324cda74beaf7ab8d1f88e36213a9b
```

Wait for the server reply before preparing another claim.
