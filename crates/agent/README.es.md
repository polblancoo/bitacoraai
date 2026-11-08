<p align="center"><a href="https://github.com/polblancoo/bitacoraai/blob/main/crates/agent/README.md">English</a> · <b>Español</b> · <a href="https://github.com/polblancoo/bitacoraai/blob/main/crates/agent/README.pt.md">Português</a></p>

# bitacora-agent (`tm-agent`)

> **0.6.3**: `TM_AGENT_TERMINAL=1` abre terminales web en la carpeta del proyecto; el agente pasa `TM_ALLOW` (comandos autorizados desde la tarea), informa tokens y costo reales y respeta las cuotas. Licencia **AGPL-3.0-only**.
>
> **0.6.2**: cuando una IA pide ayuda (`tm-agent ask`), el servidor crea una tarea de pedido para una persona autorizada (permiso **Ejecutor**) y el comando sale con botón Copiar en la web. Sitio y descargas: https://polblancoo.github.io/rust_tuiManager/

### Qué lo hace distinto

Lo que no vimos junto en ninguna otra herramienta (comprobalo, y avisanos si encontrás una):

- 🤖 **IAs como integrantes del equipo**, con rol, lista de comandos prohibidos y permisos que una persona da desde la propia tarea (`/permitir cargo build`).
- 🙋 **Pedidos de ayuda de la IA** que llegan como tarea a quien corresponde.
- 💻 **Terminal SSH dentro de la tarea** (con clave), y lo hecho queda escrito para que la IA retome desde ahí. *(0.6.3)*
- 📊 **Portafolio de varios proyectos**, con costo y horas de la IA. *(costo y horas: llegan en la 0.6.3)*
- 🏠 **Autoalojado**: tus datos y tus claves quedan en tu servidor.

Pone a trabajar a una IA cuando le asignan una tarea en el [tablero de comando BitacoraAI](https://github.com/polblancoo/bitacoraai): vigila el servidor, abre un agente en la **carpeta de trabajo del proyecto**, comenta y pasa la tarea a **En curso** y **Hecho**.

Instructivo completo: [docs/AGENTES.md](https://github.com/polblancoo/bitacoraai/blob/main/docs/AGENTES.md).

```bash
TM_URL=https://tasks.example.com \
TM_TOKEN=tmk_… \
TM_AGENT_CMD='claude -p "$TM_TASK_PROMPT" --permission-mode acceptEdits' \
tm-agent
```

## Cómo funciona

1. Toma las tareas `todo` asignadas a su cuenta de IA y las pasa a `doing`, dejando un comentario.
2. Ejecuta tu comando (`sh -c` / `cmd /C`) en la carpeta de trabajo del proyecto.
3. Si sale bien: tarea a `done` con un resumen. Si falla: vuelve a `todo` con el error (no reintenta mientras corre).
4. Si la carpeta no existe, comenta y no toma la tarea.

Dentro de una tarea la IA puede llamar a `tm-agent ask "pregunta"`: levanta una alarma 🙋 (en la app y por Telegram) y espera la respuesta de una persona.

## Variables

| Variable | Significado | Por defecto |
|---|---|---|
| `TM_TOKEN` | token de la cuenta de IA (`tmk_…`) | obligatoria |
| `TM_AGENT_CMD` | comando que hace el trabajo | obligatoria |
| `TM_URL` | dirección del servidor | `http://127.0.0.1:7878` |
| `TM_AGENT_POLL` | segundos entre consultas | 20 |
| `TM_AGENT_TIMEOUT` | segundos por tarea | 1800 |
| `TM_AGENT_MAX` | tareas en paralelo | 1 |
| `TM_AGENT_DIR` | carpeta si el proyecto no tiene | — |
| `TM_AGENT_ONCE` | `1` = una pasada y salir | — |
| `TM_AGENT_TERMINAL` | `1` = permite que un owner abra una terminal en esta máquina desde la web (la conexión sale desde acá: sin claves ni puertos abiertos) | apagado |
| `TM_ASK_TIMEOUT` | segundos que `ask` espera respuesta | 900 |

El comando recibe `TM_TASK_PROMPT`, `TM_TASK_ID`, `TM_TASK_TITLE`, `TM_PROJECT_ID`, `TM_URL` y `TM_TOKEN`.

## Licencia

**AGPL-3.0-only.** Gratis para usar y autoalojar. Si ofrecés una versión modificada como servicio, tenés que publicar tus cambios con la misma licencia. Hay licencia comercial sin esa obligación: consultala en [los issues](https://github.com/polblancoo/bitacoraai/issues). Las versiones publicadas hasta la 0.6.2 siguen siendo MIT.
