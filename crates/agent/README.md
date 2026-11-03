<p align="center"><b>English</b> · <a href="https://github.com/polblancoo/crewhelm/blob/main/crates/agent/README.es.md">Español</a> · <a href="https://github.com/polblancoo/crewhelm/blob/main/crates/agent/README.pt.md">Português</a></p>

# crewhelm-agent (`tm-agent`)

> **0.6.3**: `TM_AGENT_TERMINAL=1` opens web terminals in the project folder; the agent passes `TM_ALLOW` (commands allowed from the task), reports real tokens and cost and honors quotas. License **AGPL-3.0-only**.
>
> **0.6.2**: when an AI asks for help (`tm-agent ask`), the server creates a request task for an authorized person (the **Executor** permission) and the command shows with a Copy button in the web app. Site and downloads: https://polblancoo.github.io/rust_tuiManager/

### Why it is different

What we have not seen together in any other tool (check, and tell us if you find one):

- 🤖 **AIs as team members**, with a role, a list of forbidden commands and permissions that a person grants from the task itself (`/permitir cargo build`).
- 🙋 **Help requests from the AI** that arrive as a task for the right person.
- 💻 **SSH terminal inside the task** (key-based), and what was done is written down so the AI can pick up from there. *(0.6.3)*
- 📊 **Portfolio of several projects**, with the AI's cost and hours. *(cost and hours: coming in 0.6.3)*
- 🏠 **Self-hosted**: your data and keys stay on your server.

Puts an AI to work when a task is assigned to it on the [Rust TUI Manager command board](https://github.com/polblancoo/crewhelm): it watches the server, opens an agent in the project's **work folder**, comments and moves the task to **Doing** and **Done**.

Full guide: [docs/AGENTS.md](https://github.com/polblancoo/crewhelm/blob/main/docs/AGENTS.md).

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
| `TM_AGENT_TERMINAL` | `1` = lets an owner open a terminal on this machine from the web (the connection leaves from here: no keys or open ports) | off |
| `TM_ASK_TIMEOUT` | seconds `ask` waits for an answer | 900 |

The command receives `TM_TASK_PROMPT`, `TM_TASK_ID`, `TM_TASK_TITLE`, `TM_PROJECT_ID`, `TM_URL` and `TM_TOKEN`.

## License

**AGPL-3.0-only.** Free to use and to self-host. If you offer a modified version as a service, you must publish your changes under the same license. A commercial license without that obligation is available: ask in [the issues](https://github.com/polblancoo/crewhelm/issues). Versions published up to 0.6.2 remain under MIT.
