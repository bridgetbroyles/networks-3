#!/usr/bin/env python3
"""Build the narrow link 1-2 repeated-flapping claim pair."""

from __future__ import annotations

import hashlib
import json
import os
import shutil
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parent
CONFIG = """const CLAIM_ID:u32=24;
const MODE:&str="probe_flap";
const LOGGER:u32=2;
const SITES:&[u32]=&[1,2];
const EDGE:Option<(u32,u32)>=Some((1,2));
const PACKET_SIZE:u64=0;
const PACKET_PROTO:u8=0;
const PACKET_TTL:u8=255;
const THRESHOLD:u32=5;
const PROBE_MODE:u8=3;
"""


def digest(path: Path) -> dict[str, object]:
    data = path.read_bytes()
    return {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}


def main() -> None:
    env = os.environ.copy()
    env["PATH"] = str(Path.home() / ".cargo/bin") + os.pathsep + env.get("PATH", "")
    toolchain = subprocess.check_output(
        ["rustc", "--print", "sysroot"], env=env, text=True
    ).strip()
    env["DYLD_LIBRARY_PATH"] = str(Path(toolchain) / "lib") + os.pathsep + env.get(
        "DYLD_LIBRARY_PATH", ""
    )
    env["A3_CLAIM_CONFIG"] = CONFIG
    generated = ROOT / "generated"
    generated.mkdir(exist_ok=True)
    files = {}
    for source, output in (("observer", "switch.wasm"), ("detector", "detect.wasm")):
        subprocess.run(
            ["cargo", "build", "--offline", "--release", "--target", "wasm32-unknown-unknown"],
            cwd=ROOT / "source" / source,
            env=env,
            check=True,
        )
        artifact = (
            ROOT
            / "source"
            / source
            / "target/wasm32-unknown-unknown/release"
            / f"remaining_claim_{source}.wasm"
        )
        destination = generated / output
        shutil.copyfile(artifact, destination)
        files[output] = digest(destination)
    manifest = {
        "server_claim": 24,
        "hypothesis": "link 1-2 repeatedly loses physical connectivity and recovers",
        "minimum_recovered_outages": 2,
        "minimum_common_missing_probe_interval_ms": 500,
        "files": files,
    }
    (generated / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(json.dumps(manifest, indent=2))


if __name__ == "__main__":
    main()
