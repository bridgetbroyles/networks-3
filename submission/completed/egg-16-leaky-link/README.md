# Completed Claim 7: Egg 16, The leaky link

## Server result

Claim 7 won **Egg 16 - Link - The leaky link**, worth 16 points. The
confirmed total after this result is 40/200, with 23 claims remaining. Do not
resubmit this pair.

These are the exact successful files. They are retained only for the audit
trail; the current next upload is under `submission/next/`.

Status: hidden win confirmed by the server.

## Changes that address measured failures

- Replaced approximately 25 controller-observed probes per second with 250 probes per second counted in the switch data plane. Probes end in Drop after updating counters and do not consume controller punts. Probe export uses approximately 10,000 bytes per second at each endpoint, below the 16,000-byte budget.
- Replaced a whole-run average with matching time windows in both directions. The old version missed the tested early, late, and ten-second 7% loss events.
- Uses cumulative counters, so a missing summary does not erase measurements. Requires sufficient samples and consistent sent/received sequence accounting.
- Requires a statistical margin above 2.5%, sustained loss across several epochs, and no large sequence gaps or target-port queue drops in the chosen interval. A rejected intermediate revision misclassified 2% loss in 4 of 12 seeds; that intermediate is not included as an upload pair.
- Keeps outage and congestion vetoes within the measured interval, allowing a clean loss interval to count when unrelated congestion happens later.

## Validation

All 137 network cases and 11 detector checks passed. See ../../../experiments/wire-loss/audit-results.json, ../../../experiments/wire-loss/boundary-results.json, ../../../experiments/wire-loss/combined-results.json, and ../../../experiments/wire-loss/record-results.json for exact outcomes and binary hashes. Tests use the provided simulator, unchanged SDK, public controller/export/log budgets, and synthetic worlds. They are not hidden-grader runs.

Coverage includes full, early, late and ten-second 5–7% loss; 0%, 1%, 2% and 2.5% controls; one-way and disjoint evidence; recovered outage; queue congestion and near-saturation without wire loss; brief severe loss; other-edge loss; malformed records; simultaneous background anomalies; and a 54-sender header.

A positive result requires enough sustained, bidirectional evidence. Events shorter than roughly eight seconds, weaker loss, a different edge, or a different hidden rule may be rejected. The 2.5% null is a design assumption, not a recovered grading rule. The three-sigma calculation is a detector filter, not a confidence guarantee about hidden grading.

## Submission discipline

Use this pair only for the next single attempt if proceeding. Keep the old queue on hold. Record the complete server result and exact folder after this attempt before any further submission; one failed claim does not reveal the reason or justify another unchanged attempt.

## Source and reproduction

Source, evidence and reproducible checks are in [experiments/wire-loss](../../../experiments/wire-loss/README.md). This folder is the only canonical upload copy. Its binary hashes are unchanged by repository cleanup.
