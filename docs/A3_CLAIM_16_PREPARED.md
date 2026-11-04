# A3 Claim 16 prepared

Claim 16 targets synchronized customer rates visible in Truth Run 3. Two
different 1,000-byte sources attached to S6 had exactly the same packet counts
in every retained window: 194, 164, 210, 328, 328, 164, 328, and 164. This is
four distinct rate levels with a 2x largest-to-smallest ratio. Unlike a silence
in an incomplete remote series, missing reports cannot manufacture matching
positive local counts.

The claim makes S6 the logging switch, so its decisive counters do not depend
on network-wide report delivery. Every 1.5 seconds it records the source byte
and packet deltas. The detector finds two 1,000-byte sources and requires:

- at least six aligned windows;
- packet counts within one packet in every aligned window;
- at least three distinct positive levels; and
- a largest level at least 1.7 times the smallest.

The variability requirements reject two ordinary constant sources that happen
to have the same rate. Any phase shift, meaningful rate difference, or one
disturbed window also rejects.

Validation performed on the exact package:

- the detector returns true on the actual Truth Run 3 team log;
- 576/576 full 15-switch simulations passed across sixteen seeds and loads
  1.0, 1.25, and 1.5;
- positives kept the other observed delay, loss, failure, and scan mechanisms
  active;
- negatives included one source, constant twins, phase-shifted bursts,
  unequal and near-equal rates, one disturbed window, a late second source,
  the wrong switch, and the wrong packet size;
- 14/14 exact-record and malformed-input checks passed;
- all nine official practice-truth/load combinations returned false;
- all 158 repository tests passed; and
- the largest adversarial claim log was 883 of 24,576 bytes.

This candidate is queued behind the already-submitted Claim 15. It uses much
stronger evidence than failed Claim 14, but local validation still cannot prove
that the hidden grader associates the observed synchronization with an unwon
egg.
