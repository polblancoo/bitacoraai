<p align="center"><a href="https://github.com/polblancoo/Rust_tuiManager/blob/main/crates/agent/README.md">English</a> · <b>Español</b> · <a href="https://github.com/polblancoo/Rust_tuiManager/blob/main/crates/agent/README.pt.md">Português</a></p>

# rust-tui-manager-agent (`tm-agent`)

Pone a trabajar a una IA cuando le asignan una tarea en el [tablero de comando Rust TUI Manager](https://github.com/polblancoo/Rust_tuiManager): vigila el servidor, abre un agente en la **carpeta de trabajo del proyecto**, comenta y pasa la tarea a **En curso** y **Hecho**.

Instructivo completo: [docs/AGENTES.md](https://github.com/polblancoo/Rust_tuiManager/blob/main/docs/AGENTES.md).

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
| `TM_ASK_TIMEOUT` | segundos que `ask` espera respuesta | 900 |

El comando recibe `TM_TASK_PROMPT`, `TM_TASK_ID`, `TM_TASK_TITLE`, `TM_PROJECT_ID`, `TM_URL` y `TM_TOKEN`.

## Licencia

MIT
