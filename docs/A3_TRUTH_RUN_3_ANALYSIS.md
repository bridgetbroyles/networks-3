# A3 Truth Run 3 analysis

## Result

Truth Run 3 succeeded. The returned files are preserved unchanged in
`truthrun3/` and are ignored by Git. The log is 19,582 bytes with 1,126
complete records, far below the 65,536-byte truth cap.

Unlike the first two runs, the focused observer retained all 30 customer
counter identities, destination counters, packet examples, link quality,
link transitions, and port drops without filling the logger controller.

## Strongest first claim

The highest-confidence invariant is sustained bidirectional delay on physical
link 13-14:

- normal links are about 551 microseconds one way;
- every usable S14-hears-S13 record is 25,051 microseconds;
- every usable S13-hears-S14 record is about 25,051-25,117 microseconds on
  average, with peaks no greater than 25,843 microseconds;
- Run 1 and Run 2 independently recorded the same 25,051-microsecond direction;
- Run 3 confirms both directions repeatedly, with no missing keepalives in the
  retained periods.

This is much stronger evidence than an isolated queue spike. It is tied to a
fixed physical edge, persists throughout the run, appears in both directions,
and is roughly 45 times the ordinary propagation delay.

## Claim 1 hypothesis

The hidden egg is that link 13-14 has sustained high delay in both directions.
Likely false or near-miss cases include:

- the link is normal;
- another link is slow;
- only one direction of 13-14 is slow;
- 13-14 is slower than normal but remains below the true delay;
- a brief queue spike raises only the peak rather than the sustained average.

The detector therefore requires at least two nonempty two-second summaries in
each direction and a median average delay of at least 20,000 microseconds in
both directions. It does not use peak delay alone.

## Other evidence retained for later claims

Run 3 also contains likely candidates for later claims:

- asymmetric delay on link 1-2 (about 30 ms one way and 2 ms the other);
- repeated loss/gap evidence on several fixed links;
- bidirectional outages and recoveries on 0-1 and 10-11;
- capacity/drop evidence at S6 port 2 and S12 port 2;
- a strong 64-byte Home-users side flow during the middle of the run;
- a persistent 299-byte Akamai-users flow;
- customer pauses and burst changes;
- several small-packet and side-flow signatures.

Those should become separate claims. One claim can win at most one egg, and
the server provides no diagnostic for a miss, so Claim 1 deliberately does
not combine these unrelated hypotheses.
