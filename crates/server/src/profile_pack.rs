//! Perfil de uso del equipo («Desarrollo de software», «Contenido y redes», «Fábrica de apps», «Oficina», «Otros»).
//! El perfil cambia lo que se ve por defecto (nombres de las columnas, vista inicial, lo técnico).
//! El ejemplo es un EQUIPO aparte («Ejemplo · …», marcado `is_example`) con proyectos de muestra: así el equipo real
//! nunca se toca, se vuelve a él con un clic y el ejemplo se borra entero con un botón.
use crate::{auth::AuthUser, error::{AppError, Result}, routes, AppState};
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use chrono::{Duration, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use tm_core::{Id, NewProject, NewTask, NewTeam, Role, Status};

const PROFILES: [&str; 6] = ["dev", "content", "design", "factory", "office", "other"];

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/teams/{id}/profile", get(get_profile).put(put_profile))
        .route("/api/examples", post(add_example))
        .route("/api/examples/{id}", axum::routing::delete(remove_example))
}

async fn is_example(st: &AppState, team: Id) -> bool {
    sqlx::query_scalar::<_, i64>("SELECT is_example FROM teams WHERE id = ?").bind(team).fetch_optional(&st.db).await.ok().flatten().unwrap_or(0) != 0
}

fn profile_name(profile: &str) -> &'static str {
    match profile {
        "content" => "Contenido y redes",
        "design" => "Diseño y audiovisual",
        "factory" => "Fábrica de apps",
        "office" => "Oficina y estudios",
        "other" => "Equipo general",
        _ => "Desarrollo de software",
    }
}

async fn get_profile(State(st): State<AppState>, user: AuthUser, Path(team): Path<Id>) -> Result<Json<Value>> {
    routes::ensure_member(&st, team, user.id).await?;
    let profile: Option<String> = sqlx::query_scalar("SELECT profile FROM team_profile WHERE team_id = ?").bind(team).fetch_optional(&st.db).await?;
    Ok(Json(json!({ "profile": profile, "is_example": is_example(&st, team).await })))
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
        "design" => vec![Ex {
            project: "Ejemplo · Campaña de lanzamiento",
            color: "#f7768e",
            tasks: &[
                ("👋 Arrastrame a «En producción»", "Esto es un ejemplo. Guardá tus archivos desde tus programas en la carpeta de red y mirá la carpeta de cada pieza. «Eliminar ejemplo» en Configuración lo borra todo.", Todo, 2, None),
                ("Brief de la campaña", "Objetivo, público, tono y formatos. Subí el brief a tu carpeta de la tarea.", Done, 2, None),
                ("Logo y paleta de colores", "Subí las versiones desde Illustrator o Figma; cada vez que guardes queda la versión anterior.", Doing, 2, Some(3)),
                ("Edición del video de 30 segundos", "Entregá el export final y el enlace al proyecto en la nube (Premiere, DaVinci).", Todo, 2, Some(6)),
                ("Banners para redes (feed y stories)", "Medidas 1080×1080 y 1080×1920.", Todo, 1, Some(5)),
                ("Revisión y aprobación del cliente", "Quien creó la tarea revisa y aprueba: lo aprobado pasa a final/.", Todo, 1, Some(8)),
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

/// Crea un equipo de ejemplo (de la persona que lo pide) con los proyectos de muestra del perfil.
async fn add_example(State(st): State<AppState>, user: AuthUser, Json(b): Json<ProfileBody>) -> Result<Json<Value>> {
    user.human_only()?;
    if !PROFILES.contains(&b.profile.as_str()) {
        return Err(AppError::bad("perfil inválido"));
    }
    let name = format!("Ejemplo · {}", profile_name(&b.profile));
    let dup: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM teams WHERE owner_id = ? AND is_example = 1 AND name = ?").bind(user.id).bind(&name).fetch_one(&st.db).await?;
    if dup > 0 {
        return Err(AppError::bad("ya tenés un equipo de ejemplo de ese perfil: abrilo o eliminalo primero"));
    }
    let Json(team) = routes::create_team(State(st.clone()), AuthUser { id: user.id, bot: false }, Json(NewTeam { name })).await?;
    sqlx::query("UPDATE teams SET is_example = 1 WHERE id = ?").bind(team.id).execute(&st.db).await?;
    sqlx::query("INSERT INTO team_profile (team_id, profile) VALUES (?, ?)").bind(team.id).bind(&b.profile).execute(&st.db).await?;
    for ex in example_for(&b.profile) {
        let Json(p) = routes::create_project(
            State(st.clone()),
            AuthUser { id: user.id, bot: false },
            Path(team.id),
            Json(NewProject { name: ex.project.into(), description: String::new(), color: Some(ex.color.into()), work_dir: String::new(), repo: String::new() }),
        )
        .await?;
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
            }
        }
    }
    Ok(Json(json!({ "team": team })))
}

/// Borra un equipo de ejemplo entero (proyectos y tareas incluidos). Solo si está marcado como ejemplo y la persona es su dueña.
async fn remove_example(State(st): State<AppState>, user: AuthUser, Path(team): Path<Id>) -> Result<Json<Value>> {
    user.human_only()?;
    routes::require(&st, team, user.id, Role::Owner).await?;
    if !is_example(&st, team).await {
        return Err(AppError::bad("este equipo no es un ejemplo: para borrarlo usá «Eliminar equipo»"));
    }
    sqlx::query("DELETE FROM teams WHERE id = ?").bind(team).execute(&st.db).await?;
    Ok(Json(json!({ "deleted": team })))
}
