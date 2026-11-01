//! Perfil de uso del equipo («Desarrollo de software», «Contenido y redes», «Fábrica de apps», «Otros / equipo general»)
//! y proyecto de ejemplo. El perfil cambia lo que se ve por defecto (nombres de las columnas, vista inicial, lo técnico);
//! el ejemplo es un proyecto de muestra marcado para poder borrarlo entero con un botón y dejar todo limpio.
use crate::{auth::AuthUser, error::{AppError, Result}, routes, AppState};
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use chrono::{Duration, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use tm_core::{Event, Id, NewProject, NewTask, Role, Status};

const PROFILES: [&str; 5] = ["dev", "content", "factory", "office", "other"];

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/teams/{id}/profile", get(get_profile).put(put_profile))
        .route("/api/teams/{id}/example", post(add_example).delete(remove_example))
}

async fn has_example(st: &AppState, team: Id) -> bool {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM projects WHERE team_id = ? AND is_example = 1").bind(team).fetch_one(&st.db).await.unwrap_or(0) > 0
}

async fn get_profile(State(st): State<AppState>, user: AuthUser, Path(team): Path<Id>) -> Result<Json<Value>> {
    routes::ensure_member(&st, team, user.id).await?;
    let profile: Option<String> = sqlx::query_scalar("SELECT profile FROM team_profile WHERE team_id = ?").bind(team).fetch_optional(&st.db).await?;
    Ok(Json(json!({ "profile": profile, "has_example": has_example(&st, team).await })))
}

#[derive(Deserialize)]
struct ProfileBody {
    profile: String,
}

async fn put_profile(State(st): State<AppState>, user: AuthUser, Path(team): Path<Id>, Json(b): Json<ProfileBody>) -> Result<Json<Value>> {
    user.human_only()?;
    routes::require(&st, team, user.id, Role::Admin).await?;
    if !PROFILES.contains(&b.profile.as_str()) {
        return Err(AppError::bad("perfil inválido"));
    }
    sqlx::query("INSERT INTO team_profile (team_id, profile) VALUES (?, ?) ON CONFLICT(team_id) DO UPDATE SET profile = excluded.profile")
        .bind(team)
        .bind(&b.profile)
        .execute(&st.db)
        .await?;
    Ok(Json(json!({ "profile": b.profile })))
}

struct Ex {
    project: &'static str,
    color: &'static str,
    /// (título, descripción, estado, prioridad, días hasta el vencimiento)
    tasks: &'static [(&'static str, &'static str, Status, i64, Option<i64>)],
}

fn example_for(profile: &str) -> Vec<Ex> {
    use Status::*;
    match profile {
        "content" => vec![Ex {
            project: "Ejemplo · Calendario de contenido",
            color: "#bb9af7",
            tasks: &[
                ("👋 Arrastrame a «En producción»", "Esto es un ejemplo: movelo de columna, abrilo y mirá cómo funciona. Cuando termines, «Eliminar ejemplo» en Configuración deja todo limpio.", Todo, 2, None),
                ("Idea: serie de 5 posts sobre el producto", "Una idea por post, con el gancho de la primera línea.", Todo, 1, Some(3)),
                ("Redactar el post de lanzamiento (Instagram y X)", "Asignalo a una IA redactora: queda «en revisión» hasta que alguien lo apruebe.", Doing, 2, Some(2)),
                ("Traducir el post al inglés y al portugués", "Mismo tono de marca en los tres idiomas.", Todo, 1, Some(4)),
                ("Diseñar la imagen del post", "Medidas para feed y stories.", Todo, 1, Some(5)),
                ("Publicar el post aprobado", "Se programa la fecha y se comenta el enlace acá.", Done, 1, None),
            ],
        }],
        "factory" => vec![
            Ex {
                project: "Ejemplo · App 1: Lista de compras",
                color: "#7aa2f7",
                tasks: &[
                    ("👋 Arrastrame a «En construcción»", "Esto es un ejemplo de una app de la fábrica. «Eliminar ejemplo» en Configuración lo borra todo.", Todo, 2, None),
                    ("Definir el alcance del MVP", "Qué entra y qué no en la primera versión.", Done, 2, None),
                    ("Diseñar las pantallas", "", Doing, 1, Some(3)),
                    ("Programar el MVP (asignalo a una IA)", "", Todo, 2, Some(7)),
                    ("Probar y publicar", "", Todo, 1, Some(10)),
                ],
            },
            Ex {
                project: "Ejemplo · App 2: Agenda de turnos",
                color: "#9ece6a",
                tasks: &[
                    ("Definir el alcance del MVP", "", Todo, 2, Some(2)),
                    ("Diseñar las pantallas", "", Todo, 1, Some(6)),
                    ("Programar el MVP", "", Todo, 2, Some(14)),
                ],
            },
        ],
        "office" => vec![Ex {
            project: "Ejemplo · Expedientes y trámites",
            color: "#2ac3de",
            tasks: &[
                ("👋 Arrastrame a «En trámite»", "Esto es un ejemplo. «Eliminar ejemplo» en Configuración lo borra y deja todo limpio.", Todo, 2, None),
                ("Revisar el contrato de alquiler del cliente", "Cláusulas de ajuste y rescisión. Dejar observaciones en comentarios.", Todo, 2, Some(2)),
                ("Presentar la declaración de IVA del mes", "Reunir facturas y conciliar antes de presentar.", Doing, 2, Some(3)),
                ("Preparar el escrito para la audiencia", "Borrador para revisión del titular del estudio.", Doing, 1, Some(5)),
                ("Conciliar el banco con el libro mayor", "", Todo, 1, Some(7)),
                ("Archivar el expediente cerrado", "", Done, 1, None),
            ],
        }],
        "other" => vec![Ex {
            project: "Ejemplo · Mi equipo",
            color: "#e0af68",
            tasks: &[
                ("👋 Arrastrame a «En curso»", "Esto es un ejemplo. Cuando quieras empezar de cero, «Eliminar ejemplo» en Configuración.", Todo, 2, None),
                ("Armar la lista de cosas pendientes", "", Todo, 1, Some(2)),
                ("Repartir las tareas entre el equipo", "Asignalas desde la tarea: a personas o a IAs.", Doing, 1, Some(3)),
                ("Revisar el avance de la semana", "", Done, 1, None),
            ],
        }],
        _ => vec![Ex {
            project: "Ejemplo · Mi primera app",
            color: "#7aa2f7",
            tasks: &[
                ("👋 Arrastrame a «En curso»", "Esto es un ejemplo: movelo, abrilo y probá una IA. «Eliminar ejemplo» en Configuración deja todo limpio.", Todo, 2, None),
                ("Definir qué problema resuelve la app", "", Todo, 1, Some(2)),
                ("Crear el repositorio y la estructura", "", Doing, 2, Some(3)),
                ("Primer prototipo (asignalo a una IA)", "Indicá la carpeta del proyecto en «⚙ Proyecto» para que la IA trabaje sobre el código.", Todo, 2, Some(7)),
                ("Instalar CrewHelm", "", Done, 1, None),
            ],
        }],
    }
}

async fn add_example(State(st): State<AppState>, user: AuthUser, Path(team): Path<Id>, Json(b): Json<ProfileBody>) -> Result<Json<Value>> {
    user.human_only()?;
    routes::require(&st, team, user.id, Role::Member).await?;
    if !PROFILES.contains(&b.profile.as_str()) {
        return Err(AppError::bad("perfil inválido"));
    }
    if has_example(&st, team).await {
        return Err(AppError::bad("ya hay un ejemplo cargado: eliminalo primero"));
    }
    let mut created = Vec::new();
    for ex in example_for(&b.profile) {
        let Json(p) = routes::create_project(
            State(st.clone()),
            AuthUser { id: user.id, bot: false },
            Path(team),
            Json(NewProject { name: ex.project.into(), description: String::new(), color: Some(ex.color.into()), work_dir: String::new(), repo: String::new() }),
        )
        .await?;
        sqlx::query("UPDATE projects SET is_example = 1 WHERE id = ?").bind(p.id).execute(&st.db).await?;
        for (title, desc, status, prio, days) in ex.tasks {
            let Json(t) = routes::create_task(
                State(st.clone()),
                AuthUser { id: user.id, bot: false },
                Path(p.id),
                Json(NewTask { title: (*title).into(), description: (*desc).into(), priority: *prio, due: days.map(|d| Utc::now() + Duration::days(d)), ..Default::default() }),
            )
            .await?;
            if *status != Status::Todo {
                sqlx::query("UPDATE tasks SET status = ? WHERE id = ?").bind(status.as_str()).bind(t.id).execute(&st.db).await?;
                if let Ok(t) = routes::fetch_task(&st, t.id).await {
                    st.emit(team, Event::TaskUpdated(t));
                }
            }
        }
        created.push(p.id);
    }
    Ok(Json(json!({ "projects": created })))
}

/// Borra todos los proyectos de ejemplo del equipo (con sus tareas).
async fn remove_example(State(st): State<AppState>, user: AuthUser, Path(team): Path<Id>) -> Result<Json<Value>> {
    user.human_only()?;
    routes::require(&st, team, user.id, Role::Admin).await?;
    let ids: Vec<Id> = sqlx::query_scalar("SELECT id FROM projects WHERE team_id = ? AND is_example = 1").bind(team).fetch_all(&st.db).await?;
    for id in &ids {
        sqlx::query("DELETE FROM projects WHERE id = ?").bind(id).execute(&st.db).await?;
        st.emit(team, Event::ProjectDeleted { id: *id, team_id: team });
    }
    Ok(Json(json!({ "deleted": ids.len() })))
}
