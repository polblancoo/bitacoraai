<p align="center"><b>English</b> · <a href="https://github.com/polblancoo/Rust_tuiManager/blob/main/docs/GUIA.md">Español</a> · <a href="https://github.com/polblancoo/Rust_tuiManager/blob/main/docs/GUIA.pt.md">Português</a></p>

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

1. The AI runs `tm-agent ask "…"`. A 🙋 comment appears on its task and a **request task** («🙋 Request from #12: …») is created and assigned to a person. The comment tells you which one.
2. Open the request task: at the top is the **🙋 Request from an AI** panel with **each command on its own line and a 📋 Copy button**. Paste it in your terminal and run it.
3. **Where does the output go?** In the **«Paste the output here»** box of the same panel, then **📤 Send output to the AI**. It is posted as a comment on that task and **reaches the AI automatically** (it also shows on its original task). Writing a normal comment on the request task does the same.
4. If the request is a **decision or a permission** («may I delete X?»), use **✅ Authorize** or **❌ Reject** (with the reason in the box). For a command the AI has blocked, «authorize» is not enough: run it yourself and send the output.
5. The AI reads it on its next poll (every 10 s) and carries on. Sending marks the request as Done.

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

## 13. Allow commands from the task

When an AI asks for a command, open the request task and press **✅ Allow command…**. The dialog shows the command, **explains what it does** (red warning if dangerous) and lets you pick **once** or **always**. You can also write `/permitir cmd` (once) or `/permitir siempre cmd` (owners and admins only) in a comment. The AI receives the permissions in `TM_ALLOW` and resumes the task.

![Allow command](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-16-permitir.png)

## 14. Terminal in the browser

Owners only. The task's **💻 Terminal** button opens a console in the project folder. Two ways to connect it (chosen per project):

- **Through the agent** (recommended): run `tm-agent` with `TM_AGENT_TERMINAL=1` on the project machine. The connection starts from that machine: no keys, no open ports, works behind NAT.
- **SSH with a key**: the app generates an ed25519 key (**🔑 Generate key**) and installs it with **🔐 Install key with password** (the password is not stored). The server fingerprint is pinned the first time and rejected if it changes.

Each session leaves a 📝 comment and a `.tm/terminal-tarea-N.md` note in the project folder, which the AI reads when it resumes. In the menu, **🖥️ Terminals** shows up to 8 panes and each AI's state (🙋 needs you · ⚙️ working · ⏳ queued · ✅ done).

![Terminals view](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-14-terminales.png)

## 15. Usage and quotas

**💰 Usage** shows tokens, cost and hours per AI and task, with **quota** bars for team, project and task. On a task, **🪙 Tokens to spend** sets a cap and **🧮 Estimate** suggests a value from history. The quota is checked **before** each run: if it is short, the AI does not start and says so in a comment. To cut off in dollars use `TM_AGENT_MAX_USD` on the agent (1 USD or more). The Portfolio adds up the month's usage.

![Usage](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-15-consumo.png)
![Token cap](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-17-tokens.png)

## 16. Commercial license and install ping

The program is **AGPL-3.0-only and free**. The **commercial license** is for those who use it in a company and don't want to comply with the AGPL. In **Settings → 🏢 License** (first registered person on the server only):

1. Choose whether the server is **personal** or **company**.
2. **Buy a commercial license** → pick the plan, the network (**USDT on Tron or Ethereum**) and your email.
3. Pay **exactly** the amount shown (the last decimals identify your payment) to the given address; the QR carries it. Check the **token contract** with the explorer link.
4. The app checks the payment by itself. When confirmed you get an email with the code `TM-XXXX-XXXX-XXXX` and the license activates itself. Otherwise paste the code under **Already have a code?**.

The **bar** on top appears with more than 3 people or 2 AIs, or if you declare it a company; it can be dismissed for 7 days and never blocks anything.

**Install ping.** Once a day the server sends a random code, the version, the operating system and how many people, AIs, projects and teams there are. Never names, emails, tasks or content; the IP is not stored. Turn it off with `TM_TELEMETRY=0` or from the License window.

---

**Something not clear?** Every option in **Settings** has a «?» with help. And if you find a problem, open an [issue](https://github.com/polblancoo/Rust_tuiManager/issues).
