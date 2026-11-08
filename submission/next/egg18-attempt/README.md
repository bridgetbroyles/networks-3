# Claim 24 candidate: Egg 18, The flapping link

Upload only these two files to the claim form:

- `switch.wasm` as the switch program;
- `detect.wasm` as the detector.

## What this claim detects

The observer watches physical link 1-2 from both endpoints. It sends direct
probes and records both probe arrivals and ordinary physical-port ingress.
The detector returns true only after the same link completes at least two
separate down-and-recovered cycles in both directions.

Each accepted cycle must contain working probes before the outage, zero probe
and zero physical-ingress observations during the outage, a matching
keepalive-silence event, and later probe recovery. One recovered outage is
false; a permanent outage is false.

## Evidence

Truth Run 3 repeatedly recorded S2 losing and regaining contact with S1. The
simulator permits multiple `fail` plants on one edge, which is the direct
mechanism that creates a flapping link. This candidate is intentionally
separate from the already-won SRLG and rebooting-switch claims, which each
describe one coordinated outage affecting multiple links.

## Validation

- 10/10 encoded detector and malformed-log cases passed.
- 396/396 full-network cases passed across 12 seeds and loads 1.0, 1.25, and
  1.5.
- The matrix includes two- and three-flap positives.
- Negative cases include no target failure, one recovered outage, a permanent
  outage, one long outage, outages too short to qualify, repeated failure of
  another link, failures split across two links, keepalive gaps with a working
  physical link, and heavy congestion.
- Every world also retains the known one-way delay, symmetric delay, leaky
  link, changing-capacity link, SRLG, and rebooting-switch mechanisms.
- Maximum claim log: 4,486/24,576 bytes.

Local validation proves that the implementation matches this hypothesis; only
the private grader can confirm the hidden Egg 18 boundary.
