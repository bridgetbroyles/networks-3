# Claim 19 submission

The recommended pair is in:

```text
submission/next/claim-19-home-udp-surge/switch.wasm
submission/next/claim-19-home-udp-surge/detect.wasm
```

Upload those two files together for one claim. Do not rename, rebuild, or mix
them with files from another folder.

## What it detects

The switch observes Home-user ingress at S2. The detector requires a normal
200-byte baseline, a sustained 9-18-second 64-byte UDP surge from at least
five hosts to exactly one destination, and a later return to normal traffic.

This is a substantive correction to failed Claim 8. The old detector returned
true for no-recovery, 24-second, wrong-baseline, and five-destination cases;
the new detector rejects all of them.

## Verification

- 168/168 freshly simulated cases passed across four seeds and three loads.
- The largest log was 11,664/24,576 bytes.
- A fresh positive returned true.
- All nine ordinary practice truth/load runs returned false.
- The exact detector returned true on Truth Run 3.
- The copied files match the built and tested hashes.

```text
25dca8a1089ea10aec5edc295584d7da879a2ea294fb666dbaaaa81ae871fb72  switch.wasm
782ad587098fc661b63e0a3998dffb9712cd424327fa14f22d6be84f02ca1985  detect.wasm
```

No local process can prove the undisclosed ten-run near-miss boundary. This is
the strongest current recommendation because the behavior maps to an explicit
`side_flow` plant and the revised detector fixes observed false-positive
families in the previously failed attempt.
