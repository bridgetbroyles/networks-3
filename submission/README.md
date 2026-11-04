# Submissions

Last confirmed result: **68 points**, eighteen completed attempts, **12 remaining**.
Eggs 7, 8, 10, 14, 16, 19 and 20 are already won.

| Folder | Purpose |
| --- | --- |
| next/ | No Claim 19 pair is ready. Do not submit from this directory. |
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

Claim 18's S7 sequence-zero destination-fanout detector won no new egg and is
archived under `history/failed-claim-18-fastly-destination-fanout/`. Its
96 network cases and 12 record tests proved only that the proposed predicate
was measured correctly; the server result shows that predicate was not the
ON/OFF rule for an unwon egg. No Claim 19 pair is ready.
