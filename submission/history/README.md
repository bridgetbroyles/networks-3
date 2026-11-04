# Submission history

Last reported server result: Claim 14 won no new egg. Confirmed total: 68
points, with 16 attempts remaining before pending Claim 15 is graded; Eggs 7,
8, 10, 14, 16, 19 and 20 are won.

Known failed submissions: old recovered-link-0-1 (Claim 3), old
recovered-link-10-11, and revised capacity-12-13 (Claim 6). The server gives
no cause for these failures. Claim 8's Home-user high-volume incast rule also
failed. The wire-loss v2 pair was submitted as Claim 7 and won Egg 16.

Claim 8 is important negative evidence: a large, repeatable truth-log pattern
is not enough. It may be ordinary traffic or an effect of an already-won egg,
both of which remain present when the grader toggles some other egg. Future
claims must identify a planted mechanism whose ON/OFF state can be isolated.

Claim 9's active physical-port probes confirmed that the simultaneous outage
and recovery around S5 was the rebooting-switch egg. The exact successful pair
is archived under `submission/completed/egg-20-rebooting-switch/`.

Claim 10 directly measured S6-S7 serialization capacity but won no egg. This
separates detector correctness from causal correctness: the observer could
have detected the proposed capacity transition, but the truth-run queueing
signature was not caused by that unwon mechanism. The pair is archived under
`submission/history/failed-claim-10-capacity-change-6-7/`.

Retired speculative queues, failed upload copies and duplicate SDK/build material were removed. retired-artifacts.json preserves their former paths and exact hashes, without carrying forward unsupported confidence ratings. A recovery backup of the previous submission tree and retired failure detector crates is stored outside this repository at ../networkarena-a3-retired-2026-11-03.tar.gz (relative to the repository root). The exact successful Claim 7 pair is archived in `submission/completed/egg-16-leaky-link/`.

Completed pair folders preserve artifacts associated with already won eggs.
Claim 11's S9 Ledger-pause pair is archived here because it won no egg. Its
mechanics passed local tests, but the pause was not proven to be an independently
toggleable unwon mechanism. Claim 12's direct Echo detector then won Egg 8 and
is archived under `submission/completed/egg-08-echo/`. Claim 13's Mapper ladder
won Egg 7 and is archived under `submission/completed/egg-07-traceroute/`.
Claim 14's 299-byte Akamai reverse-flow detector won no egg and is archived
under `submission/history/failed-claim-14-akamai-299-side-flow/`. Claim 15 is
pending, and the synchronized S6 camera-rate candidate is queued behind it as
Claim 16.
