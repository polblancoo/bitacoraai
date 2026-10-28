//! Pizarra por proyecto: notas libres y tarjetas de tareas, conectadas con flechas, en tiempo real.
use crate::{
    auth::AuthUser,
    error::{AppError, Result},
    limits::{self, max_len},
    routes::{count, ensure_member, ensure_writer, team_of_project},
    AppState,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, patch, post},
    Json, Router,
};
use sqlx::{sqlite::SqliteRow, Row};
use tm_core::*;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/projects/{id}/canvas", get(get_canvas))
        .route("/api/projects/{id}/canvas/items", post(create_item))
        .route("/api/canvas/items/{id}", patch(update_item).delete(delete_item))
        .route("/api/projects/{id}/canvas/links", post(create_link))
        .route("/api/canvas/links/{id}", axum::routing::delete(delete_link))
}

const ITEM_SELECT: &str = "SELECT id, project_id, kind, x, y, w, h, text, color, task_id FROM canvas_items";

fn item_from_row(r: &SqliteRow) -> CanvasItem {
    CanvasItem {
        id: r.get("id"),
        project_id: r.get("project_id"),
        kind: r.get("kind"),
        x: r.get("x"),
        y: r.get("y"),
        w: r.get("w"),
        h: r.get("h"),
        text: r.get("text"),
        color: r.get("color"),
        task_id: r.get("task_id"),
    }
}

fn link_from_row(r: &SqliteRow) -> CanvasLink {
    CanvasLink { id: r.get("id"), project_id: r.get("project_id"), from_id: r.get("from_id"), to_id: r.get("to_id") }
}

fn coord(name: &str, v: f64) -> Result<f64> {
    if v.is_finite() && v.abs() <= limits::CANVAS_COORD_MAX {
        Ok(v)
    } else {
        Err(AppError::bad(&format!("{name} fuera de rango")))
    }
}

fn size(name: &str, v: f64) -> Result<f64> {
    if v.is_finite() && (40.0..=2000.0).contains(&v) {
        Ok(v)
    } else {
        Err(AppError::bad(&format!("{name}: entre 40 y 2000")))
    }
}

async fn item_project(st: &AppState, id: Id) -> Result<Id> {
    Ok(sqlx::query_scalar("SELECT project_id FROM canvas_items WHERE id = ?").bind(id).fetch_one(&st.db).await?)
}

async fn get_canvas(State(st): State<AppState>, user: AuthUser, Path(project): Path<Id>) -> Result<Json<Canvas>> {
    ensure_member(&st, team_of_project(&st, project).await?, user.id).await?;
    let items = sqlx::query(sqlx::AssertSqlSafe(format!("{ITEM_SELECT} WHERE project_id = ? ORDER BY id LIMIT 500")))
        .bind(project)
        .fetch_all(&st.db)
        .await?;
    let links = sqlx::query("SELECT id, project_id, from_id, to_id FROM canvas_links WHERE project_id = ? ORDER BY id LIMIT 1000")
        .bind(project)
        .fetch_all(&st.db)
        .await?;
    Ok(Json(Canvas { items: items.iter().map(item_from_row).collect(), links: links.iter().map(link_from_row).collect() }))
}

async fn create_item(
    State(st): State<AppState>,
    user: AuthUser,
    Path(project): Path<Id>,
    Json(n): Json<NewCanvasItem>,
) -> Result<Json<CanvasItem>> {
    let team = team_of_project(&st, project).await?;
    ensure_writer(&st, team, user.id).await?;
    if n.kind != "note" && n.kind != "task" {
        return Err(AppError::bad("tipo: note o task"));
    }
    max_len("texto", &n.text, limits::CANVAS_TEXT_MAX)?;
    let color = n.color.unwrap_or_else(|| "#e0af68".into());
    limits::color(&color)?;
    if count(&st, "SELECT COUNT(*) FROM canvas_items WHERE project_id = ?", project).await? >= limits::MAX_CANVAS_ITEMS {
        return Err(limits::too_many("elementos en la pizarra", limits::MAX_CANVAS_ITEMS));
    }
    let (mut text, task_id) = (n.text, n.task_id);
    if n.kind == "task" {
        let tid = task_id.ok_or_else(|| AppError::bad("falta task_id"))?;
        let title: Option<String> = sqlx::query_scalar("SELECT title FROM tasks WHERE id = ? AND project_id = ?")
            .bind(tid)
            .bind(project)
            .fetch_optional(&st.db)
            .await?;
        text = title.ok_or_else(|| AppError::bad("la tarea no es de este proyecto"))?;
    }
    let id = sqlx::query("INSERT INTO canvas_items (project_id, kind, x, y, w, h, text, color, task_id) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(project)
        .bind(&n.kind)
        .bind(coord("x", n.x)?)
        .bind(coord("y", n.y)?)
        .bind(size("ancho", n.w.unwrap_or(200.0))?)
        .bind(size("alto", n.h.unwrap_or(110.0))?)
        .bind(&text)
        .bind(&color)
        .bind(if n.kind == "task" { task_id } else { None })
        .execute(&st.db)
        .await?
        .last_insert_rowid();
    let row = sqlx::query(sqlx::AssertSqlSafe(format!("{ITEM_SELECT} WHERE id = ?"))).bind(id).fetch_one(&st.db).await?;
    let item = item_from_row(&row);
    st.emit(team, Event::CanvasItem(item.clone()));
    Ok(Json(item))
}

async fn update_item(
    State(st): State<AppState>,
    user: AuthUser,
    Path(id): Path<Id>,
    Json(p): Json<CanvasPatch>,
) -> Result<Json<CanvasItem>> {
    let project = item_project(&st, id).await?;
    let team = team_of_project(&st, project).await?;
    ensure_writer(&st, team, user.id).await?;
    let row = sqlx::query(sqlx::AssertSqlSafe(format!("{ITEM_SELECT} WHERE id = ?"))).bind(id).fetch_one(&st.db).await?;
    let cur = item_from_row(&row);
    if let Some(t) = &p.text { max_len("texto", t, limits::CANVAS_TEXT_MAX)?; }
    if let Some(c) = &p.color { limits::color(c)?; }
    // las tarjetas de tarea toman su texto de la tarea: no se editan acá
    let text = if cur.kind == "task" { cur.text.clone() } else { p.text.unwrap_or(cur.text.clone()) };
    sqlx::query("UPDATE canvas_items SET x = ?, y = ?, w = ?, h = ?, text = ?, color = ? WHERE id = ?")
        .bind(coord("x", p.x.unwrap_or(cur.x))?)
        .bind(coord("y", p.y.unwrap_or(cur.y))?)
        .bind(size("ancho", p.w.unwrap_or(cur.w))?)
        .bind(size("alto", p.h.unwrap_or(cur.h))?)
        .bind(&text)
        .bind(p.color.as_deref().unwrap_or(&cur.color))
        .bind(id)
        .execute(&st.db)
        .await?;
    let row = sqlx::query(sqlx::AssertSqlSafe(format!("{ITEM_SELECT} WHERE id = ?"))).bind(id).fetch_one(&st.db).await?;
    let item = item_from_row(&row);
    st.emit(team, Event::CanvasItem(item.clone()));
    Ok(Json(item))
}

async fn delete_item(State(st): State<AppState>, user: AuthUser, Path(id): Path<Id>) -> Result<StatusCode> {
    let project = item_project(&st, id).await?;
    let team = team_of_project(&st, project).await?;
    ensure_writer(&st, team, user.id).await?;
    sqlx::query("DELETE FROM canvas_items WHERE id = ?").bind(id).execute(&st.db).await?;
    st.emit(team, Event::CanvasItemDeleted { id, project_id: project });
    Ok(StatusCode::NO_CONTENT)
}

async fn create_link(
    State(st): State<AppState>,
    user: AuthUser,
    Path(project): Path<Id>,
    Json(n): Json<NewCanvasLink>,
) -> Result<Json<CanvasLink>> {
    let team = team_of_project(&st, project).await?;
    ensure_writer(&st, team, user.id).await?;
    if n.from_id == n.to_id {
        return Err(AppError::bad("no se puede conectar un elemento consigo mismo"));
    }
    let both: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM canvas_items WHERE project_id = ? AND id IN (?, ?)")
        .bind(project)
        .bind(n.from_id)
        .bind(n.to_id)
        .fetch_one(&st.db)
        .await?;
    if both != 2 {
        return Err(AppError::bad("los dos elementos deben ser de este proyecto"));
    }
    if count(&st, "SELECT COUNT(*) FROM canvas_links WHERE project_id = ?", project).await? >= limits::MAX_CANVAS_LINKS {
        return Err(limits::too_many("conexiones", limits::MAX_CANVAS_LINKS));
    }
    let res = sqlx::query("INSERT OR IGNORE INTO canvas_links (project_id, from_id, to_id) VALUES (?, ?, ?)")
        .bind(project)
        .bind(n.from_id)
        .bind(n.to_id)
        .execute(&st.db)
        .await?;
    let row = sqlx::query("SELECT id, project_id, from_id, to_id FROM canvas_links WHERE from_id = ? AND to_id = ?")
        .bind(n.from_id)
        .bind(n.to_id)
        .fetch_one(&st.db)
        .await?;
    let link = link_from_row(&row);
    if res.rows_affected() > 0 {
        st.emit(team, Event::CanvasLink(link.clone()));
    }
    Ok(Json(link))
}

async fn delete_link(State(st): State<AppState>, user: AuthUser, Path(id): Path<Id>) -> Result<StatusCode> {
    let project: Id = sqlx::query_scalar("SELECT project_id FROM canvas_links WHERE id = ?").bind(id).fetch_one(&st.db).await?;
    let team = team_of_project(&st, project).await?;
    ensure_writer(&st, team, user.id).await?;
    sqlx::query("DELETE FROM canvas_links WHERE id = ?").bind(id).execute(&st.db).await?;
    st.emit(team, Event::CanvasLinkDeleted { id, project_id: project });
    Ok(StatusCode::NO_CONTENT)
}
