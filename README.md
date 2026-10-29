<p align="center"><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/logo/logo-512.png" width="140" alt="Logo de Rust TUI Manager"></p>

# Rust TUI Manager

## 🎛️ Tablero de comando para tus proyectos

Dirigí **varios proyectos a la vez desde un solo lugar**, con **personas y agentes de IA trabajando en el mismo tablero**. Un CTO, una gerente de operaciones o una dueña de agencia arma los proyectos, asigna recursos (humanos e IAs de distintos modelos), delega cada uno en un líder y **ve cómo va todo, momento a momento**.

![Tablero de comando: proyectos, IAs y tareas del equipo](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-01-tablero.png)

**Un proyecto puede ser lo que necesites:**

| Ejemplo | Cómo se ve |
|---|---|
| 🏭 **Fábrica de apps** | Una app por proyecto; las IAs escriben código y tests, las personas revisan y publican. |
| 📊 **Planilla o presupuesto** | Cada área es una tarea; una IA arma las hojas, finanzas aprueba. |
| 🧭 **Procesos de la empresa** | Onboarding, compras, auditorías, campañas: pasos con responsable y fecha. |
| 🔬 **Investigación o contenidos** | Agentes que recopilan y redactan, un editor que decide. |

Cada proyecto sabe **dónde vive su trabajo** (carpeta y repositorio); el líder por proyecto está en la [hoja de ruta](ROADMAP.md). Todo es **self-hosted**: tus datos quedan en tu servidor.

> 👉 **¿Primera vez? Seguí la [guía paso a paso: tu primer proyecto](https://github.com/polblancoo/Rust_tuiManager/blob/main/docs/GUIA.md)** (instalar, crear el equipo, sumar personas e IAs, asignar y seguir el avance, con capturas).

**Cómo está hecho:** gestor de tareas colaborativo, multiusuario y self-hosted, escrito en Rust. Un único servidor y tres clientes: una **TUI** (ratatui), una **app gráfica** (Tauri 2) y una **web** que el propio servidor sirve, con actualizaciones en tiempo real entre todos los conectados.

| Pieza | Crate / ruta | Qué es |
|---|---|---|
| Servidor | [`rust-tui-manager-server`](crates/server) (binario `tm-server`) | API REST + WebSocket sobre axum y SQLite |
| Cliente de terminal | [`rust-tui-manager`](crates/tui) | Tablero kanban con ratatui |
| Cliente gráfico | [`apps/desktop`](apps/desktop) | App Tauri 2 (la misma interfaz que la web, en ventana propia) |
| Cliente web | [`crates/server/web`](crates/server/web) | Interfaz gráfica servida por `tm-server` en `/`: tablero, calendario, Gantt, pizarra, bandeja |
| Modelos compartidos | [`rust-tui-manager-core`](crates/core) | Tipos y eventos usados por todos |

> La versión 0.1.x de `rust-tui-manager` era un gestor local de notas y snippets. Ese código sigue en [`legacy/`](legacy/) y en las versiones 0.1.x del crate.

![Tablero kanban en el cliente gráfico](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/desktop-board.png)

<table>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/desktop-task.png" alt="Detalle de tarea con etiquetas, subtareas y comentarios"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/desktop-calendar.png" alt="Vista de calendario por fecha límite"></td>
  </tr>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/desktop-team.png" alt="Gestión del equipo: roles e invitaciones por código"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/tui-board.png" alt="Cliente de terminal (ratatui)"></td>
  </tr>
</table>

### Cliente web (también en la app de escritorio)

`tm-server` sirve la interfaz en `/`: abrí `http://tu-servidor:7878` y listo.

<table>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-gantt.png" alt="Gantt con dependencias entre tareas"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-canvas.png" alt="Pizarra colaborativa con notas, tarjetas de tarea y conexiones"></td>
  </tr>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-inbox.png" alt="Bandeja de novedades al iniciar sesión"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-mindmap.png" alt="Mapa mental de una tarea"></td>
  </tr>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-settings.png" alt="Menú de configuración: cuenta, Telegram, equipo y preferencias"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-message.png" alt="Mensaje al equipo (bandeja, email y Telegram)"></td>
  </tr>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-team.png" alt="Equipo: buscar usuarios registrados, generar código de invitación y miembros"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-activity.png" alt="Actividad del proyecto"></td>
  </tr>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-invite.png" alt="Código de invitación generado"></td>
  </tr>
</table>

## 🤖 Tus agentes de IA, como integrantes del equipo

Los asistentes de código (Claude Code, Cursor, Windsurf, Zed o cualquier cliente [MCP](https://modelcontextprotocol.io)) **se conectan al mismo tablero que tu equipo humano**: cada IA tiene su propio usuario, su token y su rol, y trabaja como un integrante más.

- **Sumá todas las IAs que quieras**: en la barra lateral, sección **🤖 IAs → «+»**. Se crea el usuario, se suma al equipo y te muestra **el token una sola vez** con la configuración MCP lista para copiar.
- **Se coordinan con las personas**: una IA puede **crear una tarea y asignársela a un humano** del equipo, comentar, completarla o borrarla cuando es suya, y recibir las que le asignes. Todo queda a su nombre en la actividad.
- **Control humano**: el token se **rota o revoca con un clic**; el rol es lector o miembro; una IA **no puede** crear ni borrar equipos, invitar, cambiar roles ni tocar tareas ajenas (salvo comentar).
- **Un tablero de comandos**: asignás trabajo a tus agentes desde la web, la app o la terminal, y ves en vivo qué hace cada uno.

> Estado: las cuentas de IA con token ya funcionan en el servidor y en la web (API REST). El binario **`tm-mcp`** y el *skill* para instalarlo en tu plataforma de código están en desarrollo (ver [ROADMAP.md](ROADMAP.md)).

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

## Funciones

- **Equipos** con roles (lector, miembro, admin, dueño) e **invitaciones por código** con vencimiento y cupo de usos.
- **Proyectos y tablero kanban** (por hacer / en curso / hecho), prioridades, fechas límite y asignación.
- **Etiquetas** por equipo y **subtareas** con barra de progreso.
- **Comentarios** por tarea.
- **Calendario** mensual por fecha límite, **Gantt** con dependencias entre tareas (arrastrá las barras para mover fechas), **pizarra** colaborativa por proyecto (notas, tarjetas de tarea y conexiones) y **mapa mental** de cada tarea (cliente gráfico).
- **Responsable**: el líder (admin o dueño) asigna a cualquier miembro; un miembro solo toma tareas libres o suelta la suya.
- **Bandeja de novedades**: al iniciar sesión ves lo que pasó mientras no estabas (te asignaron, comentaron, te sumaron) y tus tareas abiertas; llega en vivo y, si cargás y verificás tu email, también por correo.
- **Cliente web**: `tm-server` sirve la misma interfaz gráfica en `/` (abrí `http://tu-servidor:7878` en el navegador, sin instalar nada; `TM_WEB=0` lo desactiva).
- **Menciones**: escribí `@usuario` en un comentario y esa persona recibe el aviso (bandeja, email y Telegram).
- **Recordatorios de vencimiento**: el responsable recibe un aviso cuando falta menos de 24 horas y otro, una sola vez, cuando la tarea vence sin cerrarse.
- **Tareas recurrentes** (diaria, semanal o mensual): al completarlas se crea la siguiente, con la fecha corrida, las etiquetas y las subtareas pendientes.
- **Adjuntos** (hasta 10 por tarea, 2 MB cada uno): el servidor los entrega siempre como descarga. La web muestra miniaturas y vista previa de imágenes (PNG, JPEG, GIF, WebP) y texto plano, verificando el contenido real del archivo; nunca HTML, SVG ni PDF.
- **Plantillas**: duplicá un proyecto con sus tareas, etiquetas y dependencias (queda todo pendiente y sin responsable).
- **Exportar** las tareas de un proyecto a CSV o JSON (el CSV neutraliza fórmulas de planilla).
- **Actividad** por proyecto: quién creó, movió, asignó, comentó o adjuntó, y cuándo.
- **Web instalable (PWA)**: desde el navegador del celular o la compu podés "instalarla" como una app.
- **Telegram** (opcional): cada usuario vincula su cuenta con el bot y recibe por ahí las novedades; cualquier miembro puede mandar **mensajes al equipo** (bandeja, email y Telegram).
- **Agregar miembros con búsqueda** de usuarios registrados, atajos con prefijo `Ctrl+T` y **ayuda con buscador** (`?`) en ambos clientes.
- **Google Calendar** (opcional): tus tareas con fecha aparecen en tu calendario; ver *Novedades de la 0.5*.
- **Tiempo real**: un cambio de cualquier usuario aparece al instante en todos los clientes del equipo.

## Instalación rápida

```sh
cargo install rust-tui-manager-server   # instala el binario `tm-server`
cargo install rust-tui-manager          # cliente de terminal
```

O desde este repositorio:

```sh
cargo run -p rust-tui-manager-server                          # servidor en 0.0.0.0:7878
cargo run -p rust-tui-manager -- http://127.0.0.1:7878        # cliente TUI (F2 = crear cuenta)
cargo run -p tm-desktop                                       # cliente gráfico
```

El cliente gráfico necesita las dependencias de Tauri (en Linux: `webkit2gtk-4.1`, `libsoup-3.0`).

## Configuración del servidor

| Variable | Por defecto | Descripción |
|---|---|---|
| `TM_ADDR` | `0.0.0.0:7878` | Dirección donde escucha |
| `TM_DATABASE` | `sqlite://tm-server.db` | Base de datos (se crea con permisos `0600`) |
| `TM_JWT_SECRET` | generado y guardado en la base | Secreto de los tokens; mínimo 32 caracteres |
| `TM_CORS_ORIGINS` | Tauri y localhost | Orígenes extra permitidos, separados por coma |
| `TM_TLS_CERT` / `TM_TLS_KEY` | — | Certificado y clave PEM para servir HTTPS directamente |
| `TM_TRUSTED_PROXY` | — | `1` si corre detrás de un proxy inverso propio (usa `X-Forwarded-For` para el rate limit) |

Para los avisos por correo (opcionales; sin esto solo funciona la bandeja):

| Variable | Descripción |
|---|---|
| `TM_SMTP_URL` | Servidor SMTP, p. ej. `smtps://smtp.ejemplo.com:465` o `smtp://127.0.0.1:2525` |
| `TM_SMTP_FROM` | Remitente, p. ej. `Tareas <avisos@ejemplo.com>` |
| `TM_SMTP_USER` / `TM_SMTP_PASSWORD` | Credenciales, si el servidor las pide |

Para Telegram, creá un bot con [@BotFather](https://t.me/BotFather) y pasale el token al servidor:

| Variable | Descripción |
|---|---|
| `TM_REMINDER_SECS` | Cada cuántos segundos se revisan los vencimientos (por defecto 300) |
| `TM_TELEGRAM_TOKEN` | Token del bot. El servidor lo atiende por *long polling*: no necesita URL pública ni HTTPS propio |
| `TM_GOOGLE_CLIENT_ID`, `TM_GOOGLE_CLIENT_SECRET`, `TM_GOOGLE_REDIRECT` | Cliente OAuth de Google Cloud que activa la sincronización con Google Calendar (redirección: `https://tu-servidor/api/google/callback`) |

Cada usuario lo vincula en **Configuración → Telegram** (web/escritorio) o con `Ctrl+T g` (TUI): genera un código de un solo uso (15 min) y se lo manda al bot como `/start CÓDIGO`; `/stop` desvincula. Una vez vinculado, el bot también responde: `/tareas` (tus tareas abiertas) y `/hecho N` (cierra la tarea N si es tuya, o cualquiera si sos admin). Solo se escribe a chats vinculados así, nunca a ids arbitrarios.

Cada usuario carga su email y lo confirma con un código de 6 dígitos; solo a las direcciones verificadas se les envían avisos (tope por hora).

Sin TLS propio, ponelo detrás de un proxy TLS antes de exponerlo a internet.

## Seguridad

- Contraseñas con argon2; tokens JWT de 7 días revocables con "cerrar sesión en todos los dispositivos".
- Bloqueo por cuenta (5 fallos = 15 minutos) y limitación por IP en login, registro y WebSocket.
- Límites de tamaño de cuerpo, de largo de cada campo y de cantidad por recurso en todos los endpoints.
- CORS restringido, cabeceras de seguridad y CSP estricta en la app Tauri.
- El token del WebSocket no viaja en la URL: se envía como primer mensaje.
- Los códigos de invitación se guardan hasheados y solo se muestran al crearlos.

## Roles

| Rol | Puede |
|---|---|
| Lector | Ver todo |
| Miembro | Crear y editar tareas, subtareas, comentarios, etiquetas y proyectos |
| Admin | Además, agregar o quitar miembros de rango menor, cambiar roles e invitar |
| Dueño | Todo; no puede abandonar el equipo |

Nadie puede otorgar un rol igual o superior al propio.

## Estructura

```
crates/core      modelos y eventos compartidos
crates/server    servidor axum + SQLite + WebSocket
crates/tui       cliente de terminal
apps/desktop     cliente Tauri (frontend estático en ui/)
legacy/          gestor local anterior (0.1.x)
```

## Licencia

MIT
