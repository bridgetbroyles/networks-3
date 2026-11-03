use std::collections::BTreeMap;
#[derive(Clone,Debug)]
struct Frame{ms:u32,tx:u32,rx:u32,gaps:u32,big:u32,drops:u32}
fn u32b(b:&[u8],i:usize)->u32{u32::from_le_bytes(b[i..i+4].try_into().unwrap())}
fn parse(bytes:&[u8])->Option<BTreeMap<u8,Vec<Frame>>>{
 let mut out:BTreeMap<u8,Vec<Frame>>=BTreeMap::new();let mut at=0;
 while at<bytes.len(){
  let tag=*bytes.get(at)?;let len=*bytes.get(at+1)? as usize;at+=2;let b=bytes.get(at..at+len)?;at+=len;
  if tag!=21{continue;}
  if len!=33{return None;}
  if !((b[0]==11&&b[1]==12)||(b[0]==12&&b[1]==11)){continue;}
  out.entry(b[0]).or_default().push(Frame{ms:u32b(b,2),tx:u32b(b,6),rx:u32b(b,10),gaps:u32b(b,14),big:u32b(b,18),drops:u32b(b,22)});
 }
 for fs in out.values_mut(){fs.sort_by_key(|f|f.ms);fs.dedup_by_key(|f|f.ms);}
 Some(out)
}
fn nearest(fs:&[Frame],ms:u32)->Option<usize>{fs.iter().enumerate().min_by_key(|(_,f)|f.ms.abs_diff(ms)).and_then(|(i,f)|if f.ms.abs_diff(ms)<=100{Some(i)}else{None})}
fn diff(a:u32,b:u32)->Option<u64>{b.checked_sub(a).map(|n|n as u64)}
fn evidence(recv:&[Frame],sender:&[Frame])->bool{
 if recv.len()<8||sender.len()<8{return false;}
 if recv.windows(2).any(|w|w[1].ms-w[0].ms>2200)||sender.windows(2).any(|w|w[1].ms-w[0].ms>2200){return false;}
 let a=&recv[0];let b=recv.last().unwrap();let sa=&sender[0];let sb=sender.last().unwrap();
 if b.ms.saturating_sub(a.ms)<8000{return false;}
 let Some(rx)=diff(a.rx,b.rx)else{return false;};
 let Some(gaps)=diff(a.gaps,b.gaps)else{return false;};
 let Some(big)=diff(a.big,b.big)else{return false;};
 let Some(drops)=diff(sa.drops,sb.drops)else{return false;};
 let Some(sent)=diff(sa.tx,sb.tx)else{return false;};
 if big>0||drops>0{return false;}
 if sender.windows(2).any(|w|w[1].tx<w[0].tx||w[1].drops<w[0].drops){return false;}
 let n=rx+gaps;
 if n<1500||n>100_000||sent>100_000||sent.abs_diff(n)>64{return false;}
 // Three-sigma margin above a 2.5% null, in integer arithmetic.
 let Some(excess)=(1000*gaps).checked_sub(25*n)else{return false;};
 if excess*excess<9*25*975*n{return false;}
 let mut epochs=vec![];
 for w in recv.windows(2){
  if w[1].gaps<w[0].gaps||w[1].rx<w[0].rx||w[1].big<w[0].big{return false;}
  if w[1].gaps>w[0].gaps{epochs.push(w[1].ms);}
 }
 epochs.len()>=3&&epochs.last().unwrap().saturating_sub(epochs[0])>=4000
}
fn detect(bytes:&[u8])->bool{
 let Some(mut data)=parse(bytes)else{return false;};
 let Some(a)=data.remove(&11)else{return false;};let Some(b)=data.remove(&12)else{return false;};
 for end in 1..a.len(){
  let wanted=a[end].ms.saturating_sub(10_000);
  let Some(start)=a.iter().position(|f|f.ms>=wanted)else{continue;};
  if start>=end||a[end].ms-a[start].ms<8000{continue;}
  let Some(bs)=nearest(&b,a[start].ms)else{continue;};let Some(be)=nearest(&b,a[end].ms)else{continue;};
  if bs>=be{continue;}
  if evidence(&a[start..=end],&b[bs..=be])&&evidence(&b[bs..=be],&a[start..=end]){return true;}
 }
 false
}
switch_program_sdk::detect_program!(detect);
