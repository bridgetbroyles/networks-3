# Home-users incast claim source

This directory preserves the Rust source used to build the Claim 8 candidate
in `submission/next/claim-08-home-users-incast/`. The observer and detector
are the previously tested multi-mode claim sources, fixed at build time to the
S2 high-volume incast configuration by `build.py`.

Run `python3 build.py` from this directory to create review builds under
`generated/`. The script never replaces the canonical upload files. Because
Rust can embed source paths, a relocated rebuild may differ byte-for-byte from
the tested upload even when behavior is unchanged. The canonical pair and its
hashes remain authoritative.

The exact packaged pair was freshly tested on 30 network cases spanning two
seeds and loads 1.0, 1.25, and 1.5. Cases covered positive, off, nine-source,
low-volume, and short-burst variants. See the candidate README and validation
summary for the exact predicate and results.
