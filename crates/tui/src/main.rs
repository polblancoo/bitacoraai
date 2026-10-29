mod api;
mod config;
mod ui;

use anyhow::Result;
use api::Api;
use crossterm::{
    event::{self, Event as CEvent, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures_util::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::time::Duration;
use tm_core::*;
use tokio::sync::mpsc;

pub const CONFIG_ITEMS: usize = 12;

pub enum Msg {
    Key(KeyEvent),
    Server(tm_core::Event),
    Connected(bool),
}

#[derive(Clone, Copy, PartialEq)]
pub enum PromptKind {
    NewTask,
    NewProject,
    NewTeam,
    AddMember,
    Comment,
    EditTitle,
    Subtask,
    Label,
    JoinCode,
    SetRole,
    AssignTo,
    SetEmail,
    VerifyEmail,
    TeamMessage,
}

pub enum Mode {
    Login,
    Board,
    Detail,
    Prompt(PromptKind),
    Help,
    /// Tras Ctrl+T: esperando la letra del atajo de equipo.
    Prefix,
    Inbox,
    Config,
}

pub struct App {
    pub api: Api,
    pub mode: Mode,
    pub return_mode_detail: bool,
    pub me: Option<User>,
    // login
    pub field: usize,
    pub login: [String; 3], // servidor, usuario, contraseña
    pub register: bool,
    // datos
    pub teams: Vec<Team>,
    pub team: usize,
    pub projects: Vec<Project>,
    pub project: usize,
    pub tasks: Vec<Task>,
    pub members: Vec<Member>,
    pub labels: Vec<Label>,
    pub comments: Vec<Comment>,
    pub sub: usize,
    // selección
    pub col: usize,
    pub row: [usize; 3],
    pub input: String,
    pub status: String,
    pub online: bool,
    pub quit: bool,
    // sugerencias del prompt (agregar miembro / asignar)
    pub suggest: Vec<String>,
    pub sug_sel: usize,
    pub sug_role: Role,
    sug_q: String,
    sug_full: bool,
    // ayuda con buscador
    pub help_q: String,
    // bandeja
    pub inbox: Option<Inbox>,
    pub inbox_sel: usize,
    pub unread: i64,
    pub email_info: Option<Me>,
    pub cfg_sel: usize,
}

impl App {
    pub fn column(&self, c: usize) -> Vec<&Task> {
        self.tasks.iter().filter(|t| t.parent_id.is_none() && t.status == Status::ALL[c]).collect()
    }

    pub fn subtasks(&self, parent: Id) -> Vec<&Task> {
        self.tasks.iter().filter(|t| t.parent_id == Some(parent)).collect()
    }

    pub fn selected(&self) -> Option<&Task> {
        self.column(self.col).get(self.row[self.col]).copied()
    }

    fn sort_tasks(&mut self) {
        self.tasks.sort_by(|a, b| b.priority.cmp(&a.priority).then(a.id.cmp(&b.id)));
        for c in 0..3 {
            let n = self.column(c).len();
            self.row[c] = self.row[c].min(n.saturating_sub(1));
        }
    }

    fn upsert(&mut self, t: Task) {
        if t.project_id != self.cur_project() {
            return;
        }
        let keep = self.selected().map(|s| s.id);
        match self.tasks.iter_mut().find(|x| x.id == t.id) {
            Some(x) => *x = t,
            None => self.tasks.push(t),
        }
        self.sort_tasks();
        if let Some(pos) = keep.and_then(|id| self.column(self.col).iter().position(|x| x.id == id)) {
            self.row[self.col] = pos;
        }
    }

    pub fn cur_team(&self) -> Id { self.teams.get(self.team).map(|t| t.id).unwrap_or(0) }
    pub fn cur_project(&self) -> Id { self.projects.get(self.project).map(|p| p.id).unwrap_or(0) }

    async fn load_teams(&mut self) -> Result<()> {
        self.teams = self.api.teams().await?;
        self.team = self.team.min(self.teams.len().saturating_sub(1));
        self.load_team().await
    }

    async fn load_team(&mut self) -> Result<()> {
        if self.teams.is_empty() {
            self.projects.clear();
            self.tasks.clear();
            return Ok(());
        }
        let id = self.cur_team();
        self.projects = self.api.projects(id).await?;
        self.members = self.api.members(id).await?;
        self.labels = self.api.team_labels(id).await?;
        if let Some(me) = self.me.as_mut().filter(|m| m.id == 0) {
            if let Some(u) = self.members.iter().find(|u| u.username.eq_ignore_ascii_case(&me.username)) {
                me.id = u.id;
            }
        }
        self.project = self.project.min(self.projects.len().saturating_sub(1));
        self.load_project().await
    }

    async fn load_project(&mut self) -> Result<()> {
        self.row = [0; 3];
        self.tasks = if self.projects.is_empty() { vec![] } else { self.api.tasks(self.cur_project()).await? };
        self.sort_tasks();
        Ok(())
    }

    async fn enter_session(&mut self, auth: AuthResponse, tx: &mpsc::UnboundedSender<Msg>) -> Result<()> {
        self.api.token = Some(auth.token.clone());
        config::save(&config::Config {
            server: Some(self.api.base.clone()),
            token: Some(auth.token),
            username: Some(auth.user.username.clone()),
        });
        self.me = Some(auth.user);
        self.mode = Mode::Board;
        spawn_ws(self.api.ws_url(), self.api.token.clone().unwrap_or_default(), tx.clone());
        self.load_teams().await?;
        self.load_me().await;
        self.open_inbox(false).await;
        Ok(())
    }

    async fn load_me(&mut self) {
        if let Ok(m) = self.api.me().await {
            self.me = Some(User { id: m.id, username: m.username.clone() });
            self.email_info = Some(m);
        }
    }

    /// Rol propio en el equipo actual.
    pub fn my_role(&self) -> Option<Role> {
        let id = self.me.as_ref()?.id;
        self.members.iter().find(|m| m.id == id).map(|m| m.role)
    }

    /// Carga la bandeja; `force` la abre aunque no haya nada nuevo.
    async fn open_inbox(&mut self, force: bool) {
        match self.api.inbox().await {
            Ok(inb) => {
                self.unread = inb.unread;
                let show = force || inb.unread > 0 || !inb.tasks.is_empty();
                self.inbox = Some(inb);
                self.inbox_sel = 0;
                if show { self.mode = Mode::Inbox; }
            }
            Err(e) => self.status = format!("✗ {e}"),
        }
    }

    async fn close_inbox(&mut self) {
        if self.unread > 0 {
            if self.api.mark_read(vec![]).await.is_ok() {
                self.unread = 0;
                if let Some(i) = self.inbox.as_mut() { for n in &mut i.notifications { n.read = true; } }
            }
        }
        self.mode = Mode::Board;
    }

    /// Salta a la tarea elegida en la bandeja (cambia de equipo/proyecto si hace falta).
    async fn jump_to(&mut self, team: &str, task: &Task) -> Result<()> {
        if let Some(i) = self.teams.iter().position(|t| t.name == team) {
            if i != self.team { self.team = i; self.project = 0; self.load_team().await?; }
        }
        if let Some(i) = self.projects.iter().position(|p| p.id == task.project_id) {
            if i != self.project { self.project = i; self.load_project().await?; }
        }
        if let Some(c) = Status::ALL.iter().position(|s| *s == task.status) {
            self.col = c;
            if let Some(r) = self.column(c).iter().position(|x| x.id == task.id) { self.row[c] = r; }
        }
        Ok(())
    }

    fn open_prompt_suggest(&mut self, kind: PromptKind) {
        self.suggest.clear();
        self.sug_sel = 0;
        self.sug_q = "\u{0}".into();
        self.sug_full = false;
        self.prompt(kind);
    }

    /// Recalcula las sugerencias según lo escrito.
    async fn refresh_suggest(&mut self, kind: PromptKind) {
        let q = self.input.trim().to_lowercase();
        match kind {
            PromptKind::AssignTo => {
                self.suggest = self.members.iter().filter(|m| m.role >= Role::Member && m.username.to_lowercase().starts_with(&q)).map(|m| m.username.clone()).collect();
            }
            PromptKind::AddMember => {
                if q.is_empty() {
                    self.suggest.clear();
                } else if self.sug_full && q.starts_with(&self.sug_q) {
                    // la búsqueda previa trajo todo lo que empieza con el prefijo: filtra local
                    self.suggest.retain(|n| n.to_lowercase().starts_with(&q));
                } else if let Ok(us) = self.api.search_users(self.cur_team(), &q).await {
                    self.sug_full = us.len() < 8;
                    self.sug_q = q.clone();
                    self.suggest = us.into_iter().map(|u| u.username).collect();
                }
            }
            _ => {}
        }
        self.sug_sel = self.sug_sel.min(self.suggest.len().saturating_sub(1));
    }

    async fn on_server(&mut self, ev: tm_core::Event) {
        match ev {
            tm_core::Event::TaskCreated(t) | tm_core::Event::TaskUpdated(t) => self.upsert(t),
            tm_core::Event::TaskDeleted { id, .. } => {
                self.tasks.retain(|t| t.id != id && t.parent_id != Some(id));
                self.sort_tasks();
            }
            tm_core::Event::ProjectDeleted { .. } => {}
            tm_core::Event::ProjectCreated(p) => {
                if p.team_id == self.cur_team() && !self.projects.iter().any(|x| x.id == p.id) {
                    self.projects.push(p);
                }
            }
            tm_core::Event::LabelCreated(l) => {
                if l.team_id == self.cur_team() && !self.labels.iter().any(|x| x.id == l.id) {
                    self.labels.push(l);
                }
            }
            tm_core::Event::LabelDeleted { id, .. } => {
                self.labels.retain(|l| l.id != id);
                for t in &mut self.tasks {
                    t.labels.retain(|l| l.id != id);
                }
            }
            tm_core::Event::CommentAdded(c) => {
                if matches!(self.mode, Mode::Detail) && self.selected().map(|t| t.id) == Some(c.task_id) && !self.comments.iter().any(|x| x.id == c.id) {
                    self.comments.push(c);
                }
            }
            tm_core::Event::MemberRemoved { team_id, user_id } => {
                if self.me.as_ref().is_some_and(|m| m.id == user_id) {
                    self.status = "✗ ya no sos parte de ese equipo".into();
                    let _ = self.load_teams().await;
                } else if team_id == self.cur_team() {
                    if let Ok(m) = self.api.members(team_id).await { self.members = m; }
                }
            }
            tm_core::Event::MemberRoleChanged { team_id, .. } => {
                if team_id == self.cur_team() {
                    if let Ok(m) = self.api.members(team_id).await { self.members = m; }
                }
            }
            tm_core::Event::Notification(n) => {
                self.status = format!("ℹ 🔔 {}", n.text);
                self.unread += 1;
                if let Some(i) = self.inbox.as_mut() { i.notifications.insert(0, n); i.unread += 1; }
            }
            tm_core::Event::CanvasItem(_) | tm_core::Event::CanvasItemDeleted { .. } | tm_core::Event::CanvasLink(_) | tm_core::Event::CanvasLinkDeleted { .. } => {}
            tm_core::Event::MemberAdded { team_id, .. } => {
                if team_id == self.cur_team() {
                    if let Ok(m) = self.api.members(team_id).await { self.members = m; }
                } else if self.me.is_some() {
                    let _ = self.load_teams().await;
                }
            }
        }
    }

    async fn submit(&mut self, kind: PromptKind) -> Result<()> {
        let text = std::mem::take(&mut self.input);
        let text = text.trim().to_string();
        if text.is_empty() {
            return Ok(());
        }
        match kind {
            PromptKind::NewTask => { let t = self.api.new_task(self.cur_project(), &text).await?; self.upsert(t); }
            PromptKind::NewProject => {
                let p = self.api.new_project(self.cur_team(), &text).await?;
                if !self.projects.iter().any(|x| x.id == p.id) { self.projects.push(p); }
                self.project = self.projects.len() - 1;
                self.load_project().await?;
            }
            PromptKind::NewTeam => {
                let t = self.api.new_team(&text).await?;
                self.teams.push(t);
                self.team = self.teams.len() - 1;
                self.project = 0;
                self.load_team().await?;
            }
            PromptKind::AddMember => {
                let role = self.sug_role;
                let u = self.api.add_member(self.cur_team(), &text, Some(role)).await?;
                self.status = format!("ℹ {} agregado al equipo como {}", u.username, role.label());
            }
            PromptKind::AssignTo => {
                let id = if text == "-" {
                    None
                } else {
                    Some(self.members.iter().find(|m| m.username.eq_ignore_ascii_case(&text)).map(|m| m.id)
                        .ok_or_else(|| anyhow::anyhow!("no hay un miembro llamado {text}"))?)
                };
                if let Some(tid) = self.selected().map(|t| t.id) {
                    let t = self.api.patch_task(tid, &TaskPatch { assignee_id: Some(id), ..Default::default() }).await?;
                    self.upsert(t);
                }
            }
            PromptKind::SetEmail => {
                self.api.set_email(&text).await?;
                self.load_me().await;
                self.status = "ℹ te enviamos un código de 6 dígitos: Ctrl+T v para confirmarlo".into();
            }
            PromptKind::TeamMessage => {
                let r = self.api.send_message(self.cur_team(), &text).await?;
                self.status = format!("ℹ mensaje enviado a {} personas", r.sent);
            }
            PromptKind::VerifyEmail => {
                self.api.verify_email(&text).await?;
                self.load_me().await;
                self.status = "ℹ email verificado: vas a recibir avisos por correo".into();
            }
            PromptKind::SetRole => {
                let mut it = text.split_whitespace();
                let name = it.next().unwrap_or_default();
                let role = parse_role(it.next().unwrap_or_default())?;
                let id = self.members.iter().find(|m| m.username.eq_ignore_ascii_case(name)).map(|m| m.id)
                    .ok_or_else(|| anyhow::anyhow!("no hay un miembro llamado {name}"))?;
                self.api.set_role(self.cur_team(), id, role).await?;
                self.members = self.api.members(self.cur_team()).await?;
                self.status = format!("ℹ {name} ahora es {}", role.label());
            }
            PromptKind::JoinCode => {
                let team = self.api.accept_invite(&text).await?;
                self.load_teams().await?;
                if let Some(i) = self.teams.iter().position(|t| t.id == team.id) {
                    self.team = i;
                    self.project = 0;
                    self.load_team().await?;
                }
                self.status = format!("ℹ te uniste a {}", team.name);
            }
            PromptKind::Comment => {
                if let Some(id) = self.selected().map(|t| t.id) {
                    let c = self.api.new_comment(id, &text).await?;
                    if !self.comments.iter().any(|x| x.id == c.id) { self.comments.push(c); }
                }
            }
            PromptKind::Subtask => {
                if let Some(parent) = self.selected().map(|t| t.id) {
                    let t = self.api.new_subtask(self.cur_project(), parent, &text).await?;
                    self.upsert(t);
                }
            }
            PromptKind::Label => {
                if let Some(task) = self.selected().cloned() {
                    let existing = self.labels.iter().find(|l| l.name.eq_ignore_ascii_case(&text)).cloned();
                    let label = match existing {
                        Some(l) => l,
                        None => {
                            const PALETTE: [&str; 6] = ["#7aa2f7", "#bb9af7", "#9ece6a", "#e0af68", "#f7768e", "#7dcfff"];
                            let l = self.api.new_label(self.cur_team(), &text, PALETTE[self.labels.len() % 6]).await?;
                            if !self.labels.iter().any(|x| x.id == l.id) { self.labels.push(l.clone()); }
                            l
                        }
                    };
                    // alterna: si ya la tiene la quita, si no la agrega
                    let mut ids: Vec<Id> = task.labels.iter().map(|l| l.id).collect();
                    match ids.iter().position(|i| *i == label.id) {
                        Some(p) => { ids.remove(p); }
                        None => ids.push(label.id),
                    }
                    let t = self.api.set_labels(task.id, ids).await?;
                    self.upsert(t);
                }
            }
            PromptKind::EditTitle => {
                if let Some(id) = self.selected().map(|t| t.id) {
                    let t = self.api.patch_task(id, &TaskPatch { title: Some(text), ..Default::default() }).await?;
                    self.upsert(t);
                }
            }
        }
        Ok(())
    }

    async fn key(&mut self, k: KeyEvent, tx: &mpsc::UnboundedSender<Msg>) -> Result<()> {
        if k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('c') {
            self.quit = true;
            return Ok(());
        }
        match self.mode {
            Mode::Login => self.key_login(k, tx).await,
            Mode::Prompt(kind) => {
                let suggesting = matches!(kind, PromptKind::AddMember | PromptKind::AssignTo);
                match k.code {
                    KeyCode::Esc => { self.input.clear(); self.mode = if self.return_mode_detail { Mode::Detail } else { Mode::Board }; }
                    KeyCode::Enter => {
                        if suggesting && !self.input.trim().is_empty() && self.input.trim() != "-" {
                            let typed = self.input.trim().to_string();
                            if !self.suggest.iter().any(|s| s.eq_ignore_ascii_case(&typed)) {
                                if let Some(s) = self.suggest.get(self.sug_sel) { self.input = s.clone(); }
                            }
                        }
                        self.mode = if self.return_mode_detail { Mode::Detail } else { Mode::Board };
                        self.suggest.clear();
                        self.submit(kind).await?;
                    }
                    KeyCode::Backspace => { self.input.pop(); self.refresh_suggest(kind).await; }
                    KeyCode::Down if suggesting => { if !self.suggest.is_empty() { self.sug_sel = (self.sug_sel + 1) % self.suggest.len(); } }
                    KeyCode::Up if suggesting => { if !self.suggest.is_empty() { self.sug_sel = (self.sug_sel + self.suggest.len() - 1) % self.suggest.len(); } }
                    KeyCode::Tab if suggesting => {
                        if let Some(s) = self.suggest.get(self.sug_sel).cloned() { self.input = s; self.refresh_suggest(kind).await; }
                    }
                    KeyCode::Char('r') if kind == PromptKind::AddMember && k.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.sug_role = match self.sug_role { Role::Viewer => Role::Member, Role::Member => Role::Admin, _ => Role::Viewer };
                    }
                    KeyCode::Char(c) if !k.modifiers.contains(KeyModifiers::CONTROL) => { self.input.push(c); self.refresh_suggest(kind).await; }
                    _ => {}
                }
                Ok(())
            }
            Mode::Help => {
                match k.code {
                    KeyCode::Esc => { if self.help_q.is_empty() { self.mode = Mode::Board; } else { self.help_q.clear(); } }
                    KeyCode::Enter => self.mode = Mode::Board,
                    KeyCode::Backspace => { self.help_q.pop(); }
                    KeyCode::Char(c) if !k.modifiers.contains(KeyModifiers::CONTROL) => self.help_q.push(c),
                    _ => {}
                }
                Ok(())
            }
            Mode::Config => {
                match k.code {
                    KeyCode::Esc | KeyCode::Char('q') => self.mode = Mode::Board,
                    KeyCode::Down | KeyCode::Char('j') => self.cfg_sel = (self.cfg_sel + 1) % CONFIG_ITEMS,
                    KeyCode::Up | KeyCode::Char('k') => self.cfg_sel = (self.cfg_sel + CONFIG_ITEMS - 1) % CONFIG_ITEMS,
                    KeyCode::Enter => {
                        let i = self.cfg_sel;
                        self.mode = Mode::Board;
                        self.config_action(i).await?;
                    }
                    _ => {}
                }
                Ok(())
            }
            Mode::Prefix => {
                self.mode = Mode::Board;
                if let KeyCode::Char(c) = k.code { self.key_prefix(c).await?; }
                Ok(())
            }
            Mode::Inbox => {
                let n = self.inbox.as_ref().map(|i| i.tasks.len()).unwrap_or(0);
                match k.code {
                    KeyCode::Esc | KeyCode::Char('q') => self.close_inbox().await,
                    KeyCode::Down | KeyCode::Char('j') => { if n > 0 { self.inbox_sel = (self.inbox_sel + 1).min(n - 1); } }
                    KeyCode::Up | KeyCode::Char('k') => self.inbox_sel = self.inbox_sel.saturating_sub(1),
                    KeyCode::Enter => {
                        let it = self.inbox.as_ref().and_then(|i| i.tasks.get(self.inbox_sel)).map(|t| (t.team.clone(), t.task.clone()));
                        self.close_inbox().await;
                        if let Some((team, task)) = it { self.jump_to(&team, &task).await?; }
                    }
                    _ => {}
                }
                Ok(())
            }
            Mode::Detail => {
                match k.code {
                    KeyCode::Esc | KeyCode::Char('q') => self.mode = Mode::Board,
                    KeyCode::Char('c') => { self.return_mode_detail = true; self.mode = Mode::Prompt(PromptKind::Comment); }
                    KeyCode::Char('s') => { self.input.clear(); self.return_mode_detail = true; self.mode = Mode::Prompt(PromptKind::Subtask); }
                    KeyCode::Char('l') => { self.input.clear(); self.return_mode_detail = true; self.mode = Mode::Prompt(PromptKind::Label); }
                    KeyCode::Down | KeyCode::Char('j') => {
                        let n = self.selected().map(|t| self.subtasks(t.id).len()).unwrap_or(0);
                        if n > 0 { self.sub = (self.sub + 1).min(n - 1); }
                    }
                    KeyCode::Up | KeyCode::Char('k') => self.sub = self.sub.saturating_sub(1),
                    KeyCode::Char(' ') | KeyCode::Char('x') => {
                        let sub = self.selected().and_then(|t| self.subtasks(t.id).get(self.sub).map(|s| (s.id, s.status)));
                        if let Some((id, st)) = sub {
                            let next = if st == Status::Done { Status::Todo } else { Status::Done };
                            let t = self.api.patch_task(id, &TaskPatch { status: Some(next), ..Default::default() }).await?;
                            self.upsert(t);
                        }
                    }
                    KeyCode::Char('d') => {
                        let sub = self.selected().and_then(|t| self.subtasks(t.id).get(self.sub).map(|s| s.id));
                        if let Some(id) = sub {
                            self.api.delete_task(id).await?;
                            self.tasks.retain(|t| t.id != id);
                            self.sub = self.sub.saturating_sub(1);
                            if let Some(pid) = self.selected().map(|t| t.id) {
                                if let Some(p) = self.api.tasks(self.cur_project()).await?.into_iter().find(|t| t.id == pid) { self.upsert(p); }
                            }
                        }
                    }
                    _ => {}
                }
                Ok(())
            }
            Mode::Board => self.key_board(k).await,
        }
    }

    /// Acción de cada fila del menú de configuración (el orden coincide con `ui::config_rows`).
    async fn config_action(&mut self, i: usize) -> Result<()> {
        let has_team = !self.teams.is_empty();
        match i {
            0 => self.prompt(PromptKind::SetEmail),
            1 => self.prompt(PromptKind::VerifyEmail),
            2 => {
                if self.email_info.as_ref().is_some_and(|m| m.telegram_linked) {
                    self.api.telegram_unlink().await?;
                    self.load_me().await;
                    self.status = "ℹ Telegram desvinculado".into();
                } else {
                    self.key_prefix('g').await?;
                }
            }
            3 => self.open_inbox(true).await,
            4 if has_team => self.prompt(PromptKind::TeamMessage),
            5 if has_team => self.open_prompt_suggest(PromptKind::AddMember),
            6 if has_team => self.prompt(PromptKind::SetRole),
            7 if has_team => self.key_prefix('i').await?,
            8 => self.prompt(PromptKind::JoinCode),
            9 => self.prompt(PromptKind::NewTeam),
            10 => {
                self.api.logout_all().await?;
                config::save(&config::Config { server: Some(self.api.base.clone()), token: None, username: self.me.as_ref().map(|u| u.username.clone()) });
                self.quit = true;
            }
            11 => { self.help_q.clear(); self.mode = Mode::Help; }
            _ => self.status = "✗ primero creá o elegí un equipo".into(),
        }
        Ok(())
    }

    async fn key_prefix(&mut self, c: char) -> Result<()> {
        let has_team = !self.teams.is_empty();
        match c {
            'm' if has_team => self.open_prompt_suggest(PromptKind::AddMember),
            'r' if has_team => self.prompt(PromptKind::SetRole),
            'i' if has_team => {
                let inv = self.api.create_invite(self.cur_team(), Role::Member).await?;
                self.status = format!("ℹ código de invitación (rol miembro, 7 días, 10 usos): {}", inv.code);
            }
            'j' => self.prompt(PromptKind::JoinCode),
            't' => self.prompt(PromptKind::NewTeam),
            'e' => self.prompt(PromptKind::SetEmail),
            'v' => self.prompt(PromptKind::VerifyEmail),
            's' if has_team => self.prompt(PromptKind::TeamMessage),
            'g' => {
                let l = self.api.telegram_link().await?;
                self.status = format!("ℹ Telegram: escribile a @{} → /start {}  (un solo uso, vence en {} min)", l.bot, l.code, l.expires_minutes);
            }
            'n' => self.open_inbox(true).await,
            'c' => { self.load_me().await; self.mode = Mode::Config; }
            _ => self.status = "✗ atajo de equipo desconocido (m r i j t e v n)".into(),
        }
        Ok(())
    }

    async fn key_login(&mut self, k: KeyEvent, tx: &mpsc::UnboundedSender<Msg>) -> Result<()> {
        match k.code {
            KeyCode::Esc => self.quit = true,
            KeyCode::Tab | KeyCode::Down => self.field = (self.field + 1) % 3,
            KeyCode::BackTab | KeyCode::Up => self.field = (self.field + 2) % 3,
            KeyCode::F(2) => self.register = !self.register,
            KeyCode::Backspace => { self.login[self.field].pop(); }
            KeyCode::Char(c) => self.login[self.field].push(c),
            KeyCode::Enter => {
                self.api = Api::new(&self.login[0], None);
                match self.api.auth(self.register, &self.login[1], &self.login[2]).await {
                    Ok(a) => { self.login[2].clear(); self.status.clear(); self.enter_session(a, tx).await?; }
                    Err(e) => self.status = format!("✗ {e}"),
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn key_board(&mut self, k: KeyEvent) -> Result<()> {
        let n = self.column(self.col).len();
        match k.code {
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Char('?') => { self.help_q.clear(); self.mode = Mode::Help; }
            KeyCode::Char('t') if k.modifiers.contains(KeyModifiers::CONTROL) => self.mode = Mode::Prefix,
            KeyCode::Char('N') => self.open_inbox(true).await,
            KeyCode::Char('C') => { self.load_me().await; self.mode = Mode::Config; }
            KeyCode::Char('A') if self.selected().is_some() => {
                if self.my_role().is_some_and(|r| r >= Role::Admin) {
                    self.open_prompt_suggest(PromptKind::AssignTo);
                    self.refresh_suggest(PromptKind::AssignTo).await;
                } else {
                    self.status = "✗ solo admin o dueño asigna a otros (con a tomás o soltás la tuya)".into();
                }
            }
            KeyCode::Char('X') => {
                self.api.logout_all().await?;
                config::save(&config::Config { server: Some(self.api.base.clone()), token: None, username: self.me.as_ref().map(|u| u.username.clone()) });
                self.quit = true;
            }
            KeyCode::Left | KeyCode::Char('h') => self.col = (self.col + 2) % 3,
            KeyCode::Right | KeyCode::Char('l') => self.col = (self.col + 1) % 3,
            KeyCode::Down | KeyCode::Char('j') => { if n > 0 { self.row[self.col] = (self.row[self.col] + 1).min(n - 1); } }
            KeyCode::Up | KeyCode::Char('k') => self.row[self.col] = self.row[self.col].saturating_sub(1),
            KeyCode::Char('n') if !self.projects.is_empty() => self.prompt(PromptKind::NewTask),
            KeyCode::Char('P') if !self.teams.is_empty() => self.prompt(PromptKind::NewProject),
            KeyCode::Char('T') => self.prompt(PromptKind::NewTeam),
            KeyCode::Char('m') if !self.teams.is_empty() => self.open_prompt_suggest(PromptKind::AddMember),
            KeyCode::Char('R') if !self.teams.is_empty() => self.prompt(PromptKind::SetRole),
            KeyCode::Char('J') => self.prompt(PromptKind::JoinCode),
            KeyCode::Char('I') if !self.teams.is_empty() => {
                let inv = self.api.create_invite(self.cur_team(), Role::Member).await?;
                self.status = format!("ℹ código de invitación (rol miembro, 7 días, 10 usos): {}", inv.code);
            }
            KeyCode::Char('e') if self.selected().is_some() => {
                self.input = self.selected().map(|t| t.title.clone()).unwrap_or_default();
                self.prompt_keep(PromptKind::EditTitle);
            }
            KeyCode::Char('[') | KeyCode::Char(']') if self.teams.len() > 1 => {
                let d = if k.code == KeyCode::Char(']') { 1 } else { self.teams.len() - 1 };
                self.team = (self.team + d) % self.teams.len();
                self.project = 0;
                self.load_team().await?;
            }
            KeyCode::Tab | KeyCode::BackTab if self.projects.len() > 1 => {
                let d = if k.code == KeyCode::Tab { 1 } else { self.projects.len() - 1 };
                self.project = (self.project + d) % self.projects.len();
                self.load_project().await?;
            }
            KeyCode::Char('H') | KeyCode::Char('L') => {
                if let Some(t) = self.selected() {
                    let i = Status::ALL.iter().position(|s| *s == t.status).unwrap() as i32;
                    let ni = i + if k.code == KeyCode::Char('L') { 1 } else { -1 };
                    if (0..3).contains(&ni) {
                        let id = t.id;
                        let t = self.api.patch_task(id, &TaskPatch { status: Some(Status::ALL[ni as usize]), ..Default::default() }).await?;
                        self.upsert(t);
                        self.col = ni as usize;
                        self.row[self.col] = self.column(self.col).iter().position(|x| x.id == id).unwrap_or(0);
                    }
                }
            }
            KeyCode::Char('+') | KeyCode::Char('-') => {
                if let Some(t) = self.selected() {
                    let p = (t.priority + if k.code == KeyCode::Char('+') { 1 } else { -1 }).clamp(0, 3);
                    let t = self.api.patch_task(t.id, &TaskPatch { priority: Some(p), ..Default::default() }).await?;
                    self.upsert(t);
                }
            }
            KeyCode::Char('a') => {
                if let (Some(t), Some(me)) = (self.selected(), self.me.clone()) {
                    // alterna: asignarme / quitar asignación
                    let v = if t.assignee_id == Some(me.id) { None } else { Some(me.id) };
                    let t = self.api.patch_task(t.id, &TaskPatch { assignee_id: Some(v), ..Default::default() }).await?;
                    self.upsert(t);
                }
            }
            KeyCode::Char('d') => {
                if let Some(id) = self.selected().map(|t| t.id) {
                    self.api.delete_task(id).await?;
                    self.tasks.retain(|t| t.id != id);
                    self.sort_tasks();
                }
            }
            KeyCode::Enter => {
                if let Some(id) = self.selected().map(|t| t.id) {
                    self.comments = self.api.comments(id).await?;
                    self.sub = 0;
                    self.mode = Mode::Detail;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn prompt(&mut self, kind: PromptKind) {
        self.input.clear();
        self.prompt_keep(kind);
    }

    fn prompt_keep(&mut self, kind: PromptKind) {
        self.return_mode_detail = false;
        self.mode = Mode::Prompt(kind);
    }
}

fn parse_role(s: &str) -> Result<Role> {
    match s.to_lowercase().as_str() {
        "viewer" | "lector" => Ok(Role::Viewer),
        "member" | "miembro" => Ok(Role::Member),
        "admin" => Ok(Role::Admin),
        _ => Err(anyhow::anyhow!("rol inválido: usá viewer, member o admin")),
    }
}

fn spawn_ws(url: String, token: String, tx: mpsc::UnboundedSender<Msg>) {
    use futures_util::SinkExt;
    tokio::spawn(async move {
        loop {
            if let Ok((mut ws, _)) = tokio_tungstenite::connect_async(&url).await {
                let hello = serde_json::json!({ "token": token }).to_string();
                if ws.send(tokio_tungstenite::tungstenite::Message::Text(hello.into())).await.is_ok() {
                    let _ = tx.send(Msg::Connected(true));
                }
                while let Some(Ok(m)) = ws.next().await {
                    if let Ok(text) = m.into_text() {
                        if let Ok(ev) = serde_json::from_str::<tm_core::Event>(&text) {
                            if tx.send(Msg::Server(ev)).is_err() { return; }
                        }
                    }
                }
            }
            if tx.send(Msg::Connected(false)).is_err() { return; }
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
    });
}

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = config::load();
    let server = std::env::args().nth(1).or(cfg.server.clone()).unwrap_or_else(|| "http://127.0.0.1:7878".into());
    let mut app = App {
        api: Api::new(&server, None),
        mode: Mode::Login,
        return_mode_detail: false,
        me: None,
        field: 1,
        login: [server.clone(), cfg.username.clone().unwrap_or_default(), String::new()],
        register: false,
        teams: vec![],
        team: 0,
        projects: vec![],
        project: 0,
        tasks: vec![],
        members: vec![],
        labels: vec![],
        comments: vec![],
        sub: 0,
        col: 0,
        row: [0; 3],
        input: String::new(),
        status: String::new(),
        online: false,
        quit: false,
        suggest: vec![],
        sug_sel: 0,
        sug_role: Role::Member,
        sug_q: String::new(),
        sug_full: false,
        help_q: String::new(),
        inbox: None,
        inbox_sel: 0,
        unread: 0,
        email_info: None,
        cfg_sel: 0,
    };

    let (tx, mut rx) = mpsc::unbounded_channel::<Msg>();

    // teclado en un hilo bloqueante
    let ktx = tx.clone();
    std::thread::spawn(move || loop {
        if event::poll(Duration::from_millis(250)).unwrap_or(false) {
            if let Ok(CEvent::Key(k)) = event::read() {
                if k.kind == KeyEventKind::Press && ktx.send(Msg::Key(k)).is_err() { break; }
            }
        } else if ktx.is_closed() {
            break;
        }
    });

    // sesión guardada
    if let Some(token) = cfg.token {
        app.api = Api::new(&server, Some(token.clone()));
        match app.api.teams().await {
            Ok(_) => {
                app.me = Some(User { id: 0, username: cfg.username.unwrap_or_default() });
                app.mode = Mode::Board;
                spawn_ws(app.api.ws_url(), token.clone(), tx.clone());
                let _ = app.load_teams().await;
                app.load_me().await;
                app.open_inbox(false).await;
            }
            Err(_) => app.api.token = None,
        }
    }

    enable_raw_mode()?;
    let mut out = std::io::stdout();
    execute!(out, EnterAlternateScreen)?;
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |i| {
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
        hook(i);
    }));
    let mut term = Terminal::new(CrosstermBackend::new(out))?;

    while !app.quit {
        term.draw(|f| ui::draw(f, &app))?;
        match rx.recv().await {
            Some(Msg::Key(k)) => {
                if !matches!(app.mode, Mode::Login) { app.status.clear(); }
                if let Err(e) = app.key(k, &tx).await { app.status = format!("✗ {e}"); }
            }
            Some(Msg::Server(ev)) => app.on_server(ev).await,
            Some(Msg::Connected(c)) => app.online = c,
            None => break,
        }
    }

    disable_raw_mode()?;
    execute!(term.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
