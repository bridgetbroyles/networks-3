from pathlib import Path
import json,subprocess,os
from worlds import app,world
P=Path(__file__).resolve().parent
SIM=str(P.parents[1]/'target/release/competitive_net_sim')
M=json.loads((P/'manifest.json').read_text())[0];PAIR=P/M['folder']
V=P/'audit-worlds';V.mkdir(exist_ok=True)
def scenario(name,seed):
 apps=[app(100,11,101),app(101,12)];plants=[]
 mapping={'full-run':(4000,39000,7.),'ten-seconds':(12000,22000,7.),'early':(1500,11500,7.),'late':(29000,39500,7.),'near-one-percent':(4000,39000,1.),'near-two-percent':(4000,39000,2.),'boundary-2.5-percent':(4000,39000,2.5),'full-five-percent':(4000,39000,5.),'ten-seconds-five-percent':(12000,22000,5.),'off':(0,0,0.)}
 if name in mapping:
  start,stop,pct=mapping[name]
  if pct:plants=[dict(kind='wire_loss',a=11,b=12,loss_pct=pct,start_ms=start,stop_ms=stop)]
  expected=pct>=5.
 elif name in ['loss-plus-later-congestion','congestion-only']:
  apps=[app(100,11,101,size=1000,rate=6.,preset='amazon',on_ms=300,off_ms=700,start_ms=5030),app(101,12)]
  plants=[dict(kind='link_capacity',a=11,b=12,steps=[[0,10],[25000,1],[35000,10]])]
  if name=='loss-plus-later-congestion':plants.append(dict(kind='wire_loss',a=11,b=12,loss_pct=7.,start_ms=4000,stop_ms=39000))
  expected=name=='loss-plus-later-congestion'
 elif name=='high-load-no-loss':
  apps=[app(100,11,101,size=1000,rate=9.95),app(101,12,100,size=1000,rate=9.95)];expected=False
 elif name=='recovered-outage':
  plants=[dict(kind='fail',a=11,b=12,at_ms=12000,for_ms=6000)];expected=False
 elif name=='other-edge':
  plants=[dict(kind='wire_loss',a=6,b=7,loss_pct=20.,start_ms=4000,stop_ms=39000)];expected=False
 elif name=='brief-spike':
  plants=[dict(kind='wire_loss',a=11,b=12,loss_pct=70.,start_ms=12000,stop_ms=14000)];expected=False
 else:raise ValueError(name)
 return world(apps,plants,seed),expected
def main():
 cases=['full-run','ten-seconds','early','late','full-five-percent','ten-seconds-five-percent','near-one-percent','near-two-percent','boundary-2.5-percent','off','loss-plus-later-congestion','congestion-only','high-load-no-loss','recovered-outage','other-edge','brief-spike']
 seeds=[int(x) for x in os.environ.get('LOSS_SEEDS','3341,3349').split(',')]
 loads=[float(x) for x in os.environ.get('LOSS_LOADS','1.0,1.5').split(',')]
 only=os.environ.get('LOSS_CASES')
 if only:cases=only.split(',')
 rows=[]
 for case in cases:
  for seed in seeds:
   for load in loads:
    text,expected=scenario(case,seed)
    stem=f'{case}-{seed}-{load}';w=V/(stem+'.toml');log=V/(stem+'.team');w.write_text(text)
    proc=subprocess.run([SIM,'run-world',str(w),'--program',str(PAIR/'switch.wasm'),'--team-log',str(log),'--load-scale',str(load)],capture_output=True,text=True)
    if proc.returncode or 'program failed' in proc.stderr.lower():raise RuntimeError(stem+proc.stderr)
    got=subprocess.check_output([SIM,'detect',str(PAIR/'detect.wasm'),str(log)],text=True).strip()=='true'
    row={'case':case,'seed':seed,'load':load,'expected':expected,'actual':got,'log_bytes':log.stat().st_size,'pass':got==expected and log.stat().st_size<20000}
    rows.append(row);(P/os.environ.get('LOSS_RESULT','audit-results.json')).write_text(json.dumps({'files':M['files'],'runs':rows},indent=2)+'\n')
    if not row['pass']:raise AssertionError(row)
  print('PASS',case,flush=True)
 print('PASS',len(rows),'network cases; max log',max(r['log_bytes'] for r in rows),flush=True)
if __name__=='__main__':main()
