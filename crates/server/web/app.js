"use strict";
// Cliente de rust-tui-manager. Todo el contenido de usuario se inserta con textContent (sin innerHTML).

const STATUS = [
  { id: "todo", label: "Por hacer", color: "var(--blue)" },
  { id: "doing", label: "En curso", color: "var(--yellow)" },
  { id: "done", label: "Hecho", color: "var(--green)" },
];
const PRIO = ["Baja", "Media", "Alta", "Urgente"];
const ROLES = [["viewer", "Lector"], ["member", "Miembro"], ["admin", "Admin"], ["owner", "Dueño"]];
const RANK = { viewer: 0, member: 1, admin: 2, owner: 3 };
const AVATAR = ["#7aa2f7", "#bb9af7", "#9ece6a", "#e0af68", "#f7768e", "#7dcfff"];

const store = {
  get: (k) => { try { return localStorage.getItem(k); } catch { return null; } },
  set: (k, v) => { try { v == null ? localStorage.removeItem(k) : localStorage.setItem(k, v); } catch {} },
};

const S = {
  // servida por tm-server (web): el servidor es el propio origen; en Tauri se elige a mano
  server: /^https?:$/.test(location.protocol) && location.hostname !== "tauri.localhost" ? location.origin : store.get("server") || "http://127.0.0.1:7878",
  token: store.get("token"),
  user: JSON.parse(store.get("user") || "null"),
  teams: [], team: null, projects: [], project: null,
  tasks: [], members: [], labels: [], online: false, query: "", view: store.get("view") || "board", month: new Date(),
  open: null, comments: [], ws: null, register: false, loginError: "",
  inbox: null, unread: 0, me: null, canvas: null, prefix: false,
};

const myRole = () => S.members.find((m) => m.id === S.user?.id)?.role || "viewer";
const can = (min) => RANK[myRole()] >= RANK[min];

// ---------- utilidades ----------
function h(tag, attrs = {}, ...kids) {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs || {})) {
    if (v == null || v === false) continue;
    if (k.startsWith("on")) el.addEventListener(k.slice(2), v);
    else if (k === "class") el.className = v;
    else if (k === "style") el.style.cssText = v;
    else if (k in el) el[k] = v;
    else el.setAttribute(k, v);
  }
  for (const kid of kids.flat()) if (kid != null && kid !== false) el.append(kid);
  return el;
}

function toast(msg, error = false) {
  const t = document.getElementById("toast");
  t.textContent = msg; t.className = error ? "error" : ""; t.hidden = false;
  clearTimeout(toast.t); toast.t = setTimeout(() => (t.hidden = true), 3500);
}

async function api(method, path, body) {
  const res = await fetch(S.server + path, {
    method,
    headers: { "Content-Type": "application/json", ...(S.token ? { Authorization: "Bearer " + S.token } : {}) },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  if (res.status === 401 && S.token) { logout(); throw new Error("sesión vencida"); }
  if (!res.ok) {
    const j = await res.json().catch(() => ({}));
    throw new Error(j.error || res.statusText);
  }
  return res.status === 204 ? null : res.json();
}

const guard = (fn) => async (...a) => { try { await fn(...a); } catch (e) { toast(e.message, true); } };
const color = (name) => AVATAR[[...name].reduce((a, c) => a + c.charCodeAt(0), 0) % AVATAR.length];
const avatar = (name) => h("span", { class: "avatar", title: name, style: `background:${color(name)}` }, name.slice(0, 2).toUpperCase());

// ---------- sesión ----------
const logoutAll = guard(async () => {
  if (!confirm("¿Cerrar sesión en todos tus dispositivos?")) return;
  await api("POST", "/api/logout-all");
  logout();
});

function logout() {
  S.token = null; S.user = null; S.ws?.close(); S.ws = null;
  store.set("token", null); store.set("user", null);
  render();
}

async function authenticate(user, pass) {
  const r = await api("POST", S.register ? "/api/register" : "/api/login", { username: user, password: pass });
  S.token = r.token; S.user = r.user;
  store.set("token", r.token); store.set("user", JSON.stringify(r.user)); store.set("server", S.server);
  await boot(true);
}

async function boot(first) {
  S.teams = await api("GET", "/api/teams");
  await selectTeam(S.teams.find((t) => t.id === S.team?.id) || S.teams[0] || null);
  connectWs();
  if (first) loadInbox(true);
}

// ---------- bandeja y perfil ----------
async function loadInbox(auto) {
  const [inbox, me] = await Promise.all([api("GET", "/api/inbox"), api("GET", "/api/me")]).catch(() => [null, null]);
  if (!inbox) return;
  S.inbox = inbox; S.me = me; S.unread = inbox.unread;
  if (!auto || inbox.unread > 0 || inbox.tasks.length > 0) openInbox();
  else render();
}

async function closeInbox() {
  document.getElementById("inbox-modal")?.remove();
  if (S.unread > 0) {
    await api("POST", "/api/notifications/read", { ids: [] }).catch(() => {});
    S.unread = 0; S.inbox?.notifications.forEach((n) => (n.read = true));
  }
  render();
}

const jumpToTask = guard(async (it) => {
  const team = S.teams.find((t) => t.name === it.team);
  if (team && team.id !== S.team?.id) await selectTeam(team);
  const p = S.projects.find((x) => x.id === it.task.project_id);
  if (p && p.id !== S.project?.id) await selectProject(p);
  const t = S.tasks.find((x) => x.id === it.task.id);
  if (t) openTask(t);
});

function openInbox() {
  document.getElementById("inbox-modal")?.remove();
  const inb = S.inbox; if (!inb) return;
  const KIND = { assigned: "📌", comment: "💬", member_added: "👥", role_changed: "🎖", message: "✉" };
  document.body.append(h("div", { class: "overlay", id: "inbox-modal", onmousedown: (e) => e.target.id === "inbox-modal" && closeInbox() },
    h("div", { class: "modal" },
      h("h3", {}, `Novedades${S.unread ? ` (${S.unread} sin leer)` : ""}`),
      h("div", { class: "inbox-list" }, inb.notifications.length === 0 ? h("small", {}, "Nada nuevo.") :
        inb.notifications.map((n) => h("div", { class: "note-row" + (n.read ? "" : " unread") },
          h("span", {}, KIND[n.kind] || "•"), h("span", { class: "grow" }, n.text),
          h("small", {}, new Date(n.created_at).toLocaleString())))),
      h("h3", {}, `Mis tareas abiertas (${inb.tasks.length})`),
      h("div", { class: "inbox-list" }, inb.tasks.length === 0 ? h("small", {}, "No tenés tareas asignadas.") :
        inb.tasks.map((it) => h("div", { class: "note-row link", onclick: () => { closeInbox(); jumpToTask(it); } },
          h("span", { class: `pill p${it.task.priority}` }, PRIO[it.task.priority]),
          h("span", { class: "grow" }, it.task.title),
          h("small", {}, `${it.team} / ${it.project}${it.task.due ? " · vence " + new Date(it.task.due).toLocaleDateString() : ""}`)))),
      h("div", { class: "row" }, h("button", { onclick: openProfile }, "Mi email"), h("span", { class: "spacer" }),
        h("button", { class: "primary", onclick: closeInbox }, "Marcar como leído y cerrar")))));
}

const openProfile = guard(async () => {
  document.getElementById("profile-modal")?.remove();
  S.me = await api("GET", "/api/me");
  const me = S.me;
  const email = h("input", { type: "email", placeholder: "tu@email.com", maxLength: 254, value: me.email || "" });
  const code = h("input", { placeholder: "Código de 6 dígitos", maxLength: 12, inputMode: "numeric" });
  const close = () => document.getElementById("profile-modal")?.remove();
  const state = !me.email ? "Sin email cargado." : me.email_verified ? "✓ Email verificado: te enviamos los avisos por correo." : "Falta verificar: revisá tu correo e ingresá el código.";
  document.body.append(h("div", { class: "overlay", id: "profile-modal", onmousedown: (e) => e.target.id === "profile-modal" && close() },
    h("div", { class: "modal" },
      h("h3", {}, `Perfil de ${me.username}`),
      !me.email_available && h("small", { class: "warn" }, "Este servidor no tiene el envío de emails configurado (TM_SMTP_*): vas a ver los avisos solo en la bandeja."),
      h("label", {}, "Email para recibir avisos de tareas y comentarios", email),
      h("div", { class: "row" },
        me.email && h("button", { class: "danger", onclick: guard(async () => { await api("DELETE", "/api/me/email"); close(); openProfile(); }) }, "Quitar"),
        h("span", { class: "spacer" }),
        h("button", { class: "primary", onclick: guard(async () => {
          await api("PUT", "/api/me/email", { email: email.value }); toast("Te enviamos un código"); close(); openProfile();
        }) }, "Enviar código")),
      h("small", {}, state),
      me.email && !me.email_verified && h("div", { class: "row" }, code, h("button", { class: "primary", onclick: guard(async () => {
        await api("POST", "/api/me/email/verify", { code: code.value }); toast("Email verificado"); close(); openProfile();
      }) }, "Verificar")),
      h("div", { class: "row" }, h("button", { onclick: close }, "Cerrar")))));
});

async function selectTeam(team) {
  S.team = team; S.projects = []; S.project = null; S.tasks = []; S.members = []; S.labels = []; S.canvas = null;
  if (team) {
    [S.projects, S.members, S.labels] = await Promise.all([
      api("GET", `/api/teams/${team.id}/projects`), api("GET", `/api/teams/${team.id}/members`),
      api("GET", `/api/teams/${team.id}/labels`),
    ]);
    await selectProject(S.projects[0] || null);
  } else render();
}

async function selectProject(p) {
  S.project = p; S.canvas = null; S.tasks = p ? await api("GET", `/api/projects/${p.id}/tasks`) : [];
  render();
}

// ---------- tiempo real ----------
function connectWs() {
  S.ws?.close();
  if (!S.token) return;
  // el token viaja como primer mensaje, no en la URL (quedaría en logs de proxies)
  const ws = new WebSocket(S.server.replace(/^http/, "ws") + "/ws");
  S.ws = ws;
  ws.onopen = () => { ws.send(JSON.stringify({ token: S.token })); S.online = true; render(); };
  ws.onclose = () => { S.online = false; if (S.ws === ws && S.token) setTimeout(connectWs, 3000); render(); };
  ws.onmessage = (m) => { try { onEvent(JSON.parse(m.data)); } catch {} };
}

function onEvent({ type, data }) {
  if (type === "task_created" || type === "task_updated") {
    if (data.project_id === S.project?.id) {
      const i = S.tasks.findIndex((t) => t.id === data.id);
      i >= 0 ? (S.tasks[i] = data) : S.tasks.push(data);
    }
  } else if (type === "label_created") {
    if (data.team_id === S.team?.id && !S.labels.some((l) => l.id === data.id)) S.labels.push(data);
  } else if (type === "label_deleted") {
    S.labels = S.labels.filter((l) => l.id !== data.id);
    S.tasks.forEach((t) => (t.labels = t.labels.filter((l) => l.id !== data.id)));
  } else if (type === "task_deleted") {
    S.tasks = S.tasks.filter((t) => t.id !== data.id && t.parent_id !== data.id);
    if (S.open === data.id) closeModal();
  } else if (type === "project_created") {
    if (data.team_id === S.team?.id && !S.projects.some((p) => p.id === data.id)) S.projects.push(data);
  } else if (type === "comment_added") {
    if (S.open === data.task_id && !S.comments.some((c) => c.id === data.id)) { S.comments.push(data); renderComments(); }
  } else if (type === "notification") {
    S.unread++; if (S.inbox) { S.inbox.notifications.unshift(data); S.inbox.unread++; }
    toast("🔔 " + data.text);
    if (document.getElementById("inbox-modal")) openInbox();
  } else if (type === "canvas_item") {
    if (S.canvas && data.project_id === S.canvas.project_id) {
      const i = S.canvas.items.findIndex((x) => x.id === data.id);
      i >= 0 ? (S.canvas.items[i] = data) : S.canvas.items.push(data);
      if (CV.drag) return;
    }
  } else if (type === "canvas_item_deleted") {
    if (S.canvas) { S.canvas.items = S.canvas.items.filter((x) => x.id !== data.id); S.canvas.links = S.canvas.links.filter((l) => l.from_id !== data.id && l.to_id !== data.id); }
  } else if (type === "canvas_link") {
    if (S.canvas && data.project_id === S.canvas.project_id && !S.canvas.links.some((l) => l.id === data.id)) S.canvas.links.push(data);
  } else if (type === "canvas_link_deleted") {
    if (S.canvas) S.canvas.links = S.canvas.links.filter((l) => l.id !== data.id);
  } else if (type === "member_added") {
    boot().catch(() => {}); return;
  } else if (type === "member_removed") {
    if (data.user_id === S.user?.id) { toast("Ya no sos parte de ese equipo", true); closeModal(); boot().catch(() => {}); }
    else refreshMembers();
    return;
  } else if (type === "member_role_changed") {
    refreshMembers(); return;
  }
  render();
  S.redrawSubs?.();
}

const refreshMembers = () => api("GET", `/api/teams/${S.team.id}/members`).then((m) => { S.members = m; render(); document.getElementById("team-modal") && openTeam(); }).catch(() => {});

// ---------- diálogos propios (en lugar de prompt() del navegador) ----------
// fields: [{ name, label, type?: "text"|"textarea"|"select"|"date", options?: [[value,label]], value?, max? }]
function form(title, fields, okLabel = "Aceptar") {
  return new Promise((resolve) => {
    const inputs = fields.map((f) => {
      const el = f.type === "textarea" ? h("textarea", { rows: 4, maxLength: f.max || 10000, value: f.value || "" })
        : f.type === "select" ? h("select", {}, f.options.map(([v, l]) => h("option", { value: v, selected: String(v) === String(f.value) }, l)))
        : h("input", { type: f.type || "text", maxLength: f.max || 200, value: f.value || "", placeholder: f.placeholder || "" });
      return [f, el];
    });
    const done = (v) => { overlay.remove(); resolve(v); };
    const submit = () => {
      const out = {};
      for (const [f, el] of inputs) out[f.name] = el.value.trim();
      if (fields[0] && !out[fields[0].name]) { inputs[0][1].focus(); return; }
      done(out);
    };
    const overlay = h("div", { class: "overlay", onmousedown: (e) => e.target === overlay && done(null) },
      h("div", { class: "modal small", onkeydown: (e) => { if (e.key === "Escape") { e.stopPropagation(); done(null); } else if (e.key === "Enter" && e.target.tagName !== "TEXTAREA") { e.preventDefault(); submit(); } } },
        h("h3", {}, title),
        inputs.map(([f, el]) => h("label", {}, f.label, el)),
        h("div", { class: "row" }, h("button", { onclick: () => done(null) }, "Cancelar"), h("button", { class: "primary", onclick: submit }, okLabel))));
    document.body.append(overlay);
    inputs[0][1].focus();
  });
}

// ---------- acciones ----------
const createTeam = guard(async () => {
  const f = await form("Nuevo equipo", [{ name: "name", label: "Nombre del equipo", max: 60 }], "Crear"); if (!f) return;
  const t = await api("POST", "/api/teams", { name: f.name });
  S.teams.push(t); await selectTeam(t);
});
const createProject = guard(async () => {
  const f = await form("Nuevo proyecto", [{ name: "name", label: "Nombre del proyecto", max: 60 }], "Crear"); if (!f) return;
  const p = await api("POST", `/api/teams/${S.team.id}/projects`, { name: f.name });
  if (!S.projects.some((x) => x.id === p.id)) S.projects.push(p);
  await selectProject(p);
});
const joinWithCode = guard(async () => {
  const f = await form("Unirse a un equipo", [{ name: "code", label: "Código de invitación", max: 64 }], "Unirme"); if (!f) return;
  const team = await api("POST", `/api/invites/${encodeURIComponent(f.code)}/accept`, {});
  S.teams = await api("GET", "/api/teams");
  await selectTeam(S.teams.find((t) => t.id === team.id) || S.teams[0]);
  toast(`Te uniste a ${team.name}`);
});

// ---------- modal de equipo: miembros, roles e invitaciones ----------
const openTeam = guard(async () => {
  document.getElementById("team-modal")?.remove();
  const admin = can("admin");
  const invites = admin ? await api("GET", `/api/teams/${S.team.id}/invites`) : [];
  const roleSel = (value, onchange, disabled, max) => h("select", { disabled, onchange },
    ROLES.filter(([r]) => r !== "owner" || value === "owner").filter(([r]) => r === value || RANK[r] < max)
      .map(([r, l]) => h("option", { value: r, selected: r === value }, l)));

  const rows = S.members.map((m) => {
    const manageable = admin && m.role !== "owner" && RANK[m.role] < RANK[myRole()];
    return h("div", { class: "member" }, avatar(m.username), h("span", { class: "grow" }, m.username),
      roleSel(m.role, guard(async (e) => {
        await api("PUT", `/api/teams/${S.team.id}/members/${m.id}`, { role: e.target.value }); refreshMembers();
      }), !manageable, RANK[myRole()]),
      (manageable || (m.id === S.user.id && m.role !== "owner")) && h("button", { class: "ghost danger", title: m.id === S.user.id ? "Salir del equipo" : "Quitar", onclick: guard(async () => {
        if (!confirm(m.id === S.user.id ? "¿Salir del equipo?" : `¿Quitar a ${m.username}?`)) return;
        await api("DELETE", `/api/teams/${S.team.id}/members/${m.id}`);
        if (m.id === S.user.id) { document.getElementById("team-modal")?.remove(); await boot(); } else refreshMembers();
      }) }, "✕"));
  });

  const body = [h("h3", {}, `Equipo ${S.team.name}`), h("div", { class: "members-list" }, rows)];
  if (admin) {
    const user = h("input", { placeholder: "Buscar usuario…", maxLength: 32, list: "user-suggest", autocomplete: "off" });
    const sug = h("datalist", { id: "user-suggest" });
    let lastQ = "";
    user.addEventListener("input", async () => {
      const q = user.value.trim(); if (!q || q === lastQ) return; lastQ = q;
      try {
        const us = await api("GET", `/api/users/search?q=${encodeURIComponent(q)}&team_id=${S.team.id}`);
        sug.replaceChildren(...us.map((u) => h("option", { value: u.username })));
      } catch {}
    });
    const role = roleSel("member", null, false, RANK[myRole()]);
    const inv = h("div", { class: "invite-out" });
    const grant = ROLES.filter(([r]) => RANK[r] < RANK[myRole()] && r !== "owner");
    const invRole = h("select", {}, grant.map(([r, l]) => h("option", { value: r, selected: r === "member" }, l)));
    body.push(
      h("h3", {}, "Agregar por usuario"),
      h("div", { class: "row" }, user, sug, role, h("button", { class: "primary", onclick: guard(async () => {
        await api("POST", `/api/teams/${S.team.id}/members`, { username: user.value, role: role.value });
        user.value = ""; toast("Miembro agregado");
      }) }, "Agregar")),
      h("h3", {}, "Invitación por código"),
      h("div", { class: "row" }, invRole, h("button", { onclick: guard(async () => {
        const r = await api("POST", `/api/teams/${S.team.id}/invites`, { role: invRole.value });
        inv.replaceChildren(h("code", {}, r.code), h("button", { onclick: () => navigator.clipboard?.writeText(r.code).then(() => toast("Código copiado")) }, "Copiar"),
          h("small", {}, "Se muestra una sola vez · vence en 7 días · 10 usos"));
      }) }, "Generar código")),
      inv,
      invites.length > 0 && h("h3", {}, "Invitaciones activas"),
      ...invites.map((i) => h("div", { class: "member" }, h("span", { class: "grow" },
        `${ROLES.find(([r]) => r === i.role)[1]} · ${i.uses}/${i.max_uses} usos · vence ${new Date(i.expires_at).toLocaleDateString()}`),
        h("button", { class: "ghost danger", onclick: guard(async () => { await api("DELETE", `/api/invites/${i.id}`); openTeam(); }) }, "Revocar"))));
  }
  body.push(h("div", { class: "row" }, h("button", { onclick: () => document.getElementById("team-modal")?.remove() }, "Cerrar")));
  document.body.append(h("div", { class: "overlay", id: "team-modal", onmousedown: (e) => e.target.id === "team-modal" && e.target.remove() },
    h("div", { class: "modal" }, body)));
});
const createTask = guard(async (status) => {
  const f = await form("Nueva tarea", [
    { name: "title", label: "Título", max: 200 },
    { name: "description", label: "Descripción (opcional)", type: "textarea" },
    { name: "status", label: "Estado", type: "select", options: STATUS.map((s) => [s.id, s.label]), value: status },
    { name: "priority", label: "Prioridad", type: "select", options: PRIO.map((p, i) => [i, p]), value: 1 },
    { name: "start", label: "Empieza (opcional)", type: "date" },
    { name: "due", label: "Vence (opcional)", type: "date" },
  ], "Crear tarea"); if (!f) return;
  const day = (d) => (d ? new Date(d + "T12:00:00Z").toISOString() : undefined);
  let t = await api("POST", `/api/projects/${S.project.id}/tasks`, { title: f.title, description: f.description, priority: +f.priority, start: day(f.start), due: day(f.due) });
  if (f.status !== "todo") t = await api("PATCH", `/api/tasks/${t.id}`, { status: f.status });
  upsert(t); render();
});
const patchTask = guard(async (id, patch) => { upsert(await api("PATCH", `/api/tasks/${id}`, patch)); render(); });

function upsert(t) {
  const i = S.tasks.findIndex((x) => x.id === t.id);
  i >= 0 ? (S.tasks[i] = t) : S.tasks.push(t);
}

// ---------- modal de tarea ----------
function closeModal() { S.open = null; S.redrawSubs = null; document.getElementById("modal")?.remove(); }

function renderComments() {
  const box = document.getElementById("comments"); if (!box) return;
  box.replaceChildren(...S.comments.map((c) => h("div", { class: "comment" },
    h("b", {}, c.username), h("small", {}, new Date(c.created_at).toLocaleString()), h("div", {}, c.body))));
  box.scrollTop = box.scrollHeight;
}

const openTask = guard(async (task) => {
  S.open = task.id;
  S.comments = await api("GET", `/api/tasks/${task.id}/comments`);
  const cur = () => S.tasks.find((t) => t.id === task.id) || task;
  const title = h("input", { value: cur().title, maxLength: 200 });
  const desc = h("textarea", { rows: 5, maxLength: 10000, value: cur().description });
  const prio = h("select", {}, PRIO.map((p, i) => h("option", { value: i, selected: cur().priority === i }, p)));
  const status = h("select", {}, STATUS.map((s) => h("option", { value: s.id, selected: cur().status === s.id }, s.label)));
  // líder (admin+) asigna a cualquiera; miembro solo toma una tarea libre o suelta la suya; lector no asigna
  const leader = can("admin"), meId = S.user?.id, ass = cur().assignee_id;
  const pickable = leader ? S.members.filter((m) => RANK[m.role] >= 1 || m.id === ass)
    : S.members.filter((m) => m.id === meId && can("member") && (ass == null || ass === meId));
  const who = h("select", { disabled: !leader && !(can("member") && (ass == null || ass === meId)) },
    h("option", { value: "" }, "Sin asignar"),
    (leader ? pickable : [...pickable, ...S.members.filter((m) => m.id === ass && m.id !== meId)]).map((m) => h("option", { value: m.id, selected: ass === m.id }, m.id === meId && !leader ? `Yo (${m.username})` : m.username)));
  const whoHint = leader ? "Como líder podés asignarla a cualquier miembro" : can("member") ? "Podés tomar una tarea libre o soltar la tuya" : "";
  const start = h("input", { type: "date", value: cur().start ? cur().start.slice(0, 10) : "" });
  const due = h("input", { type: "date", value: cur().due ? cur().due.slice(0, 10) : "" });
  const comment = h("input", { placeholder: "Escribí un comentario…", maxLength: 4000 });

  // etiquetas: chips que se alternan + alta de etiqueta nueva
  const labelBox = h("div", { class: "chips" });
  const drawLabels = () => {
    const on = new Set(cur().labels.map((l) => l.id));
    labelBox.replaceChildren(...S.labels.map((l) => h("button", {
      class: "chip" + (on.has(l.id) ? " on" : ""), style: `--c:${l.color}`,
      onclick: guard(async () => {
        const ids = on.has(l.id) ? [...on].filter((i) => i !== l.id) : [...on, l.id];
        upsert(await api("PUT", `/api/tasks/${task.id}/labels`, { label_ids: ids }));
        drawLabels(); render();
      }),
    }, l.name)));
  };
  const newLabel = h("input", { placeholder: "Nueva etiqueta + Enter", maxLength: 30 });
  newLabel.addEventListener("keydown", guard(async (e) => {
    if (e.key !== "Enter" || !newLabel.value.trim()) return;
    const colors = ["#7aa2f7", "#bb9af7", "#9ece6a", "#e0af68", "#f7768e", "#7dcfff"];
    const l = await api("POST", `/api/teams/${S.team.id}/labels`, { name: newLabel.value, color: colors[S.labels.length % 6] });
    if (!S.labels.some((x) => x.id === l.id)) S.labels.push(l);
    newLabel.value = ""; drawLabels();
  }));

  // subtareas: checklist
  const subBox = h("div", { class: "subs" });
  const drawSubs = () => {
    const subs = S.tasks.filter((t) => t.parent_id === task.id);
    subBox.replaceChildren(...subs.map((st) => h("label", { class: "sub" + (st.status === "done" ? " done" : "") },
      h("input", { type: "checkbox", checked: st.status === "done", onchange: (e) => patchTask(st.id, { status: e.target.checked ? "done" : "todo" }).then(drawSubs) }),
      h("span", {}, st.title),
      h("button", { class: "ghost danger", title: "Borrar subtarea", onclick: guard(async (e) => {
        e.preventDefault(); await api("DELETE", `/api/tasks/${st.id}`);
        S.tasks = S.tasks.filter((t) => t.id !== st.id); drawSubs(); render();
      }) }, "✕"))));
  };
  S.redrawSubs = drawSubs;
  const newSub = h("input", { placeholder: "Nueva subtarea + Enter", maxLength: 200 });
  newSub.addEventListener("keydown", guard(async (e) => {
    if (e.key !== "Enter" || !newSub.value.trim()) return;
    const st = await api("POST", `/api/projects/${S.project.id}/tasks`, { title: newSub.value, parent_id: task.id });
    upsert(st); newSub.value = ""; drawSubs(); render();
  }));
  // dependencias: esta tarea no puede empezar hasta que terminen las listadas
  const depBox = h("div", { class: "subs" });
  const drawDeps = () => {
    depBox.replaceChildren(...(cur().depends_on || []).map((id) => {
      const d = S.tasks.find((t) => t.id === id);
      return h("div", { class: "sub" }, h("span", {}, "⛓ " + (d ? d.title : "#" + id)),
        can("member") && h("button", { class: "ghost danger", title: "Quitar dependencia", onclick: guard(async () => {
          upsert(await api("DELETE", `/api/tasks/${task.id}/deps/${id}`)); drawDeps(); render();
        }) }, "✕"));
    }));
  };
  const depAdd = h("select", { onchange: guard(async (e) => {
    if (!e.target.value) return;
    upsert(await api("PUT", `/api/tasks/${task.id}/deps/${e.target.value}`)); drawDeps(); render();
    e.target.value = "";
  }), disabled: !can("member") }, h("option", { value: "" }, "+ Agregar dependencia…"),
    S.tasks.filter((t) => !t.parent_id && t.id !== task.id).map((t) => h("option", { value: t.id }, clip(t.title, 50))));
  const send = guard(async () => {
    if (!comment.value.trim()) return;
    const c = await api("POST", `/api/tasks/${task.id}/comments`, { body: comment.value });
    comment.value = "";
    if (!S.comments.some((x) => x.id === c.id)) S.comments.push(c);
    renderComments();
  });
  comment.addEventListener("keydown", (e) => e.key === "Enter" && send());

  const save = guard(async () => {
    await patchTask(task.id, {
      title: title.value, description: desc.value, priority: +prio.value, status: status.value,
      ...(+(who.value || 0) !== (cur().assignee_id || 0) ? { assignee_id: who.value ? +who.value : null } : {}),
      due: due.value ? new Date(due.value + "T12:00:00Z").toISOString() : null,
      start: start.value ? new Date(start.value + "T12:00:00Z").toISOString() : null,
    });
    closeModal();
  });
  const del = guard(async () => {
    if (!confirm("¿Borrar la tarea?")) return;
    await api("DELETE", `/api/tasks/${task.id}`);
    S.tasks = S.tasks.filter((t) => t.id !== task.id); closeModal(); render();
  });

  const modal = h("div", { class: "overlay", id: "modal", onmousedown: (e) => e.target.id === "modal" && closeModal() },
    h("div", { class: "modal" },
      h("h3", {}, "Tarea"),
      h("label", {}, "Título", title),
      h("label", {}, "Descripción", desc),
      h("h3", {}, "Etiquetas"), labelBox, newLabel,
      h("h3", {}, "Subtareas"), subBox, newSub,
      h("div", { class: "grid2" },
        h("label", {}, "Estado", status), h("label", {}, "Prioridad", prio),
        h("label", {}, "Responsable", who), h("label", {}, "Empieza", start),
        h("label", {}, "Vence", due), h("small", { class: "hint" }, whoHint)),
      h("h3", {}, "Depende de"), depBox, depAdd,
      h("h3", {}, "Comentarios"),
      h("div", { class: "comments", id: "comments" }),
      comment,
      h("div", { class: "row" },
        can("member") && h("button", { class: "danger", onclick: del }, "Borrar"),
        h("button", { onclick: () => { closeModal(); openMindmap(cur()); } }, "Mapa mental"), h("span", { class: "spacer" }),
        h("button", { onclick: closeModal }, "Cancelar"), h("button", { class: "primary", onclick: save }, "Guardar"))));
  document.body.append(modal);
  drawLabels(); drawSubs(); drawDeps();
  renderComments();
  title.focus();
});

// ---------- vistas ----------
function loginView() {
  const srv = h("input", { value: S.server });
  const user = h("input", { value: S.user?.username || "", autocomplete: "username", maxLength: 32 });
  const pass = h("input", { type: "password", autocomplete: "current-password", maxLength: 128 });
  const err = h("div", { class: "err" }, S.loginError);
  const go = async () => {
    S.server = srv.value.trim().replace(/\/$/, "");
    try { await authenticate(user.value.trim(), pass.value); }
    catch (e) { err.textContent = e.message === "Failed to fetch" ? "no se pudo conectar al servidor" : e.message; }
  };
  pass.addEventListener("keydown", (e) => e.key === "Enter" && go());
  return h("div", { class: "login" }, h("div", { class: "login-card" },
    h("h1", {}, "Rust TUI Manager"),
    h("label", {}, "Servidor", srv), h("label", {}, "Usuario", user), h("label", {}, "Contraseña", pass),
    err,
    h("button", { class: "primary", onclick: go }, S.register ? "Crear cuenta" : "Entrar"),
    h("button", { class: "ghost", onclick: () => { S.register = !S.register; render(); } },
      S.register ? "Ya tengo cuenta" : "Crear una cuenta")));
}

function taskCard(t) {
  const late = t.due && t.status !== "done" && new Date(t.due) < new Date();
  const card = h("div", { class: "card", draggable: true, onclick: () => openTask(t) },
    h("div", { class: "title" }, t.title),
    t.labels?.length > 0 && h("div", { class: "chips" }, t.labels.map((l) => h("span", { class: "chip on", style: `--c:${l.color}` }, l.name))),
    t.subtasks_total > 0 && h("div", { class: "progress", title: `${t.subtasks_done}/${t.subtasks_total} subtareas` },
      h("div", { class: "bar", style: `width:${Math.round(100 * t.subtasks_done / t.subtasks_total)}%` })),
    h("div", { class: "meta" },
      t.subtasks_total > 0 && h("span", {}, `☑ ${t.subtasks_done}/${t.subtasks_total}`),
      h("span", { class: `pill p${t.priority}` }, PRIO[t.priority]),
      t.due && h("span", { class: "due" + (late ? " late" : "") }, "⏰ " + new Date(t.due).toLocaleDateString()),
      h("span", { class: "spacer" }),
      t.assignee && avatar(t.assignee)));
  card.draggable = can("member");
  card.addEventListener("dragstart", (e) => { e.dataTransfer.setData("text/plain", String(t.id)); card.classList.add("dragging"); });
  card.addEventListener("dragend", () => card.classList.remove("dragging"));
  return card;
}

function column(s) {
  const q = S.query.toLowerCase();
  const tasks = S.tasks
    .filter((t) => !t.parent_id && t.status === s.id && (!q || t.title.toLowerCase().includes(q) || (t.assignee || "").toLowerCase().includes(q)
      || t.labels.some((l) => l.name.toLowerCase().includes(q))))
    .sort((a, b) => b.priority - a.priority || a.id - b.id);
  const col = h("section", { class: "col" },
    h("header", {}, h("span", { class: "bar", style: `background:${s.color}` }), s.label, h("span", { class: "count" }, tasks.length)),
    h("div", { class: "cards" }, tasks.map(taskCard)),
    can("member") && h("button", { class: "add", onclick: () => createTask(s.id) }, "+ Nueva tarea"));
  col.addEventListener("dragover", (e) => { e.preventDefault(); col.classList.add("over"); });
  col.addEventListener("dragleave", () => col.classList.remove("over"));
  col.addEventListener("drop", (e) => {
    e.preventDefault(); col.classList.remove("over");
    const id = +e.dataTransfer.getData("text/plain");
    const t = S.tasks.find((x) => x.id === id);
    if (t && can("member") && t.status !== s.id) { t.status = s.id; render(); patchTask(id, { status: s.id }); }
  });
  return col;
}

function calendarView() {
  const m = S.month, y = m.getFullYear(), mo = m.getMonth();
  const first = new Date(y, mo, 1);
  const start = new Date(y, mo, 1 - ((first.getDay() + 6) % 7)); // lunes
  const today = new Date().toDateString();
  const q = S.query.toLowerCase();
  const byDay = {};
  for (const t of S.tasks) {
    if (!t.due || t.parent_id || (q && !t.title.toLowerCase().includes(q))) continue;
    (byDay[new Date(t.due).toDateString()] ||= []).push(t);
  }
  const days = Array.from({ length: 42 }, (_, i) => new Date(start.getFullYear(), start.getMonth(), start.getDate() + i));
  const undated = S.tasks.filter((t) => !t.due && !t.parent_id && t.status !== "done").length;
  return h("div", { class: "calendar" },
    h("div", { class: "cal-head" },
      h("button", { onclick: () => { S.month = new Date(y, mo - 1, 1); render(); } }, "‹"),
      h("strong", {}, m.toLocaleDateString(undefined, { month: "long", year: "numeric" })),
      h("button", { onclick: () => { S.month = new Date(y, mo + 1, 1); render(); } }, "›"),
      h("button", { class: "ghost", onclick: () => { S.month = new Date(); render(); } }, "Hoy"),
      h("span", { class: "spacer" }), h("small", {}, `${undated} pendientes sin fecha`)),
    h("div", { class: "cal-grid" },
      ["L", "M", "X", "J", "V", "S", "D"].map((d) => h("div", { class: "cal-dow" }, d)),
      days.map((d) => h("div", { class: "cal-day" + (d.getMonth() !== mo ? " other" : "") + (d.toDateString() === today ? " today" : "") },
        h("span", { class: "num" }, d.getDate()),
        (byDay[d.toDateString()] || []).map((t) => h("div", {
          class: `cal-task p${t.priority}` + (t.status === "done" ? " done" : ""), title: t.title, onclick: () => openTask(t),
        }, t.title))))));
}

function appView() {
  const search = h("input", { placeholder: "Buscar…", value: S.query, oninput: (e) => { S.query = e.target.value; render(true); } });
  return h("div", { class: "shell" },
    h("aside", {},
      h("div", { class: "brand" }, "▣ Rust TUI Manager"),
      h("div", {}, h("h2", {}, "Equipos", h("span", {}, h("button", { class: "ghost", onclick: joinWithCode, title: "Unirse con un código" }, "↪"), h("button", { class: "ghost", onclick: createTeam, title: "Nuevo equipo" }, "+"))),
        h("div", { class: "nav" }, S.teams.map((t) => h("button", { class: t.id === S.team?.id ? "active" : "", onclick: () => selectTeam(t).catch((e) => toast(e.message, true)) }, t.name)))),
      S.team && h("div", {}, h("h2", {}, "Proyectos", h("button", { class: "ghost", onclick: createProject, title: "Nuevo proyecto" }, "+")),
        h("div", { class: "nav" }, S.projects.map((p) => h("button", { class: p.id === S.project?.id ? "active" : "", onclick: () => selectProject(p).catch((e) => toast(e.message, true)) },
          h("span", { class: "dot", style: `background:${p.color}` }), p.name)))),
      h("div", { class: "me" }, h("span", { class: "live" + (S.online ? " on" : ""), title: S.online ? "en vivo" : "desconectado" }),
        S.user?.username, h("span", { class: "spacer" }),
        h("button", { class: "ghost bell" + (S.unread ? " has" : ""), title: "Bandeja de novedades (Ctrl+T n)", onclick: () => loadInbox(false) }, S.unread ? `🔔 ${S.unread}` : "🔔"),
        h("button", { class: "ghost", onclick: helpModal, title: "Atajos y ayuda (?)" }, "?"),
        h("button", { class: "ghost", onclick: openSettings, title: "Configuración (,)" }, "⚙"),
        h("button", { class: "ghost", onclick: logout }, "Salir"),
        h("button", { class: "ghost", onclick: logoutAll, title: "Cierra la sesión en todos los dispositivos" }, "⏻"))),
    h("main", {},
      h("div", { class: "top" },
        h("h1", {}, S.project ? S.project.name : S.team ? "Sin proyectos" : "Creá un equipo para empezar"),
        h("div", { class: "members" }, S.members.map((m) => avatar(m.username))),
        S.team && h("button", { onclick: openTeam }, can("admin") ? "Equipo · Invitar" : "Equipo"),
        S.team && can("member") && h("button", { onclick: sendTeamMessage, title: "Mensaje al equipo (Ctrl+T s)" }, "✉ Mensaje"),
        h("div", { class: "seg" },
          [["board", "Tablero"], ["calendar", "Calendario"], ["gantt", "Gantt"], ["canvas", "Pizarra"]].map(([v, l]) =>
            h("button", { class: S.view === v ? "on" : "", onclick: () => { S.view = v; render(); } }, l))),
        search),
      S.project ? (S.view === "calendar" ? calendarView() : S.view === "gantt" ? ganttView() : S.view === "canvas" ? canvasView() : h("div", { class: "board" }, STATUS.map(column)))
        : h("div", { class: "empty" }, S.team ? "Creá un proyecto con + en la barra lateral" : "Usá + junto a Equipos")));
}

function render(keepFocus) {
  const root = document.getElementById("app");
  const focused = keepFocus && document.activeElement?.tagName === "INPUT";
  root.replaceChildren(S.token ? appView() : loginView());
  if (focused) { const i = root.querySelector(".top input"); i?.focus(); i?.setSelectionRange(S.query.length, S.query.length); }
}

// ---------- Telegram y mensajes al equipo ----------
const sendTeamMessage = guard(async () => {
  const others = S.members.filter((m) => m.id !== S.user?.id);
  const f = await form("Mensaje al equipo", [
    { name: "text", label: "Mensaje (llega a la bandeja y, si lo tienen, por email y Telegram)", type: "textarea", max: 1000 },
    { name: "to", label: "Para", type: "select", options: [["", "Todo el equipo"], ...others.map((m) => [m.id, m.username])] },
  ], "Enviar");
  if (!f) return;
  const r = await api("POST", `/api/teams/${S.team.id}/messages`, { text: f.text, to: f.to ? [+f.to] : [] });
  toast(`Mensaje enviado a ${r.sent} ${r.sent === 1 ? "persona" : "personas"}`);
});

const linkTelegram = guard(async () => {
  const l = await api("POST", "/api/me/telegram/link");
  document.getElementById("tg-modal")?.remove();
  const close = () => document.getElementById("tg-modal")?.remove();
  document.body.append(h("div", { class: "overlay", id: "tg-modal", onmousedown: (e) => e.target.id === "tg-modal" && close() },
    h("div", { class: "modal small" },
      h("h3", {}, "Vincular Telegram"),
      h("p", {}, "1. Abrí el bot ", h("a", { href: `https://t.me/${l.bot}`, target: "_blank", rel: "noopener noreferrer" }, "@" + l.bot), " en Telegram."),
      h("p", {}, "2. Mandale este mensaje:"),
      h("div", { class: "invite-out" }, h("code", {}, `/start ${l.code}`),
        h("button", { onclick: () => navigator.clipboard?.writeText(`/start ${l.code}`).then(() => toast("Copiado")) }, "Copiar")),
      h("small", {}, `El código sirve una sola vez y vence en ${l.expires_minutes} minutos. Cuando el bot responda ✅, cerrá esta ventana.`),
      h("div", { class: "row" }, h("button", { class: "primary", onclick: close }, "Listo")))));
});

// ---------- configuración ----------
const IS_WEB = /^https?:$/.test(location.protocol) && location.hostname !== "tauri.localhost";
const VIEWS = [["board", "Tablero"], ["calendar", "Calendario"], ["gantt", "Gantt"], ["canvas", "Pizarra"]];

const openSettings = guard(async () => {
  document.getElementById("settings-modal")?.remove();
  S.me = await api("GET", "/api/me");
  const me = S.me, close = () => document.getElementById("settings-modal")?.remove();
  const go = (fn) => () => { close(); fn(); };
  const sec = (title, ...kids) => h("section", { class: "cfg-sec" }, h("h3", {}, title), ...kids);
  const line = (label, ...ctl) => h("div", { class: "cfg-line" }, h("span", { class: "grow" }, label), ...ctl);

  const view = h("select", { onchange: (e) => { store.set("view", e.target.value); S.view = e.target.value; render(); } },
    VIEWS.map(([v, l]) => h("option", { value: v, selected: S.view === v }, l)));
  const server = h("input", { value: S.server, disabled: IS_WEB, maxLength: 200 });
  const tgState = !me.telegram_available ? "el servidor no tiene el bot configurado" : me.telegram_linked ? "✓ vinculado" : "sin vincular";
  const emailState = !me.email ? "sin cargar" : me.email_verified ? `✓ ${me.email}` : `${me.email} (sin verificar)`;

  document.body.append(h("div", { class: "overlay", id: "settings-modal", onmousedown: (e) => e.target.id === "settings-modal" && close() },
    h("div", { class: "modal" },
      h("h2", {}, "Configuración"),
      sec("Cuenta",
        line(`Usuario: ${me.username}`),
        line(`Email para avisos: ${emailState}`, h("button", { onclick: go(openProfile) }, "Configurar")),
        line(`Telegram: ${tgState}`, me.telegram_available && (me.telegram_linked
          ? h("button", { class: "danger", onclick: guard(async () => { await api("DELETE", "/api/me/telegram"); close(); openSettings(); }) }, "Desvincular")
          : h("button", { onclick: () => { close(); linkTelegram(); } }, "Vincular"))),
        line("Bandeja de novedades", h("button", { onclick: go(() => loadInbox(false)) }, S.unread ? `Abrir (${S.unread})` : "Abrir")),
        line("Cerrar sesión en este dispositivo", h("button", { onclick: () => { close(); logout(); } }, "Salir")),
        line("Cerrar sesión en todos los dispositivos", h("button", { class: "danger", onclick: () => { close(); logoutAll(); } }, "Cerrar todas"))),
      S.team && sec(`Equipo «${S.team.name}»`,
        line(`Tu rol: ${ROLES.find(([r]) => r === myRole())?.[1]}`),
        can("member") && line("Enviar un mensaje al equipo", h("button", { onclick: go(sendTeamMessage) }, "Escribir")),
        line("Miembros, roles e invitaciones", h("button", { onclick: go(openTeam) }, "Administrar")),
        line("Unirme a otro equipo con un código", h("button", { onclick: go(joinWithCode) }, "Unirme")),
        line("Crear un equipo nuevo", h("button", { onclick: go(createTeam) }, "Crear"))),
      S.team && sec("Proyecto",
        line(S.project ? `Proyecto actual: ${S.project.name}` : "Sin proyectos", h("button", { onclick: go(createProject) }, "Nuevo proyecto"))),
      sec("Preferencias",
        line("Vista inicial", view),
        line("Servidor", server),
        IS_WEB ? h("small", {}, "Estás usando la web del propio servidor: la dirección no se cambia desde acá.")
          : h("div", { class: "row" }, h("button", { onclick: guard(async () => {
            const url = server.value.trim().replace(/\/$/, ""); if (!/^https?:\/\//.test(url)) throw new Error("la dirección debe empezar con http:// o https://");
            S.server = url; store.set("server", url); toast("Servidor guardado: volvé a iniciar sesión"); close(); logout();
          }) }, "Guardar servidor y reiniciar sesión"))),
      sec("Ayuda", line("Atajos de teclado (también con ?)", h("button", { onclick: go(helpModal) }, "Ver atajos"))),
      h("div", { class: "row" }, h("button", { class: "primary", onclick: close }, "Listo")))));
});

// ---------- ayuda con buscador y atajos de teclado ----------
const SHORTCUTS = [
  ["?", "Abrir esta ayuda"], ["/", "Ir al buscador"], ["n", "Nueva tarea (en Por hacer)"],
  ["1", "Vista Tablero"], ["2", "Vista Calendario"], ["3", "Vista Gantt"], ["4", "Vista Pizarra"],
  ["Esc", "Cerrar ventana"], [",", "Configuración"], ["Ctrl+T c", "Configuración"], ["Ctrl+T s", "Enviar un mensaje al equipo"], ["Ctrl+T g", "Vincular Telegram"],
  ["Ctrl+T m", "Equipo: miembros y agregar con búsqueda"], ["Ctrl+T i", "Equipo: generar código de invitación"],
  ["Ctrl+T j", "Unirse a un equipo con código"], ["Ctrl+T t", "Crear equipo"], ["Ctrl+T p", "Crear proyecto"],
  ["Ctrl+T e", "Mi email para recibir avisos"], ["Ctrl+T n", "Bandeja de novedades y mis tareas"],
  ["Arrastrar tarjeta", "Cambiar de estado (Tablero)"], ["Arrastrar barra", "Mover fechas (Gantt)"],
  ["Doble clic en la pizarra", "Nueva nota en ese lugar"], ["↗ en un elemento", "Conectar dos elementos (Pizarra)"],
];
function helpModal() {
  document.getElementById("help-modal")?.remove();
  const q = h("input", { placeholder: "Buscar un atajo o función…", autofocus: true });
  const list = h("div", { class: "help-list" });
  const draw = () => {
    const s = q.value.toLowerCase();
    const rows = SHORTCUTS.filter(([k, d]) => !s || k.toLowerCase().includes(s) || d.toLowerCase().includes(s));
    list.replaceChildren(...(rows.length ? rows.map(([k, d]) => h("div", { class: "help-row" }, h("kbd", {}, k), h("span", {}, d))) : [h("small", {}, "Sin coincidencias")]));
  };
  q.addEventListener("input", draw); draw();
  document.body.append(h("div", { class: "overlay", id: "help-modal", onmousedown: (e) => e.target.id === "help-modal" && e.target.remove() },
    h("div", { class: "modal" }, h("h3", {}, "Atajos"), q, list, h("div", { class: "row" }, h("button", { onclick: () => document.getElementById("help-modal")?.remove() }, "Cerrar")))));
  q.focus();
}

document.addEventListener("keydown", (e) => {
  if (!S.token) return;
  const typing = /^(INPUT|TEXTAREA|SELECT)$/.test(document.activeElement?.tagName);
  if (e.key === "Escape") {
    const top = [...document.querySelectorAll(".overlay")].pop();
    if (top) { top.id === "inbox-modal" ? closeInbox() : top.id === "modal" ? closeModal() : top.remove(); }
    S.prefix = false; return;
  }
  if (e.ctrlKey && e.key.toLowerCase() === "t") { e.preventDefault(); S.prefix = true; toast("Ctrl+T ▸ m miembros · i invitar · j unirse · t equipo · p proyecto · e email · n bandeja · s mensaje · g Telegram · c config"); return; }
  if (S.prefix) {
    S.prefix = false; e.preventDefault();
    const act = { m: () => S.team && openTeam(), i: () => S.team && openTeam(), j: joinWithCode, t: createTeam, p: () => S.team && createProject(), e: openProfile, n: () => loadInbox(false), c: openSettings, s: () => S.team && can("member") && sendTeamMessage(), g: linkTelegram }[e.key.toLowerCase()];
    act ? act() : toast("Atajo de equipo desconocido", true);
    return;
  }
  if (typing || e.ctrlKey || e.metaKey || e.altKey) return;
  if (e.key === "?") { e.preventDefault(); helpModal(); }
  else if (e.key === ",") { e.preventDefault(); openSettings(); }
  else if (e.key === "/") { e.preventDefault(); document.querySelector(".top input")?.focus(); }
  else if (e.key === "n" && S.project && can("member")) { e.preventDefault(); createTask("todo"); }
  else if (["1", "2", "3", "4"].includes(e.key)) { S.view = ["board", "calendar", "gantt", "canvas"][+e.key - 1]; render(); }
});

render();
if (S.token) boot(true).catch(() => { S.token = null; render(); });
