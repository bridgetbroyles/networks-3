# Submissions

Last confirmed result: **56 points**, nine attempts used, **21 remaining**.
Eggs 10, 14, 16, 19 and 20 are already won.

| Folder | Purpose |
| --- | --- |
| [next/claim-10-capacity-change-6-7](next/claim-10-capacity-change-6-7/README.md) | Current audited Claim 10 pair. Upload its two Wasm files together only. |
| completed/ | Already submitted pairs associated with won eggs; do not resubmit. |
| truth/ | Current truth-run observer. |
| evidence/ | Exact earlier truth-run observers for reproducibility. |
| practice/ | Disclosed local practice artifacts. |
| templates/observer/ | Observer template; not a complete claim pair. |
| [history](history/README.md) | Failed-claim history, current status and retired artifact hashes. |

Claim 7's active-probe pair won Egg 16 and is archived under `completed/`.
Claim 8's Home-user incast pair won no new egg and is archived under
`history/failed-claim-08-home-users-incast/`. Its local tests established only
that the guessed rule was implemented; they did not prove that the traffic was
caused by an unwon egg. No next claim is ready until it passes causal,
ON/OFF, near-miss, simultaneous-egg, log-cap, and exact-artifact checks.

Claim 9's S5 physical-interruption pair won Egg 20, the rebooting switch, and
is archived under `completed/`. Claim 10 actively measures serialization on
S6-S7 to distinguish a capacity transition from delay, wire loss, queueing,
and an unrelated capacity change. It passed 432 whole-network cases, 10 record
checks, and all nine practice-truth/load runs.
