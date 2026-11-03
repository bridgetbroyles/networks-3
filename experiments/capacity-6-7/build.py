#!/usr/bin/env python3
"""Build the narrow link 6-7 capacity-change claim pair."""

from __future__ import annotations

import hashlib
import json
import os
import shutil
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parent
CONFIG = """const CLAIM_ID:u32=10;
const MODE:&str="capacity_pair";
const LOGGER:u32=6;
const SITES:&[u32]=&[6,7];
const EDGE:Option<(u32,u32)>=Some((6,7));
const PACKET_SIZE:u64=0;
const PACKET_PROTO:u8=0;
const PACKET_TTL:u8=255;
const THRESHOLD:u32=6000;
const PROBE_MODE:u8=2;
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
        "server_claim": 10,
        "hypothesis": "link 6-7 changes from normal capacity to 6 Mbps or below and back",
        "normal_capacity_floor_kbps": 8000,
        "reduced_capacity_ceiling_kbps": 6000,
        "files": files,
    }
    (generated / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(json.dumps(manifest, indent=2))


if __name__ == "__main__":
    main()
