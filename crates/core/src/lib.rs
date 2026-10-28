//! Modelos y contratos de API compartidos entre servidor, TUI y cliente Tauri.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub type Id = i64;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct User {
    pub id: Id,
    pub username: String,
}

/// Rol dentro de un equipo, de menor a mayor privilegio.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Solo lectura.
    Viewer,
    /// Crea y edita tareas, comentarios y etiquetas.
    Member,
    /// Además gestiona miembros e invitaciones.
    Admin,
    Owner,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Viewer => "viewer",
            Role::Member => "member",
            Role::Admin => "admin",
            Role::Owner => "owner",
        }
    }

    pub fn parse(s: &str) -> Option<Role> {
        [Role::Viewer, Role::Member, Role::Admin, Role::Owner].into_iter().find(|r| r.as_str() == s)
    }

    pub fn label(self) -> &'static str {
        match self {
            Role::Viewer => "Lector",
            Role::Member => "Miembro",
            Role::Admin => "Admin",
            Role::Owner => "Dueño",
        }
    }
}

/// Miembro de un equipo (compatible con `User`: los clientes que ignoran `role` siguen funcionando).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Member {
    pub id: Id,
    pub username: String,
    pub role: Role,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetRole {
    pub role: Role,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewInvite {
    pub role: Role,
    /// Cantidad de usos; por defecto 10.
    #[serde(default)]
    pub max_uses: Option<i64>,
    /// Horas de validez; por defecto 168 (7 días), máximo 720.
    #[serde(default)]
    pub expires_hours: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invite {
    pub id: Id,
    pub team_id: Id,
    pub role: Role,
    pub max_uses: i64,
    pub uses: i64,
    pub expires_at: DateTime<Utc>,
}

/// El código en claro solo se devuelve al crearlo; en la base queda su hash.
#[derive(Debug, Serialize, Deserialize)]
pub struct InviteCreated {
    pub code: String,
    pub invite: Invite,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    pub id: Id,
    pub name: String,
    pub owner_id: Id,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Id,
    pub team_id: Id,
    pub name: String,
    pub description: String,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Label {
    pub id: Id,
    pub team_id: Id,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Todo,
    Doing,
    Done,
}

impl Status {
    pub const ALL: [Status; 3] = [Status::Todo, Status::Doing, Status::Done];

    pub fn as_str(self) -> &'static str {
        match self {
            Status::Todo => "todo",
            Status::Doing => "doing",
            Status::Done => "done",
        }
    }

    pub fn parse(s: &str) -> Option<Status> {
        Self::ALL.into_iter().find(|st| st.as_str() == s)
    }

    pub fn label(self) -> &'static str {
        match self {
            Status::Todo => "Por hacer",
            Status::Doing => "En curso",
            Status::Done => "Hecho",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Id,
    pub project_id: Id,
    pub title: String,
    pub description: String,
    pub status: Status,
    /// 0 = baja, 1 = media, 2 = alta, 3 = urgente
    pub priority: i64,
    pub assignee_id: Option<Id>,
    pub assignee: Option<String>,
    pub due: Option<DateTime<Utc>>,
    /// Tarea padre si es una subtarea (un solo nivel de anidación).
    #[serde(default)]
    pub parent_id: Option<Id>,
    #[serde(default)]
    pub labels: Vec<Label>,
    #[serde(default)]
    pub subtasks_total: i64,
    #[serde(default)]
    pub subtasks_done: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: Id,
    pub task_id: Id,
    pub user_id: Id,
    pub username: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

// ---- Requests / responses ----

#[derive(Debug, Serialize, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: User,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewTeam {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddMember {
    pub username: String,
    #[serde(default)]
    pub role: Option<Role>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewProject {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub color: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct NewTask {
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub priority: i64,
    #[serde(default)]
    pub assignee_id: Option<Id>,
    #[serde(default)]
    pub due: Option<DateTime<Utc>>,
    #[serde(default)]
    pub parent_id: Option<Id>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewLabel {
    pub name: String,
    #[serde(default)]
    pub color: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetLabels {
    pub label_ids: Vec<Id>,
}

/// Parche parcial; `assignee_id`/`due` usan `Some(None)` para limpiar el valor.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TaskPatch {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<Status>,
    pub priority: Option<i64>,
    #[serde(default, deserialize_with = "double_option")]
    pub assignee_id: Option<Option<Id>>,
    #[serde(default, deserialize_with = "double_option")]
    pub due: Option<Option<DateTime<Utc>>>,
}

fn double_option<'de, T, D>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    Option::<T>::deserialize(d).map(Some)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewComment {
    pub body: String,
}

/// Evento en tiempo real emitido por el servidor vía WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum Event {
    ProjectCreated(Project),
    TaskCreated(Task),
    TaskUpdated(Task),
    TaskDeleted { id: Id, project_id: Id },
    CommentAdded(Comment),
    LabelCreated(Label),
    LabelDeleted { id: Id, team_id: Id },
    MemberAdded { team_id: Id, user: User },
    MemberRemoved { team_id: Id, user_id: Id },
    MemberRoleChanged { team_id: Id, user_id: Id, role: Role },
}
