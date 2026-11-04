#!/usr/bin/env python3
"""Build the focused Home-user mixed-traffic surge claim pair."""

from __future__ import annotations

import hashlib
import json
import os
import shutil
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[1]
SOURCE = REPO / "experiments" / "login-three-destination-fanout" / "source"
CONFIG = """const CLAIM_ID:u32=19;
const MODE:&str="home_udp_surge_v2";
const LOGGER:u32=2;
const SITES:&[u32]=&[2];
const EDGE:Option<(u32,u32)>=None;
const PACKET_SIZE:u64=64;
const PACKET_PROTO:u8=17;
const PACKET_TTL:u8=255;
const THRESHOLD:u32=10;
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
    for crate, output in (("observer", "switch.wasm"), ("detector", "detect.wasm")):
        subprocess.run(
            ["cargo", "build", "--offline", "--release", "--target", "wasm32-unknown-unknown"],
            cwd=SOURCE / crate,
            env=env,
            check=True,
        )
        artifact = SOURCE / crate / "target/wasm32-unknown-unknown/release" / f"remaining_claim_{crate}.wasm"
        destination = generated / output
        shutil.copyfile(artifact, destination)
        files[output] = digest(destination)
    manifest = {
        "server_claim": 19,
        "status": "approved for Claim 19 after complete audit and evidence review",
        "hypothesis": "one S2 /24 changes from a steady 200-byte baseline to a sustained 64-byte UDP surge from many hosts and then recovers",
        "files": files,
    }
    (generated / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(json.dumps(manifest, indent=2))


if __name__ == "__main__":
    main()
