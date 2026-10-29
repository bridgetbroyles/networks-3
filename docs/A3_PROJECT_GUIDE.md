# Assignment 3 project guide

## 1. What A3 asks us to build

A3 has two programs with different jobs:

- `switch.wasm` runs on every switch. The harness handles forwarding, so this program only measures, reports, and writes a team log.
- `detect.wasm` runs once after the simulation. It receives only the team-log bytes and answers `true` or `false` for one egg.

There are two kinds of server attempt. A truth run accepts a switch program and returns its larger team log plus customer-range names. A claim accepts a switch program and one detector and tests the pair on ten positive, negative, and near-miss runs. The hidden eggs cannot be completed honestly before the truth-run evidence exists.

## 2. What was implemented

### Switch program

`a3_switch/` contains one observer with two build profiles:

- Default: compact claim mode.
- `--features truth`: broad truth-run mode.

Both modes record:

- segmented headers mapping up to 63 internal customer numbers to shuffled prefixes;
- per-customer bytes and packets (500 ms in compact claim mode; 1.5-second
  source and four-second destination windows in truth mode);
- small source packets with source, destination, size, switch, and time;
- link-silent and link-heard transitions.

Truth mode additionally records:

- per-customer destination byte and packet totals;
- the first two ordinary packets of a hashed flow signature, then one out of
  every 1,024 medium packets or 65,536 large packets;
- every eighth sequence number for small packets, with repeated
  source/destination pairs logged at most once per five seconds (open-loop
  scans expose sequence zero, so the pair rule retains one record per target);
- per-direction keepalive count, sequence gaps, mean delay, and peak delay;
- port drop totals.

### Detectors

- `a3_detect_broken_link/`: detects the disclosed practice egg in which both ends of link 1-2 become silent and do not recover.
- `a3_detect_long_pause/`: identifies Spotify from its traffic behavior and detects its long pause.
- `a3_detect_scanner/`: detects at least 50 distinct destinations reached by one source inside five seconds.

These are practice detectors. They demonstrate the claim workflow but do not earn hidden-egg points.

### Analysis helper

`scripts/decode-team-log.py` turns the binary team log into a readable summary of:

- record counts and log size;
- shuffled customer prefixes;
- median active rate, average packet size, and longest gap per customer;
- link state changes;
- truth-mode link quality;
- small-packet destination diversity;
- port drops.

Run it with:

```sh
scripts/decode-team-log.py run-a.team
```

## 3. Data-plane observation pipeline

The eight-stage TinyVM program does not forward packets. The harness ignores our forwarding result and uses its own routes.

1. An exact table marks ports that have produced switch keepalives.
2. Packets arriving on unmarked, customer-facing ports are classified by source prefix.
3. Its destination prefix is classified separately.
4. Source bytes are counted.
5. Source packets are counted.
6. Destination bytes are counted.
7. Destination packets are counted.
8. Keepalives, reports, and qualifying packet samples are punted.

This division is deliberate. Counters observe all customer packets without spending controller events. Punts are used only when the controller needs details that a counter cannot preserve, such as a packet’s source and destination.

The program initially does not know which ports face switches. The first keepalive on a port teaches that role and installs the exact-table entry. Statistics before one second are ignored so the short learning period does not create duplicate transit counts.

## 4. Controller state and timing

Each switch keeps:

- known switch-facing ports and neighbor IDs;
- last keepalive arrival and sequence per link;
- received-count, sequence-gap, and delay accumulators;
- the last byte and packet counter snapshots;
- pending log records;
- report sequence numbers and duplicate-suppression state.

The timer runs every 500 ms. Compact builds report source deltas on that
cadence. Truth builds report source deltas every 1.5 seconds, destination
byte deltas every four seconds, and link summaries every two seconds; check link silence; flush at
most one report batch, and schedule the next timer.

A link is called silent after twenty keepalive periods. In the practice
worlds, keepalives are 100 ms apart, so the nominal quiet threshold is two
seconds. This longer threshold was selected after a 1.5x-load grid stress
test showed that a one-second threshold could briefly mistake controller
contention for a failed link. Link summaries still expose shorter keepalive
gaps.

## 5. Reporting design

The practice build requests the logger role on switch 0. The final hidden
build uses switch 2. It remains near the center of the 3x5 grid but, unlike
switch 7, was not the attachment point of the high-rate customer that
overloaded the first two truth observers.

Every non-logger report has:

```text
[origin u32][sequence u32][one or more framed records]
```

Practice builds flood reports over known switch ports and suppress duplicates by `(origin, sequence)`. Hidden-grid builds normally send one copy along a decreasing grid-distance path toward switch 2. They exclude locally silent links and alternate equal-cost parents using the report sequence. Reports containing a silent-link record are the exception: that state is repeated while silent and duplicate-suppressed flooded, because the failed link may also have broken the ordinary report path.

The fanout is bounded according to payload size so a dense switch does not exceed the 4,096-byte returned-action limit. Sparse topologies still use every neighbor. The batch is at most 800 bytes, and only one new batch originates per timer; excess truth records remain queued for a later timer.

Routine single-copy routing leaves room in the logger's 80-event/s budget for keepalives and observations. Selective redundancy gives rare failure evidence alternate paths without imposing flood overhead on normal counter and link-summary traffic.

## 6. Log format

Every stored record is self-delimiting:

```text
[tag u8][body_length u8][body bytes]
```

| Tag | Record | Purpose |
|---:|---|---|
| 1 | Header | Format version and customer source prefixes |
| 2 | Customer statistic | Switch, customer, time, byte delta, packet delta |
| 3 | Small packet | Switch, time, source, destination, size |
| 4 | Link silent | Switch, neighbor, port, time |
| 5 | Link heard | Recovery of a previously silent port |
| 6 | Link summary | Truth-only keepalive count, gaps, mean and max delay |
| 7 | Packet sample | Truth-only sampled source/destination/size/sequence |
| 8 | Port drops | Truth-only switch, time, port, drop count |
| 9 | Destination statistic | Truth-only destination bytes and packets |

Length-prefixing lets a detector skip records it does not understand and lets later claim-specific builds reuse the format while deleting unrelated tags.

## 7. Practice detector logic

### Broken link

The detector tracks the last silent/heard state for every directed neighbor pair. It returns true only when both `(1,2)` and `(2,1)` end silent. A recovered link ends heard, and another failed link does not match.

### Long pause

The detector reconstructs each source’s 500 ms activity series. It identifies the Spotify-like source using:

- average packet size at least 800 bytes;
- median active rate between 0.7 and 4 Mb/s;
- activity outside its largest gap above 82%.

These conditions reject Fortnite and Jane Street’s 200-byte traffic, Steam’s much higher unlimited rate, and Amazon’s repeated off periods. It then requires a timer step consistent with at least a ten-second pause. The bounds were checked across every practice load and near miss.

### Scanner

The detector groups small packets by source, sorts them by observation time, and slides a five-second window. It returns true when one window contains 50 distinct destinations. Repeated packets to three destinations and 50 destinations spread over 36 seconds both return false.

## 8. Builds and stable outputs

Normal commands are:

```sh
cd a3_switch
cargo build --release --target wasm32-unknown-unknown

cargo build --release --target wasm32-unknown-unknown --features hidden

cargo build --release --target wasm32-unknown-unknown --features "hidden truth"

# Final truth-run build after measured S7 controller overload
cargo build --release --target wasm32-unknown-unknown --features "hidden truth focused"
```

Each feature combination overwrites the same Cargo target name, so stable
copies are kept in `submission/`:

- `a3_practice_switch.wasm` (logger 0; local validation)
- `a3_practice_truth_switch.wasm` (logger 0; local truth validation)
- `claim/switch.wasm` (preserved claim observer template; rebuild from current
  source before a new claim)
- `truth/switch.wasm` (logger 2; final truth observer)
- `detect_broken_link.wasm`
- `detect_long_pause.wasm`
- `detect_scanner.wasm`

## 9. Verified results

- Broken-link practice checker: 10/10.
- Long-pause practice checker: 10/10.
- Scanner practice checker: 10/10.
- Revised practice truth logs are 9,945 bytes at 1.0x, 10,097 bytes at
  1.25x, and 10,230 bytes at 1.5x.
- A local replica of the published 15-switch grid passed eleven scenarios at
  1.5x load: baseline, overlapping and permanent failures, sustained
  small-packet traffic, high-rate traffic, a logger-local hotspot, simultaneous
  lossy links, scan multiplicity, mixed plants, and a dedicated capacity
  bottleneck, and the exact packet-sampling boundaries. Every case retained records
  from all 15 switches and all 44 directed link views. The largest
  final-candidate log was 51,398 of 65,536 bytes.
- Compact mode still passes all three practice detectors at 10/10.
- `cargo test --workspace`: all simulator, SDK, integration, and document tests pass.
- Program-specific unit tests pass for the switch and all three detectors.

## 10. Known limits and next competitive steps

Truth Run 2 showed that staying under the byte-log cap is not sufficient:
one-in-1,024 sampling of the high-rate Cloudflare flow consumed most of S7's
80-event/s controller budget. The final focused truth build keeps the first two
packets of each hashed flow signature, samples medium flows at 1/1,024 and
large flows at only 1/65,536, and measures every packet in counters. See
`A3_TRUTH_RUN_2_ANALYSIS.md` for the measured evidence.

The current work is complete for the disclosed practice tasks. The
published submission rules fix the hidden network as a 15-switch grid,
set the truth cap to 65,536 bytes and claim cap to 24,576 bytes, and give
three truth runs and 30 claims. Hidden egg definitions still require
truth-run evidence:

1. Submit `submission/truth/switch.wasm` and preserve the returned
   `team.log` and `customers.txt` from each attempt.
2. Decode each log and compare all three loads. Look for persistent link
   silence, unusual delay or gaps, customer pauses, rate shifts, many
   destinations, unexpected packet sizes, or drops.
3. Form one precise egg hypothesis at a time, including likely near misses.
4. Make a claim-specific switch that logs only the necessary tags and a
   detector that rejects those near misses.
5. Upload exactly `switch.wasm` and `detect.wasm`, one claim at a time.

The remaining uncertainty is the intentionally hidden definition and
near-miss boundary of each graded egg. Local testing can establish that the
evidence arrives intact, but only returned truth logs can reveal which exact
claim detectors to build.
