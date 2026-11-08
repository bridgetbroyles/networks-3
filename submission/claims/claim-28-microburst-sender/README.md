# Claim 28 candidate (low odds): the microburst, seen at the sender (Egg 9)

Upload these two files together:

```text
switch.wasm  122,994 bytes  SHA-256 8e0832938569bf1ac7ecdd0cd2909e28b2301c304e4327a1b8f69ec64f42ffbf
detect.wasm   41,274 bytes  SHA-256 f1b6198aa4d8bec6b5fa36b10d81666da35f730dd8be82fddd1f155c91c85a07
```

Sources: `a3_mb_switch/` and `a3_detect_mb/`.

## Why low odds

No truth log shows which customer bursts. The evidence is indirect: in runs
B and C the queue from S6 toward S7 filled completely about once every 2 s
while the link's average delay stayed low, and S6 dropped packets on that
port in runs A and C. Two link-side claims already missed (Claim 10, capacity
on 6-7; Claim 21, queue spikes on 6->7). This is a different hypothesis: some
customer sends short, intense bursts.

## Rule

Every switch counts each customer range's packets per 100 ms at its customer
ports and reports spike ticks (≥ 20 packets and ≥ 3× the range's mean over
the previous 3 s) to S7. A microburst is a spike of at most 200 ms that is not
a lag-switch release (a hold of ≥ 3 empty ticks by a normally busy sender).
True if one range has at least 3 microbursts at least 1 s apart.

## Validation

Patched simulator copy (nested /32 for the Fortnite lag switch), every other
known egg switched on in every scenario (lag switch, Steam surges, Home-users
flood, Amazon on/off, breathing 12-13, leaky 11-12, SRLG, 1-2 delay). Loads
1.0 / 1.25 / 1.5: **30 of 30 right**.

| Scenario | Expected |
|---|---|
| 60-packet bursts every 2 s or 4 s; 30-packet bursts; a sender that only bursts | true |
| off; off plus extra 0.5 s lag holds | false |
| one burst; two bursts; 1.5 s bursts; the same average sent evenly | false |

In the "off" run the other eggs produced 76 spike records — Steam, the flood
and Amazon as long events, the lag switch as releases after holds — and none
counted. The switch completes all practice truth worlds without failure; the
detector said no on all 30 practice runs.
