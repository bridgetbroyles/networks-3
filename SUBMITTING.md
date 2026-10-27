# Submitting

You submit everything on Canvas, through three assignments. One person
uploads for the whole team. A server picks up each upload within a few
minutes and replies in the submission's comments.

| Canvas assignment | What you upload | What comes back |
|---|---|---|
| **A3 — Truth run** | one file: your `switch.wasm` | `team.log` and `customers.txt` |
| **A3 — Claim** | two files: `switch.wasm` and `detect.wasm` | the eggs you have won, and your points |
| **A2 — Hidden practice** | one file: your A2 switch program | an A2 report card, and the score it would have earned |

## Before you upload

Build both programs for `wasm32-unknown-unknown` in release mode, as the
README shows. The files to upload are the `.wasm` files in
`target/wasm32-unknown-unknown/release/`. Try them on the practice
network first, with `scripts/practice-check.py`.

**Send one upload at a time, and wait for its reply before the next.**
The server only looks at your team's newest upload, so a second upload
sent while the first is still waiting takes its place.

## The two log caps

| Your log in a... | Handout name | Cap |
|---|---|---|
| **truth run** | *T* | **64 KB** (65,536 bytes) |
| **claim** | *S* | **24 KB** (24,576 bytes) |

A write that does not fit is lost completely, not cut short. The
practice network caps the log at 16 KB (16,384 bytes), so a log that
fits there will fit in a claim.

## Truth runs: 3 per team

Upload your switch program to **A3 — Truth run**. Every egg is present
in all three runs. Each run has its own seed, and each is busier than
the one before.

The reply attaches:

- `team.log`: what your logging switch wrote, up to 64 KB;
- `customers.txt`: which address range belongs to which named customer.

A run is used when you get a log back. An install failure does not use a
run. The team's first two installed runs that crash, loop, or time out
are free; from the third such error onward, each error uses a truth run
and returns whatever partial output is available.

Before each upload, run the switch on all three practice truth worlds:
`run-a.toml` at load 1.0, `run-b.toml` at 1.25, and `run-c.toml` at 1.5.

## Claims: 30 per team

Upload exactly two files to **A3 — Claim**: `switch.wasm` and
`detect.wasm`. The server tries the pair against every egg not yet won.
Each egg has ten fixed runs, and the detector must be correct on all ten.
A claim wins at most one egg, so use a separate, narrow detector for each
egg hypothesis. Claim logs are capped at 24 KB.

Wrong files, an install failure, or resubmitting the exact same pair do
not consume a claim. A claim running longer than one hour is stopped,
wins nothing, and does consume a claim. Wait for the reply before making
another upload.

## Points

| Level | Eggs | Points each |
|---|---|---|
| easy | 4 | 4 |
| medium | 9 | 8 |
| hard | 7 | 16 |

There are 200 possible points. 150 earns 100%; points above 150 count
toward A4 bonus levels.

## Hidden map used by every truth run and claim

```text
 0 ---- 1 ---- 2 ---- 3 ---- 4
 |      |      |      |      |
 5 ---- 6 ---- 7 ---- 8 ---- 9
 |      |      |      |      |
10 ---- 11 --- 12 --- 13 --- 14
```

Links: 0-1, 0-5, 1-2, 1-6, 2-3, 2-7, 3-4, 3-8, 4-9, 5-6, 5-10,
6-7, 6-11, 7-8, 7-12, 8-9, 8-13, 9-14, 10-11, 11-12, 12-13,
13-14.

## A2 hidden practice

Each team has ten ungraded A2 practice attempts. A3 jobs have priority,
so replies may be slow. This does not change the final A2 grade.

## Deadline

Truth runs and claims are accepted through the end of Tuesday,
November 3 (Central time). Questions go to Ritul at ritul@utexas.edu.
