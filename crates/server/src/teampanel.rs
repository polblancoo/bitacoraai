//! Panel del equipo: resumen con miembros (rol, desde cuándo, quién los sumó, carga), invitaciones con
//! quiénes entraron con cada código, y ajustes del equipo (renombrar, transferir la propiedad, eliminar).
use crate::{
    auth::AuthUser,
    error::{AppError, Result},
    limits::{max_len, NAME_MAX},
    routes::{require, role_of},
    AppState,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use tm_core::{Event, Id, Role};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/teams/{id}/overview", get(overview))
        .route("/api/teams/{id}", axum::routing::patch(update_team).delete(delete_team))
        .route("/api/teams/{id}/transfer", post(transfer))
        .route("/api/projects/{id}", axum::routing::patch(update_project).delete(delete_project))
}

async fn overview(State(st): State<AppState>, user: AuthUser, Path(team): Path<Id>) -> Result<Json<Value>> {
    let my_role = role_of(&st, team, user.id).await?;
    let t = sqlx::query("SELECT name, description, owner_id FROM teams WHERE id = ?").bind(team).fetch_one(&st.db).await?;
    let projects: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects WHERE team_id = ?").bind(team).fetch_one(&st.db).await?;
    let open: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tasks t JOIN projects p ON p.id = t.project_id WHERE p.team_id = ? AND t.status != 'done'",
    )
    .bind(team)
    .fetch_one(&st.db)
    .await?;
    let members = sqlx::query(
        "SELECT u.id, u.username, u.is_bot, u.model, m.role, m.joined_at, m.via, ib.username AS invited_by, \
         (SELECT COUNT(*) FROM tasks t JOIN projects p ON p.id = t.project_id \
          WHERE p.team_id = m.team_id AND t.assignee_id = u.id AND t.status != 'done') AS open_tasks \
         FROM team_members m JOIN users u ON u.id = m.user_id LEFT JOIN users ib ON ib.id = m.invited_by \
         WHERE m.team_id = ? ORDER BY m.role = 'owner' DESC, u.username",
    )
    .bind(team)
    .fetch_all(&st.db)
    .await?;
    let members: Vec<Value> = members
        .iter()
        .map(|r| {
            json!({
                "id": r.get::<Id, _>("id"), "username": r.get::<String, _>("username"), "bot": r.get::<i64, _>("is_bot") == 1, "model": r.get::<String, _>("model"), "role": r.get::<String, _>("role"),
                "joined_at": r.get::<Option<String>, _>("joined_at"), "via": r.get::<Option<String>, _>("via"),
                "invited_by": r.get::<Option<String>, _>("invited_by"), "open_tasks": r.get::<i64, _>("open_tasks"),
            })
        })
        .collect();

    // las invitaciones y quién entró con cada una solo las ven admin y dueño
    let mut invites = Vec::new();
    if my_role >= Role::Admin {
        let rows = sqlx::query(
            "SELECT i.id, i.role, i.uses, i.max_uses, i.expires_at, u.username AS created_by FROM invites i \
             LEFT JOIN users u ON u.id = i.created_by WHERE i.team_id = ? ORDER BY i.id DESC LIMIT 50",
        )
        .bind(team)
        .fetch_all(&st.db)
        .await?;
        for r in &rows {
            let id: Id = r.get("id");
            let joined = sqlx::query(
                "SELECT u.username, m.joined_at FROM team_members m JOIN users u ON u.id = m.user_id \
                 WHERE m.team_id = ? AND m.invite_id = ? ORDER BY m.joined_at",
            )
            .bind(team)
            .bind(id)
            .fetch_all(&st.db)
            .await?;
            invites.push(json!({
                "id": id, "role": r.get::<String, _>("role"), "uses": r.get::<i64, _>("uses"), "max_uses": r.get::<i64, _>("max_uses"),
                "expires_at": r.get::<String, _>("expires_at"), "created_by": r.get::<Option<String>, _>("created_by"),
                "joined": joined.iter().map(|j| json!({ "username": j.get::<String, _>("username"), "joined_at": j.get::<Option<String>, _>("joined_at") })).collect::<Vec<_>>(),
            }));
        }
    }
    Ok(Json(json!({
        "id": team, "name": t.get::<String, _>("name"), "description": t.get::<String, _>("description"), "owner_id": t.get::<Id, _>("owner_id"),
        "my_role": my_role.as_str(), "projects": projects, "open_tasks": open, "members": members, "invites": invites,
    })))
}

#[derive(Deserialize)]
struct UpdateTeam {
    name: Option<String>,
    description: Option<String>,
}

async fn update_team(State(st): State<AppState>, user: AuthUser, Path(team): Path<Id>, Json(b): Json<UpdateTeam>) -> Result<StatusCode> {
    user.human_only()?;
    require(&st, team, user.id, Role::Admin).await?;
    if let Some(n) = &b.name {
        if n.trim().is_empty() {
            return Err(AppError::bad("nombre requerido"));
        }
        max_len("nombre", n, NAME_MAX)?;
        sqlx::query("UPDATE teams SET name = ? WHERE id = ?").bind(n.trim()).bind(team).execute(&st.db).await?;
    }
    if let Some(d) = &b.description {
        max_len("descripción", d, 500)?;
        sqlx::query("UPDATE teams SET description = ? WHERE id = ?").bind(d.trim()).bind(team).execute(&st.db).await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct Transfer {
    user_id: Id,
}

/// Solo el dueño: el nuevo dueño recibe el rol owner y el anterior pasa a admin.
async fn transfer(State(st): State<AppState>, user: AuthUser, Path(team): Path<Id>, Json(b): Json<Transfer>) -> Result<StatusCode> {
    user.human_only()?;
    require(&st, team, user.id, Role::Owner).await?;
    if b.user_id == user.id {
        return Err(AppError::bad("ya sos el dueño"));
    }
    role_of(&st, team, b.user_id).await.map_err(|_| AppError::bad("esa persona no es miembro del equipo"))?;
    let mut tx = st.db.begin().await?;
    sqlx::query("UPDATE teams SET owner_id = ? WHERE id = ?").bind(b.user_id).bind(team).execute(&mut *tx).await?;
    sqlx::query("UPDATE team_members SET role = 'owner' WHERE team_id = ? AND user_id = ?").bind(team).bind(b.user_id).execute(&mut *tx).await?;
    sqlx::query("UPDATE team_members SET role = 'admin' WHERE team_id = ? AND user_id = ?").bind(team).bind(user.id).execute(&mut *tx).await?;
    tx.commit().await?;
    st.emit(team, Event::MemberRoleChanged { team_id: team, user_id: b.user_id, role: Role::Owner });
    st.emit(team, Event::MemberRoleChanged { team_id: team, user_id: user.id, role: Role::Admin });
    let name: String = sqlx::query_scalar("SELECT name FROM teams WHERE id = ?").bind(team).fetch_one(&st.db).await?;
    let who = crate::notify::username_of(&st, user.id).await;
    crate::notify::notify(&st, b.user_id, "role_changed", Some(team), None, format!("{who} te transfirió la propiedad del equipo «{name}»: ahora sos Dueño")).await;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct Confirm {
    confirm: String,
}

/// Solo el dueño, y hay que repetir el nombre del equipo (`?confirm=`). Borra proyectos, tareas e invitaciones.
async fn delete_team(State(st): State<AppState>, user: AuthUser, Path(team): Path<Id>, Query(c): Query<Confirm>) -> Result<StatusCode> {
    user.human_only()?;
    require(&st, team, user.id, Role::Owner).await?;
    let name: String = sqlx::query_scalar("SELECT name FROM teams WHERE id = ?").bind(team).fetch_one(&st.db).await?;
    if c.confirm.trim() != name {
        return Err(AppError::bad("para eliminar el equipo hay que escribir su nombre exacto"));
    }
    sqlx::query("DELETE FROM teams WHERE id = ?").bind(team).execute(&st.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct UpdateProject {
    name: Option<String>,
    description: Option<String>,
    work_dir: Option<String>,
    repo: Option<String>,
}

/// Nombre, descripción, carpeta de trabajo y repositorio del proyecto: los define un admin o el dueño.
async fn update_project(State(st): State<AppState>, user: AuthUser, Path(id): Path<Id>, Json(b): Json<UpdateProject>) -> Result<Json<Value>> {
    user.human_only()?;
    let team: Id = sqlx::query_scalar("SELECT team_id FROM projects WHERE id = ?").bind(id).fetch_optional(&st.db).await?.ok_or_else(AppError::not_found)?;
    require(&st, team, user.id, Role::Admin).await?;
    if let Some(n) = &b.name {
        if n.trim().is_empty() {
            return Err(AppError::bad("nombre requerido"));
        }
        max_len("nombre", n, NAME_MAX)?;
    }
    if let Some(d) = &b.description { max_len("descripción", d, crate::limits::DESCRIPTION_MAX)?; }
    if let Some(w) = &b.work_dir { max_len("carpeta", w, 300)?; }
    if let Some(r) = &b.repo { max_len("repositorio", r, 300)?; }
    sqlx::query(
        "UPDATE projects SET name = COALESCE(?, name), description = COALESCE(?, description), \
         work_dir = COALESCE(?, work_dir), repo = COALESCE(?, repo) WHERE id = ?",
    )
    .bind(b.name.as_deref().map(str::trim))
    .bind(b.description.as_deref())
    .bind(b.work_dir.as_deref().map(str::trim))
    .bind(b.repo.as_deref().map(str::trim))
    .bind(id)
    .execute(&st.db)
    .await?;
    let r = sqlx::query("SELECT id, team_id, name, description, color, work_dir, repo FROM projects WHERE id = ?").bind(id).fetch_one(&st.db).await?;
    Ok(Json(json!({
        "id": id, "team_id": team, "name": r.get::<String, _>("name"), "description": r.get::<String, _>("description"),
        "color": r.get::<String, _>("color"), "work_dir": r.get::<String, _>("work_dir"), "repo": r.get::<String, _>("repo"),
    })))
}

/// Elimina el proyecto con todas sus tareas; un admin o el dueño, confirmando con el nombre.
async fn delete_project(State(st): State<AppState>, user: AuthUser, Path(id): Path<Id>, Query(c): Query<Confirm>) -> Result<StatusCode> {
    user.human_only()?;
    let row = sqlx::query("SELECT team_id, name FROM projects WHERE id = ?").bind(id).fetch_optional(&st.db).await?.ok_or_else(AppError::not_found)?;
    let (team, name): (Id, String) = (row.get("team_id"), row.get("name"));
    require(&st, team, user.id, Role::Admin).await?;
    if c.confirm != name {
        return Err(AppError::bad("escribí el nombre del proyecto para confirmar"));
    }
    sqlx::query("DELETE FROM projects WHERE id = ?").bind(id).execute(&st.db).await?;
    st.emit(team, Event::ProjectDeleted { id, team_id: team });
    Ok(StatusCode::NO_CONTENT)
}
