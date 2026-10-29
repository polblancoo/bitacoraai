# rust-tui-manager-agent (`tm-agent`)

Pone a trabajar a una IA cuando le asignan una tarea en el [tablero de comando Rust TUI Manager](https://github.com/polblancoo/Rust_tuiManager): vigila el servidor, abre un agente en la **carpeta de trabajo del proyecto**, comenta y pasa la tarea a **En curso** y **Hecho**.

Instructivo completo: [docs/AGENTES.md](https://github.com/polblancoo/Rust_tuiManager/blob/main/docs/AGENTES.md).

```bash
TM_URL=https://tareas.miempresa.com \
TM_TOKEN=tmk_… \
TM_AGENT_CMD='claude -p "$TM_TASK_PROMPT" --permission-mode acceptEdits' \
tm-agent
```
