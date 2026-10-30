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
  panel: store.get("panel") === "1", overview: null,
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
const TIPS = {
 "Salir": "Cierra tu sesión en este dispositivo",
 "Cerrar": "Cierra esta ventana",
 "Cancelar": "Descarta los cambios y cierra",
 "Guardar": "Guarda los cambios",
 "Borrar": "Elimina esta tarea; no se puede deshacer",
 "Mapa mental": "Abre el mapa mental de esta tarea",
 "+ Nueva tarea": "Crea una tarea en esta columna",
 "Entrar": "Inicia sesión con tu usuario y contraseña",
 "Crear cuenta": "Registra un usuario nuevo en este servidor",
 "Hoy": "Vuelve al mes actual",
 "Actualizar": "Vuelve a cargar los datos",
 "Copiar": "Copia al portapapeles",
 "Revocar": "Anula este código: ya nadie podrá usarlo",
 "Quitar": "Quita este dato de tu cuenta",
 "Mi email": "Configura el email donde te llegan los avisos",
 "Marcar como leído y cerrar": "Marca todas las novedades como leídas",
 "✎ Editar datos": "Cambia el nombre, la descripción y el color del equipo",
 "➕ Agregar / invitar": "Agrega miembros o genera un código de invitación",
 "Transferir la propiedad": "Pasa el equipo a otro miembro; vos quedás como admin",
 "Eliminar el equipo": "Borra el equipo con todos sus proyectos y tareas",
 "Nuevo token": "Genera un token nuevo; el anterior deja de funcionar",
 "Revocar token": "Desactiva el acceso de esta IA hasta que le generes otro token",
 "Copiar token": "Copia el token de la IA (se muestra una sola vez)",
 "Copiar comando": "Copia el comando para lanzar tm-agent con este token",
 "Copiar configuración": "Copia la configuración MCP para el cliente de IA",
 "Ver instructivo": "Explica paso a paso cómo conectar una IA",
 "Administrar": "Miembros, roles, ejecutores e invitaciones",
 "Unirme": "Entra a otro equipo con un código de invitación",
 "Nuevo proyecto": "Crea un proyecto con su carpeta y repositorio",
 "Duplicar": "Copia el proyecto como plantilla (tareas, etiquetas y dependencias)",
 "CSV": "Descarga las tareas para abrir en una planilla",
 "JSON": "Descarga las tareas en formato JSON",
 "Ver atajos": "Lista de atajos de teclado con buscador",
 "Escribir": "Manda un mensaje a todo el equipo",
 "Vincular": "Conecta tu cuenta con Telegram para recibir avisos",
 "Desvincular": "Quita Telegram de tu cuenta",
 "Cerrar todas": "Cierra tu sesión en todos los dispositivos",
 "Configurar": "Configura el email para los avisos",
 "Entendido": "Cierra la ayuda",
 "Listo": "Cierra esta ventana",
 "Descargar": "Descarga el archivo adjunto",
 "Crear": "Crea un equipo nuevo",
 "‹": "Mes anterior",
 "›": "Mes siguiente"
};
function h(tag, attrs = {}, ...kids) {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs || {})) {
    if (v == null || v === false) continue;
    if (typeof v === "string" && (k === "title" || k === "placeholder" || k === "aria-label" || k === "alt")) el.setAttribute(k, tr(v));
    else if (k.startsWith("on")) el.addEventListener(k.slice(2), v);
    else if (k === "class") el.className = v;
    else if (k === "style") el.style.cssText = v;
    else if (k in el) el[k] = v;
    else el.setAttribute(k, v);
  }
  for (const kid of kids.flat()) if (kid != null && kid !== false) el.append(typeof kid === "string" ? tr(kid) : kid);
  // tooltip automático para los botones que no traen uno propio
  if (tag === "button" && !el.title) { const tip = TIPS[kids.flat().find((k) => typeof k === "string")?.trim()]; if (tip) el.title = tr(tip); }
  return el;
}

function toast(msg, error = false) {
  const t = document.getElementById("toast");
  t.textContent = tr(msg); t.className = error ? "error" : ""; t.hidden = false;
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

// cuerpo binario (adjuntos); devuelve la respuesta cruda
async function apiRaw(method, path, body, type = "application/octet-stream") {
  const res = await fetch(S.server + path, { method, headers: { "Content-Type": type, ...(S.token ? { Authorization: "Bearer " + S.token } : {}) }, body });
  if (!res.ok) { const j = await res.json().catch(() => ({})); throw new Error(j.error || res.statusText); }
  return res;
}
async function saveFile(path, filename) {
  const blob = await (await apiRaw("GET", path)).blob();
  const a = h("a", { href: URL.createObjectURL(blob), download: filename });
  document.body.append(a); a.click(); a.remove(); setTimeout(() => URL.revokeObjectURL(a.href), 5000);
}
// Vista previa segura: el tipo se decide por los primeros bytes (no por la extensión ni por lo que diga el
// archivo) y solo se muestran imágenes raster y texto plano. Nunca HTML, SVG ni PDF.
function sniff(buf) {
  const b = new Uint8Array(buf.slice(0, 12)), is = (...s) => s.every((v, i) => b[i] === v);
  if (is(0x89, 0x50, 0x4e, 0x47)) return "image/png";
  if (is(0xff, 0xd8, 0xff)) return "image/jpeg";
  if (is(0x47, 0x49, 0x46, 0x38)) return "image/gif";
  if (is(0x52, 0x49, 0x46, 0x46) && b[8] === 0x57 && b[9] === 0x45 && b[10] === 0x42 && b[11] === 0x50) return "image/webp";
  return null;
}
const TEXT_EXT = /\.(txt|md|csv|log|json|ini|yml|yaml|toml|rs|js|py|sh)$/i;
const isPreviewable = (name) => /\.(png|jpe?g|gif|webp)$/i.test(name) || TEXT_EXT.test(name);
async function loadPreview(id, name) {
  const buf = await (await apiRaw("GET", `/api/attachments/${id}`)).arrayBuffer();
  const mime = sniff(buf);
  if (mime) return { kind: "image", url: URL.createObjectURL(new Blob([buf], { type: mime })) };
  if (TEXT_EXT.test(name)) {
    try { return { kind: "text", text: new TextDecoder("utf-8", { fatal: true }).decode(buf.slice(0, 200000)), cut: buf.byteLength > 200000 }; } catch {}
  }
  return null;
}
async function openPreviewRaw(a) {
  const p = await loadPreview(a.id, a.name);
  if (!p) throw new Error("este archivo no se puede mostrar: descargalo");
  document.getElementById("preview-modal")?.remove();
  const close = () => { document.getElementById("preview-modal")?.remove(); if (p.url) URL.revokeObjectURL(p.url); };
  document.body.append(h("div", { class: "overlay", id: "preview-modal", onmousedown: (e) => e.target.id === "preview-modal" && close() },
    h("div", { class: "modal wide", onkeydown: (e) => e.key === "Escape" && (e.stopPropagation(), close()) },
      h("h3", {}, a.name),
      p.kind === "image" ? h("img", { class: "preview-img", src: p.url, alt: a.name }) : h("pre", { class: "preview-text" }, p.text + (p.cut ? "\n… (se muestra el inicio)" : "")),
      h("div", { class: "row" }, h("button", { onclick: guard(() => saveFile(`/api/attachments/${a.id}`, a.name)) }, "Descargar"), h("button", { class: "primary", onclick: close }, "Cerrar")))));
}

const fmtSize = (n) => (n < 1024 ? `${n} B` : n < 1048576 ? `${(n / 1024).toFixed(1)} KB` : `${(n / 1048576).toFixed(1)} MB`);
const RECUR = [["", "No se repite"], ["daily", "Cada día"], ["weekly", "Cada semana"], ["monthly", "Cada mes"]];

const guard = (fn) => async (...a) => { try { await fn(...a); } catch (e) { toast(e.message, true); } };
const openPreview = guard(openPreviewRaw);
const color = (name) => AVATAR[[...name].reduce((a, c) => a + c.charCodeAt(0), 0) % AVATAR.length];
const botInfo = (name) => (S.bots || []).find((b) => b.username === name);
// una IA se ve con 🤖 y su modelo (ej. "claude:Opus"); las personas, con su nombre
const label = (name) => { const b = botInfo(name); return b ? `🤖 ${name}${b.model ? " · " + b.model : ""}` : name; };
const avatar = (name) => h("span", { class: "avatar" + (botInfo(name) ? " bot" : ""), title: label(name), style: `background:${color(name)}` }, botInfo(name) ? "🤖" : name.slice(0, 2).toUpperCase());

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
  if (first) { loadInbox(true); loadLicense(); }
}

// ---------- bandeja y perfil ----------
async function loadInbox(auto) {
  const [inbox, me] = await Promise.all([api("GET", "/api/inbox"), api("GET", "/api/me")]).catch(() => [null, null]);
  if (!inbox) return;
  S.inbox = inbox; S.me = me; S.unread = inbox.unread;
  if (!auto || inbox.unread > 0) openInbox();
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
  const KIND = { assigned: "📌", comment: "💬", member_added: "👥", role_changed: "🎖", message: "✉", mention: "@", reminder: "⏰" };
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
        h("button", { class: "primary", onclick: closeInbox }, S.unread ? "Marcar como leído y cerrar" : "Cerrar")))));
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
  S.portfolio = false; S.terms = false; S.team = team; S.projects = []; S.project = null; S.tasks = []; S.members = []; S.labels = []; S.bots = []; S.canvas = null;
  if (team) {
    [S.projects, S.members, S.labels, S.bots] = await Promise.all([
      api("GET", `/api/teams/${team.id}/projects`), api("GET", `/api/teams/${team.id}/members`),
      api("GET", `/api/teams/${team.id}/labels`), api("GET", `/api/teams/${team.id}/bots`).catch(() => []),
    ]);
    await selectProject(S.projects[0] || null);
    loadOverview();
  } else render();
}

async function selectProject(p) {
  S.portfolio = false; S.terms = false; S.project = p; S.canvas = null; S.tasks = p ? await api("GET", `/api/projects/${p.id}/tasks`) : [];
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
  if (S.portfolio) { clearTimeout(onEvent.pf); onEvent.pf = setTimeout(() => loadPortfolio(true), 800); }
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
  } else if (type === "project_deleted") {
    if (data.team_id === S.team?.id) {
      S.projects = S.projects.filter((p) => p.id !== data.id);
      if (S.project?.id === data.id) { selectProject(S.projects[0] || null).catch(() => {}); return; }
    }
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

const refreshMembers = () => api("GET", `/api/teams/${S.team.id}/members`).then((m) => { S.members = m; render(); document.getElementById("team-modal") && openTeam(); loadOverview(); }).catch(() => {});

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
  const f = await form("Nuevo proyecto", [
    { name: "name", label: "Nombre del proyecto", max: 60 },
    { name: "work_dir", label: "Carpeta de trabajo, dónde vive el proyecto (opcional)", max: 300, placeholder: "/home/ana/proyectos/app" },
    { name: "repo", label: "Repositorio (opcional)", max: 300, placeholder: "usuario/repo o URL" },
  ], "Crear"); if (!f) return;
  const p = await api("POST", `/api/teams/${S.team.id}/projects`, { name: f.name, work_dir: f.work_dir, repo: f.repo });
  if (!S.projects.some((x) => x.id === p.id)) S.projects.push(p);
  await selectProject(p);
});
// ---------- IAs (cuentas bot con token propio) ----------
const mcpConfig = (token) => JSON.stringify({ mcpServers: { "rust-tui-manager": { command: "tm-mcp", env: { TM_URL: S.server, TM_TOKEN: token } } } }, null, 2);
const copy = (text, msg) => navigator.clipboard?.writeText(text).then(() => toast(msg));
const agentCmd = (token) => `TM_URL=${S.server} \\\nTM_TOKEN=${token || "tmk_…"} \\\nTM_AGENT_CMD='claude -p "$TM_TASK_PROMPT" --permission-mode acceptEdits' \\\ntm-agent`;
function agentsGuide() {
  const overlay = h("div", { class: "overlay", onmousedown: (e) => e.target === overlay && overlay.remove() },
    h("div", { class: "modal" },
      h("h3", {}, "🤖 Que una IA trabaje sola: tm-agent"),
      h("p", {}, "Una IA solo actúa mientras algo la despierta. tm-agent es ese «algo»: corre en la máquina donde está la carpeta del proyecto, mira este servidor y, cuando le asignás una tarea a la IA, abre un agente en la carpeta de trabajo, comenta y pasa la tarea a En curso y Hecho."),
      h("ol", { class: "guide" },
        h("li", {}, "Creá la IA en la barra lateral (🤖 IAs → «+») y copiá su token."),
        h("li", {}, "Definí la carpeta de trabajo del proyecto (⚙ Proyecto): tiene que existir en la máquina donde corre tm-agent."),
        h("li", {}, "Instalá tm-agent en esa máquina: cargo install --git https://github.com/polblancoo/Rust_tuiManager rust-tui-manager-agent"),
        h("li", {}, "Ejecutalo con el token y el comando de tu agente (Claude Code, Gemini CLI, un script…). Ejemplo:"),
        h("pre", { class: "bot-cfg" }, agentCmd()),
        h("li", {}, "Asignale una tarea a la IA: en unos segundos pasa a En curso, queda comentada y termina en Hecho. Si falla, vuelve a Por hacer con el error.")),
      h("p", {}, "⚠ El texto de las tareas llega al agente como instrucciones: dejalo correr con una carpeta y permisos acotados, y asigná tareas solo desde cuentas de confianza. Instructivo completo (servicio, seguridad, variables): docs/AGENTES.md en el repositorio."),
      h("div", { class: "row" }, h("button", { onclick: () => copy(agentCmd(), "Comando copiado") }, "Copiar ejemplo"), h("button", { class: "primary", onclick: () => overlay.remove() }, "Cerrar"))));
  document.body.append(overlay);
}
function showBotToken(name, token) {
  const overlay = h("div", { class: "overlay", onmousedown: (e) => e.target === overlay && overlay.remove() },
    h("div", { class: "modal" },
      h("h3", {}, `🤖 Token de ${name}`),
      h("p", {}, "Copialo ahora: no se vuelve a mostrar. Si lo perdés, generás uno nuevo y el anterior deja de funcionar."),
      h("code", { class: "bot-token" }, token),
      h("div", { class: "row" }, h("button", { onclick: () => copy(token, "Token copiado") }, "Copiar token")),
      h("h3", {}, "Para que trabaje sola (tm-agent)"),
      h("pre", { class: "bot-cfg" }, agentCmd(token)),
      h("div", { class: "row" }, h("button", { onclick: () => copy(agentCmd(token), "Comando copiado") }, "Copiar comando"), h("button", { onclick: agentsGuide }, "Ver instructivo")),
      h("h3", {}, "Configuración para tu cliente MCP"),
      h("pre", { class: "bot-cfg" }, mcpConfig(token)),
      h("div", { class: "row" }, h("button", { onclick: () => copy(mcpConfig(token), "Configuración copiada") }, "Copiar configuración"),
        h("button", { class: "primary", onclick: () => overlay.remove() }, "Listo"))));
  document.body.append(overlay);
}
const loadBots = async () => { S.bots = await api("GET", `/api/teams/${S.team.id}/bots`).catch(() => []); render(); };
const createBot = guard(async () => {
  const f = await form("Nueva IA en el equipo", [
    { name: "name", label: "Nombre (ej. claude-ana)", max: 32 },
    { name: "model", label: "IA / modelo (ej. claude:Opus)", placeholder: "claude:Opus", max: 40 },
    { name: "role", label: "Rol", type: "select", value: "member", options: [["member", "Miembro (crea y edita sus tareas)"], ["viewer", "Lector (solo mira)"]] },
  ], "Crear"); if (!f) return;
  const r = await api("POST", `/api/teams/${S.team.id}/bots`, { name: f.name, role: f.role, model: f.model });
  S.members = await api("GET", `/api/teams/${S.team.id}/members`);
  await loadBots(); showBotToken(r.user.username, r.token);
});
const botModal = (b) => {
  const admin = can("admin");
  const overlay = h("div", { class: "overlay", onmousedown: (e) => e.target === overlay && overlay.remove() },
    h("div", { class: "modal small" },
      h("h3", {}, `🤖 ${b.username}`, b.model && h("small", { class: "inl" }, " · " + b.model)),
      h("p", {}, `${b.role === "viewer" ? "Lector" : "Miembro"} · token ${b.active ? "activo" : "revocado"} · ${b.last_used ? "último uso " + new Date(b.last_used).toLocaleString() : "sin usar"}${b.created_by ? " · creada por " + b.created_by : ""}`),
      h("div", { class: "row" },
        admin && h("button", { onclick: guard(async () => {
          const f = await form("IA / modelo", [{ name: "model", label: "Qué IA es (ej. claude:Opus)", value: b.model, max: 40 }], "Guardar"); if (!f) return;
          await api("PATCH", `/api/bots/${b.id}`, { model: f.model }); overlay.remove(); await loadBots();
        }) }, "Cambiar modelo"),
        admin && h("button", { onclick: guard(async () => {
          const f = await form("Comandos prohibidos", [{ name: "deny", label: "Uno por línea, ej. Bash(rm:*) · Bash(git push:*). La IA no puede ejecutarlos; si los necesita, te avisa con una alarma 🙋.", type: "textarea", value: b.deny || "", max: 4000 }], "Guardar"); if (!f) return;
          await api("PATCH", `/api/bots/${b.id}`, { deny: f.deny }); overlay.remove(); await loadBots(); toast("Lista guardada");
        }) }, "🚫 Comandos prohibidos"),
        admin && h("button", { onclick: guard(async () => { const r = await api("POST", `/api/bots/${b.id}/token`); overlay.remove(); await loadBots(); showBotToken(b.username, r.token); }) }, "Nuevo token"),
        admin && b.active && h("button", { class: "danger", onclick: guard(async () => { await api("DELETE", `/api/bots/${b.id}/token`); overlay.remove(); await loadBots(); toast("Token revocado"); }) }, "Revocar"),
        h("button", { onclick: () => overlay.remove() }, "Cerrar"))));
  document.body.append(overlay);
};
const editProject = guard(async () => {
  const p = S.project; if (!p) return;
  const field = (label, value, ph, max) => [h("input", { value, placeholder: ph || "", maxLength: max }), label];
  const [fn, fd, fw, fr] = [field("Nombre", p.name, "", 60), field("Descripción", p.description, "", 500),
    field("Carpeta de trabajo (dónde vive el proyecto)", p.work_dir, "/home/ana/proyectos/app", 300), field("Repositorio", p.repo, "usuario/repo o URL", 300)];
  const overlay = h("div", { class: "overlay", onmousedown: (e) => e.target === overlay && overlay.remove() },
    h("div", { class: "modal small", onkeydown: (e) => e.key === "Escape" && overlay.remove() },
      h("h3", {}, "Proyecto"),
      [fn, fd, fw, fr].map(([el, l]) => h("label", {}, l, el)),
      h("div", { class: "row" },
        h("button", { class: "danger", onclick: guard(async () => {
          const c = await form("Eliminar proyecto", [{ name: "confirm", label: `Se borran también sus tareas. Escribí «${p.name}» para confirmar`, max: 60 }], "Eliminar"); if (!c) return;
          await api("DELETE", `/api/projects/${p.id}?confirm=${encodeURIComponent(c.confirm)}`);
          overlay.remove(); S.projects = S.projects.filter((x) => x.id !== p.id); await selectProject(S.projects[0] || null); toast("Proyecto eliminado");
        }) }, "🗑 Eliminar"),
        h("span", { class: "spacer" }),
        h("button", { onclick: () => overlay.remove() }, "Cancelar"),
        h("button", { class: "primary", onclick: guard(async () => {
          if (!fn[0].value.trim()) { fn[0].focus(); return; }
          const u = await api("PATCH", `/api/projects/${p.id}`, { name: fn[0].value, description: fd[0].value, work_dir: fw[0].value, repo: fr[0].value });
          Object.assign(p, u); S.projects = S.projects.map((x) => (x.id === p.id ? p : x)); overlay.remove(); render();
        }) }, "Guardar"))));
  document.body.append(overlay); fn[0].focus();
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
    return h("div", { class: "member" }, avatar(m.username), h("span", { class: "grow" }, label(m.username)),
      admin && !/^claude|bot$/i.test(m.username) && h("label", { class: "exec", title: "Autorizado a ejecutar lo que piden las IAs (comandos, permisos). Los pedidos le llegan como tarea." },
        h("input", { type: "checkbox", checked: !!m.executor, onchange: guard(async (e) => {
          await api("PUT", `/api/teams/${S.team.id}/members/${m.id}/executor`, { executor: e.target.checked }); refreshMembers();
        }) }), " Ejecutor"),
      roleSel(m.role, guard(async (e) => {
        await api("PUT", `/api/teams/${S.team.id}/members/${m.id}`, { role: e.target.value }); refreshMembers();
      }), !manageable, RANK[myRole()]),
      (manageable || (m.id === S.user.id && m.role !== "owner")) && h("button", { class: "ghost danger", title: m.id === S.user.id ? "Salir del equipo" : "Quitar", onclick: guard(async () => {
        if (!confirm(m.id === S.user.id ? "¿Salir del equipo?" : `¿Quitar a ${m.username}?`)) return;
        await api("DELETE", `/api/teams/${S.team.id}/members/${m.id}`);
        if (m.id === S.user.id) { document.getElementById("team-modal")?.remove(); await boot(); } else refreshMembers();
      }) }, "✕"));
  });

  // primero lo que se hace seguido (agregar e invitar); la lista de miembros va al final
  const body = [h("h3", {}, `Equipo ${S.team.name}`)];
  if (!admin) body.push(h("small", { class: "warn" }, "Solo un admin o el dueño del equipo puede agregar miembros y generar códigos de invitación."));
  if (admin) {
    const user = h("input", { placeholder: "Buscar usuario…", maxLength: 32, autocomplete: "off" });
    const sug = h("div", { class: "user-results" });
    const hint = h("small", { class: "warn" });
    let lastQ = null;
    const search = async () => {
      const q = user.value.trim(); if (q === lastQ) return; lastQ = q;
      if (!q) { sug.replaceChildren(); hint.textContent = ""; return; }
      try {
        const us = await api("GET", `/api/users/search?q=${encodeURIComponent(q)}&team_id=${S.team.id}`);
        if (user.value.trim() !== q) return;
        sug.replaceChildren(...us.map((u) => h("button", { class: "ghost", onclick: () => { user.value = u.username; lastQ = null; sug.replaceChildren(); hint.textContent = ""; } },
          avatar(u.username), u.username)));
        hint.textContent = us.length ? "Tocá un usuario para elegirlo." : "No hay usuarios registrados con ese nombre (o ya están en el equipo): pedile que se registre o mandale un código de invitación.";
      } catch (e) { hint.textContent = e.message; }
    };
    user.addEventListener("input", search);
    const role = roleSel("member", null, false, RANK[myRole()]);
    const inv = h("div", { class: "invite-out" });
    const grant = ROLES.filter(([r]) => RANK[r] < RANK[myRole()] && r !== "owner");
    const invRole = h("select", {}, grant.map(([r, l]) => h("option", { value: r, selected: r === "member" }, l)));
    body.push(
      h("h3", {}, "Agregar por usuario (ya registrado)"),
      h("div", { class: "row" }, user, role, h("button", { class: "primary", onclick: guard(async () => {
        await api("POST", `/api/teams/${S.team.id}/members`, { username: user.value, role: role.value });
        user.value = ""; lastQ = null; sug.replaceChildren(); toast("Miembro agregado"); refreshMembers();
      }) }, "Agregar")),
      sug,
      hint,
      h("h3", {}, "Invitar con un código (si todavía no tiene cuenta)"),
      h("div", { class: "row" }, invRole, h("button", { onclick: guard(async () => {
        const r = await api("POST", `/api/teams/${S.team.id}/invites`, { role: invRole.value });
        const msg = inviteMessage(r.code);
        inv.replaceChildren(h("code", {}, r.code), h("button", { onclick: () => navigator.clipboard?.writeText(r.code).then(() => toast("Código copiado")) }, "Copiar"),
          h("button", { title: "Texto listo para pegar en WhatsApp, Telegram, etc.", onclick: () => navigator.clipboard?.writeText(msg).then(() => toast("Invitación copiada: pegala en un chat")) }, "Copiar invitación"),
          h("a", { class: "btnlike", href: `mailto:?subject=${encodeURIComponent("Invitación a Rust TUI Manager")}&body=${encodeURIComponent(msg)}` }, "Enviar por email"),
          h("small", {}, "Se muestra una sola vez · vence en 7 días · 10 usos"),
          /localhost|127\.0\.0\.1/.test(S.server) && h("small", { class: "warn" }, "Ojo: tu servidor es local (" + S.server + "); para que otra persona entre, tiene que ser una dirección que ella alcance."));
      }) }, "Generar código")),
      inv,
      invites.length > 0 && h("h3", {}, "Invitaciones activas"),
      ...invites.map((i) => h("div", { class: "member" }, h("span", { class: "grow" },
        `${ROLES.find(([r]) => r === i.role)[1]} · ${i.uses}/${i.max_uses} usos · vence ${new Date(i.expires_at).toLocaleDateString()}`),
        h("button", { class: "ghost danger", onclick: guard(async () => { await api("DELETE", `/api/invites/${i.id}`); openTeam(); }) }, "Revocar"))));
  }
  body.push(h("h3", {}, `Miembros (${S.members.length})`), h("div", { class: "members-list" }, rows));
  body.push(h("div", { class: "row" }, h("button", { onclick: () => document.getElementById("team-modal")?.remove() }, "Cerrar")));
  document.body.append(h("div", { class: "overlay", id: "team-modal", onmousedown: (e) => e.target.id === "team-modal" && e.target.remove() },
    h("div", { class: "modal" }, body)));
});

// ---------- panel del equipo (columna derecha) ----------
const VIA = { owner: "creó el equipo", direct: "lo sumó un admin", invite: "entró con un código" };
const fmtDay = (d) => (d ? new Date(d).toLocaleDateString() : "fecha desconocida");

const loadOverview = async () => {
  if (!S.panel || !S.team) return;
  const team = S.team.id;
  try {
    const o = await api("GET", `/api/teams/${team}/overview`);
    if (S.team?.id !== team) return;
    S.overview = o; render();
  } catch {}
};
const togglePanel = () => {
  S.panel = !S.panel; store.set("panel", S.panel ? "1" : null);
  S.panel ? loadOverview() : render();
};

function teamPanel() {
  const o = S.overview && S.overview.id === S.team.id ? S.overview : null;
  const box = h("section", { class: "team-panel" });
  if (!o) { box.append(h("small", {}, "Cargando…")); return box; }
  const mine = o.my_role, admin = RANK[mine] >= RANK.admin, owner = mine === "owner";
  const sec = (title, ...kids) => h("div", { class: "tp-sec" }, h("h4", {}, title), ...kids);
  const reload = async () => { const t = await api("GET", "/api/teams"); S.teams = t; S.team = t.find((x) => x.id === S.team.id) || S.team; await refreshMembers(); };

  const rolePick = (m) => {
    const can_edit = admin && m.role !== "owner" && RANK[m.role] < RANK[mine];
    if (!can_edit) return h("span", { class: "pill role-" + m.role }, ROLES.find(([r]) => r === m.role)[1]);
    return h("select", { onchange: guard(async (e) => { await api("PUT", `/api/teams/${o.id}/members/${m.id}`, { role: e.target.value }); }) },
      ROLES.filter(([r]) => r === m.role || (r !== "owner" && RANK[r] < RANK[mine])).map(([r, l]) => h("option", { value: r, selected: r === m.role }, l)));
  };
  const memberCard = (m) => h("div", { class: "tp-member" },
    h("div", { class: "tp-line" }, avatar(m.username), h("span", { class: "tp-name grow" }, m.bot ? "🤖 " : "", m.username, m.bot && m.model && h("small", { class: "inl" }, " · " + m.model), m.id === S.user.id && h("small", { class: "inl" }, " (vos)")), rolePick(m),
      ((admin && m.role !== "owner" && RANK[m.role] < RANK[mine]) || (m.id === S.user.id && m.role !== "owner")) &&
        h("button", { class: "ghost danger", title: m.id === S.user.id ? "Salir del equipo" : "Quitar del equipo", onclick: guard(async () => {
          if (!confirm(m.id === S.user.id ? "¿Salir del equipo?" : `¿Quitar a ${m.username}?`)) return;
          await api("DELETE", `/api/teams/${o.id}/members/${m.id}`);
          if (m.id === S.user.id) await boot();
        }) }, "✕")),
    h("small", {}, `Desde ${fmtDay(m.joined_at)}${m.via ? " · " + (m.invited_by && m.via !== "owner" ? `${VIA[m.via] || m.via} (${m.invited_by})` : VIA[m.via] || m.via) : ""}`),
    h("small", {}, m.open_tasks ? `📌 ${m.open_tasks} ${m.open_tasks === 1 ? "tarea abierta" : "tareas abiertas"}` : "Sin tareas abiertas"));

  const inviteCard = (i) => h("div", { class: "tp-invite" },
    h("div", { class: "tp-line" }, h("b", {}, ROLES.find(([r]) => r === i.role)[1]), h("small", {}, `${i.uses}/${i.max_uses} usos`), h("span", { class: "spacer" }),
      h("button", { class: "ghost danger", onclick: guard(async () => { await api("DELETE", `/api/invites/${i.id}`); loadOverview(); }) }, "Revocar")),
    h("small", {}, `Creada por ${i.created_by || "—"} · vence ${fmtDay(i.expires_at)}`),
    i.joined.length ? h("small", { class: "ok" }, "Entraron: " + i.joined.map((j) => `${j.username} (${fmtDay(j.joined_at)})`).join(", ")) : h("small", {}, "Todavía no entró nadie"));

  const edit = guard(async () => {
    const f = await form("Datos del equipo", [{ name: "name", label: "Nombre", value: o.name, max: 100 }, { name: "description", label: "Descripción (opcional)", type: "textarea", value: o.description, max: 500 }], "Guardar"); if (!f) return;
    await api("PATCH", `/api/teams/${o.id}`, { name: f.name, description: f.description });
    await reload();
  });
  const transfer = guard(async () => {
    const others = o.members.filter((m) => m.id !== S.user.id); if (!others.length) throw new Error("No hay otros miembros a quienes transferir");
    const f = await form("Transferir la propiedad", [{ name: "to", label: "Nuevo dueño (vos pasás a Admin)", type: "select", options: others.map((m) => [m.id, m.username]) }], "Transferir"); if (!f) return;
    await api("POST", `/api/teams/${o.id}/transfer`, { user_id: +f.to });
    toast("Propiedad transferida"); await reload();
  });
  const remove = guard(async () => {
    const f = await form(`Eliminar «${o.name}»`, [{ name: "confirm", label: `Se borran proyectos, tareas e invitaciones. Escribí el nombre del equipo para confirmar (${o.name})` }], "Eliminar"); if (!f) return;
    await api("DELETE", `/api/teams/${o.id}?confirm=${encodeURIComponent(f.confirm)}`);
    toast("Equipo eliminado"); await boot();
  });

  box.append(
    h("div", { class: "tp-head" }, h("h3", {}, o.name), h("span", { class: "spacer" }),
      h("button", { class: "ghost", title: "Cerrar panel", onclick: togglePanel }, "✕")),
    o.description && h("p", { class: "tp-desc" }, o.description),
    h("div", { class: "tp-stats" }, h("span", {}, h("b", {}, o.members.length), " miembros"), h("span", {}, h("b", {}, o.projects), " proyectos"), h("span", {}, h("b", {}, o.open_tasks), " tareas abiertas")),
    admin && h("div", { class: "row" }, h("button", { onclick: edit }, "✎ Editar datos"), h("button", { class: "primary", onclick: openTeam }, "➕ Agregar / invitar")),
    sec(`Miembros (${o.members.length})`, h("div", { class: "tp-list" }, o.members.map(memberCard))),
    admin && sec(`Invitaciones (${o.invites.length})`, o.invites.length ? h("div", { class: "tp-list" }, o.invites.map(inviteCard)) : h("small", {}, "No hay invitaciones. Generá un código con «Agregar / invitar».")),
    sec("Ajustes",
      h("div", { class: "tp-list" },
        !owner && h("button", { class: "danger", onclick: guard(async () => { if (!confirm("¿Salir del equipo?")) return; await api("DELETE", `/api/teams/${o.id}/members/${S.user.id}`); await boot(); }) }, "Salir del equipo"),
        owner && h("button", { onclick: transfer }, "Transferir la propiedad"),
        owner && h("button", { class: "danger", onclick: remove }, "Eliminar el equipo"),
        !admin && h("small", {}, "Solo un admin o el dueño puede cambiar los datos del equipo y gestionar miembros."))));
  return box;
}
const RELEASES_URL = "https://github.com/polblancoo/Rust_tuiManager/releases/latest";
const inviteMessage = (code) => [
  `Te invito al equipo «${S.team.name}» en Rust TUI Manager.`,
  "",
  `1. Instalá la app (Linux .deb/.rpm/AppImage): ${RELEASES_URL}`,
  `   o abrila directo en el navegador: ${S.server}`,
  `2. Servidor: ${S.server}`,
  "3. Creá tu cuenta y tocá «Unirme con un código» (el ↪ de la barra lateral).",
  `4. Pegá este código: ${code}`,
  "",
  "El código vence en 7 días.",
].join("\n");

const createTask = guard(async (status) => {
  // mismas reglas que en el modal: un líder asigna a cualquiera; un miembro solo a sí mismo
  const assignable = can("admin") ? S.members.filter((m) => RANK[m.role] >= 1) : S.members.filter((m) => m.id === S.user?.id && can("member"));
  const f = await form("Nueva tarea", [
    { name: "title", label: "Título", max: 200 },
    { name: "description", label: "Descripción (opcional)", type: "textarea" },
    { name: "status", label: "Estado", type: "select", options: STATUS.map((s) => [s.id, s.label]), value: status },
    { name: "priority", label: "Prioridad", type: "select", options: PRIO.map((p, i) => [i, p]), value: 1 },
    ...(assignable.length ? [{ name: "assignee", label: "Responsable", type: "select", options: [["", "Sin asignar"], ...assignable.map((m) => [m.id, m.id === S.user?.id && !can("admin") ? `Yo (${m.username})` : label(m.username)])] }] : []),
    { name: "start", label: "Empieza (opcional)", type: "date" },
    { name: "due", label: "Vence (opcional)", type: "date" },
    { name: "recurrence", label: "Repetición", type: "select", options: RECUR },
  ], "Crear tarea"); if (!f) return;
  const day = (d) => (d ? new Date(d + "T12:00:00Z").toISOString() : undefined);
  let t = await api("POST", `/api/projects/${S.project.id}/tasks`, { title: f.title, description: f.description, priority: +f.priority, assignee_id: f.assignee ? +f.assignee : undefined, start: day(f.start), due: day(f.due), recurrence: f.recurrence || undefined });
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

// resalta @usuario cuando coincide con un miembro del equipo (todo como texto, nunca como HTML)
function withMentions(body) {
  const names = new Set(S.members.map((m) => m.username.toLowerCase()));
  return body.split(/(@[\p{L}\p{N}_.-]+)/u).map((part) =>
    part.startsWith("@") && names.has(part.slice(1).replace(/[.-]+$/, "").toLowerCase()) ? h("b", { class: "mention" }, part) : part);
}

// comandos entre comillas o `backticks` de un pedido 🙋 de una IA, para copiar y pegar en una terminal
function commandsOf(body) {
  if (!body.startsWith("🙋")) return [];
  const out = [...body.matchAll(/`([^`\n]{2,300})`|'([^'\n]{4,300})'/g)].map((m) => (m[1] || m[2]).trim());
  return [...new Set(out)].slice(0, 4);
}

function renderComments() {
  const box = document.getElementById("comments"); if (!box) return;
  box.replaceChildren(...S.comments.map((c) => h("div", { class: "comment" },
    h("b", {}, c.username), h("small", {}, new Date(c.created_at).toLocaleString()), h("div", {}, withMentions(c.body)),
    commandsOf(c.body).map((cmd) => h("div", { class: "cmdbox" }, h("code", {}, cmd),
      h("button", { class: "ghost", title: "Copiar el comando", onclick: () => navigator.clipboard?.writeText(cmd).then(() => toast("Comando copiado")) }, "📋 Copiar"))))));
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
    (leader ? pickable : [...pickable, ...S.members.filter((m) => m.id === ass && m.id !== meId)]).map((m) => h("option", { value: m.id, selected: ass === m.id }, m.id === meId && !leader ? `Yo (${m.username})` : label(m.username))));
  const whoHint = leader ? "Como líder podés asignarla a cualquier miembro" : can("member") ? "Podés tomar una tarea libre o soltar la tuya" : "";
  const start = h("input", { type: "date", value: cur().start ? cur().start.slice(0, 10) : "" });
  const rec = h("select", { disabled: !can("member") }, RECUR.map(([v, l]) => h("option", { value: v, selected: (cur().recurrence || "") === v }, l)));
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
  // adjuntos: hasta 10 por tarea y 2 MB cada uno; siempre se descargan, nunca se abren en la página
  const attBox = h("div", { class: "subs" });
  // miniatura de imágenes: se carga con la sesión y se valida por bytes; si no es imagen, no se muestra
  const thumb = (a) => {
    const img = h("img", { class: "thumb", alt: "", hidden: true, onclick: () => openPreview(a) });
    loadPreview(a.id, a.name).then((p) => { if (p?.kind === "image") { img.src = p.url; img.hidden = false; } }).catch(() => {});
    return img;
  };
  const drawAtts = async () => {
    const list = await api("GET", `/api/tasks/${task.id}/attachments`).catch(() => []);
    attBox.replaceChildren(...list.map((a) => h("div", { class: "sub" },
      /\.(png|jpe?g|gif|webp)$/i.test(a.name) ? thumb(a) : null,
      h("span", {}, "📎 " + a.name), h("small", {}, `${fmtSize(a.size)} · ${a.username}`),
      isPreviewable(a.name) && h("button", { class: "ghost", title: "Ver", onclick: () => openPreview(a) }, "👁"),
      h("button", { class: "ghost", title: "Descargar", onclick: guard(() => saveFile(`/api/attachments/${a.id}`, a.name)) }, "⬇"),
      (can("admin") || a.user_id === S.user?.id) && can("member") && h("button", { class: "ghost danger", title: "Borrar", onclick: guard(async () => {
        await api("DELETE", `/api/attachments/${a.id}`); drawAtts();
      }) }, "✕"))));
  };
  const attAdd = h("input", { type: "file", disabled: !can("member"), onchange: guard(async (e) => {
    const f = e.target.files[0]; if (!f) return;
    if (f.size > 2 * 1024 * 1024) { e.target.value = ""; throw new Error("el archivo supera los 2 MB"); }
    await apiRaw("POST", `/api/tasks/${task.id}/attachments?name=${encodeURIComponent(f.name)}`, f);
    e.target.value = ""; drawAtts(); toast("Archivo adjuntado");
  }) });
  const send = guard(async () => {
    if (!comment.value.trim()) return;
    const c = await api("POST", `/api/tasks/${task.id}/comments`, { body: comment.value });
    comment.value = "";
    if (!S.comments.some((x) => x.id === c.id)) S.comments.push(c);
    renderComments();
  });
  comment.addEventListener("keydown", (e) => e.key === "Enter" && send());

  // tarea de pedido de una IA («🙋 Pedido de #N: …»): comandos para copiar, dónde pegar la salida y botones de respuesta
  const requestPanel = () => {
    if (!/^🙋 Pedido de #\d+:/.test(cur().title) || !can("member")) return null;
    const cmds = (cur().description.match(/COMANDOS PARA EJECUTAR[^\n]*\n[^\n]*\n\n([\s\S]*?)\n\n/) || [])[1]?.split("\n").filter(Boolean) || [];
    const out = h("textarea", { rows: 5, maxLength: 3500, placeholder: "Pegá acá lo que mostró la terminal (la salida del comando)…" });
    const reply = guard(async (body, done) => {
      const c = await api("POST", `/api/tasks/${task.id}/comments`, { body });
      if (!S.comments.some((x) => x.id === c.id)) S.comments.push(c);
      renderComments();
      if (done) { upsert(await api("PATCH", `/api/tasks/${task.id}`, { status: "done" })); render(); toast("Respuesta enviada a la IA; pedido marcado como Hecho"); }
      else toast("Respuesta enviada a la IA");
    });
    const grantCmd = guard(async () => {
      const ask = (cur().description.match(/LO QUE PIDE\n([\s\S]*?)\n\n/) || [])[1] || "";
      const found = [...new Set([...cmds, ...[...ask.matchAll(/`([^`\n]+)`/g)].map((m) => m[1])])].map((c) => c.replace(/^cd [^&;]+&&\s*/, "").trim()).filter(Boolean);
      const r = await grantDialog(found, ask);
      if (!r) return;
      await api("POST", `/api/tasks/${task.id}/grant`, { command: r.command, always: r.always });
      await reply(`✅ Autorizado: \`${r.command}\` (${r.always ? "siempre" : "una vez"}). La IA lo puede usar en su próxima ejecución.`, true);
    });
    return h("div", { class: "reqpanel" },
      h("h3", {}, "🙋 Pedido de una IA"),
      cmds.length > 0 && h("div", {}, h("b", {}, "1. Copiá y ejecutá en tu terminal:"),
        cmds.map((cmd) => h("div", { class: "cmdbox" }, h("code", {}, cmd),
          h("button", { class: "ghost", title: "Copiar el comando", onclick: () => navigator.clipboard?.writeText(cmd).then(() => toast("Comando copiado")) }, "📋 Copiar")))),
      h("div", {}, h("b", {}, cmds.length ? "2. Pegá la salida acá y enviala:" : "Respondé acá:"), out,
        h("small", { class: "hint" }, "Se publica como comentario de esta tarea y le llega sola a la IA (también queda en su tarea original).")),
      h("div", { class: "row" },
        h("button", { class: "primary", title: "Publica la salida como comentario y marca el pedido como Hecho", onclick: () => out.value.trim() ? reply("📤 Salida:\n" + out.value.trim(), true).then(() => (out.value = "")) : toast("Pegá primero la salida", true) }, "📤 Enviar salida a la IA"),
        h("span", { class: "spacer" }),
        h("button", { title: "Le responde a la IA que sí: úsalo cuando el pedido es una decisión", onclick: () => reply("✅ Autorizado: seguí.", true) }, "✅ Autorizar"),
        can("admin") && h("button", { class: "primary", title: "Te muestra qué comando pide la IA, qué hace, y elegís si lo permitís solo esta vez o siempre", onclick: () => grantCmd() }, "✅ Permitir comando…"),
        h("button", { class: "danger", title: "Le responde a la IA que no; podés escribir el motivo arriba", onclick: () => reply("❌ No autorizado." + (out.value.trim() ? "\n" + out.value.trim() : ""), true).then(() => (out.value = "")) }, "❌ Rechazar")),
      h("small", { class: "hint" }, "«Autorizar» es para decisiones (¿puedo borrar X?). Si la IA no puede ejecutar un comando porque está bloqueado, ejecutalo vos y enviá la salida."));
  };

  const save = guard(async () => {
    await patchTask(task.id, {
      title: title.value, description: desc.value, priority: +prio.value, status: status.value,
      ...(+(who.value || 0) !== (cur().assignee_id || 0) ? { assignee_id: who.value ? +who.value : null } : {}),
      due: due.value ? new Date(due.value + "T12:00:00Z").toISOString() : null,
      start: start.value ? new Date(start.value + "T12:00:00Z").toISOString() : null,
      recurrence: rec.value || null,
    });
    closeModal();
  });
  const del = guard(async () => {
    if (!confirm("¿Borrar la tarea?")) return;
    await api("DELETE", `/api/tasks/${task.id}`);
    S.tasks = S.tasks.filter((t) => t.id !== task.id); closeModal(); render();
  });

  const modal = h("div", { class: "overlay", id: "modal", onmousedown: (e) => e.target.id === "modal" && closeModal() },
    h("div", { class: "modal task-modal" + (store.get("taskMax") === "1" ? " max" : "") },
      h("h3", { class: "task-head" }, "Tarea ", h("span", { class: "tid" }, "#" + task.id), h("span", { class: "spacer" }),
        can("owner") && h("button", { class: "ghost", title: "Abre una terminal SSH a la máquina del proyecto, sin salir de la tarea (solo owners)", onclick: guard(() => openTerminal(cur())) }, "💻 Terminal"),
        h("button", { class: "ghost max-btn", title: "Maximizar o restaurar la ventana para leer los comentarios más cómodo", onclick: (e) => {
          const m = e.currentTarget.closest(".modal"); const on = m.classList.toggle("max"); store.set("taskMax", on ? "1" : "0");
          e.currentTarget.textContent = on ? "⤡ Restaurar" : "⤢ Maximizar";
        } }, store.get("taskMax") === "1" ? "⤡ Restaurar" : "⤢ Maximizar")),
      requestPanel(),
      h("label", {}, "Título", title),
      h("label", {}, "Descripción", desc),
      can("member") && budgetBox(cur(), title, desc),
      h("h3", {}, "Etiquetas"), labelBox, newLabel,
      h("h3", {}, "Subtareas"), subBox, newSub,
      h("div", { class: "grid2" },
        h("label", {}, "Estado", status), h("label", {}, "Prioridad", prio),
        h("label", {}, "Responsable", who), h("label", {}, "Empieza", start),
        h("label", {}, "Vence", due), h("label", {}, "Repetición", rec), h("small", { class: "hint" }, whoHint)),
      h("h3", {}, "Depende de"), depBox, depAdd,
      h("h3", {}, "Adjuntos"), attBox, attAdd,
      h("h3", {}, "Comentarios"),
      h("div", { class: "comments", id: "comments" }),
      comment,
      h("div", { class: "row" },
        can("member") && h("button", { class: "danger", onclick: del }, "Borrar"),
        h("button", { onclick: () => { closeModal(); openMindmap(cur()); } }, "Mapa mental"), h("span", { class: "spacer" }),
        h("button", { onclick: closeModal }, "Cancelar"), h("button", { class: "primary", onclick: save }, "Guardar"))));
  document.body.append(modal);
  drawLabels(); drawSubs(); drawDeps(); drawAtts();
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
    h("div", { class: "title" }, h("span", { class: "tid" }, "#" + t.id), " ", t.title),
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
  card.addEventListener("dragover", (e) => { if (can("member")) { e.preventDefault(); e.stopPropagation(); card.classList.add("drop-before"); } });
  card.addEventListener("dragleave", () => card.classList.remove("drop-before"));
  card.addEventListener("drop", (e) => { e.preventDefault(); e.stopPropagation(); card.classList.remove("drop-before"); moveTask(+e.dataTransfer.getData("text/plain"), t.status, t.id); });
  return card;
}

// mueve una tarea a la columna `status`, justo antes de `beforeId` (o al final); el orden manual manda sobre la prioridad
function moveTask(id, status, beforeId) {
  const t = S.tasks.find((x) => x.id === id);
  if (!t || !can("member") || id === beforeId) return;
  const col = S.tasks.filter((x) => !x.parent_id && x.status === status && x.id !== id).sort(byOrder);
  const i = beforeId == null ? col.length : col.findIndex((x) => x.id === beforeId);
  if (i < 0) return;
  const prev = col[i - 1]?.position, next = col[i]?.position;
  const position = prev == null ? (next == null ? t.position : next - 1) : next == null ? prev + 1 : (prev + next) / 2;
  if (position === t.position && status === t.status) return;
  const patch = { position };
  if (status !== t.status) patch.status = status;
  Object.assign(t, patch); render(); patchTask(id, patch);
}
const byOrder = (a, b) => a.position - b.position || a.id - b.id;

function column(s) {
  const q = S.query.toLowerCase();
  const tasks = S.tasks
    .filter((t) => !t.parent_id && t.status === s.id && (!q || t.title.toLowerCase().includes(q) || (t.assignee || "").toLowerCase().includes(q)
      || t.labels.some((l) => l.name.toLowerCase().includes(q))))
    .sort(byOrder);
  const col = h("section", { class: "col" },
    h("header", {}, h("span", { class: "bar", style: `background:${s.color}` }), s.label, h("span", { class: "count" }, tasks.length)),
    h("div", { class: "cards" }, tasks.map(taskCard)),
    can("member") && h("button", { class: "add", onclick: () => createTask(s.id) }, "+ Nueva tarea"));
  col.addEventListener("dragover", (e) => { e.preventDefault(); col.classList.add("over"); });
  col.addEventListener("dragleave", () => col.classList.remove("over"));
  col.addEventListener("drop", (e) => {
    e.preventDefault(); col.classList.remove("over");
    moveTask(+e.dataTransfer.getData("text/plain"), s.id, null);
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
      tr.dow().map((d) => h("div", { class: "cal-dow" }, d)),
      days.map((d) => h("div", { class: "cal-day" + (d.getMonth() !== mo ? " other" : "") + (d.toDateString() === today ? " today" : "") },
        h("span", { class: "num" }, d.getDate()),
        (byDay[d.toDateString()] || []).map((t) => h("div", {
          class: `cal-task p${t.priority}` + (t.status === "done" ? " done" : ""), title: t.title, onclick: () => openTask(t),
        }, t.title))))));
}

function appView() {
  const search = h("input", { placeholder: "Buscar…", value: S.query, oninput: (e) => { S.query = e.target.value; render(true); } });
  return h("div", { class: "shell" + (S.panel && S.team ? " with-panel" : "") + (S.terms && store.get("tvZen") === "1" ? " tv-zen" : "") },
    h("aside", {},
      h("div", { class: "brand" }, "▣ Rust TUI Manager"),
      h("div", {}, h("h2", {}, "Equipos", h("span", {}, S.team && h("button", { class: "ghost", onclick: () => { S.panel = true; store.set("panel", "1"); loadOverview(); setTimeout(() => document.querySelector(".tp-sec:last-child")?.scrollIntoView(), 400); }, title: "Editar o eliminar el equipo actual (ajustes en el panel de la derecha)" }, "⚙"), h("button", { class: "ghost", onclick: joinWithCode, title: "Unirse con un código" }, "↪"), h("button", { class: "ghost", onclick: createTeam, title: "Nuevo equipo" }, "+"))),
        h("div", { class: "nav" }, S.teams.map((t) => h("button", { class: t.id === S.team?.id ? "active" : "", onclick: () => selectTeam(t).catch((e) => toast(e.message, true)) }, t.name)))),
      S.team && h("div", {}, h("h2", {}, "Proyectos", h("button", { class: "ghost", onclick: createProject, title: "Nuevo proyecto" }, "+")),
        h("div", { class: "nav" }, S.projects.length > 0 && can("owner") && h("button", { class: S.terms ? "active" : "", onclick: openTerms, title: "Terminales SSH de los proyectos y estado de las IAs, en paneles" }, "🖥️ Terminales"), S.projects.length > 0 && h("button", { class: S.portfolio ? "active" : "", onclick: openPortfolio, title: "Avance de todos los proyectos del equipo" }, "📊 Portafolio"),
          S.projects.map((p) => h("button", { class: p.id === S.project?.id && !S.portfolio && !S.terms ? "active" : "", onclick: () => selectProject(p).catch((e) => toast(e.message, true)) },
          h("span", { class: "dot", style: `background:${p.color}` }), p.name)))),
      S.team && h("div", {}, h("h2", {}, "🤖 IAs", can("admin") && h("button", { class: "ghost", onclick: createBot, title: "Sumar una IA al equipo (crea su token)" }, "+")),
        h("div", { class: "nav" }, (S.bots || []).map((b) => h("button", { class: b.active ? "" : "revoked", onclick: () => botModal(b), title: b.active ? "Token activo" : "Token revocado" }, "🤖 " + b.username, b.model && h("small", { class: "inl" }, " · " + b.model))),
          !(S.bots || []).length && h("small", { class: "hint" }, can("admin") ? "Sumá asistentes de código con «+»" : "Sin IAs en este equipo"))),
      h("button", { class: "cfg-btn", onclick: openSettings, title: "Configuración (,)" }, "⚙  Configuración"),
      h("button", { class: "cfg-btn", onclick: () => openManual(), title: "Manual con buscador e índice por temas" }, "📖  Manual"),
      h("div", { class: "me" }, h("span", { class: "live" + (S.online ? " on" : ""), title: S.online ? "en vivo" : "desconectado" }),
        S.user?.username, h("span", { class: "spacer" }),
        h("button", { class: "ghost bell" + (S.unread ? " has" : ""), title: "Bandeja de novedades (Ctrl+T n)", onclick: () => loadInbox(false) }, S.unread ? `🔔 ${S.unread}` : "🔔"),
        h("button", { class: "ghost", onclick: helpModal, title: "Atajos y ayuda (?)" }, "?"),
        h("button", { class: "ghost", onclick: logout }, "Salir"),
        h("button", { class: "ghost", onclick: logoutAll, title: "Cierra la sesión en todos los dispositivos" }, "⏻"))),
    h("main", {},
      h("div", { class: "top" },
        h("h1", {}, S.terms ? "🖥️ Terminales" : S.portfolio ? "📊 Portafolio" : S.project ? S.project.name : S.team ? "Sin proyectos" : "Creá un equipo para empezar"),
        h("div", { class: "members" }, S.members.map((m) => avatar(m.username))),
        S.project && !S.portfolio && !S.terms && h("button", { onclick: openUsage, title: "Cuánto gastaron las IAs en este proyecto: tokens, costo, horas y cuotas" }, "💰 Consumo"),
        S.project && !S.portfolio && !S.terms && can("admin") && h("button", { onclick: editProject, title: "Nombre, carpeta de trabajo y repositorio del proyecto" }, "⚙ Proyecto"),
        S.team && h("button", { class: S.panel ? "on" : "", onclick: togglePanel, title: "Panel del equipo: miembros, roles, invitaciones y ajustes (Ctrl+T t)" }, "👥 Equipo"),
        S.team && can("member") && h("button", { onclick: sendTeamMessage, title: "Mensaje al equipo (Ctrl+T s)" }, "✉ Mensaje"),
        !S.portfolio && !S.terms && h("div", { class: "seg" },
          [["board", "Tablero"], ["calendar", "Calendario"], ["gantt", "Gantt"], ["canvas", "Pizarra"], ["activity", "Actividad"]].map(([v, l]) =>
            h("button", { class: S.view === v ? "on" : "", onclick: () => { S.view = v; render(); } }, l))),
        search),
      S.project && !S.portfolio && !S.terms && (S.project.work_dir || S.project.repo) && h("div", { class: "proj-meta" },
        S.project.work_dir && h("span", { title: "Carpeta de trabajo" }, "📁 " + S.project.work_dir),
        S.project.repo && h("span", { title: "Repositorio" }, "🔗 " + S.project.repo)),
      S.terms ? termsView() : S.portfolio ? portfolioView() : S.project ? (S.view === "calendar" ? calendarView() : S.view === "gantt" ? ganttView() : S.view === "canvas" ? canvasView() : S.view === "activity" ? activityView() : h("div", { class: "board" }, STATUS.map(column)))
        : h("div", { class: "empty" }, S.team ? "Creá un proyecto con + en la barra lateral" : "Usá + junto a Equipos")),
    S.panel && S.team && teamPanel());
}

function render(keepFocus) {
  if (S.terms && document.activeElement?.closest?.(".tv-grid")) { refreshTermSide(); return; }
  const root = document.getElementById("app");
  const focused = keepFocus && document.activeElement?.tagName === "INPUT";
  root.replaceChildren(S.token ? appView() : loginView());
  if (S.token) { const b = licenseBanner(); if (b) root.prepend(b); }
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

const duplicateProject = guard(async () => {
  const f = await form("Duplicar proyecto", [{ name: "name", label: "Nombre de la copia", value: `Copia de ${S.project.name}`, max: 100 }], "Duplicar"); if (!f) return;
  const p = await api("POST", `/api/projects/${S.project.id}/duplicate`, { name: f.name });
  if (!S.projects.some((x) => x.id === p.id)) S.projects.push(p);
  await selectProject(p); toast("Proyecto duplicado: las tareas quedaron pendientes y sin responsable");
});

// ---------- actividad ----------
// ---------- portafolio: el avance de todos los proyectos del equipo ----------
const HEALTH = { ok: ["🟢", "Al día"], late: ["🔴", "Con vencidas"], stalled: ["🟡", "Sin movimiento"], done: ["✅", "Terminado"], empty: ["⚪", "Sin tareas"] };
const openPortfolio = guard(async () => { S.terms = false; S.portfolio = true; S.pf = null; render(); await loadPortfolio(); });
async function loadPortfolio(quiet) {
  const team = S.team?.id; if (!team) return;
  try {
    const [d, u] = await Promise.all([api("GET", `/api/teams/${team}/portfolio`), api("GET", `/api/teams/${team}/usage`).catch(() => null)]);
    if (S.portfolio && S.team?.id === team) { S.pf = d; S.pfu = u; render(); }
  }
  catch (e) { if (!quiet) toast(e.message, true); }
}
const ago = (iso) => {
  if (!iso) return "—";
  const m = Math.max(0, Math.round((Date.now() - new Date(iso)) / 60000));
  return m < 1 ? "ahora" : m < 60 ? `hace ${m} min` : m < 1440 ? `hace ${Math.round(m / 60)} h` : `hace ${Math.round(m / 1440)} d`;
};
function portfolioView() {
  if (!S.pf) return h("div", { class: "empty" }, "Cargando…");
  const ps = S.pf.projects;
  const sum = (k) => ps.reduce((n, p) => n + p[k], 0);
  const total = sum("total"), done = sum("done");
  const tile = (n, label, cls) => h("div", { class: "pf-tile " + (cls || "") }, h("b", {}, n), h("span", {}, label));
  const card = (p) => {
    const pct = p.total ? Math.round((p.done / p.total) * 100) : 0;
    const [icon, label] = HEALTH[p.health];
    return h("div", { class: "pf-card " + p.health, onclick: () => selectProject(S.projects.find((x) => x.id === p.id)).catch((e) => toast(e.message, true)), title: "Abrir el proyecto" },
      h("div", { class: "pf-head" }, h("span", { class: "dot", style: `background:${p.color}` }), h("b", { class: "grow" }, p.name), h("span", { class: "pf-health" }, icon + " ", label)),
      h("div", { class: "pf-bar", title: `${p.done} de ${p.total} hechas` },
        h("i", { class: "done", style: `width:${p.total ? (p.done / p.total) * 100 : 0}%` }), h("i", { class: "doing", style: `width:${p.total ? (p.doing / p.total) * 100 : 0}%` })),
      h("div", { class: "pf-nums" }, h("span", {}, `${pct}% · ${p.done}/${p.total}`), h("span", {}, `En curso ${p.doing}`), h("span", {}, `Por hacer ${p.todo}`),
        p.overdue > 0 && h("span", { class: "bad" }, `⏰ ${p.overdue} vencidas`), p.unassigned > 0 && h("span", { class: "warn" }, `${p.unassigned} sin asignar`)),
      h("div", { class: "pf-people" }, p.people.length ? p.people.slice(0, 6).map((x) => h("span", { class: "pf-chip", title: `${x.open} abiertas, ${x.doing} en curso` }, (x.bot ? "🤖 " : "") + x.username + " · " + x.open)) : h("small", { class: "hint" }, "Nadie con tareas abiertas")),
      (() => { const u = S.pfu?.projects.find((x) => x.id === p.id); return u && (u.runs > 0 || u.quota > 0) ? h("div", { class: "pf-usage", title: "Consumo de las IAs este mes" }, h("small", {}, "💰 mes: " + usageLine(u)), quotaBar(u.tokens, u.quota)) : null; })(),
      h("div", { class: "pf-foot" }, h("small", {}, "Última actividad: " + ago(p.last_activity)), p.ai_24h > 0 && h("small", {}, `🤖 ${p.ai_24h} acciones de IAs en 24 h`),
        p.next_due && h("small", {}, "Próximo vencimiento: " + new Date(p.next_due).toLocaleDateString())));
  };
  const risk = ps.filter((p) => p.health === "late" || p.health === "stalled").length;
  return h("div", { class: "portfolio" },
    h("div", { class: "pf-tiles" },
      tile(ps.length, "proyectos"), tile(total ? Math.round((done / total) * 100) + "%" : "—", "avance general"), tile(sum("doing"), "en curso"),
      tile(sum("overdue"), "vencidas", sum("overdue") ? "bad" : ""), tile(risk, "proyectos en riesgo", risk ? "warn" : ""),
      S.pfu && tile(fmtTok(S.pfu.team.tokens) + " · " + fmtUsd(S.pfu.team.cost_usd), "IAs este mes (tokens · costo)", S.pfu.team.quota > 0 && S.pfu.team.tokens >= S.pfu.team.quota ? "bad" : ""),
      h("button", { onclick: () => loadPortfolio() }, "Actualizar")),
    h("div", { class: "pf-grid" }, ps.map(card)),
    h("h3", {}, "🤖 Lo último que hicieron las IAs"),
    S.pf.ai_recent.length === 0 ? h("small", { class: "hint" }, "Todavía no hay actividad de IAs.")
      : h("div", { class: "act-list" }, S.pf.ai_recent.map((a) => h("div", { class: "act-row" }, avatar(a.who),
        h("span", { class: "grow" }, h("b", {}, a.who), " ", a.text, h("small", { class: "inl" }, " · " + a.project)), h("small", {}, ago(a.at))))));
}

function activityView() {
  if (!S.activity || S.activity.project_id !== S.project.id) {
    S.activity = { project_id: S.project.id, items: [] };
    const id = S.project.id;
    api("GET", `/api/projects/${id}/activity`).then((items) => { if (S.project?.id === id) { S.activity = { project_id: id, items }; render(); } }).catch((e) => toast(e.message, true));
  }
  const items = S.activity.items;
  return h("div", { class: "activity" },
    h("div", { class: "cal-head" }, h("strong", {}, "Actividad del proyecto"), h("span", { class: "spacer" }),
      h("button", { onclick: () => { S.activity = null; render(); } }, "Actualizar")),
    items.length === 0 ? h("div", { class: "empty" }, "Todavía no hay actividad.")
      : h("div", { class: "act-list" }, items.map((a) => h("div", { class: "act-row" }, avatar(a.username || "?"),
        h("span", { class: "grow" }, h("b", {}, a.username || "alguien"), " ", a.text), h("small", {}, new Date(a.created_at).toLocaleString())))));
}

// ---------- configuración ----------
const IS_WEB = /^https?:$/.test(location.protocol) && location.hostname !== "tauri.localhost";
const VIEWS = [["board", "Tablero"], ["calendar", "Calendario"], ["gantt", "Gantt"], ["canvas", "Pizarra"], ["activity", "Actividad"]];

// Ayuda de cada fila de Configuración: qué hace (usuario) y qué hay que tener en el servidor (admin del servidor)
const CFG_HELP = {
  user: ["Usuario", "Es tu nombre en la app. Tus compañeros te buscan por él cuando un admin te agrega a un equipo.", null],
  email: ["Email para avisos", "Tocá «Configurar», escribí tu email y poné el código de 6 dígitos que te llega. Solo los emails verificados reciben avisos: asignaciones, menciones, recordatorios y mensajes del equipo.",
    "El servidor necesita una cuenta de correo saliente (Gmail, Outlook, Brevo, SendGrid, Mailgun, SES o uno propio). Creá el archivo ~/.config/tm-server/mail.env (permisos 600) con TM_SMTP_URL, TM_SMTP_USER, TM_SMTP_PASSWORD y TM_SMTP_FROM, agregá EnvironmentFile=-%h/.config/tm-server/mail.env al servicio de tm-server y reinicialo. Sin eso el servidor no manda emails y acá no se puede verificar. Los datos de cada proveedor están en el manual."],
  telegram: ["Telegram", "Tocá «Vincular», abrí el bot que se muestra y mandale /start CÓDIGO. Después recibís las novedades en Telegram y podés usar /tareas, /hecho N (marca la tarea N como hecha) y /stop (desvincula).",
    "Creá un bot con @BotFather en Telegram y arrancá tm-server con TM_TELEGRAM_TOKEN=<token>. No necesita URL pública (usa long polling). Sin el token, esta fila dice «el servidor no tiene el bot configurado»."],
  inbox: ["Bandeja de novedades", "Reúne lo que te pasó: asignaciones, comentarios, menciones (@usuario), recordatorios y mensajes. Se marcan como leídas al cerrarla. También abre con la campana 🔔 o Ctrl+T n.",
    "Los recordatorios de tareas por vencer salen solos; la frecuencia se ajusta con TM_REMINDER_SECS (300 segundos por defecto)."],
  logout: ["Cerrar sesión", "Sale de la app en este dispositivo. Tus datos quedan en el servidor.", null],
  logoutAll: ["Cerrar todas las sesiones", "Úsalo si perdiste un dispositivo o sospechás que alguien más entró: invalida todas tus sesiones abiertas y tenés que volver a iniciar sesión en cada dispositivo.", null],
  role: ["Tu rol", "Lector: solo mira. Miembro: crea, edita y comenta tareas y escribe al equipo. Admin: además agrega miembros, cambia roles y genera invitaciones. Dueño: todo lo anterior y es quien creó el equipo. Si necesitás otro permiso, pedíselo a un admin.",
    "El rol es por equipo, no del servidor: administrar el servidor (variables de entorno, base de datos) es independiente de ser admin o dueño de un equipo."],
  message: ["Mensaje al equipo", "Escribe un aviso que le llega a todos los miembros del equipo por bandeja, y por email y Telegram a quienes lo tengan conectado.", null],
  members: ["Miembros e invitaciones", "Mirá quiénes están en el equipo. Si sos admin o dueño: buscá a alguien ya registrado y agregalo, o generá un código de invitación (y «Copiar invitación» arma el texto con el enlace de instalación para pegar en un chat).", null],
  agents: ["Agentes de IA (tm-agent)", "Para que una IA trabaje sola cuando le asignás una tarea, instalás tm-agent en la máquina donde está la carpeta del proyecto. Tocá «Ver instructivo» para los pasos y el comando listo.",
    "tm-agent no es parte del servidor: es otro programa que se conecta al servidor con el token de la IA. Se instala con cargo install --git https://github.com/polblancoo/Rust_tuiManager rust-tui-manager-agent y se deja como servicio (systemd, launchd o el Programador de tareas)."],
  join: ["Unirme con un código", "Si un admin te pasó un código de invitación, pegalo acá para entrar a su equipo.", null],
  newTeam: ["Crear un equipo", "Creás un equipo nuevo y quedás como Dueño. Después invitás a la gente desde «Miembros, roles e invitaciones».", null],
  project: ["Proyecto", "Un proyecto agrupa las tareas del equipo (tablero, calendario, Gantt y pizarra). Creá uno con «Nuevo proyecto».", null],
  duplicate: ["Usar como plantilla", "Crea un proyecto nuevo copiando las tareas, etiquetas y dependencias del actual, para repetir un proceso sin armarlo de cero.", null],
  export: ["Exportar", "Descarga todas las tareas del proyecto en CSV (para planilla) o JSON (para copia de seguridad o scripts).", null],
  view: ["Vista inicial", "Qué vista se abre primero en este dispositivo: tablero, calendario, Gantt, pizarra o actividad. Se cambia en cualquier momento con las teclas 1 a 5.", null],
  server: ["Servidor", "Es la dirección donde corre tm-server (por ejemplo https://tareas.miempresa.com). En la web propia del servidor no se cambia; en la app de escritorio o la TUI la escribís una vez y volvés a iniciar sesión.",
    "tm-server escucha en TM_ADDR (0.0.0.0:7878 por defecto). Fuera de tu red, ponelo detrás de HTTPS: TM_TLS_CERT y TM_TLS_KEY, o un proxy con TM_TRUSTED_PROXY=1. La web integrada se desactiva con TM_WEB=0."],
  manual: ["Manual", "Guía completa dentro de la app: buscá una palabra o elegí un tema del índice. Los enlaces te llevan directo a cada parte.", null],
  shortcuts: ["Atajos de teclado", "Muestra todos los atajos, con buscador. También se abre con la tecla ?.", null],
};
const cfgHelp = (key) => {
  const [title, user, admin] = CFG_HELP[key];
  document.getElementById("cfg-help")?.remove();
  const close = () => document.getElementById("cfg-help")?.remove();
  document.body.append(h("div", { class: "overlay", id: "cfg-help", onmousedown: (e) => e.target.id === "cfg-help" && close() },
    h("div", { class: "modal" },
      h("h3", {}, title),
      h("h4", {}, "Si sos usuario"), h("p", {}, user),
      admin && [h("h4", {}, "Si administrás el servidor"), h("p", {}, admin),
        key === "email" && h("div", { class: "row" }, h("button", { onclick: () => { close(); openManual({ es: "12-configurar-el-correo-del-servidor", en: "12-set-up-the-servers-email", pt: "12-configurar-o-email-do-servidor" }[tr.lang]); }, title: "Abre el manual en el tema del correo del servidor" }, "📖 Ver ejemplos por proveedor en el manual"))],
      h("div", { class: "row" }, h("span", { class: "spacer" }), h("button", { class: "primary", onclick: close }, "Entendido")))));
};

const googleHelp = () => {
  document.getElementById("gc-help")?.remove();
  const code = (t) => h("code", {}, t);
  document.body.append(h("div", { class: "overlay", id: "gc-help", onmousedown: (e) => e.target.id === "gc-help" && e.target.remove() },
    h("div", { class: "modal" },
      h("h3", {}, "Google Calendar"),
      h("p", {}, "Tus tareas con fecha (asignadas a vos o sin responsable, de tus equipos) aparecen como eventos de día completo en tu calendario principal. Si cambian, se borran o las terminás, el evento se actualiza solo (cada minuto). Es de un solo sentido: lo que cambies en Google no vuelve a la app."),
      h("h3", {}, "Para usarlo"),
      h("p", {}, "Tocá «Conectar», elegí tu cuenta de Google y aceptá el permiso de eventos. Podés desconectar cuando quieras: se quitan los eventos que creó la app."),
      h("h3", {}, "Si dice «el servidor no tiene Google configurado»"),
      h("p", {}, "Lo configura UNA vez quien administra el servidor (no cada usuario):"),
      h("ol", {},
        h("li", {}, "En console.cloud.google.com creá un proyecto y habilitá la «Google Calendar API»."),
        h("li", {}, "En «Google Auth platform» configurá la pantalla de consentimiento (tipo Externo) y agregá como «usuarios de prueba» los Gmail de quienes lo usarán (hasta 100; para abrirlo a todos hay que pasar la verificación de Google)."),
        h("li", {}, "En «Clientes» creá un ID de cliente tipo «Aplicación web» con la redirección ", code("https://TU-SERVIDOR/api/google/callback"), " (con HTTPS; ", code("http://localhost:7878/api/google/callback"), " sirve solo para probar en la misma PC)."),
        h("li", {}, "Arrancá tm-server con ", code("TM_GOOGLE_CLIENT_ID"), ", ", code("TM_GOOGLE_CLIENT_SECRET"), " y ", code("TM_GOOGLE_REDIRECT"), ". El secreto es privado: no lo compartas ni lo subas a git.")),
      h("div", { class: "row" }, h("span", { class: "spacer" }), h("button", { class: "primary", onclick: () => document.getElementById("gc-help")?.remove() }, "Entendido")))));
};
const openSettings = guard(async () => {
  document.getElementById("settings-modal")?.remove();
  S.me = await api("GET", "/api/me");
  const me = S.me, close = () => document.getElementById("settings-modal")?.remove();
  const go = (fn) => () => { close(); fn(); };
  const sec = (title, ...kids) => h("section", { class: "cfg-sec" }, h("h3", {}, title), ...kids);
  const line = (label, ...ctl) => {
    const [text, key] = Array.isArray(label) ? label : [label, null];
    return h("div", { class: "cfg-line" }, h("span", { class: "grow" }, text),
      key && h("button", { class: "ghost help", title: "¿Qué es esto?", onclick: () => (key === "google" ? googleHelp() : cfgHelp(key)) }, "?"), ...ctl);
  };

  const view = h("select", { onchange: (e) => { store.set("view", e.target.value); S.view = e.target.value; render(); } },
    VIEWS.map(([v, l]) => h("option", { value: v, selected: S.view === v }, l)));
  const server = h("input", { value: S.server, disabled: IS_WEB, maxLength: 200 });
  const tgState = !me.telegram_available ? "el servidor no tiene el bot configurado" : me.telegram_linked ? "✓ vinculado" : "sin vincular";
  const gc = await api("GET", "/api/google").catch(() => ({ available: false }));
  const gcState = !gc.available ? "el servidor no tiene Google configurado" : gc.linked ? `✓ conectado (${gc.events} tareas con fecha en tu calendario)` : "sin conectar";
  const emailState = !me.email ? "sin cargar" : me.email_verified ? `✓ ${me.email}` : `${me.email} (sin verificar)`;

  document.body.append(h("div", { class: "overlay", id: "settings-modal", onmousedown: (e) => e.target.id === "settings-modal" && close() },
    h("div", { class: "modal" },
      h("h2", {}, "Configuración"),
      sec("Cuenta",
        line([`Usuario: ${me.username}`, "user"]),
        line([`Email para avisos: ${emailState}`, "email"], h("button", { onclick: go(openProfile) }, "Configurar")),
        line([`Telegram: ${tgState}`, "telegram"], me.telegram_available && (me.telegram_linked
          ? h("button", { class: "danger", onclick: guard(async () => { await api("DELETE", "/api/me/telegram"); close(); openSettings(); }) }, "Desvincular")
          : h("button", { onclick: () => { close(); linkTelegram(); } }, "Vincular"))),
        line([`Google Calendar: ${gcState}`, "google"], gc.available && (gc.linked
          ? [h("button", { onclick: guard(async () => { const r = await api("POST", "/api/google/sync"); toast(`Calendario al día (${r.created} nuevos, ${r.updated} cambiados, ${r.deleted} quitados)`); close(); openSettings(); }) }, "Sincronizar"),
            h("button", { class: "danger", onclick: guard(async () => { if (!confirm("¿Desconectar y quitar de tu calendario los eventos creados por la app?")) return; await api("DELETE", "/api/google"); close(); openSettings(); }) }, "Desconectar")]
          : h("button", { onclick: guard(async () => { const l = await api("POST", "/api/google/link"); window.open(l.url, "_blank", "noopener"); toast("Autorizá en la pestaña de Google y después volvé a abrir Configuración"); }) }, "Conectar"))),
        line(["Bandeja de novedades", "inbox"], h("button", { onclick: go(() => loadInbox(false)) }, S.unread ? `Abrir (${S.unread})` : "Abrir")),
        line(["Cerrar sesión en este dispositivo", "logout"], h("button", { onclick: () => { close(); logout(); } }, "Salir")),
        line(["Cerrar sesión en todos los dispositivos", "logoutAll"], h("button", { class: "danger", onclick: () => { close(); logoutAll(); } }, "Cerrar todas"))),
      S.team && sec(`Equipo «${S.team.name}»`,
        line([`Tu rol: ${ROLES.find(([r]) => r === myRole())?.[1]}`, "role"]),
        can("member") && line(["Enviar un mensaje al equipo", "message"], h("button", { onclick: go(sendTeamMessage) }, "Escribir")),
        line(["Miembros, roles e invitaciones", "members"], h("button", { onclick: go(openTeam) }, "Administrar")),
        can("admin") && line(["Agentes de IA: que trabajen solos (tm-agent)", "agents"], h("button", { onclick: go(agentsGuide) }, "Ver instructivo")),
        line(["Unirme a otro equipo con un código", "join"], h("button", { onclick: go(joinWithCode) }, "Unirme")),
        line(["Crear un equipo nuevo", "newTeam"], h("button", { onclick: go(createTeam) }, "Crear"))),
      S.team && sec("Proyecto",
        line([S.project ? `Proyecto actual: ${S.project.name}` : "Sin proyectos", "project"], h("button", { onclick: go(createProject) }, "Nuevo proyecto")),
        S.project && can("member") && line(["Usar como plantilla (copia tareas, etiquetas y dependencias)", "duplicate"], h("button", { onclick: go(duplicateProject) }, "Duplicar")),
        S.project && line(["Exportar las tareas", "export"], h("button", { onclick: guard(() => saveFile(`/api/projects/${S.project.id}/export?format=csv`, `${S.project.name}.csv`)) }, "CSV"),
          h("button", { onclick: guard(() => saveFile(`/api/projects/${S.project.id}/export?format=json`, `${S.project.name}.json`)) }, "JSON"))),
      sec("Licencia",
        line("Uso gratis (AGPL-3.0) o licencia comercial para empresas", h("button", { onclick: go(openLicense) }, "🏢 Licencia")),),
      sec("Preferencias",
        line("Idioma / Language", h("select", { onchange: (e) => tr.set(e.target.value) },
          Object.entries(tr.langs).map(([k, n]) => h("option", { value: k, selected: tr.lang === k }, n)))),
        line(["Vista inicial", "view"], view),
        line(["Servidor", "server"], server),
        IS_WEB ? h("small", {}, "Estás usando la web del propio servidor: la dirección no se cambia desde acá.")
          : h("div", { class: "row" }, h("button", { onclick: guard(async () => {
            const url = server.value.trim().replace(/\/$/, ""); if (!/^https?:\/\//.test(url)) throw new Error("la dirección debe empezar con http:// o https://");
            S.server = url; store.set("server", url); toast("Servidor guardado: volvé a iniciar sesión"); close(); logout();
          }) }, "Guardar servidor y reiniciar sesión"))),
      sec("Ayuda", line(["Manual con buscador e índice por temas", "manual"], h("button", { onclick: go(() => openManual()) }, "Abrir el manual")),
        line(["Atajos de teclado (también con ?)", "shortcuts"], h("button", { onclick: go(helpModal) }, "Ver atajos"))),
      h("div", { class: "row" }, h("button", { class: "primary", onclick: close }, "Listo")))));
});

// ---------- ayuda con buscador y atajos de teclado ----------
const SHORTCUTS = [
  ["?", "Abrir esta ayuda"], ["/", "Ir al buscador"], ["n", "Nueva tarea (en Por hacer)"],
  ["1", "Vista Tablero"], ["2", "Vista Calendario"], ["3", "Vista Gantt"], ["4", "Vista Pizarra"], ["5", "Vista Actividad"], ["@usuario", "En un comentario: menciona y avisa a esa persona"],
  ["Esc", "Cerrar ventana"], [",", "Configuración"], ["Ctrl+T c", "Configuración"], ["Ctrl+T s", "Enviar un mensaje al equipo"], ["Ctrl+T g", "Vincular Telegram"],
  ["Ctrl+T m", "Equipo: miembros y agregar con búsqueda"], ["Ctrl+T i", "Equipo: generar código de invitación"],
  ["Ctrl+T j", "Unirse a un equipo con código"], ["Ctrl+T t", "Crear equipo"], ["Ctrl+T p", "Crear proyecto"],
  ["Ctrl+T e", "Mi email para recibir avisos"], ["Ctrl+T n", "Bandeja de novedades y mis tareas"],
  ["Arrastrar tarjeta", "Cambiar de estado (Tablero)"], ["Arrastrar barra", "Mover fechas (Gantt)"],
  ["Doble clic en la pizarra", "Nueva nota en ese lugar"], ["↗ en un elemento", "Conectar dos elementos (Pizarra)"],
];
// ---------- manual dentro de la app: índice por temas, buscador y enlaces ----------
const mdSlug = (t) => t.toLowerCase().replace(/[`*_]/g, "").replace(/[^\p{L}\p{N}\s-]/gu, "").trim().replace(/\s+/g, "-");
// nodos de texto crudos: el manual ya viene en el idioma elegido, no pasa por tr()
const mdEl = (tag, cls, ...kids) => { const e = document.createElement(tag); if (cls) e.className = cls; for (const k of kids.flat()) if (k != null) e.append(k); return e; };
const mdText = (t) => document.createTextNode(t);
function mdInline(src, jump) {
  const out = [], re = /!\[([^\]]*)\]\(([^)\s]+)\)|\[([^\]]+)\]\(([^)\s]+)\)|`([^`]+)`|\*\*([^*]+)\*\*|\*([^*\s][^*]*)\*/g;
  let last = 0, m;
  while ((m = re.exec(src))) {
    if (m.index > last) out.push(mdText(src.slice(last, m.index)));
    if (m[2]) {
      const g = /\/docs\/screenshots\/([\w.-]+)$/.exec(m[2]);
      if (g) { const i = mdEl("img"); i.src = `/manual/img/${g[1]}`; i.alt = m[1]; i.loading = "lazy"; out.push(i); }
    } else if (m[4]) {
      const a = mdEl("a", "", ...mdInline(m[3], jump));
      if (m[4].startsWith("#")) { a.href = m[4]; a.onclick = (e) => { e.preventDefault(); jump(m[4].slice(1)); }; }
      else if (/^https?:\/\//.test(m[4])) { a.href = m[4]; a.target = "_blank"; a.rel = "noopener noreferrer"; }
      else out.push(...mdInline(m[3], jump));
      if (a.href) out.push(a);
    } else if (m[5]) out.push(mdEl("code", "", mdText(m[5])));
    else if (m[6]) out.push(mdEl("b", "", ...mdInline(m[6], jump)));
    else out.push(mdEl("i", "", ...mdInline(m[7], jump)));
    last = re.lastIndex;
  }
  if (last < src.length) out.push(mdText(src.slice(last)));
  return out;
}
// divide el markdown en secciones (una por encabezado) y arma el DOM de cada una
function mdParse(md, jump) {
  const lines = md.split("\n"), secs = [], seen = {};
  let cur = { level: 0, title: "", id: "", raw: [], node: mdEl("section") };
  const push = () => { cur.text = cur.raw.join("\n"); secs.push(cur); };
  for (let i = 0; i < lines.length; i++) {
    const l = lines[i];
    const hd = /^(#{1,4})\s+(.*)$/.exec(l);
    if (hd) {
      push();
      let id = mdSlug(hd[2]); if (seen[id]) id += "-" + (++seen[id]); else seen[id] = 1;
      const n = mdEl("section"); n.id = "man-" + id;
      n.append(mdEl("h" + (hd[1].length + 1), "", ...mdInline(hd[2], jump)));
      cur = { level: hd[1].length, title: hd[2].replace(/[`*]/g, ""), id, raw: [hd[2]], node: n };
      continue;
    }
    cur.raw.push(l);
    if (/^```/.test(l)) {
      const code = []; while (++i < lines.length && !/^```/.test(lines[i])) code.push(lines[i]);
      cur.raw.push(...code); cur.node.append(mdEl("pre", "", mdEl("code", "", mdText(code.join("\n"))))); continue;
    }
    if (/^\s*$/.test(l) || /^<\/?p[ >]/.test(l)) continue;
    if (/^---+\s*$/.test(l)) { cur.node.append(mdEl("hr")); continue; }
    if (/^>/.test(l)) {
      const q = [l.replace(/^>\s?/, "")]; while (i + 1 < lines.length && /^>/.test(lines[i + 1])) q.push(lines[++i].replace(/^>\s?/, ""));
      cur.raw.push(...q); cur.node.append(mdEl("blockquote", "", ...mdInline(q.join(" "), jump))); continue;
    }
    if (/^\|/.test(l) && /^\|[\s:|-]+\|?\s*$/.test(lines[i + 1] || "")) {
      const cells = (r) => r.replace(/^\||\|\s*$/g, "").split("|").map((c) => c.trim());
      const head = cells(l); i++; const rows = [];
      while (i + 1 < lines.length && /^\|/.test(lines[i + 1])) rows.push(cells(lines[++i]));
      cur.raw.push(...head, ...rows.flat());
      cur.node.append(mdEl("div", "md-table", mdEl("table", "", mdEl("thead", "", mdEl("tr", "", head.map((c) => mdEl("th", "", ...mdInline(c, jump))))),
        mdEl("tbody", "", rows.map((r) => mdEl("tr", "", r.map((c) => mdEl("td", "", ...mdInline(c, jump)))))))));
      continue;
    }
    const li = /^(\s*)([-*]|\d+\.)\s+(.*)$/.exec(l);
    if (li) {
      const ordered = /\d/.test(li[2]), list = mdEl(ordered ? "ol" : "ul");
      let m = li;
      for (;;) {
        const item = mdEl("li", "", ...mdInline(m[3], jump)); cur.raw.push(m[3]);
        while (i + 1 < lines.length && /^\s{2,}\S/.test(lines[i + 1]) && !/^\s*([-*]|\d+\.)\s/.test(lines[i + 1])) { const t = lines[++i].trim(); cur.raw.push(t); item.append(mdText(" "), ...mdInline(t, jump)); }
        list.append(item);
        const nx = /^(\s*)([-*]|\d+\.)\s+(.*)$/.exec(lines[i + 1] || "");
        if (!nx || /\d/.test(nx[2]) !== ordered) break;
        m = nx; i++;
      }
      cur.node.append(list); continue;
    }
    const para = [l]; while (i + 1 < lines.length && lines[i + 1].trim() && !/^(#{1,4}\s|```|>|\||---|\s*([-*]|\d+\.)\s|<)/.test(lines[i + 1])) para.push(lines[++i]);
    cur.raw.push(...para.slice(1)); cur.node.append(mdEl("p", "", ...mdInline(para.join(" "), jump)));
  }
  push();
  return secs.filter((x) => x.node.childNodes.length);
}

const openManual = guard(async (topic) => {
  document.getElementById("manual")?.remove();
  const lang = ["es", "en", "pt"].includes(tr.lang) ? tr.lang : "en";
  const res = await fetch(`/manual/${lang}.md`); if (!res.ok) throw new Error("No pude cargar el manual");
  const content = mdEl("div", "man-body"), toc = mdEl("nav", "man-toc"), count = mdEl("small", "man-count");
  const jump = (id) => { search.value = ""; draw(); document.getElementById("man-" + id)?.scrollIntoView({ block: "start" }); };
  const secs = mdParse(await res.text(), jump);
  const search = h("input", { type: "search", placeholder: "Buscar en el manual…", "aria-label": "Buscar en el manual", autocomplete: "off" });
  const draw = () => {
    const q = search.value.trim().toLowerCase(), words = q.split(/\s+/).filter(Boolean);
    const hit = words.length ? secs.filter((x) => words.every((w) => (x.title + " " + x.text).toLowerCase().includes(w))) : secs;
    content.replaceChildren(...(hit.length ? hit.map((x) => x.node) : [mdEl("p", "", mdText(tr("Sin resultados")))]));
    count.textContent = words.length ? `${hit.length} ${tr("resultados")}` : "";
    toc.replaceChildren(...secs.filter((x) => x.level && x.level <= 3).filter((x) => !words.length || hit.includes(x)).map((x) => {
      const a = mdEl("a", "t" + x.level, mdText(x.title)); a.href = "#" + x.id;
      a.onclick = (e) => { e.preventDefault(); document.getElementById("man-" + x.id)?.scrollIntoView({ block: "start" }); }; return a;
    }));
  };
  search.addEventListener("input", draw);
  const close = () => document.getElementById("manual")?.remove();
  const box = h("div", { class: "overlay", id: "manual", onclick: (e) => e.target === e.currentTarget && close() },
    h("div", { class: "modal manual", onkeydown: (e) => e.key === "Escape" && (e.stopPropagation(), close()) },
      h("div", { class: "man-head" }, h("h3", {}, "📖 Manual"), search, count, h("span", { class: "spacer" }), h("button", { onclick: close, title: "Cerrar el manual (Esc)" }, "Cerrar")),
      h("div", { class: "man-main" }, toc, content)));
  document.body.append(box); draw(); search.focus();
  if (typeof topic === "string" && topic) document.getElementById("man-" + topic)?.scrollIntoView();
});

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
  if (e.ctrlKey && e.key.toLowerCase() === "t") { e.preventDefault(); S.prefix = true; toast("Ctrl+T ▸ m miembros · i invitar · j unirse · t panel del equipo · p proyecto · e email · n bandeja · s mensaje · g Telegram · c config"); return; }
  if (S.prefix) {
    S.prefix = false; e.preventDefault();
    const act = { m: () => S.team && openTeam(), i: () => S.team && openTeam(), j: joinWithCode, t: togglePanel, p: () => S.team && createProject(), e: openProfile, n: () => loadInbox(false), c: openSettings, s: () => S.team && can("member") && sendTeamMessage(), g: linkTelegram }[e.key.toLowerCase()];
    act ? act() : toast("Atajo de equipo desconocido", true);
    return;
  }
  if (typing || e.ctrlKey || e.metaKey || e.altKey) return;
  if (e.key === "?") { e.preventDefault(); helpModal(); }
  else if (e.key === ",") { e.preventDefault(); openSettings(); }
  else if (e.key === "/") { e.preventDefault(); document.querySelector(".top input")?.focus(); }
  else if (e.key === "n" && S.project && can("member")) { e.preventDefault(); createTask("todo"); }
  else if (["1", "2", "3", "4", "5"].includes(e.key)) { S.view = ["board", "calendar", "gantt", "canvas", "activity"][+e.key - 1]; render(); }
});

if (IS_WEB && "serviceWorker" in navigator) navigator.serviceWorker.register("/sw.js").catch(() => {});
render();
if (S.token) boot(true).catch(() => { S.token = null; render(); });


// ---------- terminal SSH integrada (solo owners): el servidor abre SSH con clave a la máquina del proyecto ----------
const loadScript = (src) => new Promise((ok, no) => { const el = document.createElement("script"); el.src = src; el.onload = ok; el.onerror = () => no(new Error("no pude cargar " + src)); document.head.append(el); });

async function openTerminal(task) {
  if (!window.Terminal) { await loadScript("xterm.js"); await loadScript("xterm-fit.js"); }
  const pid = task.project_id;
  const cfg = await api("GET", `/api/projects/${pid}/ssh`);
  const agents = await api("GET", `/api/projects/${pid}/term/agents`).catch(() => []);
  const mode = h("select", { title: "Cómo se llega a la máquina del proyecto", onchange: () => applyMode() },
    h("option", { value: "agent", selected: !!cfg.agent_id || (!cfg.host && agents.some((a) => a.online)) }, "🤖 Por el agente instalado en la máquina (recomendado)"),
    h("option", { value: "ssh", selected: !cfg.agent_id && (!!cfg.host || !agents.some((a) => a.online)) }, "🔑 Por SSH con clave"));
  const agentSel = h("select", {}, agents.length ? agents.map((a) => h("option", { value: a.id, selected: a.id === cfg.agent_id }, `🤖 ${a.name}${a.model ? " · " + a.model : ""} — ${a.online ? "🟢 conectado" : "⚪ sin conectar"}`)) : h("option", { value: "" }, "(no hay IAs en este equipo)"));
  const agentBox = h("div", {}, h("label", {}, "Agente de la máquina", agentSel),
    h("small", { class: "hint" }, "En la máquina donde está el proyecto, el agente se arranca con TM_AGENT_TERMINAL=1 (junto a TM_TOKEN y TM_URL de esa IA). Sale solo hacia el servidor: no hay que abrir puertos, ni copiar claves, ni cargar IPs. Funciona también detrás de NAT y en Windows."));
  const f = {
    host: h("input", { value: cfg.host, placeholder: "máquina (IP o nombre)" }),
    port: h("input", { type: "number", value: cfg.port, placeholder: "22" }),
    user: h("input", { value: cfg.user, placeholder: "usuario" }),
    key: h("input", { value: cfg.key_path, class: "wide", placeholder: "clave privada en el servidor" }),
  };
  const pub = h("div", { class: "term-key", hidden: !cfg.public_key }, cfg.public_key || "");
  const info = h("small", { class: "hint" }, cfg.host_fp ? "Huella de la máquina: " + cfg.host_fp : "Todavía no se conectó: la primera vez se guarda la huella de la máquina.");
  const box = h("div", { class: "term-box" });
  let sock = null, term = null, fit = null, ro = null;
  const stop = () => { try { sock?.close(); } catch {} ro?.disconnect(); term?.dispose(); sock = term = null; };
  const saveCfg = async () => {
    if (mode.value === "agent") { if (!agentSel.value) throw new Error("Elegí un agente (o creá una IA en el equipo)"); return api("PUT", `/api/projects/${pid}/ssh`, { agent_id: +agentSel.value }); }
    return api("PUT", `/api/projects/${pid}/ssh`, { host: f.host.value, port: +f.port.value || 22, user: f.user.value, key_path: f.key.value });
  };
  const connect = guard(async () => {
    stop(); box.replaceChildren();
    await saveCfg();
    if (mode.value === "ssh") { const pk = (await api("GET", `/api/projects/${pid}/ssh`)).public_key; if (pk) { pub.hidden = false; pub.textContent = pk; } }
    term = new Terminal({ cursorBlink: true, fontSize: 14, fontFamily: "monospace", theme: { background: "#0b0d14" } });
    fit = new FitAddon.FitAddon(); term.loadAddon(fit); term.open(box); fit.fit();
    const ws = new WebSocket(S.server.replace(/^http/, "ws") + `/api/projects/${pid}/term${task.id ? "?task=" + task.id : ""}`);
    sock = ws; ws.binaryType = "arraybuffer";
    ws.onopen = () => ws.send(JSON.stringify({ token: S.token, cols: term.cols, rows: term.rows }));
    ws.onmessage = (e) => {
      if (typeof e.data === "string") {
        const m = JSON.parse(e.data);
        if (m.error) term.write("\r\n\x1b[31m" + m.error + "\x1b[0m\r\n");
        if (m.info) term.write("\x1b[33m" + m.info + "\x1b[0m\r\n");
      } else term.write(new Uint8Array(e.data));
    };
    ws.onclose = () => term && term.write("\r\n\x1b[90m[sesión cerrada]\x1b[0m\r\n");
    const enc = new TextEncoder();
    term.onData((d) => ws.readyState === 1 && ws.send(enc.encode(d)));
    term.onResize(({ cols, rows }) => ws.readyState === 1 && ws.send(JSON.stringify({ resize: [cols, rows] })));
    ro = new ResizeObserver(() => { try { fit.fit(); } catch {} }); ro.observe(box);
    term.focus();
    cfg.host_fp = (await api("GET", `/api/projects/${pid}/ssh`)).host_fp; info.textContent = "Huella de la máquina: " + cfg.host_fp;
  });
  const keygen = guard(async () => {
    await saveCfg();
    const r = await api("POST", `/api/projects/${pid}/ssh/keygen`);
    pub.hidden = false; pub.textContent = r.public_key;
    toast("Clave lista: pegá la pública en ~/.ssh/authorized_keys de la máquina");
  });
  const pw = h("input", { type: "password", autocomplete: "off", placeholder: "contraseña de esa máquina (no se guarda)" });
  const install = guard(async () => {
    if (!pw.value) { pw.focus(); return toast("Escribí la contraseña de esa máquina para instalar la clave", true); }
    await saveCfg();
    const r = await api("POST", `/api/projects/${pid}/ssh/install-key`, { password: pw.value });
    pw.value = "";
    info.textContent = "Huella de la máquina: " + r.host_fp;
    const pk = (await api("GET", `/api/projects/${pid}/ssh`)).public_key; if (pk) { pub.hidden = false; pub.textContent = pk; }
    toast("✅ Clave instalada en la máquina. Ya podés tocar «Conectar»");
  });
  const copyAll = guard(async () => {
    await saveCfg();
    const r = await api("POST", `/api/projects/${pid}/ssh/copy-all`);
    toast(r.copied ? `Misma máquina cargada en ${r.copied} proyecto(s) más` : "No hay otros proyectos en este equipo");
  });
  const forget = guard(async () => { await api("POST", `/api/projects/${pid}/ssh/forget-host`); info.textContent = "Huella olvidada: la próxima conexión guarda la nueva."; });
  const sshBox = h("div", {},
    h("div", { class: "term-cfg" }, f.host, f.port, f.user, f.key),
    h("div", { class: "row" }, pw, h("button", { title: "Entra una sola vez con tu contraseña, deja la clave pública en ~/.ssh/authorized_keys de esa máquina y olvida la contraseña", onclick: install }, "🔐 Instalar clave")),
    h("div", { class: "row" },
      h("button", { title: "Crea una clave ed25519 en el servidor y muestra la pública", onclick: keygen }, "🔑 Generar clave"),
      h("button", { class: "ghost", title: "Si la máquina se reinstaló: vuelve a guardar su huella en la próxima conexión", onclick: forget }, "Olvidar huella")),
    pub, info);
  const applyMode = () => { agentBox.hidden = mode.value !== "agent"; sshBox.hidden = mode.value !== "ssh"; };
  applyMode();
  const close = () => { stop(); closeModal(); };
  const modal = h("div", { class: "overlay", id: "modal", onmousedown: (e) => e.target.id === "modal" && close() },
    h("div", { class: "modal term-modal" + (store.get("termMax") === "1" ? " max" : "") },
      h("h3", { class: "task-head" }, "💻 Terminal · " + (task.id ? "tarea #" + task.id : "configuración de la conexión"), h("span", { class: "spacer" }),
        h("button", { class: "ghost max-btn", title: "Agranda la terminal para trabajar más cómodo", onclick: (e) => { const m = e.currentTarget.closest(".modal"); const on = m.classList.toggle("max"); store.set("termMax", on ? "1" : "0"); e.currentTarget.textContent = on ? "⤡ Restaurar" : "⤢ Maximizar"; setTimeout(() => { try { fit?.fit(); } catch {} }, 80); } }, store.get("termMax") === "1" ? "⤡ Restaurar" : "⤢ Maximizar"),
        h("button", { class: "ghost", onclick: close }, "✕")),
      h("label", {}, "Conexión", mode),
      agentBox,
      sshBox,
      h("div", { class: "row" },
        h("button", { class: "primary", onclick: connect }, "▶ Conectar"),
        h("button", { title: "Usa esta misma conexión en todos los demás proyectos del equipo", onclick: copyAll }, "📋 Usar en todos los proyectos")),
      box,
      h("small", { class: "hint" }, "Cada sesión queda anotada en el historial de la tarea. Se cierra sola a los 30 min sin uso.")));
  closeModal(); S.open = null;
  document.body.append(modal);
}


// ---------- vista Terminales: varias terminales SSH en paneles + estado de las IAs (solo owners) ----------
const PANES = [];
const ensureXterm = async () => { if (!window.Terminal) { await loadScript("xterm.js"); await loadScript("xterm-fit.js"); } };
const tvSide = h("div", { class: "tv-side" });

const openTerms = guard(async () => {
  S.portfolio = false; S.terms = true;
  S.tvProject = S.tvProject && S.projects.some((p) => p.id === S.tvProject) ? S.tvProject : (S.project?.id || S.projects[0]?.id);
  await ensureXterm(); await loadTermTasks(); render(); startPanes();
  clearInterval(openTerms.t); openTerms.t = setInterval(() => { if (!S.terms) return clearInterval(openTerms.t); loadTermTasks().catch(() => {}); }, 8000);
});
async function loadTermTasks() {
  if (!S.tvProject) return;
  S.tvTasks = await api("GET", `/api/projects/${S.tvProject}/tasks`);
  refreshTermSide();
}
// estado de cada IA: lo que hace ahora, si te necesita o si terminó
function aiState(t) {
  if (/^🙋 Pedido de #/.test(t.title)) return t.status === "done" ? null : ["🙋", "te necesita", "need"];
  if (t.status === "doing") return ["⚙️", "trabajando", "work"];
  if (t.status === "todo") return ["⏳", "en cola", "wait"];
  if (t.status === "done") return ["✅", "terminó", "done"];
  return null;
}
function refreshTermSide() {
  const bots = new Map((S.members || []).filter((m) => botInfo(m.username)).map((m) => [m.id, m]));
  const rows = (S.tvTasks || []).map((t) => ({ t, st: aiState(t), who: bots.get(t.assignee_id) || (/^🙋/.test(t.title) ? null : null) }))
    .filter((r) => r.st && (r.who || r.st[2] === "need"))
    .sort((a, b) => ["need", "work", "wait", "done"].indexOf(a.st[2]) - ["need", "work", "wait", "done"].indexOf(b.st[2]))
    .filter((r, i, all) => r.st[2] !== "done" || all.filter((x) => x.st[2] === "done").indexOf(r) < 5);
  tvSide.replaceChildren(h("h3", {}, "🤖 IAs y su estado"), ...(
    rows.length ? rows.map(({ t, st, who }) => h("button", { class: "tv-ai " + st[2], title: "Abrir una terminal ligada a esta tarea", onclick: () => addPane(t.project_id, t.id, `${who ? "🤖 " + who.username + (botInfo(who.username)?.model ? " · " + botInfo(who.username).model : "") : "🙋"} — #${t.id}`) },
      h("span", { class: "tv-badge" }, st[0] + " " + st[1]), h("b", {}, who ? who.username : "pedido"), h("small", {}, `#${t.id} ${t.title}`)))
      : [h("small", { class: "hint" }, "Ninguna IA con tareas en este proyecto.")]));
}
function termsView() {
  const proj = h("select", { onchange: async (e) => { S.tvProject = +e.target.value; S.tvTasks = []; refreshTermSide(); await loadTermTasks().catch(() => {}); } },
    S.projects.map((p) => h("option", { value: p.id, selected: p.id === S.tvProject }, p.name)));
  const cols = h("select", { title: "Columnas del panel", onchange: (e) => { store.set("tvCols", e.target.value); render(); startPanes(); } },
    [["0", "Auto"], ["1", "1 columna"], ["2", "2 columnas"], ["3", "3 columnas"]].map(([v, l]) => h("option", { value: v, selected: (store.get("tvCols") || "0") === v }, l)));
  const n = PANES.length, c = +(store.get("tvCols") || 0) || (n <= 1 ? 1 : n <= 4 ? 2 : 3);
  refreshTermSide();
  return h("div", { class: "termsv" },
    h("div", { class: "tv-bar" }, h("label", {}, "Proyecto ", proj), h("label", {}, "Vista ", cols),
      h("button", { class: "primary", title: "Abre una terminal SSH nueva en la carpeta de este proyecto, sin crear tarea", onclick: () => addPane(S.tvProject, null, "💻 " + (S.projects.find((p) => p.id === S.tvProject)?.name || "terminal")) }, "+ Terminal"),
      h("button", { class: store.get("tvZen") === "1" ? "on" : "", title: "Oculta el menú lateral y el panel del equipo para tener toda la pantalla para las terminales", onclick: () => { store.set("tvZen", store.get("tvZen") === "1" ? "0" : "1"); render(); startPanes(); } }, store.get("tvZen") === "1" ? "⛶ Mostrar barras" : "⛶ Ocultar barras"),
      h("button", { class: store.get("tvSide") === "0" ? "on" : "", title: "Muestra u oculta la lista de IAs de la izquierda", onclick: () => { store.set("tvSide", store.get("tvSide") === "0" ? "1" : "0"); render(); startPanes(); } }, store.get("tvSide") === "0" ? "◧ Mostrar IAs" : "◧ Ocultar IAs"),
      h("button", { title: "Cómo llega la terminal a la máquina de este proyecto: por el agente (recomendado) o por SSH", onclick: guard(() => openTerminal({ id: null, project_id: S.tvProject })) }, "⚙ Conexión"),
      h("small", { class: "hint" }, "Hasta 8 a la vez. Lo que hagas queda anotado en .tm/ de la carpeta del proyecto para que la IA retome.")),
    h("div", { class: "tv-body" }, store.get("tvSide") !== "0" && tvSide,
      h("div", { class: "tv-grid", style: `--cols:${c}` }, PANES.length ? PANES.map((p) => p.el) : h("div", { class: "empty" }, "Tocá «+ Terminal» o elegí una IA de la izquierda."))));
}
async function addPane(pid, tid, title) {
  if (PANES.length >= 8) return toast("Máximo 8 terminales abiertas", true);
  try {
    const c0 = await api("GET", `/api/projects/${pid}/ssh`); if (!c0.host && !c0.agent_id) { toast("Este proyecto todavía no tiene conexión a su máquina: configurala primero"); return openTerminal({ id: null, project_id: pid }); }
  } catch (e) { return toast(e.message, true); }
  const box = h("div", { class: "term-box" }), st = h("span", { class: "tv-st" }, "⏳");
  const pane = { pid, tid, box, st, started: false };
  pane.el = h("div", { class: "tv-pane" }, h("div", { class: "tv-head" }, st, h("b", {}, title), h("span", { class: "spacer" }),
    h("button", { class: "ghost", title: "Agranda esta terminal a toda la pantalla (el mismo botón para volver)", onclick: (e) => { const on = pane.el.classList.toggle("full"); e.currentTarget.textContent = on ? "⤡" : "⤢"; setTimeout(() => { try { pane.fit?.fit(); pane.term?.focus(); } catch {} }, 80); } }, "⤢"),
    h("button", { class: "ghost", title: "Cerrar esta terminal (guarda la nota .md)", onclick: () => closePane(pane) }, "✕")), box);
  PANES.push(pane); render(); startPanes();
}
function closePane(p) { try { p.ws?.close(); } catch {} p.ro?.disconnect(); p.term?.dispose(); PANES.splice(PANES.indexOf(p), 1); p.el.remove(); render(); startPanes(); }
function startPanes() {
  requestAnimationFrame(() => PANES.forEach((p) => {
    if (p.started || !p.el.isConnected) { try { p.fit?.fit(); } catch {} return; }
    p.started = true;
    const term = new Terminal({ cursorBlink: true, fontSize: 13, fontFamily: "monospace", theme: { background: "#0b0d14" } });
    const fit = new FitAddon.FitAddon(); term.loadAddon(fit); term.open(p.box); fit.fit();
    Object.assign(p, { term, fit });
    const ws = new WebSocket(S.server.replace(/^http/, "ws") + `/api/projects/${p.pid}/term${p.tid ? "?task=" + p.tid : ""}`); p.ws = ws; ws.binaryType = "arraybuffer";
    ws.onopen = () => { ws.send(JSON.stringify({ token: S.token, cols: term.cols, rows: term.rows })); p.st.textContent = "🟢"; };
    ws.onmessage = (e) => {
      if (typeof e.data !== "string") return term.write(new Uint8Array(e.data));
      const m = JSON.parse(e.data);
      if (m.error) term.write("\r\n\x1b[31m" + m.error + "\x1b[0m\r\n");
      if (m.info) term.write("\x1b[33m" + m.info + "\x1b[0m\r\n");
    };
    ws.onclose = () => { p.st.textContent = "⚫"; term.write("\r\n\x1b[90m[sesión cerrada]\x1b[0m\r\n"); };
    const enc = new TextEncoder();
    term.onData((d) => ws.readyState === 1 && ws.send(enc.encode(d)));
    term.onResize(({ cols, rows }) => ws.readyState === 1 && ws.send(JSON.stringify({ resize: [cols, rows] })));
    p.ro = new ResizeObserver(() => { try { fit.fit(); } catch {} }); p.ro.observe(p.box);
  }));
}


// ---------- cuadro para permitir un comando a una IA (con explicación de qué hace) ----------
const CMD_HELP = [
  ["cargo build", "Compila el proyecto Rust. Crea archivos en target/; no toca tu código."],
  ["cargo check", "Revisa que el código compile sin generar el programa. Es rápido y seguro."],
  ["cargo test", "Corre las pruebas del proyecto."],
  ["cargo run", "Compila y EJECUTA el programa del proyecto."],
  ["cargo fmt", "Reformatea el código (cambia archivos fuente)."],
  ["cargo clippy", "Revisa el código buscando errores comunes. No cambia nada."],
  ["cargo install", "Instala un programa en tu máquina."],
  ["cargo publish", "⚠️ PUBLICA el crate en crates.io (público e irreversible)."],
  ["git status", "Muestra qué archivos cambiaron. Solo lectura."],
  ["git diff", "Muestra los cambios línea por línea. Solo lectura."],
  ["git log", "Muestra el historial de commits. Solo lectura."],
  ["git add", "Prepara archivos para el próximo commit."],
  ["git commit", "Guarda un commit en el repositorio local."],
  ["git push", "⚠️ Sube commits al repositorio remoto (visible para otros)."],
  ["git pull", "Trae y mezcla cambios del remoto en tu carpeta."],
  ["git checkout", "Cambia de rama o descarta cambios de archivos."],
  ["git reset", "⚠️ Mueve o descarta commits y cambios."],
  ["git stash", "Guarda cambios sin commitear en un cajón temporal."],
  ["npm install", "Descarga las dependencias del proyecto (crea node_modules/)."],
  ["npm run", "Ejecuta un script definido en package.json."],
  ["npm test", "Corre las pruebas del proyecto."],
  ["node --check", "Revisa la sintaxis de un archivo JavaScript sin ejecutarlo."],
  ["node", "Ejecuta código JavaScript."],
  ["python", "Ejecuta código Python."],
  ["pip install", "Instala paquetes de Python."],
  ["docker", "Maneja contenedores: puede crear, borrar o ejecutar cosas."],
  ["systemctl", "⚠️ Inicia, detiene o reinicia servicios del sistema."],
  ["make", "Ejecuta las tareas del Makefile del proyecto."],
  ["curl", "Hace pedidos web; puede descargar o enviar datos a Internet."],
  ["wget", "Descarga archivos de Internet."],
  ["ssh", "Se conecta a otra máquina."],
  ["tm-agent", "Comando del agente de este tablero (por ejemplo, pedir ayuda)."],
  ["ls", "Lista los archivos de una carpeta. Solo lectura."],
  ["cat", "Muestra el contenido de un archivo. Solo lectura."],
  ["grep", "Busca texto dentro de archivos. Solo lectura."],
  ["find", "Busca archivos por nombre o tipo. Solo lectura (salvo con -delete o -exec)."],
  ["mkdir", "Crea carpetas."],
  ["cp", "Copia archivos."],
  ["mv", "Mueve o renombra archivos."],
  ["rm", "⚠️ BORRA archivos. No hay papelera."],
  ["chmod", "Cambia los permisos de archivos."],
  ["sudo", "⚠️ Ejecuta con permisos de administrador: puede cambiar cualquier cosa del sistema."],
];
const RISKY = /\b(rm|sudo|dd|mkfs|chmod|chown|kill|pkill|reboot|shutdown)\b|push|publish|reset --hard|--force|-rf|\|\s*(sh|bash)|>\s*\/|systemctl/;
function explainCmd(cmd) {
  const c = cmd.trim().replace(/^Bash\((.*?)(:\*)?\)$/, "$1");
  if (!c) return ["", false];
  const hit = CMD_HELP.find(([k]) => c === k || c.startsWith(k + " ")) || CMD_HELP.find(([k]) => c.split(/\s+/)[0] === k.split(" ")[0]);
  const risky = RISKY.test(c) || !!hit?.[1].startsWith("⚠️");
  return [hit ? hit[1] : "No tengo una explicación para este comando: leelo bien antes de permitirlo, o preguntale a la IA qué necesita hacer.", risky];
}
function grantDialog(found, asked) {
  return new Promise((resolve) => {
    const cmd = h("input", { value: found[0] || "", maxLength: 200, placeholder: "ej.: cargo build" });
    const why = h("div", { class: "gd-why" });
    const scope = h("select", {}, h("option", { value: "once" }, "Solo esta vez (la próxima ejecución de esta tarea)"), h("option", { value: "always" }, "Siempre (todas las tareas de esta IA)"));
    const upd = () => { const [t, risky] = explainCmd(cmd.value); why.className = "gd-why" + (risky ? " risky" : ""); why.textContent = t ? (risky && !t.startsWith("⚠️") ? "⚠️ Comando delicado. " : "") + t : "Escribí el comando que querés permitir."; };
    cmd.addEventListener("input", upd); upd();
    const done = (v) => { overlay.remove(); resolve(v); };
    const ok = () => { if (!cmd.value.trim()) return cmd.focus(); done({ command: cmd.value.trim(), always: scope.value === "always" }); };
    const overlay = h("div", { class: "overlay", onmousedown: (e) => e.target === overlay && done(null) },
      h("div", { class: "modal small grant-dlg", onkeydown: (e) => { if (e.key === "Escape") { e.stopPropagation(); done(null); } else if (e.key === "Enter") { e.preventDefault(); ok(); } } },
        h("h3", {}, "✅ Permitir un comando a la IA"),
        asked && h("div", { class: "gd-ask" }, h("b", {}, "La IA pidió: "), asked.slice(0, 400)),
        found.length > 1 && h("div", { class: "gd-chips" }, h("small", {}, "Comandos que mencionó: "), found.map((c) => h("button", { class: "ghost", onclick: () => { cmd.value = c; upd(); cmd.focus(); } }, c))),
        h("label", {}, "Comando a permitir", cmd),
        why,
        h("small", { class: "hint" }, "Se permite con cualquier argumento (por ejemplo «cargo build» también deja pasar «cargo build --release»). Un comando que esté en la lista de prohibidos sigue prohibido."),
        h("label", {}, "¿Por cuánto tiempo?", scope),
        h("div", { class: "row" }, h("button", { onclick: () => done(null) }, "Cancelar"), h("button", { class: "primary", onclick: ok }, "✅ Permitir"))));
    document.body.append(overlay); cmd.focus();
  });
}


// ---------- consumo de las IAs: tokens, costo, horas y cuotas ----------
const fmtTok = (n) => n >= 1e6 ? (n / 1e6).toFixed(n >= 1e7 ? 0 : 1) + " M" : n >= 1e3 ? (n / 1e3).toFixed(n >= 1e4 ? 0 : 1) + " k" : String(n);
const fmtUsd = (n) => "$" + (n >= 100 ? n.toFixed(0) : n.toFixed(2));
const fmtDur = (sec) => sec < 60 ? sec + " s" : sec < 3600 ? Math.round(sec / 60) + " min" : (sec / 3600).toFixed(1) + " h";
const quotaBar = (used, quota) => quota > 0
  ? h("div", { class: "qbar " + (used >= quota ? "over" : used >= quota * 0.8 ? "warn" : ""), title: `${fmtTok(used)} de ${fmtTok(quota)} tokens` }, h("i", { style: `width:${Math.min(100, (used / quota) * 100)}%` }))
  : null;
const usageLine = (u) => u ? `${fmtTok(u.tokens)} tokens · ${fmtUsd(u.cost_usd)} · ${fmtDur(u.seconds)} · ${u.runs} ejecuciones` : "—";

const openUsage = guard(async () => {
  if (!S.project) return;
  const [pu, tu] = await Promise.all([api("GET", `/api/projects/${S.project.id}/usage`), api("GET", `/api/teams/${S.team.id}/usage`)]);
  const pq = h("input", { type: "number", min: 0, step: 10000, value: pu.quota, disabled: !can("admin") });
  const tq = h("input", { type: "number", min: 0, step: 10000, value: tu.team.quota, disabled: !can("owner") });
  const saveQ = guard(async (kind) => {
    if (kind === "p") await api("PUT", `/api/projects/${S.project.id}/quota`, { tokens: Math.max(0, +pq.value || 0) });
    else await api("PUT", `/api/teams/${S.team.id}/quota`, { tokens: Math.max(0, +tq.value || 0) });
    toast("Cuota guardada"); document.getElementById("modal")?.remove(); openUsage();
  });
  const modal = h("div", { class: "overlay", id: "modal", onmousedown: (e) => e.target.id === "modal" && closeModal() },
    h("div", { class: "modal usage-modal" },
      h("h3", {}, "💰 Consumo de las IAs · " + S.project.name, h("span", { class: "spacer" }), h("button", { class: "ghost", onclick: closeModal }, "✕")),
      h("p", { class: "hint" }, "Los tokens son los frescos (entrada + salida + creación de caché). El costo y las horas vienen de lo que informa cada IA al terminar. Sin informe (IAs que no devuelven el gasto) solo se cuentan las horas."),
      h("h4", {}, "Este mes · proyecto"), h("div", {}, usageLine(pu.month)), quotaBar(pu.month.tokens, pu.quota),
      h("label", {}, "Cuota mensual del proyecto (tokens, 0 = sin límite)", h("div", { class: "row" }, pq, can("admin") && h("button", { class: "primary", onclick: () => saveQ("p") }, "Guardar"))),
      h("h4", {}, "Este mes · todo el equipo «" + S.team.name + "»"), h("div", {}, usageLine(tu.team)), quotaBar(tu.team.tokens, tu.team.quota),
      h("label", {}, "Cuota mensual del equipo (tokens, 0 = sin límite)", h("div", { class: "row" }, tq, can("owner") && h("button", { class: "primary", onclick: () => saveQ("t") }, "Guardar"))),
      h("h4", {}, "Por IA este mes (en este proyecto)"),
      pu.by_ai.length ? h("table", { class: "usage-tbl" }, h("tr", {}, ["IA", "Tokens", "Costo", "Horas", "Ejec."].map((x) => h("th", {}, x))),
        pu.by_ai.map((a) => h("tr", {}, h("td", {}, "🤖 " + a.name, a.model && h("small", { class: "inl" }, " · " + a.model)), h("td", {}, fmtTok(a.tokens)), h("td", {}, fmtUsd(a.cost_usd)), h("td", {}, fmtDur(a.seconds)), h("td", {}, a.runs))))
        : h("small", { class: "hint" }, "Todavía no hay ejecuciones este mes."),
      h("h4", {}, "Total histórico del proyecto"), h("div", {}, usageLine(pu.total)),
      h("small", { class: "hint" }, "Cuando una cuota se agota, las IAs no empiezan tareas nuevas y avisan en la tarea. Se reinicia el día 1 de cada mes (UTC).")));
  document.body.append(modal);
});

// bloque «Tokens a gastar» del detalle de la tarea: tope, estimación y lo gastado
function budgetBox(task, titleEl, descEl) {
  const inp = h("input", { type: "number", min: 0, step: 1000, value: 0, style: "max-width:160px" });
  const spent = h("small", { class: "hint" }, "");
  const basis = h("small", { class: "hint" }, "");
  api("GET", `/api/tasks/${task.id}/usage`).then((u) => { inp.value = u.budget; spent.textContent = u.runs ? `Gastó ${usageLine(u)}` : "Todavía no gastó nada."; }).catch(() => {});
  const est = guard(async () => {
    const r = await api("POST", `/api/projects/${task.project_id}/estimate`, { title: titleEl.value, description: descEl.value });
    inp.value = Math.round(r.tokens / 1000) * 1000 || r.tokens;
    basis.textContent = r.basis === "historial" ? `Estimación: promedio de las últimas ${r.runs} ejecuciones de este proyecto.` : "Estimación a ojo según el largo del pedido (todavía no hay historial en este proyecto): es solo una guía.";
  });
  const save = guard(async () => { await api("PUT", `/api/tasks/${task.id}/budget`, { tokens: Math.max(0, +inp.value || 0) }); toast(+inp.value > 0 ? "Tope guardado" : "Sin tope"); });
  return h("div", { class: "budget-box" },
    h("label", {}, "🪙 Tokens a gastar (tope; 0 = sin tope)",
      h("div", { class: "row" }, inp, h("button", { title: "Calcula cuántos tokens podría gastar la IA en esta tarea, según el historial del proyecto", onclick: est }, "🧮 Estimar"), h("button", { class: "primary", onclick: save }, "Guardar tope"))),
    basis, spent);
}

// ---------- licencia comercial ----------
// El aviso nunca bloquea nada: el programa es AGPL-3.0 y gratis; la licencia comercial es para quien no quiere cumplir la AGPL.
async function loadLicense() {
  S.lic = await api("GET", "/api/license").catch(() => null);
  render();
}

function licenseBanner() {
  const l = S.lic;
  const until = +store.get("lic_dismiss") || 0;
  if (!l || !l.banner || Date.now() < until) return null;
  return h("div", { class: "lic-banner" },
    h("span", {}, l.tipo === "empresa" ? "🏢 Uso en una empresa: seguís con la licencia AGPL-3.0 (publicando tus cambios) o comprás una licencia comercial." : `🏢 Este servidor ya supera el uso gratuito (${l.free.humans} personas y ${l.free.ais} IAs): seguís con la licencia AGPL-3.0 o comprás una licencia comercial.`),
    h("span", { class: "spacer" }),
    h("button", { class: "primary", onclick: openLicense }, "Ver planes"),
    h("button", { class: "ghost", title: "No volver a mostrar durante 7 días", onclick: () => { store.set("lic_dismiss", String(Date.now() + 7 * 864e5)); render(); } }, "Recordarme en 7 días"));
}

const openLicense = guard(async () => {
  document.getElementById("lic-modal")?.remove();
  const close = () => { clearInterval(S.licPoll); document.getElementById("lic-modal")?.remove(); };
  const l = await api("GET", "/api/license");
  S.lic = l;
  const body = h("div", { class: "lic-body" });
  const link = (href, text) => (/^https:\/\//.test(href || "") ? h("a", { href, target: "_blank", rel: "noopener noreferrer" }, text) : h("span", {}, text));
  const mono = (text, msg) => h("div", { class: "row" }, h("code", { class: "lic-mono" }, text), h("button", { onclick: () => copy(text, msg) }, "Copiar"));
  const head = () => {
    const kids = [h("p", {}, `Hoy: ${l.humans} personas · ${l.ais} IAs · ${l.projects} proyectos · ${l.teams} equipos. Uso gratuito sin aviso: hasta ${l.free.humans} personas y ${l.free.ais} IAs.`)];
    kids.push(l.licensed
      ? h("p", { class: "lic-ok" }, `✅ Licencia comercial «${l.license.plan}» vigente hasta ${String(l.license.hasta).slice(0, 10)}.`)
      : h("p", { class: "hint" }, "Rust TUI Manager es software libre (AGPL-3.0-only) y puede usarse gratis cumpliendo esa licencia. La licencia comercial es para quien no quiere cumplirla (por ejemplo, no publicar sus modificaciones)."));
    if (l.message) kids.push(h("p", { class: "hint" }, l.message));
    return kids;
  };
  const settings = () => {
    if (!l.admin) return [h("small", { class: "hint" }, "Solo la primera persona registrada del servidor puede comprar o cambiar estas opciones.")];
    const tipo = h("select", { onchange: guard(async (e) => { await api("PUT", "/api/license/settings", { tipo: e.target.value }); toast("Guardado"); loadLicense(); }) },
      h("option", { value: "", disabled: true, selected: !l.tipo }, "Elegí…"), h("option", { value: "personal", selected: l.tipo === "personal" }, "Personal"), h("option", { value: "empresa", selected: l.tipo === "empresa" }, "Empresa"));
    const tel = h("input", { type: "checkbox", checked: l.telemetry, disabled: l.telemetry_forced_off, onchange: guard(async (e) => { await api("PUT", "/api/license/settings", { telemetry: e.target.checked }); toast("Guardado"); }) });
    return [h("label", {}, "Este servidor se usa de forma…", tipo),
      h("label", { class: "row" }, tel, h("span", {}, "Avisar una vez por día que esta instalación existe (para estadísticas y para saber quién ya tiene licencia)")),
      h("small", { class: "hint" }, "Se envía: un código al azar guardado en tu base, la versión, el sistema operativo y cuántas personas, IAs, proyectos y equipos hay. Nunca nombres, correos, tareas ni contenido; la IP no se guarda (solo se usa para saber el país). También se apaga con TM_TELEMETRY=0." + (l.telemetry_forced_off ? " (Ahora está apagado por TM_TELEMETRY.)" : ""))];
  };
  const showOrder = (o, email) => {
    clearInterval(S.licPoll);
    const qr = h("div", { class: "lic-qr" });
    if (o.qr_svg) qr.innerHTML = o.qr_svg;
    const state = h("p", { class: "hint" }, "⏳ Esperando el pago… (se revisa solo cada pocos segundos)");
    body.replaceChildren(
      h("h4", {}, `Pagá exactamente ${o.monto} ${o.cripto} en la red ${o.red === "tron" ? "Tron (TRC-20)" : "Ethereum (ERC-20)"}`),
      h("p", { class: "hint" }, "Los últimos decimales identifican tu pago: mandá el monto exacto, sin redondear. Si el monto no coincide no se reconoce solo."),
      h("div", { class: "lic-pay" }, qr, h("div", {},
        h("label", {}, "Monto", mono(o.monto, "Monto copiado")),
        h("label", {}, "Dirección de pago", mono(o.direccion, "Dirección copiada"), link(o.explorador_direccion, "Ver la dirección en el explorador")),
        h("label", {}, `Contrato del token ${o.cripto} (verificá que sea el oficial)`, mono(o.contrato, "Contrato copiado"), link(o.explorador_token, "Ver el contrato en el explorador")))),
      h("small", { class: "hint" }, `El pedido vale hasta ${String(o.vence).slice(0, 16).replace("T", " ")} UTC. Al confirmarse el pago te llega un mail a ${email} con tu código y la licencia se activa sola.`),
      state, h("button", { onclick: openLicense }, "← Volver"));
    store.set("lic_order", JSON.stringify({ id: o.id, token: o.token, email }));
    const check = async () => {
      try {
        const r = await api("GET", `/api/license/order/${o.id}?token=${encodeURIComponent(o.token)}`);
        if (r.estado === "pagado") { clearInterval(S.licPoll); store.set("lic_order", null); toast("¡Pago recibido! Licencia activada"); openLicense(); loadLicense(); }
        else if (r.estado === "vencido") { clearInterval(S.licPoll); state.textContent = "El pedido venció. Si ya pagaste, escribinos con el hash de la transacción."; }
      } catch (e) { state.textContent = "No pude consultar el pago: " + e.message; }
    };
    S.licPoll = setInterval(check, 15000);
  };
  const buy = async () => {
    const pl = await api("GET", "/api/license/plans");
    if (!pl.planes.length || !pl.metodos.length) { body.replaceChildren(h("p", {}, "Todavía no hay planes o formas de pago disponibles. Probá más tarde."), h("button", { onclick: openLicense }, "← Volver")); return; }
    const email = h("input", { type: "email", placeholder: "tu@email.com (acá te llega el código)", maxLength: 120 });
    const plan = h("select", {}, pl.planes.map((p) => h("option", { value: p.plan }, `${p.nombre}: hasta ${p.max_usuarios} personas · USD ${p.precio_usd} por ${p.dias >= 365 ? "año" : p.dias + " días"}`)));
    const met = h("select", {}, pl.metodos.map((m, i) => h("option", { value: i }, `${m.cripto} en ${m.red === "tron" ? "Tron (TRC-20)" : "Ethereum (ERC-20)"}`)));
    const cont = h("small", { class: "hint" });
    const upd = () => { const m = pl.metodos[+met.value]; cont.replaceChildren("Contrato del token: ", h("code", {}, m.contrato), " ", link(m.explorador_token, "ver en el explorador")); };
    met.onchange = upd; upd();
    body.replaceChildren(h("h4", {}, "Comprar una licencia comercial"),
      h("label", {}, "Plan", plan), h("label", {}, "Pagar con", met), cont, h("label", {}, "Email", email),
      h("small", { class: "hint" }, "El email se usa solo para mandarte el código y el comprobante."),
      h("div", { class: "row" }, h("button", { class: "primary", onclick: guard(async () => {
        const m = pl.metodos[+met.value];
        const o = await api("POST", "/api/license/order", { plan: plan.value, email: email.value, cripto: m.cripto, red: m.red });
        showOrder(o, email.value.trim());
      }) }, "Generar el pago"), h("button", { onclick: openLicense }, "← Volver")));
  };
  const code = h("input", { placeholder: "TM-XXXX-XXXX-XXXX", maxLength: 40 });
  const act = guard(async () => { await api("POST", "/api/license/activate", { codigo: code.value }); toast("Licencia activada"); openLicense(); loadLicense(); });
  const main = () => body.replaceChildren(...[...head(), ...settings(),
    l.admin && h("div", { class: "row" }, h("button", { class: "primary", onclick: guard(buy) }, l.licensed ? "Renovar o cambiar de plan" : "Comprar una licencia comercial")),
    l.admin && h("label", {}, "¿Ya tenés un código?", h("div", { class: "row" }, code, h("button", { onclick: act }, "Activar")))].filter(Boolean));
  main();
  document.body.append(h("div", { class: "overlay", id: "lic-modal", onmousedown: (e) => e.target.id === "lic-modal" && close() },
    h("div", { class: "modal lic-modal" }, h("h3", {}, "🏢 Licencia", h("span", { class: "spacer" }), h("button", { class: "ghost", onclick: close }, "✕")), body)));
  const saved = (() => { try { return JSON.parse(store.get("lic_order") || "null"); } catch { return null; } })();
  if (saved && l.admin && !l.licensed) {
    body.append(h("button", { onclick: guard(async () => { const r = await api("GET", `/api/license/order/${saved.id}?token=${encodeURIComponent(saved.token)}`); toast(r.estado === "pendiente" ? "Pedido pendiente: generá uno nuevo si hace falta" : "Estado del pedido: " + r.estado); }) }, "Ver mi último pedido"));
  }
});
