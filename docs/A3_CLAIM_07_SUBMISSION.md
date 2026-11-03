# A3 Claim 7 submission: sustained wire loss on 11-12

## Result

Claim 7 won **Egg 16 - Link - The leaky link**, worth 16 points. The confirmed
total became 40/200. This pair is complete and must not be resubmitted.

The exact successful files are archived at:

```text
submission/completed/egg-16-leaky-link/switch.wasm
submission/completed/egg-16-leaky-link/detect.wasm
```

They are retained for reproducibility only.

## Why this claim is different from the failed claims

Truth Run 3 measured physical edge 11-12 in both directions:

- S11 receiving from S12: 116 keepalives and 8 missing sequence numbers;
- S12 receiving from S11: 111 keepalives and 9 missing sequence numbers;
- delay stayed near the healthy baseline rather than showing a congested or
  failed link.

That is approximately 6.5% and 7.5% observed loss in the two directions. The
failed capacity claim relied on customer-induced drops and queue delay, which
can remain present when another egg is toggled. This observer instead sends
numbered probes directly over edge 11-12 and counts them in the data plane.
It therefore measures the physical edge rather than inferring it from customer
traffic.

The detector requires an aligned interval of at least eight seconds with:

- enough probes in both directions;
- statistically significant loss above a 2.5% null in both directions;
- loss appearing in at least three separated epochs;
- consistent sender and receiver sequence accounting;
- no large sequence gap indicating an outage;
- no target-port drops indicating local congestion.

## Fresh verification on November 3

The exact packaged pair was rerun, without rebuilding or replacing either
Wasm file:

- 64 main network cases passed;
- 48 additional boundary/seed cases passed;
- 25 simultaneous-anomaly and header cases passed;
- 11 encoded-record and malformed-input checks passed;
- all three official practice truth worlds loaded and completed at 1.5x;
- largest claim log: 2,730 bytes of the 24,576-byte cap.

Positive cases include sustained 5% and 7% loss lasting ten seconds or most of
the run. Negative cases include 0%, 1%, 2%, and 2.5% loss, one-way loss,
disjoint evidence, a two-second 70% spike, congestion without loss, near-link
saturation, a recovered outage, loss on another edge, queue drops, large gaps,
counter reset, malformed data, and missing sender accounting.

These tests prove that the packaged programs implement the stated rule and
survive the public limits. They cannot prove the undisclosed ten hidden runs or
their exact near-miss threshold. The truth evidence makes this a strong causal
hypothesis, not a certainty.

## Exact artifacts

```text
switch.wasm
  bytes: 121509
  SHA-256: 3aa689d58d014597cfa2e1383b61442173ed6dabdb2c7479dbfcfc203d8f2792

detect.wasm
  bytes: 39649
  SHA-256: 528f4ee5dfff29327aa15c153b4c0585c14f682fa81da91fd776f3106cdc455b
```

## What to report after submission

Send the complete server comment and say that the exact folder was
`submission/next/claim-07-wire-loss`. A win identifies the next family to
remove from consideration. A failure means the observed loss is not isolated
by this egg definition or one hidden boundary differs; it does not justify
submitting a weaker passive-loss variant.
