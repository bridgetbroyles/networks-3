#!/usr/bin/env python3
"""Build review copies of the Home-users incast observer and detector."""

from pathlib import Path
import hashlib
import json
import os
import shutil
import subprocess

ROOT = Path(__file__).resolve().parent
OUT = ROOT / "generated"
OUT.mkdir(exist_ok=True)
CONFIG = """const CLAIM_ID:u32=8;
const MODE:&str="incast_volume";
const LOGGER:u32=2;
const SITES:&[u32]=&[2];
const EDGE:Option<(u32,u32)>=None;
const PACKET_SIZE:u64=64;
const PACKET_PROTO:u8=17;
const PACKET_TTL:u8=255;
const THRESHOLD:u32=10;
const PROBE_MODE:u8=0;
"""

env = os.environ.copy()
env["PATH"] = str(Path.home() / ".cargo" / "bin") + os.pathsep + env.get("PATH", "")
toolchain = subprocess.check_output(["rustc", "--print", "sysroot"], env=env, text=True).strip()
env["DYLD_LIBRARY_PATH"] = str(Path(toolchain) / "lib") + os.pathsep + env.get("DYLD_LIBRARY_PATH", "")
env["A3_CLAIM_CONFIG"] = CONFIG

result = {}
for kind, output_name in (("observer", "switch"), ("detector", "detect")):
    crate = ROOT / "source" / kind
    subprocess.run(
        ["cargo", "build", "--offline", "--release", "--target", "wasm32-unknown-unknown"],
        cwd=crate,
        env=env,
        check=True,
    )
    built = crate / "target" / "wasm32-unknown-unknown" / "release" / f"remaining_claim_{kind}.wasm"
    destination = OUT / f"{output_name}.wasm"
    shutil.copy2(built, destination)
    result[destination.name] = {
        "bytes": destination.stat().st_size,
        "sha256": hashlib.sha256(destination.read_bytes()).hexdigest(),
    }

(OUT / "manifest.json").write_text(json.dumps(result, indent=2) + "\n")
print(json.dumps(result, indent=2))
