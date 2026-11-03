from pathlib import Path
import ast,copy,json,subprocess
from worlds import app,world
P=Path(__file__).resolve().parent
M=json.loads((P/'manifest.json').read_text())[0]
SIM=str(P.parents[1]/'target/release/competitive_net_sim')
src=(P/'combined-fixtures.py').read_text()
tree=ast.parse(src)
nodes=[n for n in tree.body if (isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id in ['apps','plants'] for t in n.targets)) or (isinstance(n,ast.FunctionDef) and n.name=='text')]
exec(compile(ast.Module(body=nodes,type_ignores=[]),'combined-fixtures','exec'),globals())
V=P/'combined-worlds';V.mkdir(exist_ok=True)
rows=[]
def run(name,t,load,expected):
 w=V/(name+'.toml');log=V/(name+'.team');w.write_text(t)
 proc=subprocess.run([SIM,'run-world',str(w),'--program',str(P/M['folder']/'switch.wasm'),'--team-log',str(log),'--load-scale',str(load)],capture_output=True,text=True)
 if proc.returncode or 'program failed' in proc.stderr.lower():raise RuntimeError(proc.stderr)
 got=subprocess.check_output([SIM,'detect',str(P/M['folder']/'detect.wasm'),str(log)],text=True).strip()=='true'
 row=dict(case=name,load=load,expected=expected,actual=got,log_bytes=log.stat().st_size,passed=got==expected and log.stat().st_size<20000)
 rows.append(row)
 (P/'combined-results.json').write_text(json.dumps(dict(files=M['files'],runs=rows),indent=2)+'\n')
 assert row['passed'],row
 print('PASS',name,flush=True)
for on in [False,True]:
 for load in [1.,1.5]:run(f'combined-{on}-{load}',text(M,on),load,on)
full=[app(100+i,i%15,100+(i+27)%54,size=200,rate=.02) for i in range(54)]
run('54-sender-header',world(full,seed=3389),1.5,False)

for seed in range(3501,3511):
 for on in [False,True]:run(f"combined-seed-{seed}-{on}",text(M,on).replace("seed=3373",f"seed={seed}"),1.5,on)
