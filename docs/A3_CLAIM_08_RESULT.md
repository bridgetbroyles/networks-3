# Claim 8 result and correction

Claim 8 used the high-volume Home-user UDP incast pair now archived under
`submission/history/failed-claim-08-home-users-incast/`. The server returned
**no new egg won**; the total remained 40/200 with 22 claims left.

The failed reasoning step was treating a repeatable truth-run anomaly as proof
of an unwon planted egg. In a claim run, every other egg remains ON. A detector
for normal traffic or for an effect caused by an already-won egg therefore
stays true in both the target egg's ON and OFF runs and cannot win.

The submission policy is now stricter. A new claim must:

1. map to a simulator plant or another directly toggleable mechanism;
2. have a truth-run signature not better explained by normal customer setup;
3. pass causal ON/OFF and realistic near-miss cases;
4. run with the already-observed eggs active at the same time;
5. reject incomplete telemetry rather than treating missing data as zero;
6. stay below the claim log cap; and
7. be repackaged only after exact hashes of the tested files are recorded.

Vault duplicate packets and Mapper's TTL ladder were removed from the active
queue after this review because both are ordinary app-generator properties in
the simulator, not planted events. The next audited candidate instead targets
the simultaneous recovered physical interruption of all three links around
S5, which maps directly to the `fail` plant.
