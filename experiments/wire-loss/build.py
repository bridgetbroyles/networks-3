#!/usr/bin/env python3
"""Build for review without replacing the tested upload pair."""
from pathlib import Path
import hashlib,json,os,shutil,subprocess
P=Path(__file__).resolve().parent
out=P/'generated';out.mkdir(exist_ok=True)
env=os.environ.copy()
env['PATH']=str(Path.home()/'.cargo/bin')+os.pathsep+env.get('PATH','')
toolchain=subprocess.check_output(['rustc','--print','sysroot'],env=env,text=True).strip()
env['DYLD_LIBRARY_PATH']=str(Path(toolchain)/'lib')+os.pathsep+env.get('DYLD_LIBRARY_PATH','')
env['A3_CLAIM_CONFIG']=''
result={}
for kind,name in [('observer','switch'),('detector','detect')]:
 subprocess.run(['cargo','build','--offline','--release','--target','wasm32-unknown-unknown'],cwd=P/'source'/kind,env=env,check=True)
 src=P/'source'/kind/f'target/wasm32-unknown-unknown/release/remaining_claim_{kind}.wasm'
 dst=out/f'{name}.wasm';shutil.copy2(src,dst)
 result[dst.name]={'bytes':dst.stat().st_size,'sha256':hashlib.sha256(dst.read_bytes()).hexdigest()}
(out/'manifest.json').write_text(json.dumps(result,indent=2)+'\n')
expected=json.loads((P/'manifest.json').read_text())[0]['files']
print('Rebuilt hashes match tested upload pair:',result==expected)
