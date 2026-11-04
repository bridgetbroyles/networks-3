# S7 sequence-zero destination fanout

This experiment targets the strongest still-unclaimed traffic signature in all
three truth runs.  At S7, one 200-byte TCP source repeatedly sends packets with
sequence zero to a large number of addresses in one destination `/24`.

The decisive retained evidence is:

- Truth Run 1: 121 distinct destinations over the run;
- Truth Run 2: 128 distinct destinations over the run;
- Truth Run 3: 69 distinct destinations between 1.2 and 3.8 seconds, including
  at least 50 inside a five-second window.

Normal mixed-workload packets are numbered.  The observer therefore samples
only 200-byte TCP data arriving from a customer-facing port at S7 with sequence
zero, then keeps one record per exact source/destination pair.  The detector
requires one exact source to reach at least 50 distinct addresses in one
destination `/24` inside five seconds.

This is deliberately a single-mechanism claim.  It does not require the
1,000-byte Cloudflare shuffle (already associated with won Egg 10), the sparse
Akamai traffic, or any link/customer symptom.

Run `build.py`, then `audit.py` and `record_audit.py`.  Generated worlds, logs,
and Wasm files are ignored build artifacts until the pair is explicitly
promoted into `submission/next/`.
