from pathlib import Path
import json,struct,subprocess
P=Path(__file__).resolve().parent;SIM=str(P.parents[1]/'target/release/competitive_net_sim');m=json.loads((P/'manifest.json').read_text())[0]
def encode(kind):
 out=bytearray()
 for sw,peer in [(11,12),(12,11)]:
  gaps=0
  for i in range(11):
   if i:
    g=18 if kind not in ['off','two-percent','one-way','disjoint'] else 0
    if kind=='two-percent':g=5
    if kind=='one-way' and sw==11:g=18
    if kind=='disjoint':g=40 if (sw==11 and i<=4) or (sw==12 and i>=7) else 0
    gaps+=g
   tx=i*250;rx=tx-gaps;big=1 if kind=='big-gap' and i>=5 else 0;drops=1 if kind=='queue-drop' and i>=5 else 0
   if kind=='counter-reset' and i==6:rx=0;gaps=0
   if kind=='bad-sent-accounting':rx+=500*i
   body=struct.pack('<BBIIIIIIHIB',sw,peer,(i+1)*1000,tx,rx,gaps,big,drops,10,551,0)
   assert len(body)==33
   out+=bytes([21,len(body)])+body
 return bytes(out)
def main():
 rows=[];v=P/'record-tests';v.mkdir(exist_ok=True)
 for kind,want in [('positive',True),('off',False),('two-percent',False),('one-way',False),('disjoint',False),('big-gap',False),('queue-drop',False),('counter-reset',False),('bad-sent-accounting',False),('empty',False),('malformed',False)]:
  data=b'' if kind=='empty' else encode('positive' if kind=='malformed' else kind)
  if kind=='malformed':data+=b'\x15\x21\x00'
  f=v/(kind+'.team');f.write_bytes(data)
  got=subprocess.check_output([SIM,'detect',str(P/m['folder']/'detect.wasm'),str(f)],text=True).strip()=='true'
  assert got==want,(kind,want,got)
  rows.append({'case':kind,'expected':want,'actual':got,'pass':True})
 (P/'record-results.json').write_text(json.dumps({'files':m['files'],'checks':rows},indent=2)+'\n')
 print('PASS',len(rows),'record checks')
if __name__=='__main__':main()
