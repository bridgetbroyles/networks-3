use std::collections::{BTreeMap,HashSet};
use switch_program_sdk::*;
const LOGGER:u32=11;
const REPORT:u8=252;
const PROBE:u8=251;
const BASE:u32=0xa3f7_0000;
const REPORT_IP:u32=0xe000_00fc;
const TARGET_TABLE:u32=1;
const LAST_SEQ:u32=1;
fn selected(s:u32)->bool{s==11||s==12}
fn peer(s:u32)->u32{if s==11{12}else{11}}
fn u16p(b:&mut Vec<u8>,n:u16){b.extend_from_slice(&n.to_le_bytes());}
fn u32p(b:&mut Vec<u8>,n:u32){b.extend_from_slice(&n.to_le_bytes());}
fn frame(body:Vec<u8>)->Vec<u8>{let mut b=vec![21,body.len() as u8];b.extend(body);b}
fn stage(lines:&[&str],alu:u32,mem:u32)->String{
 let mut labels=BTreeMap::new();let mut instructions=vec![];
 for line in lines{if let Some(label)=line.strip_suffix(':'){labels.insert(label,instructions.len());}else{instructions.push(line.to_string());}}
 for line in &mut instructions{if let Some((head,label))=line.split_once(" -> "){if let Some(target)=labels.get(label){*line=format!("{} -> {}",head,target);}}}
 format!("stage alu={} mem={}\n{}\n",alu,mem,instructions.join("\n"))
}
struct Link{n:u32,last:u64,count:u16,sum:u64}
pub struct LossObserver{
 me:u32,started:bool,links:BTreeMap<u16,Link>,drops:BTreeMap<u16,u64>,
 sent:u32,credit_ns:u64,last_tick:u64,last_emit:u64,report_seq:u32,seen:HashSet<(u32,u32)>
}
impl LossObserver {
 fn target_port(&self)->Option<u16>{self.links.iter().find(|(_,l)|l.n==peer(self.me)).map(|(p,_)|*p)}
 fn flood(&self,data:&[u8],skip:Option<u16>,now:u64,out:&mut Vec<Action>){
  for (p,l) in &self.links{
   if Some(*p)==skip||now.saturating_sub(l.last)>2_500_000_000{continue;}
   out.push(actions::inject_packet(*p,REPORT_IP,REPORT_IP,REPORT,1,data.to_vec()));
  }
 }
 fn emit(&mut self,body:Vec<u8>,now:u64,out:&mut Vec<Action>){
  if self.me==LOGGER{out.push(actions::log_write(frame(body)));return;}
  self.report_seq+=1;self.seen.insert((self.me,self.report_seq));
  let mut data=vec![];u32p(&mut data,self.me);u32p(&mut data,self.report_seq);data.extend(frame(body));self.flood(&data,None,now,out);
 }
}
impl SwitchProgram for LossObserver{
 fn init(_:u32,_:Vec<u16>)->(Self,ProgramSetup){panic!("init_with is required")}
 fn init_with(input:InitInput)->(Self,ProgramSetup){
  let mut setup=ProgramSetup::new();
  setup.declare_table(TARGET_TABLE,MatchKind::Exact,16);
  setup.declare_register_array(LAST_SEQ,64);
  for id in 1..=3{setup.declare_counter_array(id,64);}
  let mut code=stage(&[
   "load r0, ip_proto","const r1, 252","eq r2, r0, r1","bif r2 -> report",
   "load r0, kind","const r1, 5","eq r2, r0, r1","bif r2 -> keepalive",
   "load r0, ip_proto","const r1, 251","eq r2, r0, r1","bif r2 -> probe","drop",
   "report:","punt 1","keepalive:","punt 2","probe:","noop"
  ],16,0);
  code+=&stage(&[
   "load r0, ingress_port","table t1, r0 -> m0","loadm r1, m1","load r2, ip_src",
   "eq r3, r1, r2","bif r3 -> valid","drop","valid:","noop"
  ],8,1);
  code+=&stage(&[
   "load r0, ingress_port","load r1, ip_dst","rread a1[r0] -> r2","lt r3, r2, r1",
   "bif r3 -> fresh","drop","fresh:","rwrite a1[r0], r1",
   "const r3, 0","eq r4, r2, r3","bif r4 -> first",
   "sub r4, r1, r2","const r3, 1","sub r4, r4, r3","storem m2, r4",
   "const r3, 1","bif r3 -> done","first:","const r3, 0","storem m2, r3","done:","noop"
  ],24,2);
  code+=&stage(&["load r0, ingress_port","const r1, 1","counter c1[r0], r1"],4,1);
  code+=&stage(&["load r0, ingress_port","loadm r1, m2","counter c2[r0], r1"],4,1);
  code+=&stage(&[
   "load r0, ingress_port","loadm r1, m2","const r2, 8","lt r3, r1, r2",
   "bif r3 -> done","const r1, 1","counter c3[r0], r1","done:","drop"
  ],8,1);
  // Probes end in Drop after updating counters: they never spend a CPU punt.
  // Under the harness a dropped customer observation copy leaves its original packet unaffected.
  setup.set_program(text::parse_tiny_program(&code).expect("static loss pipeline"));
  (Self{me:input.switch_id,started:false,links:BTreeMap::new(),drops:BTreeMap::new(),sent:0,
    credit_ns:0,last_tick:0,last_emit:0,report_seq:0,seen:HashSet::new()},setup)
 }
 fn on_punt(&mut self,ev:PuntEvent)->Vec<Action>{
  let mut out=vec![];
  if ev.ip_proto==REPORT {
   let b=&ev.payload;if b.len()<10{return out;}
   let origin=u32::from_le_bytes(b[0..4].try_into().unwrap());let seq=u32::from_le_bytes(b[4..8].try_into().unwrap());
   if !self.seen.insert((origin,seq)){return out;}
   if self.me==LOGGER{out.push(actions::log_write(b[8..].to_vec()));}
   else{self.flood(b,Some(ev.ingress_port),ev.now_ns,&mut out);}
   return out;
  }
  if let Some(k)=keepalive(&ev.payload){
   let fresh=!self.links.contains_key(&ev.ingress_port);
   let l=self.links.entry(ev.ingress_port).or_insert(Link{n:k.switch,last:0,count:0,sum:0});
   l.n=k.switch;l.last=ev.arrived_ns;l.count=l.count.saturating_add(1);
   l.sum+=ev.arrived_ns.saturating_sub(k.sent_ns)/1000;
   if fresh&&selected(self.me)&&k.switch==peer(self.me){
    out.push(actions::install_entry(TARGET_TABLE,TableEntry{
     id:wire::EntryIdW(ev.ingress_port as u64+1),key:ev.ingress_port as u64,prefix_len:64,priority:0,
     action:TableAction::SetMeta{key:wire::MetaKeyW(1),value:(BASE|k.switch) as u64}
    }));
   }
  }
  out
 }
 fn on_timer(&mut self,ev:TimerEvent)->Vec<Action>{
  let mut out=vec![];
  if !self.started{self.started=true;if self.me==LOGGER{out.push(actions::name_logging_switch());}}
  let elapsed=ev.now_ns.saturating_sub(self.last_tick);self.last_tick=ev.now_ns;
  for d in &ev.port_drops{*self.drops.entry(d.port).or_default()+=d.dropped as u64;}
  if selected(self.me){
   if ev.now_ns>=1_000_000_000{
    // At most two 40-byte probes per callback; 250/s is 10 KB/s, leaving export headroom.
    self.credit_ns=self.credit_ns.saturating_add(elapsed).min(8_000_000);
    if let Some(port)=self.target_port(){
     for _ in 0..2{
      if self.credit_ns<4_000_000{break;}
      self.credit_ns-=4_000_000;self.sent+=1;
      out.push(actions::inject_packet(port,BASE|self.me,self.sent,PROBE,1,vec![]));
     }
    }
   }else{self.credit_ns=0;}
   if ev.now_ns.saturating_sub(self.last_emit)>=1_000_000_000{
    self.last_emit=ev.now_ns;
    let port=self.target_port();
    let counter=|id:u32|->u32{
     let total=ev.counters.iter().find(|c|c.id.0==id).and_then(|c|port.and_then(|p|c.values.get(p as usize))).copied().unwrap_or(0);
     total.min(u32::MAX as u64) as u32
    };
    let mut body=vec![self.me as u8,peer(self.me) as u8];u32p(&mut body,(ev.now_ns/1_000_000) as u32);
    u32p(&mut body,self.sent);u32p(&mut body,counter(1));u32p(&mut body,counter(2));u32p(&mut body,counter(3));
    u32p(&mut body,port.and_then(|p|self.drops.get(&p)).copied().unwrap_or(0).min(u32::MAX as u64) as u32);
    let (count,mean,quiet)=if let Some(l)=port.and_then(|p|self.links.get_mut(&p)){
     let count=l.count;let mean=if count>0{(l.sum/count as u64) as u32}else{0};
     let quiet=ev.now_ns.saturating_sub(l.last)>=2_000_000_000;l.count=0;l.sum=0;(count,mean,quiet)
    }else{(0,0,true)};
    u16p(&mut body,count);u32p(&mut body,mean);body.push(quiet as u8);
    self.emit(body,ev.now_ns,&mut out);
   }
  }
  out.push(actions::schedule_timer(if selected(self.me){5_000_000}else{500_000_000}));
  out
 }
}
switch_program!(LossObserver);
