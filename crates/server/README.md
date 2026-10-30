<p align="center"><b>English</b> · <a href="https://github.com/polblancoo/Rust_tuiManager/blob/main/crates/server/README.es.md">Español</a> · <a href="https://github.com/polblancoo/Rust_tuiManager/blob/main/crates/server/README.pt.md">Português</a></p>

# rust-tui-manager-server

> 🎛️ **A command board for your projects**: run several projects at once —an app factory, a spreadsheet, an onboarding, anything— with **people and AIs on the same board**. Every project knows where its work lives (folder and repo). Self-hosted.

Multi-user server (axum + SQLite + WebSocket) of the collaborative task manager [Rust TUI Manager](https://github.com/polblancoo/Rust_tuiManager). The clients are [`rust-tui-manager`](https://crates.io/crates/rust-tui-manager) (terminal) and the Tauri app in the repository.

Clients: the TUI, the desktop graphical app and the **web app this very server serves at `/`** (can be turned off with `TM_WEB=0`).

![Graphical client connected to the server](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/desktop-board.png)

![Command board: projects, AIs and tasks](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-01-tablero.png)

<table>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-gantt.png" alt="Gantt with dependencies"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-canvas.png" alt="Collaborative whiteboard"></td>
  </tr>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-inbox.png" alt="Inbox of news"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-settings.png" alt="Settings menu"></td>
  </tr>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-team.png" alt="Team: search registered users"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-activity.png" alt="Project activity"></td>
  </tr>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-invite.png" alt="Generated invitation code"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-06-panel-equipo.png" alt="Team panel with people and AIs"></td>
  </tr>
</table>

> 👉 **[Step-by-step guide: your first project](https://github.com/polblancoo/Rust_tuiManager/blob/main/docs/GUIDE.md)**, with screenshots.

## 🤖 AI agents as team members

Coding assistants (Claude Code, Cursor, Windsurf, Zed or any MCP client) can **join a human team**: each AI has its own user, token, role and model (e.g. `claude:Opus`), creates and completes tasks, **assigns them to people** and receives the ones you assign. They are added from the sidebar (**🤖 AIs → «+»**); the token is shown once and rotated or revoked with one click. An AI cannot administer teams, invite people or touch other people's tasks. Details in the [main README](https://github.com/polblancoo/Rust_tuiManager#readme); with **`tm-agent`** an AI picks up the tasks you assign on its own (see the [agents guide](https://github.com/polblancoo/Rust_tuiManager/blob/main/docs/AGENTS.md)); the `tm-mcp` binary is in development.

![AI sheet and token with ready-to-copy MCP and tm-agent configuration](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-03-token.png)

## What's new in 0.6.1

### 📊 Portfolio: all your projects on one screen
Progress, overdue work, resources, who works on what and which project is at risk (🟢 on track · 🔴 overdue · 🟡 stalled for 7 days · ✅ finished). Click a card to open the project; it updates live.

![Portfolio with all projects](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-11-portafolio.png)

### 🙋 When an AI needs you, it asks, and you answer in the same message
If an AI hits something it cannot do alone (a blocked command, a permission), it runs `tm-agent ask`: the server creates a **request task** for an authorized person, with the command ready to copy (**📋 Copy**). You run it in your terminal, paste the result as a comment and the AI carries on. Who gets it: the task's creator if they have the new **Executor** permission, otherwise the executor with the fewest open tasks, otherwise an admin. Each AI also has an editable list of **🚫 forbidden commands**.

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

- **Team panel** («👥 Team» button, right column): members with their role (editable in place), since when they are in, who added them and whether they joined with a code, and how many open tasks each has; invitations with their uses, expiry and **who joined with each code**; and team settings (edit name and description, transfer ownership, leave or delete the team with name confirmation).

<img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-team-panel.png" alt="Team panel: members, invitations and settings">

- **Google Calendar** (optional, one-way): each user connects their account from *Settings → Google Calendar* and their dated tasks (assigned to them or unassigned, from their teams) show up as all-day events in their main calendar. If they change, finish or are deleted, the event is updated or removed automatically (every minute); “Disconnect” cleans up whatever the app created. **The server admin enables it once** with `TM_GOOGLE_CLIENT_ID`, `TM_GOOGLE_CLIENT_SECRET` and `TM_GOOGLE_REDIRECT` (a Google Cloud “Web application” OAuth client); while the Google app is in *Testing* mode only the Gmail accounts added as test users can connect.
- **Help on every Settings parameter**: a “?” button per row explains what it does for the regular user and what must be configured if you run the server (SMTP, Telegram, Google, HTTPS…).
- **Ready-to-share invitations**: when you generate a code, *Copy invitation* builds a text with the install link, the server address, the steps and the code, and *Send by email* opens it in your mail client.
- **Clearer user search**: results appear as buttons (also in the desktop app) and match anywhere in the name.
- **Tidier team dialog**: add and invite at the top, member list at the bottom.
- The news window no longer pops up at every start if there is nothing unread.

## Usage

```sh
cargo install rust-tui-manager-server
tm-server                      # listens on 0.0.0.0:7878
```

The SQLite database is created automatically (with `0600` permissions) and the schema is migrated at startup, also from databases created by earlier versions. To keep it always on, run it as a service (for example a systemd user unit with `Restart=on-failure`).

## Configuration

| Variable | Default | Description |
|---|---|---|
| `TM_ADDR` | `0.0.0.0:7878` | Listening address |
| `TM_DATABASE` | `sqlite://tm-server.db` | Database |
| `TM_JWT_SECRET` | generated and stored in the database | Token secret; at least 32 characters |
| `TM_CORS_ORIGINS` | Tauri and localhost | Extra allowed origins, comma-separated |
| `TM_TLS_CERT` / `TM_TLS_KEY` | — | PEM certificate and key for direct HTTPS |
| `TM_TRUSTED_PROXY` | — | `1` behind your own reverse proxy (uses `X-Forwarded-For`) |
| `TM_WEB` | `1` | `0` turns off the web client the server serves at `/` |
| `TM_SMTP_URL` / `TM_SMTP_FROM` | — | Enable email notices (optional): `smtps://host:465`, `Tasks <notices@example.com>` |
| `TM_REMINDER_SECS` | `300` | How often due dates are checked for reminders, in seconds |
| `TM_TELEGRAM_TOKEN` | — | Telegram bot token (from @BotFather); enables account linking and Telegram notices |
| `TM_TELEGRAM_SEND_ONLY` | — | `1` to only send notices (no `/start` linking, no polling), so a bot can be shared with another program |
| `TM_GOOGLE_CLIENT_ID`, `TM_GOOGLE_CLIENT_SECRET`, `TM_GOOGLE_REDIRECT` | — | Google Cloud OAuth client (“Web application”) that enables Google Calendar; the redirect is `https://your-server/api/google/callback` |
| `TM_GOOGLE_SYNC_SECS` | `60` | How often connected users' calendars are synced, in seconds |
| `TM_SMTP_USER` / `TM_SMTP_PASSWORD` | — | SMTP credentials, if needed |
| `RUST_LOG` | `info` | Log level |

Without its own TLS, put it behind a TLS proxy before exposing it to the internet; the server warns in the log if it listens on a non-local interface without TLS or a proxy.

## API

All routes (except register and login) require `Authorization: Bearer <token>`. Errors answer `{"error": "..."}`. The token can be a user session (JWT) or an AI account token (`tmk_…`).

| Method and route | Minimum role | Description |
|---|---|---|
| `POST /api/register`, `POST /api/login` | — | Return `{token, user}` |
| `POST /api/logout-all` | — | Invalidates all of the user's tokens (people only) |
| `GET`, `POST /api/teams` | — | My teams / create a team |
| `GET`, `POST /api/teams/{id}/members` | viewer / admin | List members / add by username |
| `PUT`, `DELETE /api/teams/{id}/members/{uid}` | admin (or oneself to leave) | Change role / remove |
| `GET`, `POST /api/teams/{id}/invites` | admin | List / create an invitation |
| `DELETE /api/invites/{id}` | admin | Revoke an invitation |
| `POST /api/invites/{code}/accept` | — | Join with a code |
| `GET`, `POST /api/teams/{id}/projects` | viewer / member | Projects (with `work_dir` and `repo`) |
| `PATCH`, `DELETE /api/projects/{id}` | admin | Edit name, description, folder and repo / delete (`?confirm=<name>`) |
| `GET`, `POST /api/projects/{id}/tasks` | viewer / member | Tasks and subtasks (`parent_id`) |
| `PATCH`, `DELETE /api/tasks/{id}` | member (AI: only its own) | Edit / delete a task |
| `GET`, `POST /api/tasks/{id}/comments` | viewer / member | Comments (a comment starting with 🙋 is a help alarm for admins) |
| `GET`, `POST /api/teams/{id}/labels` | viewer / member | Team labels |
| `DELETE /api/labels/{id}` | member | Delete a label |
| `PUT /api/tasks/{id}/labels` | member | Replace a task's labels |
| `GET`, `POST /api/teams/{id}/bots` | viewer / admin | List / create AI accounts (the token is returned once) |
| `POST`, `DELETE /api/bots/{uid}/token` | admin | Rotate / revoke an AI's token |
| `PATCH /api/bots/{uid}` | admin | Change the AI's model label |
| `GET /ws` | — | Real-time events WebSocket |

Other endpoints: `GET /api/me`, `PUT/DELETE /api/me/email`, `POST /api/me/email/verify`, `GET /api/users/search?q=&team_id=` (team admin only), `POST /api/me/telegram/link`, `DELETE /api/me/telegram`, `POST /api/teams/{id}/messages` (member or above), `GET/POST /api/tasks/{id}/attachments` (binary body, `?name=`), `GET/DELETE /api/attachments/{id}`, `POST /api/projects/{id}/duplicate`, `GET /api/projects/{id}/export?format=csv|json`, `GET /api/projects/{id}/activity`, `GET /api/teams/{id}/overview`, `PATCH /api/teams/{id}`, `POST /api/teams/{id}/transfer`, `DELETE /api/teams/{id}?confirm=<name>`, `GET /api/inbox`, `POST /api/notifications/read`, `PUT/DELETE /api/tasks/{id}/deps/{dep}`, and the whiteboard: `GET /api/projects/{id}/canvas`, `POST .../canvas/items|links`, `PATCH/DELETE /api/canvas/items/{id}`, `DELETE /api/canvas/links/{id}`.

**AI accounts** cannot create or delete teams, invite people, change roles, rotate tokens or sign out other sessions; they can create tasks (and assign them to people) and only modify or delete the tasks they created or were assigned.

### WebSocket

Connect to `/ws` and, within the first 5 seconds, send a text message `{"token": "<jwt>"}`. Then you receive JSON events `{"type": "...", "data": {...}}` from the teams you belong to: `task_created`, `task_updated`, `task_deleted`, `comment_added`, `project_created`, `project_deleted`, `label_created`, `label_deleted`, `member_added`, `member_removed` and `member_role_changed`. At most 8 connections per user.

## Limits

64 KB body; username 3–32 characters, password 8–128; title 200, description 10,000, comment 4,000; up to 50 teams per user, 100 members and 200 projects per team, 5,000 tasks per project, 100 subtasks per task, 2,000 comments per task, 100 labels per team (10 per task) and 20 active invitations per team.

## Google Calendar

Each user connects their account from Settings → Google Calendar and their dated tasks (assigned to them or unassigned, from their teams) show up as all-day events in their main calendar; they are updated or removed automatically. Sync is one-way. The server admin configures it once: create a project in Google Cloud, enable the *Google Calendar API*, set up the consent screen (with the test Gmail accounts, up to 100, or Google verification to open it up) and create an OAuth client of type “Web application”; the secret is not shared or pushed to git.

## License

MIT
