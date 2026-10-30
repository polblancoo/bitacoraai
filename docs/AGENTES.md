<p align="center"><a href="https://github.com/polblancoo/Rust_tuiManager/blob/main/docs/AGENTS.md">English</a> · <b>Español</b> · <a href="https://github.com/polblancoo/Rust_tuiManager/blob/main/docs/AGENTES.pt.md">Português</a></p>

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
