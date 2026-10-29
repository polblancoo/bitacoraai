<p align="center"><b>English</b> · <a href="https://github.com/polblancoo/Rust_tuiManager/blob/main/docs/AGENTES.md">Español</a> · <a href="https://github.com/polblancoo/Rust_tuiManager/blob/main/docs/AGENTES.pt.md">Português</a></p>

# Guide: let an AI work on its own with `tm-agent`

An AI only acts while something wakes it up. **`tm-agent`** is that «something»: a small program that runs on the **machine where the project folder lives**, watches your server and, when you assign a task to the AI:

1. opens an agent (Claude Code, Gemini CLI, a script of yours…) **inside the project's work folder**,
2. moves the task to **Doing** and comments on it,
3. when finished, leaves it in **Done** with a summary; if it fails, sends it back to **To do** with the error.

Everything shows live on the board. This guide is for **whoever installs it** and **whoever manages the team**.

```
 Board (server)  ◄── polls every 20 s ──  tm-agent  ──launches──►  your agent (claude -p …)
   To do → Doing → Done                (on the project machine)       works in the folder
```

The same guide is **inside the app**: *Settings → AI agents → View guide*, and when you create an AI the ready-to-run command with its token is shown.

![tm-agent guide inside the app](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-10-agentes.png)

## What you need

- The server running and an account that is **admin or owner** of the team.
- A machine with the **project folder** and with the agent you want to use installed and signed in (for example `claude`).
- Rust (`cargo`) to install `tm-agent`. Ready-made installers: see the [ROADMAP](https://github.com/polblancoo/Rust_tuiManager/blob/main/ROADMAP.md).

## Step by step

### 1. Create the AI

In the sidebar: **🤖 AIs → «+»**. Pick a name (`claude-backend`), its **model** (`claude:Opus`) and the **Member** role. Copy the **token** (`tmk_…`): it is shown only once. If you lose it, click the AI → **New token**.

### 2. Set the work folder

With **⚙ Project** fill in **Work folder** (for example `/home/ana/projects/app`) and the **repository**. The path must **exist on the machine where `tm-agent` runs**. If not, the agent warns with a comment on the task and does not take it (or use `TM_AGENT_DIR`).

### 3. Install `tm-agent`

On the project machine:

```bash
cargo install rust-tui-manager-agent
```

### 4. Choose the agent command

`TM_AGENT_CMD` is what runs for each task, **inside the project folder**. It receives the task text in the `TM_TASK_PROMPT` variable. Examples (check the options of your client version):

| Agent | `TM_AGENT_CMD` |
|---|---|
| Claude Code | `claude -p "$TM_TASK_PROMPT" --permission-mode acceptEdits` |
| Gemini CLI | `gemini -p "$TM_TASK_PROMPT"` |
| Your own script | `./my-script.sh` (reads `$TM_TASK_PROMPT`, `$TM_TASK_ID`, …) |

The prompt is always passed through an **environment variable**, never pasted into the command line, so a task's text cannot inject commands into the shell.

### 5. Run it

```bash
TM_URL=https://tasks.example.com \
TM_TOKEN=tmk_… \
TM_AGENT_CMD='claude -p "$TM_TASK_PROMPT" --permission-mode acceptEdits' \
tm-agent
```

You will see `[tm-agent] conectado como claude-backend…`. Assign a task to that AI from the web: within seconds it moves to **Doing**.

### 6. Leave it as a service

**Linux (systemd, user)**, `~/.config/systemd/user/tm-agent.service`:

```ini
[Unit]
Description=tm-agent (command board AI)
After=network-online.target

[Service]
Environment=TM_URL=https://tasks.example.com
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

On macOS use a `launchd` agent and on Windows the Task Scheduler, with the same environment variables.

## Variables

| Variable | What for | Default |
|---|---|---|
| `TM_TOKEN` | The AI's token (required) | — |
| `TM_AGENT_CMD` | Command that launches the agent (required) | — |
| `TM_URL` | Server address | `http://127.0.0.1:7878` |
| `TM_AGENT_POLL` | Seconds between polls | `20` |
| `TM_AGENT_TIMEOUT` | Maximum seconds per task (then it is cancelled) | `1800` |
| `TM_AGENT_MAX` | Tasks in parallel | `1` |
| `TM_AGENT_DIR` | Fallback folder if the project has none (or it does not exist here) | — |
| `TM_AGENT_ONCE=1` | Process what is pending and exit (useful for testing) | — |
| `TM_ASK_TIMEOUT` | Seconds `tm-agent ask` waits for a human answer | `900` |

The command also receives `TM_TASK_ID`, `TM_TASK_TITLE`, `TM_PROJECT_ID`, `TM_URL` and `TM_TOKEN`, in case your agent wants to comment or create subtasks through the API.

**Asking a human.** Inside a task the AI can run `tm-agent ask "question"`: it raises a 🙋 alarm for the team's owners and admins (in the app and by Telegram) and waits for the answer as a comment.

## How it behaves

- It only takes tasks **assigned to its AI** and in **To do**. Tasks of people or other AIs are ignored.
- If the agent finishes well → **Done** + a comment with the summary. If it fails or runs out of time → back to **To do** + a comment with the error, and **while `tm-agent` keeps running** it does not retry until someone modifies it.
- An AI can only modify the tasks it created or that were assigned to it (a server rule), even if the agent tries more.
- **Sub-agents** your agent launches work under that AI's identity; for internal work, **subtasks** are better.

## Security (read this)

- **The text of tasks reaches the agent as instructions.** Whoever can assign a task to the AI can, in practice, give it orders. Assign tasks only from trusted accounts.
- Run the agent with **limited permissions**: an unprivileged user, the project folder and nothing else. Avoid options like «skip all permissions».
- The **token** is that AI's key: keep it in a file with `600` permissions, not in the repository. It is revoked with one click on the web.
- An AI **cannot** create or delete teams, invite people or change roles.

## If something does not work

| Symptom | Likely cause |
|---|---|
| `falta TM_TOKEN` / `falta TM_AGENT_CMD` | The variable is not set. |
| `no pude iniciar sesión con TM_TOKEN` | Revoked or wrong token, or misspelled server. |
| Comment «work folder … does not exist» | The project's path does not exist on that machine: fix it in ⚙ Project or use `TM_AGENT_DIR`. |
| The task returns to To do with an error | Read the comment: it is the agent's output. Fix it and touch the task again. |
| Nothing happens when assigning | Is the task in **To do** and assigned to *that* AI? Is `tm-agent` running? |
