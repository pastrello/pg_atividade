use crate::model::{self, Action, Identity, Profile, Session, Snapshot};
use std::{io::Write, sync::{Arc, Mutex}, time::{Duration, Instant, SystemTime, UNIX_EPOCH}};
use tokio::{task::JoinHandle, time::{sleep, timeout}};
use tokio_postgres::{Client, Config, NoTls, Row, config::SslMode};
use zeroize::Zeroizing;

pub type Secret = Arc<Zeroizing<String>>;
#[derive(Clone, Default)]
pub struct State {
    pub identity: Option<Identity>, pub snapshot: Option<Arc<Snapshot>>, pub last: Option<Instant>,
    pub online: bool, pub message: String,
}
pub type Shared = Arc<Mutex<State>>;
struct Connection { client: Client, driver: JoinHandle<()> }
impl Drop for Connection { fn drop(&mut self) { self.driver.abort(); } }
fn pg_error(e: tokio_postgres::Error) -> String {
    if let Some(db) = e.as_db_error() { format!("PostgreSQL SQLSTATE {}", db.code().code()) }
    else { "Falha de conexão, autenticação, TLS ou protocolo. Verifique host, porta, senha e certificado.".into() }
}
async fn connect(p: &Profile, secret: &Secret, admin: bool) -> Result<Connection,String> {
    p.validate()?;
    let mut cfg = Config::new();
    cfg.host(&p.host).port(p.port).dbname(&p.database).user(&p.user).password(secret.as_bytes())
       .application_name(if admin { "PGActivity-RS admin" } else { "PGActivity-RS monitor" })
       .connect_timeout(Duration::from_secs(5))
       .options(if admin { "-c statement_timeout=5000 -c lock_timeout=1000 -c timezone=UTC -c search_path=pg_catalog -c DateStyle=ISO,YMD" }
                else { "-c default_transaction_read_only=on -c statement_timeout=5000 -c lock_timeout=1000 -c timezone=UTC -c search_path=pg_catalog -c DateStyle=ISO,YMD" });
    if p.tls {
        cfg.ssl_mode(SslMode::Require);
        let mut roots = rustls::RootCertStore::empty();
        let native = rustls_native_certs::load_native_certs();
        roots.add_parsable_certificates(native.certs);
        if !p.ca_file.trim().is_empty() {
            let file = std::fs::File::open(&p.ca_file).map_err(|_| "Não foi possível abrir a CA PEM")?;
            let mut reader = std::io::BufReader::new(file);
            let mut count = 0;
            for cert in rustls_pemfile::certs(&mut reader) {
                roots.add(cert.map_err(|_| "CA PEM inválida")?).map_err(|_| "Certificado CA inválido")?; count += 1;
            }
            if count == 0 { return Err("Arquivo CA não contém certificados".into()); }
        }
        if roots.is_empty() { return Err("Nenhuma CA confiável encontrada".into()); }
        let tls = rustls::ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions().map_err(|_| "Configuração TLS inválida")?
            .with_root_certificates(roots).with_no_client_auth();
        let (client, driver) = timeout(Duration::from_secs(10), cfg.connect(tokio_postgres_rustls::MakeRustlsConnect::new(tls)))
            .await.map_err(|_| "Timeout de conexão TLS")?.map_err(pg_error)?;
        Ok(Connection {client, driver: tokio::spawn(async move { let _ = driver.await; })})
    } else {
        cfg.ssl_mode(SslMode::Disable);
        let (client, driver) = timeout(Duration::from_secs(10), cfg.connect(NoTls)).await.map_err(|_| "Timeout de conexão")?.map_err(pg_error)?;
        Ok(Connection {client, driver: tokio::spawn(async move { let _ = driver.await; })})
    }
}
async fn identity(c: &Client) -> Result<Identity,String> {
    let row = timeout(Duration::from_secs(7), c.query_one(include_str!("../sql/identity.sql"), &[])).await.map_err(|_| "Timeout de identificação")?.map_err(pg_error)?;
    Ok(Identity { version: row.try_get("version").map_err(pg_error)?, version_num: row.try_get("version_num").map_err(pg_error)?,
        user: row.try_get("usr").map_err(pg_error)?, superuser: row.try_get("superuser").map_err(pg_error)?,
        read_all: row.try_get("read_all_stats").map_err(pg_error)?, can_signal: row.try_get("can_signal").map_err(pg_error)?, replica: row.try_get("replica").map_err(pg_error)?, instance: row.try_get("instance").map_err(pg_error)? })
}
fn session(r: Row) -> Result<Session, tokio_postgres::Error> {
    Ok(Session { pid:r.try_get("pid")?, db:r.try_get("db")?, user:r.try_get("usr")?, app:r.try_get("app")?, ip:r.try_get("ip")?,
        state:r.try_get("state")?, wait_type:r.try_get("wait_type")?, wait:r.try_get("wait_event")?, kind:r.try_get("backend_type")?,
        started:r.try_get("started")?, query_started:r.try_get("query_started")?, query_age:r.try_get("query_age")?, xact_age:r.try_get("xact_age")?, state_age:r.try_get("state_age")?, query:r.try_get("query")?, blockers:r.try_get("blockers")? })
}
async fn collect(c: &Client) -> Result<Snapshot,String> {
    let rows = timeout(Duration::from_secs(7), c.query(include_str!("../sql/activity.sql"), &[])).await.map_err(|_| "Timeout de coleta")?.map_err(pg_error)?;
    let truncated = rows.len() > 5000;
    let sessions = rows.into_iter().take(5000).map(session).collect::<Result<Vec<_>,_>>().map_err(pg_error)?;
    Ok(Snapshot {sessions, truncated})
}
pub async fn run(p: Profile, secret: Secret, state: Shared, wake: Arc<tokio::sync::Notify>, ctx: eframe::egui::Context) {
    let mut backoff = 2;
    loop {
        { let mut s = state.lock().unwrap(); s.online = false; s.message = "Conectando...".into(); }
        ctx.request_repaint();
        let failure = match connect(&p,&secret,false).await {
            Err(e) => e,
            Ok(conn) => match identity(&conn.client).await {
                Err(e) => e,
                Ok(id) if id.version_num < 130000 => "PostgreSQL anterior à versão 13 não suportado".into(),
                Ok(id) => {
                    { let mut s = state.lock().unwrap(); s.identity = Some(id); s.snapshot = None; s.last = None; }
                    loop {
                        match collect(&conn.client).await {
                            Ok(snap) => { let mut s = state.lock().unwrap(); s.snapshot = Some(Arc::new(snap)); s.last = Some(Instant::now()); s.online = true; s.message = "Conectado".into(); backoff = 2; }
                            Err(e) => break e,
                        }
                        ctx.request_repaint(); tokio::select! { _ = sleep(Duration::from_secs(p.interval)) => {}, _ = wake.notified() => {} }
                    }
                }
            }
        };
        { let mut s = state.lock().unwrap(); s.online = false; s.message = format!("{} · nova tentativa em {}s",failure,backoff); }
        ctx.request_repaint(); sleep(Duration::from_secs(backoff)).await; backoff = (backoff*2).min(60);
    }
}
fn audit(p: &Profile, s: &Session, action: Action, result: &str) -> Result<(),String> {
    let path = model::data_dir()?.join("admin-audit.log");
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(path).map_err(|_| "Auditoria indisponível")?;
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_| "Relógio inválido")?.as_secs();
    // No credentials, query text or arbitrary server strings in this log.
    writeln!(file,"unix={} profile={} pid={} action={} outcome={}",ts,p.id,s.pid,if action==Action::Cancel {"cancel"}else{"terminate"},result).map_err(|_| "Falha ao gravar auditoria")?;
    file.sync_data().map_err(|_| "Falha ao persistir auditoria".into())
}
pub async fn perform(p: Profile, secret: Secret, s: Session, expected_instance: String, action: Action) -> Result<String,String> {
    if s.started.is_empty() || s.pid <= 0 || s.kind != "client backend" || s.app.starts_with("PGActivity-RS ") { return Err("Sessão não elegível".into()); }
    if action==Action::Cancel && (s.state!="active" || s.query_started.is_empty()) { return Err("Não há consulta ativa elegível".into()); }
    if !p.tls { return Err("Ações administrativas exigem TLS validado".into()); }
    audit(&p,&s,action,"requested")?;
    let outcome = async {
        let conn = connect(&p,&secret,true).await?;
        let id = identity(&conn.client).await?;
        if id.instance != expected_instance { return Err("Instância PostgreSQL mudou; ação abortada".into()); }
        if !(id.superuser || id.can_signal) { return Err("Privilégio administrativo não detectado".into()); }
        let sql = match action {
            Action::Cancel => "SELECT pg_cancel_backend(pid) FROM pg_stat_activity WHERE pid=$1 AND backend_start::text=$2 AND query_start::text=$3 AND state='active' AND backend_type='client backend' AND application_name NOT LIKE 'PGActivity-RS %' AND pid<>pg_backend_pid()",
            Action::Terminate => "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE pid=$1 AND backend_start::text=$2 AND backend_type='client backend' AND application_name NOT LIKE 'PGActivity-RS %' AND pid<>pg_backend_pid()",
        };
        let mut args: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = vec![&s.pid,&s.started];
        if action==Action::Cancel { args.push(&s.query_started); }
        let row = timeout(Duration::from_secs(7),conn.client.query_opt(sql,&args)).await.map_err(|_| "Timeout administrativo; resultado desconhecido, verifique a sessão antes de repetir")?.map_err(pg_error)?;
        match row { Some(row) if row.try_get::<_,bool>(0).map_err(pg_error)? => Ok("Sinal solicitado com sucesso; confirme o efeito no monitor.".into()), _ => Err("Sessão/consulta mudou ou sinal recusado; ação não confirmada".into()) }
    }.await;
    if audit(&p,&s,action,if outcome.is_ok(){"signal_accepted"}else{"not_confirmed"}).is_err() { return Err("Ação enviada ou tentada, mas auditoria final falhou. Verifique o servidor.".into()); }
    outcome
}
