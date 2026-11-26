#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
mod collector;
mod model;
use collector::{Secret, Shared, State};
use eframe::egui::{self, Color32, RichText};
use egui_extras::{Column, TableBuilder};
use model::{Action, Profile, Session, Snapshot};
use std::{collections::HashMap, sync::{Arc, Mutex}, time::Duration};
use tokio::{runtime::Runtime, task::JoinHandle, sync::Notify};
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

struct Worker { task: JoinHandle<()>, state: Shared, secret: Secret, wake: Arc<Notify> }
impl Drop for Worker { fn drop(&mut self) { self.task.abort(); } }
struct Editor { p: Profile, original: Option<Profile>, password: Zeroizing<String>, allow_plain: bool }
struct Confirm { profile: Uuid, session: Session, action: Action, typed: String }
struct App {
    runtime: Runtime, profiles: Vec<Profile>, workers: HashMap<Uuid, Worker>, selected: Option<Uuid>,
    editor: Option<Editor>, password: Zeroizing<String>, filter: String, only_active: bool, locks_only: bool,
    selected_session: Option<(i32,String)>, frozen: Option<State>, sort: usize, descending: bool, dark: bool,
    admin: bool, confirm: Option<Confirm>, action_busy: bool, action_result: Arc<Mutex<Option<Result<String,String>>>>,
    message: String, storage_ok: bool, plain_confirm: bool,
}
fn color(s: &Session) -> Color32 { match s.status() {
    "LOCK" => Color32::from_rgb(234,85,85), "IDLE IN TX"=>Color32::from_rgb(186,122,225),
    "I/O"=>Color32::from_rgb(219,170,65), "VACUUM"=>Color32::from_rgb(82,156,230),
    "ACTIVE"=>Color32::from_rgb(70,177,116), "WAIT"=>Color32::from_rgb(219,170,65), _=>Color32::GRAY } }
fn age(x: Option<f64>) -> String { match x { Some(v) if v.is_finite() => format!("{:.1}s",v.max(0.0)), _=>"—".into() } }
impl App {
    fn new(runtime: Runtime, ctx: &egui::Context) -> Self {
        ctx.set_visuals(egui::Visuals::dark());
        let (profiles,storage_ok,message)=match model::load_profiles(){Ok(p)=>(p,true,String::new()),Err(e)=>(vec![],false,e)};
        let selected=profiles.first().map(|p|p.id);
        Self {runtime,profiles,workers:HashMap::new(),selected,editor:None,password:Zeroizing::new(String::new()),filter:String::new(),only_active:false,locks_only:false,selected_session:None,frozen:None,sort:7,descending:true,dark:true,admin:false,confirm:None,action_busy:false,action_result:Arc::new(Mutex::new(None)),message,storage_ok,plain_confirm:false}
    }
    fn choose(&mut self,id:Uuid) { if self.selected!=Some(id) { self.selected=Some(id); self.password.zeroize(); self.selected_session=None; self.frozen=None; self.admin=false; self.confirm=None; self.plain_confirm=false; } }
    fn persist(&mut self, next: Vec<Profile>) -> bool {
        if self.storage_ok { match model::save_profiles(&next) { Ok(())=>{},Err(e)=>{self.message=format!("Perfis não salvos: {e}"); return false;} } }
        else { self.message="Perfis somente nesta sessão: arquivo original preservado.".into(); }
        self.profiles=next; true
    }
    fn start(&mut self,p:Profile,ctx:&egui::Context) {
        if self.workers.contains_key(&p.id) {return;}
        if !p.tls && !self.plain_confirm {self.message="Confirme a conexão sem TLS (somente laboratório).".into();return;}
        let password=if !self.password.is_empty(){self.password.to_string()}else if p.remember && p.tls { match model::get_password(&p){Ok(v)=>v,Err(e)=>{self.message=e;return;}} }else{self.message="Informe a senha.".into();return;};
        let secret=Arc::new(Zeroizing::new(password)); self.password.zeroize(); self.plain_confirm=false;
        if p.remember && p.tls && self.storage_ok { if let Err(e)=model::store_password(&p,secret.as_str()) {self.message=e;return;} }
        let state=Arc::new(Mutex::new(State::default())); let wake=Arc::new(Notify::new());
        let task=self.runtime.spawn(collector::run(p.clone(),secret.clone(),state.clone(),wake.clone(),ctx.clone()));
        self.workers.insert(p.id,Worker{task,state,secret,wake}); self.message.clear(); self.admin=false; self.frozen=None;
    }
    fn current(&self)->Option<State>{ self.selected.and_then(|id|self.workers.get(&id)).map(|w|w.state.lock().unwrap().clone()) }
    fn disconnect(&mut self,id:Uuid){ self.workers.remove(&id); self.selected_session=None; self.frozen=None; self.admin=false; self.confirm=None; }
    fn profile_editor(&mut self,ctx:&egui::Context) {
        let Some(mut ed)=self.editor.take() else{return;};
        let mut open=true; let mut save=false; let mut cancel=false;
        egui::Window::new("Perfil de conexão").open(&mut open).collapsible(false).resizable(false).show(ctx,|ui|{
            egui::Grid::new("profile_form").num_columns(2).spacing([12.0,8.0]).show(ui,|ui|{
                ui.label("Nome");ui.text_edit_singleline(&mut ed.p.name);ui.end_row();
                ui.label("Host DNS / IP");ui.text_edit_singleline(&mut ed.p.host);ui.end_row();
                ui.label("Porta");ui.add(egui::DragValue::new(&mut ed.p.port).range(1..=65535));ui.end_row();
                ui.label("Banco");ui.text_edit_singleline(&mut ed.p.database);ui.end_row();
                ui.label("Usuário");ui.text_edit_singleline(&mut ed.p.user);ui.end_row();
                ui.label("Senha (opcional)");ui.add(egui::TextEdit::singleline(&mut *ed.password).password(true));ui.end_row();
                ui.label("Intervalo");ui.add(egui::DragValue::new(&mut ed.p.interval).range(1..=60).suffix(" s"));ui.end_row();
                ui.label("TLS");ui.checkbox(&mut ed.p.tls,"Exigir e validar certificado");ui.end_row();
                ui.label("CA PEM adicional");ui.add_enabled(ed.p.tls,egui::TextEdit::singleline(&mut ed.p.ca_file));ui.end_row();
                ui.label("Credenciais");ui.add_enabled(cfg!(windows)&&self.storage_ok,egui::Checkbox::new(&mut ed.p.remember,"Salvar no Credential Manager"));ui.end_row();
            });
            if !ed.p.tls {ui.colored_label(Color32::YELLOW,"Sem TLS: tráfego e SQL podem ser interceptados.");ui.checkbox(&mut ed.allow_plain,"Aceito usar este perfil apenas em rede de laboratório");}
            ui.horizontal(|ui|{save=ui.add_enabled(ed.p.tls||ed.allow_plain,egui::Button::new("Salvar perfil")).clicked();cancel=ui.button("Cancelar").clicked();});
        });
        if !open||cancel{return;}
        if save {
            if let Err(e)=ed.p.validate(){self.message=e;self.editor=Some(ed);return;}
            if !ed.p.tls {ed.p.remember=false;}
            if let Some(old)=&ed.original {
                if self.workers.contains_key(&old.id){self.message="Desconecte antes de editar.".into();self.editor=Some(ed);return;}
                if old.remember && (!ed.p.remember || old.host!=ed.p.host || old.port!=ed.p.port || old.user!=ed.p.user || old.database!=ed.p.database || old.tls!=ed.p.tls || old.ca_file!=ed.p.ca_file) {
                    if let Err(e)=model::delete_password(old){self.message=e;self.editor=Some(ed);return;}
                }
            }
            let mut next=self.profiles.clone(); if let Some(v)=next.iter_mut().find(|v|v.id==ed.p.id){*v=ed.p.clone();}else{next.push(ed.p.clone());}
            if self.persist(next) {self.choose(ed.p.id);self.password.zeroize();self.password=ed.password;self.plain_confirm=false;}
            else {self.editor=Some(ed);}
        }else{self.editor=Some(ed);}
    }
    fn sidebar(&mut self,ctx:&egui::Context){
        egui::SidePanel::left("servers").resizable(true).default_width(235.0).min_width(165.0).show(ctx,|ui|{
            ui.heading("Servidores");
            if ui.button("+ Novo perfil").clicked(){self.editor=Some(Editor{p:Profile::default(),original:None,password:Zeroizing::new(String::new()),allow_plain:false});}
            ui.separator();
            let mut picked=None;
            egui::ScrollArea::vertical().id_salt("profiles").max_height(ui.available_height()-75.0).show(ui,|ui|{
                for p in &self.profiles {
                    let state=self.workers.get(&p.id).map(|w|w.state.lock().unwrap().clone());
                    let (dot,c)=match state {Some(s) if s.online=>("●",Color32::from_rgb(70,177,116)),Some(_)=>("●",Color32::YELLOW),None=>("○",Color32::GRAY)};
                    ui.horizontal(|ui|{ui.colored_label(c,dot);if ui.selectable_label(self.selected==Some(p.id),&p.name).clicked(){picked=Some(p.id);}});
                    ui.small(format!("{}:{} / {}",p.host,p.port,p.database));
                }
            });
            if let Some(id)=picked{self.choose(id);}
            ui.separator();ui.small(format!("{} conexões monitoradas",self.workers.len()));
            if ui.checkbox(&mut self.dark,"Tema escuro").changed(){ctx.set_visuals(if self.dark{egui::Visuals::dark()}else{egui::Visuals::light()});}
        });
    }
    fn confirm_window(&mut self,ctx:&egui::Context){
        let Some(mut c)=self.confirm.take()else{return;}; let mut open=true;let mut send=false;let mut cancel=false;
        let target=self.profiles.iter().find(|p|p.id==c.profile).cloned();
        egui::Window::new("Confirmar operação administrativa").open(&mut open).collapsible(false).show(ctx,|ui|{
            ui.colored_label(Color32::from_rgb(234,85,85),c.action.label());
            if let Some(p)=&target {ui.label(format!("Servidor: {} ({}:{})",p.name,p.host,p.port));}
            ui.label(format!("PID: {} | Usuário: {} | Banco: {}",c.session.pid,c.session.user,c.session.db));
            ui.label("A operação pode interromper trabalho e provocar rollback. Digite o PID para confirmar.");
            ui.text_edit_singleline(&mut c.typed);
            ui.horizontal(|ui|{send=ui.add_enabled(c.typed.trim()==c.session.pid.to_string()&&!self.action_busy,egui::Button::new("Executar")).clicked();cancel=ui.button("Voltar").clicked();});
        });
        if !open||cancel{return;}
        if send {
            if let (Some(p),Some(w))=(target,self.workers.get(&c.profile)){
                let live=w.state.lock().unwrap().clone();
                let fresh=live.online&&live.last.is_some_and(|t|t.elapsed()<Duration::from_secs(p.interval*2+8));
                let same=live.snapshot.as_ref().is_some_and(|snap|snap.sessions.iter().any(|s|s.key()==c.session.key()&&(c.action!=Action::Cancel||s.query_started==c.session.query_started)));
                if !self.admin||self.selected!=Some(c.profile)||self.frozen.is_some()||!fresh||!same {self.message="Sessão alterada, indisponível ou desatualizada; operação abortada.".into();return;}
                let Some(identity)=live.identity else {self.message="Identidade do servidor indisponível".into();return;};
                let expected_instance=identity.instance;
                let secret=w.secret.clone();let result=self.action_result.clone();let repaint=ctx.clone();self.action_busy=true;self.admin=false;
                self.runtime.spawn(async move {let value=collector::perform(p,secret,c.session,expected_instance,c.action).await;*result.lock().unwrap()=Some(value);repaint.request_repaint();});
            }
        }else{self.confirm=Some(c);}
    }
    fn monitor(&mut self,ui:&mut egui::Ui,ctx:&egui::Context,p:&Profile){
        let Some(live)=self.current()else{
            ui.heading(&p.name);ui.label(format!("{}@{}:{}/{}",p.user,p.host,p.port,p.database));
            ui.horizontal(|ui|{ui.label("Senha");ui.add(egui::TextEdit::singleline(&mut *self.password).password(true).hint_text(if p.remember{"Usar credencial salva"}else{"Informe a senha"}));});
            if !p.tls{ui.checkbox(&mut self.plain_confirm,"Confirmo conexão sem TLS (laboratório)");}
            ui.horizontal(|ui|{
                if ui.button("Conectar").clicked(){self.start(p.clone(),ctx);}
                if ui.button("Editar").clicked(){self.editor=Some(Editor{p:p.clone(),original:Some(p.clone()),password:Zeroizing::new(String::new()),allow_plain:false});}
                if p.remember&&ui.button("Esquecer senha").clicked(){match model::delete_password(p){Ok(())=>self.message="Credencial excluída.".into(),Err(e)=>self.message=e}}
                // Explicitly labelled removal; no server operation.
                if ui.button("Excluir perfil local").clicked(){match model::delete_password(p){Ok(())=>{let next=self.profiles.iter().filter(|v|v.id!=p.id).cloned().collect();if self.persist(next){self.selected=self.profiles.first().map(|v|v.id);self.password.zeroize();}},Err(e)=>self.message=e}}
            });return;
        };
        let mut disconnect=false;
        ui.horizontal_wrapped(|ui|{
            ui.heading(&p.name);
            ui.colored_label(if live.online{Color32::from_rgb(70,177,116)}else{Color32::YELLOW},&live.message);
            if let Some(id)=&live.identity {ui.label(format!("PG {} · {} · {}",id.version,id.user,if id.superuser{"SUPERUSER"}else if id.read_all{"MONITOR"}else{"VISIBILIDADE LIMITADA"}));if id.replica{ui.label("RÉPLICA");}}
            ui.label(if p.tls{"TLS validado"}else{"SEM TLS"});
            disconnect=ui.button("Desconectar").clicked();
        });
        if disconnect{self.disconnect(p.id);return;}
        let fresh=live.online&&live.last.is_some_and(|t|t.elapsed()<Duration::from_secs(p.interval*2+8));
        let allowed=fresh&&p.tls&&live.identity.as_ref().is_some_and(|v|v.superuser||v.can_signal)&&self.frozen.is_none()&&!self.action_busy;
        if !allowed {self.admin=false;self.confirm=None;}
        ui.horizontal_wrapped(|ui|{
            if ui.button(if self.frozen.is_some(){"Retomar"}else{"Pausar visualização"}).clicked()||(!ctx.wants_keyboard_input()&&ctx.input(|i|i.key_pressed(egui::Key::Space))) {
                if self.frozen.is_some(){self.frozen=None;}else{self.frozen=Some(live.clone());self.admin=false;self.confirm=None;}
            }
            if ui.button("Atualizar (F5)").clicked()||ctx.input(|i|i.key_pressed(egui::Key::F5)){if let Some(w)=self.workers.get(&p.id){w.wake.notify_one();}}
            ui.add_enabled(allowed&&self.frozen.is_none(),egui::Checkbox::new(&mut self.admin,"Habilitar ação administrativa"));
            if self.admin{ui.colored_label(Color32::YELLOW,"ADMINISTRATIVO");}else{ui.label("Monitor somente leitura");}
            if self.action_busy{ui.spinner();ui.label("Ação em andamento");}
        });
        let display=self.frozen.clone().unwrap_or_else(||live.clone());
        if self.frozen.is_some(){ui.colored_label(Color32::YELLOW,"VISUALIZAÇÃO CONGELADA — coleta continua em segundo plano.");}
        else if !fresh{ui.colored_label(Color32::YELLOW,"Dados ausentes ou desatualizados; ações indisponíveis.");}
        if let Some(t)=display.last{ui.small(format!("Última amostra: {:.1}s atrás",t.elapsed().as_secs_f64()));}
        let Some(snap)=display.snapshot else{ui.spinner();return;};
        self.activity(ui,ctx,&snap);
    }
    fn activity(&mut self,ui:&mut egui::Ui,ctx:&egui::Context,snap:&Snapshot){
        let sessions=&snap.sessions;
        let active=sessions.iter().filter(|s|s.state=="active").count();let locks=sessions.iter().filter(|s|s.wait_type=="Lock").count();let idle_tx=sessions.iter().filter(|s|s.state.starts_with("idle in transaction")).count();
        ui.horizontal_wrapped(|ui|{
            ui.strong(format!("Sessões/processos: {}{}",sessions.len(),if snap.truncated{"+"}else{""}));
            ui.colored_label(Color32::from_rgb(70,177,116),format!("Ativas: {active}"));ui.colored_label(Color32::from_rgb(234,85,85),format!("Locks: {locks}"));ui.colored_label(Color32::from_rgb(186,122,225),format!("Idle in TX: {idle_tx}"));
        });
        if snap.truncated{ui.colored_label(Color32::YELLOW,"Amostra limitada a 5.000 processos; contadores parciais.");}
        ui.horizontal_wrapped(|ui|{ui.label("Filtro");ui.add(egui::TextEdit::singleline(&mut self.filter).hint_text("PID, usuário, SQL, IP, wait..."));ui.checkbox(&mut self.only_active,"Ativas");ui.checkbox(&mut self.locks_only,"Somente locks");});
        let mut rows:Vec<&Session>=sessions.iter().filter(|s|s.matches(&self.filter)&&(!self.only_active||s.state=="active")&&(!self.locks_only||s.wait_type=="Lock")).collect();
        let sort=self.sort;let descending=self.descending;
        rows.sort_by(|a,b|{let ord=match sort{0=>a.pid.cmp(&b.pid),1=>a.db.cmp(&b.db),2=>a.user.cmp(&b.user),3=>a.app.cmp(&b.app),4=>a.ip.cmp(&b.ip),5=>a.state.cmp(&b.state),6=>a.wait.cmp(&b.wait),7=>a.query_age.unwrap_or(-1.0).total_cmp(&b.query_age.unwrap_or(-1.0)),8=>a.xact_age.unwrap_or(-1.0).total_cmp(&b.xact_age.unwrap_or(-1.0)),9=>a.blockers.len().cmp(&b.blockers.len()),_=>a.query.cmp(&b.query)}; (if descending{ord.reverse()}else{ord}).then(a.pid.cmp(&b.pid))});
        ui.small(format!("{} linhas visíveis",rows.len()));
        let headings=["PID","Banco","Usuário","Aplicação","IP","Estado","Wait event","Consulta (s)","Transação","Bloqueadores","SQL"];
        let widths=[62.,95.,85.,100.,100.,95.,130.,82.,82.,92.,300.];
        let height=(ui.available_height()-if self.selected_session.is_some(){225.0}else{20.0}).max(130.0);
        let mut picked=None;
        egui::ScrollArea::horizontal().id_salt("activity_x").show(ui,|ui|{
            let mut table=TableBuilder::new(ui).striped(true).resizable(true).max_scroll_height(height).min_scrolled_height(100.0);
            for w in widths {table=table.column(Column::initial(w).at_least(45.0).clip(true));}
            table.header(25.0,|mut header|{for (i,h) in headings.iter().enumerate(){header.col(|ui|{let label=if self.sort==i{format!("{} {}",h,if self.descending{"▼"}else{"▲"})}else{h.to_string()};if ui.button(label).clicked(){if self.sort==i{self.descending=!self.descending;}else{self.sort=i;self.descending=matches!(i,7|8|9);}}});}})
            .body(|body|{body.rows(23.0,rows.len(),|mut row|{
                let s=rows[row.index()];row.col(|ui|{if ui.selectable_label(self.selected_session.as_ref().is_some_and(|k|*k==s.key()),s.pid.to_string()).clicked(){picked=Some(s.key());}});
                for text in [&s.db,&s.user,&s.app,&s.ip]{row.col(|ui|{ui.add(egui::Label::new(text.as_str()).truncate());});}
                row.col(|ui|{ui.add(egui::Label::new(RichText::new(s.status()).color(color(s))).truncate());});
                row.col(|ui|{ui.add(egui::Label::new(format!("{} {}",s.wait_type,s.wait)).truncate());});
                row.col(|ui|{ui.label(age(s.query_age));});row.col(|ui|{ui.label(age(s.xact_age));});
                row.col(|ui|{ui.add(egui::Label::new(s.blockers.iter().map(i32::to_string).collect::<Vec<_>>().join(", ")).truncate());});
                row.col(|ui|{ui.add(egui::Label::new(s.query.replace(['\n','\r']," ")).truncate());});
            });});
        });
        if let Some(k)=picked{self.selected_session=Some(k);}
        if let Some(key)=self.selected_session.clone(){
            ui.separator();
            if let Some(s)=sessions.iter().find(|s|s.key()==key){
                ui.horizontal_wrapped(|ui|{
                    ui.strong(format!("PID {} · {} · {}",s.pid,s.user,s.db));ui.label(format!("Estado: {} | desde {}",s.state,age(s.state_age)));
                    if ui.button("Copiar SQL").clicked(){ctx.copy_text(s.query.clone());}
                    if ui.button("Fechar detalhe").clicked(){self.selected_session=None;}
                    let eligible=self.admin&&self.frozen.is_none()&&!self.action_busy&&s.kind=="client backend"&&!s.started.is_empty()&&!s.app.starts_with("PGActivity-RS ");
                    if ui.add_enabled(eligible&&s.state=="active"&&!s.query_started.is_empty(),egui::Button::new("Cancelar consulta")).clicked(){self.confirm=Some(Confirm{profile:self.selected.unwrap(),session:s.clone(),action:Action::Cancel,typed:String::new()});}
                    if ui.add_enabled(eligible,egui::Button::new("Encerrar sessão")).clicked(){self.confirm=Some(Confirm{profile:self.selected.unwrap(),session:s.clone(),action:Action::Terminate,typed:String::new()});}
                });
                let edges=model::blocker_chain(sessions,s.pid);if !edges.is_empty(){ui.colored_label(Color32::YELLOW,format!("Cadeia (bloqueador → bloqueado): {}",edges.iter().map(|(a,b)|format!("{a} → {b}")).collect::<Vec<_>>().join("  |  ")));}
                egui::ScrollArea::vertical().id_salt("sql_details").max_height(140.0).show(ui,|ui|{ui.add(egui::Label::new(RichText::new(&s.query).monospace()).wrap());});
                ui.small("SQL limitado a 8.192 caracteres e ao track_activity_query_size do servidor; sessão ociosa mostra a última consulta.");
            }else{ui.label("Sessão selecionada não aparece na amostra atual.");if ui.button("Limpar seleção").clicked(){self.selected_session=None;}}
        }
    }
}
impl eframe::App for App {
    fn update(&mut self,ctx:&egui::Context,_:&mut eframe::Frame){
        let result=self.action_result.lock().unwrap().take();if let Some(r)=result{self.action_busy=false;self.message=match r{Ok(s)|Err(s)=>s};}
        egui::TopBottomPanel::top("header").show(ctx,|ui|{ui.horizontal(|ui|{ui.heading("PGActivity-RS");ui.small(concat!("v",env!("CARGO_PKG_VERSION")," · PostgreSQL monitor"));});if !self.message.is_empty(){ui.horizontal_wrapped(|ui|{ui.label(&self.message);if ui.button("×").clicked(){self.message.clear();}});}});
        self.sidebar(ctx);
        egui::CentralPanel::default().show(ctx,|ui|{if let Some(p)=self.selected.and_then(|id|self.profiles.iter().find(|p|p.id==id).cloned()){self.monitor(ui,ctx,&p);}else{ui.heading("Monitoramento PostgreSQL");ui.label("Crie um perfil para iniciar. Nenhum agente ou extensão é necessário.");}});
        self.profile_editor(ctx);self.confirm_window(ctx);
        if !self.workers.is_empty(){ctx.request_repaint_after(Duration::from_millis(500));}
    }
}
fn main()->eframe::Result<()> {
    let runtime=tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().expect("Tokio runtime");
    let options=eframe::NativeOptions{viewport:egui::ViewportBuilder::default().with_inner_size([1400.0,850.0]).with_min_inner_size([850.0,550.0]),..Default::default()};
    eframe::run_native("PGActivity-RS",options,Box::new(move |cc|Ok(Box::new(App::new(runtime,&cc.egui_ctx)))))
}
