<p align="center"><b>English</b> · <a href="https://github.com/polblancoo/crewhelm/blob/main/crates/tui/README.es.md">Español</a> · <a href="https://github.com/polblancoo/crewhelm/blob/main/crates/tui/README.pt.md">Português</a></p>

# crewhelm

> **0.6.3**: license **AGPL-3.0-only**; the server adds a browser terminal, usage and quotas.
>
> **0.6.2**: the terminal client is now available in **Spanish, English and Portuguese** (the language comes from `client.json` or `$LANG`). Site and downloads: https://polblancoo.github.io/rust_tuiManager/

> 🎛️ **A command board for your projects**: run several projects at once —an app factory, a spreadsheet, an onboarding, anything— with **people and AIs on the same board**. Every project knows where its work lives (folder and repo). Self-hosted.

### Why it is different

What we have not seen together in any other tool (check, and tell us if you find one):

- 🤖 **AIs as team members**, with a role, a list of forbidden commands and permissions that a person grants from the task itself (`/permitir cargo build`).
- 🙋 **Help requests from the AI** that arrive as a task for the right person.
- 💻 **SSH terminal inside the task** (key-based), and what was done is written down so the AI can pick up from there. *(0.6.3)*
- 📊 **Portfolio of several projects**, with the AI's cost and hours. *(cost and hours: coming in 0.6.3)*
- 🏠 **Self-hosted**: your data and keys stay on your server.

Terminal client (ratatui) of the collaborative task manager [Rust TUI Manager](https://github.com/polblancoo/crewhelm). It needs a running [`crewhelm-server`](https://crates.io/crates/crewhelm-server).

> **Versions 0.2 onwards**: this crate went from being a local notes and snippets manager to being a network client. The earlier code is in the `legacy/` folder of the repository and in the 0.1.x releases.

![Board in the terminal](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/tui-board.png)

![Task detail: subtasks, labels and comments](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/tui-detail.png)

> 👉 **[Step-by-step guide: your first project](https://github.com/polblancoo/crewhelm/blob/main/docs/GUIDE.md)**, with screenshots.

## 🤖 AI agents as team members

Coding assistants (Claude Code, Cursor, Windsurf, Zed or any MCP client) can **join a human team**: each AI has its own user, token, role and model (e.g. `claude:Opus`), creates and completes tasks, **assigns them to people** and receives the ones you assign. They are added from the sidebar of the web or desktop app (**🤖 AIs → «+»**); the token is shown once and rotated or revoked with one click. An AI cannot administer teams, invite people or touch other people's tasks. Details in the [main README](https://github.com/polblancoo/crewhelm#readme); with **`tm-agent`** an AI picks up the tasks you assign on its own (see the [agents guide](https://github.com/polblancoo/crewhelm/blob/main/docs/AGENTS.md)); the `tm-mcp` binary is in development. In this client, AI accounts show up as members.

## Usage

```sh
cargo install crewhelm
crewhelm http://127.0.0.1:7878
```

On the sign-in screen, `F2` toggles between signing in and creating an account. The session is saved in `~/.config/rust-tui-manager/client.json` (`0600` permissions) and the server is remembered for next time.

## Shortcuts

**Board**

| Key | Action |
|---|---|
| `h` `l` / `←` `→` | Switch column |
| `j` `k` / `↑` `↓` | Move the selection |
| `Enter` | Open detail (subtasks, labels, comments) |
| `n` | New task |
| `e` | Edit the title |
| `d` | Delete the task |
| `H` / `L` | Move the task to another status |
| `+` / `-` | Raise or lower the priority |
| `a` | Take the task or release mine |
| `A` | Assign to a member, with suggestions (admin or owner; `-` clears) |
| `N` | Inbox: news and my tasks (opens by itself at sign-in) |
| `Tab` / `Shift+Tab` | Next or previous project |
| `[` / `]` | Previous or next team |
| `P` / `T` | New project / new team |
| `m` | Add a member with search: `↑↓` choose, `Tab` completes, `Ctrl+R` changes the role |
| `Ctrl+T` + letter | Team prefix: `m` member · `r` role · `i` invite · `j` join · `t` team · `e` my email · `v` verify email · `s` message the team · `g` link Telegram · `n` inbox |
| `R` | Change a member's role: `user role` |
| `I` | Generate an invitation code (member role, 7 days, 10 uses) |
| `J` | Join a team with a code |
| `X` | Sign out of all devices |
| `C` | Settings menu: email, Telegram, inbox, messages and team, session |
| `?` | Help with shortcut search |
| `q` | Quit |

**Task detail**

| Key | Action |
|---|---|
| `s` | New subtask |
| `j` `k` | Choose a subtask |
| `Space` / `x` | Check or uncheck the subtask |
| `d` | Delete the subtask |
| `l` | Add or remove a label (created if it does not exist) |
| `c` | Comment |
| `Esc` | Back |

The header marks roles: `★` owner, `⚙` admin, `👁` viewer. The green dot in the corner shows a live connection.

More information, security and server configuration in the [project README](https://github.com/polblancoo/crewhelm).

## License

**AGPL-3.0-only.** Free to use and to self-host. If you offer a modified version as a service, you must publish your changes under the same license. A commercial license without that obligation is available: ask in [the issues](https://github.com/polblancoo/crewhelm/issues). Versions published up to 0.6.2 remain under MIT.
