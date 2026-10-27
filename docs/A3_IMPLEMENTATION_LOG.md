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
- Compact statistics are reported every 500 ms; truth source and
  destination statistics are reported every second.
- Ten missing keepalive periods trigger a silent record; recovery triggers a heard record.
- Small data packets are recorded exactly for scanner detection.
- Truth mode samples the first two packets plus one in 1,024 and records
  link quality, destination activity, and drops.
- Reports use compact length-prefixed records and batching. Practice uses
  duplicate-suppressed flooding; the fixed hidden grid uses one
  decreasing-distance path toward switch 7.
- The local practice build logs at switch 0. Graded builds use switch 7,
  the center of the published 3x5 hidden grid.

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

- Switch unit tests: 2 passed.
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
- Truth switch with all three practice eggs: 41,655 bytes, below the 65,536-byte practice truth cap.
- Required truth-world checks after publication: 41,655 bytes (`run-a`,
  1.0x), 43,726 bytes (`run-b`, 1.25x), and 45,797 bytes (`run-c`,
  1.5x), all below 65,536 bytes.
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
- samples first packets plus one in 1,024;
- reports customer totals every second and links every two seconds;
- routes one report toward switch 7 instead of flooding duplicates.

An attempted 64x64 source-destination matrix was rejected during local
testing because two 4,096-slot arrays made each timer event exceed the
4,096-byte Wasm input buffer. It was replaced with separate 64-slot
source and destination counters. This preserves complete rate and packet
size data while sparse packet samples retain source-to-destination pairs.

The revised observer produces 11,961, 12,113, and 12,246 bytes on the
three practice truth loads. Compact mode was rerun on all 30 disclosed
claim cases and still passes each practice egg at 10/10.

## Remaining work that requires external evidence

- Spend truth attempts carefully and analyze the returned logs.
- Infer hidden eggs and their near misses.
- Build one compact switch/detector pair per claim hypothesis.

Those steps cannot be completed from the starter repository because the handout deliberately withholds the hidden eggs and returns no claim diagnostics.
