<p align="center"><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/logo/logo-512.png" width="140" alt="Rust TUI Manager logo"></p>

<p align="center"><b>English</b> · <a href="https://github.com/polblancoo/Rust_tuiManager/blob/main/README.es.md">Español</a> · <a href="https://github.com/polblancoo/Rust_tuiManager/blob/main/README.pt.md">Português</a></p>

# Rust TUI Manager

## 🎛️ A command board for your projects

Run **several projects at once from a single place**, with **people and AI agents working on the same board**. A CTO, an operations manager or an agency owner sets up the projects, assigns resources (humans and AIs of different models), delegates each one to a lead and **sees how everything is going, moment by moment**.

![Command board: projects, AIs and the team's tasks](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-01-tablero.png)

### Why it is different

What we have not seen together in any other tool (check, and tell us if you find one):

- 🤖 **AIs as team members**, with a role, a list of forbidden commands and permissions that a person grants from the task itself (`/permitir cargo build`).
- 🙋 **Help requests from the AI** that arrive as a task for the right person.
- 💻 **SSH terminal inside the task** (key-based), and what was done is written down so the AI can pick up from there. *(0.6.3)*
- 📊 **Portfolio of several projects**, with the AI's cost and hours.
- 🏠 **Self-hosted**: your data and keys stay on your server.

### Who it is for

- 🏢 **A company with its own AI and collaborative work.** Your people and your AIs share one board. The board, its data, the history and your keys stay on **your** server, and the AI runs on **your** machines; only what the model needs to read goes to the provider you chose. Each AI has a role, lists of allowed and forbidden commands, and a history of who did what.
- 🌍 **An AI the whole team uses: hired, or your own model on the server.** Either **you contract a model (Claude, Gemini, Codex…)** and install it **once on the server** with `tm-agent`, or **you run an open model on your own server** (for example with Ollama or llama.cpp, through any tool that takes a prompt and works on files; `tm-agent` launches whatever command you give it, though we have not tested local models yet). Everyone on the team uses it just by assigning it tasks, and nobody needs their own account. With a contracted model you pay the provider once (your subscription or API key); with your own model **nothing leaves your infrastructure**. **You stay in control:** `TM_AGENT_MAX` limits how many tasks it runs at the same time and the rest wait in line; each AI has its own role and lists of allowed and forbidden commands; it asks for permission when it needs it, and any owner or admin grants it from the task. Mix several models and change them whenever you want, since the board depends on no vendor. Tokens, cost and hours per project (with quotas) show what the shared AI costs each project. It also works with an AI that someone else runs on their own server: it connects over HTTPS with a token you **revoke in one click**, limited by role; *honest note:* the command lists are applied by the agent where the AI runs, so on machines you do not control combine them with the role and with reviewing each task.
- 👥 **Several people supervising several AIs.** Help requests reach the right person, anyone authorized grants a command from the task itself, and everything stays written down.

### More advantages

- 📦 **One binary + SQLite**: installed in minutes, backed up by copying one file. No per-seat licenses.
- 🔔 Real-time updates and notices by web, Telegram and email. Three languages, tooltips and a built-in manual.
- 💻 **Terminal in the browser** *(0.6.3)*, owners only. Two ways in: **through the agent** (`TM_AGENT_TERMINAL=1`: the connection leaves the project's machine, so no keys, no open ports, works behind NAT; Windows not tested yet) or **by SSH with a key** (with a button that installs the key using your password once, without storing it). Every session is noted in the task and in `.tm/terminal-*.md` inside the project folder, so the AI picks up where you left off.
- 🖥️ **Terminals view** *(0.6.3)*: up to 8 panes at once, and the state of each AI (working, needs you, finished) beside them.
- 📊 Tokens, cost and hours per project and AI, with **quotas** per team, project and task, and an estimate when you create the task.

**A project can be anything you need:**

| Example | What it looks like |
|---|---|
| 🏭 **App factory** | One app per project; AIs write code and tests, people review and ship. |
| 📊 **Spreadsheet or budget** | Each department is a task; an AI builds the sheets, finance approves. |
| 🧭 **Company processes** | Onboarding, purchasing, audits, campaigns: steps with an owner and a date. |
| 🔬 **Research or content** | Agents that gather and write, an editor who decides. |

Every project knows **where its work lives** (folder and repository); per-project leads are on the [roadmap](ROADMAP.md). Everything is **self-hosted**: your data stays on your server.

> 👉 **First time? Follow the [step-by-step guide: your first project](https://github.com/polblancoo/Rust_tuiManager/blob/main/docs/GUIDE.md)** (install, create the team, add people and AIs, assign work and follow progress, with screenshots).

**How it is built:** a collaborative, multi-user, self-hosted task manager written in Rust. One server and three clients: a **TUI** (ratatui), a **desktop app** (Tauri 2) and a **web app** served by the server itself, with real-time updates for everyone connected.

| Piece | Crate / path | What it is |
|---|---|---|
| Server | [`rust-tui-manager-server`](crates/server) (binary `tm-server`) | REST API + WebSocket on axum and SQLite |
| Terminal client | [`rust-tui-manager`](crates/tui) | Kanban board with ratatui |
| AI agent | [`rust-tui-manager-agent`](crates/agent) (binary `tm-agent`) | Watches the board and puts an AI to work when a task is assigned to it |
| Desktop client | [`apps/desktop`](apps/desktop) | Tauri 2 app (the same interface as the web, in its own window) |
| Web client | [`crates/server/web`](crates/server/web) | Graphical interface served by `tm-server` at `/`: board, calendar, Gantt, whiteboard, inbox |
| Shared models | [`rust-tui-manager-core`](crates/core) | Types and events used by everything |

> Version 0.1.x of `rust-tui-manager` was a local notes and snippets manager. That code still lives in [`legacy/`](legacy/) and in the 0.1.x releases of the crate.

![Kanban board in the desktop client](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/desktop-board.png)

<table>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/desktop-task.png" alt="Task detail with labels, subtasks and comments"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/desktop-calendar.png" alt="Calendar view by due date"></td>
  </tr>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/desktop-team.png" alt="Team management: roles and invitations by code"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/tui-board.png" alt="Terminal client (ratatui)"></td>
  </tr>
</table>

### Web client (also inside the desktop app)

`tm-server` serves the interface at `/`: open `http://your-server:7878` and you are done.

<table>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-gantt.png" alt="Gantt chart with task dependencies"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-canvas.png" alt="Collaborative whiteboard with notes, task cards and connections"></td>
  </tr>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-inbox.png" alt="Inbox of news shown at sign-in"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-mindmap.png" alt="Mind map of a task"></td>
  </tr>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-settings.png" alt="Settings menu: account, Telegram, team and preferences"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-message.png" alt="Message to the team (inbox, email and Telegram)"></td>
  </tr>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-team.png" alt="Team: search registered users, generate an invitation code, members"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-activity.png" alt="Project activity"></td>
  </tr>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-invite.png" alt="Generated invitation code"></td>
  </tr>
</table>

## 🤖 Your AI agents, as team members

Coding assistants (Claude Code, Cursor, Windsurf, Zed or any [MCP](https://modelcontextprotocol.io) client) **connect to the same board as your human team**: each AI has its own user, token and role, and works as one more member.

- **Add as many AIs as you want**: in the sidebar, **🤖 AIs → «+»**. It creates the user, adds it to the team and shows you **the token once**, with the MCP configuration ready to copy. Each AI carries its **model** (e.g. `claude:Opus`) and shows up with a 🤖 everywhere users are listed.
- **They coordinate with people**: an AI can **create a task and assign it to a human** on the team, comment, complete or delete its own tasks, and receive the ones you assign. Everything is logged under its name in the activity feed.
- **Human control**: the token is **rotated or revoked with one click**; the role is viewer or member; an AI **cannot** create or delete teams, invite people, change roles or touch other people's tasks (except to comment).
- **A command board**: you assign work to your agents from the web, the desktop app or the terminal, and watch live what each one is doing.

> 🚀 **Let them work on their own**: **`tm-agent`** watches the board and, when you assign a task to an AI, opens your agent (Claude Code, Gemini CLI…) in the project's folder, moves the task to *In progress*, comments on it and leaves it in *Done*. An agent can also raise a 🙋 **help alarm** (`tm-agent ask "..."`) and wait for a human answer. Step by step in the **[agents guide](https://github.com/polblancoo/Rust_tuiManager/blob/main/docs/AGENTS.md)**; inside the app it is under *Settings → AI agents*. The `tm-mcp` binary (MCP tools) is still in development (see [ROADMAP.md](ROADMAP.md)).

## What's new in 0.6.3

### 💻 Terminal in the browser and Terminals view
A **💻 Terminal** button on every task (owners only) opens a console in the project folder. Two ways: **through the agent** (`TM_AGENT_TERMINAL=1`, the connection starts from the project machine: no keys, no open ports) or **SSH with a key** (the app generates the key and pins the host fingerprint). Each session leaves a `.tm/terminal-tarea-N.md` note the AI reads when it resumes. The **🖥️ Terminals** view shows up to 8 panes and each AI's state.

![Terminals view](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-14-terminales.png)

### 💰 Usage and quotas
Tokens, cost and hours per task, project, team and AI, with **quotas** checked before every run and a token **estimate** (🧮) to set the task cap.

![Usage](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-15-consumo.png)
![Token cap on the task](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-17-tokens.png)

### ✅ Allow commands from the task
When an AI asks for a command, **✅ Allow command…** explains what it does and authorizes it **once** or **always** (or with `/permitir` in a comment).

![Allow command](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-16-permitir.png)

### License
From 0.6.3 the project is **AGPL-3.0-only** (earlier versions stay MIT). It bundles xterm.js (MIT).

## What's new in 0.6.2

### 📊 Portfolio: all your projects on one screen
Progress, overdue work, resources, who works on what and which project is at risk (🟢 on track · 🔴 overdue · 🟡 stalled for 7 days · ✅ finished). Click a card to open the project; it updates live.

![Portfolio with all projects](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-11-portafolio.png)

### 🙋 When an AI needs you, it asks, and you answer in the same message
If an AI hits something it cannot do alone (a blocked command, a permission), it runs `tm-agent ask`: the server creates a **request task** for an authorized person, with the command ready to copy (**📋 Copy**). The task opens with a panel: copy each command, run it in your terminal, paste the output in the box and press **📤 Send** (or **✅ Authorize / ❌ Reject** for a decision): it reaches the AI on its own. Who gets it: the task's creator if they have the new **Executor** permission, otherwise the executor with the fewest open tasks, otherwise an admin. Each AI also has an editable list of **🚫 forbidden commands**.

![An AI's request: task created and command to copy](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-12-pedido.png)

### 📖 Manual inside the app, tooltips and three languages
A manual with **search and a topic index** (button **📖 Manual**), a tooltip on every button, and the whole interface in **Spanish, English and Portuguese** (Settings → Preferences → Language). The task detail can be **maximized** to read long comments.

![The manual inside the app, with search](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-13-manual.png)

### Also new
- Numbered tasks (`#12`) and **manual ordering by drag and drop**.
- **Terminal (TUI)** in three languages.
- **Server email**: guide for Gmail, Outlook, Brevo, SendGrid, Mailgun, SES or your own server.
- CI on Linux, Windows and macOS. Downloads: **Linux available**, **Windows and macOS coming soon**.
- A website: **https://polblancoo.github.io/rust_tuiManager/**

## What's new in 0.5

<table>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-google.png" alt="Settings with Google Calendar connected and a “?” help button on every row"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-google-help.png" alt="Google Calendar help: what it does and what the server admin configures"></td>
  </tr>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-config-help.png" alt="Help for a row: “If you are a user” and “If you run the server”"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-invite.png" alt="Invitation code with “Copy invitation” and “Send by email”"></td>
  </tr>
</table>

- **Team panel** («👥 Team» button, right column): members with their role (editable in place), since when they are in, who added them and whether they joined with a code, and how many open tasks each one has; invitations with their uses, expiry and **who joined with each code**; and team settings (edit name and description, transfer ownership, leave or delete the team with name confirmation).

<img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-team-panel.png" alt="Team panel: members, invitations and settings">

- **Google Calendar** (optional, one-way): each user connects their account from *Settings → Google Calendar* and their dated tasks (assigned to them or unassigned, from their teams) show up as all-day events in their main calendar. If they change, finish or are deleted, the event is updated or removed automatically (every minute); “Disconnect” cleans up whatever the app created. **The server admin enables it once** with `TM_GOOGLE_CLIENT_ID`, `TM_GOOGLE_CLIENT_SECRET` and `TM_GOOGLE_REDIRECT` (a Google Cloud “Web application” OAuth client); while the Google app is in *Testing* mode only the Gmail accounts added as test users can connect.
- **Help on every Settings row**: a “?” button per row explains what it does for the regular user and what must be configured if you run the server (SMTP, Telegram, Google, HTTPS…).
- **Ready-to-share invitations**: when you generate a code, *Copy invitation* builds a text with the install link, the server address, the steps and the code, and *Send by email* opens it in your mail client.
- **Clearer user search**: results appear as buttons (also in the desktop app) and match anywhere in the name.
- **Tidier team dialog**: add and invite at the top, member list at the bottom.
- The news window no longer pops up at every start if there is nothing unread.

## Features

- **Teams** with roles (viewer, member, admin, owner) and **invitations by code** with expiry and a usage cap.
- **Projects and kanban board** (to do / in progress / done), priorities, due dates and assignment. Each project stores its **working folder and repository**; admins can **edit or delete** it (deletion asks you to type its name).
- **Labels** per team and **subtasks** with a progress bar.
- **Comments** on every task.
- Monthly **calendar** by due date, **Gantt** with task dependencies (drag the bars to move dates), a collaborative **whiteboard** per project (notes, task cards and connections) and a **mind map** of each task (graphical client).
- **Assignee**: the lead (admin or owner) assigns to any member; a member can only take free tasks or release their own.
- **Inbox**: at sign-in you see what happened while you were away (assigned to you, comments, added to a team) and your open tasks; it arrives live and, if you add and verify your email, also by mail.
- **Web client**: `tm-server` serves the same graphical interface at `/` (open `http://your-server:7878` in the browser, nothing to install; `TM_WEB=0` turns it off).
- **Mentions**: write `@user` in a comment and that person gets a notice (inbox, email and Telegram).
- **Due-date reminders**: the assignee gets a notice when less than 24 hours are left and another, only once, when the task is overdue.
- **Recurring tasks** (daily, weekly or monthly): completing one creates the next, with the date shifted, the labels and the pending subtasks.
- **Attachments** (up to 10 per task, 2 MB each): the server always delivers them as downloads. The web shows thumbnails and previews of images (PNG, JPEG, GIF, WebP) and plain text, checking the real content of the file; never HTML, SVG or PDF.
- **Templates**: duplicate a project with its tasks, labels and dependencies (everything comes back pending and unassigned).
- **Export** a project's tasks to CSV or JSON (the CSV neutralises spreadsheet formulas).
- **Activity** per project: who created, moved, assigned, commented or attached, and when.
- **Installable web (PWA)**: from the phone or desktop browser you can “install” it as an app.
- **Telegram** (optional): each user links their account to the bot and receives updates there; any member can send **messages to the team** (inbox, email and Telegram).
- **Add members with search** among registered users, `Ctrl+T`-prefixed shortcuts and **searchable help** (`?`) in both clients.
- **Google Calendar** (optional): your dated tasks appear in your calendar; see *What's new in 0.5*.
- **AI agents** with their own account, model and token, and `tm-agent` to make them work on their own.
- **Real time**: a change by any user appears instantly in every client of the team.

## Quick install

**Download**: [latest release](https://github.com/polblancoo/Rust_tuiManager/releases/latest) (Linux: `.deb`, `.rpm`, AppImage and `tar.gz`; **Windows and macOS coming soon**) · Website: https://polblancoo.github.io/rust_tuiManager/


```sh
cargo install rust-tui-manager-server   # installs the `tm-server` binary
cargo install rust-tui-manager          # terminal client
cargo install rust-tui-manager-agent    # installs `tm-agent` (optional, for AI agents)
```

Or from this repository:

```sh
cargo run -p rust-tui-manager-server                          # server on 0.0.0.0:7878
cargo run -p rust-tui-manager -- http://127.0.0.1:7878        # TUI client (F2 = create account)
cargo run -p tm-desktop                                       # desktop client
```

The desktop client needs the Tauri dependencies (on Linux: `webkit2gtk-4.1`, `libsoup-3.0`).

## Server configuration

| Variable | Default | Description |
|---|---|---|
| `TM_ADDR` | `0.0.0.0:7878` | Listening address |
| `TM_DATABASE` | `sqlite://tm-server.db` | Database (created with `0600` permissions) |
| `TM_JWT_SECRET` | generated and stored in the database | Token secret; at least 32 characters |
| `TM_CORS_ORIGINS` | Tauri and localhost | Extra allowed origins, comma-separated |
| `TM_TLS_CERT` / `TM_TLS_KEY` | — | PEM certificate and key to serve HTTPS directly |
| `TM_TRUSTED_PROXY` | — | `1` if it runs behind your own reverse proxy (uses `X-Forwarded-For` for rate limiting) |

For email notices (optional; without this only the inbox works):

| Variable | Description |
|---|---|
| `TM_SMTP_URL` | SMTP server, e.g. `smtps://smtp.example.com:465` or `smtp://127.0.0.1:2525` |
| `TM_SMTP_FROM` | Sender, e.g. `Tasks <notices@example.com>` |
| `TM_SMTP_USER` / `TM_SMTP_PASSWORD` | Credentials, if the server asks for them |

For Telegram, create a bot with [@BotFather](https://t.me/BotFather) and give the token to the server:

| Variable | Description |
|---|---|
| `TM_REMINDER_SECS` | How often due dates are checked, in seconds (default 300) |
| `TM_TELEGRAM_TOKEN` | Bot token. The server serves it by *long polling*: no public URL or own HTTPS needed |
| `TM_TELEGRAM_SEND_ONLY` | `1` to only send notices (no `/start` linking, no polling); lets you share a bot with another program |
| `TM_GOOGLE_CLIENT_ID`, `TM_GOOGLE_CLIENT_SECRET`, `TM_GOOGLE_REDIRECT` | Google Cloud OAuth client that enables Google Calendar sync (redirect: `https://your-server/api/google/callback`) |

Each user links it in **Settings → Telegram** (web/desktop) or with `Ctrl+T g` (TUI): it generates a single-use code (15 min) and they send it to the bot as `/start CODE`; `/stop` unlinks. Once linked, the bot also answers: `/tareas` (your open tasks) and `/hecho N` (closes task N if it is yours, or any if you are an admin). Messages are only sent to chats linked this way, never to arbitrary ids.

Each user adds their email and confirms it with a 6-digit code; notices are only sent to verified addresses (with an hourly cap).

Without its own TLS, put it behind a TLS proxy before exposing it to the internet.

To keep the server always on, run it as a service (e.g. a systemd user unit with `ExecStart=~/.local/bin/tm-server` and `Restart=on-failure`).

## Security

- Passwords with argon2; 7-day JWT tokens that can be revoked with “sign out of all devices”.
- Per-account lockout (5 failures = 15 minutes) and per-IP limiting on login, registration and WebSocket.
- Limits on body size, field length and count per resource on every endpoint.
- Restricted CORS, security headers and a strict CSP in the Tauri app.
- The WebSocket token does not travel in the URL: it is sent as the first message.
- Invitation codes are stored hashed and shown only when created.
- AI accounts use their own revocable tokens (`tmk_…`, stored hashed) and cannot administer teams, roles or invitations.

## Roles

| Role | Can |
|---|---|
| Viewer | See everything |
| Member | Create and edit tasks, subtasks, comments, labels and projects |
| Admin | Also add or remove lower-rank members, change roles, invite, and edit or delete projects |
| Owner | Everything; cannot leave the team |

Nobody can grant a role equal to or higher than their own.

## Structure

```
crates/core      shared models and events
crates/server    axum + SQLite + WebSocket server
crates/tui       terminal client
crates/agent     tm-agent: puts an AI to work on assigned tasks
apps/desktop     Tauri client (static frontend in ui/)
legacy/          previous local manager (0.1.x)
```

## License

**AGPL-3.0-only.** Free to use and to self-host. If you offer a modified version as a service, you must publish your changes under the same license. A commercial license without that obligation is available: ask in [the issues](https://github.com/polblancoo/Rust_tuiManager/issues). Versions published up to 0.6.2 remain under MIT.
