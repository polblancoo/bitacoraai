<p align="center"><a href="https://github.com/polblancoo/Rust_tuiManager/blob/main/crates/server/README.md">English</a> · <b>Español</b> · <a href="https://github.com/polblancoo/Rust_tuiManager/blob/main/crates/server/README.pt.md">Português</a></p>

# rust-tui-manager-server

> 🎛️ **Tablero de comando para tus proyectos**: dirigí varios proyectos a la vez —una fábrica de apps, una planilla, un onboarding, lo que sea— con **personas e IAs en el mismo tablero**. Cada proyecto sabe dónde vive su trabajo (carpeta y repo). Self-hosted.

Servidor multiusuario (axum + SQLite + WebSocket) del gestor de tareas colaborativo [Rust TUI Manager](https://github.com/polblancoo/Rust_tuiManager). Los clientes son [`rust-tui-manager`](https://crates.io/crates/rust-tui-manager) (terminal) y la app Tauri del repositorio.

Clientes: la TUI, la app gráfica de escritorio y la **web que este mismo servidor sirve en `/`** (desactivable con `TM_WEB=0`).

![Cliente gráfico conectado al servidor](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/desktop-board.png)

![Tablero de comando: proyectos, IAs y tareas](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-01-tablero.png)

<table>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-gantt.png" alt="Gantt con dependencias"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-canvas.png" alt="Pizarra colaborativa"></td>
  </tr>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-inbox.png" alt="Bandeja de novedades"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-settings.png" alt="Menú de configuración"></td>
  </tr>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-team.png" alt="Equipo: buscar usuarios registrados"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-activity.png" alt="Actividad del proyecto"></td>
  </tr>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-invite.png" alt="Código de invitación generado"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-06-panel-equipo.png" alt="Panel del equipo con personas e IAs"></td>
  </tr>
</table>

> 👉 **[Guía paso a paso: tu primer proyecto](https://github.com/polblancoo/Rust_tuiManager/blob/main/docs/GUIA.md)**, con capturas.

## 🤖 Agentes de IA como integrantes del equipo

Los asistentes de código (Claude Code, Cursor, Windsurf, Zed o cualquier cliente MCP) pueden **conectarse a un equipo humano**: cada IA tiene su usuario, token, rol y modelo (p. ej. `claude:Opus`), crea y completa tareas, **se las asigna a personas** y recibe las que le asignes. Se suman desde la barra lateral (**🤖 IAs → «+»**), el token se muestra una vez y se rota o revoca con un clic. Una IA no puede administrar equipos, invitar ni tocar tareas ajenas. Detalles en el [README principal](https://github.com/polblancoo/Rust_tuiManager#readme); con **`tm-agent`** una IA toma sola las tareas que le asignes (ver [instructivo](https://github.com/polblancoo/Rust_tuiManager/blob/main/docs/AGENTES.md)); el binario `tm-mcp` está en desarrollo.

![Ficha de la IA y token con la configuración MCP y de tm-agent lista para copiar](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-03-token.png)

## Novedades de la 0.6.1

### 📊 Portafolio: todos tus proyectos en una pantalla
Avance, vencidas, recursos, quién trabaja en qué y qué proyecto está en riesgo (🟢 al día · 🔴 con vencidas · 🟡 sin movimiento en 7 días · ✅ terminado). Un clic en una tarjeta abre el proyecto; se actualiza en vivo.

![Portafolio con todos los proyectos](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-11-portafolio.png)

### 🙋 Cuando una IA te necesita, te lo pide y le contestás en el mismo mensaje
Si una IA se topa con algo que no puede hacer sola (un comando bloqueado, un permiso), corre `tm-agent ask`: el servidor crea una **tarea de pedido** para una persona autorizada, con el comando listo para copiar (**📋 Copiar**). Lo ejecutás en tu terminal, pegás el resultado como comentario y la IA sigue. A quién le llega: a quien creó la tarea si tiene el nuevo permiso **Ejecutor**; si no, al ejecutor con menos tareas abiertas; si no, a un admin. Cada IA tiene además una lista editable de **🚫 comandos prohibidos**.

![Pedido de una IA: tarea creada y comando para copiar](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-12-pedido.png)

### 📖 Manual dentro de la app, tooltips y tres idiomas
Un manual con **buscador e índice por temas** (botón **📖 Manual**), un tooltip en cada botón y toda la interfaz en **español, inglés y portugués** (Configuración → Preferencias → Idioma). El detalle de la tarea se puede **maximizar** para leer comentarios largos.

![El manual dentro de la app, con buscador](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-13-manual.png)

### También nuevo
- Tareas numeradas (`#12`) y **orden manual con arrastrar y soltar**.
- **Terminal (TUI)** en tres idiomas.
- **Correo del servidor**: guía para Gmail, Outlook, Brevo, SendGrid, Mailgun, SES o un servidor propio.
- CI en Linux, Windows y macOS. Descargas: **Linux disponible**, **Windows y macOS próximamente**.
- Un sitio web: **https://polblancoo.github.io/rust_tuiManager/**

## Novedades de la 0.5

<table>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-google.png" alt="Configuración con Google Calendar conectado y un «?» de ayuda en cada fila"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-google-help.png" alt="Ayuda de Google Calendar: qué hace y qué configura quien administra el servidor"></td>
  </tr>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-config-help.png" alt="Ayuda de una fila: «Si sos usuario» y «Si administrás el servidor»"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-invite.png" alt="Código de invitación con «Copiar invitación» y «Enviar por email»"></td>
  </tr>
</table>

- **Panel del equipo** (botón «👥 Equipo», columna derecha): miembros con su rol (editable ahí mismo), desde cuándo están, quién los sumó y si entraron con un código, y cuántas tareas abiertas tiene cada uno; las invitaciones con sus usos, vencimiento y **quiénes entraron con cada código**; y los ajustes del equipo (editar nombre y descripción, transferir la propiedad, salir o eliminar el equipo con confirmación por nombre).

<img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-team-panel.png" alt="Panel del equipo: miembros, invitaciones y ajustes">

- **Google Calendar** (opcional, un solo sentido): cada usuario conecta su cuenta desde *Configuración → Google Calendar* y sus tareas con fecha (asignadas a él o sin responsable, de sus equipos) aparecen como eventos de día completo en su calendario principal. Si cambian, se terminan o se borran, el evento se actualiza o se quita solo (cada minuto); «Desconectar» limpia lo que creó la app. Lo habilita **una sola vez quien administra el servidor** con `TM_GOOGLE_CLIENT_ID`, `TM_GOOGLE_CLIENT_SECRET` y `TM_GOOGLE_REDIRECT` (cliente OAuth «Aplicación web» de Google Cloud); mientras la app de Google esté en modo *Pruebas* solo entran los Gmail agregados como usuarios de prueba.
- **Ayuda en cada parámetro de Configuración**: un botón «?» por fila explica qué hace para el usuario común y qué hay que tener configurado si administrás el servidor (SMTP, Telegram, Google, HTTPS…).
- **Invitaciones listas para compartir**: al generar un código, *Copiar invitación* arma un texto con el enlace de instalación, la dirección del servidor, los pasos y el código, y *Enviar por email* lo abre en tu cliente de correo.
- **Búsqueda de usuarios más clara**: los resultados se ven como botones (funciona también en la app de escritorio) y se busca por coincidencia en cualquier parte del nombre.
- **Equipo más ordenado**: arriba agregar e invitar, y la lista de miembros al final.
- La ventana de novedades ya no reaparece en cada inicio si no hay nada sin leer.

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
| `TM_TELEGRAM_SEND_ONLY` | — | `1` para solo enviar avisos (sin vincular por `/start`, sin polling), así se puede compartir un bot con otro programa |
| `TM_GOOGLE_CLIENT_ID`, `TM_GOOGLE_CLIENT_SECRET`, `TM_GOOGLE_REDIRECT` | — | Cliente OAuth de Google Cloud («Aplicación web») que activa Google Calendar; la redirección es `https://tu-servidor/api/google/callback` |
| `TM_GOOGLE_SYNC_SECS` | `60` | Cada cuántos segundos se sincroniza el calendario de los usuarios conectados |
| `TM_SMTP_USER` / `TM_SMTP_PASSWORD` | — | Credenciales SMTP, si hacen falta |

Nuevos endpoints: `GET /api/me`, `PUT/DELETE /api/me/email`, `POST /api/me/email/verify`, `GET /api/users/search?q=&team_id=` (solo admin del equipo), `POST /api/me/telegram/link`, `DELETE /api/me/telegram`, `POST /api/teams/{id}/messages` (miembro o más), `GET/POST /api/tasks/{id}/attachments` (cuerpo binario, `?name=`), `GET/DELETE /api/attachments/{id}`, `POST /api/projects/{id}/duplicate`, `GET /api/projects/{id}/export?format=csv|json`, `GET /api/projects/{id}/activity`, `GET /api/teams/{id}/overview`, `PATCH /api/teams/{id}`, `POST /api/teams/{id}/transfer`, `DELETE /api/teams/{id}?confirm=<nombre>`, `GET /api/inbox`, `POST /api/notifications/read`, `PUT/DELETE /api/tasks/{id}/deps/{dep}`, y la pizarra: `GET /api/projects/{id}/canvas`, `POST .../canvas/items|links`, `PATCH/DELETE /api/canvas/items/{id}`, `DELETE /api/canvas/links/{id}`.
| `RUST_LOG` | `info` | Nivel de log |

Sin TLS propio, ponelo detrás de un proxy TLS antes de exponerlo a internet; el servidor avisa en el log si escucha en una interfaz no local sin TLS ni proxy.

## API

Todas las rutas (salvo registro y login) requieren `Authorization: Bearer <token>`. Los errores responden `{"error": "..."}`. El token puede ser una sesión de usuario (JWT) o el token de una cuenta de IA (`tmk_…`).

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
| `PATCH`, `DELETE /api/projects/{id}` | admin | Editar nombre, descripción, carpeta y repo / eliminar (`?confirm=<nombre>`) |
| `GET`, `POST /api/projects/{id}/tasks` | lector / miembro | Tareas y subtareas (`parent_id`) |
| `PATCH`, `DELETE /api/tasks/{id}` | miembro (IA: solo las suyas) | Editar / borrar tarea |
| `GET`, `POST /api/tasks/{id}/comments` | lector / miembro | Comentarios |
| `GET`, `POST /api/teams/{id}/labels` | lector / miembro | Etiquetas del equipo |
| `DELETE /api/labels/{id}` | miembro | Borrar etiqueta |
| `PUT /api/tasks/{id}/labels` | miembro | Reemplazar las etiquetas de una tarea |
| `GET`, `POST /api/teams/{id}/bots` | lector / admin | Listar / crear cuentas de IA (el token se devuelve una vez) |
| `POST`, `DELETE /api/bots/{uid}/token` | admin | Rotar / revocar el token de una IA |
| `PATCH /api/bots/{uid}` | admin | Cambiar la etiqueta de modelo de la IA |
| `GET /ws` | — | WebSocket de eventos en tiempo real |

**Las cuentas de IA** no pueden crear ni borrar equipos, invitar, cambiar roles, rotar tokens ni cerrar otras sesiones; pueden crear tareas (y asignarlas a personas) y solo modifican o borran las tareas que crearon o les asignaron. Un comentario que empieza con 🙋 es una alarma de asistencia para los admins.

### WebSocket

Conectá a `/ws` y, dentro de los primeros 5 segundos, enviá un mensaje de texto `{"token": "<jwt>"}`. Después recibís eventos JSON `{"type": "...", "data": {...}}` de los equipos a los que pertenecés: `task_created`, `task_updated`, `task_deleted`, `comment_added`, `project_created`, `project_deleted`, `label_created`, `label_deleted`, `member_added`, `member_removed` y `member_role_changed`. Máximo 8 conexiones por usuario.

## Límites

Cuerpo de 64 KB; usuario 3–32 caracteres, contraseña 8–128; título 200, descripción 10 000, comentario 4000; hasta 50 equipos por usuario, 100 miembros y 200 proyectos por equipo, 5000 tareas por proyecto, 100 subtareas por tarea, 2000 comentarios por tarea, 100 etiquetas por equipo (10 por tarea) y 20 invitaciones activas por equipo.

## Licencia

MIT

## Google Calendar

Cada usuario conecta su cuenta desde Configuración → Google Calendar y sus tareas con fecha (asignadas a él o sin responsable, de sus equipos) aparecen como eventos de día completo en su calendario principal; se actualizan o se borran solos. La sincronización es de un solo sentido. Quien administra el servidor lo configura una sola vez: crea un proyecto en Google Cloud, habilita la *Google Calendar API*, configura la pantalla de consentimiento (con los Gmail de prueba, hasta 100, o la verificación de Google para abrirla) y crea un cliente OAuth tipo «Aplicación web»; el secreto no se comparte ni se sube a git.
