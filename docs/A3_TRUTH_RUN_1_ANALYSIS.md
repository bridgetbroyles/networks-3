# A3 truth run 1 analysis

## What came back

The returned files are kept locally in `truthrun1/` and ignored by Git.
The log is 57,417 bytes, containing 3,076 complete records:

| Record | Count |
|---|---:|
| header | 1 |
| customer statistics | 94 |
| small packets | 97 |
| link silent/heard | 14 / 7 |
| link summaries | 206 |
| ordinary packet samples | 2,655 |
| port drops | 2 |

Decode it with customer names using:

```sh
scripts/decode-team-log.py truthrun1/team.log --customers truthrun1/customers.txt
```

## What the run taught us about the first observer

The run found two important limitations in our instrumentation.

First, the original observer installed and reported only the first 15
source prefixes. The returned `customers.txt` contains 54 ranges. This
means the first run measured only a minority of the customer set with
counters, even though packet sampling still exposed some other ranges.

Second, sampling one ordinary packet in 64 overloaded the logging
switch's 80-event/s controller. Of 2,655 ordinary samples, 2,637 were
observed at switch 7, mainly a high-rate Cloudflare-to-Cloudflare-users
flow. Keepalives and incoming reports then competed with those samples.
That explains all of these suspicious artifacts:

- switch 7 reported three local links silent at 1.0 s;
- several switches contributed only one or two batches of statistics;
- many link-summary periods never reached the logger;
- switch 7 accumulated reports showing zero keepalives on healthy links.

Those silent events are measurement overload, not reliable evidence that
the physical links failed. We must not build a claim detector from them.

## Useful evidence that survived

### Traffic relationships

The packet samples still reveal several strong relationships:

- Cloudflare sent high-volume 1000-byte traffic to Cloudflare users.
- Fastly sent 200-byte packets to at least 121 Home-user addresses over
  most of the run. This resembles a wide fanout, but not the practice
  scanner's five-second burst.
- Home users sent a short burst of 64-byte traffic back to Fastly around
  16.7-17.2 s.
- Login generated 100-byte traffic to three Login destinations around
  13.8-14.2 s.
- Mapper generated a short 64-byte burst around 30.6-31.0 s.
- Ping generated a short 64-byte burst around 3.0-3.3 s.
- Cloudflare users sent a short reverse burst around 32.3-32.8 s.
- Other observed named flows included Netflix, Netflix live, Steam,
  Vault, Amazon, Fortnite, Zoom, Akamai, and Offsite.

These are candidates, not proven egg definitions. Normal customer
behavior also appears in every truth run. We need to compare the same
named customers in runs 2 and 3 and look for the invariant unusual rule.

### Physical-network candidates

Most measured one-way keepalives took about 551 microseconds. Two
directions stood out sharply:

- switch 1 receiving from switch 2: about 30,051 microseconds;
- switch 14 receiving from switch 13: about 25,051 microseconds.

The direction matters: `S1->S2` in the decoder means the record was made
at S1 about its neighbor S2, so it measures traffic from S2 toward S1.
These fixed, much larger values are plausible slow-link eggs. Run 2 must
confirm them with complete time series before we choose thresholds.

Switch 8 receiving from switch 3 showed smaller delay variation, with
peaks around 1.5 ms. Switch 6 also reported 168 drops on one port. Either
could be a planted degradation or ordinary congestion; run 1 is too
incomplete to decide.

## Observer prepared for truth run 2

The new truth observer fixes the problems instead of spending another
attempt on the same blind spots:

- supports up to 63 shuffled customer ranges rather than 15;
- logs segmented headers, so all 54 current names can be matched;
- counts both source and destination bytes/packets in the data plane;
- reports truth source counters every 1.5 seconds and destination counters
  every two seconds;
- samples the first two ordinary packets plus one in 1,024, and every eighth
  small-packet sequence number;
- summarizes links every two seconds;
- routes one report copy toward central switch 7 on the published grid,
  rather than flooding duplicates into the logger;
- excludes locally silent links from report routing and varies equal-cost
  parents by report sequence.

On all three local truth worlds the final observer produced only
9,945-10,230 bytes while retaining complete link summaries, source and
destination rates, packet examples, small-packet details, and drops. A
15-switch 1.5x-load validation suite peaked at 49,525 of 65,536 bytes and
retained all switches and directed links. It also still passes every
disclosed practice claim at 10/10 in compact mode.

The exact run-1 Wasm is preserved at
`submission/truth-run1/switch.wasm`. The improved run-2 file is
`submission/truth/switch.wasm`.

## Next decision

Submit the improved truth observer for truth run 2. Do not spend truth
run 3 immediately. After run 2 returns:

1. verify that all customer ranges and all 44 directed grid links appear;
2. compare named-customer rates, packet sizes, start/stop times, fanout,
   and reverse traffic with run 1;
3. confirm the two large one-way link delays and identify any gaps,
   failures, recoveries, or drops;
4. form high-confidence egg hypotheses and decide whether run 3 should
   repeat this observer or target remaining ambiguity;
5. then build one narrow claim pair per hypothesis.
