# A3 Claim 10 submission

> Historical record: this claim was submitted and won no new egg. Its files
> are now archived under
> `submission/history/failed-claim-10-capacity-change-6-7/`. Do not submit it
> again. See `A3_CLAIM_10_RESULT.md` for the correction.

Claim 10 tests a sustained capacity transition on physical link S6-S7. It uses
active adjacent-packet serialization measurements rather than customer drops
or queue delay, both of which can be caused by ordinary congestion.

The detector requires repeated samples in both a normal regime (at least
8 Mb/s) and a reduced regime (at most 6 Mb/s). The whole-network audit passed
432/432 cases across 12 seeds and three loads. It rejected delay, loss,
congestion, another edge's capacity change, an 8 Mb/s near miss, a permanently
slow link, and a brief dip. Ten record checks, nine practice-truth/load runs,
and the repository's 158 Rust tests also passed. The largest claim log was
4,528 of 24,576 bytes.

The exact submitted pair is preserved in the failed-claim archive for
reproducibility only.
