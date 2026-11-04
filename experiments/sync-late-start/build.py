#!/usr/bin/env python3
"""Build the S1 synchronized late-start claim pair."""

from __future__ import annotations

import hashlib
import json
import os
import shutil
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parent
CONFIG = """const CLAIM_ID:u32=17;
const MODE:&str="late_sync_start";
const LOGGER:u32=1;
const SITES:&[u32]=&[1];
const EDGE:Option<(u32,u32)>=None;
const PACKET_SIZE:u64=0;
const PACKET_PROTO:u8=0;
const PACKET_TTL:u8=255;
const THRESHOLD:u32=2;
const PROBE_MODE:u8=0;
"""


def digest(path: Path) -> dict[str, object]:
    data = path.read_bytes()
    return {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}


def main() -> None:
    env = os.environ.copy()
    env["PATH"] = str(Path.home() / ".cargo/bin") + os.pathsep + env.get("PATH", "")
    toolchain = subprocess.check_output(["rustc", "--print", "sysroot"], env=env, text=True).strip()
    env["DYLD_LIBRARY_PATH"] = str(Path(toolchain) / "lib") + os.pathsep + env.get("DYLD_LIBRARY_PATH", "")
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
        artifact = ROOT / "source" / source / "target/wasm32-unknown-unknown/release" / f"remaining_claim_{source}.wasm"
        destination = generated / output
        shutil.copyfile(artifact, destination)
        files[output] = digest(destination)
    manifest = {
        "server_claim": 17,
        "hypothesis": "two low-rate 1000-byte sources at S1 begin together after startup",
        "required_same_1500ms_observation_window": True,
        "maximum_first_window_packets": 120,
        "maximum_rate_ratio": 1.35,
        "source_site": 1,
        "files": files,
    }
    (generated / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(json.dumps(manifest, indent=2))


if __name__ == "__main__":
    main()
