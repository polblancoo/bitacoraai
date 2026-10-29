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

> 🚧 **Coming soon**: **lead per project** (scoped to that project only), **configurable scope per AI** (which projects it touches, whether it may invite other AIs), **progress reports** and a **portfolio panel** with all your projects at once. See the [roadmap](https://github.com/polblancoo/Rust_tuiManager/blob/main/ROADMAP.md).

---

**Something not clear?** Every option in **Settings** has a «?» with help. And if you find a problem, open an [issue](https://github.com/polblancoo/Rust_tuiManager/issues).
