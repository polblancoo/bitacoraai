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
        .route("/api/teams/{id}/flow-templates", get(templates).post(template_save))
        .route("/api/teams/{id}/flow-templates/{tid}", axum::routing::delete(template_delete))
}

const MAX_TEMPLATES: i64 = 50;
const MAX_GROUP: usize = 5;

/// Reglas de las etapas en paralelo: solo de trabajo, la primera nunca lo es, grupos de hasta 5 y siempre tiene que haber una etapa después del grupo.
fn validate_par(steps: &[(&str, bool)]) -> Result<()> {
    if steps.first().is_some_and(|s| s.1) {
        return Err(AppError::bad("la primera etapa no puede ir en paralelo con otra anterior"));
    }
    if steps.last().is_some_and(|s| s.1) {
        return Err(AppError::bad("después de un grupo en paralelo tiene que haber una etapa más (por ejemplo una revisión)"));
    }
    let mut run = 1;
    for (i, (kind, par)) in steps.iter().enumerate() {
        if *par {
            if *kind != "work" || steps[i - 1].0 != "work" {
                return Err(AppError::bad("solo las etapas de trabajo pueden ir en paralelo"));
            }
            run += 1;
            if run > MAX_GROUP {
                return Err(AppError::bad("un grupo en paralelo admite hasta 5 integrantes"));
            }
        } else {
            run = 1;
        }
    }
    Ok(())
}

async fn templates(State(st): State<AppState>, user: AuthUser, Path(team): Path<Id>) -> Result<Json<Value>> {
    routes::ensure_member(&st, team, user.id).await?;
    let ts = sqlx::query("SELECT t.id, t.name, t.max_rounds, u.username FROM flow_templates t JOIN users u ON u.id = t.created_by WHERE t.team_id = ? ORDER BY t.name").bind(team).fetch_all(&st.db).await?;
    let mut out = vec![];
    for t in &ts {
        let id: Id = t.get(0);
        let rows = sqlx::query("SELECT s.assignee_id, s.kind, s.instruction, m.user_id IS NOT NULL, s.par FROM flow_template_steps s LEFT JOIN team_members m ON m.team_id = ? AND m.user_id = s.assignee_id WHERE s.template_id = ? ORDER BY s.pos").bind(team).bind(id).fetch_all(&st.db).await?;
        out.push(json!({
            "id": id, "name": t.get::<String, _>(1), "max_rounds": t.get::<i64, _>(2), "by": t.get::<String, _>(3),
            // si la persona ya no es del equipo, la etapa queda por elegir
            "steps": rows.iter().map(|r| json!({ "assignee_id": if r.get::<bool, _>(3) { r.get::<Option<Id>, _>(0) } else { None }, "kind": r.get::<String, _>(1), "instruction": r.get::<String, _>(2), "par": r.get::<i64, _>(4) != 0 })).collect::<Vec<_>>(),
        }));
    }
    Ok(Json(json!(out)))
}

#[derive(Deserialize)]
struct TplStep {
    #[serde(default)]
    assignee_id: Option<Id>,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    instruction: String,
    #[serde(default)]
    par: bool,
}
#[derive(Deserialize)]
struct TplIn {
    name: String,
    steps: Vec<TplStep>,
    max_rounds: Option<i64>,
}

async fn template_save(State(st): State<AppState>, user: AuthUser, Path(team): Path<Id>, Json(b): Json<TplIn>) -> Result<Json<Value>> {
    user.human_only()?;
    routes::require(&st, team, user.id, Role::Member).await?;
    let name = b.name.trim();
    if name.is_empty() || name.chars().count() > 60 || name.chars().any(|c| c.is_control()) {
        return Err(AppError::bad("poné un nombre de hasta 60 caracteres"));
    }
    if b.steps.len() < 2 || b.steps.len() > MAX_STEPS || b.steps[0].kind != "work" {
        return Err(AppError::bad("una plantilla necesita entre 2 y 8 etapas y la primera de trabajo"));
    }
    validate_par(&b.steps.iter().map(|s| (s.kind.as_str(), s.par)).collect::<Vec<_>>())?;
    for s in &b.steps {
        if !matches!(s.kind.as_str(), "work" | "review") || s.instruction.chars().count() > INSTRUCTION_MAX {
            return Err(AppError::bad("etapa no válida"));
        }
        if let Some(a) = s.assignee_id {
            let m: Option<i64> = sqlx::query_scalar("SELECT 1 FROM team_members WHERE team_id = ? AND user_id = ?").bind(team).bind(a).fetch_optional(&st.db).await?;
            m.ok_or_else(|| AppError::bad("cada responsable tiene que ser del equipo"))?;
        }
    }
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM flow_templates WHERE team_id = ?").bind(team).fetch_one(&st.db).await?;
    let dup: Option<Id> = sqlx::query_scalar("SELECT id FROM flow_templates WHERE team_id = ? AND name = ? COLLATE NOCASE").bind(team).bind(name).fetch_optional(&st.db).await?;
    if dup.is_none() && n >= MAX_TEMPLATES {
        return Err(AppError::bad("máximo 50 plantillas por equipo"));
    }
    let mut tx = st.db.begin().await?;
    let id = match dup {
        Some(id) => {
            sqlx::query("UPDATE flow_templates SET max_rounds = ? WHERE id = ?").bind(b.max_rounds.unwrap_or(3).clamp(1, 10)).bind(id).execute(&mut *tx).await?;
            sqlx::query("DELETE FROM flow_template_steps WHERE template_id = ?").bind(id).execute(&mut *tx).await?;
            id
        }
        None => sqlx::query("INSERT INTO flow_templates (team_id, name, max_rounds, created_by) VALUES (?, ?, ?, ?)").bind(team).bind(name).bind(b.max_rounds.unwrap_or(3).clamp(1, 10)).bind(user.id).execute(&mut *tx).await?.last_insert_rowid(),
    };
    for (i, s) in b.steps.iter().enumerate() {
        sqlx::query("INSERT INTO flow_template_steps (template_id, pos, assignee_id, kind, instruction, par) VALUES (?, ?, ?, ?, ?, ?)").bind(id).bind(i as i64).bind(s.assignee_id).bind(&s.kind).bind(s.instruction.trim()).bind(s.par).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    templates(State(st), user, Path(team)).await
}

async fn template_delete(State(st): State<AppState>, user: AuthUser, Path((team, tid)): Path<(Id, Id)>) -> Result<Json<Value>> {
    user.human_only()?;
    let role = routes::require(&st, team, user.id, Role::Member).await?;
    let r = sqlx::query("DELETE FROM flow_templates WHERE id = ? AND team_id = ? AND (? OR created_by = ?)").bind(tid).bind(team).bind(role >= Role::Admin).bind(user.id).execute(&st.db).await?;
    if r.rows_affected() == 0 {
        return Err(AppError::forbidden());
    }
    templates(State(st), user, Path(team)).await
}

struct Step {
    assignee_id: Id,
    username: String,
    bot: bool,
    kind: String,
    instruction: String,
    /// Trabaja a la vez que la etapa anterior (forman un grupo en paralelo).
    par: bool,
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
        "SELECT s.assignee_id, u.username, s.kind, s.instruction, EXISTS(SELECT 1 FROM api_tokens WHERE user_id = s.assignee_id) AS bot, s.par \
         FROM flow_steps s JOIN users u ON u.id = s.assignee_id WHERE s.task_id = ? ORDER BY s.pos",
    )
    .bind(task)
    .fetch_all(&st.db)
    .await?;
    Ok(rows.iter().map(|r| Step { assignee_id: r.get(0), username: r.get(1), kind: r.get(2), instruction: r.get(3), bot: r.get::<i64, _>(4) != 0, par: r.get::<i64, _>(5) != 0 }).collect())
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
    if steps.get(idx + 1).is_some_and(|s| s.par) {
        return launch_group(st, team, task, actor, steps, idx, header, extra_title, extra, round).await;
    }
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

/// Última posición del grupo en paralelo que empieza en `start`.
fn group_end(steps: &[Step], start: usize) -> usize {
    let mut e = start;
    while e + 1 < steps.len() && steps[e + 1].par {
        e += 1;
    }
    e
}

/// Primera posición del grupo en paralelo al que pertenece `idx`.
fn group_start(steps: &[Step], idx: usize) -> usize {
    let mut s = idx;
    while s > 0 && steps[s].par {
        s -= 1;
    }
    s
}

/// Etapa en paralelo: cada integrante recibe una subtarea y la tarea principal espera a que terminen todas.
async fn launch_group(st: &AppState, team: Id, task: Id, actor: Id, steps: &[Step], start: usize, header: &str, extra_title: &str, extra: &str, round: Option<i64>) -> Result<Task> {
    let end = group_end(steps, start);
    let parent = routes::fetch_task(st, task).await?;
    sqlx::query("DELETE FROM flow_par WHERE task_id = ?").bind(task).execute(&st.db).await?;
    let names = steps[start..=end].iter().map(|s| s.username.clone()).collect::<Vec<_>>().join(", ");
    let mut head = format!("{header}\n🧭 Etapas {}–{} de {} · en paralelo: {names}\n\nCada integrante recibe una subtarea; la etapa sigue sola cuando terminan todas.", start + 1, end + 1, steps.len());
    if !extra.trim().is_empty() {
        head += &format!("\n\n{extra_title}:\n{extra}");
    }
    let cid = post_comment(st, team, task, actor, head).await;
    for (i, s) in steps.iter().enumerate().take(end + 1).skip(start) {
        let others = steps[start..=end].iter().enumerate().filter(|(j, _)| start + j != i).map(|(_, x)| x.username.clone()).collect::<Vec<_>>().join(", ");
        let mut desc = format!("Parte de la tarea «{}» (tarea {task}), en paralelo con: {others}.\n", parent.title);
        if !s.instruction.trim().is_empty() {
            desc += &format!("\nInstrucción:\n{}\n", s.instruction.trim());
        }
        if !extra.trim().is_empty() {
            desc += &format!("\n{extra_title}:\n{extra}\n");
        }
        let now = Utc::now();
        let child = sqlx::query(
            "INSERT INTO tasks (project_id, title, description, priority, assignee_id, parent_id, created_at, updated_at, created_by, position) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, (SELECT COALESCE(MAX(position), 0) + 1 FROM tasks WHERE project_id = ?))",
        )
        .bind(parent.project_id)
        .bind(trunc(&format!("{} · {}", parent.title, s.username), 120))
        .bind(trunc(&desc, 6000))
        .bind(parent.priority)
        .bind(s.assignee_id)
        .bind(task)
        .bind(now)
        .bind(now)
        .bind(parent.created_by.unwrap_or(actor))
        .bind(parent.project_id)
        .execute(&st.db)
        .await?
        .last_insert_rowid();
        sqlx::query("INSERT INTO flow_par (task_id, child_id, step_pos) VALUES (?, ?, ?)").bind(task).bind(child).bind(i as i64).execute(&st.db).await?;
        let ct = routes::fetch_task(st, child).await?;
        st.emit(team, Event::TaskCreated(ct.clone()));
        routes::notify_assigned(st, team, actor, &ct).await;
    }
    sqlx::query("UPDATE tasks SET status = 'doing', waiting = 1, remind_state = 0, updated_at = ? WHERE id = ?").bind(Utc::now()).bind(task).execute(&st.db).await?;
    sqlx::query("UPDATE task_flows SET step = ?, round = COALESCE(?, round), from_comment = ? WHERE task_id = ?").bind(start as i64).bind(round).bind(cid).bind(task).execute(&st.db).await?;
    let t = routes::fetch_task(st, task).await?;
    st.emit(team, Event::TaskUpdated(t.clone()));
    Ok(t)
}

/// Una subtarea de una etapa en paralelo terminó: cuando terminan todas, la tarea principal pasa a la etapa siguiente con las entregas juntas.
pub async fn on_child_done(st: &AppState, team: Id, user: &AuthUser, child: &Task) -> Result<()> {
    let Some(parent): Option<Id> = sqlx::query_scalar("SELECT task_id FROM flow_par WHERE child_id = ?").bind(child.id).fetch_optional(&st.db).await? else { return Ok(()) };
    sqlx::query("UPDATE flow_par SET done = 1 WHERE child_id = ?").bind(child.id).execute(&st.db).await?;
    let Some(fl) = load(st, parent).await?.filter(|f| !f.finished) else { return Ok(()) };
    // solo cuenta mientras la tarea siga en esa etapa (una subtarea reabierta y cerrada después no la hace avanzar de nuevo)
    let first: Option<i64> = sqlx::query_scalar("SELECT MIN(step_pos) FROM flow_par WHERE task_id = ?").bind(parent).fetch_one(&st.db).await?;
    if first != Some(fl.step) {
        return Ok(());
    }
    let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM flow_par WHERE task_id = ? AND done = 0").bind(parent).fetch_one(&st.db).await?;
    let who = crate::notify::username_of(st, user.id).await;
    if left > 0 {
        post_comment(st, team, parent, user.id, format!("✔ {who} terminó su parte. Faltan {left}.")).await;
        return Ok(());
    }
    let sts = steps(st, parent).await?;
    let rows = sqlx::query("SELECT child_id, step_pos FROM flow_par WHERE task_id = ? ORDER BY step_pos").bind(parent).fetch_all(&st.db).await?;
    let mut out = String::new();
    let mut last = 0usize;
    for r in &rows {
        let (cid, pos): (Id, i64) = (r.get(0), r.get(1));
        last = last.max(pos as usize);
        let Some(s) = sts.get(pos as usize) else { continue };
        let d = delivery(st, cid, s.assignee_id, 0).await;
        out += &format!("**{}**:\n{}\n\n", s.username, if d.is_empty() { "(sin comentario)".to_string() } else { d });
    }
    if last + 1 >= sts.len() {
        hold_for_person(st, team, parent, user.id, "🧭 Terminaron todas las partes en paralelo.".into()).await?;
        return Ok(());
    }
    let out = trunc(out.trim(), DELIVERY_MAX * 2);
    goto_step(st, team, parent, user.id, &sts, last + 1, "📤 Terminó la etapa en paralelo.", "Entregas del grupo", &out, None).await?;
    crate::extras::log_task(st, parent, user.id, format!("terminó una etapa en paralelo de «{}»", child.title)).await;
    Ok(())
}

pub(crate) async fn finish(st: &AppState, team: Id, task: Id, actor: Id, text: String) -> Result<Task> {
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

/// Nombre de carpeta seguro para un usuario (sin barras, puntos dobles ni espacios).
pub(crate) fn safe_name(n: &str) -> String {
    let s: String = n.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    if s.is_empty() { "usuario".into() } else { s }
}

fn copy_tree(from: &std::path::Path, to: &std::path::Path) -> std::io::Result<usize> {
    std::fs::create_dir_all(to)?;
    let mut n = 0;
    for e in std::fs::read_dir(from)? {
        let e = e?;
        let (src, dst) = (e.path(), to.join(e.file_name()));
        if e.file_type()?.is_dir() { n += copy_tree(&src, &dst)?; } else { std::fs::copy(&src, &dst)?; n += 1; }
    }
    Ok(n)
}

/// Al terminar el flujo: lo que dejó la última etapa de trabajo en `.bitacora/<usuario>/<id>/` se copia a `final/`.
/// Solo funciona si la carpeta del proyecto está en la máquina del servidor.
pub(crate) async fn publish_final(st: &AppState, team: Id, task: Id, actor: Id, author: &str) {
    let wd: Option<String> = sqlx::query_scalar("SELECT p.work_dir FROM projects p JOIN tasks t ON t.project_id = p.id WHERE t.id = ?").bind(task).fetch_optional(&st.db).await.ok().flatten();
    let Some(wd) = wd.filter(|w| !w.trim().is_empty()) else { return };
    let crew = crate::taskfiles::data_dir(std::path::Path::new(&wd));
    let from = crew.join(safe_name(author)).join(task.to_string());
    if !from.is_dir() { return; }
    let to = crew.join("_tareas").join(task.to_string()).join("final");
    let r = tokio::task::spawn_blocking(move || copy_tree(&from, &to)).await;
    if let Ok(Ok(n)) = r {
        if n > 0 { post_comment(st, team, task, actor, format!("📦 Entrega final: {n} archivo(s) de {author} copiados a `.bitacora/_tareas/{task}/final/`.")).await; }
    }
}

/// Entrega final de una etapa en paralelo: lo de cada integrante se copia a `final/<usuario>/`.
async fn publish_group(st: &AppState, team: Id, task: Id, actor: Id) {
    let wd: Option<String> = sqlx::query_scalar("SELECT p.work_dir FROM projects p JOIN tasks t ON t.project_id = p.id WHERE t.id = ?").bind(task).fetch_optional(&st.db).await.ok().flatten();
    let Some(wd) = wd.filter(|w| !w.trim().is_empty()) else { return };
    let crew = crate::taskfiles::data_dir(std::path::Path::new(&wd));
    let rows = sqlx::query("SELECT f.child_id, u.username FROM flow_par f JOIN tasks c ON c.id = f.child_id JOIN users u ON u.id = c.assignee_id WHERE f.task_id = ?").bind(task).fetch_all(&st.db).await.unwrap_or_default();
    let mut total = 0;
    for r in rows {
        let (cid, name): (Id, String) = (r.get(0), r.get(1));
        let from = crew.join(safe_name(&name)).join(cid.to_string());
        if !from.is_dir() { continue; }
        let to = crew.join("_tareas").join(task.to_string()).join("final").join(safe_name(&name));
        if let Ok(Ok(n)) = tokio::task::spawn_blocking(move || copy_tree(&from, &to)).await { total += n; }
    }
    if total > 0 { post_comment(st, team, task, actor, format!("📦 Entrega final: {total} archivo(s) del grupo copiados a `.bitacora/_tareas/{task}/final/`.")).await; }
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
    if sts.get(idx + 1).is_some_and(|s| s.par) {
        return Err(AppError::bad("la etapa en paralelo sigue en curso: se cierra sola cuando terminan todas las subtareas"));
    }
    if sts[idx].assignee_id != user.id && role < Role::Admin {
        return Ok(None);
    }
    let who = sts[idx].username.clone();
    if sts[idx].kind == "review" {
        let t = hold_for_person(st, team, cur.id, user.id, format!("🧭 {who} terminó la revisión sin dar veredicto: decidí vos con «Aprobar» o «Devolver» en el flujo de la tarea.")).await?;
        return Ok(Some(t));
    }
    if idx + 1 >= sts.len() {
        // última etapa de trabajo: decide quien creó la tarea (si es otra persona, queda esperando su aprobación)
        if cur.created_by.is_some_and(|c| c != user.id) {
            let t = hold_for_person(st, team, cur.id, user.id, format!("🧭 {who} terminó la última etapa: queda esperando que quien creó la tarea la apruebe («✅ Aprobar y publicar») o la devuelva con comentarios.")).await?;
            return Ok(Some(t));
        }
        sqlx::query("UPDATE task_flows SET finished = 1 WHERE task_id = ?").bind(cur.id).execute(&st.db).await?;
        publish_final(st, team, cur.id, user.id, &who).await;
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
    let creator = routes::fetch_task(&st, id).await.ok().and_then(|t| t.created_by);
    if s.assignee_id != user.id && creator != Some(user.id) && role < Role::Admin {
        return Err(AppError::forbidden());
    }
    let reason = trunc(b.reason.trim(), INSTRUCTION_MAX);
    let who = crate::notify::username_of(&st, user.id).await;
    if b.ok {
        if idx + 1 >= sts.len() {
            if let Some(w) = (0..idx).rev().find(|i| sts[*i].kind == "work") {
                if sts[w].par || sts.get(w + 1).is_some_and(|s| s.par) { publish_group(&st, team, id, user.id).await; } else { publish_final(&st, team, id, user.id, &sts[w].username).await; }
            }
            return Ok(Json(finish(&st, team, id, user.id, format!("✅ {who} aprobó{}. El flujo terminó.", if reason.is_empty() { String::new() } else { format!(": {reason}") })).await?));
        }
        let t = goto_step(&st, team, id, user.id, &sts, idx + 1, &format!("✅ {who} aprobó la etapa {}.", idx + 1), "Comentario del revisor", &reason, None).await?;
        crate::extras::log_task(&st, id, user.id, format!("aprobó «{}» y pasó a la etapa {}", t.title, idx + 2)).await;
        return Ok(Json(t));
    }
    if reason.is_empty() {
        return Err(AppError::bad("para devolver hay que explicar qué corregir"));
    }
    let Some(back) = (0..idx).rev().find(|i| sts[*i].kind == "work").map(|b| group_start(&sts, b)) else {
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
    json!({ "pos": i, "assignee_id": s.assignee_id, "username": s.username, "bot": s.bot, "kind": s.kind, "instruction": s.instruction, "par": s.par })
}

async fn get_flow(State(st): State<AppState>, user: AuthUser, Path(id): Path<Id>) -> Result<Json<Value>> {
    routes::ensure_member(&st, routes::team_of_task(&st, id).await?, user.id).await?;
    let Some(fl) = load(&st, id).await? else { return Ok(Json(json!({ "active": false, "steps": [] }))) };
    let sts = steps(&st, id).await?;
    let parts: Vec<Value> = sqlx::query("SELECT f.step_pos, f.child_id, f.done FROM flow_par f WHERE f.task_id = ? ORDER BY f.step_pos").bind(id).fetch_all(&st.db).await?
        .iter().map(|r| json!({ "pos": r.get::<i64, _>(0), "task_id": r.get::<Id, _>(1), "done": r.get::<i64, _>(2) != 0 })).collect();
    Ok(Json(json!({
        "active": !fl.finished, "finished": fl.finished, "step": fl.step, "round": fl.round, "max_rounds": fl.max_rounds,
        "steps": sts.iter().enumerate().map(|(i, s)| step_json(s, i)).collect::<Vec<_>>(),
        "parts": parts,
    })))
}

#[derive(Deserialize)]
struct StepIn {
    assignee_id: Id,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    instruction: String,
    #[serde(default)]
    par: bool,
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
    validate_par(&b.steps.iter().map(|s| (s.kind.as_str(), s.par)).collect::<Vec<_>>())?;
    if b.steps.iter().any(|s| s.par) && cur.parent_id.is_some() {
        return Err(AppError::bad("una subtarea no puede tener etapas en paralelo"));
    }
    let max_rounds = b.max_rounds.unwrap_or(3).clamp(1, 10);
    sqlx::query("DELETE FROM flow_steps WHERE task_id = ?").bind(id).execute(&st.db).await?;
    for (i, s) in b.steps.iter().enumerate() {
        sqlx::query("INSERT INTO flow_steps (task_id, pos, assignee_id, kind, instruction, par) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(id)
            .bind(i as i64)
            .bind(s.assignee_id)
            .bind(&s.kind)
            .bind(s.instruction.trim())
            .bind(s.par)
            .execute(&st.db)
            .await?;
    }
    sqlx::query("DELETE FROM flow_par WHERE task_id = ?").bind(id).execute(&st.db).await?;
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
    sqlx::query("DELETE FROM flow_par WHERE task_id = ?").bind(id).execute(&st.db).await?;
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

/// ¿La tarea tiene un flujo todavía en marcha?
pub(crate) async fn has_active(st: &AppState, task: Id) -> bool {
    load(st, task).await.ok().flatten().is_some_and(|f| !f.finished)
}

/// Flujo parado en su última etapa de trabajo (esperando que quien creó la tarea apruebe): ¿en qué etapa de trabajo y de quién?
pub(crate) async fn last_work_author(st: &AppState, task: Id) -> Option<String> {
    let fl = load(st, task).await.ok().flatten()?;
    let sts = steps(st, task).await.ok()?;
    let idx = (fl.step as usize).min(sts.len().checked_sub(1)?);
    (0..=idx).rev().find(|i| sts[*i].kind == "work").map(|i| sts[i].username.clone())
}

pub(crate) async fn on_last_stage(st: &AppState, task: Id) -> bool {
    let (Some(fl), Ok(sts)) = (load(st, task).await.ok().flatten(), steps(st, task).await) else { return false };
    !sts.is_empty() && fl.step as usize + 1 >= sts.len() && sts[fl.step as usize].kind == "work"
}
