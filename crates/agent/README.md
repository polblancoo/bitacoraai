<p align="center"><b>English</b> · <a href="https://github.com/polblancoo/Rust_tuiManager/blob/main/crates/agent/README.es.md">Español</a> · <a href="https://github.com/polblancoo/Rust_tuiManager/blob/main/crates/agent/README.pt.md">Português</a></p>

# rust-tui-manager-agent (`tm-agent`)

Puts an AI to work when a task is assigned to it on the [Rust TUI Manager command board](https://github.com/polblancoo/Rust_tuiManager): it watches the server, opens an agent in the project's **work folder**, comments and moves the task to **Doing** and **Done**.

Full guide: [docs/AGENTS.md](https://github.com/polblancoo/Rust_tuiManager/blob/main/docs/AGENTS.md).

```bash
TM_URL=https://tasks.example.com \
TM_TOKEN=tmk_… \
TM_AGENT_CMD='claude -p "$TM_TASK_PROMPT" --permission-mode acceptEdits' \
tm-agent
```

## How it works

1. Takes the `todo` tasks assigned to its AI account and moves them to `doing`, leaving a comment.
2. Runs your command (`sh -c` / `cmd /C`) in the project's work folder.
3. On success: task to `done` with a summary. On failure: back to `todo` with the error (no retry while it is running).
4. If the folder does not exist, it comments and does not take the task.

Inside a task the AI can call `tm-agent ask "question"`: it raises a 🙋 alarm (in-app and Telegram) and waits for a human answer.

## Variables

| Variable | Meaning | Default |
|---|---|---|
| `TM_TOKEN` | AI account token (`tmk_…`) | required |
| `TM_AGENT_CMD` | command that does the work | required |
| `TM_URL` | server address | `http://127.0.0.1:7878` |
| `TM_AGENT_POLL` | seconds between checks | 20 |
| `TM_AGENT_TIMEOUT` | seconds per task | 1800 |
| `TM_AGENT_MAX` | tasks in parallel | 1 |
| `TM_AGENT_DIR` | work folder if the project has none | — |
| `TM_AGENT_ONCE` | `1` = one pass and exit | — |
| `TM_ASK_TIMEOUT` | seconds `ask` waits for an answer | 900 |

The command receives `TM_TASK_PROMPT`, `TM_TASK_ID`, `TM_TASK_TITLE`, `TM_PROJECT_ID`, `TM_URL` and `TM_TOKEN`.

## License

MIT
