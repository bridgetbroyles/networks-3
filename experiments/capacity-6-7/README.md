# S6-S7 capacity-change experiment

This experiment builds and audits Claim 10. `build.py` creates the observer
and detector from `source/`; `record-audit.py` checks the detector against
encoded and malformed logs; `audit.py` runs the complete 432-case simulator
matrix. Generated worlds, logs, build products, and raw results are ignored.

The observer measures capacity using two adjacent probes whose on-wire sizes
are 64 and 800 bytes. It accepts a pair only when the ingress ordinal proves
that no packet arrived between the probes. The detector then requires repeated
normal and reduced serialization regimes on S6-S7.
