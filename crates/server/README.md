# rust-tui-manager-server

Servidor multiusuario (axum + SQLite + WebSocket) del gestor de tareas colaborativo [Rust TUI Manager](https://github.com/polblancoo/tui_comandNote). Los clientes son [`rust-tui-manager`](https://crates.io/crates/rust-tui-manager) (terminal) y la app Tauri del repositorio.

Clientes: la TUI y la app gráfica de escritorio.

![Cliente gráfico conectado al servidor](https://raw.githubusercontent.com/polblancoo/tui_comandNote/main/docs/screenshots/desktop-board.png)

## Uso

```sh
cargo install rust-tui-manager-server
tm-server                      # escucha en 0.0.0.0:7878
```

La base SQLite se crea sola (con permisos `0600`) y el esquema se migra al arrancar, también desde bases creadas por versiones anteriores.

## Configuración

| Variable | Por defecto | Descripción |
|---|---|---|
| `TM_ADDR` | `0.0.0.0:7878` | Dirección donde escucha |
| `TM_DATABASE` | `sqlite://tm-server.db` | Base de datos |
| `TM_JWT_SECRET` | generado y guardado en la base | Secreto de los tokens; mínimo 32 caracteres |
| `TM_CORS_ORIGINS` | Tauri y localhost | Orígenes extra permitidos, separados por coma |
| `TM_TLS_CERT` / `TM_TLS_KEY` | — | Certificado y clave PEM para HTTPS directo |
| `TM_TRUSTED_PROXY` | — | `1` detrás de un proxy inverso propio (usa `X-Forwarded-For`) |
| `TM_WEB` | `1` | `0` desactiva el cliente web que el servidor sirve en `/` |
| `TM_SMTP_URL` / `TM_SMTP_FROM` | — | Habilitan los avisos por email (opcional): `smtps://host:465`, `Tareas <avisos@ejemplo.com>` |
| `TM_TELEGRAM_TOKEN` | — | Token del bot de Telegram (de @BotFather); activa vincular cuentas y avisos por Telegram |
| `TM_SMTP_USER` / `TM_SMTP_PASSWORD` | — | Credenciales SMTP, si hacen falta |

Nuevos endpoints: `GET /api/me`, `PUT/DELETE /api/me/email`, `POST /api/me/email/verify`, `GET /api/users/search?q=&team_id=` (solo admin del equipo), `POST /api/me/telegram/link`, `DELETE /api/me/telegram`, `POST /api/teams/{id}/messages` (miembro o más), `GET /api/inbox`, `POST /api/notifications/read`, `PUT/DELETE /api/tasks/{id}/deps/{dep}`, y la pizarra: `GET /api/projects/{id}/canvas`, `POST .../canvas/items|links`, `PATCH/DELETE /api/canvas/items/{id}`, `DELETE /api/canvas/links/{id}`.
| `RUST_LOG` | `info` | Nivel de log |

Sin TLS propio, ponelo detrás de un proxy TLS antes de exponerlo a internet; el servidor avisa en el log si escucha en una interfaz no local sin TLS ni proxy.

## API

Todas las rutas (salvo registro y login) requieren `Authorization: Bearer <token>`. Los errores responden `{"error": "..."}`.

| Método y ruta | Rol mínimo | Descripción |
|---|---|---|
| `POST /api/register`, `POST /api/login` | — | Devuelven `{token, user}` |
| `POST /api/logout-all` | — | Invalida todos los tokens del usuario |
| `GET`, `POST /api/teams` | — | Mis equipos / crear equipo |
| `GET`, `POST /api/teams/{id}/members` | lector / admin | Listar miembros / agregar por usuario |
| `PUT`, `DELETE /api/teams/{id}/members/{uid}` | admin (o uno mismo para salir) | Cambiar rol / quitar |
| `GET`, `POST /api/teams/{id}/invites` | admin | Listar / crear invitación |
| `DELETE /api/invites/{id}` | admin | Revocar invitación |
| `POST /api/invites/{code}/accept` | — | Unirse con un código |
| `GET`, `POST /api/teams/{id}/projects` | lector / miembro | Proyectos |
| `GET`, `POST /api/projects/{id}/tasks` | lector / miembro | Tareas y subtareas (`parent_id`) |
| `PATCH`, `DELETE /api/tasks/{id}` | miembro | Editar / borrar tarea |
| `GET`, `POST /api/tasks/{id}/comments` | lector / miembro | Comentarios |
| `GET`, `POST /api/teams/{id}/labels` | lector / miembro | Etiquetas del equipo |
| `DELETE /api/labels/{id}` | miembro | Borrar etiqueta |
| `PUT /api/tasks/{id}/labels` | miembro | Reemplazar las etiquetas de una tarea |
| `GET /ws` | — | WebSocket de eventos en tiempo real |

### WebSocket

Conectá a `/ws` y, dentro de los primeros 5 segundos, enviá un mensaje de texto `{"token": "<jwt>"}`. Después recibís eventos JSON `{"type": "...", "data": {...}}` de los equipos a los que pertenecés: `task_created`, `task_updated`, `task_deleted`, `comment_added`, `project_created`, `label_created`, `label_deleted`, `member_added`, `member_removed` y `member_role_changed`. Máximo 8 conexiones por usuario.

## Límites

Cuerpo de 64 KB; usuario 3–32 caracteres, contraseña 8–128; título 200, descripción 10 000, comentario 4000; hasta 50 equipos por usuario, 100 miembros y 200 proyectos por equipo, 5000 tareas por proyecto, 100 subtareas por tarea, 2000 comentarios por tarea, 100 etiquetas por equipo (10 por tarea) y 20 invitaciones activas por equipo.

## Licencia

MIT
