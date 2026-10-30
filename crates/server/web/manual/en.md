# Step-by-step guide: your first project on the command board

Rust TUI Manager is a **command board**: one place where you see and direct several projects at once, with people and AIs as resources. This guide is for whoever will **direct work with a team of people and AIs**: a CTO, an operations lead, an agency owner… The project can be an **app**, a **budget spreadsheet**, an **onboarding process**, a campaign, an audit: Rust TUI Manager knows nothing about apps, it knows about **tasks, owners and progress**. In 10 minutes you will have the first one set up.

> The screenshots are from a sample company («Mi empresa») with three very different projects.

**What you will do**

1. [Install the server](#1-install-the-server)
2. [Create your account and your team](#2-create-your-account-and-your-team)
3. [Create the projects](#3-create-the-projects)
4. [Add people](#4-add-people)
5. [Add AIs](#5-add-ais)
6. [Build and assign tasks](#6-build-and-assign-tasks)
7. [Follow progress](#7-follow-progress)
8. [Delegate to a lead](#8-delegate-to-a-lead)
9. [Run several projects at once](#9-run-several-projects-at-once)
10. [When an AI needs you to do something](#10-when-an-ai-needs-you-to-do-something)
11. [Language, tooltips and this manual](#11-language-tooltips-and-this-manual)
12. [Set up the server's email](#12-set-up-the-servers-email)

---

## 1. Install the server

The server stores everything (teams, tasks, users) and serves the web app. You install it once.

1. Download `tm-server-linux-x86_64.tar.gz` from the [latest release](https://github.com/polblancoo/Rust_tuiManager/releases/latest) (or `cargo install rust-tui-manager-server`).
2. Unpack it and run `./tm-server`.
3. Open `http://localhost:7878` in your browser.

For other people to join from other machines, put the server at an address they can reach (ideally with HTTPS: see the [server configuration](https://github.com/polblancoo/Rust_tuiManager#readme)). If you are only trying it out, `localhost` is enough.

If you prefer a terminal or a window of its own, there is also a terminal client and a desktop app on the same downloads page.

## 2. Create your account and your team

1. On the start screen choose **Create account** (username and a password of 8 characters or more).
2. In the sidebar, **Teams → «+»**, enter your organization's name (for example «Mi empresa»). You become the **Owner**.

A **team** is the group of people and AIs who share projects. You can have several.

## 3. Create the projects

In **Projects → «+»** create one for each thing you want to move forward. In the example: an app, a spreadsheet and a process, each with its own color.

![Board of the project «Reservations mobile app»](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-01-tablero.png)

Each project has its own **board** (To do · Doing · Done). With **+ New task** you load the work: title, description, priority, dates and owner. The same data also shows up in **calendar**, **Gantt** and **whiteboard** views.

![Another project: a budget spreadsheet, with human and AI owners](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-08-otro-proyecto.png)

Each project keeps **where its work lives**: with **⚙ Project** (admin or owner) you set the work folder and the repository, and they are shown above the board.

![Edit the project: name, work folder and repository](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-09-proyecto.png)

> 💡 Tip: it does not have to be software. A spreadsheet, an onboarding process or an audit are handled the same way: tasks with an owner and a date.

## 4. Add people

**👥 Team** button (top right) → **➕ Add / invite**.

- **Already has an account**: type part of their name, choose the role and **Add**.
- **Does not have an account yet**: choose a role and **Generate code**. Use **Copy invitation** (text ready for WhatsApp or email with the download link, server and code) or **Send by email**.

![Invite with a code](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-07-invitar.png)

| Role | What they can do |
|---|---|
| **Viewer** | Look. |
| **Member** | Create, edit and comment on tasks; take unassigned ones. |
| **Admin** | Also assign tasks to others, add people and generate invitations. |
| **Owner** | Everything, including transferring or deleting the team. |

The team panel shows each member with their role, since when they have been in, who added them and how many open tasks they have:

![Team panel with people and AIs](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-06-panel-equipo.png)

## 5. Add AIs

AIs (Claude Code, Cursor, Windsurf, Zed, Gemini or any other client compatible with [MCP](https://modelcontextprotocol.io)) work **as one more team member**. As many as you need, from different models.

1. In the sidebar, **🤖 AIs → «+»**.
2. Enter a name that tells you what it is (`claude-backend`, `gemini-data`) and its role: **Member** (creates and edits its own tasks) or **Viewer** (only looks).

   ![New AI in the team](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-02-nueva-ia.png)

3. **Its token is shown only once**, with the MCP configuration ready to copy. It is that AI's «key».

   ![AI token and MCP configuration](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-03-token.png)

4. If you lose it or want to cut it off: click the AI → **New token** (the old one stops working) or **Revoke**.

   ![An AI's card](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-03-ia.png)

**What an AI can and cannot do**

- ✅ Create tasks and **assign them to people** on the team, comment, complete or delete **the ones it created or was assigned**.
- ❌ Create or delete teams, invite people, change roles, or touch other people's tasks (except commenting).
- Everything it does is recorded **under its name** in the activity feed, and it is shown with 🤖 in the panel.

> 🚧 **Status**: AI accounts and their tokens already work against the server API. The `tm-mcp` binary and the *skill* that connect them directly to your coding platform are in development (see the [ROADMAP](https://github.com/polblancoo/Rust_tuiManager/blob/main/ROADMAP.md)). Meanwhile, **`tm-agent`** already lets an AI pick up its assigned tasks on its own: see the [agents guide](https://github.com/polblancoo/Rust_tuiManager/blob/main/docs/AGENTS.md).

## 6. Build and assign tasks

- **New task** in any column; open it to add description, dates, labels, subtasks, attachments and dependencies.
- **Owner**: the admin assigns to whoever they want (person or AI); a member takes the unassigned ones or releases their own.
- **Mentions** `@user` in comments notify that person.
- An AI can create a task and **leave it assigned to a person**, to ask them for something.

## 7. Follow progress

Everything updates **live**: no need to reload.

| View | What for |
|---|---|
| **Board** | What is to do, doing and done. |
| **Gantt** | Schedule with dates and dependencies; overdue ones are marked. |
| **Activity** | Who did what and when, people and AIs together. |
| **Calendar** | What is due each day. |
| **🔔 Inbox** | News: assignments, mentions, messages. Also by email and Telegram. |

![Project Gantt, with tasks from people and AIs](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-04-gantt.png)

![Activity: the project history, with the AIs' actions under their own names](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-05-actividad.png)

In addition, **Google Calendar** (your tasks with a date appear in your calendar) is connected from **Settings**.

## 8. Delegate to a lead

Today, to delegate a project: add the person with the **Admin** role (they can assign tasks, add people and generate invitations) and assign them the main tasks. They build their part and bring in their people.

> 🚧 **Coming soon**: **lead per project** (scoped to that project only), **configurable scope per AI** (which projects it touches, whether it may invite other AIs) and **progress reports** with charts. See the [roadmap](https://github.com/polblancoo/Rust_tuiManager/blob/main/ROADMAP.md).

## 9. Run several projects at once

**📊 Portfolio** is the first button in the *Projects* list. It shows every project of the team on one screen:

- **Top**: projects, overall progress, tasks in progress, overdue and projects at risk.
- **One card per project**: progress bar, who has open tasks (AIs marked 🤖), last activity, next due date and how many actions the AIs took in 24 h.
- **Status**: 🟢 on track · 🔴 overdue tasks · 🟡 no movement for 7 days · ✅ finished · ⚪ no tasks.
- **Bottom**: the latest things the AIs did. Click a card to open the project; everything updates live.

![Portfolio with all projects](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-11-portafolio.png)

**Numbered tasks.** Every task shows its number (`#12`) on the card and in the detail, so you can refer to it unambiguously in messages and comments.

**Manual order.** Drag a task inside its column to change the order it gets done; your order wins over priority. You can also pick the assignee when creating it.

## 10. When an AI needs you to do something

An AI may hit something it cannot do alone: a blocked command, a permission, a decision. It then **asks you** and waits for your answer:

1. The AI runs `tm-agent ask "…"`. A 🙋 comment appears on the task and a **request task** («🙋 Request from #12: …») is created and assigned to a person. The comment tells you who, and that you can **reply right there**.
2. If the request contains a command in quotes or `backticks`, it shows in a box with **📋 Copy**: paste it in your terminal.
3. Paste the output (or your answer) as a comment on the task. The AI reads it on its next poll (every 10 s) and carries on. Mark the request task as Done.

![An AI's request: task created, command to copy](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-12-pedido.png)

**Who gets it?** Members with the **Executor** permission (team window → *Executor* checkbox next to each person; only an admin changes it, and AIs cannot be executors):

1. whoever created the task, if they are an executor;
2. otherwise (e.g. another AI created it), the executor with the fewest open tasks;
3. with no executors, a team admin.

**Forbidden commands.** On each AI's card, **🚫 Forbidden commands** (one per line, e.g. `Bash(rm:*)`). The AI cannot run them: if it needs one, it asks you as above.

## 11. Language, tooltips and this manual

- **Language**: *Settings → Preferences → Language* (Spanish, English or Portuguese). By default it follows your browser.
- **Tooltips**: hover over a button to see what it does.
- **This manual** lives inside the app: **📖 Manual** button (bottom left, or *Settings → Help*). It has search and a topic index with links.

![The manual inside the app, with search](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-13-manual.png)

## 12. Set up the server's email

For the app to send **emails** (verification codes, assignments, mentions, reminders) the **server** needs an outgoing mail account. Whoever runs the server sets it up once; users just verify their email from *Settings → My email*.

**Which file do I edit?** If you use the systemd service (`tm-server.service`), create **`~/.config/tm-server/mail.env`** (permissions `600`, never inside the repository) with these four variables:

```
TM_SMTP_URL=smtps://smtp.gmail.com:465
TM_SMTP_USER=youraccount@gmail.com
TM_SMTP_PASSWORD=the-app-password
TM_SMTP_FROM=Command board <youraccount@gmail.com>
```

and in `~/.config/systemd/user/tm-server.service` add, next to the other `EnvironmentFile` lines:

```
EnvironmentFile=-%h/.config/tm-server/mail.env
```

Then: `systemctl --user daemon-reload && systemctl --user restart tm-server`. If you start `tm-server` by hand or with Docker, define the same variables in the environment (`export …` or `-e …`).

**Depending on your provider**, `TM_SMTP_URL`, the user and the password change (check the details in your provider's documentation):

| Provider | `TM_SMTP_URL` | `TM_SMTP_USER` | `TM_SMTP_PASSWORD` |
|---|---|---|---|
| Gmail | `smtps://smtp.gmail.com:465` | your full address | app password (2-step verification required): https://myaccount.google.com/apppasswords |
| Outlook / Microsoft 365 | `smtp://smtp.office365.com:587?tls=required` | your full address | your password or an app password; in Microsoft 365 the admin must enable *authenticated SMTP* |
| Yahoo | `smtps://smtp.mail.yahoo.com:465` | your full address | app password |
| Zoho | `smtps://smtp.zoho.com:465` | your full address | your password or an app password |
| Brevo | `smtp://smtp-relay.brevo.com:587?tls=required` | your Brevo login | SMTP key |
| SendGrid | `smtps://smtp.sendgrid.net:465` | `apikey` | your API key |
| Mailgun | `smtps://smtp.mailgun.org:465` | `postmaster@your-domain` | the domain's SMTP password |
| Amazon SES | `smtps://email-smtp.REGION.amazonaws.com:465` | SES SMTP user | SES SMTP password |
| Own server (Postfix…) | `smtp://localhost:25` | — | — |

- `smtps://` uses port 465 (direct TLS); `smtp://…:587?tls=required` uses STARTTLS.
- Put the user and password in `TM_SMTP_USER` and `TM_SMTP_PASSWORD` (not inside the URL) and you do not need to escape special characters.
- `TM_SMTP_FROM` is the sender; most providers require it to be your own address or a verified domain of yours.

**Test it**: *Settings → My email*, type your address and the 6-digit code should arrive. If it does not, check the server log (`journalctl --user -u tm-server -n 50`): a rejected password or a blocked port shows up there.

> Gmail and other free services limit sending (Gmail, about 500 per day) and rewrite the sender. Fine to start; with many users, use a sending service with your own domain.

---

**Something not clear?** Every option in **Settings** has a «?» with help. And if you find a problem, open an [issue](https://github.com/polblancoo/Rust_tuiManager/issues).

---

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

When the AI asks, the server also **creates a request task** for a person (the task's creator if they have the **Executor** permission; otherwise the executor with the fewest open tasks; with no executors, an admin). The comment says which task and that you can answer right there; commands in the request show with a **📋 Copy** button. See the [guide, chapter 10](https://github.com/polblancoo/Rust_tuiManager/blob/main/docs/GUIDE.md#10-when-an-ai-needs-you-to-do-something).

## How it behaves

- It only takes tasks **assigned to its AI** and in **To do**. Tasks of people or other AIs are ignored.
- If the agent finishes well → **Done** + a comment with the summary. If it fails or runs out of time → back to **To do** + a comment with the error, and **while `tm-agent` keeps running** it does not retry until someone modifies it.
- An AI can only modify the tasks it created or that were assigned to it (a server rule), even if the agent tries more.
- **Sub-agents** your agent launches work under that AI's identity; for internal work, **subtasks** are better.

## Security (read this)

- **The text of tasks reaches the agent as instructions.** Whoever can assign a task to the AI can, in practice, give it orders. Assign tasks only from trusted accounts.
- Run the agent with **limited permissions**: an unprivileged user, the project folder and nothing else. Avoid options like «skip all permissions».
- **Forbidden commands**: in the AI's card, **🚫 Comandos prohibidos** (one per line, e.g. `Bash(rm:*)`). `tm-agent` hands the list to your command in `TM_DENY`, and an example wrapper can add it to Claude Code's `--disallowedTools`. If the AI is blocked from something it needs, it alerts you with a 🙋 alarm (`tm-agent ask`).
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
