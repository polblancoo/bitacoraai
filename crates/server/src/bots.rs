//! Cuentas de IA: un usuario marcado como bot con un token de API propio (`tmk_…`), sumado a un equipo con
//! rol limitado. Lo crea y lo revoca una persona admin; el bot no puede administrar equipos ni invitaciones.
use crate::{
    auth::{hash_password, random_hex, sha256_hex, AuthUser, API_TOKEN_PREFIX},
    error::{AppError, Result},
    limits,
    routes::{require, role_of},
    AppState,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use tm_core::{Id, Role, User};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/teams/{id}/bots", get(list_bots).post(create_bot))
        .route("/api/bots/{uid}/token", post(rotate).delete(revoke))
        .route("/api/bots/{uid}", axum::routing::patch(update_bot))
        .route("/api/commands", get(general_cmds).put(set_general_cmds))
        .route("/api/me/deny", get(my_deny))
        .route("/api/me/allow", get(my_allow))
        .route("/api/bots/{uid}/suggest", get(suggest_cmds))
        .route("/api/me/base-lists", post(set_base_lists))
        .route("/api/tasks/{id}/grant", post(grant_cmd))
        .route("/api/tasks/{id}/grants", get(task_grants))
        .route("/api/tasks/{id}/grants/claim", post(claim_grants))
}

#[derive(Deserialize)]
struct NewBot {
    name: String,
    /// `viewer` o `member` (por defecto).
    role: Option<Role>,
    /// Etiqueta de qué IA es, por ejemplo `claude:Opus` (solo informativa).
    #[serde(default)]
    model: String,
}

#[derive(Deserialize)]
struct UpdateBot {
    model: Option<String>,
    /// Comandos prohibidos para esta IA, uno por línea (por ejemplo `Bash(rm:*)`).
    deny: Option<String>,
    /// Comandos permitidos de forma permanente para esta IA, uno por línea (por ejemplo `git clone`).
    allow: Option<String>,
    /// Autorización total: la IA puede ejecutar todo salvo lo prohibido (las listas de prohibidos siempre ganan).
    full_access: Option<bool>,
    /// Las listas se manejan desde la app: el agente ignora whitelist.txt / blacklist.txt.
    lists_from_app: Option<bool>,
}

fn new_token() -> String {
    format!("{API_TOKEN_PREFIX}{}", random_hex(24))
}

pub(crate) async fn store_token(st: &AppState, bot: Id, by: Id) -> Result<String> {
    let token = new_token();
    sqlx::query("INSERT OR REPLACE INTO api_tokens (user_id, hash, created_by, created_at) VALUES (?, ?, ?, ?)")
        .bind(bot)
        .bind(sha256_hex(&token))
        .bind(by)
        .bind(Utc::now())
        .execute(&st.db)
        .await?;
    Ok(token)
}

/// El llamante debe ser admin+ en algún equipo al que pertenezca el bot.
async fn manages_bot(st: &AppState, caller: Id, bot: Id) -> Result<()> {
    let is_bot: Option<i64> = sqlx::query_scalar("SELECT is_bot FROM users WHERE id = ?").bind(bot).fetch_optional(&st.db).await?;
    if is_bot != Some(1) {
        return Err(AppError::not_found());
    }
    let teams: Vec<Id> = sqlx::query_scalar("SELECT team_id FROM team_members WHERE user_id = ?").bind(bot).fetch_all(&st.db).await?;
    for t in teams {
        if role_of(st, t, caller).await.is_ok_and(|r| r >= Role::Admin) {
            return Ok(());
        }
    }
    Err(AppError::forbidden())
}

async fn list_bots(State(st): State<AppState>, user: AuthUser, Path(team): Path<Id>) -> Result<Json<Value>> {
    require(&st, team, user.id, Role::Viewer).await?;
    let rows = sqlx::query(
        "SELECT u.id, u.username, u.model, u.deny, u.allow, u.full_access, u.base_allow, u.base_deny, u.lists_from_app, m.role, m.joined_at, cb.username AS created_by, t.last_used, t.id AS tok, COALESCE(t.last_used >= ?, 0) AS online, COALESCE(r.enabled, 0) AS run_on \
         FROM team_members m JOIN users u ON u.id = m.user_id \
         LEFT JOIN api_tokens t ON t.user_id = u.id LEFT JOIN users cb ON cb.id = t.created_by LEFT JOIN bot_runners r ON r.user_id = u.id \
         WHERE m.team_id = ? AND u.is_bot = 1 ORDER BY u.username",
    )
    .bind(chrono::Utc::now() - chrono::Duration::seconds(90))
    .bind(team)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(Value::Array(
        rows.iter()
            .map(|r| {
                json!({
                    "id": r.get::<Id, _>("id"),
                    "username": r.get::<String, _>("username"),
                    "model": r.get::<String, _>("model"),
                    "deny": r.get::<String, _>("deny"),
                    "allow": r.get::<String, _>("allow"),
                    "full_access": r.get::<i64, _>("full_access") != 0,
                    "base_allow": r.get::<String, _>("base_allow"),
                    "base_deny": r.get::<String, _>("base_deny"),
                    "lists_from_app": r.get::<i64, _>("lists_from_app") != 0,
                    "role": r.get::<String, _>("role"),
                    "joined_at": r.get::<Option<String>, _>("joined_at"),
                    "created_by": r.get::<Option<String>, _>("created_by"),
                    "last_used": r.get::<Option<String>, _>("last_used"),
                    "active": r.get::<Option<Id>, _>("tok").is_some(),
                    "online": r.get::<i64, _>("online") != 0,
                    "run_on": r.get::<i64, _>("run_on") != 0,
                })
            })
            .collect(),
    )))
}

async fn create_bot(
    State(st): State<AppState>,
    user: AuthUser,
    Path(team): Path<Id>,
    Json(b): Json<NewBot>,
) -> Result<Json<Value>> {
    user.human_only()?;
    require(&st, team, user.id, Role::Admin).await?;
    let role = b.role.unwrap_or(Role::Member);
    if !matches!(role, Role::Viewer | Role::Member) {
        return Err(AppError::bad("un bot solo puede ser lector o miembro"));
    }
    let name = b.name.trim();
    limits::username(name)?;
    limits::max_len("modelo", &b.model, 100)?;
    let hash = hash_password(&random_hex(32))?;
    let res = sqlx::query("INSERT INTO users (username, password_hash, is_bot, model) VALUES (?, ?, 1, ?)")
        .bind(name)
        .bind(hash)
        .bind(b.model.trim())
        .execute(&st.db)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(d) if d.is_unique_violation() => AppError(StatusCode::CONFLICT, "el usuario ya existe".into()),
            e => e.into(),
        })?;
    let id = res.last_insert_rowid();
    crate::routes::insert_member(&st, team, &User { id, username: name.into() }, role, Some(user.id), "bot", None).await?;
    let token = store_token(&st, id, user.id).await?;
    Ok(Json(json!({ "user": { "id": id, "username": name }, "model": b.model.trim(), "role": role.as_str(), "token": token })))
}

async fn rotate(State(st): State<AppState>, user: AuthUser, Path(uid): Path<Id>) -> Result<Json<Value>> {
    user.human_only()?;
    manages_bot(&st, user.id, uid).await?;
    Ok(Json(json!({ "token": store_token(&st, uid, user.id).await? })))
}

async fn revoke(State(st): State<AppState>, user: AuthUser, Path(uid): Path<Id>) -> Result<StatusCode> {
    user.human_only()?;
    manages_bot(&st, user.id, uid).await?;
    sqlx::query("DELETE FROM api_tokens WHERE user_id = ?").bind(uid).execute(&st.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn update_bot(State(st): State<AppState>, user: AuthUser, Path(uid): Path<Id>, Json(b): Json<UpdateBot>) -> Result<StatusCode> {
    user.human_only()?;
    manages_bot(&st, user.id, uid).await?;
    if let Some(model) = &b.model {
        limits::max_len("modelo", model, 100)?;
        sqlx::query("UPDATE users SET model = ? WHERE id = ?").bind(model.trim()).bind(uid).execute(&st.db).await?;
    }
    if let Some(deny) = &b.deny {
        limits::max_len("lista de comandos prohibidos", deny, 4000)?;
        let clean = deny.lines().map(str::trim).filter(|l| !l.is_empty()).collect::<Vec<_>>().join("\n");
        sqlx::query("UPDATE users SET deny = ? WHERE id = ?").bind(clean).bind(uid).execute(&st.db).await?;
    }
    if let Some(full) = b.full_access {
        sqlx::query("UPDATE users SET full_access = ? WHERE id = ?").bind(full).bind(uid).execute(&st.db).await?;
    }
    if let Some(v) = b.lists_from_app {
        sqlx::query("UPDATE users SET lists_from_app = ? WHERE id = ?").bind(v).bind(uid).execute(&st.db).await?;
    }
    if let Some(allow) = &b.allow {
        limits::max_len("lista de comandos permitidos", allow, 4000)?;
        let clean = allow.lines().map(str::trim).filter(|l| !l.is_empty()).collect::<Vec<_>>().join("\n");
        sqlx::query("UPDATE users SET allow = ? WHERE id = ?").bind(clean).bind(uid).execute(&st.db).await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// La IA lee su propia lista de comandos prohibidos (la edita un admin desde la app).
async fn my_deny(State(st): State<AppState>, user: AuthUser) -> Result<Json<Value>> {
    let own: String = sqlx::query_scalar("SELECT deny FROM users WHERE id = ?").bind(user.id).fetch_one(&st.db).await?;
    Ok(Json(json!({ "deny": join_lines(&general(&st, "cmd_deny").await, &own) })))
}

/// La IA lee su lista permanente de comandos permitidos (se agrega con «/permitir siempre …» o desde el pedido).
async fn my_allow(State(st): State<AppState>, user: AuthUser) -> Result<Json<Value>> {
    let own: String = sqlx::query_scalar("SELECT allow FROM users WHERE id = ?").bind(user.id).fetch_one(&st.db).await?;
    let (full, from_app): (i64, i64) = sqlx::query_as("SELECT full_access, lists_from_app FROM users WHERE id = ?").bind(user.id).fetch_one(&st.db).await?;
    Ok(Json(json!({ "allow": join_lines(&general(&st, "cmd_allow").await, &own), "full": full != 0, "from_app": from_app != 0 })))
}

/// Tarea a la que se aplica el permiso (si es un pedido «🙋 Pedido de #N: …», la tarea N) y la IA responsable.
async fn grant_target(st: &AppState, team: Id, task: Id) -> Result<(Id, Id)> {
    let title: String = sqlx::query_scalar("SELECT title FROM tasks WHERE id = ?").bind(task).fetch_one(&st.db).await?;
    let origin = title.strip_prefix("🙋 Pedido de #").and_then(|t| t.split(':').next()).and_then(|n| n.parse::<Id>().ok()).filter(|o| *o != task);
    let target = match origin {
        Some(o) => sqlx::query_scalar("SELECT t.id FROM tasks t JOIN projects p ON p.id = t.project_id WHERE t.id = ? AND p.team_id = ?")
            .bind(o)
            .bind(team)
            .fetch_optional(&st.db)
            .await?
            .unwrap_or(task),
        None => task,
    };
    let bot: Option<Id> = sqlx::query_scalar("SELECT u.id FROM tasks t JOIN users u ON u.id = t.assignee_id WHERE t.id = ? AND u.is_bot = 1")
        .bind(target)
        .fetch_optional(&st.db)
        .await?;
    let bot = bot.ok_or_else(|| AppError::bad("la tarea no está asignada a una IA: no hay a quién darle el permiso"))?;
    Ok((target, bot))
}

/// Da un permiso: `always` lo suma a la lista de la IA; si no, vale para la próxima ejecución de esa tarea.
pub(crate) async fn grant(st: &AppState, user: &AuthUser, task: Id, command: &str, always: bool) -> Result<Id> {
    user.human_only()?;
    let team = crate::routes::team_of_task(st, task).await?;
    crate::routes::require(st, team, user.id, Role::Admin).await?;
    let command = command.trim();
    if command.is_empty() || command.contains('\n') {
        return Err(AppError::bad("falta el comando (una línea), por ejemplo: /permitir cargo build"));
    }
    limits::max_len("comando", command, 1000)?;
    let (target, bot) = grant_target(st, team, task).await?;
    if always {
        let cur: String = sqlx::query_scalar("SELECT allow FROM users WHERE id = ?").bind(bot).fetch_one(&st.db).await?;
        if !cur.lines().any(|l| l == command) {
            limits::max_len("lista de comandos permitidos", &format!("{cur}\n{command}"), 4000)?;
            let new = if cur.is_empty() { command.to_string() } else { format!("{cur}\n{command}") };
            sqlx::query("UPDATE users SET allow = ? WHERE id = ?").bind(new).bind(bot).execute(&st.db).await?;
        }
    } else {
        sqlx::query("INSERT INTO task_grants (task_id, pattern, created_by, created_at, used) VALUES (?, ?, ?, ?, 0)")
            .bind(target)
            .bind(command)
            .bind(user.id)
            .bind(chrono::Utc::now())
            .execute(&st.db)
            .await?;
    }
    crate::extras::log_task(st, target, user.id, format!("autorizó «{command}» ({})", if always { "siempre" } else { "una vez" })).await;
    // si la IA ya había terminado (o fallado) por falta del permiso, la tarea vuelve a «Por hacer» y se reintenta sola
    let requeued = sqlx::query("UPDATE tasks SET status = 'todo', waiting = 0, updated_at = ? WHERE id = ? AND (status = 'done' OR (status = 'doing' AND waiting = 1))")
        .bind(chrono::Utc::now())
        .bind(target)
        .execute(&st.db)
        .await?
        .rows_affected();
    if requeued > 0 {
        if let Ok(t) = crate::routes::fetch_task(st, target).await {
            st.emit(team, tm_core::Event::TaskUpdated(t));
        }
        crate::extras::log_task(st, target, user.id, "volvió a «Por hacer» para reintentar con el permiso".to_string()).await;
    }
    Ok(target)
}

#[derive(Deserialize)]
struct GrantReq {
    command: String,
    #[serde(default)]
    always: bool,
}

async fn grant_cmd(State(st): State<AppState>, user: AuthUser, Path(task): Path<Id>, Json(g): Json<GrantReq>) -> Result<Json<Value>> {
    let target = grant(&st, &user, task, &g.command, g.always).await?;
    Ok(Json(json!({ "task_id": target })))
}

/// Permisos «una vez» de la tarea que todavía no se usaron (los ven las personas del equipo y la IA responsable).
async fn task_grants(State(st): State<AppState>, user: AuthUser, Path(task): Path<Id>) -> Result<Json<Value>> {
    crate::routes::ensure_member(&st, crate::routes::team_of_task(&st, task).await?, user.id).await?;
    let rows: Vec<String> = sqlx::query_scalar("SELECT pattern FROM task_grants WHERE task_id = ? AND used = 0 ORDER BY id LIMIT 50").bind(task).fetch_all(&st.db).await?;
    Ok(Json(json!({ "pending": rows })))
}

/// La IA toma sus permisos «una vez» al empezar: quedan marcados como usados y no vuelven a valer.
async fn claim_grants(State(st): State<AppState>, user: AuthUser, Path(task): Path<Id>) -> Result<Json<Value>> {
    let mine: Option<i64> = sqlx::query_scalar("SELECT 1 FROM tasks WHERE id = ? AND assignee_id = ?").bind(task).bind(user.id).fetch_optional(&st.db).await?;
    mine.ok_or_else(AppError::forbidden)?;
    let rows: Vec<String> = sqlx::query_scalar("UPDATE task_grants SET used = 1 WHERE task_id = ? AND used = 0 RETURNING pattern").bind(task).fetch_all(&st.db).await?;
    Ok(Json(json!({ "once": rows })))
}

async fn general(st: &AppState, key: &str) -> String {
    crate::license::meta_get(&st.db, key).await.unwrap_or_default()
}

fn join_lines(a: &str, b: &str) -> String {
    [a, b].iter().flat_map(|x| x.lines()).map(str::trim).filter(|l| !l.is_empty()).collect::<Vec<_>>().join("\n")
}

/// Listas de comandos que valen para todas las IAs del servidor (además de la propia de cada una).
async fn general_cmds(State(st): State<AppState>, user: AuthUser) -> Result<Json<Value>> {
    user.human_only()?;
    Ok(Json(json!({
        "allow": general(&st, "cmd_allow").await, "deny": general(&st, "cmd_deny").await,
        "can_edit": crate::license::instance_admin(&st.db, user.id).await,
    })))
}

#[derive(Deserialize)]
struct GeneralCmds {
    allow: Option<String>,
    deny: Option<String>,
}

async fn set_general_cmds(State(st): State<AppState>, user: AuthUser, Json(b): Json<GeneralCmds>) -> Result<StatusCode> {
    user.human_only()?;
    if !crate::license::instance_admin(&st.db, user.id).await {
        return Err(AppError::forbidden());
    }
    for (key, v) in [("cmd_allow", &b.allow), ("cmd_deny", &b.deny)] {
        if let Some(v) = v {
            limits::max_len("lista de comandos", v, 4000)?;
            crate::license::meta_set(&st.db, key, &join_lines(v, "")).await?;
        }
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Comandos mencionados en lo que pide una IA: líneas de bloques de código, `comandos` entre acentos graves y 'comandos con espacios' entre comillas simples.
fn commands_in(ask: &str) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    let mut rest = String::new();
    let mut in_block = false;
    for line in ask.lines() {
        if line.trim_start().starts_with("```") {
            in_block = !in_block;
            continue;
        }
        if in_block {
            out.push(line.trim().trim_start_matches("$ ").trim().to_string());
        } else {
            rest.push_str(line);
            rest.push('\n');
        }
    }
    for (i, part) in rest.split('`').enumerate() {
        if i % 2 == 1 && !part.contains('\n') {
            out.push(part.trim().to_string());
        }
    }
    out.into_iter()
        .map(|c| match c.split_once("&&") {
            Some((a, b)) if a.trim_start().starts_with("cd ") => b.trim().to_string(),
            _ => c,
        })
        .filter(|c| c.len() > 1 && c.len() <= 200 && !c.contains('\n'))
        .collect()
}

/// Lo que esta IA pidió antes en sus pedidos «🙋», del más repetido al menos: sirve para proponerlos a la lista de permitidos.
async fn suggest_cmds(State(st): State<AppState>, user: AuthUser, Path(uid): Path<Id>) -> Result<Json<Value>> {
    user.human_only()?;
    manages_bot(&st, user.id, uid).await?;
    let name: String = sqlx::query_scalar("SELECT username FROM users WHERE id = ?").bind(uid).fetch_one(&st.db).await?;
    let own: String = sqlx::query_scalar("SELECT allow FROM users WHERE id = ?").bind(uid).fetch_one(&st.db).await?;
    let allowed = join_lines(&general(&st, "cmd_allow").await, &own);
    let descs: Vec<String> = sqlx::query_scalar(
        "SELECT t.description FROM tasks t JOIN projects p ON p.id = t.project_id \
         WHERE t.title LIKE '🙋 Pedido de #%' AND p.team_id IN (SELECT team_id FROM team_members WHERE user_id = ?) AND t.description LIKE ? \
         ORDER BY t.id DESC LIMIT 300",
    )
    .bind(uid)
    .bind(format!("%La IA «{name}»%"))
    .fetch_all(&st.db)
    .await?;
    let mut count: std::collections::HashMap<String, usize> = Default::default();
    for d in &descs {
        let ask = d.split("LO QUE PIDE\n").nth(1).and_then(|r| r.split("\n\n").next()).unwrap_or("");
        let mut seen = std::collections::HashSet::new();
        for c in commands_in(ask) {
            if seen.insert(c.clone()) {
                *count.entry(c).or_default() += 1;
            }
        }
    }
    let mut list: Vec<(String, usize)> = count.into_iter().collect();
    list.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let asked: Vec<Value> = list.into_iter().take(40).map(|(c, n)| json!({ "cmd": c, "count": n, "allowed": allowed.lines().any(|l| l.trim() == c) })).collect();
    Ok(Json(json!({ "asked": asked, "requests": descs.len() })))
}

#[cfg(test)]
mod suggest_tests {
    use super::commands_in;

    #[test]
    fn saca_comandos_de_un_pedido() {
        let ask = "Necesito compilar:\n```bash\n$ cd /x/y && cargo build --release\ngit status\n```\ny también `npm test` por favor";
        assert_eq!(commands_in(ask), vec!["cargo build --release", "git status", "npm test"]);
    }
}

#[derive(Deserialize)]
struct BaseLists {
    #[serde(default)]
    allow: String,
    #[serde(default)]
    deny: String,
}

/// El agente cuenta qué dicen sus archivos whitelist.txt / blacklist.txt, para que la ficha de la IA los muestre (y se puedan pasar a la app).
async fn set_base_lists(State(st): State<AppState>, user: AuthUser, Json(b): Json<BaseLists>) -> Result<StatusCode> {
    if !user.bot {
        return Err(AppError::forbidden());
    }
    limits::max_len("whitelist", &b.allow, 8000)?;
    limits::max_len("blacklist", &b.deny, 8000)?;
    sqlx::query("UPDATE users SET base_allow = ?, base_deny = ? WHERE id = ?").bind(b.allow.trim()).bind(b.deny.trim()).bind(user.id).execute(&st.db).await?;
    Ok(StatusCode::NO_CONTENT)
}
