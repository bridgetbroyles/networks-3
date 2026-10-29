# Assignment 3 implementation log

## Repository safety

- Confirmed the supplied A3 directory was not already a Git repository.
- Initialized `main` and committed the untouched starter kit as `841f3d2` before implementation.
- Did not modify the simulator, SDK, worlds, grading script, or supplied examples.

## Repository and handout investigation

Read the six-page handout, repository README, submission placeholder, logging API, limits, practice-world descriptions, all practice plants and answers, SDK types, TinyVM counter support, observation limiter, solved broken-link example, and detector ABI.

Key conclusions:

- Harness forwarding makes A1/A2 routing code unnecessary.
- High-volume measurements belong in counters because punts are capped at 80 events/s with 64 waiting.
- Every controller export is budgeted and rides behind customer traffic.
- One logger writes a byte-capped log; detectors see nothing else.
- Hidden eggs cannot be implemented before truth-run evidence exists.

## Implementation

Created:

- `a3_switch/`: one source with compact default mode and broad `truth` feature.
- `a3_detect_broken_link/`
- `a3_detect_long_pause/`
- `a3_detect_scanner/`
- `scripts/decode-team-log.py`
- stable compiled artifacts in `submission/`
- design, project, submission, implementation, and quiz documentation.

### Main choices

- Customer bytes and packets are counted in data-plane arrays.
- Keepalives teach switch-facing ports; customer traffic is counted only at source ingress.
- Compact statistics are reported every 500 ms. Truth mode reports source
  totals every 1.5 seconds and destination totals every four seconds so all
  54 observed customers fit with useful log headroom.
- Twenty missing keepalive periods trigger a silent record; recovery
  triggers a heard record. With 100 ms keepalives this is a two-second
  threshold, chosen to reject congestion-induced false failures.
- Compact claim mode records small packets exactly for scanner detection.
  Truth mode samples every eighth small-packet sequence number and suppresses
  repeat source/destination pairs for five seconds.
- General truth mode samples the first two ordinary packets plus one in 1,024.
  The final focused build instead omits packets of 800 bytes or more from punts
  while retaining their counter evidence. Both record link quality,
  destination activity, and drops.
- Reports use compact length-prefixed records and batching. Practice uses
  duplicate-suppressed flooding; the fixed hidden grid uses one
  decreasing-distance path toward switch 7.
- The local practice build logs at switch 0. The final graded build uses switch
  2, which is near the center of the published grid but avoids the observed
  switch-7 hotspot.

## Failures and fixes

### Rust build environment

The installed toolchain's `rust-lld` could not locate `libLLVM.dylib`. This was an environment path problem, not a program defect. Builds succeeded after supplying the toolchain library directory through `DYLD_LIBRARY_PATH`. The resulting WebAssembly modules load in the simulator.

### Truth link summaries were missing

The first truth build emitted packet samples but no periodic link summaries. The code tested whether timer time was exactly divisible by one second. Controller-action delay shifts recurring timer callbacks slightly, so this equality was never true. The fix stores the last summary time and emits when at least one second has elapsed. The final truth log contains 624 link-summary records in the practice truth run.

### Dense-topology action-size risk

Naively flooding an 800-byte report out of every port could exceed the 4,096-byte returned-action limit on a dense hidden switch. Flood fanout is now bounded from the payload's estimated encoded size. Sparse switches still use every neighbor. Only one new batch originates per timer, with excess records deferred.

### Customer identity

Customer prefixes, contract order, law, and demand cannot name Spotify in A3. The long-pause detector instead uses observed packet size, active rate, and duty cycle. Thresholds were validated across all ten reshuffled practice runs and all three load levels.

## Verification

### Unit and repository tests

- Switch unit tests: 4 passed.
- Broken-link detector: 1 passed.
- Long-pause detector: 2 passed.
- Scanner detector: 1 passed.
- Full root workspace: 114 simulator unit tests plus all SDK, detector, integration, sim-log, trace, Wasm, and doctest groups passed.

### Practice claims

The final compact switch and each matching detector were run through `scripts/practice-check.py`:

- Egg 21: 10/10, won.
- Egg 22: 10/10, won.
- Egg 23: 10/10, won.

This includes every disclosed positive, off, and near-miss run.

### Log sizes and measured evidence

- Compact switch with all three practice eggs: 5,864 bytes.
- Final revised truth switch with all three practice eggs: 9,945 bytes.
- Final practice truth checks: 9,945 bytes (`run-a`, 1.0x), 10,097 bytes
  (`run-b`, 1.25x), and 10,230 bytes (`run-c`, 1.5x), all well below
  65,536 bytes.
- Measured failure at 17.2 s: both 1-2 directions logged silent around 18.5 s.
- Measured Spotify source: median 1.68 Mb/s, 1000-byte packets, about 11.6 s missing between active windows.
- Measured scanner: 50 small packets to 50 distinct destinations inside five seconds.
- All three detectors returned true on the final truth log.

## Submission rules update

The published `SUBMITTING.md` resolved the earlier unknowns:

- Truth uploads are one file named `switch.wasm`, with a 65,536-byte log.
- Claim uploads are exactly `switch.wasm` and `detect.wasm`, with a
  24,576-byte log and 30 claims per team.
- The hidden topology is a fixed 3x5, 15-switch grid.

Added an `a3_switch` feature named `hidden`. It changes only the logger
from practice switch 0 to hidden-grid center switch 7. The stable graded
truth and claim observer artifacts are built with that feature. Keeping
the choice build-time avoids a nonexistent switch-7 logger on practice.
Both hidden artifacts were also loaded successfully by the simulator;
the practice claim observer still passed eggs 21, 22, and 23 at 10/10.

## Truth run 1 and observer revision

Truth run 1 returned 57,417 bytes and 3,076 records. Analysis found that
the original observer covered only 15 of the 54 returned customer ranges.
It also sampled 2,637 ordinary packets locally at switch 7, exhausting
the logger's 80-event/s controller budget and starving keepalives and
incoming reports. Several apparent silent links are therefore observer
artifacts, not trustworthy network events.

The exact run-1 observer is preserved in
`submission/truth-run1/switch.wasm`. The revised truth observer:

- supports 63 source and destination prefixes;
- uses four 64-slot data-plane counters, which keep timer input below the
  4,096-byte Wasm input buffer;
- uses segmented version-2 headers;
- samples the first two ordinary packets plus one in 1,024, and takes a
  bounded one-in-eight sample of short small-packet sequences;
- reports source totals every 1.5 seconds, destination totals and links every
  two seconds, and de-duplicates repeated small source/destination pairs;
- routes one report toward switch 7 instead of flooding duplicates.

An attempted 64x64 source-destination matrix was rejected during local
testing because two 4,096-slot arrays made each timer event exceed the
4,096-byte Wasm input buffer. It was replaced with separate 64-slot
source and destination counters. This preserves complete rate and packet
size data while sparse packet samples retain source-to-destination pairs.

The final observer produces 9,945, 10,097, and 10,230 bytes on the three
practice truth loads. Compact mode was rerun on all 30 disclosed claim cases
and still passes each practice egg at 10/10.

## Full-grid Truth Run 2 readiness validation

Added `scripts/a3-grid-validation.py`, which recreates the published
15-switch 3x5 topology with 54 shuffled sending customers. It checks that
records arrive from all switches, all 44 directed link views appear, and the
65,536-byte truth cap retains a safety margin.

The first full-grid test found two real defects before submission: a normal
run used 65,480 bytes, and sustained 64-byte traffic filled the log while
creating false silent-link events. The fixes were to stagger source and
destination counter snapshots, bound small-packet sequence sampling,
de-duplicate repeated small pairs, and use a two-second quiet threshold.

The exact final `submission/truth/switch.wasm` was tested at load 1.5:

- baseline: 46,160 bytes, all 15 switches, all 44 link directions;
- three overlapping logger-adjacent failures: 40,292 bytes, all six SILENT
  and all six HEARD transitions, all switches and directions retained;
- sustained small-packet stress: 47,214 bytes, no false link transitions;
- high-rate load stress: 49,238 bytes, no false link transitions;
- simultaneous delay, loss, capacity, pause, scan, and side-flow plants:
  49,525 bytes, with the planted delay, loss, pause, scan diversity, and
  side flow visible in the decoded evidence.

The worst measured case used 75.6% of the log cap, leaving 16,011 bytes.
All repository tests passed (114 simulator unit tests plus integration, SDK,
Wasm, detector, and doctest groups). The final truth artifact SHA-256 is
`9cb5ea50346bae90feb8db6eeab88b26ab268ccd1d812d02017970097fd019b0`.

## Remaining work that requires external evidence

- Spend truth attempts carefully and analyze the returned logs.
- Infer hidden eggs and their near misses.
- Build one compact switch/detector pair per claim hypothesis.

Those steps cannot be completed from the starter repository because the handout deliberately withholds the hidden eggs and returns no claim diagnostics.

## Truth Run 2 result and final observer

Truth Run 2 returned a valid 53,574-byte log, but only 91 directed-link
summaries instead of the roughly 836 expected. The byte cap was not the
problem. A Cloudflare-to-Cloudflare-users flow generated 2,337 one-in-1,024
samples at S7, about 62 punt events per second. With S7's four local
keepalive streams and incoming reports, that exceeded the 80-event/s
controller limit and starved remote telemetry.

The returned customer map exactly matches Truth Run 1. Useful repeated
evidence includes the 25,051-us S13-to-S14 direction, but the incomplete
network-wide series is not enough to finalize all hidden claims. Full details
are in `A3_TRUTH_RUN_2_ANALYSIS.md`.

The exact Run 2 Wasm was preserved in `submission/truth-run2/`. A new
`focused` feature keeps the first two packets and reduces ongoing ordinary
sampling from one in 1,024 to one in 8,192. Counters, link summaries, port
drops, report routing, and the two-second silence rule are unchanged.

The grid validator now requires all 836 periodic summaries in healthy
40-second cases and includes a high-rate customer local to S7. The final
candidate retained all 836 summaries, all 15 switches, all 44 directions,
complete source/destination counters, and no false link transitions. Its
1.5x high-rate log used 47,631 of 65,536 bytes. All four switch tests and the full
workspace suite (114 simulator tests plus all integration groups) pass.

Truth Run 3 candidate SHA-256:

```text
e76e922fcdaebb42e9279b99d98851ce094432dfeb828ce26645c8d7c4e2cbf1
```

## Final truth-run audit after Run 2

Run 2 established that S7, not the 64 KB byte cap, was the bottleneck: its
high-rate Cloudflare flow consumed most of the 80-event/s controller budget.
The final candidate therefore moves the truth logger to S2. It samples small
packets every eighth sequence, retains the first two identifying examples of
medium packets below 800 bytes, and does not punt larger packets. Data-plane
counters still measure every large packet.

An audit of the simulator revealed an important detail: planted scan packets
are described as unnumbered at the application level, but the observation copy
receives a sequence derived from the planted packet ID. An attempted
`seq == 0` sampler therefore retained only the first scan target. The fix uses
the existing bounded small-packet rule (every eighth sequence plus the first
two), which retained eight distinct targets under stress.

The overlapping-failure test then revealed that a one-time silence record can
lose the same route whose failure it reports. Silence records are now repeated
while the link remains silent. Both SILENT and HEARD transition chunks use
duplicate-suppressed flooding; routine reports still use one low-overhead path.
The corrected test retained all six failed directions and all six recoveries.

Two candidate changes were rejected after testing. Keeping the first 64 small
packets created a startup burst, seven false link transitions, and 259 false
sequence gaps in the healthy small-packet stress case. Moving the logger to the
corner switch S0 reduced local work but lost one recovery direction during an
overlapping-failure run. The final build therefore keeps bounded one-in-eight
small sampling and central logger S2.

The stress generator itself was also audited. Its intended hotspot now uses
actual 1,000-byte packets and a local peer, so the test stresses observation
rather than physically saturating a network link. Scanner traffic uses a
separate customer port on S2 so the extreme source cannot make the scan vanish
before it reaches the switch.

Final checks performed on the exact artifact:

- all `a3_switch` feature tests passed and the pipeline uses seven stages,
  three 64-entry counters, and no registers;
- all 158 workspace tests passed, plus all four `a3_switch` tests;
- the exact hidden Wasm installed and completed all three practice truth
  worlds;
- all nine full-grid scenarios passed at 1.5x load: baseline, overlapping
  failures, small stress, load stress, a logger-local hotspot, three lossy
  links, three permanent failures, scan multiplicity, and the simultaneous
  seven-family plant scenario;
- largest log: 47,618 / 65,536 bytes; healthy link summaries: 836 / 836;
- permanent failures retained 6/6 SILENT and 0 false HEARD records; recovering
  failures retained 6/6 SILENT and 6/6 HEARD records;
- the exact upload produced 5,623-byte logs in each official practice truth
  world;
- final SHA-256:

```text
cdd754ec8f588d9cf723c84e3313744f35108bb20c47dc962bb0661778574a67
```

## Final coverage correction: open-loop sampling and packet counts

The earlier focused candidate still used the packet sequence field to decide
whether to keep medium packets. The simulator represents an unnumbered planted
packet as sequence zero, so every medium open-loop packet passed the supposed
“first two” test. In local mixed tests the controller retained only two useful
medium examples after overload, which was both expensive and fragile.

The final sampler now hashes source address, destination address, packet size,
and protocol into a 4,096-entry data-plane register. Each signature retains its
first two packets. It then samples 128–799-byte traffic at 1/1,024 and traffic
of at least 800 bytes at 1/65,536. The sparse large-flow rule closes the endpoint
visibility gap without repeating the Run 2 hotspot failure. A dedicated
1,000-byte side-flow test retained exactly two identifying samples.

A fourth counter records destination packet totals. Source and destination
packet fields were widened from 16 to 32 bits so the known high-rate customer
cannot saturate a reporting window. The decoder remains backward-compatible
with the older 16-bit records from Truth Runs 1 and 2.

The pipeline now intentionally uses the full eight-stage limit, four 64-entry
counter arrays, and one 4,096-entry register array. All four switch unit tests
pass, including the 4,096-byte init/timer serialization checks.

The grid validator gained a dedicated capacity-bottleneck case and now checks
destination packet counts, sample sizes, drop totals, and exact evidence for
both 500-byte and 1,000-byte planted side flows. The complete ten-scenario
suite passed at 1.5× load against the final source:

- all 15 switches and all 44 directed link views were retained;
- healthy cases retained 836/836 periodic link summaries;
- overlapping recovered failures retained 6/6 SILENT and 6/6 HEARD
  directions;
- permanent failures retained 6/6 SILENT and zero false HEARD directions;
- the mixed case retained 50/50 scan targets, two 500-byte side-flow samples,
  67 loss gaps, and a 30,199-us delay peak;
- the capacity case retained two 1,000-byte side-flow samples and 10,818 port
  drops;
- the largest 1.5× log was 51,398/65,536 bytes;
- a separate 3× logger-hotspot run retained 836/836 summaries and 50/50 scan
  targets in 54,373 bytes.
- all 158 root-workspace tests and all four `a3_switch` tests passed;
- the exact hidden upload completed all three official practice truth worlds,
  producing a valid 6,889-byte log on each.

Final Truth Run 3 artifact:

```text
submission/truth/switch.wasm
SHA-256: deed7bef8ddb8b772620a11fbfea6c979510135acd415cc8edb789f2d60d506d
```

This section supersedes the older resource counts, sampling description, log
maximum, and candidate hash recorded above; the older entries are retained as
history of the tested iterations.
