# Claim 26 result: won Egg 11, Cloudflare DDoS

Claim 26 (Nov 8, 12:39am) won **Egg 11 · Traffic · Cloudflare DDoS**, worth
8 points: 112 of 200.

The uploaded pair is `submission/claims/claim-26-home-udp-surge-fixed/`: the
partner's Home UDP surge v2 switch, unchanged, with its detector fixed for
load 1.0 (see that folder's README and `detector.patch`).

So the planted Home-users → Fastly 64-byte UDP flood seen in truth runs A and
C is the Cloudflare DDoS egg. What the winning rule required, and the earlier
attempts did not:

- a surge lasting 9–18 s (the real one lasts 15 s), with a normal 200-byte
  baseline before and after — Claims 8, 20 and 22 accepted floods of any
  length;
- working at load 1.0 — the unfixed v2 needed samples that only exist at
  higher loads.
