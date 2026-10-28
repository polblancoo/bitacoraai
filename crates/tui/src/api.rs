use anyhow::{anyhow, Result};
use serde::{de::DeserializeOwned, Serialize};
use tm_core::*;

#[derive(Clone)]
pub struct Api {
    pub base: String,
    pub token: Option<String>,
    http: reqwest::Client,
}

impl Api {
    pub fn new(base: &str, token: Option<String>) -> Self {
        Self { base: base.trim_end_matches('/').into(), token, http: reqwest::Client::new() }
    }

    /// URL del WebSocket; el token NO va en la URL: se envía como primer mensaje.
    pub fn ws_url(&self) -> String {
        let b = self.base.replacen("https://", "wss://", 1).replacen("http://", "ws://", 1);
        format!("{b}/ws")
    }

    async fn put<T: DeserializeOwned>(&self, path: &str, body: &impl Serialize) -> Result<T> {
        self.send(self.http.put(format!("{}{path}", self.base)).json(body)).await
    }

    async fn send<T: DeserializeOwned>(&self, rb: reqwest::RequestBuilder) -> Result<T> {
        let rb = match &self.token {
            Some(t) => rb.bearer_auth(t),
            None => rb,
        };
        let resp = rb.send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let msg = resp.json::<serde_json::Value>().await.ok().and_then(|v| v["error"].as_str().map(String::from));
            return Err(anyhow!("{}", msg.unwrap_or_else(|| status.to_string())));
        }
        Ok(resp.json().await?)
    }

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.send(self.http.get(format!("{}{path}", self.base))).await
    }

    async fn post<T: DeserializeOwned>(&self, path: &str, body: &impl Serialize) -> Result<T> {
        self.send(self.http.post(format!("{}{path}", self.base)).json(body)).await
    }

    pub async fn auth(&self, register: bool, user: &str, pass: &str) -> Result<AuthResponse> {
        let path = if register { "/api/register" } else { "/api/login" };
        self.post(path, &Credentials { username: user.into(), password: pass.into() }).await
    }

    pub async fn teams(&self) -> Result<Vec<Team>> { self.get("/api/teams").await }
    pub async fn projects(&self, team: Id) -> Result<Vec<Project>> { self.get(&format!("/api/teams/{team}/projects")).await }
    pub async fn members(&self, team: Id) -> Result<Vec<Member>> { self.get(&format!("/api/teams/{team}/members")).await }
    pub async fn tasks(&self, project: Id) -> Result<Vec<Task>> { self.get(&format!("/api/projects/{project}/tasks")).await }
    pub async fn comments(&self, task: Id) -> Result<Vec<Comment>> { self.get(&format!("/api/tasks/{task}/comments")).await }

    pub async fn new_team(&self, name: &str) -> Result<Team> { self.post("/api/teams", &NewTeam { name: name.into() }).await }
    pub async fn add_member(&self, team: Id, username: &str, role: Option<Role>) -> Result<User> {
        self.post(&format!("/api/teams/{team}/members"), &AddMember { username: username.into(), role }).await
    }
    pub async fn set_role(&self, team: Id, user: Id, role: Role) -> Result<()> {
        let rb = self.http.put(format!("{}/api/teams/{team}/members/{user}", self.base)).json(&SetRole { role });
        self.send_empty(rb).await
    }
    pub async fn create_invite(&self, team: Id, role: Role) -> Result<InviteCreated> {
        self.post(&format!("/api/teams/{team}/invites"), &NewInvite { role, max_uses: None, expires_hours: None }).await
    }
    pub async fn accept_invite(&self, code: &str) -> Result<Team> {
        self.post(&format!("/api/invites/{code}/accept"), &serde_json::json!({})).await
    }
    async fn send_empty(&self, mut rb: reqwest::RequestBuilder) -> Result<()> {
        if let Some(t) = &self.token { rb = rb.bearer_auth(t); }
        let resp = rb.send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let msg = resp.json::<serde_json::Value>().await.ok().and_then(|v| v["error"].as_str().map(String::from));
            return Err(anyhow!("{}", msg.unwrap_or_else(|| status.to_string())));
        }
        Ok(())
    }
    pub async fn new_project(&self, team: Id, name: &str) -> Result<Project> {
        self.post(&format!("/api/teams/{team}/projects"), &NewProject { name: name.into(), description: String::new(), color: None }).await
    }
    pub async fn new_task(&self, project: Id, title: &str) -> Result<Task> {
        self.post(&format!("/api/projects/{project}/tasks"), &NewTask { title: title.into(), priority: 1, ..Default::default() }).await
    }
    pub async fn patch_task(&self, id: Id, patch: &TaskPatch) -> Result<Task> {
        self.send(self.http.patch(format!("{}/api/tasks/{id}", self.base)).json(patch)).await
    }
    pub async fn delete_task(&self, id: Id) -> Result<()> {
        let mut rb = self.http.delete(format!("{}/api/tasks/{id}", self.base));
        if let Some(t) = &self.token { rb = rb.bearer_auth(t); }
        rb.send().await?.error_for_status()?;
        Ok(())
    }
    pub async fn team_labels(&self, team: Id) -> Result<Vec<Label>> { self.get(&format!("/api/teams/{team}/labels")).await }
    pub async fn new_label(&self, team: Id, name: &str, color: &str) -> Result<Label> {
        self.post(&format!("/api/teams/{team}/labels"), &NewLabel { name: name.into(), color: Some(color.into()) }).await
    }
    pub async fn set_labels(&self, task: Id, ids: Vec<Id>) -> Result<Task> {
        self.put(&format!("/api/tasks/{task}/labels"), &SetLabels { label_ids: ids }).await
    }
    pub async fn new_subtask(&self, project: Id, parent: Id, title: &str) -> Result<Task> {
        self.post(
            &format!("/api/projects/{project}/tasks"),
            &NewTask { title: title.into(), priority: 1, parent_id: Some(parent), ..Default::default() },
        )
        .await
    }
    pub async fn logout_all(&self) -> Result<()> {
        let mut rb = self.http.post(format!("{}/api/logout-all", self.base));
        if let Some(t) = &self.token { rb = rb.bearer_auth(t); }
        rb.send().await?.error_for_status()?;
        Ok(())
    }

    pub async fn new_comment(&self, task: Id, body: &str) -> Result<Comment> {
        self.post(&format!("/api/tasks/{task}/comments"), &NewComment { body: body.into() }).await
    }

    pub async fn me(&self) -> Result<Me> { self.get("/api/me").await }
    pub async fn inbox(&self) -> Result<Inbox> { self.get("/api/inbox").await }
    pub async fn search_users(&self, team: Id, q: &str) -> Result<Vec<User>> {
        let rb = self.http.get(format!("{}/api/users/search", self.base)).query(&[("q", q), ("team_id", &team.to_string())]);
        self.send(rb).await
    }
    pub async fn mark_read(&self, ids: Vec<Id>) -> Result<()> {
        let rb = self.http.post(format!("{}/api/notifications/read", self.base)).json(&MarkRead { ids });
        self.send_empty(rb).await
    }
    pub async fn set_email(&self, email: &str) -> Result<()> {
        let rb = self.http.put(format!("{}/api/me/email", self.base)).json(&SetEmail { email: email.into() });
        self.send_empty(rb).await
    }
    pub async fn verify_email(&self, code: &str) -> Result<()> {
        let rb = self.http.post(format!("{}/api/me/email/verify", self.base)).json(&VerifyEmail { code: code.into() });
        self.send_empty(rb).await
    }
    pub async fn telegram_link(&self) -> Result<TelegramLink> { self.post("/api/me/telegram/link", &serde_json::json!({})).await }
    pub async fn send_message(&self, team: Id, text: &str) -> Result<MessageSent> {
        self.post(&format!("/api/teams/{team}/messages"), &TeamMessage { text: text.into(), to: vec![] }).await
    }
    pub async fn telegram_unlink(&self) -> Result<()> {
        self.send_empty(self.http.delete(format!("{}/api/me/telegram", self.base))).await
    }
}
