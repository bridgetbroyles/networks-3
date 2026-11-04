# Submissions

Last confirmed result: **68 points**, seventeen completed attempts, **13 remaining**.
Eggs 7, 8, 10, 14, 16, 19 and 20 are already won.

| Folder | Purpose |
| --- | --- |
| next/ | Focused, tested Claim 18 pair; submit only the two Wasm files in its folder. |
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
is archived under `completed/`. Claim 10's active S6-S7 capacity detector won
no egg; its local mechanics were correct, but its causal hypothesis was not.
It is archived under `history/`.

Claim 11's source-specific Ledger silence rule won no egg and is archived under
`history/`. Claim 12's explicit packet-echo detector won Egg 8 and is archived
under `completed/`. Claim 13's Mapper TTL ladder won Egg 7 and is also archived
under `completed/`. Claim 14's 299-byte Akamai reverse-flow hypothesis won no
egg and is archived under `history/failed-claim-14-akamai-299-side-flow/`.

Claim 15's Login one-source/three-destination pair won no new egg and is
archived under `history/failed-claim-15-login-three-destination-fanout/`.
Claim 16's synchronized-camera rule also won no egg and is archived under
`history/failed-claim-16-camera-synchronized-rates/`. Claim 17's simultaneous
late activation of the two returned Sync sources also failed and is archived
under `history/failed-claim-17-sync-late-start/`. These results rule out the
current family of source-correlation guesses.

Claim 18 is prepared under
`next/claim-18-fastly-destination-fanout/`. It targets the independently
retained packet-level signature at S7: one sequence-zero 200-byte TCP source
reaches at least 50 destinations in one `/24` within five seconds. This is not
the rejected three-pattern bundle and does not depend on another anomaly being
present. See `docs/A3_CLAIM_18_PREPARED.md` for evidence and limits.
