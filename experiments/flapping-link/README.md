# Link 1-2 flapping experiment

This is the highest-confidence candidate for the still-unwon Egg 18,
"The flapping link." Truth Run 3 repeatedly recorded S2 losing and regaining
contact with S1. A single `fail` plant explains one outage and recovery; the
simulator permits several `fail` plants on the same edge, which produces the
repeated up/down behavior implied by *flapping*.

The observer sends direct probes across link 1-2 from both endpoints. Every
snapshot also records how much ordinary physical-port traffic arrived. The
detector accepts only when both directions independently show at least two
overlapping intervals with all of the following evidence:

- probes arrived before the interval;
- probes and physical ingress both fell to zero;
- the keepalive state declared the same link silent;
- probes later arrived again;
- this complete down/recovery cycle happened at least twice.

That predicate separates the candidate from a single recovered outage, a
permanent failure, failure of another link, the already-won two-link SRLG and
three-link rebooting-switch events, one-way delay, wire loss, changing
capacity, congestion, and missing keepalives while data still crosses the
link.

Run `python3 build.py`, `python3 audit.py`, and `python3 record-audit.py` from
this directory. Generated Wasm files and test worlds remain local until the
pair passes every audit and is deliberately promoted for upload.
