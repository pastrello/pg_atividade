use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::PathBuf};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub id: Uuid,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    pub tls: bool,
    pub ca_file: String,
    pub interval: u64,
    pub remember: bool,
}
impl Default for Profile {
    fn default() -> Self {
        Self { id: Uuid::new_v4(), name: "PostgreSQL".into(), host: "localhost".into(), port: 5432,
            database: "postgres".into(), user: "postgres".into(), tls: true, ca_file: String::new(), interval: 2, remember: false }
    }
}
impl Profile {
    pub fn validate(&self) -> Result<(), String> {
        if [&self.name, &self.host, &self.database, &self.user].iter().any(|v| v.trim().is_empty()) || self.port == 0 {
            return Err("Preencha nome, host, porta, banco e usuário.".into());
        }
        if !(1..=60).contains(&self.interval) { return Err("Intervalo deve ser de 1 a 60 segundos.".into()); }
        if self.host.contains(',') || self.host.contains('/') || self.host.contains('\\') { return Err("Informe um único host DNS ou IP.".into()); }
        Ok(())
    }
}
#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Profiles { profiles: Vec<Profile> }
pub fn data_dir() -> Result<PathBuf, String> {
    let root = std::env::var_os("LOCALAPPDATA").or_else(|| std::env::var_os("XDG_DATA_HOME"))
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/share").into_os_string()))
        .ok_or("Diretório de dados do usuário não localizado")?;
    let path = PathBuf::from(root).join("PGActivity-RS");
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    Ok(path)
}
pub fn load_profiles() -> Result<Vec<Profile>, String> {
    let path = data_dir()?.join("profiles.toml");
    let contents = match std::fs::read_to_string(path) {
        Ok(s) => s, Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]), Err(e) => return Err(e.to_string())
    };
    let p: Profiles = toml::from_str(&contents).map_err(|_| "Arquivo de perfis inválido; não será sobrescrito nesta sessão.".to_string())?;
    let mut ids = BTreeSet::new();
    for v in &p.profiles { v.validate()?; if !ids.insert(v.id) { return Err("ID duplicado no arquivo de perfis".into()); } }
    Ok(p.profiles)
}
pub fn save_profiles(profiles: &[Profile]) -> Result<(), String> {
    let path = data_dir()?.join("profiles.toml");
    let tmp = path.with_extension("toml.tmp");
    let contents = toml::to_string_pretty(&Profiles { profiles: profiles.to_vec() }).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, contents).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
}
#[cfg(windows)]
fn entry(p: &Profile) -> Result<keyring::Entry, String> {
    // Bind the credential to the endpoint as well as the immutable profile ID.
    let target = format!("{}:{}:{}:{}:{}:{}:{}", p.id, p.host, p.port, p.database, p.user, p.tls, p.ca_file);
    keyring::Entry::new("PGActivity-RS", &target).map_err(|_| "Falha ao acessar Credential Manager".into())
}
#[cfg(windows)]
pub fn get_password(p: &Profile) -> Result<String, String> { entry(p)?.get_password().map_err(|_| "Credencial não disponível; informe a senha.".into()) }
#[cfg(windows)]
pub fn store_password(p: &Profile, password: &str) -> Result<(), String> { entry(p)?.set_password(password).map_err(|_| "Não foi possível salvar a credencial.".into()) }
#[cfg(windows)]
pub fn delete_password(p: &Profile) -> Result<(), String> {
    match entry(p)?.delete_credential() { Ok(()) | Err(keyring::Error::NoEntry) => Ok(()), Err(_) => Err("Falha ao excluir a credencial.".into()) }
}
#[cfg(not(windows))]
pub fn get_password(_: &Profile) -> Result<String, String> { Err("Credenciais persistentes disponíveis somente no Windows.".into()) }
#[cfg(not(windows))]
pub fn store_password(_: &Profile, _: &str) -> Result<(), String> { Err("Credenciais persistentes disponíveis somente no Windows.".into()) }
#[cfg(not(windows))]
pub fn delete_password(_: &Profile) -> Result<(), String> { Ok(()) }

#[derive(Clone, Default)]
pub struct Identity { pub version: String, pub version_num: i32, pub user: String, pub superuser: bool, pub read_all: bool, pub can_signal: bool, pub replica: bool, pub instance: String }
#[derive(Clone, Default)]
pub struct Session {
    pub pid: i32, pub db: String, pub user: String, pub app: String, pub ip: String, pub state: String,
    pub wait_type: String, pub wait: String, pub kind: String, pub started: String, pub query_started: String,
    pub query_age: Option<f64>, pub xact_age: Option<f64>, pub state_age: Option<f64>, pub query: String, pub blockers: Vec<i32>,
}
#[derive(Debug, PartialEq, Eq)]
pub enum QueryAgeBand { Normal, Green, Yellow, Red }
impl Session {
    pub fn is_slow_query(&self)->bool {self.state=="active"&&self.kind=="client backend"&&self.query_age.is_some_and(|t|t.is_finite()&&t>1.0)}
    pub fn query_age_band(&self) -> QueryAgeBand {
        use QueryAgeBand::*;
        if self.state!="active" || self.kind!="client backend" {return Normal;}
        match self.query_age {
            Some(t) if t.is_finite() && t>=10.0 => Red,
            Some(t) if t.is_finite() && t>=5.0 => Yellow,
            Some(t) if t.is_finite() && t>=2.0 => Green,
            _=>Normal
        }
    }
    pub fn key(&self) -> (i32, String) { (self.pid, self.started.clone()) }
    pub fn matches(&self, filter: &str) -> bool {
        let f = filter.trim().to_lowercase();
        f.is_empty() || self.pid.to_string().contains(&f) || [&self.db,&self.user,&self.app,&self.ip,&self.state,&self.wait_type,&self.wait,&self.kind,&self.query].iter().any(|v| v.to_lowercase().contains(&f))
    }
    pub fn status(&self) -> &'static str {
        if self.wait_type == "Lock" { "LOCK" } else if self.state.starts_with("idle in transaction") { "IDLE IN TX" }
        else if self.wait_type == "IO" { "I/O" } else if self.kind == "autovacuum worker" { "VACUUM" }
        else if self.state == "active" && self.wait_type.is_empty() { "ACTIVE" }
        else if self.state == "active" { "WAIT" } else if self.state == "idle" { "IDLE" } else { "OTHER" }
    }
}
#[derive(Clone, Default)]
pub struct Snapshot { pub sessions: Vec<Session>, pub truncated: bool }
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Action { Cancel, Terminate }
impl Action { pub fn label(self) -> &'static str { match self { Self::Cancel => "Cancelar consulta", Self::Terminate => "Encerrar sessão" } } }

// Traverse all blocker edges, preserving missing/external PIDs and avoiding cycles.
pub fn blocker_chain(sessions: &[Session], target: i32) -> Vec<(i32,i32)> {
    let mut todo = vec![target]; let mut visited = BTreeSet::new(); let mut edges = BTreeSet::new();
    while let Some(pid) = todo.pop() {
        if !visited.insert(pid) { continue; }
        if let Some(s) = sessions.iter().find(|s| s.pid == pid) {
            for &b in &s.blockers { edges.insert((b,pid)); todo.push(b); }
        }
    }
    edges.into_iter().collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn slow_query_strict_threshold(){
        let mut s=Session{state:"active".into(),kind:"client backend".into(),query_age:Some(1.0),..Default::default()};
        assert!(!s.is_slow_query());s.query_age=Some(1.001);assert!(s.is_slow_query());
        s.state="idle in transaction".into();assert!(!s.is_slow_query());
        s.state="active".into();s.kind="walsender".into();assert!(!s.is_slow_query());
        s.kind="client backend".into();s.query_age=None;assert!(!s.is_slow_query());
    }
    #[test] fn query_age_bands_exact_boundaries() {
        use QueryAgeBand::*;
        for (t,want) in [(0.0,Normal),(1.999,Normal),(2.0,Green),(4.999,Green),(5.0,Yellow),(9.999,Yellow),(10.0,Red),(3600.0,Red),(f64::NAN,Normal),(f64::INFINITY,Normal)] {
            let s=Session{state:"active".into(),kind:"client backend".into(),query_age:Some(t),..Default::default()};
            assert_eq!(s.query_age_band(),want);
        }
    }
    #[test] fn query_age_excludes_idle_and_replication() {
        use QueryAgeBand::*;
        for (state,kind) in [("idle","client backend"),("idle in transaction","client backend"),("active","walsender"),("active","autovacuum worker")]{
            let s=Session{state:state.into(),kind:kind.into(),query_age:Some(1000.0),..Default::default()};
            assert_eq!(s.query_age_band(),Normal);
        }
        let s=Session{state:"active".into(),kind:"client backend".into(),query_age:None,..Default::default()};
        assert_eq!(s.query_age_band(),Normal);
    }
    #[test] fn profile_has_no_password() { let s = toml::to_string(&Profile::default()).unwrap(); assert!(!s.contains("password")); }
    #[test] fn profile_rejects_multi_host() { let mut p = Profile::default(); p.host = "a,b".into(); assert!(p.validate().is_err()); }
    #[test] fn lock_has_priority() { let s = Session { state: "active".into(), wait_type: "Lock".into(), ..Default::default() }; assert_eq!(s.status(), "LOCK"); }
    #[test] fn chain_handles_cycle_and_missing() { let rows = vec![Session {pid: 1, blockers: vec![2,0], ..Default::default()},Session{pid:2,blockers:vec![1],..Default::default()}]; assert_eq!(blocker_chain(&rows,1),vec![(0,1),(1,2),(2,1)]); }
    #[test] fn filter_is_case_insensitive() { let s = Session { query:"SELECT ABC".into(), ..Default::default() }; assert!(s.matches("abc")); assert!(!s.matches("xyz")); }
}
