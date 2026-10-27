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

- one header mapping internal customer numbers to the shuffled source prefixes;
- per-customer bytes and packets every 500 ms;
- small source packets with source, destination, size, switch, and time;
- link-silent and link-heard transitions.

Truth mode additionally records:

- one out of every 64 ordinary source packets;
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

The five-stage TinyVM program does not forward packets. The harness ignores our forwarding result and uses its own routes.

1. An exact table marks ports that have produced switch keepalives.
2. Packets arriving on unmarked, customer-facing ports are classified by source prefix.
3. The packet size is added to a byte counter indexed by customer.
4. One is added to a packet counter indexed by customer.
5. Keepalives, reports, and qualifying packet samples are punted.

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

The timer runs every 500 ms. It computes counter deltas, checks link silence, creates truth summaries when enabled, flushes at most one report batch, and schedules the next timer.

A link is called silent after ten keepalive periods. In the practice worlds, keepalives are 100 ms apart, so the nominal quiet threshold is one second. The visible log time also includes timer alignment, controller/action delay, queuing, propagation, and report forwarding. In the measured truth replay, a failure at 17.2 s produced both silent records at about 18.5 s.

## 5. Reporting design

Switch 0 requests the logger role on its first timer. This constant should be reconsidered after the hidden topology map is published: a central, well-connected switch is a better choice if switch 0 is peripheral.

Every non-logger report has:

```text
[origin u32][sequence u32][one or more framed records]
```

Reports are flooded over known switch ports. A switch accepts an `(origin, sequence)` only once, preventing loops and duplicate writes. The logger strips the report header and writes the framed records.

The fanout is bounded according to payload size so a dense switch does not exceed the 4,096-byte returned-action limit. Sparse topologies still use every neighbor. The batch is at most 800 bytes, and only one new batch originates per timer; excess truth records remain queued for a later timer.

Flooding costs more than a single routing tree, but it is robust when an egg breaks one possible report path. This is a reasonable correctness-first choice for the small networks and low summarized report rate.

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

cargo build --release --target wasm32-unknown-unknown --features truth
```

The truth build overwrites the same Cargo target name, so stable copies are kept in `submission/`:

- `a3_claim_switch.wasm`
- `a3_truth_switch.wasm`
- `detect_broken_link.wasm`
- `detect_long_pause.wasm`
- `detect_scanner.wasm`

## 9. Verified results

- Broken-link practice checker: 10/10.
- Long-pause practice checker: 10/10.
- Scanner practice checker: 10/10.
- Compact log with all three practice eggs: 5,864 bytes.
- Truth log with all three practice eggs: 41,655 bytes of the 65,536-byte practice cap.
- All three detectors return true on the practice truth log.
- `cargo test --workspace`: all simulator, SDK, integration, and document tests pass.
- Program-specific unit tests pass for the switch and all three detectors.

## 10. Known limits and next competitive steps

The current work is complete for the disclosed practice tasks, but hidden claims require evidence that the repository cannot provide:

1. Obtain the updated `SUBMITTING.md`, especially the hidden topology, true truth-log cap, claim-log cap, and upload procedure.
2. Choose the logging switch from the hidden map, then rebuild the truth switch if a switch other than 0 is better.
3. Submit the truth build and preserve the returned `.team` log and `customers.txt` from each attempt.
4. Decode each log and compare all three loads. Look for persistent link silence, unusual delay or gaps, customer pauses, rate shifts, many destinations, unexpected packet sizes, or drops.
5. Form one precise egg hypothesis at a time, including likely near misses.
6. Make a claim-specific switch that logs only the necessary tags and a detector that rejects those near misses.

The biggest current uncertainty is not an implementation bug; it is the intentionally hidden definition of the twenty graded eggs. The truth workflow is the mechanism the assignment provides to resolve it.
