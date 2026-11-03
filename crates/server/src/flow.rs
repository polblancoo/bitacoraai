//! Flujo de una tarea: una cadena de etapas (hacer → revisar → …), cada una con su responsable y su instrucción.
//! Cuando una etapa termina, la tarea pasa sola a la siguiente con lo que entregó la anterior; el revisor aprueba o
//! devuelve (con el motivo) a la etapa de trabajo, hasta un tope de vueltas para que no se pasen la tarea para siempre.
use crate::{
    auth::AuthUser,
    error::{AppError, Result},
    routes::{self, comment_from_row, COMMENT_SELECT},
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
use tm_core::{Event, Id, Role, Task};

const MAX_STEPS: usize = 8;
const INSTRUCTION_MAX: usize = 2000;
const DELIVERY_MAX: usize = 3000;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/tasks/{id}/flow", get(get_flow).put(put_flow).delete(delete_flow))
        .route("/api/tasks/{id}/flow/verdict", post(verdict))
        .route("/api/projects/{id}/flows", get(project_flows))
}

struct Step {
    assignee_id: Id,
    username: String,
    bot: bool,
    kind: String,
    instruction: String,
}

struct Flow {
    step: i64,
    round: i64,
    max_rounds: i64,
    from_comment: Id,
    finished: bool,
}

async fn load(st: &AppState, task: Id) -> Result<Option<Flow>> {
    let r = sqlx::query("SELECT step, round, max_rounds, from_comment, finished FROM task_flows WHERE task_id = ?").bind(task).fetch_optional(&st.db).await?;
    Ok(r.map(|r| Flow { step: r.get(0), round: r.get(1), max_rounds: r.get(2), from_comment: r.get(3), finished: r.get::<i64, _>(4) != 0 }))
}

async fn steps(st: &AppState, task: Id) -> Result<Vec<Step>> {
    let rows = sqlx::query(
        "SELECT s.assignee_id, u.username, s.kind, s.instruction, EXISTS(SELECT 1 FROM api_tokens WHERE user_id = s.assignee_id) AS bot \
         FROM flow_steps s JOIN users u ON u.id = s.assignee_id WHERE s.task_id = ? ORDER BY s.pos",
    )
    .bind(task)
    .fetch_all(&st.db)
    .await?;
    Ok(rows.iter().map(|r| Step { assignee_id: r.get(0), username: r.get(1), kind: r.get(2), instruction: r.get(3), bot: r.get::<i64, _>(4) != 0 }).collect())
}

fn trunc(s: &str, n: usize) -> String {
    let t: String = s.chars().take(n).collect();
    if s.chars().count() > n { format!("{t}…") } else { t }
}

/// Escribe un comentario del flujo (con el nombre de quien lo dispara) y lo avisa a quien mire la tarea.
async fn post_comment(st: &AppState, team: Id, task: Id, author: Id, body: String) -> Id {
    let Ok(id) = sqlx::query("INSERT INTO comments (task_id, user_id, body, created_at) VALUES (?, ?, ?, ?)")
        .bind(task)
        .bind(author)
        .bind(body)
        .bind(Utc::now())
        .execute(&st.db)
        .await
        .map(|r| r.last_insert_rowid())
    else {
        return 0;
    };
    if let Ok(row) = sqlx::query(sqlx::AssertSqlSafe(format!("{COMMENT_SELECT} WHERE c.id = ?"))).bind(id).fetch_one(&st.db).await {
        st.emit(team, Event::CommentAdded(comment_from_row(&row)));
    }
    id
}

/// Lo que entregó quien terminó la etapa: su último comentario desde que empezó (sin el «✅ Terminé.» automático).
async fn delivery(st: &AppState, task: Id, author: Id, since: Id) -> String {
    let b: Option<String> = sqlx::query_scalar("SELECT body FROM comments WHERE task_id = ? AND user_id = ? AND id > ? ORDER BY id DESC LIMIT 1")
        .bind(task)
        .bind(author)
        .bind(since)
        .fetch_optional(&st.db)
        .await
        .ok()
        .flatten();
    let b = b.unwrap_or_default();
    trunc(b.trim_start_matches("✅ Terminé.").trim(), DELIVERY_MAX)
}

fn stage_text(steps: &[Step], idx: usize, header: &str, extra_title: &str, extra: &str) -> String {
    let s = &steps[idx];
    let recorrido = steps.iter().enumerate().map(|(i, x)| if i == idx { format!("**{}**", x.username) } else { x.username.clone() }).collect::<Vec<_>>().join(" → ");
    let mut t = format!("{header}\n🧭 Etapa {} de {} · {} · {}\n", idx + 1, steps.len(), if s.kind == "review" { "revisar" } else { "hacer" }, recorrido);
    if !extra.trim().is_empty() {
        t += &format!("\n{extra_title}:\n{extra}\n");
    }
    if !s.instruction.trim().is_empty() {
        t += &format!("\nInstrucción para {}:\n{}\n", s.username, s.instruction.trim());
    }
    if s.kind == "review" && s.bot {
        t += "\nCuando termines, dá tu veredicto con uno de estos comandos (sin veredicto la tarea queda esperando a una persona):\n  tm-agent verdict ok \"comentario opcional\"\n  tm-agent verdict return \"qué hay que corregir\"\n";
    }
    t
}

/// Pasa la tarea a la etapa `idx`: nuevo responsable, de nuevo «Por hacer», con el comentario que le explica qué se espera.
async fn goto_step(st: &AppState, team: Id, task: Id, actor: Id, steps: &[Step], idx: usize, header: &str, extra_title: &str, extra: &str, round: Option<i64>) -> Result<Task> {
    sqlx::query("UPDATE tasks SET assignee_id = ?, status = 'todo', waiting = 0, remind_state = 0, updated_at = ? WHERE id = ?")
        .bind(steps[idx].assignee_id)
        .bind(Utc::now())
        .bind(task)
        .execute(&st.db)
        .await?;
    let cid = post_comment(st, team, task, actor, stage_text(steps, idx, header, extra_title, extra)).await;
    sqlx::query("UPDATE task_flows SET step = ?, round = COALESCE(?, round), from_comment = ? WHERE task_id = ?")
        .bind(idx as i64)
        .bind(round)
        .bind(cid)
        .bind(task)
        .execute(&st.db)
        .await?;
    let t = routes::fetch_task(st, task).await?;
    st.emit(team, Event::TaskUpdated(t.clone()));
    routes::notify_assigned(st, team, actor, &t).await;
    Ok(t)
}

async fn finish(st: &AppState, team: Id, task: Id, actor: Id, text: String) -> Result<Task> {
    sqlx::query("UPDATE task_flows SET finished = 1 WHERE task_id = ?").bind(task).execute(&st.db).await?;
    sqlx::query("UPDATE tasks SET status = 'done', waiting = 0, updated_at = ? WHERE id = ?").bind(Utc::now()).bind(task).execute(&st.db).await?;
    post_comment(st, team, task, actor, text).await;
    let t = routes::fetch_task(st, task).await?;
    st.emit(team, Event::TaskUpdated(t.clone()));
    if let Some(c) = t.created_by.filter(|c| *c != actor) {
        crate::notify::notify(st, c, "comment", Some(team), Some(task), format!("✅ El flujo de «{}» terminó", t.title)).await;
    }
    Ok(t)
}

/// Pasa a esperar a una persona (zona «Esperando tu próximo paso») y le avisa a quien creó la tarea.
async fn hold_for_person(st: &AppState, team: Id, task: Id, actor: Id, text: String) -> Result<Task> {
    sqlx::query("UPDATE tasks SET status = 'doing', waiting = 1, updated_at = ? WHERE id = ?").bind(Utc::now()).bind(task).execute(&st.db).await?;
    post_comment(st, team, task, actor, text.clone()).await;
    let t = routes::fetch_task(st, task).await?;
    st.emit(team, Event::TaskUpdated(t.clone()));
    if let Some(c) = t.created_by.filter(|c| *c != actor) {
        crate::notify::notify(st, c, "comment", Some(team), Some(task), format!("⏳ «{}»: {}", t.title, trunc(&text, 160))).await;
    }
    Ok(t)
}

/// Llamado cuando alguien marca la tarea como Hecha: si tiene un flujo en marcha y quien la termina es el responsable de la
/// etapa actual (o un admin), en vez de cerrarla la pasa a la etapa siguiente. `None` = no hay flujo que atender, seguir normal.
pub async fn on_done(st: &AppState, team: Id, user: &AuthUser, role: Role, cur: &Task) -> Result<Option<Task>> {
    let Some(fl) = load(st, cur.id).await? else { return Ok(None) };
    let sts = steps(st, cur.id).await?;
    if fl.finished || fl.step as usize >= sts.len() {
        return Ok(None);
    }
    let idx = fl.step as usize;
    if sts[idx].assignee_id != user.id && role < Role::Admin {
        return Ok(None);
    }
    let who = sts[idx].username.clone();
    if sts[idx].kind == "review" {
        let t = hold_for_person(st, team, cur.id, user.id, format!("🧭 {who} terminó la revisión sin dar veredicto: decidí vos con «Aprobar» o «Devolver» en el flujo de la tarea.")).await?;
        return Ok(Some(t));
    }
    if idx + 1 >= sts.len() {
        sqlx::query("UPDATE task_flows SET finished = 1 WHERE task_id = ?").bind(cur.id).execute(&st.db).await?;
        return Ok(None);
    }
    let out = delivery(st, cur.id, sts[idx].assignee_id, fl.from_comment).await;
    let t = goto_step(st, team, cur.id, user.id, &sts, idx + 1, &format!("📤 {who} terminó la etapa {}.", idx + 1), &format!("Entrega de {who}"), &out, None).await?;
    crate::extras::log_task(st, cur.id, user.id, format!("pasó «{}» a la etapa {} ({})", t.title, idx + 2, sts[idx + 1].username)).await;
    Ok(Some(t))
}

#[derive(Deserialize)]
struct Verdict {
    ok: bool,
    #[serde(default)]
    reason: String,
}

async fn verdict(State(st): State<AppState>, user: AuthUser, Path(id): Path<Id>, Json(b): Json<Verdict>) -> Result<Json<Task>> {
    let team = routes::team_of_task(&st, id).await?;
    let role = routes::require(&st, team, user.id, Role::Member).await?;
    let fl = load(&st, id).await?.filter(|f| !f.finished).ok_or_else(|| AppError::bad("la tarea no tiene un flujo en marcha"))?;
    let sts = steps(&st, id).await?;
    let idx = fl.step as usize;
    let s = sts.get(idx).ok_or_else(|| AppError::bad("flujo inválido"))?;
    if s.kind != "review" {
        return Err(AppError::bad("la etapa actual no es de revisión"));
    }
    if s.assignee_id != user.id && role < Role::Admin {
        return Err(AppError::forbidden());
    }
    let reason = trunc(b.reason.trim(), INSTRUCTION_MAX);
    let who = crate::notify::username_of(&st, user.id).await;
    if b.ok {
        if idx + 1 >= sts.len() {
            return Ok(Json(finish(&st, team, id, user.id, format!("✅ {who} aprobó{}. El flujo terminó.", if reason.is_empty() { String::new() } else { format!(": {reason}") })).await?));
        }
        let t = goto_step(&st, team, id, user.id, &sts, idx + 1, &format!("✅ {who} aprobó la etapa {}.", idx + 1), "Comentario del revisor", &reason, None).await?;
        crate::extras::log_task(&st, id, user.id, format!("aprobó «{}» y pasó a la etapa {}", t.title, idx + 2)).await;
        return Ok(Json(t));
    }
    if reason.is_empty() {
        return Err(AppError::bad("para devolver hay que explicar qué corregir"));
    }
    let Some(back) = (0..idx).rev().find(|i| sts[*i].kind == "work") else {
        return Err(AppError::bad("no hay una etapa de trabajo anterior a la que devolver"));
    };
    let round = fl.round + 1;
    if round > fl.max_rounds {
        sqlx::query("UPDATE task_flows SET round = ? WHERE task_id = ?").bind(round).bind(id).execute(&st.db).await?;
        let t = hold_for_person(&st, team, id, user.id, format!("⛔ {who} la devolvió otra vez ({reason}), pero ya se llegó al tope de {} vueltas: decide una persona.", fl.max_rounds)).await?;
        return Ok(Json(t));
    }
    let t = goto_step(&st, team, id, user.id, &sts, back, &format!("↩ {who} la devolvió (vuelta {round} de {}).", fl.max_rounds), &format!("Qué corregir, según {who}"), &reason, Some(round)).await?;
    crate::extras::log_task(&st, id, user.id, format!("devolvió «{}» a la etapa {} (vuelta {round})", t.title, back + 1)).await;
    Ok(Json(t))
}

fn step_json(s: &Step, i: usize) -> Value {
    json!({ "pos": i, "assignee_id": s.assignee_id, "username": s.username, "bot": s.bot, "kind": s.kind, "instruction": s.instruction })
}

async fn get_flow(State(st): State<AppState>, user: AuthUser, Path(id): Path<Id>) -> Result<Json<Value>> {
    routes::ensure_member(&st, routes::team_of_task(&st, id).await?, user.id).await?;
    let Some(fl) = load(&st, id).await? else { return Ok(Json(json!({ "active": false, "steps": [] }))) };
    let sts = steps(&st, id).await?;
    Ok(Json(json!({
        "active": !fl.finished, "finished": fl.finished, "step": fl.step, "round": fl.round, "max_rounds": fl.max_rounds,
        "steps": sts.iter().enumerate().map(|(i, s)| step_json(s, i)).collect::<Vec<_>>(),
    })))
}

#[derive(Deserialize)]
struct StepIn {
    assignee_id: Id,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    instruction: String,
}

#[derive(Deserialize)]
struct FlowIn {
    steps: Vec<StepIn>,
    max_rounds: Option<i64>,
}

/// Define (o redefine) el flujo y lo arranca en la etapa 1: la tarea pasa a su responsable con la instrucción de esa etapa.
async fn put_flow(State(st): State<AppState>, user: AuthUser, Path(id): Path<Id>, Json(b): Json<FlowIn>) -> Result<Json<Task>> {
    let team = routes::team_of_task(&st, id).await?;
    let role = routes::require(&st, team, user.id, Role::Member).await?;
    routes::bot_owns(&st, &user, id).await?;
    if b.steps.len() < 2 || b.steps.len() > MAX_STEPS {
        return Err(AppError::bad("un flujo necesita entre 2 y 8 etapas"));
    }
    let cur = routes::fetch_task(&st, id).await?;
    for (i, s) in b.steps.iter().enumerate() {
        if !matches!(s.kind.as_str(), "work" | "review") {
            return Err(AppError::bad("cada etapa es de trabajo o de revisión"));
        }
        if s.instruction.chars().count() > INSTRUCTION_MAX {
            return Err(AppError::bad("la instrucción de una etapa es demasiado larga"));
        }
        let member: Option<i64> = sqlx::query_scalar("SELECT 1 FROM team_members WHERE team_id = ? AND user_id = ?").bind(team).bind(s.assignee_id).fetch_optional(&st.db).await?;
        if member.is_none() {
            return Err(AppError::bad("cada responsable tiene que ser del equipo"));
        }
        routes::check_assign(&st, team, user.id, role, if i == 0 { cur.assignee_id } else { None }, Some(s.assignee_id), false).await?;
    }
    if b.steps[0].kind != "work" {
        return Err(AppError::bad("la primera etapa tiene que ser de trabajo"));
    }
    let max_rounds = b.max_rounds.unwrap_or(3).clamp(1, 10);
    sqlx::query("DELETE FROM flow_steps WHERE task_id = ?").bind(id).execute(&st.db).await?;
    for (i, s) in b.steps.iter().enumerate() {
        sqlx::query("INSERT INTO flow_steps (task_id, pos, assignee_id, kind, instruction) VALUES (?, ?, ?, ?, ?)")
            .bind(id)
            .bind(i as i64)
            .bind(s.assignee_id)
            .bind(&s.kind)
            .bind(s.instruction.trim())
            .execute(&st.db)
            .await?;
    }
    sqlx::query("INSERT INTO task_flows (task_id, step, round, max_rounds, from_comment, finished) VALUES (?, 0, 0, ?, 0, 0) ON CONFLICT(task_id) DO UPDATE SET step = 0, round = 0, max_rounds = excluded.max_rounds, from_comment = 0, finished = 0")
        .bind(id)
        .bind(max_rounds)
        .execute(&st.db)
        .await?;
    let sts = steps(&st, id).await?;
    let t = goto_step(&st, team, id, user.id, &sts, 0, "🧭 Flujo iniciado.", "", "", Some(0)).await?;
    crate::extras::log_task(&st, id, user.id, format!("armó un flujo de {} etapas en «{}»", sts.len(), t.title)).await;
    Ok(Json(t))
}

async fn delete_flow(State(st): State<AppState>, user: AuthUser, Path(id): Path<Id>) -> Result<StatusCode> {
    let team = routes::team_of_task(&st, id).await?;
    routes::ensure_writer(&st, team, user.id).await?;
    routes::bot_owns(&st, &user, id).await?;
    sqlx::query("DELETE FROM flow_steps WHERE task_id = ?").bind(id).execute(&st.db).await?;
    sqlx::query("DELETE FROM task_flows WHERE task_id = ?").bind(id).execute(&st.db).await?;
    let t = routes::fetch_task(&st, id).await?;
    st.emit(team, Event::TaskUpdated(t));
    Ok(StatusCode::NO_CONTENT)
}

/// Resumen de los flujos de un proyecto para dibujar el recorrido en cada tarjeta.
async fn project_flows(State(st): State<AppState>, user: AuthUser, Path(project): Path<Id>) -> Result<Json<Value>> {
    let team: Id = sqlx::query_scalar("SELECT team_id FROM projects WHERE id = ?").bind(project).fetch_optional(&st.db).await?.ok_or_else(AppError::not_found)?;
    routes::ensure_member(&st, team, user.id).await?;
    let flows = sqlx::query(
        "SELECT f.task_id, f.step, f.round, f.max_rounds, f.finished FROM task_flows f JOIN tasks t ON t.id = f.task_id WHERE t.project_id = ?",
    )
    .bind(project)
    .fetch_all(&st.db)
    .await?;
    let rows = sqlx::query(
        "SELECT s.task_id, s.kind, u.username FROM flow_steps s JOIN users u ON u.id = s.assignee_id JOIN tasks t ON t.id = s.task_id WHERE t.project_id = ? ORDER BY s.task_id, s.pos",
    )
    .bind(project)
    .fetch_all(&st.db)
    .await?;
    let out: Vec<Value> = flows
        .iter()
        .map(|f| {
            let tid: Id = f.get(0);
            let steps: Vec<Value> = rows.iter().filter(|r| r.get::<Id, _>(0) == tid).map(|r| json!({ "kind": r.get::<String, _>(1), "username": r.get::<String, _>(2) })).collect();
            json!({ "task_id": tid, "step": f.get::<i64, _>(1), "round": f.get::<i64, _>(2), "max_rounds": f.get::<i64, _>(3), "finished": f.get::<i64, _>(4) != 0, "steps": steps })
        })
        .collect();
    Ok(Json(json!(out)))
}
