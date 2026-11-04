# Claim 19: sustained Home-user UDP surge and recovery

Upload **both** `switch.wasm` and `detect.wasm` from this directory together.
Do not mix either file with another claim pair.

The pair detects one S2 `/24` changing from a steady 200-byte baseline to a
sustained 64-byte UDP surge from multiple hosts to one destination, followed
by recovery. It is the corrected successor to failed Claim 8. Claim 8 also
accepted no-recovery, 24-second, wrong-baseline, and five-destination cases;
this detector rejects all four families.

Final checks:

- 168/168 freshly generated whole-network cases passed;
- four seeds and loads 1.0, 1.25, and 1.5;
- unrelated loss, delay, and recovery incidents remained active;
- fresh exact positive: true;
- all nine ordinary practice truth/load runs: false;
- exact Truth Run 3 result: true;
- largest generated log: 11,664/24,576 bytes;
- packaged hashes match the tested files.

The private ten-run boundary is unavailable locally, so this cannot be an
absolute guarantee. It is recommended because it has a direct simulator-plant
explanation and repairs concrete false positives in the failed earlier claim.
