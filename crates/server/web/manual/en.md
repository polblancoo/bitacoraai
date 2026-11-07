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
13. [Allow commands from the task](#13-allow-commands-from-the-task)
14. [Terminal in the browser](#14-terminal-in-the-browser)
15. [Usage and quotas](#15-usage-and-quotas)
16. [Commercial license and install ping](#16-commercial-license-and-install-ping)

---

## 1. Install the server

The server stores everything (teams, tasks, users) and serves the web app. You install it once.

1. Download `tm-server-linux-x86_64.tar.gz` from the [latest release](https://github.com/polblancoo/crewhelm/releases/latest) (or `cargo install crewhelm-server`).
2. Unpack it and run `./tm-server`.
3. Open `http://localhost:7878` in your browser.

For other people to join from other machines, put the server at an address they can reach (ideally with HTTPS: see the [server configuration](https://github.com/polblancoo/crewhelm#readme)). If you are only trying it out, `localhost` is enough.

**One-line install (Linux x86_64, no sudo):** `curl -fsSL https://raw.githubusercontent.com/polblancoo/crewhelm/main/install.sh | sh`. It downloads the server from the release, **verifies its SHA256** (no match, no install), puts it in `~/.local/bin` and creates a user service that starts by itself. Options: `--port N`, `--no-service`, `--with-agent`, `--with-claude`, `--with-gemini`, `--with-codex`, `--with-opencode` (install that AI's program; `--with-all-ais` all four), `--version X`. The first time you sign in, a **start guide** with drawings walks you through team, project and AI (repeat it from Settings → Preferences → 🧭 Start guide).

If you prefer a terminal or a window of its own, there is also a terminal client and a desktop app on the same downloads page.

## 2. Create your account and your team

1. On the start screen choose **Create account** (username and a password of 8 characters or more).
2. In the sidebar, **Teams → «+»**, enter your organization's name (for example «Mi empresa»). You become the **Owner**.

A **team** is the group of people and AIs who share projects. You can have several.

## 3. Create the projects

In **Projects → «+»** create one for each thing you want to move forward. In the example: an app, a spreadsheet and a process, each with its own color.

![Board of the project «Reservations mobile app»](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-01-tablero.png)

Each project has its own **board** (To do · Doing · Done). With **+ New task** you load the work: title, description, priority, dates and owner. The same data also shows up in **calendar**, **Gantt** and **whiteboard** views.

![Another project: a budget spreadsheet, with human and AI owners](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-08-otro-proyecto.png)

Each project keeps **where its work lives**: with **⚙ Project** (admin or owner) you set the work folder and the repository, and they are shown above the board.

![Edit the project: name, work folder and repository](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-09-proyecto.png)

> 💡 Tip: it does not have to be software. A spreadsheet, an onboarding process or an audit are handled the same way: tasks with an owner and a date.

## 4. Add people

**👥 Team** button (top right) → **➕ Add / invite**.

- **Already has an account**: type part of their name, choose the role and **Add**.
- **Does not have an account yet**: choose a role and **Generate code**. Use **Copy invitation** (text ready for WhatsApp or email with the download link, server and code) or **Send by email**.

![Invite with a code](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-07-invitar.png)

| Role | What they can do |
|---|---|
| **Viewer** | Look. |
| **Member** | Create, edit and comment on tasks; take unassigned ones. |
| **Admin** | Also assign tasks to others, add people and generate invitations. |
| **Owner** | Everything, including transferring or deleting the team. |

The team panel shows each member with their role, since when they have been in, who added them and how many open tasks they have:

![Team panel with people and AIs](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-06-panel-equipo.png)

## 5. Add AIs

AIs (Claude Code, Cursor, Windsurf, Zed, Gemini or any other client compatible with [MCP](https://modelcontextprotocol.io)) work **as one more team member**. As many as you need, from different models.

1. In the sidebar, **🤖 AIs → «+»**.
2. Enter a name that tells you what it is (`claude-backend`, `gemini-data`) and its role: **Member** (creates and edits its own tasks) or **Viewer** (only looks).

   ![New AI in the team](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-02-nueva-ia.png)

3. **Its token is shown only once**, with the MCP configuration ready to copy. It is that AI's «key».

   ![AI token and MCP configuration](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-03-token.png)

4. If you lose it or want to cut it off: click the AI → **New token** (the old one stops working) or **Revoke**.

   ![An AI's card](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-03-ia.png)

**What an AI can and cannot do**

- ✅ Create tasks and **assign them to people** on the team, comment, complete or delete **the ones it created or was assigned**.
- ❌ Create or delete teams, invite people, change roles, or touch other people's tasks (except commenting).
- Everything it does is recorded **under its name** in the activity feed, and it is shown with 🤖 in the panel.

> 🚧 **Status**: AI accounts and their tokens already work against the server API. The `tm-mcp` binary and the *skill* that connect them directly to your coding platform are in development (see the [ROADMAP](https://github.com/polblancoo/crewhelm/blob/main/ROADMAP.md)). Meanwhile, **`tm-agent`** already lets an AI pick up its assigned tasks on its own: see the [agents guide](https://github.com/polblancoo/crewhelm/blob/main/docs/AGENTS.md).

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

![Project Gantt, with tasks from people and AIs](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-04-gantt.png)

![Activity: the project history, with the AIs' actions under their own names](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-05-actividad.png)

In addition, **Google Calendar** (your tasks with a date appear in your calendar) is connected from **Settings**.

## 8. Delegate to a lead

Today, to delegate a project: add the person with the **Admin** role (they can assign tasks, add people and generate invitations) and assign them the main tasks. They build their part and bring in their people.

> 🚧 **Coming soon**: **lead per project** (scoped to that project only), **configurable scope per AI** (which projects it touches, whether it may invite other AIs) and **progress reports** with charts. See the [roadmap](https://github.com/polblancoo/crewhelm/blob/main/ROADMAP.md).

## 9. Run several projects at once

**📊 Portfolio** is the first button in the *Projects* list. It shows every project of the team on one screen:

- **Top**: projects, overall progress, tasks in progress, overdue and projects at risk.
- **One card per project**: progress bar, who has open tasks (AIs marked 🤖), last activity, next due date and how many actions the AIs took in 24 h.
- **Status**: 🟢 on track · 🔴 overdue tasks · 🟡 no movement for 7 days · ✅ finished · ⚪ no tasks.
- **Bottom**: the latest things the AIs did. Click a card to open the project; everything updates live.

![Portfolio with all projects](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-11-portafolio.png)

**Numbered tasks.** Every task shows its number (`#12`) on the card and in the detail, so you can refer to it unambiguously in messages and comments.

**Manual order.** Drag a task inside its column to change the order it gets done; your order wins over priority. You can also pick the assignee when creating it.

## 10. When an AI needs you to do something

An AI may hit something it cannot do alone: a blocked command, a permission, a decision. It then **asks you** and waits for your answer:

1. The AI runs `tm-agent ask "…"`. A 🙋 comment appears on its task and a **request task** («🙋 Request from #12: …») is created and assigned to a person. The comment tells you which one.
2. Open the request task: at the top is the **🙋 Request from an AI** panel with **each command on its own line and a 📋 Copy button**. Paste it in your terminal and run it.
3. **Where does the output go?** In the **«Paste the output here»** box of the same panel, then **📤 Send output to the AI**. It is posted as a comment on that task and **reaches the AI automatically** (it also shows on its original task). Writing a normal comment on the request task does the same.
4. If the request is a **decision or a permission** («may I delete X?»), use **✅ Authorize** or **❌ Reject** (with the reason in the box). For a command the AI has blocked, «authorize» is not enough: run it yourself and send the output.
5. The AI reads it on its next poll (every 10 s) and carries on. Sending marks the request as Done.

![An AI's request: task created, command to copy](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-12-pedido.png)

**Who gets it?** Members with the **Executor** permission (team window → *Executor* checkbox next to each person; only an admin changes it, and AIs cannot be executors):

1. whoever created the task, if they are an executor;
2. otherwise (e.g. another AI created it), the executor with the fewest open tasks;
3. with no executors, a team admin.

**Forbidden commands.** On each AI's card, **🚫 Forbidden commands** (one per line, e.g. `Bash(rm:*)`). The AI cannot run them: if it needs one, it asks you as above.

## 11. Language, tooltips and this manual

- **Language**: *Settings → Preferences → Language* (Spanish, English or Portuguese). By default it follows your browser.
- **Tooltips**: hover over a button to see what it does.
- **This manual** lives inside the app: **📖 Manual** button (bottom left, or *Settings → Help*). It has search and a topic index with links.

![The manual inside the app, with search](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-13-manual.png)

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

## 13. Allow commands from the task

When an AI asks for a command, open the request task and press **✅ Allow command…**. The dialog shows the command, **explains what it does** (red warning if dangerous) and lets you pick **once** or **always**. You can also write `/permitir cmd` (once) or `/permitir siempre cmd` (owners and admins only) in a comment. The AI receives the permissions in `TM_ALLOW` and resumes the task.

![Allow command](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-16-permitir.png)

## 14. Terminal in the browser

For **owners** (members use their own space, see section 21). The task's **💻 Terminal** button opens a console in the project folder. Two ways to connect it (chosen per project):

- **Through the agent** (recommended): run `tm-agent` with `TM_AGENT_TERMINAL=1` on the project machine. The connection starts from that machine: no keys, no open ports, works behind NAT.
- **SSH with a key**: the app generates an ed25519 key (**🔑 Generate key**) and installs it with **🔐 Install key with password** (the password is not stored). The server fingerprint is pinned the first time and rejected if it changes.

Each session leaves a 📝 comment and a `.tm/terminal-tarea-N.md` note in the project folder, which the AI reads when it resumes. In the menu, **🖥️ Terminals** shows up to 8 panes and each AI's state (🙋 needs you · ⚙️ working · ⏳ queued · ✅ done).

![Terminals view](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-14-terminales.png)

## 15. Usage and quotas

**💰 Usage** shows tokens, cost and hours per AI and task, with **quota** bars for team, project and task. On a task, **🪙 Tokens to spend** sets a cap and **🧮 Estimate** suggests a value from history. The quota is checked **before** each run: if it is short, the AI does not start and says so in a comment. To cut off in dollars use `TM_AGENT_MAX_USD` on the agent (1 USD or more). The Portfolio adds up the month's usage.

![Usage](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-15-consumo.png)
![Token cap](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-17-tokens.png)

## 16. Commercial license and install ping

The program is **AGPL-3.0-only and free**. The **commercial license** is for those who use it in a company and don't want to comply with the AGPL. In **Settings → 🏢 License** (first registered person on the server only):

1. Choose whether the server is **personal** or **company**.
2. **Buy a commercial license** → pick the plan, the network (**USDT on Solana or Ethereum**) and your email.
3. Pay **exactly** the amount shown (the last decimals identify your payment) to the given address; the QR carries it. Check the **token contract** with the explorer link.
4. The app checks the payment by itself. When confirmed you get an email with the code `TM-XXXX-XXXX-XXXX` and the license activates itself. Otherwise paste the code under **Already have a code?**.

The **bar** on top appears with more than 3 people or 2 AIs, or if you declare it a company; it can be dismissed for 7 days and never blocks anything.

**Install ping.** Once a day the server sends a random code, the version, the operating system and how many people, AIs, projects and teams there are. Never names, emails, tasks or content; the IP is not stored. Turn it off with `TM_TELEMETRY=0` or from the License window.

### Plans and prices (per year)

| Plan | Includes | List price | Founder price |
|---|---|---|---|
| **Free** (AGPL) | up to 3 people and 2 AIs, no notice | USD 0 | — |
| **Team** | up to 10 people and 10 AIs | **USD 290** | **USD 290 for life** (first 5) |
| **Company** | up to 50 people and 50 AIs, with support | **USD 990** | — |

- **🎖 Founder price:** the **first 5 customers** pay the founder price and keep it **for life** on renewals. The app shows it by itself while spots remain.
- **If you grow:** if, with a valid license, you go over your plan's people or AIs, the app tells you and **⬆ Upgrade plan** appears: you pay only the **difference** between the new plan's price and what you already paid, and your expiry date does not change.
- **Terms:** before paying you read and accept the **commercial license terms** (in your language). We keep the version you accepted and the date. Among other things, the terms state that every command or permission you approve for an AI is your responsibility and that the software comes without warranties.
- Prices are confirmed on the purchase screen: what you see there is what is charged.


---

**Something not clear?** Every option in **Settings** has a «?» with help. And if you find a problem, open an [issue](https://github.com/polblancoo/crewhelm/issues).

## 17. Interface news (0.7.1)

- **CrewHelm.** The app is now called **CrewHelm** (interface, emails and manifest). Programs and services keep their names for now.
- **Start guide.** A 5-step assistant with illustrations: welcome, team, project, AI (optional) and a sample task. It opens by itself if you have no teams; on an empty server the sign-in screen offers **Create account** directly.
- **AI requests, faster.** The commands the AI asks for appear together in one list: **📋** copies, **✕** removes the ones you don't want and **✅ Allow all N commands always** adds them all to the AI's list, tells it and marks the request Done. **⚙ AI list** opens the allowed list so you can delete or edit lines (saved instantly). A command can be up to 1000 characters long.
- **A friendlier task window.** **💾 Save** and **🗑 Delete** are also at the top, in a sticky header, and the bottom bar stays pinned. **⤢ Maximize** gives you bigger comments.
- **Task terminal.** A single terminal fills the screen and, if the machine has `herdr`, it opens by itself (turn off with `localStorage autoHerdr = "0"`). `~/folder` works as a working directory.
- **Expandable inbox.** Enlarge it with **⤢**; tapping an item opens **its** task.
- **Clickable portfolio.** In "Latest from the AIs", a row with **💬 ver** opens the task and the whole conversation.
- **Settings in tabs**, with help under each line.
- **Text zoom.** Bottom right: **A−**, the percentage (click = 100%) and **A+**; also **Ctrl + Alt + wheel**. Remembered per device.
- **Our own confirmations.** Deleting, leaving a team or disconnecting an account asks in an app window (Esc cancels), not the browser's alert.

### Usage profile (Settings → Team and project)

Pick what your team uses CrewHelm for and the interface sets itself up: *Software development* (To do / In progress / Done, with terminals and code folder), *Content and social* (Draft / In production / Published, calendar as the starting view, no technical parts), *App factory* (Backlog / Building / Delivered, several projects in parallel), *Office and professional practices* (administration, accounting, law: Pending / In process / Closed, with the deadlines calendar) or *Other / general team*. «Create example» builds a **separate example team** («Ejemplo · …») with sample projects and tasks: your real team is untouched, you return to it with «Volver a mi equipo» and «Eliminar ejemplo» deletes the whole example team. It only changes names and what is shown: your data is untouched and you can switch profile any time. It can also be chosen in the start guide.

If the team handles confidential information (for example a law firm), turn on the project's **human review**, don't load data you don't want an AI to see, and remember that task text reaches whichever model you use; for very sensitive data, use a company-owned model.

### GitHub per project (🐙 GitHub button, top bar)

1. In **⚙ Settings → Alerts and connections → GitHub** connect your account (once). The server admin must have set `TM_GITHUB_CLIENT_ID`.
2. In **⚙ Project details** enter the repository (`owner/repo` or the URL). Then click **🐙 GitHub** (top bar, next to ⚙ Project): list the open issues and **import the ticked ones as tasks**.
3. **GitHub settings for this project** has one checkbox per feature, each switching on or off by itself: **check every 5 minutes** (moves a task to *Done* when its issue is closed or its Pull Request is merged, and shows the PR state), **sync comments** between the task and the issue (on a public repository anyone can read them), **publish new tasks as issues**, **notify when an AI finishes** so you open the Pull Request, **CI status** with ✅/❌ next to the button, **log commits** in Activity, and **webhook** (instant updates; the server must be reachable from the internet; the box shows the URL and the secret to paste in GitHub).
4. In a task: **🐙 Publish as issue**, **Open Pull Request** (from branch `tm/tarea-N` or another you choose; it is always your decision) and **Close issue**.

GitHub and Google tokens are stored **encrypted** (AES-256-GCM) in the database; the key lives in `~/.config/tm-server/credenciales.key` (or `TM_KEY_FILE`). Back it up with your other backups: without it you must reconnect the accounts.

## 18. Communication: chat, video calls and translation
The **💬 Communication** button opens a panel with two tabs.

- **💬 Chat:** messages to the team, a project or a person, with files. Links open with one click. Everything is stored encrypted and only works over a secure connection (HTTPS).
- **📹 Video call:** *Enter the room* (the team's permanent room), *Start a call* (creates a new room and posts a «Join» button in the conversation, visible only to its participants) and *Invite someone from outside*.
- **Inviting someone from outside:** creates a unique link with an expiry (1 hour to 7 days) for a supplier or client. No account is needed and they see nothing else. Copy it or send it by email (needs the server's email, section 12). You can cancel it at any time.
- **Setting up video calls (administrator):** *Settings → 🔌 Integrations → Manage*. The wizard offers three paths: public Jitsi, your own Jitsi on the internal network, or your own Jitsi open to the internet, with a ready-to-paste install block. With the public `meet.jit.si` an embedded call is cut after 5 minutes, so it opens in its own tab.
- **Chat translation (administrator):** the **⚙🌐** button turns it on. Everyone writes in their own language and reads in theirs. Pick the engine: *your own LibreTranslate* (free, nothing leaves your network; the «Install and enable automatically» button installs it with Docker) or *a team AI* (with its daily spending cap). Each person turns it on or off with the **🌐** button. When translating, the text goes to the chosen engine.
- **Minimizing the call:** it becomes a small bar at the bottom right with «▲ Back to the call».

## 19. Task folders
Each task has its own working folder shaped like `.crewhelm/<user>/<task>/`: every person or AI keeps their own work there. Final results are published to `final/`, shared by the whole task.

- In the task, the **Folder** block lets you view, download and upload files (up to 25 MB; executables and scripts are rejected).
- **Whoever creates the task decides approval** and publishing to `final/`, for a loose task or a flow.
- Remote workers can upload their files from there.

## 20. Server roles and Processes
Besides each team's roles there are three **server roles**: *User*, *CTO* and *Administrator*. The first person to register is the administrator. An administrator changes them in *Settings → 🛡 Roles and limits* (at least one administrator must always remain). The CTO also sets the terminal limits there and sees usage per person (section 24).

- **🖥 Processes** (CTO and administrator): RAM, disk, load, CrewHelm processes with their CPU and RAM, containers and app usage (database, files, people, AIs, tasks). It refreshes every 5 seconds.
- **Overload protection:** a per-IP per-minute request cap, a cap on simultaneous requests and a per-request time limit. A large network attack is stopped outside (firewall, proxy or CDN). Behind a proxy, set `TM_TRUSTED_PROXY=1`.
- **Team shortcuts:** the prefix on the web is **Alt+K** (the browser keeps Ctrl+T).

## 21. People's terminal: working copy and git bar
When an owner turns on **“Allow the terminal for team people”** (🖥️ Terminals → ⚙ Connection), each member opens their terminal in a sandbox:

- **Personal space** (`~`): theirs, writable, kept between sessions. Nobody else sees it (owners can look at it read-only with **👁 View space of…**).
- **Working copy** (`~/<project>`): their own copy of the project where they can create folders and files and use editors. The original stays at `/proyectos/<project>`, read-only. The copy **updates itself** on entry.
- **“🧭 Your work” bar** under the terminal, made for people who don't code. Each button shows the equivalent git command in its tooltip: *Update*, *New branch*, *Save*, *Send for review*, *My branches*, *Switch branch*.
- **`cw` command** (the same, typed): `cw estado`, `cw actualizar`, `cw rama`, `cw guardar`, `cw enviar`, `cw ramas`, `cw cambiar`, `cw espacio` (how much space you use) and `cw ayuda`. Normal order: **branch → edit → save → send**.
- **Send for review** pushes the branch to GitHub and opens the Pull Request **without the person needing a GitHub account**: the server does it with the owner's credentials. Each person's branches start with their name (`marta/topic`); others' branches can't be touched or force-pushed.
- If the project **has no repository**, the bar says so and offers **🙋 Ask for a repository**, which notifies the owners.

## 22. Project repository (git assistant)
An owner connects the repository from **🐙 Repository** (in 🖥️ Terminals), from the **⚠ No repository** warning in the project header or, when creating a project, in the window that opens by itself. The assistant offers:

- **⬇ Bring a GitHub repository** into the project folder.
- **✨ Create a new one** in your GitHub account (private by default) and upload what is in the folder, without keys or heavy folders (`.gitignore` included).
- **🔗 Connect** a folder that is already a local repository to one on GitHub.
- **💻 Only on this machine**: `git init` without GitHub; it can be connected later.

Your GitHub account must be connected (Settings → Alerts and connections) only during the operation. To push changes the server uses a **deploy key per repository**; you can renew it with 🔑. Each project can have its own repository, local or on GitHub.

## 23. Integrate: review and merge the team's work
The **🔀 Integrate** button (project header) lists open Pull Requests: who sent it, branch, files, `+/−` lines and status (ready, conflicts, failing tests, blocked).

- **👁 View changes**: file by file, additions in green and deletions in red.
- **✅ Integrate** (owners only): merges the work into the main branch (option “Combine into a single change”), deletes the person's branch and **updates the project folder**; the person gets a notice and runs `cw actualizar`.
- **↩ Send back** (owners only): closes the request without merging and sends the person the reason.
- Owners get a notice when someone sends their work. Drafts and requests with conflicts are not merged.

## 24. Limits and quotas (Settings → 🛡 Roles and limits)
The **CTO** and the **administrator** set the limits of members' terminals (owners have none); each machine's agent applies them to terminals opened from then on:

- **💾 Disk per person** (5120 MB by default): when full, the terminal won't open; at 80 % it warns; past 125 % it closes. Measured every 90 seconds (not a hard cap).
- **🧠 Memory** (4096 MB), **⚙ CPU** (200 % = two cores) and **🧵 Processes** (1024): applied with Linux control groups (`systemd-run`). If a program exceeds memory, only that program is killed. Without systemd, the terminal opens without these three limits.
- **📊 View usage per person**: how much each space takes on each machine.
- With `0` in a field there is no limit. Initial values can also be set with `TM_PERSON_QUOTA_MB`, `TM_PERSON_MEM_MB`, `TM_PERSON_CPU_PCT` and `TM_PERSON_TASKS`.

## 25. Inviting and installing on another person's machine
In **👥 Team → Invite with a code**, **Copy invitation** builds a message ready to paste in a chat: the server address (with the local network IP if you came in through `localhost`), the code and the steps. The easiest way is to **open the address in the browser**, installing nothing.

For the desktop app on Linux, without a server: `curl -fsSL https://raw.githubusercontent.com/polblancoo/crewhelm/main/install.sh | sh -s -- --client --server http://IP:7878`. It installs only the app (the service and terminal tools are left out); once open, enter the address under “Server”. In Settings each person sees only what applies to them: personal GitHub only for admins (or if already linked), Google only if the server has it, and the AI command list only for the administrator.

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

![tm-agent guide inside the app](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-10-agentes.png)

## What you need

- The server running and an account that is **admin or owner** of the team.
- A machine with the **project folder** and with the agent you want to use installed and signed in (for example `claude`).
- Rust (`cargo`) to install `tm-agent`. Ready-made installers: see the [ROADMAP](https://github.com/polblancoo/crewhelm/blob/main/ROADMAP.md).

## Step by step

### 1. Create the AI

In the sidebar: **🤖 AIs → «+»**. Pick a name (`claude-backend`), its **model** (`claude:Opus`) and the **Member** role. Copy the **token** (`tmk_…`): it is shown only once. If you lose it, click the AI → **New token**.

### 2. Set the work folder

With **⚙ Project** fill in **Work folder** (for example `/home/ana/projects/app`) and the **repository**. The path must **exist on the machine where `tm-agent` runs**. If not, the agent warns with a comment on the task and does not take it (or use `TM_AGENT_DIR`).

### 3. Install `tm-agent`

On the project machine:

```bash
cargo install crewhelm-agent
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
| `TM_AGENT_TERMINAL=1` | Opens web terminals (PTY) in the project folder, connecting out to the server | — |
| `TM_AGENT_MAX_USD` | Per-run dollar cap, read by your script (e.g. `--max-budget-usd`) | — |
| `TM_ASK_TIMEOUT` | Seconds `tm-agent ask` waits for a human answer | `900` |

The command also receives `TM_TASK_ID`, `TM_TASK_TITLE`, `TM_PROJECT_ID`, `TM_URL` and `TM_TOKEN`, in case your agent wants to comment or create subtasks through the API.

**Asking a human.** Inside a task the AI can run `tm-agent ask "question"`: it raises a 🙋 alarm for the team's owners and admins (in the app and by Telegram) and waits for the answer as a comment.

When the AI asks, the server also **creates a request task** for a person (the task's creator if they have the **Executor** permission; otherwise the executor with the fewest open tasks; with no executors, an admin). The comment says which task and that you can answer right there; commands in the request show with a **📋 Copy** button. See the [guide, chapter 10](https://github.com/polblancoo/crewhelm/blob/main/docs/GUIDE.md#10-when-an-ai-needs-you-to-do-something).

**Ready-to-use example.** In [`examples/`](https://github.com/polblancoo/crewhelm/tree/main/examples) there is a `run-claude.sh` with allowed and forbidden command lists (`whitelist.txt`, `blacklist.txt`) to use as `TM_AGENT_CMD`.

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

## Common commands and full authorization

- **📋 Common commands** (AI profile, admins only): lists what that AI **asked for before** in its 🙋 requests (by how many times), plus a catalog of common commands (Git, Rust, Node, Python, files, system). Tick and press **✅ Add to allowed**. Below, a list of **dangerous** commands (`rm -rf`, `sudo`, `git push`, `git reset --hard`…) can be added to **🚫 prohibited** in one click.
- **🔒 Full authorization** (AI profile): "allow everything except this". The AI may run any command except those in **🚫 Prohibited commands** (the AI's own, the general ones and `blacklist.txt`). Prohibited always wins. It applies from the AI's next task. Review the prohibited list before turning it on.
- Works with the `run-claude.sh` script (Claude Code). If you use **your own copy** of the script, add this line before `exec claude`: `[ "$TM_ALLOW_ALL" = "1" ] && ALLOW="Bash$(printf '\t')$ALLOW"`. `tm-agent` passes `TM_ALLOW_ALL=1` when the AI has full authorization.
- **Agent files (`whitelist.txt` / `blacklist.txt`).** They apply in addition to the app's lists and cannot be deleted from the app. The AI's profile shows a box **📄 The agent's files also apply** listing them; **Move to the app** copies them into the AI's lists (🚫 and ✅, where they can be edited) and the agent stops reading the files (`TM_LISTS_FROM_APP=1`). **Use the files again** undoes it. If you use your own copy of `run-claude.sh`, take the new version from `crates/server/assets/agents/run-claude.sh`.
- **Authorize without the runaround.** A 🙋 request has **one main button**: **✅ Authorize and let the AI continue**. It permanently authorizes everything the AI asked for (commands **and tools** such as `WebSearch`, `WebFetch` or MCP ones), notifies it, and the task runs again by itself. To stop an AI from asking at all: **🔓 trusted profile** (full authorization with a short prohibited list: `sudo`, `git push`, `git commit`, `rm -rf`, `git reset --hard`, `git clean`, `systemctl`, `dd`, `mkfs`, `cargo/npm publish`). Also in the AI profile (**⚡ Trusted profile**). Note: `mcp__server__*` allows every tool of one MCP server; there is no global permission for all servers.
