# Guía paso a paso: tu primer proyecto en el tablero de comando

Rust TUI Manager es un **tablero de comando**: un lugar donde ves y dirigís varios proyectos a la vez, con personas e IAs como recursos. Esta guía es para quien va a **dirigir trabajo con un equipo de personas y de IAs**: un CTO, una jefa de operaciones, una dueña de agencia… El proyecto puede ser una **app**, una **planilla de presupuesto**, un **proceso de onboarding**, una campaña, una auditoría: Rust TUI Manager no sabe de apps, sabe de **tareas, responsables y avance**. En 10 minutos tenés el primero armado.

> Las capturas son de una empresa de ejemplo («Mi empresa») con tres proyectos muy distintos.

**Lo que vas a hacer**

1. [Instalar el servidor](#1-instalar-el-servidor)
2. [Crear tu cuenta y tu equipo](#2-crear-tu-cuenta-y-tu-equipo)
3. [Crear los proyectos](#3-crear-los-proyectos)
4. [Sumar personas](#4-sumar-personas)
5. [Sumar IAs](#5-sumar-ias)
6. [Armar y asignar tareas](#6-armar-y-asignar-tareas)
7. [Seguir el avance](#7-seguir-el-avance)
8. [Delegar a un líder](#8-delegar-a-un-líder)
9. [Dirigir varios proyectos a la vez](#9-dirigir-varios-proyectos-a-la-vez)
10. [Cuando una IA necesita que hagas algo](#10-cuando-una-ia-necesita-que-hagas-algo)
11. [Idioma, tooltips y este manual](#11-idioma-tooltips-y-este-manual)
12. [Configurar el correo del servidor](#12-configurar-el-correo-del-servidor)

---

## 1. Instalar el servidor

El servidor guarda todo (equipos, tareas, usuarios) y sirve la web. Se instala una vez.

1. Descargá `tm-server-linux-x86_64.tar.gz` de la [última versión](https://github.com/polblancoo/Rust_tuiManager/releases/latest) (o `cargo install rust-tui-manager-server`).
2. Descomprimí y ejecutá `./tm-server`.
3. Abrí `http://localhost:7878` en el navegador.

Para que otras personas entren desde otras máquinas, poné el servidor en una dirección que alcancen (idealmente con HTTPS: ver [Configuración del servidor](https://github.com/polblancoo/Rust_tuiManager/blob/main/README.es.md)). Si solo vas a probar, alcanza con `localhost`.

Prefieras terminal o ventana propia, también hay un cliente de terminal y una app de escritorio en la misma página de descargas.

## 2. Crear tu cuenta y tu equipo

1. En la pantalla de inicio elegí **Crear cuenta** (usuario y contraseña de 8 caracteres o más).
2. En la barra lateral, **Equipos → «+»**, poné el nombre de tu organización (por ejemplo «Mi empresa»). Quedás como **Dueño**.

Un **equipo** es el grupo de personas e IAs que comparten proyectos. Podés tener varios.

## 3. Crear los proyectos

En **Proyectos → «+»** creá uno por cada cosa que quieras llevar adelante. En el ejemplo: una app, una planilla y un proceso, cada uno con su color.

![Tablero del proyecto «App móvil de reservas»](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-01-tablero.png)

Cada proyecto tiene su **tablero** (Por hacer · En curso · Hecho). Con **+ Nueva tarea** cargás el trabajo: título, descripción, prioridad, fechas y responsable. Los mismos datos se ven también en **calendario**, **Gantt** y **pizarra**.

![Otro proyecto: una planilla de presupuesto, con responsables humanos y de IA](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-08-otro-proyecto.png)

Cada proyecto guarda **dónde vive su trabajo**: con **⚙ Proyecto** (admin o dueño) definís la carpeta de trabajo y el repositorio, y se muestran arriba del tablero.

![Editar el proyecto: nombre, carpeta de trabajo y repositorio](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-09-proyecto.png)

> 💡 Tip: no hace falta que sea software. Una planilla, un proceso de onboarding o una auditoría se manejan igual: tareas con responsable y fecha.

## 4. Sumar personas

Botón **👥 Equipo** (arriba a la derecha) → **➕ Agregar / invitar**.

- **Ya tiene cuenta**: escribí parte de su nombre, elegí el rol y **Agregar**.
- **Todavía no tiene cuenta**: elegí un rol y **Generar código**. Usá **Copiar invitación** (texto listo para WhatsApp o mail con enlace de descarga, servidor y código) o **Enviar por email**.

![Invitar con un código](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-07-invitar.png)

| Rol | Qué puede hacer |
|---|---|
| **Lector** | Mirar. |
| **Miembro** | Crear, editar y comentar tareas; tomar las libres. |
| **Admin** | Además, asignar tareas a otros, agregar gente y generar invitaciones. |
| **Dueño** | Todo, incluido transferir o eliminar el equipo. |

El panel del equipo muestra a cada integrante con su rol, desde cuándo está, quién lo sumó y cuántas tareas abiertas tiene:

![Panel del equipo con personas e IAs](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-06-panel-equipo.png)

## 5. Sumar IAs

Las IAs (Claude Code, Cursor, Windsurf, Zed, Gemini u otro cliente compatible con [MCP](https://modelcontextprotocol.io)) trabajan **como un integrante más**. Tantas como necesites, de distintos modelos.

1. En la barra lateral, **🤖 IAs → «+»**.
2. Poné un nombre que te diga qué es (`claude-backend`, `gemini-datos`) y su rol: **Miembro** (crea y edita sus tareas) o **Lector** (solo mira).

   ![Nueva IA en el equipo](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-02-nueva-ia.png)

3. Se muestra **su token una sola vez**, con la configuración MCP lista para copiar. Es la «llave» de esa IA.

   ![Token de la IA y configuración MCP](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-03-token.png)

4. Si lo perdés o lo querés cortar: clic en la IA → **Nuevo token** (el anterior deja de servir) o **Revocar**.

   ![Ficha de una IA](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-03-ia.png)

**Qué puede y qué no puede hacer una IA**

- ✅ Crear tareas y **asignárselas a personas** del equipo, comentar, completar o borrar **las que creó ella o le asignaron**.
- ❌ Crear o borrar equipos, invitar gente, cambiar roles, o tocar tareas ajenas (salvo comentar).
- Todo lo que hace queda **a su nombre** en la actividad, y se ve con 🤖 en el panel.

> 🚧 **Estado**: las cuentas de IA y sus tokens ya funcionan contra la API del servidor. El binario `tm-mcp` y el *skill* que las conectan directo a tu plataforma de código están en desarrollo (ver el [ROADMAP](https://github.com/polblancoo/Rust_tuiManager/blob/main/ROADMAP.md)).

## 6. Armar y asignar tareas

- **Nueva tarea** en cualquier columna; abrila para sumar descripción, fechas, etiquetas, subtareas, adjuntos y dependencias.
- **Responsable**: el admin asigna a quien quiera (persona o IA); un miembro toma las libres o suelta la suya.
- **Menciones** `@usuario` en los comentarios avisan a esa persona.
- Una IA puede crear una tarea y **dejarla asignada a una persona**, para pedirle algo.

## 7. Seguir el avance

Todo se actualiza **en vivo**: no hace falta recargar.

| Vista | Para qué |
|---|---|
| **Tablero** | Qué está por hacer, en curso y hecho. |
| **Gantt** | Cronograma con fechas y dependencias; las vencidas se marcan. |
| **Actividad** | Quién hizo qué y cuándo, personas e IAs juntas. |
| **Calendario** | Qué vence cada día. |
| **🔔 Bandeja** | Novedades: asignaciones, menciones, mensajes. También por email y Telegram. |

![Gantt del proyecto, con tareas de personas y de IAs](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-04-gantt.png)

![Actividad: el historial del proyecto, con las acciones de las IAs a su nombre](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-05-actividad.png)

Además, **Google Calendar** (tus tareas con fecha aparecen en tu calendario) se conecta desde **Configuración**.

## 8. Delegar a un líder

Hoy, para delegar un proyecto: sumá a la persona con rol **Admin** (puede asignar tareas, agregar gente y generar invitaciones) y asignale las tareas principales. Ella arma su parte y suma a su gente.

> 🚧 **Próximamente**: **líder por proyecto** (con alcance solo en ese proyecto), **ámbito configurable por IA** (qué proyectos toca, si puede invitar a otras IAs) y **reportes de avance** con gráficos. Mirá la [hoja de ruta](https://github.com/polblancoo/Rust_tuiManager/blob/main/ROADMAP.md).

## 9. Dirigir varios proyectos a la vez

**📊 Portafolio** es el primer botón de la lista de *Proyectos*. Muestra todos los proyectos del equipo en una sola pantalla:

- **Arriba**: proyectos, avance general, tareas en curso, vencidas y proyectos en riesgo.
- **Una tarjeta por proyecto**: barra de avance, quién tiene tareas abiertas (las IAs con 🤖), última actividad, próximo vencimiento y cuántas acciones hicieron las IAs en 24 h.
- **Estado**: 🟢 al día · 🔴 con tareas vencidas · 🟡 sin movimiento en 7 días · ✅ terminado · ⚪ sin tareas.
- **Abajo**: lo último que hicieron las IAs. Un clic en una tarjeta abre el proyecto, y todo se actualiza en vivo.

![Portafolio con todos los proyectos](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-11-portafolio.png)

**Tareas numeradas.** Cada tarea muestra su número (`#12`) en la tarjeta y en el detalle, para nombrarla sin ambigüedad en mensajes y comentarios.

**Orden manual.** Arrastrá una tarea dentro de su columna para cambiar el orden en que se hace; el orden que dejás manda sobre la prioridad. También podés elegir el responsable al crearla.

## 10. Cuando una IA necesita que hagas algo

Una IA puede toparse con algo que no puede hacer sola: un comando bloqueado, un permiso, una decisión. Entonces **te lo pide** y espera tu respuesta:

1. La IA corre `tm-agent ask "…"`. En la tarea aparece un comentario con 🙋, y se **crea una tarea de pedido** («🙋 Pedido de #12: …») asignada a una persona. El comentario te dice cuál y que podés **responder ahí mismo**.
2. Si el pedido trae un comando entre comillas o `backticks`, aparece en una caja con **📋 Copiar**: lo pegás en tu terminal.
3. Pegás la salida (o tu respuesta) como comentario en la tarea. La IA lo lee en su siguiente consulta (cada 10 s) y sigue. Marcá la tarea de pedido como Hecha.

![Pedido de una IA: tarea creada, comando para copiar](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-12-pedido.png)

**¿A quién le llega?** A los miembros con el permiso **Ejecutor** (ventana del equipo → tilde *Ejecutor* junto a cada persona; solo lo cambia un admin, y las IAs no pueden serlo):

1. a quien creó la tarea, si es ejecutor;
2. si no (por ejemplo la creó otra IA), al ejecutor con menos tareas abiertas;
3. si no hay ejecutores, a un admin del equipo.

**Comandos prohibidos.** En la ficha de cada IA, **🚫 Comandos prohibidos** (uno por línea, ej. `Bash(rm:*)`). La IA no puede ejecutarlos: si los necesita, te lo pide como arriba.

## 11. Idioma, tooltips y este manual

- **Idioma**: *Configuración → Preferencias → Idioma* (español, inglés o portugués). Por defecto usa el del navegador.
- **Tooltips**: dejá el cursor sobre un botón para ver qué hace.
- **Este manual** está dentro de la app: botón **📖 Manual** (abajo a la izquierda, o *Configuración → Ayuda*). Tiene buscador y un índice por temas con enlaces.

![El manual dentro de la app, con buscador](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-13-manual.png)

## 12. Configurar el correo del servidor

Para que la app mande **emails** (códigos de verificación, asignaciones, menciones, recordatorios) el **servidor** necesita una cuenta de correo saliente. Lo configura quien administra el servidor, una sola vez; los usuarios solo verifican su email desde *Configuración → Mi email*.

**¿Qué archivo se edita?** Si usás el servicio de systemd (`tm-server.service`), creá **`~/.config/tm-server/mail.env`** (permisos `600`, nunca dentro del repositorio) con estas cuatro variables:

```
TM_SMTP_URL=smtps://smtp.gmail.com:465
TM_SMTP_USER=tucuenta@gmail.com
TM_SMTP_PASSWORD=la-contraseña-de-aplicación
TM_SMTP_FROM=Tablero de comando <tucuenta@gmail.com>
```

y en `~/.config/systemd/user/tm-server.service` agregá, junto a los otros `EnvironmentFile`:

```
EnvironmentFile=-%h/.config/tm-server/mail.env
```

Después: `systemctl --user daemon-reload && systemctl --user restart tm-server`. Si lanzás `tm-server` a mano o con Docker, definí las mismas variables en el entorno (`export …` o `-e …`).

**Según tu proveedor** cambian `TM_SMTP_URL`, el usuario y la clave (verificá los datos en la documentación de tu proveedor):

| Proveedor | `TM_SMTP_URL` | `TM_SMTP_USER` | `TM_SMTP_PASSWORD` |
|---|---|---|---|
| Gmail | `smtps://smtp.gmail.com:465` | tu dirección completa | contraseña de aplicación (hace falta verificación en 2 pasos): https://myaccount.google.com/apppasswords |
| Outlook / Microsoft 365 | `smtp://smtp.office365.com:587?tls=required` | tu dirección completa | tu clave o contraseña de aplicación; en Microsoft 365 el administrador debe habilitar *SMTP autenticado* |
| Yahoo | `smtps://smtp.mail.yahoo.com:465` | tu dirección completa | contraseña de aplicación |
| Zoho | `smtps://smtp.zoho.com:465` | tu dirección completa | tu clave o contraseña de aplicación |
| Brevo | `smtp://smtp-relay.brevo.com:587?tls=required` | tu login de Brevo | clave SMTP |
| SendGrid | `smtps://smtp.sendgrid.net:465` | `apikey` | tu API key |
| Mailgun | `smtps://smtp.mailgun.org:465` | `postmaster@tu-dominio` | clave SMTP del dominio |
| Amazon SES | `smtps://email-smtp.REGIÓN.amazonaws.com:465` | usuario SMTP de SES | clave SMTP de SES |
| Servidor propio (Postfix…) | `smtp://localhost:25` | — | — |

- `smtps://` usa el puerto 465 (TLS directo); `smtp://…:587?tls=required` usa STARTTLS.
- Poné usuario y clave en `TM_SMTP_USER` y `TM_SMTP_PASSWORD` (no dentro de la URL) y no hace falta escapar caracteres especiales.
- `TM_SMTP_FROM` es el remitente; la mayoría de los proveedores exige que sea una dirección tuya o de tu dominio verificado.

**Probar**: *Configuración → Mi email*, escribí tu dirección y debería llegar el código de 6 dígitos. Si no llega, mirá el log del servidor (`journalctl --user -u tm-server -n 50`): una clave rechazada o un puerto bloqueado aparece ahí.

> Gmail y otros servicios gratuitos limitan los envíos (Gmail, unos 500 por día) y reescriben el remitente. Para empezar alcanza; con muchos usuarios conviene un servicio de envío con dominio propio.

---

**¿Algo no quedó claro?** Cada opción de **Configuración** tiene un «?» con ayuda. Y si encontrás un problema, abrí un [issue](https://github.com/polblancoo/Rust_tuiManager/issues).

---

# Instructivo: que una IA trabaje sola con `tm-agent`

Una IA solo actúa mientras algo la despierta. **`tm-agent`** es ese «algo»: un programa chico que corre en la **máquina donde está la carpeta del proyecto**, mira tu servidor y, cuando le asignás una tarea a la IA:

1. abre un agente (Claude Code, Gemini CLI, un script tuyo…) **dentro de la carpeta de trabajo del proyecto**,
2. pasa la tarea a **En curso** y la comenta,
3. al terminar la deja en **Hecho** con un resumen; si falla, la devuelve a **Por hacer** con el error.

Todo se ve en vivo en el tablero. Este instructivo es para **quien instala** y para **quien administra el equipo**.

```
 Tablero (servidor)  ◄── consulta cada 20 s ──  tm-agent  ──lanza──►  tu agente (claude -p …)
   Por hacer → En curso → Hecho                (en la máquina del proyecto)      trabaja en la carpeta
```

El mismo instructivo está **dentro de la app**: *Configuración → Agentes de IA → Ver instructivo*, y al crear una IA se muestra el comando listo con su token.

![Instructivo de tm-agent dentro de la app](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-10-agentes.png)

## Lo que necesitás

- El servidor andando y una cuenta de **admin o dueño** del equipo.
- Una máquina con la **carpeta del proyecto** y con el agente que querés usar instalado y con sesión iniciada (por ejemplo `claude`).
- Rust (`cargo`) para instalar `tm-agent`. Instaladores listos para descargar: ver el [ROADMAP](https://github.com/polblancoo/Rust_tuiManager/blob/main/ROADMAP.md).

## Paso a paso

### 1. Crear la IA

En la barra lateral: **🤖 IAs → «+»**. Elegí un nombre (`claude-backend`), su **modelo** (`claude:Opus`) y el rol **Miembro**. Copiá el **token** (`tmk_…`): se muestra una sola vez. Si lo perdés, clic en la IA → **Nuevo token**.

### 2. Definir la carpeta de trabajo

Con **⚙ Proyecto** completá **Carpeta de trabajo** (por ejemplo `/home/ana/proyectos/app`) y el **repositorio**. La ruta tiene que **existir en la máquina donde corre `tm-agent`**. Si no, el agente avisa con un comentario en la tarea y no la toma (o usá `TM_AGENT_DIR`).

### 3. Instalar `tm-agent`

En la máquina del proyecto:

```bash
cargo install rust-tui-manager-agent
```

### 4. Elegir el comando del agente

`TM_AGENT_CMD` es lo que se ejecuta por cada tarea, **dentro de la carpeta del proyecto**. Recibe el texto de la tarea en la variable `TM_TASK_PROMPT`. Ejemplos (revisá las opciones de tu versión del cliente):

| Agente | `TM_AGENT_CMD` |
|---|---|
| Claude Code | `claude -p "$TM_TASK_PROMPT" --permission-mode acceptEdits` |
| Gemini CLI | `gemini -p "$TM_TASK_PROMPT"` |
| Un script propio | `./mi-script.sh` (lee `$TM_TASK_PROMPT`, `$TM_TASK_ID`, …) |

Siempre se pasa el prompt por **variable de entorno**, nunca pegado en la línea de comandos, para que el texto de una tarea no pueda inyectar comandos en el shell.

### 5. Ejecutarlo

```bash
TM_URL=https://tareas.miempresa.com \
TM_TOKEN=tmk_… \
TM_AGENT_CMD='claude -p "$TM_TASK_PROMPT" --permission-mode acceptEdits' \
tm-agent
```

Verás `[tm-agent] conectado como claude-backend…`. Asignale una tarea a esa IA desde la web: en unos segundos pasa a **En curso**.

### 6. Dejarlo como servicio

**Linux (systemd, usuario)** — `~/.config/systemd/user/tm-agent.service`:

```ini
[Unit]
Description=tm-agent (IA del tablero de comando)
After=network-online.target

[Service]
Environment=TM_URL=https://tareas.miempresa.com
EnvironmentFile=%h/.config/tm-agent/claude-backend.env   # TM_TOKEN=tmk_… (chmod 600)
Environment="TM_AGENT_CMD=claude -p \"$TM_TASK_PROMPT\" --permission-mode acceptEdits"
ExecStart=%h/.cargo/bin/tm-agent
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
```

```bash
systemctl --user daemon-reload && systemctl --user enable --now tm-agent
```

En macOS se usa un `launchd` agent y en Windows el Programador de tareas, con las mismas variables de entorno.

## Variables

| Variable | Para qué | Por defecto |
|---|---|---|
| `TM_TOKEN` | Token de la IA (obligatorio) | — |
| `TM_AGENT_CMD` | Comando que lanza al agente (obligatorio) | — |
| `TM_URL` | Dirección del servidor | `http://127.0.0.1:7878` |
| `TM_AGENT_POLL` | Segundos entre consultas | `20` |
| `TM_AGENT_TIMEOUT` | Segundos máximos por tarea (después se cancela) | `1800` |
| `TM_AGENT_MAX` | Tareas en paralelo | `1` |
| `TM_AGENT_DIR` | Carpeta de respaldo si el proyecto no tiene una (o no existe acá) | — |
| `TM_AGENT_ONCE=1` | Procesa lo pendiente y sale (útil para probar) | — |

El comando también recibe `TM_TASK_ID`, `TM_TASK_TITLE`, `TM_PROJECT_ID`, `TM_URL` y `TM_TOKEN`, por si tu agente quiere comentar o crear subtareas por la API.

**Pedir ayuda a una persona.** Dentro de una tarea la IA puede ejecutar `tm-agent ask "pregunta"`: levanta una alarma 🙋 (en la app y por Telegram) y espera la respuesta como comentario. El servidor además **crea una tarea de pedido** para una persona: quien creó la tarea, si tiene el permiso **Ejecutor**; si no, el ejecutor con menos tareas abiertas; si no hay ejecutores, un admin. El comentario dice cuál es esa tarea y que podés responder ahí mismo; los comandos del pedido salen con un botón **📋 Copiar**. Mirá el [capítulo 10 de la guía](https://github.com/polblancoo/Rust_tuiManager/blob/main/docs/GUIA.md#10-cuando-una-ia-necesita-que-hagas-algo).

## Cómo se comporta

- Solo toma tareas **asignadas a su IA** y en **Por hacer**. Las de personas o de otras IAs las ignora.
- Si el agente termina bien → **Hecho** + comentario con el resumen. Si falla o se pasa del tiempo → vuelve a **Por hacer** + comentario con el error, y **mientras `tm-agent` siga corriendo** no la reintenta hasta que alguien la modifique.
- Una IA solo puede modificar las tareas que creó o que le asignaron (regla del servidor), aunque el agente intente más.
- Los **sub-agentes** que lance tu agente trabajan bajo la identidad de esa IA; para el trabajo interno conviene usar **subtareas**.

## Seguridad (leelo)

- **El texto de las tareas llega al agente como instrucciones.** Quien puede asignarle una tarea a la IA puede, en la práctica, darle órdenes. Asigná tareas solo desde cuentas de confianza.
- Corré el agente con **permisos acotados**: un usuario sin privilegios, la carpeta del proyecto y nada más. Evitá opciones del estilo «saltear todos los permisos».
- **Comandos prohibidos**: en la ficha de la IA, **🚫 Comandos prohibidos** (uno por línea, ej. `Bash(rm:*)`). `tm-agent` se la pasa al comando en `TM_DENY` y el agente de ejemplo (`run-claude.sh`) la suma a `--disallowedTools`. Si a la IA le bloquean algo que necesita, te avisa con una alarma 🙋 (`tm-agent ask`).
- El **token** es la llave de esa IA: guardalo en un archivo con permisos `600`, no en el repositorio. Se revoca con un clic en la web.
- Una IA **no puede** crear ni borrar equipos, invitar gente ni cambiar roles.

## Si algo no anda

| Síntoma | Causa probable |
|---|---|
| `falta TM_TOKEN` / `falta TM_AGENT_CMD` | Falta definir la variable. |
| `no pude iniciar sesión con TM_TOKEN` | Token revocado, equivocado o servidor mal escrito. |
| Comentario «la carpeta de trabajo … no existe» | La ruta del proyecto no existe en esa máquina: corregila en ⚙ Proyecto o usá `TM_AGENT_DIR`. |
| La tarea vuelve a Por hacer con un error | Mirá el comentario: es la salida del agente. Corregí y volvé a tocar la tarea. |
| No pasa nada al asignar | ¿La tarea está en **Por hacer** y asignada a *esa* IA? ¿`tm-agent` está corriendo? |
