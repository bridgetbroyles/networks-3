# S5 physical-interruption experiment

This experiment builds and audits the candidate for server Claim 9. Truth Run
3 shows all three neighbors of S5 losing contact together and then recovering
together. Unlike the failed Home-user incast claim, this maps directly to the
simulator's planted `fail` mechanism rather than a normal customer-generator
property or a possible consequence of an already-won egg.

The observer sends direct probes across S5's three links and counts all
physical-port arrivals without sampling. The detector requires a common
missing-probe interval of at least 3.5 seconds at S0, S6, and S10, explicit
zero arrival counts on all three links, matching link-silence evidence, and
subsequent probe recovery.

Run:

```sh
python3 experiments/hub-outage/build.py
python3 experiments/hub-outage/audit.py
python3 experiments/hub-outage/record-audit.py
```

The final audit covered 396 whole-network cases over 12 seeds and three load
levels. The positive cases kept the other observed link plants active. False
cases included one or two failed links, 2- and 3-second interruptions,
permanent failure, staggered failures, another isolated hub, and suppressed
keepalives with physically working links. All passed; the maximum claim log
was 11,723 of 24,576 bytes. Nine encoded malformed/incomplete-log cases and
nine practice-truth load combinations also passed.
