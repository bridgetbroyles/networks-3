# A3 Claim 24: Netflix side channel (Egg 1)

## Upload exactly these two files

```text
submission/claims/claim-24-netflix-side-channel/switch.wasm
submission/claims/claim-24-netflix-side-channel/detect.wasm
```

```text
switch.wasm  112,840 bytes  SHA-256 52125605711b75fdde56ce043768b7aa931e6006968f7d76f6721ea59dbe9ec3
detect.wasm   43,440 bytes  SHA-256 49b5e9622423174880bdafbcad94a6548dc90dae808b070d1629cd9e447bf729
```

Sources: `a3_side_switch/` and `a3_detect_side/`.

## Evidence

Every bulk customer in runs B and C averages exactly 1000.0 bytes a packet
(Steam, Amazon, Backup, Dropbox, Cam-A/B, Ledger, Offsite, Sync, Netflix
live) — except Netflix's /24: 988.4 (run B) and 988.0 (run C).

Run C, every 1.5 s window: 748 packets / 739,200 bytes, or 749 / 739,400.
The only split that keeps Netflix's stream constant is 737 × 1000 B (3.93
Mb/s, Netflix's measured rate) plus 11 or 12 × 200 B; the extra packet
in the 749 windows is exactly one more 200-byte packet. Run B: 614 + 9 per
window, both streams scaled by exactly the load ratio (1.25 vs 1.5). Present
in every window from 3.5 s to 38.1 s. Another team's card confirms Egg 1
"Customer · Netflix side channel".

## Rule

- Identity: Netflix is the /24 holding a /32 of its own whose packets
  average ≥ 900 bytes (Netflix live). Every truth-run header lists the
  nested /32s; Amazon's and Fortnite's /32s send 200-byte packets.
- Side channel: in ≥ 70% of the seconds Netflix sends anything (from 2 s),
  it also sends packets under 990 bytes; ≥ 40 in all; median ≥ 2 a second.
  Run C: 7–8 a second in every window.

## Switch

Only S3 (Netflix's switch, the logger) does anything: data-plane counters of
bytes, packets and packets under 990 bytes per source range, written once a
second for the nested ranges only, plus all ranges once at start. No punts,
no reports. Log ≈ 2.2 KB.

## Validation

The local simulator rejects nested ranges, so tests ran on a scratch copy
patched only to allow a /32 inside a /24 (the harness already routes by
longest prefix). Crowded replica with Netflix (/24) + Netflix live (/32) at
S3, Amazon + Prime /32, Fortnite + player /32, Steam, Backup, Vault, Home
users / Fastly. Loads 1.0 / 1.25 / 1.5: **36 of 36 right**.

| Scenario | Expected |
|---|---|
| Netflix side flow 200 B 5/s; to another customer; 600 B; with other eggs' faults, flood and breathing link | true |
| off; off + other eggs | false |
| side flow on Netflix live's /32; on Amazon shoppers; on Steam | false |
| Netflix side flow for 5 s only; a trickle (0.3/s); extra 1000-byte packets | false |

On the unmodified simulator the switch completes all practice truth worlds
at all loads without failure; the detector said no on all 30 practice runs.

## Remaining risks

- If the egg's near miss is defined by where the side flow goes (its
  destination was never visible in the truth logs), this cannot tell.
- If Netflix is not on S3 in claim runs, nothing is counted and it says no.
