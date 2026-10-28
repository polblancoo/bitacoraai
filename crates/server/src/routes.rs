use crate::{
    limits::{self, *},
    auth::{hash_password, make_token, verify_password, AuthUser},
    error::{AppError, Result},
    AppState,
};
use axum::{
    extract::{ConnectInfo, Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, patch, post, put},
    Json, Router,
};
use chrono::Utc;
use std::{net::SocketAddr, time::Duration};
use sqlx::{sqlite::SqliteRow, Row};
use tm_core::*;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/register", post(register))
        .route("/api/login", post(login))
        .route("/api/logout-all", post(logout_all))
        .route("/api/teams/{id}/labels", get(list_labels).post(create_label))
        .route("/api/labels/{id}", axum::routing::delete(delete_label))
        .route("/api/tasks/{id}/labels", put(set_labels))
        .route("/api/teams", get(list_teams).post(create_team))
        .route("/api/teams/{id}/members", get(list_members).post(add_member))
        .route("/api/teams/{id}/members/{uid}", axum::routing::put(set_role).delete(remove_member))
        .route("/api/teams/{id}/invites", get(list_invites).post(create_invite))
        .route("/api/invites/{id}", axum::routing::delete(revoke_invite))
        .route("/api/invites/{id}/accept", post(accept_invite))
        .route("/api/teams/{id}/projects", get(list_projects).post(create_project))
        .route("/api/projects/{id}/tasks", get(list_tasks).post(create_task))
        .route("/api/tasks/{id}", patch(update_task).delete(delete_task))
        .route("/api/tasks/{id}/comments", get(list_comments).post(add_comment))
}

// ---------- helpers ----------

async fn role_of(st: &AppState, team: Id, user: Id) -> Result<Role> {
    let r: Option<String> = sqlx::query_scalar("SELECT role FROM team_members WHERE team_id = ? AND user_id = ?")
        .bind(team)
        .bind(user)
        .fetch_optional(&st.db)
        .await?;
    r.and_then(|r| Role::parse(&r)).ok_or_else(AppError::forbidden)
}

/// Exige al menos el rol `min` y devuelve el rol real.
async fn require(st: &AppState, team: Id, user: Id, min: Role) -> Result<Role> {
    let r = role_of(st, team, user).await?;
    if r < min { Err(AppError::forbidden()) } else { Ok(r) }
}

/// Cualquier miembro (incluido lector): lecturas.
async fn ensure_member(st: &AppState, team: Id, user: Id) -> Result<()> {
    require(st, team, user, Role::Viewer).await.map(|_| ())
}

/// Miembro o superior: escrituras (tareas, comentarios, etiquetas, proyectos).
async fn ensure_writer(st: &AppState, team: Id, user: Id) -> Result<()> {
    require(st, team, user, Role::Member).await.map(|_| ())
}

/// Un rol se puede otorgar solo si es inferior al del que lo otorga (nadie crea otro dueño).
fn can_grant(caller: Role, role: Role) -> bool {
    role != Role::Owner && role < caller
}

async fn team_of_project(st: &AppState, project: Id) -> Result<Id> {
    Ok(sqlx::query_scalar("SELECT team_id FROM projects WHERE id = ?").bind(project).fetch_one(&st.db).await?)
}

async fn team_of_task(st: &AppState, task: Id) -> Result<Id> {
    Ok(sqlx::query_scalar(
        "SELECT p.team_id FROM tasks t JOIN projects p ON p.id = t.project_id WHERE t.id = ?",
    )
    .bind(task)
    .fetch_one(&st.db)
    .await?)
}

const TASK_SELECT: &str = "SELECT t.id, t.project_id, t.title, t.description, t.status, t.priority, \
    t.assignee_id, u.username AS assignee, t.due, t.parent_id, t.created_at, t.updated_at, \
    (SELECT COUNT(*) FROM tasks s WHERE s.parent_id = t.id) AS sub_total, \
    (SELECT COUNT(*) FROM tasks s WHERE s.parent_id = t.id AND s.status = 'done') AS sub_done \
    FROM tasks t LEFT JOIN users u ON u.id = t.assignee_id";

fn task_from_row(r: &SqliteRow) -> Task {
    Task {
        id: r.get("id"),
        project_id: r.get("project_id"),
        title: r.get("title"),
        description: r.get("description"),
        status: Status::parse(r.get("status")).unwrap_or(Status::Todo),
        priority: r.get("priority"),
        assignee_id: r.get("assignee_id"),
        assignee: r.get("assignee"),
        due: r.get("due"),
        parent_id: r.get("parent_id"),
        labels: vec![],
        subtasks_total: r.get("sub_total"),
        subtasks_done: r.get("sub_done"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    }
}

fn label_from_row(r: &SqliteRow) -> Label {
    Label { id: r.get("id"), team_id: r.get("team_id"), name: r.get("name"), color: r.get("color") }
}

/// Completa `labels` de las tareas con una sola consulta (`scope` filtra `task_id`).
async fn hydrate_labels(st: &AppState, tasks: &mut [Task], scope: &'static str, id: Id) -> Result<()> {
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

async fn fetch_task(st: &AppState, id: Id) -> Result<Task> {
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
    }
}

async fn count(st: &AppState, sql: &'static str, id: Id) -> Result<i64> {
    Ok(sqlx::query_scalar(sql).bind(id).fetch_one(&st.db).await?)
}

fn throttle(st: &AppState, headers: &HeaderMap, peer: SocketAddr) -> Result<()> {
    let ip = limits::client_ip(headers, peer, st.trust_proxy);
    st.ip_limiter.hit(&ip, 20, Duration::from_secs(60))
}

// ---------- auth ----------

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
    sqlx::query("INSERT INTO team_members (team_id, user_id, role) VALUES (?, ?, 'owner')").bind(id).bind(user.id).execute(&mut *tx).await?;
    tx.commit().await?;
    // el dueño ya tiene abierto el WebSocket: este evento hace que se suscriba al equipo nuevo
    let username: String = sqlx::query_scalar("SELECT username FROM users WHERE id = ?").bind(user.id).fetch_one(&st.db).await?;
    st.emit(id, Event::MemberAdded { team_id: id, user: User { id: user.id, username } });
    Ok(Json(Team { id, name: n.name.trim().into(), owner_id: user.id }))
}

async fn list_members(State(st): State<AppState>, user: AuthUser, Path(team): Path<Id>) -> Result<Json<Vec<Member>>> {
    ensure_member(&st, team, user.id).await?;
    let rows = sqlx::query(
        "SELECT u.id, u.username, m.role FROM users u JOIN team_members m ON m.user_id = u.id \
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
            })
            .collect(),
    ))
}

async fn insert_member(st: &AppState, team: Id, user: &User, role: Role) -> Result<bool> {
    if count(st, "SELECT COUNT(*) FROM team_members WHERE team_id = ?", team).await? >= MAX_MEMBERS_PER_TEAM {
        return Err(too_many("miembros", MAX_MEMBERS_PER_TEAM));
    }
    let res = sqlx::query("INSERT OR IGNORE INTO team_members (team_id, user_id, role) VALUES (?, ?, ?)")
        .bind(team)
        .bind(user.id)
        .bind(role.as_str())
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
    let caller = require(&st, team, user.id, Role::Admin).await?;
    let role = a.role.unwrap_or(Role::Member);
    if !can_grant(caller, role) {
        return Err(AppError::forbidden());
    }
    max_len("usuario", &a.username, USERNAME_MAX)?;
    let row = sqlx::query("SELECT id, username FROM users WHERE username = ?").bind(a.username.trim()).fetch_one(&st.db).await?;
    let new = User { id: row.get("id"), username: row.get("username") };
    insert_member(&st, team, &new, role).await?;
    Ok(Json(new))
}

async fn set_role(
    State(st): State<AppState>,
    user: AuthUser,
    Path((team, target)): Path<(Id, Id)>,
    Json(b): Json<SetRole>,
) -> Result<StatusCode> {
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
    Ok(StatusCode::NO_CONTENT)
}

/// Quita a un miembro (admin+ sobre rangos inferiores) o permite salir del equipo; el dueño no puede irse.
async fn remove_member(
    State(st): State<AppState>,
    user: AuthUser,
    Path((team, target)): Path<(Id, Id)>,
) -> Result<StatusCode> {
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
         RETURNING team_id, role",
    )
    .bind(&hash)
    .bind(Utc::now())
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::bad("invitación inválida, vencida o agotada"))?;
    let (team, role): (Id, Role) = (used.get("team_id"), Role::parse(used.get("role")).unwrap_or(Role::Viewer));
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
    insert_member(&st, team, &me, role).await?;
    let row = sqlx::query("SELECT id, name, owner_id FROM teams WHERE id = ?").bind(team).fetch_one(&st.db).await?;
    Ok(Json(Team { id: row.get("id"), name: row.get("name"), owner_id: row.get("owner_id") }))
}

// ---------- projects ----------

async fn list_projects(State(st): State<AppState>, user: AuthUser, Path(team): Path<Id>) -> Result<Json<Vec<Project>>> {
    ensure_member(&st, team, user.id).await?;
    let rows = sqlx::query("SELECT id, team_id, name, description, color FROM projects WHERE team_id = ? ORDER BY name LIMIT 200")
        .bind(team)
        .fetch_all(&st.db)
        .await?;
    Ok(Json(rows.iter().map(project_from_row).collect()))
}

async fn create_project(
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
    if count(&st, "SELECT COUNT(*) FROM projects WHERE team_id = ?", team).await? >= MAX_PROJECTS_PER_TEAM {
        return Err(too_many("proyectos", MAX_PROJECTS_PER_TEAM));
    }
    let id = sqlx::query("INSERT INTO projects (team_id, name, description, color) VALUES (?, ?, ?, ?)")
        .bind(team)
        .bind(n.name.trim())
        .bind(&n.description)
        .bind(&color)
        .execute(&st.db)
        .await?
        .last_insert_rowid();
    let p = Project { id, team_id: team, name: n.name.trim().into(), description: n.description, color };
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

async fn list_tasks(State(st): State<AppState>, user: AuthUser, Path(project): Path<Id>) -> Result<Json<Vec<Task>>> {
    ensure_member(&st, team_of_project(&st, project).await?, user.id).await?;
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!("{TASK_SELECT} WHERE t.project_id = ? ORDER BY t.priority DESC, t.id LIMIT 5000")))
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
    Ok(Json(tasks))
}

async fn create_task(
    State(st): State<AppState>,
    user: AuthUser,
    Path(project): Path<Id>,
    Json(n): Json<NewTask>,
) -> Result<Json<Task>> {
    let team = team_of_project(&st, project).await?;
    ensure_writer(&st, team, user.id).await?;
    if n.title.trim().is_empty() {
        return Err(AppError::bad("título requerido"));
    }
    max_len("título", &n.title, TITLE_MAX)?;
    max_len("descripción", &n.description, DESCRIPTION_MAX)?;
    if count(&st, "SELECT COUNT(*) FROM tasks WHERE project_id = ?", project).await? >= MAX_TASKS_PER_PROJECT {
        return Err(too_many("tareas", MAX_TASKS_PER_PROJECT));
    }
    if let Some(a) = n.assignee_id {
        ensure_member(&st, team, a).await.map_err(|_| AppError::bad("el asignado no es miembro del equipo"))?;
    }
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
        "INSERT INTO tasks (project_id, title, description, priority, assignee_id, due, parent_id, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(project)
    .bind(n.title.trim())
    .bind(&n.description)
    .bind(n.priority.clamp(0, 3))
    .bind(n.assignee_id)
    .bind(n.due)
    .bind(n.parent_id)
    .bind(now)
    .bind(now)
    .execute(&st.db)
    .await?
    .last_insert_rowid();
    let t = fetch_task(&st, id).await?;
    st.emit(team, Event::TaskCreated(t.clone()));
    emit_parent(&st, team, t.parent_id).await;
    Ok(Json(t))
}

async fn update_task(
    State(st): State<AppState>,
    user: AuthUser,
    Path(id): Path<Id>,
    Json(p): Json<TaskPatch>,
) -> Result<Json<Task>> {
    let team = team_of_task(&st, id).await?;
    ensure_writer(&st, team, user.id).await?;
    if let Some(t) = &p.title { max_len("título", t, TITLE_MAX)?; }
    if let Some(d) = &p.description { max_len("descripción", d, DESCRIPTION_MAX)?; }
    let cur = fetch_task(&st, id).await?;
    if let Some(Some(a)) = p.assignee_id {
        ensure_member(&st, team, a).await.map_err(|_| AppError::bad("el asignado no es miembro del equipo"))?;
    }
    let assignee = p.assignee_id.unwrap_or(cur.assignee_id);
    let due = p.due.unwrap_or(cur.due);
    sqlx::query(
        "UPDATE tasks SET title = ?, description = ?, status = ?, priority = ?, assignee_id = ?, due = ?, updated_at = ? \
         WHERE id = ?",
    )
    .bind(p.title.as_deref().map(str::trim).filter(|s| !s.is_empty()).unwrap_or(&cur.title))
    .bind(p.description.as_deref().unwrap_or(&cur.description))
    .bind(p.status.unwrap_or(cur.status).as_str())
    .bind(p.priority.map(|v| v.clamp(0, 3)).unwrap_or(cur.priority))
    .bind(assignee)
    .bind(due)
    .bind(Utc::now())
    .bind(id)
    .execute(&st.db)
    .await?;
    let t = fetch_task(&st, id).await?;
    st.emit(team, Event::TaskUpdated(t.clone()));
    emit_parent(&st, team, t.parent_id).await;
    Ok(Json(t))
}

async fn delete_task(State(st): State<AppState>, user: AuthUser, Path(id): Path<Id>) -> Result<StatusCode> {
    let team = team_of_task(&st, id).await?;
    ensure_writer(&st, team, user.id).await?;
    let row = sqlx::query("SELECT project_id, parent_id FROM tasks WHERE id = ?").bind(id).fetch_one(&st.db).await?;
    let (project, parent): (Id, Option<Id>) = (row.get("project_id"), row.get("parent_id"));
    sqlx::query("DELETE FROM tasks WHERE id = ?").bind(id).execute(&st.db).await?;
    st.emit(team, Event::TaskDeleted { id, project_id: project });
    emit_parent(&st, team, parent).await;
    Ok(StatusCode::NO_CONTENT)
}

// ---------- comments ----------

fn comment_from_row(r: &SqliteRow) -> Comment {
    Comment {
        id: r.get("id"),
        task_id: r.get("task_id"),
        user_id: r.get("user_id"),
        username: r.get("username"),
        body: r.get("body"),
        created_at: r.get("created_at"),
    }
}

const COMMENT_SELECT: &str = "SELECT c.id, c.task_id, c.user_id, u.username, c.body, c.created_at \
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
    Json(n): Json<NewComment>,
) -> Result<Json<Comment>> {
    let team = team_of_task(&st, task).await?;
    ensure_writer(&st, team, user.id).await?;
    if n.body.trim().is_empty() {
        return Err(AppError::bad("comentario vacío"));
    }
    max_len("comentario", &n.body, COMMENT_MAX)?;
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
