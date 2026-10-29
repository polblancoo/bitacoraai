# rust-tui-manager-server

Servidor multiusuario (axum + SQLite + WebSocket) del gestor de tareas colaborativo [Rust TUI Manager](https://github.com/polblancoo/tui_comandNote). Los clientes son [`rust-tui-manager`](https://crates.io/crates/rust-tui-manager) (terminal) y la app Tauri del repositorio.

Clientes: la TUI, la app gráfica de escritorio y la **web que este mismo servidor sirve en `/`** (desactivable con `TM_WEB=0`).

![Cliente gráfico conectado al servidor](https://raw.githubusercontent.com/polblancoo/tui_comandNote/main/docs/screenshots/desktop-board.png)

<table>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/tui_comandNote/main/docs/screenshots/web-gantt.png" alt="Gantt con dependencias"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/tui_comandNote/main/docs/screenshots/web-canvas.png" alt="Pizarra colaborativa"></td>
  </tr>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/tui_comandNote/main/docs/screenshots/web-inbox.png" alt="Bandeja de novedades"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/tui_comandNote/main/docs/screenshots/web-settings.png" alt="Menú de configuración"></td>
  </tr>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/tui_comandNote/main/docs/screenshots/web-team.png" alt="Equipo: buscar usuarios registrados"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/tui_comandNote/main/docs/screenshots/web-activity.png" alt="Actividad del proyecto"></td>
  </tr>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/tui_comandNote/main/docs/screenshots/web-invite.png" alt="Código de invitación generado"></td>
  </tr>
</table>

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
| `TM_REMINDER_SECS` | `300` | Cada cuántos segundos se revisan los vencimientos para los recordatorios |
| `TM_TELEGRAM_TOKEN` | — | Token del bot de Telegram (de @BotFather); activa vincular cuentas y avisos por Telegram |
| `TM_GOOGLE_CLIENT_ID`, `TM_GOOGLE_CLIENT_SECRET`, `TM_GOOGLE_REDIRECT` | — | Cliente OAuth de Google Cloud («Aplicación web») que activa Google Calendar; la redirección es `https://tu-servidor/api/google/callback` |
| `TM_GOOGLE_SYNC_SECS` | `60` | Cada cuántos segundos se sincroniza el calendario de los usuarios conectados |
| `TM_SMTP_USER` / `TM_SMTP_PASSWORD` | — | Credenciales SMTP, si hacen falta |

Nuevos endpoints: `GET /api/me`, `PUT/DELETE /api/me/email`, `POST /api/me/email/verify`, `GET /api/users/search?q=&team_id=` (solo admin del equipo), `POST /api/me/telegram/link`, `DELETE /api/me/telegram`, `POST /api/teams/{id}/messages` (miembro o más), `GET/POST /api/tasks/{id}/attachments` (cuerpo binario, `?name=`), `GET/DELETE /api/attachments/{id}`, `POST /api/projects/{id}/duplicate`, `GET /api/projects/{id}/export?format=csv|json`, `GET /api/projects/{id}/activity`, `GET /api/inbox`, `POST /api/notifications/read`, `PUT/DELETE /api/tasks/{id}/deps/{dep}`, y la pizarra: `GET /api/projects/{id}/canvas`, `POST .../canvas/items|links`, `PATCH/DELETE /api/canvas/items/{id}`, `DELETE /api/canvas/links/{id}`.
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

## Google Calendar

Cada usuario conecta su cuenta desde Configuración → Google Calendar y sus tareas con fecha (asignadas a él o sin responsable, de sus equipos) aparecen como eventos de día completo en su calendario principal; se actualizan o se borran solos. La sincronización es de un solo sentido. Quien administra el servidor lo configura una sola vez: crea un proyecto en Google Cloud, habilita la *Google Calendar API*, configura la pantalla de consentimiento (con los Gmail de prueba, hasta 100, o la verificación de Google para abrirla) y crea un cliente OAuth tipo «Aplicación web»; el secreto no se comparte ni se sube a git.
