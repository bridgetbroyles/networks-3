use std::collections::{BTreeMap,BTreeSet};
include!(concat!(env!("OUT_DIR"), "/config.rs"));
#[derive(Clone,Debug)]
struct Stat { sw:u8,c:u8,t:u16,b:u32,p:u32 }
#[derive(Clone,Debug)]
struct Packet {sw:u8,t:u16,src:u32,dst:u32,size:u16,proto:u8,ttl:u8,seq:u32,us:u32}
#[derive(Clone,Debug)]
struct Summary {sw:u8,n:u8,port:u16,t:u16,received:u16,gaps:u16,avg:u32,peak:u32}
#[derive(Clone,Debug)]
struct Probe {sw:u8,n:u8,t:u16,count:u16,gaps:u16,last:u16,min_us:u32,max_us:u32,pairs:u16,pair_min:u32,pair_mean:u32}
#[derive(Default,Debug)]
struct Log {ingress:Vec<(u8,u8,u16,u32)>,probes:Vec<Probe>,stats:Vec<Stat>, packets:Vec<Packet>, links:Vec<Summary>,events:Vec<(u8,u8,u16,u8)>,drops:Vec<(u8,u16,u32)>,prefix:BTreeMap<u8,(u32,u8)>,heartbeats:BTreeMap<u8,Vec<u16>>}
fn u16b(b:&[u8],at:usize)->u16 {u16::from_le_bytes([b[at],b[at+1]])}
fn u32b(b:&[u8],at:usize)->u32 {u32::from_le_bytes(b[at..at+4].try_into().unwrap())}
fn parse(bytes:&[u8])->Option<Log> {
 let mut log=Log::default(); let mut at=0;
 while at<bytes.len() {
  if at+2>bytes.len(){return None;}
  let tag=bytes[at];let len=bytes[at+1] as usize;at+=2;
  let b=bytes.get(at..at+len)?;at+=len;
  match (tag,b.len()) {
   (1,_) if b.len()>=2 => {
    let (start,count,off)=if b[0]==1 {(1,b[1] as usize,2)} else if b[0]==2 && b.len()>=3 {(b[1],b[2] as usize,3)} else {return None;};
    if off+count*5!=b.len(){return None;}
    for i in 0..count {if b[off+i*5+4]>32{return None;}log.prefix.insert(start.checked_add(i as u8)?,(u32b(b,off+i*5),b[off+i*5+4]));}
   }
   (2,10)|(2,12)=>log.stats.push(Stat{sw:b[0],c:b[1],t:u16b(b,2),b:u32b(b,4),p:if len==12 {u32b(b,8)} else {u16b(b,8) as u32}}),
   (3,19)|(7,19)|(3,23)|(7,23)=>log.packets.push(Packet{sw:b[0],t:u16b(b,1),src:u32b(b,3),dst:u32b(b,7),size:u16b(b,11),proto:b[17],ttl:b[18],seq:u32b(b,13),us:if len==23 {u32b(b,19)}else{0}}),
   (4,6)|(5,6)=>log.events.push((b[0],b[1],u16b(b,4),tag)),
   (6,18)=>log.links.push(Summary{sw:b[0],n:b[1],port:u16b(b,2),t:u16b(b,4),received:u16b(b,6),gaps:u16b(b,8),avg:u32b(b,10),peak:u32b(b,14)}),
   (8,9)=>log.drops.push((b[0],u16b(b,3),u32b(b,5))),
   (13,8)=>log.ingress.push((b[0],b[1],u16b(b,2),u32b(b,4))),
   (11,28)=>log.probes.push(Probe{sw:b[0],n:b[1],t:u16b(b,2),count:u16b(b,4),gaps:u16b(b,6),last:u16b(b,8),min_us:u32b(b,10),max_us:u32b(b,14),pairs:u16b(b,18),pair_min:u32b(b,20),pair_mean:u32b(b,24)}),
   (10,3)=>log.heartbeats.entry(b[0]).or_default().push(u16b(b,1)),
   _=>{},
  }
 }
 log.events.sort_unstable();for v in log.heartbeats.values_mut(){v.sort_unstable();v.dedup();}
 Some(log)
}
fn median(v:&[u32])->u32{if v.is_empty(){return 0;}let mut s=v.to_vec();s.sort_unstable();s[s.len()/2]}
fn intervals(l:&Log,a:u8,b:u8,min:u16)->Vec<(u16,u16)> {
 let mut events:Vec<_>=l.events.iter().filter(|x|x.0==a&&x.1==b).map(|x|(x.2,x.3)).collect();events.sort_unstable();events.dedup();
 let mut down=None;let mut out=vec![];
 for (t,tag) in events {if tag==4 {down.get_or_insert(t);} else if let Some(start)=down.take(){if t>=start+min{out.push((start,t));}}}
 out
}
fn overlap(a:(u16,u16),b:(u16,u16),min:u16)->bool{a.1.min(b.1).saturating_sub(a.0.max(b.0))>=min}
fn target(s:&Summary)->bool{let (a,b)=EDGE.unwrap_or((255,255));(s.sw as u32,s.n as u32)==(a,b)||(s.sw as u32,s.n as u32)==(b,a)}
fn series(l:&Log)->BTreeMap<(u8,u8),Vec<Stat>> {
 let mut map:BTreeMap<_,Vec<Stat>>=BTreeMap::new();
 for s in &l.stats {if SITES.contains(&(s.sw as u32)){map.entry((s.sw,s.c)).or_default().push(s.clone());}}
 for v in map.values_mut(){v.sort_by_key(|s|s.t);v.dedup_by_key(|s|s.t);}
 map
}
fn avg_size(v:&[Stat])->u64{let p:u64=v.iter().map(|s|s.p as u64).sum();if p==0{0}else{v.iter().map(|s|s.b as u64).sum::<u64>()/p}}
// The observer interval is known. Inferring it from received heartbeats
// could mistake systematic report loss for a longer measurement interval.
fn nominal(_l:&Log,_sw:u8)->u16 {15}
// Quiet windows count only when a zero statistic was explicitly received with its heartbeat.
fn quiet_intervals(l:&Log,v:&[Stat],duration:u16)->Vec<(u16,u16)> {
 if v.len()<5{return vec![];}let step=nominal(l,v[0].sw);let mut start=None;let mut last=0;let mut out=vec![];
 let h=l.heartbeats.get(&v[0].sw);if h.is_none(){return vec![];}
 for (i,s) in v.iter().enumerate(){
  if i>0 && s.t.saturating_sub(last)>step+2{start=None;}
  if s.p==0 && h.unwrap().contains(&s.t) {
   if i>0 && (v[i-1].p>0 || start.is_some()){start.get_or_insert(s.t.saturating_sub(step));}
  } else if s.p>0 {if let Some(st)=start.take(){if last.saturating_sub(st)>=duration {out.push((st,last));}}}
  last=s.t;
 }
 out
}

fn probe_intervals(l:&Log,a:u8,b:u8,min:u16)->Vec<(u16,u16)> {
 let mut p:Vec<_>=l.probes.iter().filter(|p|p.sw==a&&p.n==b).collect();p.sort_by_key(|p|p.t);
 let mut out=vec![];let mut start=None;let mut last=0;let mut active=false;
 for s in p {
   if last>0&&s.t.saturating_sub(last)>12 {start=None;active=false;}
   if s.count==0 && l.ingress.iter().any(|x|x.0==a&&x.1==b&&x.2==s.t&&x.3==0) {if active{start.get_or_insert(s.t.saturating_sub(10));}}
   else if s.count>0 {if let Some(t)=start.take(){if last.saturating_sub(t)>=min&&intervals(l,a,b,1).iter().any(|i|overlap((t,last),*i,1)){out.push((t,last));}}active=true;}else{start=None;active=false;}
   last=s.t;
 }
 out
}
fn detect(bytes:&[u8])->bool {
 let Some(l)=parse(bytes) else{return false;};
 match MODE {

 "echo"=>{
   let mut m:BTreeMap<(u32,u32,u32),Vec<u32>>=BTreeMap::new();
   for p in &l.packets {if p.sw==3&&p.size==500&&p.proto==6&&p.seq>0&&p.us>0 {m.entry((p.src,p.dst,p.seq)).or_default().push(p.us);}}
   let mut flows:BTreeMap<(u32,u32),usize>=BTreeMap::new();
   for ((src,dst,_),mut ts) in m {ts.sort_unstable();if ts.windows(2).any(|w|w[1]>w[0]&&w[1]-w[0]<=750){*flows.entry((src,dst)).or_default()+=1;}}
   flows.values().any(|n|*n>=2)
 },
 "probe_loss"=>{
   let(a,b)=EDGE.unwrap();
   if l.events.iter().any(|(s,n,_,tag)|*tag==4&&((*s as u32,*n as u32)==(a,b)||(*s as u32,*n as u32)==(b,a))){return false;}
   [(a,b),(b,a)].iter().all(|(a,b)|{
     let p:Vec<_>=l.probes.iter().filter(|p|p.sw as u32==*a&&p.n as u32==*b&&p.t>=50&&p.t<=380&&p.count>0).collect();
     let recv:u32=p.iter().map(|p|p.count as u32).sum();let gaps:u32=p.iter().map(|p|p.gaps as u32).sum();
     let delays:Vec<_>=l.links.iter().filter(|s|s.sw as u32==*a&&s.n as u32==*b&&s.received>0).map(|s|s.avg).collect();
     let ports:BTreeSet<_>=l.links.iter().filter(|s|s.sw as u32==*a&&s.n as u32==*b).map(|s|(s.sw,s.port)).collect();
     let drops:u32=l.drops.iter().filter(|(s,p,_)|ports.contains(&(*s,*p))).map(|x|x.2).sum();
     recv>=300&&gaps>=15&&gaps*1000>=(recv+gaps)*THRESHOLD&&p.iter().filter(|p|p.gaps>0).count()>=4&&median(&delays)<1500&&drops<10
   })
 },
 "capacity_pair"=>{
   let(a,b)=EDGE.unwrap();
   [(a,b),(b,a)].iter().any(|(a,b)|{
     let ps:Vec<_>=l.probes.iter().filter(|p|p.sw as u32==*a&&p.n as u32==*b&&p.pairs>=2&&p.pair_min>0).collect();
     let speeds:Vec<_>=ps.iter().map(|p|6_400_000u32/p.pair_min).collect();
     // An 800-byte second probe takes ~640 us at 10 Mbps and ~1600 us at 4 Mbps.
     // Require both the normal and reduced serialization regimes.
     speeds.iter().filter(|s|**s>=8000).count()>=3&&speeds.iter().filter(|s|**s<=THRESHOLD).count()>=3
   })
 },
 "incast_volume"=>{
   let all=series(&l);
   all.iter().any(|((sw,c),v)|{
     if *sw!=2{return false;}let Some((addr,len))=l.prefix.get(c)else{return false;};let mask=if *len==0{0}else{u32::MAX<<(32-*len)};
     let ps:Vec<_>=l.packets.iter().filter(|p|p.sw==2&&p.size==64&&p.proto==17&&(p.src&mask)==(*addr&mask)).collect();
     let mut destinations:BTreeMap<u32,Vec<&Packet>>=BTreeMap::new();for p in ps{destinations.entry(p.dst).or_default().push(p);}
     let mut counts:Vec<_>=v.iter().filter(|s|s.p>0).map(|s|s.p).collect();counts.sort_unstable();if counts.len()<8{return false;}let baseline=counts[counts.len()/4].max(1);
     let volume=v.windows(3).any(|w|w.iter().all(|s|s.p>=1000&&s.p as u64>=baseline as u64*10&&s.b as u64<=s.p as u64*100)&&w.windows(2).all(|x|x[1].t-x[0].t<=17));
     volume&&destinations.values().any(|ps|ps.iter().map(|p|p.src).collect::<BTreeSet<_>>().len()>=10&&ps.iter().map(|p|p.t).max().unwrap_or(0).saturating_sub(ps.iter().map(|p|p.t).min().unwrap_or(0))>=50)
   })
 },
 "mapper_ladder"=>{
   let mut m:BTreeMap<(u32,u32),BTreeMap<u8,usize>>=BTreeMap::new();
   for p in &l.packets{if p.sw==4&&p.size==64&&p.proto==17&&p.ttl<=15{*m.entry((p.src,p.dst)).or_default().entry(p.ttl).or_default()+=1;}}
   m.values().any(|h|h.values().filter(|n|**n>=3).count()>=THRESHOLD as usize && h.keys().min().copied().unwrap_or(255)<=1)
 },
 "probe_hub"=>{
   let active:Vec<_>=[0u8,6,10].iter().filter_map(|s|probe_intervals(&l,*s,5,THRESHOLD as u16).first().copied()).collect();
   active.len()==3&&active.iter().map(|x|x.1).min().unwrap().saturating_sub(active.iter().map(|x|x.0).max().unwrap())>=THRESHOLD as u16
 },
 "keepalive_only"=>{
   let(a,b)=EDGE.unwrap();
   intervals(&l,a as u8,b as u8,THRESHOLD as u16).iter().any(|i|{
     intervals(&l,b as u8,a as u8,THRESHOLD as u16).iter().any(|j|{
       let lo=i.0.max(j.0);let hi=i.1.min(j.1);
       hi.saturating_sub(lo)>=THRESHOLD as u16&&[(a,b),(b,a)].iter().all(|(a,b)|{
          let ps:Vec<_>=l.probes.iter().filter(|p|p.sw as u32==*a&&p.n as u32==*b&&p.t>=lo&&p.t<=hi).collect();
          ps.iter().filter(|p|p.count>0).count()>=3&&ps.iter().map(|p|p.count as u32).sum::<u32>()>=12
       })
     })
   })
 },
 "probe_recover"|"probe_coupled"=>{
   let edges=if MODE=="probe_coupled"{vec![(0,1),(1,0),(10,11),(11,10)]}else{let(a,b)=EDGE.unwrap();vec![(a as u8,b as u8),(b as u8,a as u8)]};
   let groups:Vec<_>=edges.iter().map(|(a,b)|probe_intervals(&l,*a,*b,THRESHOLD as u16)).collect();
   let mut common=groups.first().cloned().unwrap_or_default();
   for g in groups.iter().skip(1) {let mut next=vec![];for x in &common {for y in g {let t=x.0.max(y.0);let u=x.1.min(y.1);if u.saturating_sub(t)>=THRESHOLD as u16 {next.push((t,u));}}}common=next;}
   !common.is_empty()
 },

  "coupled"=>{
   let groups:Vec<_>=[(0,1),(1,0),(10,11),(11,10)].iter().map(|(a,b)|intervals(&l,*a,*b,THRESHOLD as u16)).collect();
   groups[0].iter().any(|a|groups[1].iter().any(|b|groups[2].iter().any(|c|groups[3].iter().any(|d|{let start=[a.0,b.0,c.0,d.0].into_iter().max().unwrap();let end=[a.1,b.1,c.1,d.1].into_iter().min().unwrap();end.saturating_sub(start)>=THRESHOLD as u16}))))
  },
  "recover"=>{let (a,b)=EDGE.unwrap();intervals(&l,a as u8,b as u8,THRESHOLD as u16).iter().any(|x|intervals(&l,b as u8,a as u8,THRESHOLD as u16).iter().any(|y|overlap(*x,*y,THRESHOLD as u16)))},
  "hub"=>{
   let mut lists=vec![];for a in [0,6,10]{let i=intervals(&l,a,5,THRESHOLD as u16);if !i.is_empty(){lists.push((a,i));}}
   // At least two independently recovered neighbors and a third neighbor silent in the same interval.
   for i in 0..lists.len(){for j in i+1..lists.len(){for x in &lists[i].1 {for y in &lists[j].1 {if overlap(*x,*y,THRESHOLD as u16) && [0,6,10].iter().all(|a|l.events.iter().any(|(s,n,t,tag)|s==a&&*n==5&&*tag==4&&*t>=x.0.max(y.0)&&*t<=x.1.min(y.1))){return true;}}}}}false
  },
  "congestion"=>{
   let summaries:Vec<_>=l.links.iter().filter(|s|target(s)).collect();
   let ports:BTreeSet<_>=summaries.iter().map(|s|(s.sw,s.port)).collect();
   let drops:u64=l.drops.iter().filter(|(s,p,_)|ports.contains(&(*s,*p))).map(|x|x.2 as u64).sum();
   drops>=THRESHOLD as u64 && summaries.iter().filter(|s|s.received>0&&s.peak>=10000).count()>=2
  },
  "loss"=>{
   let(a,b)=EDGE.unwrap();[ (a,b),(b,a) ].iter().all(|(a,b)|{
    let ss:Vec<_>=l.links.iter().filter(|s|s.sw as u32==*a&&s.n as u32==*b&&s.received>0).collect();
    ss.iter().filter(|s|s.gaps>0).count()>=2&&ss.iter().map(|s|s.gaps as u32).sum::<u32>()>=THRESHOLD&&median(&ss.iter().map(|s|s.avg).collect::<Vec<_>>())<=1500&&ss.iter().map(|s|s.peak).max().unwrap_or(0)<=5000
   })
  },
  "delaystep"=>{
   let(a,b)=EDGE.unwrap();let s:Vec<_>=l.links.iter().filter(|s|s.sw as u32==b&&s.n as u32==a&&s.received>0).collect();
   let rev:Vec<_>=l.links.iter().filter(|s|s.sw as u32==a&&s.n as u32==b&&s.received>0).map(|s|s.avg).collect();
   s.iter().filter(|s|s.avg>=THRESHOLD).count()>=3 && s.iter().any(|s|s.avg<THRESHOLD/2)&&rev.len()>=2&&median(&rev)<2000
  },
  "pulse"=>{
   let(a,b)=EDGE.unwrap();let s:Vec<_>=l.links.iter().filter(|s|s.sw as u32==b&&s.n as u32==a&&s.received>0).collect();
   s.iter().filter(|s|s.peak>=THRESHOLD&&s.avg<THRESHOLD).count()>=2 && l.links.iter().filter(|s|s.sw as u32==a&&s.n as u32==b&&s.received>0).count()>=2
  },
  "incast"=>{
   let packets:Vec<_>=l.packets.iter().filter(|p|p.sw==2&&p.size==64&&p.proto==17).collect();
   let mut dsts:BTreeMap<u32,Vec<&Packet>>=BTreeMap::new();for p in packets{dsts.entry(p.dst).or_default().push(p);}
   dsts.values().any(|ps| ps.iter().map(|p|p.src).collect::<BTreeSet<_>>().len()>=THRESHOLD as usize && ps.iter().map(|p|p.t).max().unwrap_or(0).saturating_sub(ps.iter().map(|p|p.t).min().unwrap_or(0))>=50)
  },
  "oddsize"=>{
   l.packets.iter().filter(|p|p.sw==2&&p.size==299&&p.proto==6).count()>=3&&series(&l).values().any(|v|v.len()>=6&&avg_size(v)==299)
  },
  "ttl"=>l.packets.iter().filter(|p|p.sw==4&&p.size==64&&p.proto==17&&p.ttl==1).map(|p|p.t).collect::<BTreeSet<_>>().len()>=2,
  "packet"=>l.packets.iter().filter(|p|p.sw==3&&p.size==500&&p.proto==6).count()>=2&&series(&l).values().any(|v|v.len()>=4&&avg_size(v)==500),
  "login_fanout"=>{
   let mut m:BTreeMap<u32,Vec<&Packet>>=BTreeMap::new();for p in &l.packets{if p.sw==8&&p.size==100&&p.proto==6 {m.entry(p.src).or_default().push(p);}}
   m.values().any(|v|v.iter().any(|p|v.iter().filter(|q|q.t>=p.t&&q.t<=p.t+50).map(|q|q.dst).collect::<BTreeSet<_>>().len()>=THRESHOLD as usize))
  },
  "reverse100"=>{
   let p:Vec<_>=l.packets.iter().filter(|p|p.sw==2&&p.size==100&&p.proto==6).collect();
   p.iter().map(|p|p.src).collect::<BTreeSet<_>>().len()>=THRESHOLD as usize && p.iter().map(|p|p.dst).collect::<BTreeSet<_>>().len()>=THRESHOLD as usize && series(&l).values().any(|v|v.len()>=6&&avg_size(v)==100)
  },
  "correlation"=>{
   let all=series(&l);let v:Vec<_>=all.values().filter(|v|v.len()>=4&&avg_size(v)==1000).collect();
   for i in 0..v.len(){for j in i+1..v.len(){let pairs:Vec<_>=v[i].iter().filter_map(|a|v[j].iter().find(|b|b.t==a.t&&a.p>0&&b.p>0).map(|b|(a.b as f64,b.b as f64))).collect();
    if pairs.len()<4{continue;}let n=pairs.len() as f64;let mx=pairs.iter().map(|p|p.0).sum::<f64>()/n;let my=pairs.iter().map(|p|p.1).sum::<f64>()/n;
    let vx=pairs.iter().map(|p|(p.0-mx).powi(2)).sum::<f64>();let vy=pairs.iter().map(|p|(p.1-my).powi(2)).sum::<f64>();let cov=pairs.iter().map(|p|(p.0-mx)*(p.1-my)).sum::<f64>();
    if mx>0.0&&my>0.0&&vx/n>mx*mx*0.0225&&vy/n>my*my*0.0225&&cov>0.0&&cov*cov>=0.9025*vx*vy{return true;}
   }}false
  },
  "surge"=>series(&l).values().any(|v|{
   if avg_size(v)!=1000{return false;}let med=median(&v.iter().filter(|s|s.p>0).map(|s|s.b).collect::<Vec<_>>());
   v.windows(3).any(|w|med>0&&w[1].b as u64>=med as u64*THRESHOLD as u64&&w[0].b>0&&w[2].b>0&&w[0].b<med*2&&w[2].b<med*2&&w[1].t-w[0].t<=17&&w[2].t-w[1].t<=17)
  }),
  "variability"=>series(&l).values().any(|v|{
   if avg_size(v)!=1000||v.len()<6{return false;}let b:Vec<_>=v.iter().filter(|s|s.p>0).map(|s|s.b).collect();
   median(&b)>=375000 && b.iter().max().copied().unwrap_or(0) as u64 >= b.iter().min().copied().unwrap_or(u32::MAX) as u64*THRESHOLD as u64
  }),
  "pause"|"pause_rate"=>series(&l).values().any(|v|{
   if avg_size(v)!=1000{return false;}let b=median(&v.iter().filter(|s|s.p>0).map(|s|s.b).collect::<Vec<_>>());
   (100000..=375000).contains(&b)&&!quiet_intervals(&l,v,THRESHOLD as u16).is_empty()
  }),
  "nested_pause"=>{
   let all=series(&l);
   all.iter().any(|((sw,c),v)|{
    let Some((addr,len))=l.prefix.get(c) else{return false;};if *len!=32||avg_size(v)!=THRESHOLD as u64{return false;}
    let parent=all.iter().find(|((s,pc),_)|s==sw&&l.prefix.get(pc).is_some_and(|(a,p)|*p==24&&(*a>>8)==(*addr>>8)));
    let Some((_,pv))=parent else{return false;};
    let med=median(&v.iter().filter(|s|s.p>0).map(|s|s.b).collect::<Vec<_>>());if med==0{return false;}
    // Require two consecutive observed low windows, followed by recovery; parent stays active in both.
    v.windows(4).any(|w|w[0].b>=med/2&&w[1].b<med*2/5&&w[2].b<med*2/5&&w[3].b>=med/2&&w.windows(2).all(|x|x[1].t-x[0].t<=17)&&[1,2].iter().all(|i|pv.iter().any(|s|s.t==w[*i].t&&s.p>0)))
   })
  },
  "pause_twins"|"pause_multi"=>{
   let all=series(&l);let want=if MODE=="pause_twins"{1000}else{200};let v:Vec<_>=all.values().filter(|v|avg_size(v)==want).collect();
   for i in 0..v.len(){for j in i+1..v.len(){if MODE=="pause_multi"&&v[i][0].sw==v[j][0].sw{continue;}for a in quiet_intervals(&l,v[i],THRESHOLD as u16){for b in quiet_intervals(&l,v[j],THRESHOLD as u16){if overlap(a,b,THRESHOLD as u16){return true;}}}}}false
  },
  "ping_imbalance"=>{
   let all=series(&l);for v in all.values().filter(|v|v[0].sw==1&&avg_size(v)==64){for gap in quiet_intervals(&l,v,THRESHOLD as u16){if all.values().any(|p|p[0].sw==2&&avg_size(p)==64&&p.iter().filter(|s|s.t>gap.0&&s.t<=gap.1&&s.p>0).count()>=3){return true;}}}false
  },
  _=>false,
 }
}
switch_program_sdk::detect_program!(detect);
