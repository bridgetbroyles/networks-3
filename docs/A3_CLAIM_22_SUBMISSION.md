# A3 Claim 22: Home users flood Fastly (narrowed retry of Claim 20)

## Upload exactly these two files

```text
submission/claims/claim-22-home-users-flood-fastly/switch.wasm
submission/claims/claim-22-home-users-flood-fastly/detect.wasm
```

```text
switch.wasm  121,869 bytes  SHA-256 2e79edbf17a9497691a6dc798f996f2ec2bcbd5d2d0a1a5e5cd3d35e2f5fa1e3
detect.wasm   78,739 bytes  SHA-256 cca0b844ff7b6f13e6f7373fca9edd416f4a326e8f60ab874af3ffff282f8381
```

Sources: `a3_ddos_switch/` and `a3_detect_ddos/`.

## Why this is an egg

In truth run C the flood's sequence numbers are one counter shared by every
sending host, starting at 0 at exactly 15.0 s and rising by exactly 3,000 a
second (0 at 15.0 s, 15,600 at 20.2 s, 31,200 at 25.4 s). That is how the
simulator numbers a planted flow (`sim.rs`: a planted packet's sequence is
its plant's send count); a customer's own traffic is numbered per flow.
The truth runs contain no plants except eggs.

| Run | Evidence |
|---|---|
| A | 7 Home-user hosts → 10.50.151.8 (Fastly), 64 B, from 16.7 s (heavily sampled) |
| C | 22 Home-user hosts → 10.50.151.8, 64 B UDP, from 15.0 s; Home users' small-packet rate 3,009/s from 15.0 to ~30.0 s |

Fastly's 200-byte fanout to Home users (sequence 0) is not planted: it is
normal open-loop CDN traffic, like Cloudflare's and Akamai's.

## Why Claim 20 probably missed, and what changed

Claim 20 accepted a flood on any target from any senders. Egg near misses
usually swap the subject ("another link", "another customer"), so a flood
on another customer, or by another customer's hosts, would have fooled it.

| Check | Claim 20 | Claim 22 |
|---|---|---|
| Senders | ≥ 24 in 5 s | ≥ 20 in 5 s, ≥ 80% from one range A |
| Target | any address | its range V must be A's own CDN: ≥ 40 hosts of A exchange 128–799 B packets with V |
| Rate | ≥ 500/s | ≥ 1,200/s (true: 2,000–3,000) |
| Duration | ≥ 5 s | ≥ 10 s (true: 15 s) |

## Switch

Logger S2. First two packets of every small customer-data host pair
(register 1); first packet of every 128–799 B customer-data host pair
(register 2; separate, because a host pair has the same key both ways); small
UDP packets per destination range per second (data-plane counter). Reports
flooded with duplicate suppression. The logger keeps medium pairs to 10 KB so
the flood's records always fit in the 24 KB cap.

## Validation

Grid replica: Home users 128 hosts at S2 talking to Fastly (one host, .8) at
S7, Fastly's fanout to all 128 Home-user hosts, Cloudflare, a 128-host
"Campus" customer, Ping, 40 fillers. Loads 1.0 / 1.25 / 1.5: **51 of 51
right**.

| Scenario | Expected | Senders | Related | Busy s (≥1,200/s) |
|---|---|---|---|---|
| flood 15–30 s | true | 69–73 | 126 | 15 |
| flood, relation only via Fastly's fanout | true | 69–73 | 111–119 | 15 |
| flood, relation only via Home users' traffic | true | 69–73 | 126 | 15 |
| flood + loss/slow/SRLG/narrow-link faults | true | 69–73 | 126–127 | 15 |
| off; off + faults | false | 1 | 0 | 0 |
| Home users flood Cloudflare | false | 69–73 | **0** | 15 |
| Campus (128 hosts) floods Fastly | false | 69–73 | **0** | 15 |
| one host / ten hosts flood Fastly | false | **1 / 10** | 0 | 0 / 15 |
| 2 s flood / 7 s flood | false | 69–73 | 126 | **2 / 7** |
| 300 packets/s | false | 94–101 | 126 | **0** |
| 3 packets/s over the whole run | false | **16** | 126 | 0 |
| TCP flood; 1000 B flood; scan of Fastly | false | 1 | 0 | 0 |

Real run C passes all three checks with the old truth observer's sampling:
22 senders, 119 related hosts, 3,009/s for ~15 s. The switch completes all
three practice truth worlds at all three loads without a program failure;
the detector said no on all 30 practice runs.

## Remaining risks

- If the egg's near miss is something not modelled here (for example a
  flood exactly as long and as fast but differing in some other way), this
  may still miss.
- If claim runs' Home users and Fastly do not exchange 128–799 B packets
  (they did across 119 hosts in run C), check 2 fails.
