//! Native PostgreSQL counters only. No OS, agent, extension or configuration writes.
use std::{collections::BTreeMap, time::{Duration, Instant}};
use tokio_postgres::{Client, Row};
const DB: &str = include_str!("../sql/metrics_db.sql");
const WAL: &str = include_str!("../sql/metrics_wal.sql");
const IO16: &str = include_str!("../sql/metrics_io16.sql");
const IO18: &str = include_str!("../sql/metrics_io18.sql");
const PID18: &str = include_str!("../sql/metrics_pid18.sql");

#[derive(Clone, Debug, PartialEq)]
struct Counter { reset: String, n: [u128;7] }
type Counters = BTreeMap<String,Counter>;
type Rate = [f64;7];
#[derive(Default)]
struct Baseline { previous: Option<(Instant,Counters)> }
impl Baseline {
    fn clear(&mut self){ self.previous=None; }
    fn sample(&mut self, now:Instant, next:Counters, complete:bool)->BTreeMap<String,Rate>{
        let old=self.previous.replace((now,next.clone()));
        let Some((then,old))=old else{return BTreeMap::new();};
        let secs=now.saturating_duration_since(then).as_secs_f64();
        if !(0.25..=120.0).contains(&secs) || (complete && (old.len()!=next.len() || !old.keys().eq(next.keys()))) {
            return BTreeMap::new();
        }
        let mut rates=BTreeMap::new();
        for (key,new) in &next {
            let Some(prev)=old.get(key) else{continue;};
            if new.reset!=prev.reset || (0..7).any(|i|new.n[i]<prev.n[i]) {continue;}
            let r=std::array::from_fn(|i|(new.n[i]-prev.n[i]) as f64/secs);
            rates.insert(key.clone(),r);
        }
        if complete && rates.len()!=next.len(){rates.clear();}
        rates
    }
}
fn total(rates:&BTreeMap<String,Rate>)->Option<Rate>{
    if rates.is_empty(){return None;}
    let mut r=[0.0;7];
    for v in rates.values(){for i in 0..7{r[i]+=v[i];}}
    Some(r)
}
fn decode(row:&Row)->Result<(String,Counter),String>{
    let get=|name:&str| row.try_get::<_,String>(name).map_err(|_|"Coluna de métricas ausente ou inválida".to_string());
    let mut n=[0;7];
    for (i,name) in ["a","b","c","d","e","f","g"].iter().enumerate(){
        n[i]=get(name)?.parse::<u128>().map_err(|_|"Contador de métricas inválido".to_string())?;
    }
    Ok((get("key")?,Counter{reset:get("reset")?,n}))
}
async fn read(c:&Client,sql:&str)->Result<(Instant,Counters),String>{
    let rows=tokio::time::timeout(Duration::from_secs(7),c.query(sql,&[])).await
        .map_err(|_|"Timeout nas métricas".to_string())?
        .map_err(|e|e.as_db_error().map(|d|format!("SQLSTATE {}",d.code().code())).unwrap_or_else(||"Falha na coleta SQL".into()))?;
    let values=rows.iter().map(decode).collect::<Result<Counters,_>>()?;
    Ok((Instant::now(),values))
}
async fn update(c:&Client,sql:&str,b:&mut Baseline,complete:bool)->Result<BTreeMap<String,Rate>,String>{
    match read(c,sql).await{
        Ok((t,rows))=>Ok(b.sample(t,rows,complete)),
        Err(e)=>{b.clear();Err(e)}
    }
}
#[derive(Clone,Default)]
pub struct Metrics{
    pub db:Option<Rate>, pub io:Option<Rate>, pub wal:Option<Rate>,
    pub pid:BTreeMap<String,Rate>, pub version:i32, pub enabled:bool,
    pub errors:Vec<String>, pub sampled:Option<Instant>,
}
impl Metrics{
    pub fn pid_rate(&self,pid:i32,started:&str)->Option<&Rate>{self.pid.get(&format!("{pid}|{started}"))}
}
#[derive(Default)]
pub struct Collector { db:Baseline, io:Baseline, wal:Baseline, pid:Baseline }
impl Collector {
    pub async fn collect(&mut self,c:&Client,version:i32,read_all:bool)->Metrics{
        let mut out=Metrics{version,enabled:true,..Default::default()};
        match update(c,DB,&mut self.db,true).await{Ok(r)=>out.db=total(&r),Err(e)=>out.errors.push(format!("Banco: {e}"))}
        if version>=140000{
            match update(c,WAL,&mut self.wal,true).await{Ok(r)=>out.wal=total(&r),Err(e)=>out.errors.push(format!("WAL: {e}"))}
        }
        if version>=160000{
            match update(c,if version>=180000{IO18}else{IO16},&mut self.io,true).await{Ok(r)=>out.io=total(&r),Err(e)=>out.errors.push(format!("I/O: {e}"))}
        }
        if version>=180000 && read_all{
            match update(c,PID18,&mut self.pid,false).await{Ok(r)=>out.pid=r,Err(e)=>out.errors.push(format!("I/O PID: {e}"))}
        }
        out.sampled=Some(Instant::now());out
    }
}
#[cfg(test)]
mod tests{
    use super::*;
    fn rows(n:u128,reset:&str)->Counters{
        BTreeMap::from([("x".to_owned(),Counter{reset:reset.into(),n:[n;7]})])
    }
    #[test] fn actual_interval_and_first_sample(){
        let t=Instant::now();let mut b=Baseline::default();
        assert!(b.sample(t,rows(100,"a"),true).is_empty());
        let r=b.sample(t+Duration::from_secs(4),rows(300,"a"),true);
        assert_eq!(total(&r).unwrap()[0],50.0);
    }
    #[test] fn resets_and_regressions_are_not_rates(){
        let t=Instant::now();let mut b=Baseline::default();
        b.sample(t,rows(100,"a"),true);
        assert!(b.sample(t+Duration::from_secs(2),rows(200,"b"),true).is_empty());
        assert!(b.sample(t+Duration::from_secs(4),rows(100,"b"),true).is_empty());
        assert!(b.sample(t+Duration::from_secs(200),rows(200,"b"),true).is_empty());
    }
    #[test] fn changed_membership_invalidates_aggregate_not_other_pids(){
        let t=Instant::now();let mut b=Baseline::default();b.sample(t,rows(100,"a"),true);
        let mut n=rows(200,"a");n.insert("y".into(),Counter{reset:"a".into(),n:[1;7]});
        assert!(b.sample(t+Duration::from_secs(2),n.clone(),true).is_empty());
        let mut b=Baseline::default();b.sample(t,rows(100,"a"),false);
        let r=b.sample(t+Duration::from_secs(2),n,false);
        assert!(r.contains_key("x"));assert!(!r.contains_key("y"));
    }
}
