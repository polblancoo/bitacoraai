use crate::{
    limits::{self, *},
    auth::{hash_password, make_token, verify_password, AuthUser},
    error::{AppError, Result},
    AppState,
};
use axum::{
    extract::{ConnectInfo, Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post, put},
    Json, Router,
};
use chrono::Utc;
use std::{net::SocketAddr, time::Duration};
use sqlx::{sqlite::SqliteRow, Row};
use tm_core::*;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/setup", get(setup_state))
        .route("/api/register", post(register))
        .route("/api/login", post(login))
        .route("/api/logout-all", post(logout_all))
        .route("/api/teams/{id}/labels", get(list_labels).post(create_label))
        .route("/api/labels/{id}", axum::routing::delete(delete_label))
        .route("/api/tasks/{id}/labels", put(set_labels))
        .route("/api/tasks/{id}/deps/{dep}", put(add_dep).delete(remove_dep))
        .route("/api/teams", get(list_teams).post(create_team))
        .route("/api/teams/{id}/members", get(list_members).post(add_member))
        .route("/api/teams/{id}/members/{uid}", axum::routing::put(set_role).delete(remove_member))
        .route("/api/teams/{id}/members/{uid}/executor", axum::routing::put(set_executor))
        .route("/api/tasks/{id}/help", axum::routing::post(request_help))
        .route("/api/teams/{id}/invites", get(list_invites).post(create_invite))
        .route("/api/invites/{id}", axum::routing::delete(revoke_invite))
        .route("/api/invites/{id}/accept", post(accept_invite))
        .route("/api/teams/{id}/projects", get(list_projects).post(create_project))
        .route("/api/projects/{id}/tasks", get(list_tasks).post(create_task))
        .route("/api/tasks/{id}", get(get_task).patch(update_task).delete(delete_task))
        .route("/api/tasks/{id}/comments", get(list_comments).post(add_comment))
}

// ---------- helpers ----------

pub(crate) async fn role_of(st: &AppState, team: Id, user: Id) -> Result<Role> {
    let r: Option<String> = sqlx::query_scalar("SELECT role FROM team_members WHERE team_id = ? AND user_id = ?")
        .bind(team)
        .bind(user)
        .fetch_optional(&st.db)
        .await?;
    r.and_then(|r| Role::parse(&r)).ok_or_else(AppError::forbidden)
}

/// Exige al menos el rol `min` y devuelve el rol real.
pub(crate) async fn require(st: &AppState, team: Id, user: Id, min: Role) -> Result<Role> {
    let r = role_of(st, team, user).await?;
    if r < min { Err(AppError::forbidden()) } else { Ok(r) }
}

/// Cualquier miembro (incluido lector): lecturas.
pub(crate) async fn ensure_member(st: &AppState, team: Id, user: Id) -> Result<()> {
    require(st, team, user, Role::Viewer).await.map(|_| ())
}

/// Una IA solo modifica o borra tareas que creó ella o que le asignaron; las personas siguen las reglas de rol.
async fn bot_owns(st: &AppState, user: &AuthUser, task: Id) -> Result<()> {
    if !user.bot {
        return Ok(());
    }
    let ok: Option<i64> = sqlx::query_scalar("SELECT 1 FROM tasks WHERE id = ? AND (created_by = ? OR assignee_id = ?)")
        .bind(task)
        .bind(user.id)
        .bind(user.id)
        .fetch_optional(&st.db)
        .await?;
    ok.map(|_| ()).ok_or_else(AppError::forbidden)
}

/// Miembro o superior: escrituras (tareas, comentarios, etiquetas, proyectos).
pub(crate) async fn ensure_writer(st: &AppState, team: Id, user: Id) -> Result<()> {
    require(st, team, user, Role::Member).await.map(|_| ())
}

/// Un rol se puede otorgar solo si es inferior al del que lo otorga (nadie crea otro dueño).
fn can_grant(caller: Role, role: Role) -> bool {
    role != Role::Owner && role < caller
}

pub(crate) async fn team_of_project(st: &AppState, project: Id) -> Result<Id> {
    Ok(sqlx::query_scalar("SELECT team_id FROM projects WHERE id = ?").bind(project).fetch_one(&st.db).await?)
}

pub(crate) async fn team_of_task(st: &AppState, task: Id) -> Result<Id> {
    Ok(sqlx::query_scalar(
        "SELECT p.team_id FROM tasks t JOIN projects p ON p.id = t.project_id WHERE t.id = ?",
    )
    .bind(task)
    .fetch_one(&st.db)
    .await?)
}

pub(crate) const TASK_SELECT: &str = "SELECT t.id, t.project_id, t.title, t.description, t.status, t.priority, t.position, \
    t.assignee_id, u.username AS assignee, t.due, t.start, t.recurrence, t.parent_id, t.created_by, t.created_at, t.updated_at, \
    (SELECT COUNT(*) FROM tasks s WHERE s.parent_id = t.id) AS sub_total, \
    (SELECT COUNT(*) FROM tasks s WHERE s.parent_id = t.id AND s.status = 'done') AS sub_done \
    FROM tasks t LEFT JOIN users u ON u.id = t.assignee_id";

pub(crate) fn task_from_row(r: &SqliteRow) -> Task {
    Task {
        id: r.get("id"),
        project_id: r.get("project_id"),
        title: r.get("title"),
        description: r.get("description"),
        status: Status::parse(r.get("status")).unwrap_or(Status::Todo),
        priority: r.get("priority"),
        position: r.get("position"),
        assignee_id: r.get("assignee_id"),
        assignee: r.get("assignee"),
        created_by: r.get("created_by"),
        due: r.get("due"),
        start: r.get("start"),
        depends_on: vec![],
        parent_id: r.get("parent_id"),
        labels: vec![],
        subtasks_total: r.get("sub_total"),
        subtasks_done: r.get("sub_done"),
        recurrence: r.get("recurrence"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    }
}

fn label_from_row(r: &SqliteRow) -> Label {
    Label { id: r.get("id"), team_id: r.get("team_id"), name: r.get("name"), color: r.get("color") }
}

/// Completa `labels` de las tareas con una sola consulta (`scope` filtra `task_id`).
pub(crate) async fn hydrate_labels(st: &AppState, tasks: &mut [Task], scope: &'static str, id: Id) -> Result<()> {
    let rows = sqlx::query(scope).bind(id).fetch_all(&st.db).await?;
    let mut by_task: std::collections::HashMap<Id, Vec<Label>> = Default::default();
    for r in &rows {
        by_task.entry(r.get("task_id")).or_default().push(label_from_row(r));
    }
    for t in tasks {
        t.labels = by_task.remove(&t.id).unwrap_or_default();
    }
    Ok(())
}

/// Completa `depends_on` con una sola consulta (`scope` filtra `task_id`).
pub(crate) async fn hydrate_deps(st: &AppState, tasks: &mut [Task], scope: &'static str, id: Id) -> Result<()> {
    let rows = sqlx::query(scope).bind(id).fetch_all(&st.db).await?;
    let mut by_task: std::collections::HashMap<Id, Vec<Id>> = Default::default();
    for r in &rows {
        by_task.entry(r.get("task_id")).or_default().push(r.get("depends_on"));
    }
    for t in tasks {
        t.depends_on = by_task.remove(&t.id).unwrap_or_default();
    }
    Ok(())
}

/// Reglas de asignación: un líder (admin o dueño) asigna a cualquiera; un miembro solo puede
/// tomar una tarea libre o soltar la suya.
/// `bot_create`: una IA que crea la tarea puede dejarla asignada a otra persona del equipo.
pub(crate) async fn check_assign(st: &AppState, team: Id, actor: Id, role: Role, current: Option<Id>, new: Option<Id>, bot_create: bool) -> Result<()> {
    if new == current {
        return Ok(());
    }
    if let Some(a) = new {
        let r = role_of(st, team, a).await.map_err(|_| AppError::bad("el asignado no es miembro del equipo"))?;
        if r < Role::Member {
            return Err(AppError::bad("un lector no puede ser responsable de una tarea"));
        }
    }
    if role >= Role::Admin || bot_create {
        return Ok(());
    }
    let takes = new == Some(actor) && current.is_none();
    let releases = new.is_none() && current == Some(actor);
    if takes || releases {
        Ok(())
    } else {
        Err(AppError(
            StatusCode::FORBIDDEN,
            "solo un líder (admin o dueño) asigna tareas a otros; podés tomar las libres o soltar la tuya".into(),
        ))
    }
}

pub(crate) async fn notify_assigned(st: &AppState, team: Id, actor: Id, t: &Task) {
    let Some(to) = t.assignee_id else { return };
    if to == actor {
        return;
    }
    let who = crate::notify::username_of(st, actor).await;
    let prio = ["baja", "media", "alta", "urgente"][t.priority.clamp(0, 3) as usize];
    let due = t.due.map(|d| format!(", vence el {}", d.format("%d/%m/%Y"))).unwrap_or_default();
    crate::notify::notify(st, to, "assigned", Some(team), Some(t.id), format!("{who} te asignó «{}» (prioridad {prio}{due})", t.title)).await;
}

pub(crate) async fn fetch_task(st: &AppState, id: Id) -> Result<Task> {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!("{TASK_SELECT} WHERE t.id = ?"))).bind(id).fetch_one(&st.db).await?;
    let mut t = [task_from_row(&row)];
    hydrate_labels(
        st,
        &mut t,
        "SELECT tl.task_id, l.id, l.team_id, l.name, l.color FROM task_labels tl \
         JOIN labels l ON l.id = tl.label_id WHERE tl.task_id = ? ORDER BY l.name",
        id,
    )
    .await?;
    hydrate_deps(st, &mut t, "SELECT task_id, depends_on FROM task_deps WHERE task_id = ?", id).await?;
    let [t] = t;
    Ok(t)
}

fn project_from_row(r: &SqliteRow) -> Project {
    Project {
        id: r.get("id"),
        team_id: r.get("team_id"),
        name: r.get("name"),
        description: r.get("description"),
        color: r.get("color"),
        work_dir: r.get("work_dir"),
        repo: r.get("repo"),
    }
}

pub(crate) async fn count(st: &AppState, sql: &'static str, id: Id) -> Result<i64> {
    Ok(sqlx::query_scalar(sql).bind(id).fetch_one(&st.db).await?)
}

fn throttle(st: &AppState, headers: &HeaderMap, peer: SocketAddr) -> Result<()> {
    let ip = limits::client_ip(headers, peer, st.trust_proxy);
    st.ip_limiter.hit(&ip, 20, Duration::from_secs(60))
}

// ---------- auth ----------

/// ¿Es la primera vez? (sin ninguna persona registrada): la web ofrece entonces la guía de inicio.
async fn setup_state(State(st): State<AppState>) -> Result<Json<serde_json::Value>> {
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE is_bot = 0").fetch_one(&st.db).await?;
    Ok(Json(serde_json::json!({ "first_run": n == 0, "version": env!("CARGO_PKG_VERSION") })))
}

async fn register(
    State(st): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(c): Json<Credentials>,
) -> Result<Json<AuthResponse>> {
    throttle(&st, &headers, addr)?;
    let name = c.username.trim();
    limits::username(name)?;
    let pn = c.password.chars().count();
    if !(PASSWORD_MIN..=PASSWORD_MAX).contains(&pn) {
        return Err(AppError::bad("contraseña: 8-128 caracteres"));
    }
    let hash = hash_password(&c.password)?;
    let res = sqlx::query("INSERT INTO users (username, password_hash) VALUES (?, ?)")
        .bind(name)
        .bind(hash)
        .execute(&st.db)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(d) if d.is_unique_violation() => {
                AppError(StatusCode::CONFLICT, "el usuario ya existe".into())
            }
            e => e.into(),
        })?;
    let id = res.last_insert_rowid();
    Ok(Json(AuthResponse {
        token: make_token(&st.secret, id, name, 0),
        user: User { id, username: name.into() },
    }))
}

async fn login(
    State(st): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(c): Json<Credentials>,
) -> Result<Json<AuthResponse>> {
    throttle(&st, &headers, addr)?;
    max_len("usuario", &c.username, USERNAME_MAX)?;
    max_len("contraseña", &c.password, PASSWORD_MAX)?;
    // bloqueo por cuenta: 5 fallos => 15 minutos, aunque el atacante cambie de IP
    let key = c.username.trim().to_lowercase();
    let window = Duration::from_secs(15 * 60);
    if st.user_limiter.blocked(&key, 5, window) {
        return Err(limits::too_fast());
    }
    let row = sqlx::query("SELECT id, username, password_hash, token_version FROM users WHERE username = ?")
        .bind(c.username.trim())
        .fetch_optional(&st.db)
        .await?;
    let ok = match &row {
        Some(r) => verify_password(&c.password, &r.get::<String, _>("password_hash")),
        None => { crate::auth::dummy_verify(&c.password); false }
    };
    let Some(row) = row.filter(|_| ok) else {
        let _ = st.user_limiter.hit(&key, 5, window);
        return Err(AppError::unauthorized("credenciales inválidas"));
    };
    st.user_limiter.reset(&key);
    let (id, username, ver): (Id, String, i64) = (row.get("id"), row.get("username"), row.get("token_version"));
    Ok(Json(AuthResponse { token: make_token(&st.secret, id, &username, ver), user: User { id, username } }))
}

/// Invalida todos los tokens emitidos para el usuario (todas sus sesiones).
async fn logout_all(State(st): State<AppState>, user: AuthUser) -> Result<StatusCode> {
    user.human_only()?;
    sqlx::query("UPDATE users SET token_version = token_version + 1 WHERE id = ?").bind(user.id).execute(&st.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------- teams ----------

async fn list_teams(State(st): State<AppState>, user: AuthUser) -> Result<Json<Vec<Team>>> {
    let rows = sqlx::query(
        "SELECT t.id, t.name, t.owner_id FROM teams t \
         JOIN team_members m ON m.team_id = t.id WHERE m.user_id = ? ORDER BY t.name",
    )
    .bind(user.id)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(rows.iter().map(|r| Team { id: r.get("id"), name: r.get("name"), owner_id: r.get("owner_id") }).collect()))
}

async fn create_team(State(st): State<AppState>, user: AuthUser, Json(n): Json<NewTeam>) -> Result<Json<Team>> {
    if n.name.trim().is_empty() {
        return Err(AppError::bad("nombre requerido"));
    }
    max_len("nombre", &n.name, NAME_MAX)?;
    if count(&st, "SELECT COUNT(*) FROM team_members WHERE user_id = ?", user.id).await? >= MAX_TEAMS_PER_USER {
        return Err(too_many("equipos", MAX_TEAMS_PER_USER));
    }
    let mut tx = st.db.begin().await?;
    let id = sqlx::query("INSERT INTO teams (name, owner_id) VALUES (?, ?)")
        .bind(n.name.trim())
        .bind(user.id)
        .execute(&mut *tx)
        .await?
        .last_insert_rowid();
    sqlx::query("INSERT INTO team_members (team_id, user_id, role, joined_at, via) VALUES (?, ?, 'owner', ?, 'owner')").bind(id).bind(user.id).bind(Utc::now()).execute(&mut *tx).await?;
    tx.commit().await?;
    // el dueño ya tiene abierto el WebSocket: este evento hace que se suscriba al equipo nuevo
    let username: String = sqlx::query_scalar("SELECT username FROM users WHERE id = ?").bind(user.id).fetch_one(&st.db).await?;
    st.emit(id, Event::MemberAdded { team_id: id, user: User { id: user.id, username } });
    Ok(Json(Team { id, name: n.name.trim().into(), owner_id: user.id }))
}

async fn list_members(State(st): State<AppState>, user: AuthUser, Path(team): Path<Id>) -> Result<Json<Vec<Member>>> {
    ensure_member(&st, team, user.id).await?;
    let rows = sqlx::query(
        "SELECT u.id, u.username, m.role, m.executor FROM users u JOIN team_members m ON m.user_id = u.id \
         WHERE m.team_id = ? ORDER BY u.username",
    )
    .bind(team)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(
        rows.iter()
            .map(|r| Member {
                id: r.get("id"),
                username: r.get("username"),
                role: Role::parse(r.get("role")).unwrap_or(Role::Viewer),
                executor: r.get::<i64, _>("executor") != 0,
            })
            .collect(),
    ))
}

/// `by`: quién lo sumó; `via`: "direct" (lo agregó un admin) o "invite" (entró con el código `invite_id`).
pub(crate) async fn insert_member(st: &AppState, team: Id, user: &User, role: Role, by: Option<Id>, via: &str, invite_id: Option<Id>) -> Result<bool> {
    if count(st, "SELECT COUNT(*) FROM team_members WHERE team_id = ?", team).await? >= MAX_MEMBERS_PER_TEAM {
        return Err(too_many("miembros", MAX_MEMBERS_PER_TEAM));
    }
    let res = sqlx::query("INSERT OR IGNORE INTO team_members (team_id, user_id, role, joined_at, invited_by, via, invite_id) VALUES (?, ?, ?, ?, ?, ?, ?)")
        .bind(team)
        .bind(user.id)
        .bind(role.as_str())
        .bind(Utc::now())
        .bind(by)
        .bind(via)
        .bind(invite_id)
        .execute(&st.db)
        .await?;
    if res.rows_affected() > 0 {
        st.emit(team, Event::MemberAdded { team_id: team, user: user.clone() });
    }
    Ok(res.rows_affected() > 0)
}

async fn add_member(
    State(st): State<AppState>,
    user: AuthUser,
    Path(team): Path<Id>,
    Json(a): Json<AddMember>,
) -> Result<Json<User>> {
    user.human_only()?;
    let caller = require(&st, team, user.id, Role::Admin).await?;
    let role = a.role.unwrap_or(Role::Member);
    if !can_grant(caller, role) {
        return Err(AppError::forbidden());
    }
    max_len("usuario", &a.username, USERNAME_MAX)?;
    let row = sqlx::query("SELECT id, username FROM users WHERE username = ?").bind(a.username.trim()).fetch_one(&st.db).await?;
    let new = User { id: row.get("id"), username: row.get("username") };
    insert_member(&st, team, &new, role, Some(user.id), "direct", None).await?;
    let team_name: String = sqlx::query_scalar("SELECT name FROM teams WHERE id = ?").bind(team).fetch_one(&st.db).await?;
    let who = crate::notify::username_of(&st, user.id).await;
    crate::notify::notify(&st, new.id, "member_added", Some(team), None, format!("{who} te sumó al equipo «{team_name}» como {}", role.label())).await;
    Ok(Json(new))
}

#[derive(serde::Deserialize)]
struct SetExecutor {
    executor: bool,
}

/// Autoriza (o no) a una persona a ejecutar lo que piden las IAs. Solo admins, y solo personas.
async fn set_executor(
    State(st): State<AppState>,
    user: AuthUser,
    Path((team, target)): Path<(Id, Id)>,
    Json(b): Json<SetExecutor>,
) -> Result<StatusCode> {
    user.human_only()?;
    require(&st, team, user.id, Role::Admin).await?;
    let is_bot: Option<i64> = sqlx::query_scalar("SELECT u.is_bot FROM users u JOIN team_members m ON m.user_id = u.id WHERE m.team_id = ? AND u.id = ?")
        .bind(team)
        .bind(target)
        .fetch_optional(&st.db)
        .await?;
    match is_bot {
        None => return Err(AppError::not_found()),
        Some(0) => {}
        Some(_) => return Err(AppError::bad("una IA no puede ser ejecutora")),
    }
    sqlx::query("UPDATE team_members SET executor = ? WHERE team_id = ? AND user_id = ?")
        .bind(b.executor)
        .bind(team)
        .bind(target)
        .execute(&st.db)
        .await?;
    st.emit(team, Event::MemberRoleChanged { team_id: team, user_id: target, role: role_of(&st, team, target).await? });
    Ok(StatusCode::NO_CONTENT)
}

#[derive(serde::Serialize)]
struct HelpOut {
    comment_id: Id,
    task_id: Id,
    assignee_id: Option<Id>,
}

/// Una IA pide que una persona ejecute algo: comenta la tarea con 🙋 y abre una tarea de pedido para la persona indicada.
/// Responsable: quien creó la tarea si es ejecutor; si no, el ejecutor con menos tareas abiertas; si no hay ejecutores, un admin.
async fn request_help(
    State(st): State<AppState>,
    user: AuthUser,
    Path(task): Path<Id>,
    Json(n): Json<NewComment>,
) -> Result<Json<HelpOut>> {
    let team = team_of_task(&st, task).await?;
    ensure_writer(&st, team, user.id).await?;
    let q = n.body.trim();
    if q.is_empty() {
        return Err(AppError::bad("pedido vacío"));
    }
    max_len("pedido", q, COMMENT_MAX)?;
    let t = fetch_task(&st, task).await?;
    let pick = |extra: &str| format!(
        "SELECT u.id FROM users u JOIN team_members m ON m.user_id = u.id \
         LEFT JOIN tasks o ON o.assignee_id = u.id AND o.status != 'done' \
         WHERE m.team_id = ?1 AND u.is_bot = 0 {extra} GROUP BY u.id ORDER BY COUNT(o.id), u.id LIMIT 1"
    );
    let creator_ok: Option<Id> = match t.created_by {
        Some(c) => sqlx::query_scalar(sqlx::AssertSqlSafe(pick("AND m.executor = 1 AND u.id = ?2"))).bind(team).bind(c).fetch_optional(&st.db).await?,
        None => None,
    };
    let assignee = match creator_ok {
        Some(id) => Some(id),
        None => match sqlx::query_scalar(sqlx::AssertSqlSafe(pick("AND m.executor = 1"))).bind(team).fetch_optional(&st.db).await? {
            Some(id) => Some(id),
            None => sqlx::query_scalar(sqlx::AssertSqlSafe(pick("AND m.role IN ('owner','admin')"))).bind(team).fetch_optional(&st.db).await?,
        },
    };
    let title: String = format!("🙋 Pedido de #{task}: {}", q.lines().next().unwrap_or("").chars().take(80).collect::<String>());
    let ai = member_name(&st, user.id).await;
    let work_dir: String = sqlx::query_scalar("SELECT work_dir FROM projects WHERE id = ?").bind(t.project_id).fetch_one(&st.db).await.unwrap_or_default();
    let cmds = backtick_cmds(q);
    let cmd_block = if cmds.is_empty() {
        String::new()
    } else {
        let dir = if work_dir.is_empty() { String::new() } else { format!(" (carpeta del proyecto: {work_dir})") };
        format!("COMANDOS PARA EJECUTAR{dir}\nCopiá cada línea y pegala en tu terminal:\n\n{}\n\n", cmds.join("\n"))
    };
    let description = format!(
        "La IA «{ai}» pidió ayuda en la tarea #{task} «{}».\n\nLO QUE PIDE\n{q}\n\n{cmd_block}CÓMO RESPONDER\n\
         1. Ejecutá lo que pide (los comandos de arriba) en tu terminal.\n\
         2. Pegá la salida en un comentario de ESTA tarea (abajo, en «Comentarios»). Le llega sola a la IA, que sigue.\n\
         3. Si solo hay que decir sí o no, usá los botones ✅ Autorizar / ❌ Rechazar del panel.\n\
         4. Cuando termines, marcá esta tarea como Hecha.",
        t.title
    );
    let Json(req) = create_task(
        State(st.clone()),
        AuthUser { id: user.id, bot: user.bot },
        Path(t.project_id),
        Json(NewTask { title, description, priority: 3, assignee_id: assignee, ..Default::default() }),
    )
    .await?;
    let para = match assignee {
        Some(id) => format!("para {}", member_name(&st, id).await),
        None => "sin asignar".into(),
    };
    let body = format!("🙋 {q}\n\n📌 Se creó la tarea #{} ({para}) con los pasos. La salida o la respuesta se pega como comentario en esa tarea (o acá): la IA la recibe y sigue.", req.id);
    let Json(c) = add_comment(State(st.clone()), AuthUser { id: user.id, bot: user.bot }, Path(task), Json(NewComment { body })).await?;
    Ok(Json(HelpOut { comment_id: c.id, task_id: req.id, assignee_id: assignee }))
}

/// Comandos entre `backticks` de un pedido, uno por línea en la tarea de pedido.
fn backtick_cmds(q: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut push = |c: &str| { let c = c.trim().trim_start_matches("$ ").trim(); if c.len() >= 2 && !out.iter().any(|x| x == c) { out.push(c.to_string()); } };
    // bloques con tres comillas invertidas: un comando por línea (se ignora un posible nombre de lenguaje en la primera línea)
    let mut inline = String::new();
    for (i, part) in q.split("```").enumerate() {
        if i % 2 == 1 {
            let mut lines = part.lines().collect::<Vec<_>>();
            if lines.len() > 1 && lines[0].trim().chars().all(|c| c.is_ascii_alphanumeric()) && !lines[0].trim().is_empty() { lines.remove(0); }
            for l in lines { push(l); }
        } else {
            inline.push_str(part);
        }
    }
    for (i, part) in inline.split('`').enumerate() {
        if i % 2 == 1 && !part.contains('\n') { push(part); }
    }
    out.truncate(8);
    out
}

async fn member_name(st: &AppState, id: Id) -> String {
    sqlx::query_scalar("SELECT username FROM users WHERE id = ?").bind(id).fetch_one(&st.db).await.unwrap_or_default()
}

async fn set_role(
    State(st): State<AppState>,
    user: AuthUser,
    Path((team, target)): Path<(Id, Id)>,
    Json(b): Json<SetRole>,
) -> Result<StatusCode> {
    user.human_only()?;
    let caller = require(&st, team, user.id, Role::Admin).await?;
    let current = role_of(&st, team, target).await.map_err(|_| AppError::not_found())?;
    // hay que superar el rol actual del objetivo y poder otorgar el nuevo
    if current >= caller || !can_grant(caller, b.role) {
        return Err(AppError::forbidden());
    }
    sqlx::query("UPDATE team_members SET role = ? WHERE team_id = ? AND user_id = ?")
        .bind(b.role.as_str())
        .bind(team)
        .bind(target)
        .execute(&st.db)
        .await?;
    st.emit(team, Event::MemberRoleChanged { team_id: team, user_id: target, role: b.role });
    let team_name: String = sqlx::query_scalar("SELECT name FROM teams WHERE id = ?").bind(team).fetch_one(&st.db).await?;
    crate::notify::notify(&st, target, "role_changed", Some(team), None, format!("Tu rol en «{team_name}» ahora es {}", b.role.label())).await;
    Ok(StatusCode::NO_CONTENT)
}

/// Quita a un miembro (admin+ sobre rangos inferiores) o permite salir del equipo; el dueño no puede irse.
async fn remove_member(
    State(st): State<AppState>,
    user: AuthUser,
    Path((team, target)): Path<(Id, Id)>,
) -> Result<StatusCode> {
    user.human_only()?;
    let caller = role_of(&st, team, user.id).await?;
    let current = role_of(&st, team, target).await.map_err(|_| AppError::not_found())?;
    if current == Role::Owner {
        return Err(AppError::forbidden());
    }
    if target != user.id && (caller < Role::Admin || current >= caller) {
        return Err(AppError::forbidden());
    }
    sqlx::query("DELETE FROM team_members WHERE team_id = ? AND user_id = ?").bind(team).bind(target).execute(&st.db).await?;
    // las tareas asignadas al que sale quedan sin asignar
    sqlx::query(
        "UPDATE tasks SET assignee_id = NULL WHERE assignee_id = ? AND project_id IN (SELECT id FROM projects WHERE team_id = ?)",
    )
    .bind(target)
    .bind(team)
    .execute(&st.db)
    .await?;
    st.emit(team, Event::MemberRemoved { team_id: team, user_id: target });
    Ok(StatusCode::NO_CONTENT)
}

// ---------- invitaciones ----------

fn invite_from_row(r: &SqliteRow) -> Invite {
    Invite {
        id: r.get("id"),
        team_id: r.get("team_id"),
        role: Role::parse(r.get("role")).unwrap_or(Role::Viewer),
        max_uses: r.get("max_uses"),
        uses: r.get("uses"),
        expires_at: r.get("expires_at"),
    }
}

async fn create_invite(
    State(st): State<AppState>,
    user: AuthUser,
    Path(team): Path<Id>,
    Json(n): Json<NewInvite>,
) -> Result<Json<InviteCreated>> {
    user.human_only()?;
    let caller = require(&st, team, user.id, Role::Admin).await?;
    if !can_grant(caller, n.role) {
        return Err(AppError::forbidden());
    }
    let max_uses = n.max_uses.unwrap_or(10).clamp(1, 100);
    let hours = n.expires_hours.unwrap_or(168).clamp(1, 720);
    if count(&st, "SELECT COUNT(*) FROM invites WHERE team_id = ? AND expires_at > datetime('now') AND uses < max_uses", team).await?
        >= MAX_INVITES_PER_TEAM
    {
        return Err(too_many("invitaciones activas", MAX_INVITES_PER_TEAM));
    }
    let code = crate::auth::random_hex(16);
    let expires = Utc::now() + chrono::Duration::hours(hours);
    let id = sqlx::query("INSERT INTO invites (team_id, code_hash, role, max_uses, expires_at, created_by) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(team)
        .bind(crate::auth::sha256_hex(&code))
        .bind(n.role.as_str())
        .bind(max_uses)
        .bind(expires)
        .bind(user.id)
        .execute(&st.db)
        .await?
        .last_insert_rowid();
    Ok(Json(InviteCreated { code, invite: Invite { id, team_id: team, role: n.role, max_uses, uses: 0, expires_at: expires } }))
}

async fn list_invites(State(st): State<AppState>, user: AuthUser, Path(team): Path<Id>) -> Result<Json<Vec<Invite>>> {
    require(&st, team, user.id, Role::Admin).await?;
    let rows = sqlx::query("SELECT id, team_id, role, max_uses, uses, expires_at FROM invites WHERE team_id = ? ORDER BY id DESC LIMIT 100")
        .bind(team)
        .fetch_all(&st.db)
        .await?;
    Ok(Json(rows.iter().map(invite_from_row).collect()))
}

async fn revoke_invite(State(st): State<AppState>, user: AuthUser, Path(id): Path<Id>) -> Result<StatusCode> {
    user.human_only()?;
    let team: Id = sqlx::query_scalar("SELECT team_id FROM invites WHERE id = ?").bind(id).fetch_one(&st.db).await?;
    require(&st, team, user.id, Role::Admin).await?;
    sqlx::query("DELETE FROM invites WHERE id = ?").bind(id).execute(&st.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn accept_invite(
    State(st): State<AppState>,
    user: AuthUser,
    Path(code): Path<String>,
) -> Result<Json<Team>> {
    // límite por usuario para que no se pueda adivinar códigos por fuerza bruta
    st.user_limiter.hit(&format!("invite:{}", user.id), 10, Duration::from_secs(15 * 60))?;
    max_len("código", &code, 64)?;
    let hash = crate::auth::sha256_hex(&code);
    let mut tx = st.db.begin().await?;
    // el UPDATE condicional consume un uso de forma atómica (sin carreras entre dos aceptaciones)
    let used = sqlx::query(
        "UPDATE invites SET uses = uses + 1 WHERE code_hash = ? AND uses < max_uses AND expires_at > ? \
         RETURNING team_id, role, id, created_by",
    )
    .bind(&hash)
    .bind(Utc::now())
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::bad("invitación inválida, vencida o agotada"))?;
    let (team, role): (Id, Role) = (used.get("team_id"), Role::parse(used.get("role")).unwrap_or(Role::Viewer));
    let (invite_id, invited_by): (Id, Option<Id>) = (used.get("id"), used.get("created_by"));
    let already: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM team_members WHERE team_id = ? AND user_id = ?")
        .bind(team)
        .bind(user.id)
        .fetch_one(&mut *tx)
        .await?;
    if already > 0 {
        // ya es miembro: no se gasta el uso
        return Err(AppError(StatusCode::CONFLICT, "ya sos miembro de ese equipo".into()));
    }
    tx.commit().await?;
    let me = User {
        id: user.id,
        username: sqlx::query_scalar("SELECT username FROM users WHERE id = ?").bind(user.id).fetch_one(&st.db).await?,
    };
    insert_member(&st, team, &me, role, invited_by, "invite", Some(invite_id)).await?;
    let row = sqlx::query("SELECT id, name, owner_id FROM teams WHERE id = ?").bind(team).fetch_one(&st.db).await?;
    Ok(Json(Team { id: row.get("id"), name: row.get("name"), owner_id: row.get("owner_id") }))
}

// ---------- projects ----------

async fn list_projects(State(st): State<AppState>, user: AuthUser, Path(team): Path<Id>) -> Result<Json<Vec<Project>>> {
    ensure_member(&st, team, user.id).await?;
    let rows = sqlx::query("SELECT id, team_id, name, description, color, work_dir, repo FROM projects WHERE team_id = ? ORDER BY name LIMIT 200")
        .bind(team)
        .fetch_all(&st.db)
        .await?;
    Ok(Json(rows.iter().map(project_from_row).collect()))
}

pub(crate) async fn create_project(
    State(st): State<AppState>,
    user: AuthUser,
    Path(team): Path<Id>,
    Json(n): Json<NewProject>,
) -> Result<Json<Project>> {
    ensure_writer(&st, team, user.id).await?;
    if n.name.trim().is_empty() {
        return Err(AppError::bad("nombre requerido"));
    }
    max_len("nombre", &n.name, NAME_MAX)?;
    max_len("descripción", &n.description, DESCRIPTION_MAX)?;
    let color = n.color.unwrap_or_else(|| "#7aa2f7".into());
    limits::color(&color)?;
    max_len("carpeta", &n.work_dir, 300)?;
    max_len("repositorio", &n.repo, 300)?;
    if count(&st, "SELECT COUNT(*) FROM projects WHERE team_id = ?", team).await? >= MAX_PROJECTS_PER_TEAM {
        return Err(too_many("proyectos", MAX_PROJECTS_PER_TEAM));
    }
    let id = sqlx::query("INSERT INTO projects (team_id, name, description, color, work_dir, repo) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(team)
        .bind(n.name.trim())
        .bind(&n.description)
        .bind(&color)
        .bind(n.work_dir.trim())
        .bind(n.repo.trim())
        .execute(&st.db)
        .await?
        .last_insert_rowid();
    let p = Project { id, team_id: team, name: n.name.trim().into(), description: n.description, color, work_dir: n.work_dir.trim().into(), repo: n.repo.trim().into() };
    st.emit(team, Event::ProjectCreated(p.clone()));
    Ok(Json(p))
}

/// Los contadores de subtareas viven en el padre: se reemite cuando cambia un hijo.
async fn emit_parent(st: &AppState, team: Id, parent: Option<Id>) {
    if let Some(p) = parent {
        if let Ok(t) = fetch_task(st, p).await {
            st.emit(team, Event::TaskUpdated(t));
        }
    }
}

// ---------- tasks ----------

pub(crate) async fn list_tasks(State(st): State<AppState>, user: AuthUser, Path(project): Path<Id>) -> Result<Json<Vec<Task>>> {
    ensure_member(&st, team_of_project(&st, project).await?, user.id).await?;
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!("{TASK_SELECT} WHERE t.project_id = ? ORDER BY t.position, t.id LIMIT 5000")))
        .bind(project)
        .fetch_all(&st.db)
        .await?;
    let mut tasks: Vec<Task> = rows.iter().map(task_from_row).collect();
    hydrate_labels(
        &st,
        &mut tasks,
        "SELECT tl.task_id, l.id, l.team_id, l.name, l.color FROM task_labels tl \
         JOIN labels l ON l.id = tl.label_id JOIN tasks t ON t.id = tl.task_id \
         WHERE t.project_id = ? ORDER BY l.name",
        project,
    )
    .await?;
    hydrate_deps(
        &st,
        &mut tasks,
        "SELECT d.task_id, d.depends_on FROM task_deps d JOIN tasks t ON t.id = d.task_id WHERE t.project_id = ?",
        project,
    )
    .await?;
    Ok(Json(tasks))
}

pub(crate) async fn create_task(
    State(st): State<AppState>,
    user: AuthUser,
    Path(project): Path<Id>,
    Json(n): Json<NewTask>,
) -> Result<Json<Task>> {
    let team = team_of_project(&st, project).await?;
    let caller = require(&st, team, user.id, Role::Member).await?;
    if n.title.trim().is_empty() {
        return Err(AppError::bad("título requerido"));
    }
    max_len("título", &n.title, TITLE_MAX)?;
    max_len("descripción", &n.description, DESCRIPTION_MAX)?;
    if count(&st, "SELECT COUNT(*) FROM tasks WHERE project_id = ?", project).await? >= MAX_TASKS_PER_PROJECT {
        return Err(too_many("tareas", MAX_TASKS_PER_PROJECT));
    }
    check_assign(&st, team, user.id, caller, None, n.assignee_id, user.bot).await?;
    let recurrence = crate::extras::valid_recurrence(n.recurrence.as_deref())?;
    if let Some(parent) = n.parent_id {
        let row = sqlx::query("SELECT project_id, parent_id FROM tasks WHERE id = ?")
            .bind(parent)
            .fetch_optional(&st.db)
            .await?
            .ok_or_else(|| AppError::bad("tarea padre inexistente"))?;
        if row.get::<Id, _>("project_id") != project {
            return Err(AppError::bad("la tarea padre es de otro proyecto"));
        }
        if row.get::<Option<Id>, _>("parent_id").is_some() {
            return Err(AppError::bad("las subtareas no pueden tener subtareas"));
        }
        if count(&st, "SELECT COUNT(*) FROM tasks WHERE parent_id = ?", parent).await? >= MAX_SUBTASKS_PER_TASK {
            return Err(too_many("subtareas", MAX_SUBTASKS_PER_TASK));
        }
    }
    let now = Utc::now();
    let id = sqlx::query(
        "INSERT INTO tasks (project_id, title, description, priority, assignee_id, due, parent_id, start, recurrence, created_at, updated_at, created_by, position) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, (SELECT COALESCE(MAX(position), 0) + 1 FROM tasks WHERE project_id = ?))",
    )
    .bind(project)
    .bind(n.title.trim())
    .bind(&n.description)
    .bind(n.priority.clamp(0, 3))
    .bind(n.assignee_id)
    .bind(n.due)
    .bind(n.parent_id)
    .bind(n.start)
    .bind(recurrence)
    .bind(now)
    .bind(now)
    .bind(user.id)
    .bind(project)
    .execute(&st.db)
    .await?
    .last_insert_rowid();
    let t = fetch_task(&st, id).await?;
    if t.parent_id.is_none() {
        crate::extras::log(&st, project, user.id, format!("creó la tarea «{}»", t.title)).await;
    }
    st.emit(team, Event::TaskCreated(t.clone()));
    emit_parent(&st, team, t.parent_id).await;
    notify_assigned(&st, team, user.id, &t).await;
    Ok(Json(t))
}

async fn update_task(
    State(st): State<AppState>,
    user: AuthUser,
    Path(id): Path<Id>,
    Json(p): Json<TaskPatch>,
) -> Result<Json<Task>> {
    let team = team_of_task(&st, id).await?;
    let caller = require(&st, team, user.id, Role::Member).await?;
    bot_owns(&st, &user, id).await?;
    if let Some(t) = &p.title { max_len("título", t, TITLE_MAX)?; }
    if let Some(d) = &p.description { max_len("descripción", d, DESCRIPTION_MAX)?; }
    let cur = fetch_task(&st, id).await?;
    if let Some(new) = p.assignee_id {
        check_assign(&st, team, user.id, caller, cur.assignee_id, new, false).await?;
    }
    let assignee = p.assignee_id.unwrap_or(cur.assignee_id);
    let due = p.due.unwrap_or(cur.due);
    let start = p.start.unwrap_or(cur.start);
    let recurrence = match &p.recurrence {
        Some(v) => crate::extras::valid_recurrence(v.as_deref())?,
        None => cur.recurrence.clone(),
    };
    // si cambia el vencimiento o el responsable, los recordatorios vuelven a empezar
    let reminders_reset = due != cur.due || assignee != cur.assignee_id;
    // proyecto con revisión humana: lo que una IA da por terminado queda en revisión en vez de pasar a Hecha
    let mut new_status = p.status.unwrap_or(cur.status);
    let held = crate::review::holds(&st, &user, &cur, new_status).await;
    if held {
        new_status = if cur.status == Status::Todo { Status::Doing } else { cur.status };
    }
    sqlx::query(
        "UPDATE tasks SET title = ?, description = ?, status = ?, priority = ?, position = ?, assignee_id = ?, due = ?, start = ?, recurrence = ?, \
         remind_state = CASE WHEN ? THEN 0 ELSE remind_state END, updated_at = ? WHERE id = ?",
    )
    .bind(p.title.as_deref().map(str::trim).filter(|s| !s.is_empty()).unwrap_or(&cur.title))
    .bind(p.description.as_deref().unwrap_or(&cur.description))
    .bind(new_status.as_str())
    .bind(p.priority.map(|v| v.clamp(0, 3)).unwrap_or(cur.priority))
    .bind(p.position.filter(|v| v.is_finite()).unwrap_or(cur.position))
    .bind(assignee)
    .bind(due)
    .bind(start)
    .bind(recurrence)
    .bind(reminders_reset)
    .bind(Utc::now())
    .bind(id)
    .execute(&st.db)
    .await?;
    let t = fetch_task(&st, id).await?;
    st.emit(team, Event::TaskUpdated(t.clone()));
    emit_parent(&st, team, t.parent_id).await;
    if t.assignee_id != cur.assignee_id {
        notify_assigned(&st, team, user.id, &t).await;
    }
    if held {
        crate::review::request(&st, team, &t, user.id).await;
    }
    crate::extras::after_update(&st, team, user.id, &cur, &t).await;
    Ok(Json(t))
}

async fn delete_task(State(st): State<AppState>, user: AuthUser, Path(id): Path<Id>) -> Result<StatusCode> {
    let team = team_of_task(&st, id).await?;
    ensure_writer(&st, team, user.id).await?;
    bot_owns(&st, &user, id).await?;
    let row = sqlx::query("SELECT project_id, parent_id FROM tasks WHERE id = ?").bind(id).fetch_one(&st.db).await?;
    let (project, parent): (Id, Option<Id>) = (row.get("project_id"), row.get("parent_id"));
    let title: String = sqlx::query_scalar("SELECT title FROM tasks WHERE id = ?").bind(id).fetch_one(&st.db).await.unwrap_or_default();
    sqlx::query("DELETE FROM tasks WHERE id = ?").bind(id).execute(&st.db).await?;
    if parent.is_none() {
        crate::extras::log(&st, project, user.id, format!("borró la tarea «{title}»")).await;
    }
    st.emit(team, Event::TaskDeleted { id, project_id: project });
    emit_parent(&st, team, parent).await;
    Ok(StatusCode::NO_CONTENT)
}

/// Una tarea por id (para llevar a la persona a la tarea de una novedad).
async fn get_task(State(st): State<AppState>, user: AuthUser, Path(id): Path<Id>) -> Result<Json<Task>> {
    let team = team_of_task(&st, id).await?;
    ensure_member(&st, team, user.id).await?;
    Ok(Json(fetch_task(&st, id).await?))
}

// ---------- comments ----------

pub(crate) fn comment_from_row(r: &SqliteRow) -> Comment {
    Comment {
        id: r.get("id"),
        task_id: r.get("task_id"),
        user_id: r.get("user_id"),
        username: r.get("username"),
        body: r.get("body"),
        created_at: r.get("created_at"),
    }
}

pub(crate) const COMMENT_SELECT: &str = "SELECT c.id, c.task_id, c.user_id, u.username, c.body, c.created_at \
    FROM comments c JOIN users u ON u.id = c.user_id";

async fn list_comments(State(st): State<AppState>, user: AuthUser, Path(task): Path<Id>) -> Result<Json<Vec<Comment>>> {
    ensure_member(&st, team_of_task(&st, task).await?, user.id).await?;
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!("{COMMENT_SELECT} WHERE c.task_id = ? ORDER BY c.id LIMIT 2000")))
        .bind(task)
        .fetch_all(&st.db)
        .await?;
    Ok(Json(rows.iter().map(comment_from_row).collect()))
}

async fn add_comment(
    State(st): State<AppState>,
    user: AuthUser,
    Path(task): Path<Id>,
    Json(mut n): Json<NewComment>,
) -> Result<Json<Comment>> {
    let team = team_of_task(&st, task).await?;
    ensure_writer(&st, team, user.id).await?;
    if n.body.trim().is_empty() {
        return Err(AppError::bad("comentario vacío"));
    }
    max_len("comentario", &n.body, COMMENT_MAX)?;
    // «/permitir [siempre] comando»: una persona con rol admin u owner habilita un comando para la IA de la tarea
    if let Some(rest) = n.body.trim().strip_prefix("/permitir") {
        if rest.is_empty() || rest.starts_with(char::is_whitespace) {
            let rest = rest.trim();
            let (always, cmd) = match rest.strip_prefix("siempre") {
                Some(c) if c.starts_with(char::is_whitespace) => (true, c.trim()),
                _ => (false, rest),
            };
            crate::bots::grant(&st, &user, task, cmd, always).await?;
            n.body = format!("✅ Autorizado: `{cmd}` ({}). La IA lo puede usar {}.", if always { "siempre" } else { "una vez" }, if always { "desde su próxima ejecución" } else { "en su próxima ejecución de esta tarea" });
        }
    }
    if count(&st, "SELECT COUNT(*) FROM comments WHERE task_id = ?", task).await? >= MAX_COMMENTS_PER_TASK {
        return Err(too_many("comentarios", MAX_COMMENTS_PER_TASK));
    }
    let id = sqlx::query("INSERT INTO comments (task_id, user_id, body, created_at) VALUES (?, ?, ?, ?)")
        .bind(task)
        .bind(user.id)
        .bind(n.body.trim())
        .bind(Utc::now())
        .execute(&st.db)
        .await?
        .last_insert_rowid();
    let row = sqlx::query(sqlx::AssertSqlSafe(format!("{COMMENT_SELECT} WHERE c.id = ?"))).bind(id).fetch_one(&st.db).await?;
    let c = comment_from_row(&row);
    st.emit(team, Event::CommentAdded(c.clone()));
    // un comentario de una persona en una tarea ya terminada de una IA la reabre (Por hacer) para que la retome leyéndolo;
    // las tareas programadas funcionan igual. Los comandos «/…» no reabren.
    if !user.bot && !n.body.trim_start().starts_with('/') {
        let reopen: Option<i64> = sqlx::query_scalar(
            "SELECT 1 FROM tasks WHERE id = ? AND status = 'done' AND assignee_id IN (SELECT user_id FROM api_tokens)",
        )
        .bind(task)
        .fetch_optional(&st.db)
        .await?;
        if reopen.is_some() {
            sqlx::query("UPDATE tasks SET status = 'todo', updated_at = ? WHERE id = ?").bind(Utc::now()).bind(task).execute(&st.db).await?;
            let t = fetch_task(&st, task).await?;
            st.emit(team, Event::TaskUpdated(t.clone()));
            crate::extras::log_task(&st, task, user.id, format!("reabrió «{}» con un comentario", t.title)).await;
        }
    }
    // una respuesta en la tarea de pedido («🙋 Pedido de #N: …») se copia a la tarea N, que es la que la IA está leyendo
    if !user.bot {
        let title: Option<String> = sqlx::query_scalar("SELECT title FROM tasks WHERE id = ?").bind(task).fetch_optional(&st.db).await?;
        let origin = title.as_deref().and_then(|t| t.strip_prefix("🙋 Pedido de #")).and_then(|t| t.split(':').next()).and_then(|n| n.parse::<Id>().ok());
        if let Some(origin) = origin.filter(|o| *o != task) {
            let same_team: Option<Id> = sqlx::query_scalar("SELECT t.id FROM tasks t JOIN projects p ON p.id = t.project_id WHERE t.id = ? AND p.team_id = ?").bind(origin).bind(team).fetch_optional(&st.db).await?;
            if same_team.is_some() {
                let mid = sqlx::query("INSERT INTO comments (task_id, user_id, body, created_at) VALUES (?, ?, ?, ?)")
                    .bind(origin)
                    .bind(user.id)
                    .bind(format!("↪ Respuesta en el pedido #{task}:\n{}", n.body.trim()))
                    .bind(Utc::now())
                    .execute(&st.db)
                    .await?
                    .last_insert_rowid();
                let mrow = sqlx::query(sqlx::AssertSqlSafe(format!("{COMMENT_SELECT} WHERE c.id = ?"))).bind(mid).fetch_one(&st.db).await?;
                st.emit(team, Event::CommentAdded(comment_from_row(&mrow)));
            }
        }
    }
    // avisa al responsable y a quienes ya comentaron en la tarea (menos a quien escribe)
    let people: Vec<Id> = sqlx::query_scalar(
        "SELECT assignee_id FROM tasks WHERE id = ?1 AND assignee_id IS NOT NULL \
         UNION SELECT user_id FROM comments WHERE task_id = ?1 LIMIT 20",
    )
    .bind(task)
    .fetch_all(&st.db)
    .await
    .unwrap_or_default();
    let title: String = sqlx::query_scalar("SELECT title FROM tasks WHERE id = ?").bind(task).fetch_one(&st.db).await.unwrap_or_default();
    let preview: String = c.body.chars().take(200).collect();
    crate::extras::log_task(&st, task, user.id, format!("comentó en «{title}»")).await;
    let mentioned = crate::extras::notify_mentions(&st, team, user.id, &c.username, task, &title, &c.body).await;
    // «🙋 …» es la alarma de asistencia (una IA pide autorización, una respuesta o una acción): avisa también a owners y admins
    let alarm = c.body.starts_with('🙋');
    let mut targets = people;
    if alarm {
        let admins: Vec<Id> = sqlx::query_scalar("SELECT user_id FROM team_members WHERE team_id = ? AND role IN ('owner', 'admin') LIMIT 20")
            .bind(team)
            .fetch_all(&st.db)
            .await
            .unwrap_or_default();
        targets.extend(admins);
        targets.sort_unstable();
        targets.dedup();
    }
    for to in targets.into_iter().filter(|u| *u != user.id && !mentioned.contains(u)) {
        let (kind, text) = if alarm {
            ("attention", format!("🙋 {} necesita tu ayuda en «{title}»: {}", c.username, preview.trim_start_matches('🙋').trim()))
        } else {
            ("comment", format!("{} comentó en «{title}»: {preview}", c.username))
        };
        crate::notify::notify(&st, to, kind, Some(team), Some(task), text).await;
    }
    Ok(Json(c))
}

// ---------- labels ----------

async fn list_labels(State(st): State<AppState>, user: AuthUser, Path(team): Path<Id>) -> Result<Json<Vec<Label>>> {
    ensure_member(&st, team, user.id).await?;
    let rows = sqlx::query("SELECT id, team_id, name, color FROM labels WHERE team_id = ? ORDER BY name LIMIT 100")
        .bind(team)
        .fetch_all(&st.db)
        .await?;
    Ok(Json(rows.iter().map(label_from_row).collect()))
}

async fn create_label(
    State(st): State<AppState>,
    user: AuthUser,
    Path(team): Path<Id>,
    Json(n): Json<NewLabel>,
) -> Result<Json<Label>> {
    ensure_writer(&st, team, user.id).await?;
    let name = n.name.trim();
    if name.is_empty() {
        return Err(AppError::bad("nombre requerido"));
    }
    max_len("etiqueta", name, LABEL_MAX)?;
    let color = n.color.unwrap_or_else(|| "#bb9af7".into());
    limits::color(&color)?;
    if count(&st, "SELECT COUNT(*) FROM labels WHERE team_id = ?", team).await? >= MAX_LABELS_PER_TEAM {
        return Err(too_many("etiquetas", MAX_LABELS_PER_TEAM));
    }
    let id = sqlx::query("INSERT INTO labels (team_id, name, color) VALUES (?, ?, ?)")
        .bind(team)
        .bind(name)
        .bind(&color)
        .execute(&st.db)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(d) if d.is_unique_violation() => {
                AppError(StatusCode::CONFLICT, "ya existe una etiqueta con ese nombre".into())
            }
            e => e.into(),
        })?
        .last_insert_rowid();
    let l = Label { id, team_id: team, name: name.into(), color };
    st.emit(team, Event::LabelCreated(l.clone()));
    Ok(Json(l))
}

async fn delete_label(State(st): State<AppState>, user: AuthUser, Path(id): Path<Id>) -> Result<StatusCode> {
    let team: Id = sqlx::query_scalar("SELECT team_id FROM labels WHERE id = ?").bind(id).fetch_one(&st.db).await?;
    ensure_writer(&st, team, user.id).await?;
    sqlx::query("DELETE FROM labels WHERE id = ?").bind(id).execute(&st.db).await?;
    st.emit(team, Event::LabelDeleted { id, team_id: team });
    Ok(StatusCode::NO_CONTENT)
}

/// Reemplaza el conjunto de etiquetas de una tarea.
async fn set_labels(
    State(st): State<AppState>,
    user: AuthUser,
    Path(task): Path<Id>,
    Json(b): Json<SetLabels>,
) -> Result<Json<Task>> {
    let team = team_of_task(&st, task).await?;
    ensure_writer(&st, team, user.id).await?;
    let mut ids = b.label_ids;
    ids.sort_unstable();
    ids.dedup();
    if ids.len() > MAX_LABELS_PER_TASK {
        return Err(too_many("etiquetas por tarea", MAX_LABELS_PER_TASK as i64));
    }
    let mut tx = st.db.begin().await?;
    for l in &ids {
        let ok: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM labels WHERE id = ? AND team_id = ?")
            .bind(l)
            .bind(team)
            .fetch_one(&mut *tx)
            .await?;
        if ok == 0 {
            return Err(AppError::bad("etiqueta inexistente o de otro equipo"));
        }
    }
    sqlx::query("DELETE FROM task_labels WHERE task_id = ?").bind(task).execute(&mut *tx).await?;
    for l in &ids {
        sqlx::query("INSERT INTO task_labels (task_id, label_id) VALUES (?, ?)").bind(task).bind(l).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    let t = fetch_task(&st, task).await?;
    st.emit(team, Event::TaskUpdated(t.clone()));
    Ok(Json(t))
}

// ---------- dependencias (Gantt) ----------

/// ¿Se llega a `target` siguiendo las dependencias desde `from`? (para evitar ciclos)
async fn reaches(st: &AppState, from: Id, target: Id) -> Result<bool> {
    let mut seen = std::collections::HashSet::new();
    let mut stack = vec![from];
    while let Some(n) = stack.pop() {
        if n == target {
            return Ok(true);
        }
        if !seen.insert(n) || seen.len() > 5_000 {
            continue;
        }
        let next: Vec<Id> = sqlx::query_scalar("SELECT depends_on FROM task_deps WHERE task_id = ?").bind(n).fetch_all(&st.db).await?;
        stack.extend(next);
    }
    Ok(false)
}

async fn add_dep(State(st): State<AppState>, user: AuthUser, Path((id, dep)): Path<(Id, Id)>) -> Result<Json<Task>> {
    let team = team_of_task(&st, id).await?;
    ensure_writer(&st, team, user.id).await?;
    if id == dep {
        return Err(AppError::bad("una tarea no puede depender de sí misma"));
    }
    let projects: Vec<Id> = sqlx::query_scalar("SELECT DISTINCT project_id FROM tasks WHERE id IN (?, ?)").bind(id).bind(dep).fetch_all(&st.db).await?;
    if projects.len() != 1 {
        return Err(AppError::bad("las dos tareas deben ser del mismo proyecto"));
    }
    if count(&st, "SELECT COUNT(*) FROM task_deps WHERE task_id = ?", id).await? >= MAX_DEPS_PER_TASK {
        return Err(too_many("dependencias", MAX_DEPS_PER_TASK));
    }
    if reaches(&st, dep, id).await? {
        return Err(AppError::bad("la dependencia crearía un ciclo"));
    }
    sqlx::query("INSERT OR IGNORE INTO task_deps (task_id, depends_on) VALUES (?, ?)").bind(id).bind(dep).execute(&st.db).await?;
    let t = fetch_task(&st, id).await?;
    st.emit(team, Event::TaskUpdated(t.clone()));
    Ok(Json(t))
}

async fn remove_dep(State(st): State<AppState>, user: AuthUser, Path((id, dep)): Path<(Id, Id)>) -> Result<Json<Task>> {
    let team = team_of_task(&st, id).await?;
    ensure_writer(&st, team, user.id).await?;
    sqlx::query("DELETE FROM task_deps WHERE task_id = ? AND depends_on = ?").bind(id).bind(dep).execute(&st.db).await?;
    let t = fetch_task(&st, id).await?;
    st.emit(team, Event::TaskUpdated(t.clone()));
    Ok(Json(t))
}

/// Comentario automático (a nombre de quien lo causó) que aparece en vivo en la tarea.
pub(crate) async fn system_comment(st: &AppState, task: Id, user: Id, body: String) {
    let Ok(team) = team_of_task(st, task).await else { return };
    let Ok(id) = sqlx::query("INSERT INTO comments (task_id, user_id, body, created_at) VALUES (?, ?, ?, ?)").bind(task).bind(user).bind(body).bind(Utc::now()).execute(&st.db).await.map(|r| r.last_insert_rowid()) else { return };
    if let Ok(row) = sqlx::query(sqlx::AssertSqlSafe(format!("{COMMENT_SELECT} WHERE c.id = ?"))).bind(id).fetch_one(&st.db).await {
        st.emit(team, Event::CommentAdded(comment_from_row(&row)));
    }
}

#[cfg(test)]
mod backtick_tests {
    use super::backtick_cmds;

    #[test]
    fn comandos_en_bloque_y_en_linea() {
        let q = "Necesito esto:\n```bash\n$ git clone https://x/y /tmp/y\ncargo build\n```\ny además `rar a x.rar .` por favor";
        assert_eq!(backtick_cmds(q), vec!["git clone https://x/y /tmp/y", "cargo build", "rar a x.rar ."]);
        assert!(backtick_cmds("sin comandos").is_empty());
    }
}
