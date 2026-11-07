"use strict";
// Cliente de CrewHelm. Todo el contenido de usuario se inserta con textContent (sin innerHTML).

const STATUS = [
  { id: "todo", label: "Por hacer", color: "var(--blue)" },
  { id: "doing", label: "En curso", color: "var(--yellow)" },
  { id: "done", label: "Hecho", color: "var(--green)" },
];
// Perfiles de uso: cambian cómo se llama cada columna y qué partes técnicas se ven. No cambian los datos.
const PROFILES = {
  dev: { name: "💻 Desarrollo de software", desc: "Tablero Por hacer / En curso / Hecho, terminales y carpeta del código.", labels: ["Por hacer", "En curso", "Hecho"], tech: true },
  content: { name: "📣 Contenido y redes", desc: "Borrador / En producción / Publicado, con el calendario como vista inicial. Sin partes técnicas.", labels: ["Borrador", "En producción", "Publicado"], tech: false, view: "calendar" },
  factory: { name: "🏭 Fábrica de apps", desc: "Varias apps en paralelo: Backlog / En construcción / Entregado.", labels: ["Backlog", "En construcción", "Entregado"], tech: true },
  office: { name: "🏛️ Oficina y estudios (administración, contabilidad, abogacía)", desc: "Pendiente / En trámite / Cerrado, con calendario de vencimientos y sin partes técnicas.", labels: ["Pendiente", "En trámite", "Cerrado"], tech: false, view: "calendar" },
  other: { name: "🏢 Otros / equipo general", desc: "Por hacer / En curso / Hecho, sin partes técnicas.", labels: ["Por hacer", "En curso", "Hecho"], tech: false },
};
const profileOf = () => PROFILES[S.profile] || PROFILES.dev;
const showTech = () => profileOf().tech;
function applyProfile(id) {
  S.profile = PROFILES[id] ? id : null;
  profileOf().labels.forEach((l, i) => { STATUS[i].label = l; });
}
const PRIO = ["Baja", "Media", "Alta", "Urgente"];
const ROLES = [["viewer", "Lector"], ["member", "Miembro"], ["admin", "Admin"], ["owner", "Dueño"]];
const RANK = { viewer: 0, member: 1, admin: 2, owner: 3 };
const AVATAR = ["#7aa2f7", "#bb9af7", "#9ece6a", "#e0af68", "#f7768e", "#7dcfff"];

const APP_NAME = "CrewHelm";
const store = {
  get: (k) => { try { return localStorage.getItem(k); } catch { return null; } },
  set: (k, v) => { try { v == null ? localStorage.removeItem(k) : localStorage.setItem(k, v); } catch {} },
};

const S = {
  sideOff: store.get("sideOff") === "1",
  panel: store.get("panel") === "1", overview: null, chatOpen: store.get("chat") === "1",
  // servida por tm-server (web): el servidor es el propio origen; en Tauri se elige a mano
  server: /^https?:$/.test(location.protocol) && location.hostname !== "tauri.localhost" ? location.origin : store.get("server") || "http://127.0.0.1:7878",
  token: store.get("token"),
  user: (() => { try { return JSON.parse(store.get("user") || "null"); } catch { return null; } })(),
  teams: [], team: null, projects: [], project: null,
  tasks: [], members: [], labels: [], online: false, query: "", view: store.get("view") || "board", month: new Date(),
  open: null, comments: [], ws: null, register: false, loginError: "",
  inbox: null, unread: 0, me: null, canvas: null, prefix: false,
};

const PANES = [];
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
// secciones de la barra lateral que se pueden contraer (se recuerda en el navegador)
const secOpen = (k) => store.get("sec_" + k) !== "0";
const secCls = (k) => (secOpen(k) ? "" : " collapsed");
const toggleSec = (k) => { store.set("sec_" + k, secOpen(k) ? "0" : "1"); render(true); };
const secTitle = (k, label) => h("button", { class: "ghost sec-toggle", "aria-expanded": String(secOpen(k)), title: secOpen(k) ? "Contraer" : "Expandir", onclick: () => toggleSec(k) }, label);
// tema: claro / oscuro
const THEMES = { light: ["☀️", "claro"], dark: ["🌙", "oscuro"] };
const themePref = () => (THEMES[store.get("theme")] ? store.get("theme") : "dark");
function applyTheme() {
  const t = themePref();
  document.documentElement.setAttribute("data-theme", t);
  document.querySelector('meta[name="theme-color"]')?.setAttribute("content", t === "light" ? "#e6e9ef" : "#0f1216");
}
function cycleTheme() { store.set("theme", themePref() === "light" ? "dark" : "light"); applyTheme(); render(true); }
applyTheme();

// Iconos de línea (estilo Lucide, 24×24, trazo currentColor). Los textos de la interfaz siguen empezando con su emoji
// (así no cambian las claves de i18n): h() lo reemplaza al dibujarlo por el ícono vectorial de esta tabla.
const ICON_PATHS = {
  users: '<path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M22 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/>',
  folder: '<path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"/>',
  bot: '<path d="M12 8V4H8"/><rect width="16" height="12" x="4" y="8" rx="2"/><path d="M2 14h2"/><path d="M20 14h2"/><path d="M15 13v2"/><path d="M9 13v2"/>',
  settings: '<path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/><circle cx="12" cy="12" r="3"/>',
  book: '<path d="M12 7v14"/><path d="M3 18a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h5a4 4 0 0 1 4 4 4 4 0 0 1 4-4h5a1 1 0 0 1 1 1v13a1 1 0 0 1-1 1h-6a3 3 0 0 0-3 3 3 3 0 0 0-3-3z"/>',
  message: '<path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/>',
  bell: '<path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9"/><path d="M10.3 21a1.94 1.94 0 0 0 3.4 0"/>',
  chart: '<path d="M3 3v16a2 2 0 0 0 2 2h16"/><path d="M18 17V9"/><path d="M13 17V5"/><path d="M8 17v-3"/>',
  mail: '<rect width="20" height="16" x="2" y="4" rx="2"/><path d="m22 7-8.97 5.7a1.94 1.94 0 0 1-2.06 0L2 7"/>',
  clock: '<circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/>',
  coins: '<circle cx="12" cy="12" r="10"/><path d="M16 8h-6a2 2 0 1 0 0 4h4a2 2 0 1 1 0 4H8"/><path d="M12 18V6"/>',
  search: '<circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/>',
  github: '<path d="M15 22v-4a4.8 4.8 0 0 0-1-3.5c3 0 6-2 6-5.5.08-1.25-.27-2.48-1-3.5.28-1.15.28-2.35 0-3.5 0 0-1 0-3 1.5-2.64-.5-5.36-.5-8 0C6 2 5 2 5 2c-.3 1.15-.3 2.35 0 3.5A5.403 5.403 0 0 0 4 9c0 3.5 3 5.5 6 5.5-.39.49-.68 1.05-.85 1.65-.17.6-.22 1.23-.15 1.85v4"/><path d="M9 18c-4.51 2-5-2-7-2"/>',
  monitor: '<rect width="20" height="14" x="2" y="3" rx="2"/><path d="M8 21h8"/><path d="M12 17v4"/>',
  pencil: '<path d="M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z"/><path d="m15 5 4 4"/>',
  file: '<path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/>',
  power: '<path d="M12 2v10"/><path d="M18.4 6.6a9 9 0 1 1-12.77.04"/>',
  check: '<circle cx="12" cy="12" r="10"/><path d="m9 12 2 2 4-4"/>',
};
const EMOJI_ICON = { "👥": "users", "📁": "folder", "🤖": "bot", "⚙": "settings", "📖": "book", "💬": "message", "🔔": "bell", "📊": "chart", "✉": "mail", "⏰": "clock", "💰": "coins", "🔎": "search", "🐙": "github", "🖥": "monitor", "✏": "pencil", "📄": "file", "⏻": "power", "✅": "check" };
const EMOJI_LEAD = /^(👥|📁|🤖|⚙|📖|💬|🔔|📊|✉|⏰|💰|🔎|🐙|🖥|✏|📄|⏻|✅)[️‍]*\s*/u;
const icon = (name) => { const s = document.createElementNS("http://www.w3.org/2000/svg", "svg"); s.setAttribute("viewBox", "0 0 24 24"); s.setAttribute("class", "ic"); s.setAttribute("aria-hidden", "true"); s.innerHTML = ICON_PATHS[name] || ""; return s; };

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
  for (const kid of kids.flat()) {
    if (kid == null || kid === false) continue;
    if (typeof kid !== "string") { el.append(kid); continue; }
    const txt = tr(kid), m = tag !== "option" && tag !== "title" && EMOJI_LEAD.exec(txt);
    if (m) { const ic = icon(EMOJI_ICON[m[1]]); if (txt.length > m[0].length) el.append(ic, txt.slice(m[0].length)); else { ic.classList.add("solo"); el.append(ic); } }
    else el.append(txt);
  }
  // tooltip automático para los botones que no traen uno propio
  if (tag === "button" && !el.title) { const tip = TIPS[kids.flat().find((k) => typeof k === "string")?.trim()]; if (tip) el.title = tr(tip); }
  return el;
}

// cuánto queda visible el aviso emergente (Preferencias): segundos; 0 = hasta que se toque. Tocarlo abre la tarea del aviso, si la tiene.
function toast(msg, error = false, note = null) {
  const t = document.getElementById("toast");
  t.textContent = tr(msg); t.className = (error ? "error" : "") + (note ? " link" : ""); t.hidden = false;
  t.title = note ? "Tocá para abrir la tarea" : "Tocá para cerrar";
  t.onclick = () => { t.hidden = true; clearTimeout(toast.t); if (note) jumpToNote(note); };
  clearTimeout(toast.t);
  let secs = parseFloat(store.get("toastSecs")); if (!(secs >= 0)) secs = 6;
  if (secs > 0) toast.t = setTimeout(() => (t.hidden = true), secs * 1000);
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
// avatar elegido por cada persona (emoji); sin elegir, se ven las iniciales. AV = {usuario: emoji}
const AV = {};
const AVATARS = "🙂 😀 😎 🤓 🥳 😴 🤠 🧐 🦊 🐱 🐶 🐼 🐨 🦁 🐯 🐸 🐵 🦄 🐙 🦉 🐧 🐢 🦖 🐝 🦋 🌵 🍀 🌻 🔥 ⚡ 🚀 🛸 🎸 🎮 ⚽ 🏀 🍕 ☕ 🎧 💎".split(" ");
const avatar = (name) => { const e = !botInfo(name) && AV[name]; return h("span", { class: "avatar" + (botInfo(name) ? " bot" : "") + (e ? " emoji" : ""), title: label(name), style: `background:${color(name)}` }, botInfo(name) ? "🤖" : e || name.slice(0, 2).toUpperCase()); };
async function loadAvatars() {
  try { const a = await api("GET", "/api/avatars"); for (const k of Object.keys(AV)) delete AV[k]; Object.assign(AV, a); } catch {}
}
const chooseChatExts = guard(async () => {
  const cfg = await api("GET", "/api/chat/exts"), on = new Set(cfg.allowed), close = () => document.getElementById("exts-modal")?.remove();
  document.body.append(h("div", { class: "overlay", id: "exts-modal", onmousedown: (e) => e.target.id === "exts-modal" && close() },
    h("div", { class: "modal" }, h("h3", {}, "Archivos que se pueden enviar por el chat"),
      h("small", {}, cfg.can_edit ? "Tildá los tipos permitidos (máx. 5 MB por archivo). Rige para todo el servidor." : "Solo el dueño de un equipo puede cambiar esto."),
      ...cfg.groups.map((g) => h("div", { class: "exts-group" }, h("b", {}, g.group),
        h("div", { class: "exts-list" }, g.exts.map((x) => h("label", { class: "inl" }, h("input", { type: "checkbox", checked: on.has(x), disabled: !cfg.can_edit, onchange: (e) => (e.target.checked ? on.add(x) : on.delete(x)) }), " ." + x))))),
      h("small", {}, "Nunca se permiten: programas (.exe, .msi, .bat…), scripts (.sh, .js, .ps1…), HTML y SVG."),
      h("div", { class: "row" }, h("button", { onclick: close }, "Cerrar"), h("span", { class: "spacer" }),
        cfg.can_edit && h("button", { class: "primary", onclick: guard(async () => { await api("PUT", "/api/chat/exts", { allowed: [...on] }); close(); toast("Archivos permitidos guardados"); }) }, "Guardar")))));
});
const chooseAvatar = () => {
  const mine = AV[S.user.username], close = () => document.getElementById("avatar-modal")?.remove();
  const pick = guard(async (e) => {
    await api("PUT", "/api/me/avatar", { avatar: e });
    if (e) AV[S.user.username] = e; else delete AV[S.user.username];
    close(); render(); toast(e ? "Avatar cambiado" : "Avatar: tus iniciales");
  });
  document.body.append(h("div", { class: "overlay", id: "avatar-modal", onmousedown: (e) => e.target.id === "avatar-modal" && close() },
    h("div", { class: "modal" }, h("h3", {}, "Elegí tu avatar"),
      h("small", {}, "Es el dibujito que ven los demás junto a tu nombre."),
      h("div", { class: "avatar-grid" }, AVATARS.map((e) => h("button", { class: "ghost" + (e === mine ? " on" : ""), onclick: () => pick(e) }, e))),
      h("div", { class: "row" }, h("button", { onclick: () => pick(null) }, "Usar mis iniciales"), h("span", { class: "spacer" }), h("button", { onclick: close }, "Cerrar")))));
};

// ---------- sesión ----------
const logoutAll = guard(async () => {
  if (!(await ask("¿Cerrar sesión en todos tus dispositivos?"))) return;
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
  loadAvatars();
  connectWs();
  // las preferencias de la terminal viajan con la cuenta: lo guardado en el servidor manda sobre el navegador
  api("GET", "/api/me/term-prefs").then((v) => { if (typeof v?.autoHerdr === "boolean") store.set("autoHerdr", v.autoHerdr ? "1" : "0"); }).catch(() => {});
  if (first) { loadInbox(true); loadLicense(); if (!S.teams.length && store.get("wizDone") !== "1") openWizard(); }
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

// una novedad lleva a SU tarea (cambia de equipo y de proyecto si hace falta); sin tarea, al equipo
const jumpToNote = guard(async (n) => {
  if (n.task_id) {
    const t = await api("GET", `/api/tasks/${n.task_id}`).catch(() => null);
    if (!t) { toast("Esa tarea ya no existe", true); return; }
    const team = S.teams.find((x) => x.id === (n.team_id ?? -1)) || S.team;
    if (team && team.id !== S.team?.id) await selectTeam(team);
    const p = S.projects.find((x) => x.id === t.project_id);
    if (p && (p.id !== S.project?.id || S.portfolio || S.terms)) await selectProject(p);
    const cur = S.tasks.find((x) => x.id === t.id) || t;
    openTask(cur);
  } else if (n.team_id) {
    const team = S.teams.find((x) => x.id === n.team_id);
    if (team && team.id !== S.team?.id) await selectTeam(team);
  }
});
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
  const KIND = { assigned: "📌", comment: "💬", member_added: "👥", role_changed: "🎖", message: "✉", mention: "@", reminder: "⏰", report: "📊" };
  document.body.append(h("div", { class: "overlay", id: "inbox-modal", onmousedown: (e) => e.target.id === "inbox-modal" && closeInbox() },
    h("div", { class: "modal wide" },
      h("h3", {}, `Novedades${S.unread ? ` (${S.unread} sin leer)` : ""}`, h("span", { class: "spacer" })),
      h("div", { class: "inbox-list" }, inb.notifications.length === 0 ? h("small", {}, "Nada nuevo.") :
        inb.notifications.map((n) => h("div", { class: "note-row" + (n.read ? "" : " unread") + (n.task_id || n.team_id ? " link" : ""), title: n.task_id ? "Ir a esta tarea" : n.team_id ? "Ir a este equipo" : "", onclick: (n.task_id || n.team_id) ? async () => { await closeInbox(); jumpToNote(n); } : undefined },
          h("span", {}, KIND[n.kind] || "•"), h("span", { class: "grow", style: "white-space:pre-wrap" }, n.text),
          h("small", {}, new Date(n.created_at).toLocaleString())))),
      h("h3", {}, `Mis tareas abiertas (${inb.tasks.length})`),
      h("div", { class: "inbox-list" }, inb.tasks.length === 0 ? h("small", {}, "No tenés tareas asignadas.") :
        inb.tasks.map((it) => h("div", { class: "note-row link", onclick: async () => { await closeInbox(); jumpToTask(it); } },
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
      sshKeysBox(),
      h("div", { class: "row" }, h("button", { onclick: close }, "Cerrar")))));
});

// claves SSH de la persona: con ellas entra por ssh al servidor, cada una a su espacio personal (los miembros dentro de un sandbox)
function sshKeysBox() {
  const box = h("div", { class: "sshkeys" });
  const key = h("textarea", { rows: 2, placeholder: "Pegá tu clave pública (ssh-ed25519 AAAA… o ssh-rsa AAAA…)", maxLength: 900 });
  const label = h("input", { placeholder: "Nombre (ej. mi notebook)", maxLength: 60 });
  const load = async () => {
    const list = await api("GET", "/api/me/ssh-keys").catch(() => []);
    box.replaceChildren(h("h4", {}, "🔑 Entrar por SSH"),
      h("small", {}, "Con tu clave pública entrás por ssh al servidor y caés en tu espacio personal, que se conserva entre sesiones. Si el administrador lo habilitó en el servidor (TM_SSH_KEYS=1), la clave se activa en un minuto."),
      ...list.map((k) => h("div", { class: "row" }, h("span", {}, `${k.label || "sin nombre"} · ${k.fingerprint}`), h("span", { class: "spacer" }),
        h("button", { class: "danger", onclick: guard(async () => { await api("DELETE", `/api/me/ssh-keys/${k.id}`); load(); }) }, "Quitar"))),
      key, label,
      h("div", { class: "row" }, h("span", { class: "spacer" }), h("button", { onclick: guard(async () => { await api("POST", "/api/me/ssh-keys", { key: key.value, label: label.value }); key.value = ""; label.value = ""; toast("Clave agregada"); load(); }) }, "Agregar clave")));
  };
  load();
  return box;
}

async function selectTeam(team) {
  S.portfolio = false; S.terms = false; S.team = team; S.projects = []; S.project = null; S.tasks = []; S.members = []; S.labels = []; S.bots = []; S.canvas = null;
  if (team) {
    [S.projects, S.members, S.labels, S.bots] = await Promise.all([
      api("GET", `/api/teams/${team.id}/projects`), api("GET", `/api/teams/${team.id}/members`),
      api("GET", `/api/teams/${team.id}/labels`), api("GET", `/api/teams/${team.id}/bots`).catch(() => []),
    ]);
    const pr = await api("GET", `/api/teams/${team.id}/profile`).catch(() => ({}));
    S.isExample = !!pr.is_example; applyProfile(pr.profile);
    if (profileOf().view && !store.get("view")) S.view = profileOf().view;
    await selectProject(S.projects[0] || null);
    loadOverview();
    CHAT.unread = {}; CHAT.total = 0; chatUnreadLoad(); loadInteg(); CHAT.tr = {}; loadTrCfg();
  } else render();
}
// Mensajes que le llegaron mientras no estaba conectado: cuenta los no leídos y, una vez por equipo, avisa con una burbuja.
async function chatUnreadLoad() {
  const tid = S.team?.id; if (!tid) return;
  try {
    const u = await api("GET", `/api/teams/${tid}/chat/unread`); if (S.team?.id !== tid) return;
    CHAT.unread = {}; CHAT.total = 0;
    for (const d of u.dm) { CHAT.unread["dm" + d.id] = d.n; CHAT.total += d.n; }
    for (const c of u.shared) { CHAT.unread[c.chan] = c.n; CHAT.total += c.n; }
    render();
    CHAT.announced = CHAT.announced || {};
    if (CHAT.total > 0 && !CHAT.announced[tid]) {
      CHAT.announced[tid] = true;
      const first = u.dm[0];
      chatBubble({ team_id: tid, from: first ? (u.dm.length > 1 ? u.dm.map((d) => d.name).join(", ") : first.name) : "Chat", from_user: first?.id, to_user: first ? S.user.id : null, project_id: null,
        body: `Tenés ${CHAT.total} mensaje${CHAT.total > 1 ? "s" : ""} sin leer`, id: -1 });
    }
  } catch {}
}

async function selectProject(p) {
  S.portfolio = false; S.terms = false; S.project = p; S.canvas = null; S.reviews = null; S.hold = null; S.tasks = p ? await api("GET", `/api/projects/${p.id}/tasks`) : [];
  S.ghCi = "";
  if (p) api("GET", `/api/projects/${p.id}/github/settings`).then((c) => { if (c.ci_status && S.project?.id === p.id) { S.ghCi = c.ci_state; render(); } }).catch(() => {});
  render();
  loadReviews(); loadFlows();
}
// recorrido de las tareas con flujo (para dibujarlo en las tarjetas)
async function loadFlows() {
  const id = S.project?.id; if (!id) return;
  try { const r = await api("GET", `/api/projects/${id}/flows`); if (S.project?.id === id) { S.flows = Object.fromEntries(r.map((f) => [f.task_id, f])); render(); } } catch {}
}
// revisión humana del trabajo de las IAs (función de los planes pagos): interruptor del proyecto y tareas esperando revisión
async function loadReviews() {
  const id = S.project?.id; if (!id) return;
  try { const r = await api("GET", `/api/projects/${id}/reviews`); if (S.project?.id === id) { S.reviews = r; render(); } } catch {}
  try { const r = await api("GET", `/api/projects/${id}/continuation`); if (S.project?.id === id) { S.hold = r; render(); } } catch {}
}
// tareas con continuación: lo que una IA termina queda en «En curso», zona de espera, hasta que una persona lo pase a Hecho
const toggleHold = guard(async () => {
  const on = !S.hold.enabled;
  if (!(await ask(on ? "¿Que lo que una IA termina quede «esperando tu próximo paso» (En curso) en vez de pasar a Hecho? Vos lo pasás a Hecho cuando lo cierres." : "¿Que lo que una IA termina pase directo a Hecho?"))) return;
  await api("PUT", `/api/projects/${S.project.id}/continuation`, { enabled: on });
  toast(on ? "Las IAs dejan sus tramos esperando tu próximo paso" : "Las IAs pasan sus tareas directo a Hecho"); loadReviews();
});
const toggleReview = guard(async () => {
  const on = !S.reviews.enabled;
  if (!(await ask(on ? "¿Exigir revisión humana en este proyecto? Cuando una IA termine una tarea, quedará «en revisión» y una persona tendrá que aprobarla o pedir cambios." : "¿Dejar de exigir revisión en este proyecto? Las IAs volverán a poder pasar tareas a Hecha solas."))) return;
  await api("PUT", `/api/projects/${S.project.id}/reviews`, { enabled: on });
  toast(on ? "Revisión humana activada" : "Revisión humana apagada"); loadReviews();
});
const inReview = (taskId) => !!S.reviews?.pending?.some((r) => r.task_id === taskId);

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
      clearTimeout(onEvent.fl); onEvent.fl = setTimeout(loadFlows, 700);
      const i = S.tasks.findIndex((t) => t.id === data.id);
      i >= 0 ? (S.tasks[i] = data) : S.tasks.push(data);
      if (S.reviews) { clearTimeout(onEvent.rv); onEvent.rv = setTimeout(loadReviews, 600); }
    }
  } else if (type === "chat") {
    chatAdd(data);
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
    toast("🔔 " + data.text, false, data);
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

// ---------- confirmación propia (en lugar del confirm() del navegador) ----------
function ask(msg, okLabel) {
  return new Promise((resolve) => {
    const danger = /borrar|quitar|salir|desconectar|cerrar sesi/i.test(msg);
    const done = (v) => { overlay.remove(); resolve(v); };
    const overlay = h("div", { class: "overlay", style: "z-index:10000", onmousedown: (e) => e.target === overlay && done(false) },
      h("div", { class: "modal small ask-dlg", onkeydown: (e) => { if (e.key === "Escape") { e.stopPropagation(); done(false); } } },
        h("div", { class: "ask-ico" + (danger ? " danger" : "") }, danger ? "⚠️" : "❔"),
        h("p", { class: "ask-msg" }, msg),
        h("div", { class: "row" }, h("button", { onclick: () => done(false) }, "Cancelar"),
          h("button", { class: danger ? "danger" : "primary", id: "ask-ok", onclick: () => done(true) }, okLabel || (danger ? "Sí, continuar" : "Aceptar")))));
    document.body.append(overlay); overlay.querySelector("#ask-ok").focus();
  });
}

// ---------- acciones ----------
const createTeam = guard(async () => {
  const f = await form("Nuevo equipo", [{ name: "name", label: "Nombre del equipo", max: 60 }], "Crear"); if (!f) return;
  const t = await api("POST", "/api/teams", { name: f.name });
  S.teams.push(t); await selectTeam(t);
});
// Integrar: revisar y juntar (o devolver) el trabajo que el equipo manda con «Enviar a revisión» (Pull Requests de GitHub)
const PR_STATE = { clean: ["✅ Listo para integrar", "ok"], has_hooks: ["✅ Listo para integrar", "ok"], unstable: ["⚠ Las pruebas no pasan", "bad"], dirty: ["⛔ Tiene conflictos: la persona debe actualizar su copia y volver a enviar", "bad"], blocked: ["⛔ GitHub lo bloquea (falta una aprobación o regla)", "bad"], behind: ["Desactualizado respecto de la rama principal", ""], draft: ["Borrador", ""] };
async function openIntegrar() {
  const p = S.project; if (!p) return;
  const close = () => document.getElementById("integrar")?.remove();
  const body = h("div", {}, h("p", { class: "hint" }, "⏳ Buscando lo que mandó el equipo…"));
  const modal = h("div", { class: "overlay", id: "integrar", onmousedown: (e) => e.target.id === "integrar" && close() },
    h("div", { class: "modal wide" }, h("h3", {}, "🔀 Integrar trabajo de «" + p.name + "»", h("span", { class: "spacer" }), h("button", { class: "ghost", onclick: close, title: "Cerrar" }, "✕")), body));
  document.getElementById("integrar")?.remove(); document.body.append(modal);
  const squash = h("input", { type: "checkbox" });
  const diffView = (f) => h("details", { class: "diff" }, h("summary", {}, `${f.name}  `, h("span", { class: "add" }, "+" + f.add), " ", h("span", { class: "del" }, "−" + f.del), f.status === "added" ? " (nuevo)" : f.status === "removed" ? " (borrado)" : ""),
    f.binary ? h("small", { class: "hint" }, "Archivo que no se puede mostrar como texto (imagen, documento…).")
      : h("pre", { class: "patch" }, ...f.patch.split("\n").map((l) => h("div", { class: l.startsWith("+") ? "add" : l.startsWith("-") ? "del" : l.startsWith("@@") ? "hunk" : "" }, l || " ")), f.cut && h("div", { class: "hint" }, "… (se muestra solo el principio; el resto está en GitHub)")));
  const load = async () => {
    try {
      const r = await api("GET", `/api/projects/${p.id}/prs`);
      const cards = r.prs.map((x) => {
        const [stTxt, stCls] = x.draft ? PR_STATE.draft : (PR_STATE[x.state] || [x.state || "", ""]);
        const det = h("div", {});
        return h("div", { class: "gitopt" },
          h("b", {}, x.title), h("small", { class: "hint" }, `de ${x.by} · rama ${x.branch} · ${x.files ?? "?"} archivo(s) · `, h("span", { class: "add" }, "+" + (x.add ?? 0)), " ", h("span", { class: "del" }, "−" + (x.del ?? 0))),
          stTxt && h("small", { class: stCls === "bad" ? "warn" : "hint" }, stTxt),
          h("div", { class: "row" },
            h("button", { class: "ghost", title: "Muestra qué archivos y líneas cambian  ·  git diff main...rama", onclick: guard(async () => {
              det.replaceChildren(h("small", { class: "hint" }, "⏳ Cargando cambios…"));
              const f = await api("GET", `/api/projects/${p.id}/prs/${x.number}/files`);
              det.replaceChildren(...(f.files.length ? f.files.map(diffView) : [h("small", { class: "hint" }, "Sin cambios de archivos.")]));
            }) }, "👁 Ver cambios"),
            r.can_merge && h("button", { class: "primary", disabled: x.draft || x.mergeable === false, title: "Junta este trabajo con la rama principal y actualiza la carpeta del proyecto  ·  git merge rama", onclick: guard(async () => {
              if (!(await ask(`¿Integrar «${x.title}» de ${x.by} al proyecto?`))) return;
              body.replaceChildren(h("p", { class: "hint" }, "⏳ Integrando y actualizando la carpeta del proyecto…"));
              const m = await api("POST", `/api/projects/${p.id}/prs/${x.number}/merge`, { squash: squash.checked });
              toast(m.synced ? "Integrado y carpeta del proyecto actualizada" : "Integrado en GitHub (la carpeta se actualiza cuando el agente esté disponible)");
              load();
            }) }, "✅ Integrar"),
            r.can_merge && h("button", { class: "ghost danger", title: "Cierra el pedido sin juntarlo y le avisa a la persona  ·  git branch -D rama (en GitHub: cerrar el Pull Request)", onclick: guard(async () => {
              const f = await form("Devolver «" + x.title + "»", [{ name: "comment", label: "¿Qué tiene que corregir? (se lo mandamos a " + x.by + ")", type: "textarea", max: 2000 }]);
              if (!f) return;
              await api("POST", `/api/projects/${p.id}/prs/${x.number}/close`, { comment: f.comment || "" }); toast("Devuelto"); load();
            }) }, "↩ Devolver"),
            h("a", { class: "btnlike", href: x.url, target: "_blank", rel: "noopener", title: "Abrir en GitHub" }, "🐙")),
          det);
      });
      body.replaceChildren(
        h("small", { class: "hint" }, "Repositorio " + r.repo + ". Acá llega lo que las personas mandan con «Enviar a revisión». " + (r.can_merge ? "Revisá los cambios y, si está bien, integralo." : "Solo los owners pueden integrar o devolver.")),
        r.can_merge && h("label", { class: "chk", title: "Junta todos los pasos del trabajo en uno solo, más prolijo en el historial  ·  git merge --squash" }, squash, " Juntar en un solo cambio"),
        ...(cards.length ? cards : [h("p", { class: "hint" }, "No hay trabajo esperando revisión. 🎉")]),
        h("div", { class: "actions" }, h("button", { class: "ghost", onclick: load }, "↻ Actualizar"), h("button", { class: "ghost", onclick: close }, "Cerrar")));
    } catch (e) { body.replaceChildren(h("p", { class: "err" }, "⚠ " + e.message), h("div", { class: "actions" }, h("button", { class: "ghost", onclick: close }, "Cerrar"))); }
  };
  load();
}

// ventana con el asistente de git de un proyecto (al crearlo, o desde el aviso «Sin repositorio»)
function openGitSetup(p) {
  const close = () => document.getElementById("git-setup")?.remove();
  const modal = h("div", { class: "overlay", id: "git-setup", onmousedown: (e) => e.target.id === "git-setup" && close() },
    h("div", { class: "modal wide" }, h("h3", {}, "🐙 Repositorio de «" + p.name + "»", h("span", { class: "spacer" }), h("button", { class: "ghost", onclick: close, title: "Cerrar" }, "✕")),
      h("small", { class: "hint" }, "El repositorio guarda el historial y permite que cada persona trabaje en su propia rama y la envíe a revisión. Sin él, la carpeta queda de solo lectura para el equipo."),
      gitBox(p.id),
      h("div", { class: "actions" }, h("button", { class: "ghost", onclick: close, title: "Podés hacerlo después desde el aviso «Sin repositorio» o desde 🖥️ Terminales → ⚙ Conexión" }, "Más tarde"))));
  document.body.append(modal);
}
const requestRepo = guard(async (pid) => {
  const r = await api("POST", `/api/projects/${pid}/git/request`);
  toast(r.notified ? "Listo: les avisé a los owners del equipo" : "No hay owners a quienes avisar", !r.notified);
});
const createProject = guard(async () => {
  const f = await form("Nuevo proyecto", [
    { name: "name", label: "Nombre del proyecto", max: 60 },
    { name: "work_dir", label: "Carpeta de trabajo, dónde vive el proyecto (opcional; si no existe, la IA la crea)", max: 300, placeholder: "/home/ana/proyectos/app" },
    { name: "repo", label: "Repositorio (opcional)", max: 300, placeholder: "usuario/repo o URL" },
  ], "Crear"); if (!f) return;
  const p = await api("POST", `/api/teams/${S.team.id}/projects`, { name: f.name, work_dir: f.work_dir, repo: f.repo });
  if (!S.projects.some((x) => x.id === p.id)) S.projects.push(p);
  await selectProject(p);
  if (can("owner") && !p.repo) openGitSetup(p);
});
// ---------- IAs (cuentas bot con token propio) ----------
const mcpConfig = (token) => JSON.stringify({ mcpServers: { "rust-tui-manager": { command: "tm-mcp", env: { TM_URL: S.server, TM_TOKEN: token } } } }, null, 2);
const copy = (text, msg) => navigator.clipboard?.writeText(text).then(() => toast(msg));
const agentCmd = (token) => `TM_URL=${S.server} \\\nTM_TOKEN=${token || "tmk_…"} \\\nTM_AGENT_CMD='claude -p "$TM_TASK_PROMPT" --permission-mode acceptEdits' \\\ntm-agent`;
const agentCmdGemini = (token) => `TM_URL=${S.server} \\\nTM_TOKEN=${token || "tmk_…"} \\\nGEMINI_API_KEY=tu-clave-de-gemini \\\nTM_AGENT_CMD='gemini -p "$TM_TASK_PROMPT" --approval-mode auto_edit' \\\ntm-agent`;
function agentsGuide() {
  const overlay = h("div", { class: "overlay", onmousedown: (e) => e.target === overlay && overlay.remove() },
    h("div", { class: "modal ia" },
      h("h3", {}, "🤖 Que una IA trabaje sola: tm-agent"),
      h("p", {}, "Una IA solo actúa mientras algo la despierta. tm-agent es ese «algo»: corre en la máquina donde está la carpeta del proyecto, mira este servidor y, cuando le asignás una tarea a la IA, abre un agente en la carpeta de trabajo, comenta y pasa la tarea a En curso y Hecho."),
      h("ol", { class: "guide" },
        h("li", {}, "Creá la IA en la barra lateral (🤖 IAs → «+») y copiá su token."),
        h("li", {}, "Definí la carpeta de trabajo del proyecto (⚙ Proyecto): tiene que existir en la máquina donde corre tm-agent."),
        h("li", {}, "Instalá tm-agent en esa máquina: cargo install --git https://github.com/polblancoo/crewhelm crewhelm-agent"),
        h("li", {}, "Ejecutalo con el token y el comando de tu agente (Claude Code, Gemini CLI, un script…). Ejemplo:"),
        h("pre", { class: "bot-cfg" }, agentCmd()),
        h("li", {}, "Con Gemini CLI usá este (no entiende --permission-mode; la clave sale de aistudio.google.com/apikey, o iniciá sesión una vez con «gemini»). Si algo falla, el motivo queda como comentario ❌ en la tarea de la IA:"),
        h("pre", { class: "bot-cfg" }, agentCmdGemini()),
        h("li", {}, "Asignale una tarea a la IA: en unos segundos pasa a En curso, queda comentada y termina en Hecho. Si falla, vuelve a Por hacer con el error.")),
      h("p", {}, "⚠ El texto de las tareas llega al agente como instrucciones: dejalo correr con una carpeta y permisos acotados, y asigná tareas solo desde cuentas de confianza. Instructivo completo (servicio, seguridad, variables): docs/AGENTES.md en el repositorio."),
      h("div", { class: "row" }, h("button", { onclick: () => copy(agentCmd(), "Comando copiado") }, "Copiar ejemplo Claude"), h("button", { onclick: () => copy(agentCmdGemini(), "Comando copiado") }, "Copiar ejemplo Gemini"), h("button", { class: "primary", onclick: () => overlay.remove() }, "Cerrar"))));
  document.body.append(overlay);
}
function showBotToken(name, token, cmd) {
  const overlay = h("div", { class: "overlay", onmousedown: (e) => e.target === overlay && overlay.remove() },
    h("div", { class: "modal ia" },
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
setInterval(async () => {
  if (!S.token || !S.team || document.hidden) return;
  const next = await api("GET", `/api/teams/${S.team.id}/bots`).catch(() => null);
  if (!next) return;
  const sig = (l) => (l || []).map((b) => `${b.id}${b.online}${b.run_on}${b.active}`).join();
  if (sig(next) !== sig(S.bots)) { S.bots = next; render(); }
}, 20000);
const loadBots = async () => { S.bots = await api("GET", `/api/teams/${S.team.id}/bots`).catch(() => []); render(); };
// Proveedores de IA para el alta: etiqueta, si es paga (pide clave), modelos para elegir y cómo se lanza
const AI_KINDS = {
  claude: { name: "Claude (Anthropic)", paid: true, key: "ANTHROPIC_API_KEY", models: ["Opus", "Sonnet", "Haiku"],
    env: (m) => `TM_CLAUDE_MODEL=${m.toLowerCase()}`, cmd: () => "$HOME/.config/tm-agent/run-claude.sh",
    setup: "# Una vez: copiá run-claude.sh, whitelist.txt y blacklist.txt (carpeta examples/ del repo) a ~/.config/tm-agent/\n# Claude pide permiso por la web (🙋) antes de usar un comando que no esté permitido.\n# Si usás Claude con tu suscripción (claude login), no hace falta ninguna clave." },
  gemini: { name: "Gemini (Google)", paid: true, key: "GEMINI_API_KEY", models: ["gemini-2.5-pro", "gemini-2.5-flash"],
    cmd: (m) => `gemini -p "$TM_TASK_PROMPT" -m ${m} --approval-mode auto_edit`,
    setup: "# Gemini usa --approval-mode (no --permission-mode, que es de Claude). auto_edit deja editar archivos; yolo permite todo.\n# Necesita GEMINI_API_KEY (https://aistudio.google.com/apikey) o haber iniciado sesión una vez con: gemini\n# Probalo antes a mano: gemini -p \"decime hola\"" },
  openrouter: { name: "OpenRouter (modelos gratis y de pago)", paid: true, key: "OPENROUTER_API_KEY", models: ["qwen/qwen3-coder:free", "deepseek/deepseek-r1-0528:free", "mistralai/devstral-2512:free", "moonshotai/kimi-k2:free", "meta-llama/llama-3.3-70b-instruct:free"],
    cmd: (m) => `opencode run -m openrouter/${m} "$TM_TASK_PROMPT"`, setup: "# Necesita opencode (npm i -g opencode-ai) y una clave de https://openrouter.ai/keys\n# Los modelos gratis terminan en :free" },
  openai: { name: "ChatGPT / Codex (OpenAI)", paid: true, key: "OPENAI_API_KEY", models: ["gpt-5", "gpt-5-codex"],
    cmd: (m) => `codex exec "$TM_TASK_PROMPT" -m ${m}` },
  local: { name: "Modelo propio de la empresa (local)", paid: false, local: true, models: [], cmd: (m) => `codex exec "$TM_TASK_PROMPT" -m ${m}` },
  otra: { name: "Otra (escribo el comando yo)", paid: false, models: [], cmd: () => `tu-comando "$TM_TASK_PROMPT"` },
};
const aiCmd = (token, kind, model, key, url) => {
  const k = AI_KINDS[kind];
  return `TM_URL=${S.server} \\\nTM_TOKEN=${token} \\\n` + (k.paid && key ? `${k.key}=${key} \\\n` : "") + (k.env ? `${k.env(model || "opus")} \\\n` : "")
    + (k.local ? `OPENAI_BASE_URL=${url || "http://servidor-ia:11434/v1"} \\\nOPENAI_API_KEY=${key || "${OPENAI_API_KEY:-sin-clave}"} \\\n` : "") + `TM_AGENT_CMD='${k.cmd(model || "modelo")}' \\\ntm-agent` + (k.setup ? `\n\n${k.setup}` : "") + (k.paid && !key ? `\n\n# Usa la variable ${k.key} de tu máquina. Si no la tenés:\n# export ${k.key}=tu-clave   (agregala a ~/.bashrc para que quede)` : "");
};
// lista de comandos permitidos de una IA (se abre desde su ficha y desde un pedido de ayuda)
const editAllow = guard(async (b) => {
  const f = await form("Comandos permitidos de " + b.username, [{ name: "allow", label: "Uno por línea, ej. git clone · cargo build · Bash(npm test:*). Se suman a la lista base (whitelist.txt) del agente. «/permitir siempre …» en un pedido también los agrega acá. Lo prohibido siempre gana.", type: "textarea", value: b.allow || "", max: 4000 }], "Guardar"); if (!f) return;
  await api("PATCH", `/api/bots/${b.id}`, { allow: f.allow }); await loadBots(); toast("Lista guardada");
});
const editDeny = guard(async (b) => {
  const f = await form("Comandos prohibidos de " + b.username, [{ name: "deny", label: "Uno por línea, ej. Bash(rm:*) · Bash(git push:*). La IA no puede ejecutarlos; si los necesita, te avisa con una alarma 🙋.", type: "textarea", value: b.deny || "", max: 4000 }], "Guardar"); if (!f) return;
  await api("PATCH", `/api/bots/${b.id}`, { deny: f.deny }); await loadBots(); toast("Lista guardada");
});
const toggleFull = guard(async (b) => {
  if (!b.full_access && !(await ask(`«${b.username}» podrá ejecutar CUALQUIER comando salvo los de 🚫 Comandos prohibidos (de la IA y generales). Antes de activarlo revisá esa lista. ¿Activar autorización total?`, "Activar"))) return;
  await api("PATCH", `/api/bots/${b.id}`, { full_access: !b.full_access }); await loadBots();
  toast(b.full_access ? "Autorización total apagada" : "Autorización total activada: vale desde la próxima tarea de la IA");
});
const createBot = guard(async () => {
  const rn = await api("GET", "/api/runner").catch(() => ({ allowed: false }));
  const run = h("input", { type: "checkbox", checked: !!(rn.allowed && rn.agent_found) });
  const runL = h("label", { class: "row" }, run, h("span", {}, "Que este servidor la ejecute solo (sin comandos)"),
    h("small", { class: "hint" }, rn.allowed ? "Funciona si el código del proyecto está en la misma máquina que este servidor. Si el servidor está en la nube, desmarcalo y seguí el instructivo que aparece al crearla." : "Solo la primera persona registrada puede activarlo, y el servidor debe tener tm-agent instalado."));
  runL.style.display = rn.allowed ? "" : "none";
  const kind = h("select", {}, Object.entries(AI_KINDS).map(([v, k]) => h("option", { value: v }, k.name + (k.paid ? " · paga" : ""))));
  const model = h("select", {});
  const own = h("input", { placeholder: "Escribí el nombre del modelo", maxLength: 90, style: "display:none" });
  const key = h("input", { type: "password", placeholder: "Vacío = usa la variable de entorno de tu máquina", autocomplete: "off" });
  const keyL = h("label", {}, "Clave de la IA ", key, h("small", { class: "hint" }, "Opcional. En Linux/Mac, si ya tenés la variable (ANTHROPIC_API_KEY, GEMINI_API_KEY u OPENAI_API_KEY) exportada, dejá esto vacío: se usa esa. Nunca se guarda en el servidor."));
  const url = h("input", { placeholder: "Dirección del servidor de IA (ej. http://10.0.0.5:11434/v1)", maxLength: 200 });
  const urlL = h("label", {}, "Servidor de la IA ", url, h("small", { class: "hint" }, "Te la da el área de IT. Si el servidor pide clave, escribila abajo; si no, dejala vacía."));
  const name = h("input", { placeholder: "Nombre (ej. claude-ana)", maxLength: 32, oninput: () => { name.value = name.value.replace(/\s+/g, "-").replace(/[^\p{L}\p{N}_.-]/gu, ""); } });
  const role = h("select", {}, h("option", { value: "member" }, "Miembro (crea y edita sus tareas)"), h("option", { value: "viewer" }, "Lector (solo mira)"));
  const sync = () => {
    const k = AI_KINDS[kind.value];
    model.replaceChildren(...k.models.map((m) => h("option", { value: m }, m)), h("option", { value: "__otro" }, "✏ Escribir otro…"));
    keyL.style.display = k.paid || k.local ? "" : "none";
    urlL.style.display = k.local ? "" : "none";
    if (k.local) model.value = "__otro";
    pick();
  };
  const pick = () => { own.style.display = model.value === "__otro" ? "" : "none"; };
  kind.onchange = sync; model.onchange = pick; sync();
  const overlay = h("div", { class: "overlay", onmousedown: (e) => e.target === overlay && overlay.remove() },
    h("div", { class: "modal ia" }, h("h3", {}, "Nueva IA en el equipo"),
      h("label", {}, "Nombre", name, h("small", { class: "hint" }, "Cómo se va a ver en el tablero. Los espacios se cambian por «-» (ej. open-router).")), h("label", {}, "¿Qué IA es?", kind, h("small", { class: "hint" }, "Elegí la marca. Las pagas necesitan una clave (API key) de tu cuenta. «Modelo propio» es uno que corre en un servidor de tu empresa.")), h("label", {}, "Modelo", model, own, h("small", { class: "hint" }, "Elegí uno de la lista. Si el tuyo no está, tocá «Escribir otro…» y escribí su nombre tal cual lo da el proveedor o IT.")), urlL, keyL, h("label", {}, "Rol", role, h("small", { class: "hint" }, "Miembro: puede crear y editar tareas. Lector: solo mira.")),
      runL,
      h("div", { class: "row" }, h("span", { class: "spacer" }), h("button", { onclick: () => overlay.remove() }, "Cancelar"),
        h("button", { class: "primary", onclick: guard(async () => {
          const m = (model.value === "__otro" ? own.value : model.value).trim();
          if (!name.value.trim() || !m) { toast("Falta el nombre o el modelo"); return; }
          const tag = kind.value === "otra" || kind.value === "local" ? "ia" : kind.value;
          const r = await api("POST", `/api/teams/${S.team.id}/bots`, { name: name.value.trim(), role: role.value, model: `${tag}:${m}` });
          overlay.remove();
          S.members = await api("GET", `/api/teams/${S.team.id}/members`);
          await loadBots();
          if (run.checked && kind.value !== "otra") {
            if (kind.value === "openrouter" && key.value.trim()) await api("PUT", "/api/runner/openrouter-key", { key: key.value.trim() });
            await api("PUT", `/api/bots/${r.user.id}/runner`, { kind: kind.value, model: m, url: url.value.trim(), enabled: true });
            toast(`${r.user.username} activada: el servidor la ejecuta. Asignale una tarea o probala con 🧪`);
            runnerModal(S.bots.find((x) => x.id === r.user.id) || { id: r.user.id, username: r.user.username });
            return;
          }
          showBotToken(r.user.username, r.token, aiCmd(r.token, kind.value, m, key.value, url.value.trim()));
        }) }, "Crear"))));
  document.body.append(overlay);
});
// prueba de conexión: una tarea mínima asignada a la IA; si el agente está conectado, la toma y la termina
const testBot = async (b) => {
  const note = h("p", {}, "Enviando la tarea de prueba…");
  const box = h("div", { class: "overlay", onmousedown: (e) => e.target === box && (stop(), box.remove()) },
    h("div", { class: "modal small" }, h("h3", {}, `🧪 Prueba de ${b.username}`), note,
      h("div", { class: "row" }, h("button", { onclick: () => { stop(); box.remove(); } }, "Cerrar"))));
  let alive = true; const stop = () => { alive = false; };
  document.body.append(box);
  const t = await api("POST", `/api/projects/${S.project.id}/tasks`, { title: "🧪 Prueba de conexión", description: "Prueba automática. Respondé solo «OK» con un comentario y marcá la tarea como hecha. No modifiques archivos ni ejecutes comandos.", priority: 0, assignee_id: b.id }).catch((e) => { note.textContent = "No se pudo crear la tarea: " + e.message; return null; });
  if (!t) return;
  const t0 = Date.now();
  while (alive && Date.now() - t0 < 120000) {
    await new Promise((r) => setTimeout(r, 3000));
    if (!alive) return;
    const cur = (await api("GET", `/api/projects/${S.project.id}/tasks`).catch(() => [])).find((x) => x.id === t.id);
    if (!cur) { note.textContent = "La tarea de prueba se borró."; return; }
    const secs = Math.round((Date.now() - t0) / 1000);
    if (cur.status === "done") {
      const cs = await api("GET", `/api/tasks/${t.id}/comments`).catch(() => []);
      const last = cs.filter((c) => c.username === b.username).pop();
      note.textContent = `✅ La IA respondió en ${secs} s` + (last ? `: «${last.body.slice(0, 200)}»` : " (terminó la tarea sin comentario).");
      return;
    }
    note.textContent = cur.status === "doing" ? `⏳ La IA tomó la tarea y la está haciendo… (${secs} s)` : `⌛ Esperando que el agente de ${b.username} la tome… (${secs} s)`;
  }
  if (alive) note.textContent = "❌ No respondió en 2 minutos. Revisá que el agente (tm-agent) de esta IA esté corriendo y apunte a este servidor: la tarea de prueba quedó en el tablero.";
};
// ejecución en el servidor: estado de la IA, activar o detener
const RUN_KINDS = { openrouter: ["OpenRouter (modelos gratis y de pago)", ["qwen/qwen3-coder:free", "deepseek/deepseek-r1-0528:free", "mistralai/devstral-2512:free", "moonshotai/kimi-k2:free", "meta-llama/llama-3.3-70b-instruct:free"]], claude: ["Claude (Anthropic)", ["Opus", "Sonnet", "Haiku"]], gemini: ["Gemini (Google)", ["gemini-2.5-pro", "gemini-2.5-flash"]], openai: ["ChatGPT / Codex (OpenAI)", ["gpt-5", "gpt-5-codex"]], local: ["Modelo propio de la empresa (local)", []] };
const runnerModal = async (b) => {
  document.getElementById("runner-modal")?.remove();
  const [cur, av] = await Promise.all([api("GET", `/api/bots/${b.id}/runner`).catch(() => ({ configured: false })), api("GET", "/api/runner").catch(() => ({}))]);
  const kind = h("select", {}, Object.entries(RUN_KINDS).map(([v, [n]]) => h("option", { value: v, selected: cur.kind === v }, n)));
  const model = h("input", { value: cur.model || "", placeholder: "Modelo (ej. Opus, gemini-2.5-pro, llama3.1:70b)", maxLength: 90 });
  const dl = h("datalist", { id: "runner-models" }); model.setAttribute("list", "runner-models");
  const url = h("input", { value: cur.url || "", placeholder: "http://10.0.0.5:11434/v1", maxLength: 200 });
  const urlL = h("label", {}, "Servidor de la IA (lo da IT)", url);
  const orKey = h("input", { type: "password", placeholder: av.openrouter_key ? "Ya hay una clave guardada; escribí otra para cambiarla" : "Pegá tu clave de OpenRouter (sk-or-…)", autocomplete: "off" });
  const orL = h("label", {}, "Clave de OpenRouter", orKey, h("small", { class: "hint" }, "Se guarda solo en el servidor (archivo privado ~/.config/tm-server/openrouter.env), nunca en la base de datos ni se vuelve a mostrar."));
  const sync = () => { orL.style.display = kind.value === "openrouter" ? "" : "none"; dl.replaceChildren(...RUN_KINDS[kind.value][1].map((m) => h("option", { value: m }))); urlL.style.display = kind.value === "local" ? "" : "none"; };
  kind.onchange = sync; sync();
  const st = cur.enabled ? (cur.running ? `🟢 Conectada desde ${new Date(cur.since).toLocaleString()}` : `🔴 Activada pero no está corriendo${cur.last_exit ? " (" + cur.last_exit + ")" : ""}. Se reintenta sola.`) : "⚪ No activada";
  const save = (enabled) => guard(async () => {
    if (enabled && !model.value.trim()) { toast("Falta el modelo"); return; }
    if (enabled && kind.value === "openrouter" && !orKey.value.trim() && !av.openrouter_key) { toast("Falta la clave de OpenRouter"); return; }
    if (kind.value === "openrouter" && orKey.value.trim()) await api("PUT", "/api/runner/openrouter-key", { key: orKey.value.trim() });
    await api("PUT", `/api/bots/${b.id}/runner`, { kind: kind.value, model: model.value.trim() || cur.model || "x", url: url.value.trim(), enabled });
    toast(enabled ? "Activada: en unos segundos aparece conectada" : "Detenida"); runnerModal(b);
  });
  const tool = av.tools && av.tools[kind.value === "local" ? "openai" : kind.value] === false ? h("p", { class: "warn" }, `No encuentro el programa de esta IA instalado en el servidor: ${({ openrouter: "opencode (npm i -g opencode-ai)", claude: "claude (npm i -g @anthropic-ai/claude-code)", gemini: "gemini (npm i -g @google/gemini-cli)", openai: "codex (npm i -g @openai/codex)", local: "codex (npm i -g @openai/codex)" })[kind.value]}. Instalalo ahí y se conecta.`) : null;
  const box = h("div", { class: "overlay", id: "runner-modal", onmousedown: (e) => e.target === box && box.remove() },
    h("div", { class: "modal small" }, h("h3", {}, `⚙ ${b.username}: ejecución en el servidor`), h("p", {}, st),
      !av.allowed ? h("p", { class: "warn" }, "Solo la primera persona registrada puede activar esto, y el servidor no debe tener TM_RUNNER=0.") : [
        !av.agent_found && h("p", { class: "warn" }, "El servidor no encuentra el programa tm-agent: instalalo en la misma máquina (cargo install crewhelm-agent)."),
        h("label", {}, "¿Qué IA es?", kind), h("label", {}, "Modelo", model, dl), urlL, orL, tool,
        h("small", { class: "hint" }, "La clave de la IA se toma de las variables de entorno del servidor (ANTHROPIC_API_KEY, GEMINI_API_KEY, OPENAI_API_KEY) o de su sesión iniciada. No se guarda acá."),
        h("div", { class: "row" }, cur.enabled && h("button", { class: "danger", onclick: save(false) }, "Detener"), h("span", { class: "spacer" }),
          h("button", { onclick: () => box.remove() }, "Cerrar"), h("button", { class: "primary", onclick: save(true) }, cur.enabled ? "Guardar y reiniciar" : "Activar"))]));
  document.body.append(box);
};
// Comandos habituales: lo que la IA pidió antes y un catálogo común, para sumar a permitidos; y los peligrosos, para prohibir
const CMD_CATALOG = {
  "Git (solo mirar)": ["git status", "git diff", "git log", "git show", "git branch", "git fetch"],
  "Git (cambios locales)": ["git add", "git checkout", "git switch", "git stash", "git restore"],
  "Rust": ["cargo check", "cargo build", "cargo test", "cargo fmt", "cargo clippy", "cargo run"],
  "Node": ["npm test", "npm run build", "npm run lint", "npm install", "node --check", "npx tsc --noEmit"],
  "Python": ["python3 -m pytest", "python3 -m py_compile", "python3 --version", "pip list"],
  "Archivos (leer y crear)": ["ls", "cat", "head", "tail", "grep", "find", "wc", "tree", "diff", "mkdir", "touch", "cp", "mv"],
  "Sistema (solo mirar)": ["pwd", "which", "date", "echo", "uname", "df", "ps"],
};
const CMD_DANGER = ["Bash(rm -rf:*)", "Bash(sudo:*)", "Bash(git push:*)", "Bash(git push --force:*)", "Bash(git reset --hard:*)", "Bash(git clean:*)", "Bash(git commit:*)",
  "Bash(chmod -R:*)", "Bash(chown:*)", "Bash(dd:*)", "Bash(mkfs:*)", "Bash(systemctl:*)", "Bash(shutdown:*)", "Bash(reboot:*)", "Bash(kill:*)", "Bash(pkill:*)",
  "Bash(curl:*)", "Bash(wget:*)", "Bash(ssh:*)", "Bash(scp:*)", "Bash(npm publish:*)", "Bash(cargo publish:*)"];
const mergeLines = (cur, add) => [...new Set([...(cur || "").split("\n").map((l) => l.trim()).filter(Boolean), ...add])].join("\n");
const commandsDialog = guard(async (b) => {
  const sug = await api("GET", `/api/bots/${b.id}/suggest`).catch(() => ({ asked: [], requests: 0 }));
  const allowed = new Set((b.allow || "").split("\n").map((l) => l.trim()).filter(Boolean));
  const denied = new Set((b.deny || "").split("\n").map((l) => l.trim()).filter(Boolean));
  const closeIt = () => box.remove();
  const picks = (items, on) => items.map((c) => {
    const cb = h("input", { type: "checkbox", checked: on && !allowed.has(c) });
    return { c, cb, row: h("label", { class: "sub" }, cb, h("code", {}, c), allowed.has(c) && h("small", { class: "hint" }, "ya permitido")) };
  });
  const asked = picks(sug.asked.filter((x) => !x.allowed).map((x) => x.cmd), true);
  const askedRows = sug.asked.filter((x) => !x.allowed).map((x, i) => { asked[i].row.append(h("small", { class: "hint" }, `pedido ${x.count} ${x.count === 1 ? "vez" : "veces"}`)); return asked[i].row; });
  const catalog = Object.entries(CMD_CATALOG).map(([g, list]) => ({ g, items: picks(list, false) }));
  const danger = CMD_DANGER.map((c) => { const cb = h("input", { type: "checkbox", checked: !denied.has(c) && ["Bash(rm -rf:*)", "Bash(sudo:*)", "Bash(git push:*)", "Bash(git reset --hard:*)", "Bash(git clean:*)"].includes(c) });
    return { c, cb, row: h("label", { class: "sub" }, cb, h("code", {}, c), denied.has(c) && h("small", { class: "hint" }, "ya prohibido")) }; });
  const chosen = (arr) => arr.filter((x) => x.cb.checked).map((x) => x.c);
  const saveAllow = guard(async () => {
    const add = [...chosen(asked), ...catalog.flatMap((g) => chosen(g.items))];
    if (!add.length) { toast("Tildá al menos un comando", true); return; }
    await api("PATCH", `/api/bots/${b.id}`, { allow: mergeLines(b.allow, add) }); await loadBots();
    toast(`${add.length} comando(s) agregados a lo permitido de ${b.username}`); closeIt();
  });
  const saveDeny = guard(async () => {
    const add = chosen(danger);
    if (!add.length) { toast("Tildá al menos un comando", true); return; }
    await api("PATCH", `/api/bots/${b.id}`, { deny: mergeLines(b.deny, add) }); await loadBots();
    toast(`${add.length} comando(s) agregados a lo prohibido de ${b.username}`); closeIt();
  });
  const box = h("div", { class: "overlay", onmousedown: (e) => e.target === box && closeIt() },
    h("div", { class: "modal wide" }, h("h3", {}, `📋 Comandos de ${b.username}`),
      h("h4", {}, `Lo que pidió antes (${sug.requests} pedido${sug.requests === 1 ? "" : "s"} revisados)`),
      askedRows.length ? h("div", { class: "subs" }, askedRows) : h("small", { class: "hint" }, "Todavía no pidió comandos nuevos, o ya los tenés todos permitidos."),
      h("h4", {}, "Comandos comunes (tildá los que quieras permitir)"),
      catalog.map((g) => h("details", {}, h("summary", {}, g.g), h("div", { class: "subs" }, g.items.map((x) => x.row)))),
      h("div", { class: "row" }, h("span", { class: "spacer" }), h("button", { class: "primary", onclick: saveAllow }, "✅ Agregar a permitidos")),
      h("h4", {}, "Peligrosos: conviene prohibirlos (siempre ganan sobre lo permitido)"),
      h("div", { class: "subs" }, danger.map((x) => x.row)),
      h("div", { class: "row" }, h("span", { class: "spacer" }), h("button", { onclick: closeIt }, "Cerrar"), h("button", { class: "danger", onclick: saveDeny }, "🚫 Agregar a prohibidos"))));
  document.body.append(box);
});
// Perfil confiable: la IA puede todo salvo una lista corta de cosas peligrosas (así no vuelve a pedir permiso por cada cosa)
const TRUST_DENY = ["Bash(sudo:*)", "Bash(git push:*)", "Bash(git commit:*)", "Bash(rm -rf:*)", "Bash(git reset --hard:*)", "Bash(git clean:*)",
  "Bash(systemctl:*)", "Bash(dd:*)", "Bash(mkfs:*)", "Bash(cargo publish:*)", "Bash(npm publish:*)"];
const trustProfile = async (b) => {
  if (!(await ask(`«${b.username}» va a poder usar todas las herramientas y comandos SIN pedir permiso, salvo estos, que quedan prohibidos:\n\n${TRUST_DENY.map((c) => c.replace(/^Bash\((.*):\*\)$/, "$1")).join(" · ")}\n\nLas listas pasan a manejarse desde la app (los archivos del agente se ignoran). Después podés ajustar lo prohibido en su ficha. ¿Activar?`, "Activar perfil confiable"))) return false;
  await api("PATCH", `/api/bots/${b.id}`, { full_access: true, deny: mergeLines(b.deny, TRUST_DENY), lists_from_app: true }); await loadBots();
  toast(`${b.username}: perfil confiable activado, no va a volver a pedir permisos`); return true;
};
const botModal = (b) => {
  const admin = can("admin");
  const overlay = h("div", { class: "overlay", onmousedown: (e) => e.target === overlay && overlay.remove() },
    h("div", { class: "modal ia" },
      h("h3", {}, `🤖 ${b.username}`, b.model && h("small", { class: "inl" }, " · " + b.model)),
      h("p", {}, `${b.role === "viewer" ? "Lector" : "Miembro"} · token ${b.active ? "activo" : "revocado"} · ${b.last_used ? "último uso " + new Date(b.last_used).toLocaleString() : "sin usar"}${b.created_by ? " · creada por " + b.created_by : ""}${b.full_access ? " · 🔓 autorización total (solo rige lo prohibido)" : ""}`),
      (!b.lists_from_app && (b.base_deny || b.base_allow)) && (() => {
        const lines = (t) => (t || "").split("\n").map((l) => l.trim()).filter(Boolean);
        const [bd, ba] = [lines(b.base_deny), lines(b.base_allow)];
        const run = (patch, msg) => guard(async () => { await api("PATCH", `/api/bots/${b.id}`, patch); overlay.remove(); await loadBots(); toast(msg); });
        return h("details", { class: "gh-cfg", open: !b.lists_from_app && bd.length > 0 },
          h("summary", {}, b.lists_from_app ? "📄 Los archivos del agente (whitelist.txt / blacklist.txt) ya no rigen: las listas se manejan desde acá"
            : `📄 Rigen también los archivos del agente: ${bd.length} prohibidos y ${ba.length} permitidos (no se borran desde la app)`),
          bd.length > 0 && h("p", { class: "hint" }, "🚫 blacklist.txt: ", h("code", {}, bd.join("  ·  "))),
          ba.length > 0 && h("p", { class: "hint" }, "✅ whitelist.txt: ", h("code", {}, ba.join("  ·  "))),
          admin && h("div", { class: "row" }, b.lists_from_app
            ? h("button", { onclick: run({ lists_from_app: false }, "Vuelven a regir los archivos del agente") }, "Volver a usar los archivos")
            : h("button", { class: "primary", title: "Copia esas listas a las de la IA en la app (donde se pueden editar) y el agente deja de leer los archivos", onclick: guard(async () => {
                if (!(await ask("Se copian las listas de los archivos a las de esta IA en la app y el agente deja de leer los archivos. Después podés quitar de la app lo que no quieras. ¿Pasarlas?", "Pasar a la app"))) return;
                await run({ deny: mergeLines(b.deny, bd), allow: mergeLines(b.allow, ba), lists_from_app: true }, "Listas pasadas a la app: ahora las editás desde 🚫 y ✅")();
              }) }, "Pasar a la app (para editarlas acá)")));
      })(),
      h("div", { class: "row botonera" },
        admin && h("button", { onclick: guard(async () => {
          const f = await form("IA / modelo", [{ name: "model", label: "Qué IA es (ej. claude:Opus)", value: b.model, max: 40 }], "Guardar"); if (!f) return;
          await api("PATCH", `/api/bots/${b.id}`, { model: f.model }); overlay.remove(); await loadBots();
        }) }, "Cambiar modelo"),
        admin && h("button", { title: "Cómo corre esta IA: sin aislar, con una copia de trabajo propia por tarea, o dentro de un sandbox sin acceso al resto de la máquina", onclick: guard(async () => {
          const ISO = [["off", "Ninguno (como hasta ahora)"], ["copy", "Copia de trabajo por tarea, con su rama (sin sandbox)"], ["sandbox", "Sandbox: copia + sin acceso al resto de la máquina ni a la red salvo lo permitido"]];
          const f = await form("🛡 Aislamiento de " + b.username, [{ name: "iso", label: "Cada ejecución de esta IA corre…", type: "select", options: ISO, value: b.isolation || "off" }], "Guardar"); if (!f) return;
          await api("PATCH", `/api/bots/${b.id}`, { isolation: f.iso === "off" ? "" : f.iso }); overlay.remove(); await loadBots();
          toast(f.iso === "sandbox" ? "Sandbox activado: vale desde la próxima tarea (necesita bwrap en la máquina del agente)" : f.iso === "copy" ? "Copia de trabajo por tarea activada" : "Aislamiento apagado");
        }) }, b.isolation === "sandbox" ? "🛡 Aislamiento: sandbox" : b.isolation === "copy" ? "🛡 Aislamiento: copia por tarea" : "🛡 Aislamiento"),
        admin && h("button", { onclick: guard(async () => { overlay.remove(); await editDeny(b); }) }, "🚫 Comandos prohibidos"),
        admin && h("button", { title: "Comandos que esta IA puede usar sin pedir permiso", onclick: guard(async () => { await editAllow(b); overlay.remove(); }) }, "✅ Comandos permitidos"),
        admin && h("button", { class: "primary", title: "Autorización total con una lista corta de cosas peligrosas prohibidas: la IA no vuelve a pedir permisos", onclick: guard(async () => { if (await trustProfile(b)) { overlay.remove(); } }) }, "⚡ Perfil confiable"),
        admin && h("button", { title: "Lo que esta IA suele pedir y un catálogo de comandos comunes para sumar a lo permitido", onclick: () => { overlay.remove(); commandsDialog(b); } }, "📋 Comandos habituales"),
        admin && h("button", { class: b.full_access ? "danger" : "", title: "Permitir todo menos lo que figura en 🚫 Comandos prohibidos", onclick: guard(async () => {
          if (!b.full_access && !(await ask(`«${b.username}» podrá ejecutar CUALQUIER comando salvo los de 🚫 Comandos prohibidos (de la IA y generales). Antes de activarlo revisá esa lista: git push, rm -rf, sudo, systemctl… ¿Activar autorización total?`, "Activar"))) return;
          await api("PATCH", `/api/bots/${b.id}`, { full_access: !b.full_access }); overlay.remove(); await loadBots();
          toast(b.full_access ? "Autorización total apagada" : "Autorización total activada: vale desde la próxima tarea de la IA");
        }) }, b.full_access ? "🔓 Autorización total: SÍ" : "🔒 Autorización total"),
        admin && h("button", { onclick: guard(async () => { const r = await api("POST", `/api/bots/${b.id}/token`); overlay.remove(); await loadBots(); showBotToken(b.username, r.token); }) }, "Nuevo token"),
        admin && h("button", { title: "Hacer que el servidor ejecute esta IA, sin comandos", onclick: () => { overlay.remove(); runnerModal(b); } }, "⚙ Ejecución"),
        S.project && can("member") && b.active && h("button", { title: "Le manda una tarea simple a esta IA y espera que la conteste", onclick: guard(async () => { overlay.remove(); await testBot(b); }) }, "🧪 Probar"),
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
// Pantalla de quien todavía no está en ningún equipo: acá mismo pega el código que le pasaron, o crea su equipo.
function welcomeView() {
  const code = h("input", { placeholder: "Pegá acá el código de invitación", maxLength: 64, autofocus: true, autocomplete: "off" });
  const go = guard(async () => {
    const c = code.value.trim(); if (!c) return toast("Pegá el código que te pasaron", true);
    const team = await api("POST", `/api/invites/${encodeURIComponent(c)}/accept`, {});
    S.teams = await api("GET", "/api/teams");
    await selectTeam(S.teams.find((t) => t.id === team.id) || S.teams[0]);
    toast(`Te uniste a ${team.name}`);
  });
  code.onkeydown = (e) => { if (e.key === "Enter") go(); };
  return h("div", { class: "welcome" },
    h("h2", {}, `👋 Bienvenido${S.user?.username ? ", " + S.user.username : ""}`),
    h("p", {}, "Todavía no estás en ningún equipo. Si alguien te invitó, pegá el código que te pasó:"),
    h("div", { class: "row" }, code, h("button", { class: "primary", onclick: go }, "Unirme")),
    h("p", { class: "hint" }, "El código te lo da el administrador del equipo (por mensaje, mail o como te lo haya mandado). Una vez que lo usás, entrás al tablero del equipo."),
    h("hr"),
    h("p", {}, "¿Querés empezar por tu cuenta?"),
    h("div", { class: "row" }, h("button", { onclick: createTeam }, "Crear mi propio equipo"), h("button", { class: "ghost", onclick: () => openWizard() }, "Guía de inicio")));
}
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
        if (!(await ask(m.id === S.user.id ? "¿Salir del equipo?" : `¿Quitar a ${m.username}?`))) return;
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
        const msg = inviteMessage(r.code, await reachableServer());
        inv.replaceChildren(h("code", {}, r.code), h("button", { onclick: () => navigator.clipboard?.writeText(r.code).then(() => toast("Código copiado")) }, "Copiar"),
          h("button", { title: "Texto listo para pegar en WhatsApp, Telegram, etc.", onclick: () => navigator.clipboard?.writeText(msg).then(() => toast("Invitación copiada: pegala en un chat")) }, "Copiar invitación"),
          h("a", { class: "btnlike", href: `mailto:?subject=${encodeURIComponent("Invitación a " + APP_NAME)}&body=${encodeURIComponent(msg)}` }, "Enviar por email"),
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

// ---------- chat del equipo (panel lateral) ----------
// El panel se arma una sola vez y se vuelve a colocar en cada dibujo de la pantalla, así no pierde lo que se está escribiendo.
const CHAT = { tr: {}, orig: {}, trCfg: null, trBusy: false, el: null, chan: { t: "team" }, msgs: [], people: [], unread: {}, pending: null, err: "", teamId: null, timer: null, total: 0 };
const chanKey = (c) => (c.t === "dm" ? "dm" + c.id : c.t === "project" ? "p" + c.id : "team");
const msgKey = (m) => (m.to_user != null ? "dm" + (m.from_user === S.user.id ? m.to_user : m.from_user) : m.project_id != null ? "p" + m.project_id : "team");
const inChan = (m, c) => msgKey(m) === chanKey(c);
const toggleChat = () => {
  S.chatOpen = !S.chatOpen; store.set("chat", S.chatOpen ? "1" : null);
  try { if (S.chatOpen && window.Notification && Notification.permission === "default") Notification.requestPermission(); } catch {}
  clearInterval(CHAT.timer);
  if (S.chatOpen) { CHAT.timer = setInterval(chatPresence, 20000); CHAT.teamId = null; }
  render();
};
async function chatPresence() {
  if (!S.team || !S.chatOpen) return;
  await loadAvatars();
  try { const pr = await api("GET", `/api/teams/${S.team.id}/presence`); CHAT.people = pr.people; CHAT.bots = pr.bots || []; chatDraw(); } catch {}
}
async function chatHistory() {
  const c = CHAT.chan, tid = S.team.id;
  const q = c.t === "dm" ? `with=${c.id}` : c.t === "project" ? `project=${c.id}` : "";
  try { const r = await api("GET", `/api/teams/${tid}/chat?${q}`); if (CHAT.chan !== c) return; CHAT.msgs = r; CHAT.err = ""; CHAT.unread[chanKey(c)] = 0; }
  catch (e) {
    if (CHAT.chan !== c) return;
    CHAT.msgs = []; CHAT.err = e.message; CHAT.https = "";
    try { const st = await api("GET", "/api/chat/status"); if (!st.secure) CHAT.https = st.https_url || ""; } catch {}
  }
  chatDraw();
}
function chatSwitch(c) { CHAT.chan = c; CHAT.msgs = []; chatHistory(); }
function chatAdd(m) {
  if (m.team_id !== S.team?.id) { if (m.from_user !== S.user.id && S.teams.some((t) => t.id === m.team_id)) chatBubble(m); return; }
  if (CHAT.msgs.some((x) => x.id === m.id)) return;
  const mine = m.from_user === S.user.id, seen = S.chatOpen && inChan(m, CHAT.chan) && !document.hidden;
  if (S.chatOpen && inChan(m, CHAT.chan)) CHAT.msgs.push(m);
  else if (!mine) {
    CHAT.unread[msgKey(m)] = (CHAT.unread[msgKey(m)] || 0) + 1; CHAT.total++;
    if (!S.chatOpen) render();
  }
  if (!mine && !seen) chatBubble(m);
  if (!mine && seen) api("POST", `/api/teams/${m.team_id}/chat/read`, { chan: msgKey(m), last_id: m.id }).catch(() => {});
  chatDraw();
}
// Burbuja que avisa de un mensaje nuevo aunque el chat esté cerrado o en otro canal; tocarla abre esa conversación.
// Con la pestaña en segundo plano suma un aviso del navegador (si se permitió), un sonido corto y el contador en el título.
async function chatOpenFor(m) {
  if (m.team_id !== S.team?.id) { const t = S.teams.find((x) => x.id === m.team_id); if (t) await selectTeam(t); }
  const c = m.to_user != null ? { t: "dm", id: m.from_user } : m.project_id != null ? { t: "project", id: m.project_id } : { t: "team" };
  if (!S.chatOpen) toggleChat();
  chatSwitch(c);
}
function chatBubble(m) {
  const txt = ((m.body || "").replace(CALL_RE, "").trim() || "📎 " + (m.file?.name || "archivo")).slice(0, 140);
  const other = m.team_id !== S.team?.id ? S.teams.find((t) => t.id === m.team_id)?.name : "";
  const where = other ? `en el equipo ${other}${m.to_user != null ? " (privado)" : ""}` : m.to_user != null ? "te escribió en privado" : m.project_id != null ? "en " + (S.projects.find((p) => p.id === m.project_id)?.name || "un proyecto") : "en el equipo";
  let box = document.getElementById("chat-bubbles");
  if (!box) { box = h("div", { id: "chat-bubbles" }); document.body.append(box); }
  let secs = parseFloat(store.get("bubbleSecs")); if (!(secs >= 0)) secs = 10;
  const b = h("div", { class: "chat-bubble", title: "Tocá para abrir la conversación" },
    h("div", { class: "cb-av" + (AV[m.from] ? " emoji" : ""), style: `background:${color(m.from)}` }, AV[m.from] || m.from.slice(0, 2).toUpperCase()),
    h("div", { class: "cb-main" }, h("div", { class: "cb-head" }, h("b", {}, m.from), h("small", {}, where)), h("div", { class: "cb-txt" }, txt)),
    h("button", { class: "cb-x", title: "Cerrar", onclick: (e) => { e.stopPropagation(); b.remove(); } }, "✕"),
    secs > 0 && h("div", { class: "cb-bar", style: `animation-duration:${secs}s` }));
  b.onclick = () => { b.remove(); chatOpenFor(m); };
  box.prepend(b);
  while (box.children.length > 3) box.lastChild.remove();
  if (secs > 0) setTimeout(() => b.remove(), secs * 1000);
  try {
    const a = new (window.AudioContext || window.webkitAudioContext)(), o = a.createOscillator(), g = a.createGain();
    o.connect(g); g.connect(a.destination); o.frequency.value = 880; g.gain.value = 0.05; o.start(); o.stop(a.currentTime + 0.12);
  } catch {}
  if (document.hidden) {
    chatTitle();
    try { if (window.Notification && Notification.permission === "granted") { const n = new Notification("💬 " + m.from + " " + where, { body: txt, tag: "tm-chat" }); n.onclick = () => { window.focus(); chatOpenFor(m); n.close(); }; } } catch {}
  }
}
function chatTitle() {
  CHAT.hiddenN = (CHAT.hiddenN || 0) + 1;
  if (!CHAT.baseTitle) CHAT.baseTitle = document.title;
  document.title = `(${CHAT.hiddenN}) 💬 ${CHAT.baseTitle}`;
}
document.addEventListener("visibilitychange", () => { if (!document.hidden && CHAT.baseTitle) { document.title = CHAT.baseTitle; CHAT.hiddenN = 0; } });
const chatSend = guard(async () => {
  const body = CHAT.inp.value.trim(); if (!body && !CHAT.pending) return;
  const c = CHAT.chan, payload = { body, file_id: CHAT.pending?.id };
  if (c.t === "dm") payload.to = c.id; else if (c.t === "project") payload.project_id = c.id;
  const m = await api("POST", `/api/teams/${S.team.id}/chat`, payload);
  CHAT.inp.value = ""; CHAT.pending = null; chatAdd(m);
});
const chatPick = guard(async (e) => {
  const f = e.target.files[0]; e.target.value = ""; if (!f) return;
  const r = await (await apiRaw("POST", `/api/teams/${S.team.id}/chat/files?name=${encodeURIComponent(f.name)}`, f)).json();
  CHAT.pending = r; chatDraw();
});
const chatAudit = guard(async () => {
  const rows = await api("GET", `/api/teams/${S.team.id}/chat/audit`);
  const m = h("div", { class: "overlay", id: "audit-modal", onmousedown: (e) => e.target.id === "audit-modal" && m.remove() },
    h("div", { class: "modal wide" }, h("h3", {}, "🧾 Registro del chat", h("span", { class: "spacer" }), h("button", { class: "ghost", onclick: () => m.remove() }, "✕")),
      h("small", { class: "hint" }, "Quién hizo qué, cuándo y desde qué dirección (últimos 300 movimientos). Solo lo ven administradores y dueños."),
      h("div", { class: "audit-list" }, rows.length ? rows.map((r) => h("div", { class: "audit-row" }, h("small", {}, new Date(r.at).toLocaleString()), h("b", {}, r.who || "?"), h("span", {}, r.action.replace(/_/g, " ")), h("span", { class: "hint" }, r.detail), h("small", {}, r.ip))) : h("small", {}, "Todavía no hay movimientos."))));
  document.body.append(m);
});
// ---------- traducción del chat: cada uno escribe en su idioma y lee en el suyo ----------
const trWanted = () => !!CHAT.trCfg?.enabled && store.get("chatTr") !== "0";
async function loadTrCfg() {
  const id = S.team?.id; if (!id) { CHAT.trCfg = null; return; }
  try { const r = await api("GET", `/api/teams/${id}/chat/translate-cfg`); if (S.team?.id === id) { CHAT.trCfg = r; chatDraw(); } } catch { CHAT.trCfg = null; }
}
const URL_RE = /(https?:\/\/[^\s<>"')]+)/g;
const linkify = (t) => t.split(URL_RE).map((p, i) => (i % 2 ? h("a", { href: p, target: "_blank", rel: "noopener noreferrer" }, p) : p));
function chatBody(m) {
  const call = m.body && CALL_RE.exec(m.body);
  if (call) return h("div", { class: "chat-body" }, m.body.replace(CALL_RE, "").trim(), " ", h("button", { class: "primary", onclick: () => openCall(call[1]) }, "📞 Unirme"));
  const k = tr.lang + ":" + m.id, t = CHAT.tr[k], foreign = m.from_user !== S.user.id && typeof t === "string" && t !== "=";
  if (foreign && trWanted() && !CHAT.orig[m.id])
    return h("div", { class: "chat-body" }, t, h("small", { class: "chat-tr", title: "Mostrar lo que escribió la otra persona", onclick: () => { CHAT.orig[m.id] = 1; chatDraw(); } }, " 🌐 traducido · ver original"));
  if (foreign && trWanted()) return h("div", { class: "chat-body" }, m.body, h("small", { class: "chat-tr", onclick: () => { delete CHAT.orig[m.id]; chatDraw(); } }, " 🌐 ver traducción"));
  return h("div", { class: "chat-body" }, linkify(m.body));
}
// pide las traducciones que faltan (una tanda por vez); lo que ya está en el idioma de quien lee queda marcado con «=»
async function chatTrNeed() {
  if (!trWanted() || CHAT.trBusy || !S.team) return;
  const ids = CHAT.msgs.filter((m) => m.body && !CALL_RE.test(m.body) && m.from_user !== S.user.id && CHAT.tr[tr.lang + ":" + m.id] === undefined).map((m) => m.id).slice(-30);
  if (!ids.length) return;
  CHAT.trBusy = true;
  for (const id of ids) CHAT.tr[tr.lang + ":" + id] = null;
  try {
    const r = await api("POST", `/api/teams/${S.team.id}/chat/translate`, { ids, lang: tr.lang });
    for (const id of ids) { const v = r.tr?.[id]; if (typeof v === "string") CHAT.tr[tr.lang + ":" + id] = v; else delete CHAT.tr[tr.lang + ":" + id]; }
    if (r.error) { CHAT.trCfg = { ...CHAT.trCfg, enabled: false }; toast("Traducción del chat: " + r.error, true); }
  } catch (e) { for (const id of ids) delete CHAT.tr[tr.lang + ":" + id]; CHAT.trCfg = { ...CHAT.trCfg, enabled: false }; toast("Traducción del chat: " + e.message, true); }
  CHAT.trBusy = false; chatDraw();
}
const chatTrModal = guard(async () => {
  const c = await api("GET", `/api/teams/${S.team.id}/chat/translate-cfg`);
  const on = h("input", { type: "checkbox", checked: c.enabled }), eng = h("select", {}, h("option", { value: "libre", selected: c.engine === "libre" }, "LibreTranslate propio (gratis, corre en tu red)"), h("option", { value: "ai", selected: c.engine === "ai" }, "Una IA del equipo (traduce mejor, gasta tokens)"));
  const bot = h("select", {}, h("option", { value: "" }, "— elegí una IA —"), c.bots.map((b) => h("option", { value: b.id, selected: c.bot_id === b.id }, "🤖 " + b.name)));
  const url = h("input", { value: c.url, placeholder: "http://192.168.1.60:5000", maxLength: 200 });
  const box = h("div", { class: "overlay", id: "chattr-modal", onmousedown: (e) => e.target === box && box.remove() });
  const draw = () => { botRow.hidden = eng.value !== "ai"; urlRow.hidden = eng.value !== "libre"; };
  const botRow = h("label", {}, "IA que traduce", bot, h("small", { class: "hint" }, "Usa su proveedor y su tope diario de gasto (Configuración → IAs → 💬 Chat). Conviene una IA barata, tipo «haiku».")),
    inst = h("div", { class: "hint" }), instBtn = h("button", { onclick: () => instalar() }, "⚙️ Instalar y activar automáticamente"),
    urlRow = h("label", {}, "Dirección del LibreTranslate", url, h("small", { class: "hint" }, "Nada sale de tu red."), h("div", { class: "row" }, instBtn), inst); eng.onchange = draw;
  let poll = null;
  const estado = async () => {
    try {
      const r = await api("GET", `/api/teams/${S.team.id}/chat/translate-install`);
      if (!r.docker) { inst.textContent = "Docker no está instalado en el servidor: instalalo (por ejemplo «sudo apt install docker.io») y volvé a tocar el botón. " + (r.docker_error || ""); instBtn.disabled = false; return; }
      if (r.log) { inst.textContent = "No se pudo instalar: " + r.log; instBtn.disabled = false; return; }
      if (r.ready) { url.value = r.url; on.checked = true; inst.textContent = "✅ LibreTranslate funcionando en este servidor. Tocá Guardar."; instBtn.disabled = false; return; }
      inst.textContent = r.state ? "⏳ Arrancando y cargando idiomas (tarda unos minutos la primera vez)…" : "⏳ Descargando la imagen (unos minutos, solo la primera vez)…";
      poll = setTimeout(estado, 4000);
    } catch (e) { inst.textContent = e.message; instBtn.disabled = false; }
  };
  const instalar = async () => {
    instBtn.disabled = true; inst.textContent = "Iniciando…";
    try { await api("POST", `/api/teams/${S.team.id}/chat/translate-install`, {}); estado(); } catch (e) { inst.textContent = e.message; instBtn.disabled = false; }
  };
  new MutationObserver((_, o) => { if (!document.body.contains(box)) { clearTimeout(poll); o.disconnect(); } }).observe(document.body, { childList: true });
  const save = guard(async () => {
    const r = await api("PUT", `/api/teams/${S.team.id}/chat/translate-cfg`, { enabled: on.checked, engine: eng.value, bot_id: bot.value ? +bot.value : null, url: url.value });
    clearTimeout(poll); CHAT.trCfg = r; CHAT.tr = {}; toast(r.enabled ? "Traducción del chat activada" : "Traducción del chat apagada"); box.remove(); chatDraw(); chatTrNeed();
  });
  box.append(h("div", { class: "modal small" }, h("h3", {}, "🌐 Traducir el chat"),
    h("p", { class: "hint" }, "Cada persona escribe en su idioma y lee en el suyo (español, inglés o portugués, el que eligió en Configuración). El mensaje original no se modifica y se puede ver con un toque."),
    h("label", { class: "sub" }, on, "Activar la traducción en este equipo"), h("label", {}, "Motor", eng), botRow, urlRow,
    h("p", { class: "warn" }, "Ojo con la privacidad: al traducir, el texto de los mensajes se envía al motor elegido. Con una IA de internet sale de tu servidor; con LibreTranslate propio, no."),
    h("div", { class: "row" }, h("button", { onclick: () => box.remove() }, "Cancelar"), h("button", { class: "primary", onclick: save }, "Guardar"))));
  document.body.append(box); draw();
});

// ---------- operación del servidor: procesos y roles (solo CTO / administrador) ----------
const ROLE_LBL = { user: "Usuario", cto: "CTO", admin: "Administrador" };
function procPanel() {
  const box = h("div", { class: "cfg-item proc" }), bar = (pct) => h("div", { class: "proc-bar" }, h("i", { style: `width:${Math.min(100, pct)}%` }));
  box.dataset.key = "procesos";
  const gb = (mb) => (mb >= 1024 ? (mb / 1024).toFixed(1) + " GB" : mb + " MB");
  const draw = async () => {
    if (!document.body.contains(box) && box.dataset.drawn) return;
    box.dataset.drawn = "1";
    try {
      const r = await api("GET", "/api/admin/procesos");
      const memP = r.mem_mb.total ? (100 * r.mem_mb.used) / r.mem_mb.total : 0, dskP = r.disk_mb.total ? (100 * r.disk_mb.used) / r.disk_mb.total : 0;
      box.replaceChildren(
        h("div", { class: "proc-grid" },
          h("div", {}, h("b", {}, "RAM"), bar(memP), h("small", {}, `${gb(r.mem_mb.used)} de ${gb(r.mem_mb.total)}`)),
          h("div", {}, h("b", {}, "Disco"), bar(dskP), h("small", { style: dskP > 90 ? "color:var(--red)" : "" }, r.disk_mb.total ? `${gb(r.disk_mb.used)} de ${gb(r.disk_mb.total)} (${Math.round(dskP)}%) · ${r.disk_mb.dir}` : "no se pudo leer el disco de " + r.disk_mb.dir)),
          h("div", {}, h("b", {}, "Carga"), bar((100 * (r.load[0] || 0)) / r.cpus), h("small", {}, `${(r.load || []).join(" · ")} (${r.cpus} núcleos)`)),
          h("div", {}, h("b", {}, "Encendido"), h("small", {}, `${Math.floor(r.uptime_s / 86400)} d ${Math.floor((r.uptime_s % 86400) / 3600)} h`))),
        h("h5", {}, "Procesos de CrewHelm"),
        h("table", { class: "proc-t" }, h("tr", {}, ["Proceso", "PID", "CPU %", "RAM"].map((x) => h("th", {}, x))),
          r.procs.map((p) => h("tr", {}, h("td", {}, p.name + (p.me ? " (servidor)" : "")), h("td", {}, p.pid), h("td", {}, p.cpu), h("td", {}, p.rss_mb + " MB")))),
        r.docker.length > 0 && [h("h5", {}, "Contenedores"), h("ul", { class: "wiz-list" }, r.docker.map((d) => h("li", {}, `${d.name} · ${d.status} · ${d.image}`)))],
        h("h5", {}, "Uso de la app"),
        h("p", { class: "hint" }, `Base de datos ${gb(r.app.db_mb)} · archivos del chat ${gb(r.app.chat_files_mb)} · ${r.app.users} personas · ${r.app.ais} IAs · ${r.app.teams} equipos · ${r.app.tasks} tareas (${r.app.tasks_doing} en curso)`),
        h("small", { class: "hint" }, "Se actualiza cada 5 segundos mientras esta ventana está abierta."));
    } catch (e) { box.replaceChildren(h("p", { class: "warn" }, e.message)); }
    setTimeout(draw, 5000);
  };
  box.append(h("small", { class: "hint" }, "Cargando…")); setTimeout(draw, 0);
  return box;
}
function rolesPanel() {
  const box = h("div", { class: "cfg-item" });
  box.dataset.key = "srvroles";
  const load = async () => {
    const us = await api("GET", "/api/admin/users");
    box.replaceChildren(h("p", { class: "hint" }, "Usuario: usa la app. CTO: además ve «Procesos». Administrador: todo, incluido cambiar estos roles. No cambia los roles dentro de cada equipo."),
      us.map((u) => h("div", { class: "cfg-line" }, h("span", { class: "grow" }, u.username),
        h("select", { onchange: async (e) => { try { await api("PUT", `/api/admin/users/${u.id}/role`, { role: e.target.value }); toast("Rol actualizado"); } catch (x) { toast(x.message, true); } load(); } },
          Object.entries(ROLE_LBL).map(([k, v]) => h("option", { value: k, selected: u.role === k }, v))))));
  };
  load().catch((e) => box.replaceChildren(h("p", { class: "warn" }, e.message)));
  return box;
}

// límites de las terminales de las personas (los aplica el agente de cada máquina)
function limitsPanel() {
  const box = h("div", { class: "cfg-item" });
  box.dataset.key = "srvlimits";
  const F = [
    ["disk_mb", "💾 Disco por persona (MB)", "Cuánto puede ocupar el espacio personal de cada miembro. Lleno: no abre la terminal; al pasar el 125 % se cierra.  ·  0 = sin límite"],
    ["mem_mb", "🧠 Memoria por persona (MB)", "Tope de memoria RAM de todo lo que corre en la terminal de una persona (sin swap). Si lo pasa, el sistema corta el programa que más usa.  ·  0 = sin límite"],
    ["cpu_pct", "⚙ CPU por persona (%)", "100 % es un núcleo entero. 200 % = hasta dos núcleos a la vez, aunque el equipo tenga más.  ·  0 = sin límite"],
    ["tasks", "🧵 Procesos por persona", "Cantidad máxima de procesos e hilos a la vez (frena los que se multiplican sin control).  ·  0 = sin límite"],
  ];
  const load = async () => {
    const v = await api("GET", "/api/admin/limits");
    const ins = Object.fromEntries(F.map(([k]) => [k, h("input", { type: "number", min: 0, value: v[k], style: "width:110px" })]));
    const save = guard(async () => {
      const body = {}; for (const [k] of F) body[k] = Math.round(+ins[k].value);
      await api("PUT", "/api/admin/limits", body); toast("Límites guardados: se aplican a las terminales que se abran desde ahora"); load();
    });
    box.replaceChildren(h("p", { class: "hint" }, "Límites de las terminales de los miembros (los owners no tienen límite). Los aplica el agente de cada máquina; las terminales ya abiertas siguen con los anteriores hasta cerrarse."),
      ...F.map(([k, l, t]) => h("div", { class: "cfg-line", title: t }, h("span", { class: "grow" }, l), ins[k])),
      h("div", { class: "row" }, h("button", { class: "primary", onclick: save }, "Guardar límites")));
  };
  load().catch((e) => box.replaceChildren(h("p", { class: "warn" }, e.message)));
  return box;
}

// Comunicación: una sola entrada con dos solapas, chat y videollamada
function commDraw() {
  if (!CHAT.el) return;
  const it = callItem(), tab = CHAT.tab === "video" && it ? "video" : "chat";
  CHAT.tab = tab;
  CHAT.tabs.replaceChildren(
    h("button", { class: "comm-tab" + (tab === "chat" ? " on" : ""), onclick: () => { CHAT.tab = "chat"; commDraw(); } }, "💬 Chat" + (CHAT.total > 0 ? ` (${CHAT.total})` : "")),
    h("button", { class: "comm-tab" + (tab === "video" ? " on" : ""), onclick: () => { CHAT.tab = "video"; commDraw(); } }, "📹 Videollamada"));
  CHAT.pane.hidden = tab !== "chat"; CHAT.video.hidden = tab !== "video";
  if (tab !== "video") return;
  const c = CHAT.chan, where = c.t === "dm" ? "este mensaje directo" : c.t === "project" ? "este proyecto" : "todo el equipo";
  CHAT.video.replaceChildren(
    h("div", { class: "comm-card" }, h("b", {}, "Sala permanente del equipo"), h("small", { class: "hint" }, "Siempre la misma: entra quien del equipo la abra. Sirve para reuniones habituales."),
      h("button", { class: "primary", onclick: () => openInteg(it) }, "▶ Entrar a la sala")),
    h("div", { class: "comm-card" }, h("b", {}, "Llamar ahora"), h("small", { class: "hint" }, `Crea una sala nueva y avisa en el chat de ${where} con un botón «Unirme». Solo la ven quienes participan de esa conversación.`),
      h("button", { onclick: chatCall }, "📞 Iniciar llamada")),
    can("member") && h("div", { class: "comm-card" }, h("b", {}, "Invitar a alguien de afuera"), h("small", { class: "hint" }, "Proveedor o cliente, sin cuenta: enlace único con vencimiento, para copiar o mandar por email."),
      h("button", { onclick: inviteModal }, "🔗 Crear invitación")));
}
function chatDraw() {
  if (!CHAT.el) return;
  commDraw();
  const me = S.user.id, kb = (n) => (n < 1024 ? n + " B" : (n / 1024).toFixed(n < 10240 ? 1 : 0) + " KB");
  // un solo combo: equipo, proyectos y personas (conectadas y desconectadas); a quien no está se le deja el mensaje y lo ve al conectarse
  const others = CHAT.people.filter((x) => !x.me), n = (k) => (CHAT.unread[k] ? ` (${CHAT.unread[k]})` : "");
  const person = (x) => h("option", { value: "u" + x.id, selected: CHAT.chan.t === "dm" && CHAT.chan.id === x.id }, (AV[x.username] || (x.online ? "🟢" : "⚪")) + " " + x.username + n("dm" + x.id));
  CHAT.sel.replaceChildren(
    h("optgroup", { label: "Conversaciones" }, h("option", { value: "team", selected: CHAT.chan.t === "team" }, "👥 Todo el equipo" + n("team")),
      ...S.projects.map((p) => h("option", { value: "p" + p.id, selected: CHAT.chan.t === "project" && CHAT.chan.id === p.id }, "📁 " + p.name + n("p" + p.id)))),
    others.some((x) => x.online) && h("optgroup", { label: "🟢 Conectados · mensaje directo" }, others.filter((x) => x.online).map(person)),
    others.some((x) => !x.online) && h("optgroup", { label: "⚪ Desconectados · lo ven al conectarse" }, others.filter((x) => !x.online).map(person)),
    (CHAT.bots || []).length > 0 && h("optgroup", { label: "🤖 IAs · respuestas cortas" }, CHAT.bots.map((x) => h("option", { value: "u" + x.id, selected: CHAT.chan.t === "dm" && CHAT.chan.id === x.id }, "🤖 " + x.username + n("dm" + x.id)))));
  const peer = CHAT.chan.t === "dm" ? others.find((x) => x.id === CHAT.chan.id) : null;
  const aiPeer = CHAT.chan.t === "dm" ? (CHAT.bots || []).find((x) => x.id === CHAT.chan.id) : null;
  CHAT.who.replaceChildren(aiPeer ? h("div", { class: "chat-ai-note" }, h("b", {}, `🤖 ${aiPeer.username} es una IA.`),
    ` Charla corta: solo recuerda los últimos ${aiPeer.ctx} mensajes de esta conversación, contesta en pocas líneas, no ve tus tareas, archivos ni el código del proyecto, y tiene un tope de uso por día (hoy le quedan ≈ ${aiPeer.left_usd.toFixed(2)} USD). Sirve para consultas rápidas; para trabajo de verdad, asignale una tarea.`)
    : h("small", { class: "hint" }, peer && !peer.online
    ? `⚪ ${peer.username} está desconectado: el mensaje y los archivos le quedan esperando y los ve al conectarse.`
    : `${CHAT.people.filter((x) => x.online).length} conectados · elegí a alguien en la lista para escribirle en privado` + ((CHAT.bots || []).length ? " · mencioná a una IA con @nombre" : "")));
  CHAT.err_.replaceChildren(CHAT.err, ...(CHAT.err && CHAT.https ? [h("div", {}, h("a", { href: CHAT.https + location.pathname, class: "btnlike" }, "🔒 Abrir con conexión segura (HTTPS)"))] : []));
  CHAT.err_.hidden = !CHAT.err;
  CHAT.list.replaceChildren(...CHAT.msgs.map((m) => h("div", { class: "chat-msg" + (m.from_user === me ? " mine" : "") },
    h("div", {}, h("b", {}, (botInfo(m.from) ? "🤖 " : AV[m.from] ? AV[m.from] + " " : "") + m.from), " ", h("small", {}, new Date(m.created_at).toLocaleString([], { day: "2-digit", month: "2-digit", hour: "2-digit", minute: "2-digit" }))),
    m.body && chatBody(m),
    m.file && h("button", { class: "ghost chat-file", title: "Descargar", onclick: guard(() => saveFile(`/api/chat/files/${m.file.id}`, m.file.name)) }, `📎 ${m.file.name} (${kb(m.file.size)})`))));
  CHAT.list.scrollTop = CHAT.list.scrollHeight;
  chatTrNeed();
  CHAT.att.replaceChildren(...(CHAT.pending ? [h("span", {}, "📎 " + CHAT.pending.name + " "), h("button", { class: "ghost", onclick: () => { CHAT.pending = null; chatDraw(); } }, "✕")] : []));
}
function chatPanel() {
  if (!CHAT.el) {
    CHAT.sel = h("select", { onchange: (e) => { const v = e.target.value; if (v === "team") chatSwitch({ t: "team" }); else if (v[0] === "p") chatSwitch({ t: "project", id: +v.slice(1) }); else if (v[0] === "u") chatSwitch({ t: "dm", id: +v.slice(1) }); } });
    const EMO = [["👍", "Bien"], ["👌", "Ok"], ["👋", "Hola"], ["🎉", "Felicitaciones"], ["👏", "Aplausos"], ["🙏", "Gracias"], ["😊", "Sonrisa"], ["❤️", "Me encanta"]];
    CHAT.emo = h("div", { class: "chat-emo" }, EMO.map(([e, t]) => h("button", { class: "ghost", title: t, type: "button", onclick: () => {
      const i = CHAT.inp, a = i.selectionStart ?? i.value.length, z = i.selectionEnd ?? a;
      i.value = i.value.slice(0, a) + e + i.value.slice(z); i.focus(); i.selectionStart = i.selectionEnd = a + e.length;
    } }, e)));
    CHAT.who = h("div", { class: "chat-who" }); CHAT.list = h("div", { class: "chat-list" });
    CHAT.err_ = h("div", { class: "chat-err", hidden: true }); CHAT.att = h("div", { class: "chat-att" });
    CHAT.inp = h("textarea", { class: "chat-in", rows: 3, maxLength: 4000, placeholder: "Escribí un mensaje… (Enter envía · Shift+Enter, línea nueva)" });
    CHAT.inp.addEventListener("keydown", (e) => { if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); chatSend(); } });
    const pick = h("input", { type: "file", hidden: true, onchange: chatPick });
    CHAT.el = h("section", { class: "chat-panel" },
      h("div", { class: "row" }, h("strong", {}, "💬 Comunicación"), h("span", { class: "spacer" }),
        CHAT.trCfg?.enabled && h("button", { class: "ghost" + (trWanted() ? " on" : ""), title: trWanted() ? "Traducción automática activada: tocá para ver todo en su idioma original" : "Traducción automática desactivada: tocá para activarla", onclick: () => { store.set("chatTr", trWanted() ? "0" : "1"); chatDraw(); } }, "🌐"),
        can("admin") && h("button", { class: "ghost", title: "Traducción automática del chat (configurar)", onclick: chatTrModal }, "⚙🌐"),
        can("admin") && h("button", { class: "ghost", title: "Registro de todo lo que pasó en el chat", onclick: chatAudit }, "🧾"),
        can("member") && showTech() && S.project && h("button", { class: "ghost", title: "Abrir una terminal del proyecto, sin crear tarea", onclick: guard(async () => { S.tvProject = S.project.id; await openTerms(); if (!PANES.some((p) => p.pid === S.tvProject && !p.tid)) await addPane(S.tvProject, null, "💻 " + S.project.name); }) }, "💻"),
        h("button", { class: "ghost", title: "Cerrar el chat", onclick: toggleChat }, "✕")),
      CHAT.tabs = h("div", { class: "comm-tabs" }), CHAT.video = h("div", { class: "comm-video", hidden: true }),
      CHAT.pane = h("div", { class: "chat-pane" },
      h("small", { class: "hint" }, "🔒 Solo por conexión segura; mensajes y archivos se guardan cifrados."),
      CHAT.err_, CHAT.sel, CHAT.who, CHAT.list, CHAT.att,
      CHAT.emo, CHAT.inp, h("div", { class: "row" }, h("button", { class: "ghost", title: "Adjuntar un archivo (imágenes, PDF, texto, documentos; hasta 5 MB)", onclick: () => pick.click() }, "📎 Adjuntar"), pick, h("span", { class: "spacer" }), h("button", { class: "primary", onclick: chatSend }, "Enviar"))));
  }
  if (S.team && CHAT.teamId !== S.team.id) { CHAT.teamId = S.team.id; CHAT.chan = { t: "team" }; CHAT.msgs = []; CHAT.people = []; chatHistory(); chatPresence(); }
  else chatDraw();
  return CHAT.el;
}

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
          if (!(await ask(m.id === S.user.id ? "¿Salir del equipo?" : `¿Quitar a ${m.username}?`))) return;
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
        !owner && h("button", { class: "danger", onclick: guard(async () => { if (!(await ask("¿Salir del equipo?"))) return; await api("DELETE", `/api/teams/${o.id}/members/${S.user.id}`); await boot(); }) }, "Salir del equipo"),
        owner && h("button", { onclick: transfer }, "Transferir la propiedad"),
        owner && h("button", { class: "danger", onclick: remove }, "Eliminar el equipo"),
        !admin && h("small", {}, "Solo un admin o el dueño puede cambiar los datos del equipo y gestionar miembros."))));
  return box;
}
const RELEASES_URL = "https://github.com/polblancoo/crewhelm/releases/latest";
// dirección que la otra persona puede alcanzar: si el admin entra por localhost, se usa la IP de la red local
async function reachableServer() {
  if (!/localhost|127\.0\.0\.1/.test(S.server)) return S.server;
  try { const r = await api("GET", "/api/lan"); if (r.ip) return `http://${r.ip}:${new URL(S.server).port || 7878}`; } catch {}
  return S.server;
}
const inviteMessage = (code, srv = S.server) => [
  `Te invito al equipo «${S.team.name}» en ${APP_NAME}.`,
  "",
  "LO MÁS FÁCIL (sin instalar nada):",
  `1. Abrí en el navegador: ${srv}`,
  "2. Creá tu cuenta y tocá «Unirme con un código» (el ↪ de la barra lateral).",
  `3. Pegá este código: ${code}`,
  "",
  "Si preferís la app de escritorio:",
  `- Windows / Mac / Linux: ${RELEASES_URL}`,
  `- Linux, en una línea: curl -fsSL https://raw.githubusercontent.com/polblancoo/crewhelm/main/install.sh | sh -s -- --client --server ${srv}`,
  `  (al abrirla, en «Servidor» poné ${srv})`,
  "",
  "El código vence en 7 días.",
].join("\n");

// ayudas de cada campo de tarea (se ven al pasar el mouse)
const FIELD_TIPS = {
  title: "Resumen corto: es lo que se ve en la tarjeta del tablero.",
  desc: "Lo que tiene que hacer quien la tome. Si el responsable es una IA, este texto es su prompt: sé claro y poné el contexto necesario.",
  status: "Dónde está la tarea: Por hacer, En curso o Hecho. Una IA la pasa a En curso cuando empieza.",
  prio: "Qué tan urgente es. Ordena las tarjetas y se muestra como etiqueta de color.",
  who: "Quién la hace, persona o IA. Si es una IA con su agente conectado, la toma sola.",
  start: "Cuándo debería empezar. Se ve en el Gantt y el calendario.",
  due: "Fecha límite: se avisa cuando se acerca y se marca atrasada si pasa.",
  rec: "Si se repite, al completarla se crea la siguiente automáticamente.",
  review: "Quién mira el trabajo antes de darla por terminada: una persona o una IA. Puede ser quien la pidió (vos). Esa persona aprueba o la devuelve con el motivo.",
};
// diálogo de "Nueva tarea": ancho de un tercio, prompt grande, atributos en dos filas, atajos y avisos al pie
function taskForm(status, assignable) {
  return new Promise((resolve) => {
    const title = h("input", { type: "text", maxLength: 120, placeholder: "Título corto" });
    const desc = h("textarea", { rows: 8, maxLength: 20000, placeholder: "Qué hay que hacer, con el contexto necesario. Si la hace una IA, esto es lo que va a leer." });
    const grow = () => { desc.style.height = "auto"; desc.style.height = Math.min(desc.scrollHeight + 2, window.innerHeight * 0.42) + "px"; };
    desc.oninput = grow;
    const sel = (opts, value) => h("select", {}, opts.map(([v, l]) => h("option", { value: v, selected: String(v) === String(value) }, l)));
    const st = sel(STATUS.map((x) => [x.id, x.label]), status), pr = sel(PRIO.map((x, i) => [i, x]), 1);
    const asOpts = [["", "Sin asignar"], ...assignable.map((m) => [m.id, m.id === S.user?.id && !can("admin") ? `Yo (${m.username})` : label(m.username)])];
    const as = sel(asOpts, ""), start = h("input", { type: "date" }), due = h("input", { type: "date" }), rec = sel(RECUR, "");
    // responsable final: revisa el trabajo antes de dar la tarea por hecha (arma un flujo hacer → revisar)
    const meId = S.user?.id;
    const rvOpts = [["", "Nadie: al terminar pasa a Hecha"], ...(assignable.some((m) => m.id === meId) ? [] : [[meId, "Yo (quien la pide)"]]), ...assignable.map((m) => [m.id, m.id === meId ? "Yo (quien la pide)" : label(m.username)])];
    const rv = sel(rvOpts, ""), more = h("input", { type: "checkbox" });
    const note = h("div", { class: "tf-note" });
    const notes = () => {
      const m = S.members.find((x) => String(x.id) === as.value), out = [];
      if (m?.bot) out.push([S.project.work_dir ? "🤖 " + m.username + " trabaja en 📁 " + S.project.work_dir : "🤖 El proyecto no tiene carpeta de trabajo: la IA usa una por defecto y la crea si no existe", !S.project.work_dir]);
      if (!as.value) out.push(["Sin responsable: queda en el tablero sin dueño", false]);
      if (rv.value && !as.value) out.push(["Para el flujo elegí quién la hace: esa persona o IA trabaja y el revisor aprueba", true]);
      if (rv.value && as.value && rv.value === as.value) out.push(["Quien la hace y quien la revisa es la misma persona", true]);
      if (start.value && due.value && due.value < start.value) out.push(["⚠️ Vence antes de empezar", true]);
      if (rec.value) out.push(["🔁 Se repite al completarse", false]);
      note.replaceChildren(...out.map(([t, w]) => h("span", { class: w ? "warn" : "" }, t)));
    };
    [as, start, due, rec, rv].forEach((e) => e.addEventListener("change", notes)); notes();
    const done = (v) => { overlay.remove(); resolve(v); };
    const submit = () => {
      if (!title.value.trim()) { title.focus(); return; }
      if (rv.value && !as.value) { as.focus(); return; }
      done({ reviewer: rv.value, more: more.checked, title: title.value.trim(), description: desc.value.trim(), status: st.value, priority: pr.value, assignee: as.value, start: start.value, due: due.value, recurrence: rec.value });
    };
    const fld = (lbl, key, el, tip) => h("label", { title: tip }, h("span", {}, lbl, key && h("kbd", {}, key)), el);
    const jump = { e: st, p: pr, r: as, f: start, v: due, t: title, d: desc };
    const overlay = h("div", { class: "overlay", onmousedown: (e) => e.target === overlay && done(null) },
      h("div", { class: "modal task-form", onkeydown: (e) => {
        if (e.key === "Escape") { e.stopPropagation(); done(null); }
        else if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) { e.preventDefault(); submit(); }
        else if (e.key === "Enter" && e.target === title) { e.preventDefault(); desc.focus(); }
        else if (e.altKey && jump[e.key.toLowerCase()]) { e.preventDefault(); jump[e.key.toLowerCase()].focus(); }
      } },
        h("h3", {}, "Nueva tarea"),
        fld("Título", "Alt+T", title, FIELD_TIPS.title),
        fld("Instrucción / prompt para quien la haga", "Alt+D", desc, FIELD_TIPS.desc),
        h("div", { class: "tf-row" }, fld("Estado", "Alt+E", st, FIELD_TIPS.status), fld("Prioridad", "Alt+P", pr, FIELD_TIPS.prio), fld("Responsable", "Alt+R", as, FIELD_TIPS.who)),
        h("div", { class: "tf-row" }, fld("Empieza", "Alt+F", start, FIELD_TIPS.start), fld("Vence", "Alt+V", due, FIELD_TIPS.due), fld("Repetición", "", rec, FIELD_TIPS.rec)),
        h("div", { class: "tf-row tf-flow" }, fld("Revisa al terminar (responsable final)", "", rv, FIELD_TIPS.review),
          h("label", { class: "inl tf-more", title: "Después de crear, se abre la tarea para sumar etapas: varios que trabajan, más revisores, instrucciones por etapa" }, more, " Armar más etapas después")),
        h("div", { class: "tf-help" }, h("kbd", {}, "Ctrl+Enter"), " crear · ", h("kbd", {}, "Esc"), " cancelar · ", h("kbd", {}, "Enter"), " en el título pasa al prompt · ", h("kbd", {}, "Alt+letra"), " salta al campo"),
        note,
        h("div", { class: "row" }, h("button", { onclick: () => done(null) }, "Cancelar"), h("button", { class: "primary", onclick: submit }, "Crear tarea"))));
    document.body.append(overlay); title.focus();
  });
}
const createTask = guard(async (status) => {
  // mismas reglas que en el modal: un líder asigna a cualquiera; un miembro solo a sí mismo
  const assignable = can("admin") ? S.members.filter((m) => RANK[m.role] >= 1) : S.members.filter((m) => m.id === S.user?.id && can("member"));
  const f = await taskForm(status, assignable); if (!f) return;
  const day = (d) => (d ? new Date(d + "T12:00:00Z").toISOString() : undefined);
  let t = await api("POST", `/api/projects/${S.project.id}/tasks`, { title: f.title, description: f.description, priority: +f.priority, assignee_id: f.assignee ? +f.assignee : undefined, start: day(f.start), due: day(f.due), recurrence: f.recurrence || undefined });
  if (f.reviewer) {
    // flujo hacer → revisar: la tarea pasa a quien la hace y, al terminar, a quien la revisa
    t = await api("PUT", `/api/tasks/${t.id}/flow`, { steps: [{ assignee_id: +f.assignee, kind: "work", instruction: "" }, { assignee_id: +f.reviewer, kind: "review", instruction: "" }] });
  } else if (f.status !== "todo") t = await api("PATCH", `/api/tasks/${t.id}`, { status: f.status });
  upsert(t); render();
  if (f.more && f.reviewer) { loadFlows?.(); openTask(t); }
});
const patchTask = guard(async (id, patch) => { upsert(await api("PATCH", `/api/tasks/${id}`, patch)); render(); });

function upsert(t) {
  const i = S.tasks.findIndex((x) => x.id === t.id);
  i >= 0 ? (S.tasks[i] = t) : S.tasks.push(t);
}

// ---------- modal de tarea ----------
// tareas hechas archivadas: se ven acá y se pueden devolver al tablero
function openArchived() {
  closeModal(); S.open = "archived";
  const list = S.tasks.filter((t) => t.archived && !t.parent_id).sort((a, b) => String(b.updated_at).localeCompare(String(a.updated_at)));
  document.body.append(h("div", { class: "overlay", id: "modal", onmousedown: (e) => e.target.id === "modal" && closeModal() },
    h("div", { class: "modal" },
      h("h3", {}, "🗄 Tareas archivadas", h("span", { class: "spacer" }), h("button", { class: "ghost", onclick: closeModal }, "✕")),
      h("p", { class: "hint" }, "Tareas hechas que sacaste del tablero. Seguís viéndolas acá, en el Gantt y en los informes; «Volver al tablero» las deja otra vez en Hecho."),
      list.length ? h("div", { class: "arch-list" }, list.map((t) => h("div", { class: "arch-row" },
        h("span", { class: "tid" }, "#" + t.id),
        h("a", { class: "grow arch-title", href: "#", onclick: (e) => { e.preventDefault(); closeModal(); openTask(t); } }, t.title),
        h("small", { class: "hint" }, new Date(t.updated_at).toLocaleDateString()),
        can("member") && h("button", { onclick: () => setArchived(t, false) }, "Volver al tablero"))))
        : h("p", { class: "hint" }, "No hay tareas archivadas. Usá la ✕ de una tarea en Hecho para archivarla."))));
}
// dirección web de un repositorio: «dueño/repo», URL de GitHub o git@github.com:dueño/repo.git
function repoUrl(repo) {
  const r = String(repo || "").trim().replace(/\.git$/, "").replace(/\/$/, "");
  if (/^https:\/\//.test(r)) return r;
  const ssh = r.match(/^git@([\w.-]+):(.+)$/);
  if (ssh) return `https://${ssh[1]}/${ssh[2]}`;
  return /^[\w.-]+\/[\w.-]+$/.test(r) ? "https://github.com/" + r : null;
}
// terminal acoplada debajo de la charla de una tarea (una a la vez)
let TDOCK = null;
function closeDock() { if (!TDOCK) return; try { TDOCK.ws?.close(); } catch {} TDOCK.ro?.disconnect(); TDOCK.term?.dispose(); TDOCK = null; }
function closeModal() { closeDock(); S.open = null; S.redrawSubs = null; document.getElementById("modal")?.remove(); }

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

// cuerpo de un comentario: los bloques ``` ``` se muestran enmarcados con botón para copiar; el resto como texto
function richBody(body) {
  const out = []; const re = /```[^\n`]*\n?([\s\S]*?)```/g; let last = 0, m;
  const copy = (txt) => navigator.clipboard?.writeText(txt).then(() => toast("Copiado"));
  while ((m = re.exec(body))) {
    if (m.index > last) out.push(...withMentions(body.slice(last, m.index).replace(/^\n+|\n+$/g, "")));
    const code = m[1].replace(/\n+$/, "");
    out.push(h("div", { class: "codeblock" }, h("pre", {}, h("code", {}, code)), h("button", { class: "ghost", title: "Copiar", onclick: () => copy(code) }, "📋 Copiar")));
    last = m.index + m[0].length;
  }
  if (last < body.length) out.push(...withMentions(body.slice(last).replace(/^\n+/, "")));
  return out;
}

function renderComments() {
  const box = document.getElementById("comments"); if (!box) return;
  box.replaceChildren(...S.comments.map((c) => h("div", { class: "comment" + (c.user_id === S.user?.id ? " mine" : "") },
    h("b", {}, c.username), h("small", {}, new Date(c.created_at).toLocaleString()), h("div", {}, richBody(c.body)),
    (c.body.includes("```") ? [] : commandsOf(c.body)).map((cmd) => h("div", { class: "cmdbox" }, h("code", {}, cmd),
      h("button", { class: "ghost", title: "Copiar el comando", onclick: () => navigator.clipboard?.writeText(cmd).then(() => toast("Comando copiado")) }, "📋 Copiar"))))));
  // lo último queda a la vista
  box.lastElementChild?.scrollIntoView({ block: "nearest" });
}

const openTask = guard(async (task) => {
  S.open = task.id;
  S.comments = await api("GET", `/api/tasks/${task.id}/comments`);
  const cur = () => S.tasks.find((t) => t.id === task.id) || task;
  const title = h("input", { value: cur().title, maxLength: 200 });
  const desc = h("textarea", { rows: 3, maxLength: 10000, value: cur().description });
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
  const comment = h("textarea", { class: "comment-in", rows: 2, placeholder: "Escribí un mensaje… (Enter = línea nueva · Ctrl+Enter = enviar)", maxLength: 4000, oninput: (e) => { e.target.style.height = "auto"; e.target.style.height = Math.min(e.target.scrollHeight, 320) + "px"; } });

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
  // si la tarea viene de un issue de GitHub: enlace y botón para cerrarlo allá
  const ghBox = h("div", { class: "row" });
  const PR_LABEL = { open: "abierto", draft: "borrador", merged: "integrado ✅", closed: "cerrado sin integrar" };
  api("GET", `/api/tasks/${task.id}/github`).then((g) => {
    if (g.repo) {
      ghBox.append(h("a", { href: g.url, target: "_blank", rel: "noopener" }, `🐙 ${g.repo}#${g.number}`),
        g.pr && h("span", { class: "pill", title: "Pull Request que menciona este issue" }, `PR #${g.pr}: ${PR_LABEL[g.pr_state] || g.pr_state}`));
    } else if (!g.project_repo) return;
    ghBox.append(h("span", { class: "spacer" }));
    if (!can("member")) return;
    if (!g.repo) ghBox.append(h("button", { onclick: guard(async () => {
      const r = await api("POST", `/api/tasks/${task.id}/github/publish`); toast(`Publicada como issue #${r.number}`); closeModal(); openTask(await api("GET", `/api/tasks/${task.id}`));
    }) }, "🐙 Publicar como issue"));
    ghBox.append(h("button", { title: "Crea el Pull Request de la rama tm/tarea-" + task.id + " (o la que indiques)", onclick: guard(async () => {
      const f = await form("Abrir Pull Request", [{ name: "branch", label: "Rama ya subida a GitHub", value: "tm/tarea-" + task.id, max: 120 }], "Abrir"); if (!f) return;
      const r = await api("POST", `/api/tasks/${task.id}/github/pr`, { branch: f.branch }); toast(`Pull Request #${r.number} abierto`); window.open(r.url, "_blank", "noopener");
    }) }, "Abrir Pull Request"));
    if (g.repo) ghBox.append(h("button", { onclick: guard(async () => {
      if (!(await ask("Se cierra el issue en GitHub (con un comentario). ¿Cerrarlo?", "Cerrar issue"))) return;
      await api("POST", `/api/tasks/${task.id}/github/close`); toast("Issue cerrado en GitHub");
    }) }, "Cerrar issue"));
  }).catch(() => {});
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
  // la descripción se lee como el primer mensaje; se edita solo si la tocás
  const descView = h("div", { class: "desc-msg" });
  const drawDesc = () => {
    const empty = !desc.value.trim();
    desc.hidden = !empty; descView.hidden = !empty ? false : !can("member");
    descView.replaceChildren(
      h("div", { class: "desc-text" }, empty ? h("span", { class: "hint" }, "Sin descripción") : richBody(desc.value)),
      can("member") && !empty && h("button", { class: "ghost", title: "Editar la descripción", onclick: () => { desc.hidden = false; descView.hidden = true; desc.focus(); } }, "✏ Editar"));
  };
  desc.addEventListener("blur", () => { if (desc.value.trim()) drawDesc(); });
  title.classList.add("task-title"); title.title = FIELD_TIPS.title; desc.title = FIELD_TIPS.desc;
  // terminal abajo de la charla: usa la conexión del proyecto y tus preferencias (herdr automático, etc.)
  const dockSlot = h("div", { class: "dock-slot" });
  const toggleDock = guard(async () => {
    if (TDOCK) { closeDock(); dockSlot.replaceChildren(); return; }
    // los miembros no ven la configuración de conexión (es de owners): la terminal les contesta si está habilitada o no
    const c0 = can("owner") ? await api("GET", `/api/projects/${task.project_id}/ssh`) : { agent_id: 1 };
    if (!c0.host && !c0.agent_id) { S.dockAfter = task.id; toast("Primero configurá la conexión a la máquina del proyecto; al cerrar volvés a la tarea"); return openTerminal(cur()); }
    await ensureXterm();
    const box = h("div", { class: "term-box" }), st = h("span", { class: "tv-st" }, "⏳");
    const pane = { pid: task.project_id, tid: task.id, box, st, started: false };
    pane.el = h("div", { class: "dock-term" }, h("div", { class: "tv-head" }, st, h("b", {}, "💻 Terminal · tarea #" + task.id), h("span", { class: "spacer" }),
      h("button", { class: "ghost", title: "Más alta o más baja", onclick: (e) => { const on = pane.el.classList.toggle("tall"); e.currentTarget.textContent = on ? "⤡" : "⤢"; setTimeout(() => { try { pane.fit?.fit(); } catch {} }, 80); } }, "⤢"),
      h("button", { class: "ghost", title: "Cerrar la terminal (guarda la nota .md)", onclick: () => { closeDock(); dockSlot.replaceChildren(); } }, "✕")), box, gitBar(pane));
    TDOCK = pane; dockSlot.replaceChildren(pane.el);
    requestAnimationFrame(() => { startPane(pane); pane.el.scrollIntoView({ block: "nearest" }); setTimeout(() => pane.term?.focus(), 200); });
  });
  const send = guard(async () => {
    if (!comment.value.trim()) return;
    const c = await api("POST", `/api/tasks/${task.id}/comments`, { body: comment.value });
    comment.value = ""; comment.style.height = "";
    if (!S.comments.some((x) => x.id === c.id)) S.comments.push(c);
    renderComments();
  });
  comment.addEventListener("keydown", (e) => e.key === "Enter" && (e.ctrlKey || e.metaKey) && send());

  // tarea de pedido de una IA («🙋 Pedido de #N: …»): comandos para copiar, dónde pegar la salida y botones de respuesta
  const requestPanel = () => {
    if (!/^🙋 Pedido de #\d+:/.test(cur().title) || !can("member")) return null;
    const cmds = (cur().description.match(/COMANDOS PARA EJECUTAR[^\n]*\n[^\n]*\n\n([\s\S]*?)\n\n/) || [])[1]?.split("\n").filter(Boolean) || [];
    const out = comment; // se escribe y se pega en la misma caja de la charla, abajo
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
    // lo que pide la IA, con los comandos mencionados resaltados y listos para copiar
    const askText = (cur().description.match(/LO QUE PIDE\n([\s\S]*?)\n\n/) || [])[1] || "";
    const fenced = [...askText.matchAll(/```[a-z]*\n?([\s\S]*?)```/g)].flatMap((m) => m[1].split("\n")).map((l) => l.replace(/^\$ /, "").trim()).filter((l) => l.length > 1);
    const askCmds = [...new Set(fenced.concat([...askText.replace(/```[\s\S]*?```/g, "").matchAll(/`([^`\n]+)`/g)].map((m) => m[1]).concat([...askText.matchAll(/'([a-z][^'\n]*\s[^'\n]*)'/g)].map((m) => m[1]))))].map((c) => c.trim()).filter(Boolean);
    // lista de comandos pedidos: se puede quitar los que no querés y permitir todo junto
    // herramientas de Claude que no son comandos de terminal (búsqueda web, MCP…): también se autorizan desde acá
    for (const t of [...new Set([...askText.matchAll(/\b(WebSearch|WebFetch|NotebookEdit|mcp__[\w-]+(?:__[\w*-]+)?)\b/g)].map((m) => m[1]))].reverse()) if (!askCmds.includes(t)) askCmds.unshift(t);
    let keep = [...askCmds];
    const cmdList = h("div", { class: "cmdlist" });
    const grantAll = guard(async () => {
      if (!keep.length) return toast("No quedan comandos", true);
      for (const cmd of keep) await api("POST", `/api/tasks/${task.id}/grant`, { command: cmd, always: true });
      await reply(`✅ Autorizados (siempre): ${keep.map((c) => "`" + c + "`").join(", ")}. La IA los puede usar en su próxima ejecución.`, true);
    });
    const authorizeAll = guard(async () => {
      const b = askBot();
      const fresh = keep.filter((c) => !allowedLines().includes(c));
      if (b) for (const cmd of fresh) await api("POST", `/api/tasks/${task.id}/grant`, { command: cmd, always: true });
      const text = keep.length ? `✅ Autorizado (siempre): ${keep.map((c) => "`" + c + "`").join(", ")}. La IA retoma la tarea sola.` : "✅ Autorizado: seguí.";
      if (b) await loadBots();
      await reply(text, true);
    });
    const trustBot = guard(async () => { const b = askBot(); if (!b) return toast("No encuentro a esa IA en este equipo", true); if (await trustProfile(b)) await reply("✅ Perfil confiable activado: ya no vas a necesitar pedir permisos (salvo lo prohibido). Seguí.", true); });
    // la IA que pidió ayuda y lo que ya tiene permitido: un comando de la lista puede estar ya autorizado y se quita de ahí
    const whoAsks = (cur().description.match(/La IA «([^»]+)»/) || [])[1];
    const askBot = () => (S.bots || []).find((x) => x.username === whoAsks);
    const allowedLines = () => (askBot()?.allow || "").split("\n").map((l) => l.trim()).filter(Boolean);
    const unallow = guard(async (cmd) => {
      const b = askBot(); if (!b) return;
      const rest = allowedLines().filter((l) => l !== cmd);
      await api("PATCH", `/api/bots/${b.id}`, { allow: rest.join("\n") }); await loadBots();
      toast("Quitado de la lista de " + b.username); drawCmds();
    });
    const drawCmds = () => {
      cmdList.replaceChildren(
        ...keep.map((cmd) => h("div", { class: "cmdbox big" }, h("code", {}, cmd),
          allowedLines().includes(cmd) && can("admin") && h("button", { class: "ghost", title: "Este comando ya está permitido para la IA: sacarlo de su lista de permitidos", onclick: () => unallow(cmd) }, "🗑 Quitar de la lista de la IA"),
          h("button", { class: "ghost", title: "Copiar el comando", onclick: () => navigator.clipboard?.writeText(cmd).then(() => toast("Comando copiado")) }, "📋"),
          h("button", { class: "danger", title: "Quitar este comando de la lista (no se permite)", onclick: () => { keep = keep.filter((c) => c !== cmd); drawCmds(); } }, "✕"))),
        keep.length < askCmds.length ? h("button", { class: "ghost", onclick: () => { keep = [...askCmds]; drawCmds(); } }, "↩ Restaurar quitados") : "");
    };
    drawCmds();
    return h("div", { class: "reqpanel" },
      h("b", { class: "req-title" }, "🙋 La IA espera tu respuesta"),
      askCmds.length > 0 && h("div", { class: "askbox" }, cmdList),
      cmds.length > 0 && h("div", {}, h("b", {}, "1. Copiá y ejecutá en tu terminal:"),
        cmds.map((cmd) => h("div", { class: "cmdbox" }, h("code", {}, cmd),
          h("button", { class: "ghost", title: "Copiar el comando", onclick: () => navigator.clipboard?.writeText(cmd).then(() => toast("Comando copiado")) }, "📋 Copiar")))),
      h("div", {}, h("small", { class: "hint" }, cmds.length ? "Pegá la salida abajo y apretá «Enviar salida a la IA»." : "Escribí tu respuesta abajo.")),
      h("div", { class: "row" },
        h("button", { class: "primary", title: "Publica lo que escribiste abajo como respuesta a la IA y marca el pedido como Hecho", onclick: () => out.value.trim() ? reply("📤 Salida:\n" + out.value.trim(), true).then(() => (out.value = "")) : toast("Escribí o pegá primero abajo", true) }, "📤 Enviar salida a la IA"),
        can("admin") ? h("button", { class: "primary big", title: "Permite siempre lo que pide la IA (comandos y herramientas de la lista), le avisa y marca el pedido como Hecho: la IA retoma sola", onclick: authorizeAll },
          keep.length ? `✅ Autorizar y que la IA siga (${keep.length})` : "✅ Autorizar y que la IA siga")
          : h("button", { class: "primary", onclick: () => reply("✅ Autorizado: seguí.", true) }, "✅ Autorizar"),
        h("span", { class: "spacer" }),
        h("button", { class: "danger", title: "Le responde a la IA que no; podés escribir el motivo arriba", onclick: () => reply("❌ No autorizado." + (out.value.trim() ? "\n" + out.value.trim() : ""), true).then(() => (out.value = "")) }, "❌ Rechazar")),
      can("admin") && askBot() && h("div", { class: "row" }, h("button", { class: "ghost", title: "Autorización total con una lista corta de cosas peligrosas prohibidas: la IA deja de pedir permisos", onclick: trustBot }, "🔓 Que esta IA no vuelva a pedir permisos (perfil confiable)")),
      h("details", {}, h("summary", {}, "Más opciones"),
        h("div", { class: "row" },
          can("admin") && h("button", { title: "Abre la lista de comandos permitidos de la IA, para sumar uno a mano", onclick: guard(async () => {
            const bot = askBot(); if (!bot) return toast("No encuentro a esa IA en este equipo", true); await editAllow(bot);
          }) }, "⚙ Lista de la IA"),
          can("admin") && h("button", { title: "Elegir un comando y si se permite solo esta vez o siempre", onclick: () => grantCmd() }, "Permitir solo uno…")),
        h("small", { class: "hint" }, "Si un comando está bloqueado, también podés ejecutarlo vos y enviar la salida.")));
  };

  // revisión del trabajo de la IA: aprobar o pedir cambios, e historial
  const reviewPanel = () => {
    const box = h("div", { class: "reviewpanel", hidden: true });
    api("GET", `/api/tasks/${task.id}/review`).then(({ items }) => {
      if (!items.length) return;
      const pending = items[0].state === "pending";
      const note = h("textarea", { rows: 3, maxLength: 4000, placeholder: "Si pedís cambios, contale a la IA qué tiene que corregir…" });
      const decide = guard(async (action) => {
        if (action === "changes" && !note.value.trim()) return toast("Escribí qué hay que cambiar", true);
        const r = await api("POST", `/api/tasks/${task.id}/review`, { action, note: note.value });
        upsert(r.task); toast(action === "approve" ? "Aprobada: la tarea pasó a Hecha" : "Cambios pedidos: la IA retoma la tarea"); closeModal(); render(); loadReviews();
      });
      const lbl = { pending: "⏳ Esperando revisión", approved: "✅ Aprobada", changes: "🔁 Cambios pedidos" };
      box.replaceChildren(
        h("h3", {}, "🔎 Revisión del trabajo de la IA"),
        pending && h("p", { class: "hint" }, `${items[0].requested_by || "La IA"} la dio por terminada. Revisá lo que hizo (comentarios de abajo) y decidí${items[0].reviewer ? ". Revisa: " + items[0].reviewer : ""}.`),
        pending && can("member") && h("div", {}, note, h("div", { class: "row" },
          h("button", { class: "primary", onclick: () => decide("approve") }, "✅ Aprobar y pasar a Hecha"),
          h("button", { onclick: () => decide("changes") }, "🔁 Pedir cambios"))),
        h("details", {}, h("summary", {}, `Historial (${items.length})`),
          items.map((i) => h("div", { class: "rv-row" }, h("b", {}, lbl[i.state] || i.state), h("small", {}, ` · ${new Date(i.requested_at).toLocaleString()}` + (i.decided_by ? ` · ${i.decided_by}` : "")), i.note && h("div", {}, i.note)))));
      box.hidden = false;
    }).catch(() => {});
    return box;
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
    if (!(await ask("¿Borrar la tarea?"))) return;
    await api("DELETE", `/api/tasks/${task.id}`);
    S.tasks = S.tasks.filter((t) => t.id !== task.id); closeModal(); render();
  });

  const modal = h("div", { class: "overlay", id: "modal", onmousedown: (e) => e.target.id === "modal" && closeModal() },
    h("div", { class: "modal task-modal" + (store.get("taskMax") === "1" ? " max" : "") },
      h("h3", { class: "task-head" }, "Tarea ", h("span", { class: "tid" }, "#" + task.id), h("span", { class: "spacer" }),
        h("button", { class: "primary", title: "Guardar los cambios y cerrar", onclick: save }, "💾 Guardar"),
        can("member") && h("button", { class: "danger", title: "Borrar la tarea", onclick: del }, "🗑 Borrar"),
        can("member") && showTech() && h("button", { class: "ghost", title: "Abre una terminal en la máquina del proyecto, sin salir de la tarea (owners; miembros si el servidor tiene espacios personales)", onclick: guard(() => openTaskTerminal(cur())) }, "💻 Terminal"),
        h("button", { class: "ghost", title: "Cerrar sin guardar", onclick: closeModal }, "✕"),
        h("button", { class: "ghost max-btn", title: "Maximizar o restaurar la ventana para leer los comentarios más cómodo", onclick: (e) => {
          const m = e.currentTarget.closest(".modal"); const on = m.classList.toggle("max"); store.set("taskMax", on ? "1" : "0");
          e.currentTarget.textContent = on ? "⤡ Restaurar" : "⤢ Maximizar";
        } }, store.get("taskMax") === "1" ? "⤡ Restaurar" : "⤢ Maximizar")),
      h("div", { class: "task-split" },
        h("div", { class: "task-main" },
          title, descView,
          desc,
          h("div", { class: "comments", id: "comments" }),
          requestPanel(),
          reviewPanel(),
          h("div", { class: "composer" }, comment,
            can("member") && showTech() && h("button", { class: "ghost", title: "Abre una terminal acá abajo, en la carpeta del proyecto, ligada a esta tarea (owners; miembros si el servidor tiene espacios personales)", onclick: toggleDock }, "💻 Terminal"),
            h("button", { class: "primary", onclick: send }, "Enviar")),
          dockSlot),
        h("aside", { class: "task-side" },
          h("div", { class: "grid2" },
            h("label", { title: FIELD_TIPS.status }, "Estado", status), h("label", { title: FIELD_TIPS.prio }, "Prioridad", prio),
            h("label", { title: FIELD_TIPS.who }, "Responsable", who), h("label", { title: FIELD_TIPS.start }, "Empieza", start),
            h("label", { title: FIELD_TIPS.due }, "Vence", due), h("label", { title: FIELD_TIPS.rec }, "Repetición", rec), h("small", { class: "hint" }, whoHint)),
          can("member") && budgetBox(cur(), title, desc),
          ghBox,
          flowBox(cur()),
          folderBox(cur()),
          h("h3", {}, "Etiquetas"), labelBox, newLabel,
          h("h3", {}, "Subtareas"), subBox, newSub,
          h("h3", {}, "Depende de"), depBox, depAdd,
          h("h3", {}, "Adjuntos"), attBox, attAdd,
          h("button", { class: "ghost", onclick: () => { closeModal(); openMindmap(cur()); } }, "Mapa mental"))),
      null));
  document.body.append(modal);
  drawLabels(); drawSubs(); drawDeps(); drawAtts(); drawDesc();
  renderComments();
  if (S.dockOpen === task.id) { S.dockOpen = null; toggleDock(); } else comment.focus();
});

// ---------- chat con IAs: qué IA contesta en el chat y con qué topes ----------
const botChatModal = guard(async (b) => {
  document.getElementById("botchat-modal")?.remove();
  const c = await api("GET", `/api/bots/${b.id}/chat`), close = () => document.getElementById("botchat-modal")?.remove();
  const KINDS = [["", "(como la corre el servidor)"], ["openrouter", "OpenRouter (modelos gratis y de pago)"], ["claude-cli", "Claude con tu sesión de Claude Code (sin clave de API)"], ["claude", "Claude por API (necesita ANTHROPIC_API_KEY)"], ["openai", "ChatGPT (OpenAI)"], ["gemini", "Gemini (Google)"], ["local", "Modelo propio (dirección compatible con OpenAI)"]];
  const on = h("input", { type: "checkbox", checked: c.enabled });
  const kind = h("select", {}, KINDS.map(([v, l]) => h("option", { value: v, selected: v === c.kind }, l + (v && c.available[v] === false ? " — falta la clave en el servidor" : ""))));
  const model = h("input", { value: c.model, placeholder: "ej.: haiku (Claude) · qwen/qwen3-coder:free · gpt-5-mini", maxLength: 120 });
  const url = h("input", { value: c.url, placeholder: "solo para modelo propio: http://servidor:11434", maxLength: 300 });
  const num = (v, min, max, step = 1) => h("input", { type: "number", min, max, step, value: v, style: "width:90px" });
  const usd = num(c.daily_usd, 0, 100, 0.1), hour = num(c.per_hour, 1, 200), ctx = num(c.ctx, 2, 30), tok = num(c.max_tokens, 50, 1500, 50);
  document.body.append(h("div", { class: "overlay", id: "botchat-modal", onmousedown: (e) => e.target.id === "botchat-modal" && close() },
    h("div", { class: "modal" }, h("h3", {}, `💬 Chat de ${b.username}`),
      h("small", {}, "La IA contesta solo cuando una persona le escribe en privado o la menciona con @nombre. Lo que escribe una IA nunca despierta a otra: no hay charla IA con IA ni gasto que se dispare."),
      h("label", { class: "inl" }, on, " Puede chatear con el equipo"),
      h("label", {}, "Proveedor", kind), h("label", {}, "Modelo (conviene uno barato y rápido)", model), h("label", {}, "Dirección (modelo propio)", url),
      h("div", { class: "grid2" },
        h("label", {}, "Tope por día (USD, estimado)", usd), h("label", {}, "Respuestas por hora", hour),
        h("label", {}, "Mensajes de contexto", ctx), h("label", {}, "Tokens por respuesta", tok)),
      h("small", { class: "hint" }, `Hoy gastó ≈ ${c.spent_usd.toFixed(4)} USD (${c.spent_tokens} tokens). El costo es una estimación; con OpenRouter se usa el que informa el proveedor. Si el servidor no tiene la clave del proveedor, la IA contesta que no pudo. «Claude con tu sesión» usa el cupo de tu suscripción de Claude Code (el mismo de las tareas): conviene Haiku y un tope bajo. Lo que se habla con una IA se envía a ese proveedor.`),
      h("div", { class: "row" }, h("button", { onclick: close }, "Cancelar"), h("span", { class: "spacer" }), h("button", { class: "primary", onclick: guard(async () => {
        await api("PUT", `/api/bots/${b.id}/chat`, { enabled: on.checked, kind: kind.value, model: model.value.trim(), url: url.value.trim(), daily_usd: +usd.value, per_hour: +hour.value, ctx: +ctx.value, max_tokens: +tok.value });
        close(); toast(on.checked ? `${b.username} ya puede chatear` : `${b.username}: chat apagado`);
      }) }, "Guardar")))));
});

// ---------- carpeta de la tarea: archivos por responsable y final/ ----------
function folderBox(task) {
  const body = h("div", { class: "tfolder" });
  const kb = (n) => (n < 1024 ? n + " B" : n < 1048576 ? (n / 1024).toFixed(n < 10240 ? 1 : 0) + " KB" : (n / 1048576).toFixed(1) + " MB");
  const join = (a, b) => (a ? a + "/" : "") + b;
  const t = () => S.tasks.find((x) => x.id === task.id) || task;
  const canDecide = () => can("member") && t().status !== "done" && (t().created_by === S.user?.id || can("admin")) && (!S.flows?.[task.id] || (S.flows[task.id].finished || t().waiting));
  async function show(path) {
    try {
      const r = await api("GET", `/api/tasks/${task.id}/folder?path=${encodeURIComponent(path)}`);
      const parts = path ? path.split("/") : [];
      const up = guard(async (e) => {
        const f = e.target.files[0]; if (!f) return;
        if (f.size > 25 * 1024 * 1024) return toast("Máximo 25 MB por archivo", true);
        await apiRaw("POST", `/api/tasks/${task.id}/folder/upload?name=${encodeURIComponent(f.name)}`, f);
        toast("Archivo subido a tu carpeta"); show(path);
      });
      const publish = guard(async () => {
        if (!(await ask("¿Aprobar y publicar? Se copia el trabajo del responsable a la carpeta final y la tarea pasa a Hecha.", "Aprobar y publicar"))) return;
        upsert(await api("POST", `/api/tasks/${task.id}/publish`)); toast("Publicado en final/ y tarea Hecha"); closeModal(); render();
      });
      body.replaceChildren(
        h("div", { class: "files-path" }, h("a", { onclick: () => show("") }, "📂 tarea"), ...parts.flatMap((p, i) => [" / ", h("a", { onclick: () => show(parts.slice(0, i + 1).join("/")) }, p)])),
        !r.exists ? h("small", { class: "hint" }, r.reason) : h("div", { class: "files-list" }, path && h("div", { class: "files-row", onclick: () => show(parts.slice(0, -1).join("/")) }, "⬆ .."),
          r.entries.map((e) => h("div", { class: "files-row", title: e.dir ? "Abrir carpeta" : "Descargar", onclick: () => (e.dir ? show(join(path, e.name)) : saveFile(`/api/tasks/${task.id}/folder/file?path=${encodeURIComponent(join(path, e.name))}`, e.name).catch((x) => toast(x.message, true))) },
            h("span", {}, (e.dir ? (e.name === "final" && !path ? "✅ " : "📁 ") : "📄 ") + e.name), h("small", {}, e.dir ? "" : kb(e.size)))),
          !r.entries.length && h("small", { class: "hint" }, "Carpeta vacía")),
        h("div", { class: "row" },
          can("member") && h("label", { class: "ghost-btn", title: "Sube un archivo a TU carpeta de esta tarea (para quien trabaja desde otra máquina o con su IA local). No se permiten programas ejecutables." }, "⬆ Subir a mi carpeta", h("input", { type: "file", hidden: true, onchange: up })),
          h("span", { class: "spacer" }),
          canDecide() && h("button", { class: "primary", title: "Decide quien creó la tarea (o un admin): copia el trabajo del responsable a final/ y pasa la tarea a Hecha", onclick: publish }, "✅ Aprobar y publicar")));
    } catch (e) { body.replaceChildren(h("small", { class: "hint" }, e.message)); }
  }
  const d = h("details", { class: "desc-box", title: "Cada responsable trabaja en su carpeta; lo aprobado queda en final/. Todos los del equipo pueden mirar y descargar; cada uno escribe solo en la suya.", ontoggle: (e) => e.target.open && show("") }, h("summary", {}, "📂 Carpeta de la tarea"), body);
  return d;
}

// ---------- flujo de la tarea: etapas con responsable e instrucción ----------
// la etapa actual y, si empieza un grupo en paralelo, todas las que trabajan a la vez
const inNow = (f, i) => i === f.step || (i > f.step && f.steps.slice(f.step + 1, i + 1).every((s) => s.par));
function flowBox(task) {
  const box = h("details", { class: "flow-box" }, h("summary", {}, "🧭 Flujo de la tarea"));
  const body = h("div", { class: "flow-body" }, h("small", { class: "hint" }, "Cargando…")); box.append(body);
  const reopen = (t) => { const i = S.tasks.findIndex((x) => x.id === t.id); if (i >= 0) S.tasks[i] = t; loadFlows(); closeModal(); openTask(t); };
  const who = (id) => S.members.find((m) => m.id === id);
  const edit = (f) => {
    const rows = (f.steps.length ? f.steps : [{ assignee_id: task.assignee_id || S.user.id, kind: "work", instruction: "" }, { assignee_id: "", kind: "review", instruction: "" }])
      .map((x) => ({ assignee_id: x.assignee_id, kind: x.kind, instruction: x.instruction, par: !!x.par }));
    const max = h("input", { type: "number", min: 1, max: 10, value: f.max_rounds || 3, style: "width:70px" });
    const list = h("div", { class: "flow-edit" });
    const draw = () => list.replaceChildren(...rows.map((r, i) => h("div", { class: "flow-row" },
      h("b", {}, i + 1 + "."),
      h("select", { onchange: (e) => (r.assignee_id = +e.target.value) }, h("option", { value: "" }, "— elegí —"), S.members.map((m) => h("option", { value: m.id, selected: m.id === r.assignee_id }, label(m.username)))),
      h("select", { onchange: (e) => (r.kind = e.target.value) }, [["work", "🛠 Hacer"], ["review", "🔎 Revisar"]].map(([v, l]) => h("option", { value: v, selected: v === r.kind }, l))),
      i > 0 && h("label", { class: "flow-par", title: "Trabaja a la vez que la etapa anterior: cada una recibe su subtarea y la etapa sigue sola cuando terminan todas" },
        h("input", { type: "checkbox", checked: !!r.par, onchange: (e) => { r.par = e.target.checked; if (r.par) r.kind = "work"; draw(); } }), " ⚡ en paralelo"),
      h("button", { class: "ghost", title: "Quitar la etapa", onclick: () => { rows.splice(i, 1); if (rows[0]) rows[0].par = false; draw(); } }, "✕"),
      h("textarea", { rows: 2, maxLength: 2000, placeholder: i === 0 ? "Qué tiene que hacer…" : "Qué espera de esta etapa…", oninput: (e) => (r.instruction = e.target.value) }, r.instruction))));
    draw();
    body.replaceChildren(list,
      h("div", { class: "row" }, h("button", { class: "ghost", onclick: () => { rows.push({ assignee_id: "", kind: "work", instruction: "" }); draw(); } }, "+ Etapa"), h("span", { class: "spacer" }),
        h("small", {}, "Tope de vueltas "), max),
      h("small", { class: "hint" }, "Al guardar, la tarea vuelve a la etapa 1 y pasa a su responsable. La primera etapa es de trabajo; una etapa de revisión aprueba (sigue) o devuelve a la etapa de trabajo anterior."),
      h("div", { class: "row" }, h("button", { onclick: () => draw_(f) }, "Cancelar"), h("span", { class: "spacer" }),
        can("member") && h("button", { class: "ghost", title: "Guarda este recorrido (etapas, instrucciones y tipo) para usarlo en otras tareas", onclick: guard(async () => {
          const r = await form("Guardar como plantilla", [{ name: "name", label: "Nombre de la plantilla", max: 60 }], "Guardar");
          if (!r || !r.name.trim()) return;
          await api("POST", `/api/teams/${S.team.id}/flow-templates`, { name: r.name, max_rounds: +max.value || 3, steps: rows.map((x) => ({ assignee_id: x.assignee_id || null, kind: x.kind, instruction: x.instruction, par: !!x.par })) });
          toast("Plantilla guardada");
        }) }, "💾 Guardar como plantilla"), h("button", { class: "primary", onclick: guard(async () => {
        if (rows.some((r) => !r.assignee_id)) return toast("Elegí el responsable de cada etapa", true);
        reopen(await api("PUT", `/api/tasks/${task.id}/flow`, { steps: rows.map((r) => ({ ...r, par: !!r.par })), max_rounds: +max.value || 3 }));
        toast("Flujo armado: la tarea pasó a la etapa 1");
      }) }, "Guardar y arrancar")));
  };
  // plantillas del equipo: se cargan en el editor (los responsables sin definir se eligen ahí) y se pueden borrar
  const tplPick = (f) => {
    const sel = h("select", { title: "Usar una plantilla guardada del equipo" }, h("option", { value: "" }, "📋 Usar plantilla…")), del = h("button", { class: "ghost", hidden: true, title: "Borrar esta plantilla", onclick: guard(async () => {
      const t = TPL.find((x) => x.id === +sel.value); if (!t || !confirm(`¿Borrar la plantilla «${t.name}»?`)) return;
      TPL = await api("DELETE", `/api/teams/${S.team.id}/flow-templates/${t.id}`); fill(); toast("Plantilla borrada");
    }) }, "🗑");
    let TPL = [];
    const fill = () => { sel.replaceChildren(h("option", { value: "" }, TPL.length ? "📋 Usar plantilla…" : "📋 Sin plantillas guardadas"), TPL.map((t) => h("option", { value: t.id }, `${t.name} (${t.steps.length} etapas)`))); del.hidden = true; };
    sel.onchange = () => {
      const t = TPL.find((x) => x.id === +sel.value); del.hidden = !t; if (!t) return;
      edit({ steps: t.steps.map((s) => ({ ...s, assignee_id: s.assignee_id || "" })), max_rounds: t.max_rounds });
      toast("Plantilla cargada: completá los responsables que falten y guardá");
    };
    api("GET", `/api/teams/${S.team.id}/flow-templates`).then((r) => { TPL = r; fill(); }).catch(() => {});
    return h("span", { class: "row" }, sel, del);
  };
  const draw_ = (f) => {
    if (!f.steps.length) {
      body.replaceChildren(h("small", { class: "hint" }, "Armá un recorrido: por ejemplo XXX hace, ZZZ revisa (y si hace falta, la devuelve a XXX con lo que hay que corregir), y después otro sigue. La salida de cada etapa pasa sola a la siguiente."),
        can("member") && h("div", { class: "row" }, h("button", { onclick: () => edit(f) }, "🧭 Armar flujo"), tplPick(f)));
      return;
    }
    const cur_ = f.steps[f.step], myTurn = f.active && cur_?.kind === "review" && (cur_.assignee_id === S.user.id || can("admin"));
    summary.textContent = "🧭 Flujo de la tarea · " + (f.finished ? "terminado" : `etapa ${f.step + 1} de ${f.steps.length}` + (f.round ? ` · vuelta ${f.round}/${f.max_rounds}` : ""));
    body.replaceChildren(
      h("ol", { class: "flow-list" }, f.steps.map((x, i) => h("li", { class: (!f.finished && inNow(f, i) ? "now" : "") + (f.finished || i < f.step ? " past" : "") },
        avatar(x.username), h("b", {}, " " + x.username), h("small", {}, x.kind === "review" ? " · 🔎 revisa" : " · 🛠 hace"), x.par && h("small", {}, " · ⚡ en paralelo"),
        (() => { const pt = (f.parts || []).find((q) => q.pos === i); return pt && h("small", {}, pt.done ? " · ✅ terminó" : " · ⏳ trabajando"); })(),
        !f.finished && inNow(f, i) && h("span", { class: "pill review" }, "ahora"),
        x.instruction && h("div", { class: "flow-ins" }, x.instruction)))),
      myTurn && h("div", { class: "row" },
        h("button", { class: "primary", onclick: guard(async () => { const r = await form("Aprobar", [{ name: "reason", label: "Comentario (opcional)", type: "textarea", max: 2000 }], "Aprobar y seguir"); if (!r) return; reopen(await api("POST", `/api/tasks/${task.id}/flow/verdict`, { ok: true, reason: r.reason || "" })); }) }, "✅ Aprobar y seguir"),
        h("button", { class: "danger", onclick: guard(async () => { const r = await form("Devolver", [{ name: "reason", label: "Qué hay que corregir", type: "textarea", max: 2000 }], "Devolver"); if (!r) return; if (!r.reason.trim()) return toast("Explicá qué hay que corregir", true); reopen(await api("POST", `/api/tasks/${task.id}/flow/verdict`, { ok: false, reason: r.reason })); }) }, "↩ Devolver")),
      can("member") && h("div", { class: "row" }, h("button", { class: "ghost", onclick: () => edit(f) }, "✏️ Editar flujo"), h("button", { class: "ghost", onclick: guard(async () => { await api("DELETE", `/api/tasks/${task.id}/flow`); loadFlows(); draw_({ steps: [] }); summary.textContent = "🧭 Flujo de la tarea"; toast("Flujo quitado"); }) }, "Quitar flujo")));
  };
  const summary = box.querySelector("summary");
  api("GET", `/api/tasks/${task.id}/flow`).then((f) => { if (f.steps.length) box.open = true; draw_(f); }).catch(() => body.replaceChildren(h("small", { class: "hint" }, "No se pudo cargar el flujo.")));
  return box;
}

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
    h("h1", {}, h("img", { src: "icon-192.png", alt: "", width: 56, height: 56, style: "vertical-align:middle;margin-right:10px;border-radius:12px" }), APP_NAME),
    h("label", {}, "Servidor", srv), h("label", {}, "Usuario", user), h("label", {}, "Contraseña", pass),
    err,
    h("button", { class: "primary", onclick: go }, S.register ? "Crear cuenta" : "Entrar"),
    h("button", { class: "ghost", onclick: () => { S.register = !S.register; render(); } },
      S.register ? "Ya tengo cuenta" : "Crear una cuenta")));
}

// una IA está ejecutando la tarea: En curso, asignada a una IA y sin estar en la zona de espera
const aiRunning = (t) => t.status === "doing" && !t.waiting;
const runLbl = (t) => (S.members?.find((m) => m.id === t.assignee_id)?.bot ? "🤖 Trabajando" : "En curso") + " · " + sinceTxt(t.updated_at);
function sinceTxt(iso) {
  const m = Math.max(0, Math.floor((Date.now() - new Date(iso).getTime()) / 60000));
  return m < 1 ? "recién empezó" : m < 60 ? `${m} min` : `${Math.floor(m / 60)} h ${m % 60} min`;
}
setInterval(() => document.querySelectorAll(".ai-since").forEach((e) => (e.textContent = e.dataset.lbl + " · " + sinceTxt(e.dataset.since))), 30000);
// explorador de solo lectura de la carpeta de trabajo del proyecto (para quien usa la web desde otra máquina)
const openFiles = guard(async () => {
  const pid = S.project.id;
  const body = h("div", { class: "files-body" });
  const modal = h("div", { class: "overlay", id: "files-modal", onmousedown: (e) => e.target.id === "files-modal" && modal.remove() },
    h("div", { class: "modal wide files-modal" }, h("h3", {}, "🗂 Archivos del proyecto", h("span", { class: "spacer" }), h("button", { class: "ghost", onclick: () => modal.remove() }, "✕")),
      h("small", { class: "hint" }, "Solo lectura. No se muestran archivos ocultos, claves ni bases de datos."), body));
  document.body.append(modal);
  const join = (a, b) => (a ? a + "/" : "") + b;
  const kb = (n) => (n < 1024 ? n + " B" : (n / 1024).toFixed(n < 10240 ? 1 : 0) + " KB");
  const crumbs = (path) => {
    const parts = path ? path.split("/") : [];
    return h("div", { class: "files-path" }, h("a", { onclick: () => show("") }, "📁 " + (S.project.work_dir.split("/").pop() || "/")),
      ...parts.flatMap((p, i) => [" / ", h("a", { onclick: () => show(parts.slice(0, i + 1).join("/")) }, p)]));
  };
  async function show(path) {
    try {
      const r = await api("GET", `/api/projects/${pid}/files?path=${encodeURIComponent(path)}`);
      body.replaceChildren(crumbs(r.path),
        h("div", { class: "files-list" }, path && h("div", { class: "files-row", onclick: () => show(path.split("/").slice(0, -1).join("/")) }, "⬆ ..",),
          r.entries.map((e) => h("div", { class: "files-row", onclick: () => (e.dir ? show(join(path, e.name)) : view(join(path, e.name))) },
            h("span", {}, (e.dir ? "📁 " : "📄 ") + e.name), h("small", {}, e.dir ? "" : kb(e.size)))),
          !r.entries.length && h("small", { class: "hint" }, "Carpeta vacía"), r.truncated && h("small", { class: "hint" }, "Se muestran los primeros 500.")));
    } catch (e) { toast(e.message, true); }
  }
  async function view(path) {
    try {
      const r = await api("GET", `/api/projects/${pid}/file?path=${encodeURIComponent(path)}`);
      const dir = path.split("/").slice(0, -1).join("/");
      body.replaceChildren(crumbs(dir),
        h("div", { class: "row" }, h("strong", {}, path), h("span", { class: "spacer" }),
          h("button", { onclick: () => copy(r.text, "Contenido copiado") }, "Copiar contenido"), h("button", { onclick: () => show(dir) }, "← Volver")),
        h("pre", { class: "files-view" }, r.text));
    } catch (e) { toast(e.message, true); }
  }
  show("");
});

// Recorrido del flujo en la tarjeta: quién la tiene ahora (resaltado), quién sigue y cuántas vueltas lleva.
function flowStrip(t) {
  const f = S.flows?.[t.id]; if (!f) return null;
  return h("div", { class: "flow-strip" + (f.finished ? " done" : ""), title: f.finished ? "Flujo terminado" : `Flujo: etapa ${f.step + 1} de ${f.steps.length}` + (f.round ? ` · vuelta ${f.round} de ${f.max_rounds}` : "") },
    f.steps.map((x, i) => h("span", { class: "fs-step" + (!f.finished && i === f.step ? " now" : "") + (f.finished || i < f.step ? " past" : "") + (x.kind === "review" ? " rev" : "") },
      i > 0 && h("i", {}, "→"), avatar(x.username), x.kind === "review" && h("small", {}, "🔎"))),
    f.round > 0 && !f.finished && h("span", { class: "fs-round" }, "↻" + f.round));
}

function taskCard(t) {
  const late = t.due && t.status !== "done" && new Date(t.due) < new Date();
  const card = h("div", { class: "card", draggable: true, onclick: () => openTask(t) },
    h("div", { class: "title" }, h("span", { class: "tid" }, "#" + t.id), " ", t.title),
    t.labels?.length > 0 && h("div", { class: "chips" }, t.labels.map((l) => h("span", { class: "chip on", style: `--c:${l.color}` }, l.name))),
    aiRunning(t) && h("div", { class: "ai-run", title: "Tarea en curso (el tiempo cuenta desde la última vez que se movió)" },
      h("div", { class: "ai-bar" }), h("small", { class: "ai-since", "data-since": t.updated_at, "data-lbl": runLbl(t).split(" · ")[0] }, runLbl(t))),
    flowStrip(t),
    t.subtasks_total > 0 && h("div", { class: "progress", title: `${t.subtasks_done}/${t.subtasks_total} subtareas` },
      h("div", { class: "bar", style: `width:${Math.round(100 * t.subtasks_done / t.subtasks_total)}%` })),
    h("div", { class: "meta" },
      inReview(t.id) && h("span", { class: "pill review", title: "Una IA la terminó y espera tu revisión" }, "🔎 En revisión"),
      t.subtasks_total > 0 && h("span", {}, `☑ ${t.subtasks_done}/${t.subtasks_total}`),
      h("span", { class: `pill p${t.priority}` }, PRIO[t.priority]),
      t.due && h("span", { class: "due" + (late ? " late" : "") }, "⏰ " + new Date(t.due).toLocaleDateString()),
      h("span", { class: "spacer" }),
      t.assignee && avatar(t.assignee)),
    t.status === "done" && can("member") && h("button", { class: "ghost card-x", title: "Archivar: la saca del tablero y la podés recuperar desde «Archivadas»", onclick: (e) => { e.stopPropagation(); setArchived(t, true); } }, "✕"));
  card.draggable = can("member");
  card.addEventListener("dragstart", (e) => { e.dataTransfer.setData("text/plain", String(t.id)); card.classList.add("dragging"); });
  card.addEventListener("dragend", () => card.classList.remove("dragging"));
  card.addEventListener("dragover", (e) => { if (can("member")) { e.preventDefault(); e.stopPropagation(); card.classList.add("drop-before"); } });
  card.addEventListener("dragleave", () => card.classList.remove("drop-before"));
  card.addEventListener("drop", (e) => { e.preventDefault(); e.stopPropagation(); card.classList.remove("drop-before"); moveTask(+e.dataTransfer.getData("text/plain"), t.status, t.id, t.waiting); });
  return card;
}

// mueve una tarea a la columna `status`, justo antes de `beforeId` (o al final); el orden manual manda sobre la prioridad
function moveTask(id, status, beforeId, waiting) {
  const t = S.tasks.find((x) => x.id === id);
  if (!t || !can("member") || id === beforeId) return;
  const col = S.tasks.filter((x) => !x.parent_id && x.status === status && x.id !== id).sort(byOrder);
  const i = beforeId == null ? col.length : col.findIndex((x) => x.id === beforeId);
  if (i < 0) return;
  const prev = col[i - 1]?.position, next = col[i]?.position;
  const position = prev == null ? (next == null ? t.position : next - 1) : next == null ? prev + 1 : (prev + next) / 2;
  const w = status === "doing" && !!waiting;
  if (position === t.position && status === t.status && w === !!t.waiting) return;
  const patch = { position };
  if (status !== t.status) patch.status = status;
  if (status === "doing" || t.waiting) patch.waiting = w;
  Object.assign(t, patch); render(); patchTask(id, patch);
}
const setArchived = (t, v) => { t.archived = v; render(); patchTask(t.id, { archived: v }); if (S.open === "archived") openArchived(); };
const byOrder = (a, b) => a.position - b.position || a.id - b.id;

function column(s) {
  const q = S.query.toLowerCase();
  const tasks = S.tasks
    .filter((t) => !t.parent_id && t.status === s.id && !(s.id === "done" && t.archived) && (!q || t.title.toLowerCase().includes(q) || (t.assignee || "").toLowerCase().includes(q)
      || t.labels.some((l) => l.name.toLowerCase().includes(q))))
    // en «Hecho» lo último que se terminó va arriba; en las demás, el orden manual
    .sort(s.id === "done" ? (a, b) => String(b.updated_at).localeCompare(String(a.updated_at)) : byOrder);
  const split = s.id === "doing" && (S.hold?.enabled || tasks.some((t) => t.waiting));
  const waitTasks = split ? tasks.filter((t) => t.waiting) : [];
  const waitZone = split && h("div", { class: "wait-zone" },
    h("div", { class: "wait-head" }, "⏳ Esperando tu próximo paso", h("span", { class: "count" }, waitTasks.length)),
    h("div", { class: "cards" }, waitTasks.length ? waitTasks.map(taskCard) : h("small", { class: "hint" }, "Acá queda lo que la IA entrega por tramos. Comentá para que siga, o pasalo a Hecho.")));
  waitZone && waitZone.addEventListener("drop", (e) => { e.preventDefault(); e.stopPropagation(); col.classList.remove("over"); moveTask(+e.dataTransfer.getData("text/plain"), "doing", null, true); });
  const col = h("section", { class: "col" },
    h("header", {}, h("span", { class: "bar", style: `background:${s.color}` }), s.label, h("span", { class: "count" }, tasks.length),
      s.id === "done" && h("span", { class: "spacer" }),
      s.id === "done" && h("button", { class: "ghost arch-btn", title: "Tareas hechas que sacaste del tablero: acá las ves y las recuperás", onclick: openArchived }, "🗄 Archivadas", h("span", { class: "count" }, S.tasks.filter((t) => t.archived && !t.parent_id).length))),
    h("div", { class: "cards" }, (split ? tasks.filter((t) => !t.waiting) : tasks).map(taskCard)),
    waitZone,
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

const toggleSide = () => { S.sideOff = !S.sideOff; store.set("sideOff", S.sideOff ? "1" : "0"); render(); setTimeout(() => window.dispatchEvent(new Event("resize")), 50); };
function appView() {
  const search = h("input", { placeholder: "Buscar…", value: S.query, oninput: (e) => { S.query = e.target.value; render(true); } });
  const dock = PANES.length > 0 && !S.terms && h("button", { class: "term-dock", title: "Tenés terminales abiertas: tocá para volver a ellas", onclick: () => openTerms() }, `🖥️ Terminales (${PANES.length})`);
  // la barra se parte en dos grupos (botones de acciones; vistas y buscador) que se pueden poner arriba o abajo (Preferencias)
  const actsLead = [
        // el nombre del proyecto ya se ve resaltado en la barra lateral: arriba se deja el lugar a los botones
        (S.terms || S.portfolio || !S.project) ? h("h1", {}, S.terms ? "🖥️ Terminales" : S.portfolio ? "📊 Portafolio" : S.team ? "Sin proyectos" : "Creá un equipo para empezar") : h("span", { class: "grow" }),
        h("div", { class: "members" }, S.members.map((m) => avatar(m.username))),
  ];
  const barActs = [
        ...(S.team && !S.portfolio && !S.terms ? (S.integ?.items || []).filter((it) => !it.url.includes("{sala}")).map((it) => h("button", { onclick: () => openInteg(it), title: it.mode === "panel" ? "Se abre en una pestaña flotante, sin salir del tablero" : "Se abre en una ventana nueva" }, it.icon + " " + it.name)) : []),
        S.project && !S.portfolio && !S.terms && h("button", { onclick: openSchedules, title: "Tareas que se crean solas a una hora fija: cada día, semana o mes" }, "⏰ Programadas"),
        S.project && !S.portfolio && !S.terms && h("button", { onclick: openUsage, title: "Cuánto gastaron las IAs en este proyecto: tokens, costo, horas y cuotas" }, "💰 Consumo"),
        S.team && !S.portfolio && !S.terms && h("button", { onclick: openReports, title: "Recibir un resumen diario o semanal: qué hicieron las IAs, cuánto gastaron y qué quedó trabado" }, "📊 Informes"),
        S.project && !S.portfolio && !S.terms && can("admin") && S.hold && h("button", { class: S.hold.enabled ? "primary" : "", title: "Con esto encendido, lo que una IA termina queda «⏳ esperando tu próximo paso» (En curso) y vos lo pasás a Hecho; al comentarlo, la IA lo retoma. Apagado: pasa directo a Hecho.", onclick: toggleHold }, "⏳ Continuación: " + (S.hold.enabled ? "sí" : "no")),
        S.project && !S.portfolio && !S.terms && can("admin") && S.reviews && h("button", { class: S.reviews.enabled ? "primary" : "", title: "Con esto encendido, lo que una IA da por terminado queda «en revisión» hasta que una persona lo apruebe (planes Equipo y Empresa)", onclick: toggleReview }, "🔎 Revisión: " + (S.reviews.enabled ? "sí" : "no")),
        S.project && !S.portfolio && !S.terms && can("member") && h("button", { onclick: githubProject, title: "GitHub: traer issues como tareas, estado de PR y pruebas, ajustes del proyecto" }, "🐙 GitHub" + ({ success: " ✅", failure: " ❌", pending: " ⏳" }[S.ghCi] || "")),
        S.project && !S.portfolio && !S.terms && can("admin") && h("button", { onclick: editProject, title: "Nombre, carpeta de trabajo y repositorio del proyecto" }, "⚙ Proyecto"),
        S.team && h("button", { class: S.chatOpen ? "on" : "", onclick: toggleChat, title: "Comunicación del equipo: mensajes, archivos, traducción y videollamadas" }, "💬 Comunicación" + (CHAT.total > 0 && !S.chatOpen ? ` (${CHAT.total})` : "")),
        S.team && h("button", { class: S.panel ? "on" : "", onclick: togglePanel, title: "Panel del equipo: miembros, roles, invitaciones y ajustes (Alt+K t)" }, "👥 Equipo"),
        S.team && can("member") && h("button", { onclick: sendTeamMessage, title: "Mensaje al equipo (Alt+K s)" }, "✉ Mensaje"),
  ];
  const barViews = [
        !S.portfolio && !S.terms && h("div", { class: "seg" },
          [["board", "Tablero"], ["calendar", "Calendario"], ["gantt", "Gantt"], ["canvas", "Pizarra"], ["activity", "Actividad"]].map(([v, l]) =>
            h("button", { class: S.view === v ? "on" : "", onclick: () => { S.view = v; render(); } }, l))),
        search
  ];
  const posActs = store.get("barActs") === "bottom" ? "bottom" : "top", posViews = store.get("barViews") === "bottom" ? "bottom" : "top";
  // cada grupo es un bloque: se minimiza (se desliza a la izquierda y queda una pestaña para volver a abrirlo) y se intercambia de lugar con el otro
  const viewsFirst = store.get("barOrder") === "views";
  const swapBlocks = () => {
    if (posActs !== posViews) { store.set("barActs", posViews); store.set("barViews", posActs); }
    else store.set("barOrder", viewsFirst ? "acts" : "views");
    render();
  };
  const block = (key, name, lead, items) => {
    const off = store.get("blk" + key) === "1";
    return h("div", { class: "top bblock" + (off ? " min" : "") }, ...lead,
      h("button", { class: "ghost blk-t", title: off ? `Mostrar ${name}` : `Minimizar ${name} (se desliza a la izquierda)`, onclick: () => { store.set("blk" + key, off ? "0" : "1"); render(); } }, off ? "› " + name : "‹"),
      !off && h("div", { class: "blk-body" }, items.filter(Boolean)),
      !off && h("button", { class: "ghost blk-t", title: "Intercambiar de lugar este bloque con el otro (arriba / abajo)", onclick: swapBlocks }, "⇅"));
  };
  const bActs = block("Acts", "Acciones", actsLead, barActs), bViews = block("Views", "Vistas", [], barViews);
  const inRegion = (pos) => [[posActs, bActs, 0], [posViews, bViews, 1]].filter(([p]) => p === pos).sort((x, y) => (viewsFirst ? y[2] - x[2] : x[2] - y[2])).map((x) => x[1]);
  const barTop = inRegion("top"), barBottom = inRegion("bottom");
  return h("div", { class: "shell" + (S.sideOff ? " side-off" : "") + (S.panel && S.team ? " with-panel" : "") + (S.chatOpen && S.team ? " with-chat" : "") + (S.terms && store.get("tvZen") === "1" ? " tv-zen" : "") },
    h("aside", {},
      h("div", { class: "brand" }, h("img", { src: "icon-192.png", alt: "", width: 22, height: 22, style: "vertical-align:middle;margin-right:6px;border-radius:5px" }), h("span", { class: "brand-name" }, APP_NAME),
        h("button", { class: "ghost side-toggle", title: S.sideOff ? "Mostrar la barra lateral" : "Ocultar la barra lateral (queda una franja para volver a abrirla)", onclick: toggleSide }, S.sideOff ? "»" : "«")),
      h("div", { class: "side-sec sec-teams" + secCls("teams") }, h("h2", {}, secTitle("teams", "👥 Equipos"), h("span", {}, S.team && h("button", { class: "ghost", onclick: () => { S.panel = true; store.set("panel", "1"); loadOverview(); setTimeout(() => document.querySelector(".tp-sec:last-child")?.scrollIntoView(), 400); }, title: "Editar o eliminar el equipo actual (ajustes en el panel de la derecha)" }, "⚙"), h("button", { class: "ghost", onclick: joinWithCode, title: "Unirse con un código" }, "↪"), h("button", { class: "ghost", onclick: createTeam, title: "Nuevo equipo" }, "+"))),
        h("div", { class: "nav" }, S.teams.map((t) => h("button", { class: t.id === S.team?.id ? "active" : "", onclick: () => selectTeam(t).catch((e) => toast(e.message, true)) }, t.name)))),
      S.team && h("div", { class: "side-sec sec-projects" + secCls("projects") }, h("h2", {}, secTitle("projects", "📁 Proyectos"), h("button", { class: "ghost", onclick: createProject, title: "Nuevo proyecto" }, "+")),
        h("div", { class: "nav" }, S.projects.length > 0 && can("member") && showTech() && h("button", { class: S.terms ? "active" : "", onclick: openTerms, title: "Terminales SSH de los proyectos y estado de las IAs, en paneles" }, "🖥️ Terminales"), S.projects.length > 0 && h("button", { class: S.portfolio ? "active" : "", onclick: openPortfolio, title: "Avance de todos los proyectos del equipo" }, "📊 Portafolio"),
          S.projects.map((p) => h("button", { class: p.id === S.project?.id && !S.portfolio && !S.terms ? "active" : "", onclick: () => selectProject(p).catch((e) => toast(e.message, true)) },
          h("span", { class: "dot", style: `background:${p.color}` }), p.name)))),
      S.team && h("div", { class: "side-sec sec-bots" + secCls("bots") }, h("h2", {}, secTitle("bots", "🤖 IAs"), can("admin") && h("button", { class: "ghost", onclick: createBot, title: "Sumar una IA al equipo (crea su token)" }, "+")),
        h("div", { class: "nav" }, (S.bots || []).map((b) => h("button", { class: b.active ? "" : "revoked", onclick: () => botModal(b), title: !b.active ? "Token revocado" : b.online ? "Conectada: su agente consultó hace instantes" : b.run_on ? "Activada en el servidor pero sin conectar" : "Sin agente conectado (no trabaja sola)" }, (!b.active ? "⚪ " : b.online ? "🟢 " : b.run_on ? "🔴 " : "⚪ ") + "🤖 " + b.username, b.model && h("small", { class: "inl" }, " · " + b.model))),
          !(S.bots || []).length && h("small", { class: "hint" }, can("admin") ? "Sumá asistentes de código con «+»" : "Sin IAs en este equipo"))),
      h("button", { class: "cfg-btn", onclick: openSettings, title: "Configuración (,)" }, "⚙  Configuración"),
      h("button", { class: "cfg-btn", onclick: () => openManual(), title: "Manual con buscador e índice por temas" }, "📖  Manual"),
      h("div", { class: "me" }, h("span", { class: "live" + (S.online ? " on" : ""), title: S.online ? "en vivo" : "desconectado" }),
        h("span", { title: S.team ? `Tu rol en «${S.team.name}»` : "" }, S.user?.username, S.team && h("small", { class: "hint" }, " · " + (ROLES.find(([r]) => r === myRole())?.[1] || ""))), h("span", { class: "spacer" }),
        S.team && h("button", { class: "ghost" + (S.chatOpen ? " on" : ""), title: "Comunicación del equipo: chat, archivos, traducción y videollamadas", onclick: toggleChat }, "💬" + (CHAT.total > 0 && !S.chatOpen ? ` ${CHAT.total}` : "")),
        h("button", { class: "ghost bell" + (S.unread ? " has" : ""), title: "Bandeja de novedades (Alt+K n)", onclick: () => loadInbox(false) }, S.unread ? `🔔 ${S.unread}` : "🔔"),
        h("button", { class: "ghost", onclick: cycleTheme, title: "Tema: " + THEMES[themePref()][1] + " (clic para cambiar)" }, THEMES[themePref()][0]),
        h("button", { class: "ghost", onclick: helpModal, title: "Atajos y ayuda (?)" }, "?"),
        h("button", { class: "ghost", onclick: logout }, "Salir"),
        h("button", { class: "ghost", onclick: logoutAll, title: "Cierra la sesión en todos los dispositivos" }, "⏻"))),
    h("main", {},
      barTop.length > 0 && h("div", { class: "bars" }, barTop),
      S.project && !S.portfolio && !S.terms && h("div", { class: "proj-meta" },
        !S.project.work_dir && !S.project.repo && h("span", { class: "hint" }, "Sin carpeta de trabajo ni repositorio ", can("admin") && h("button", { class: "ghost", onclick: editProject, title: "Definir la carpeta de trabajo y el repositorio" }, "✏️ Definir")),
        S.project.work_dir && h("span", { title: "Carpeta de trabajo" }, "📁 " + S.project.work_dir, " ",
          h("button", { class: "ghost", title: "Abre la carpeta si estás en la máquina del servidor; desde otra máquina copia la ruta", onclick: async () => {
            try { await api("POST", `/api/projects/${S.project.id}/open-folder`); toast("Carpeta abierta"); }
            catch (e) { copy(S.project.work_dir, "La carpeta está en la máquina del servidor: no se abre desde acá. Ruta copiada"); }
          } }, "📂 Abrir"), " ", h("button", { class: "ghost", title: "Ver los archivos de la carpeta de trabajo (solo lectura), sin importar desde qué máquina entres", onclick: openFiles }, "🗂 Archivos")),
        !S.project.repo && can("member") && h("span", { class: "warn", title: "Sin repositorio git conectado, la carpeta del proyecto queda de solo lectura para el equipo: nadie puede editar ni enviar su trabajo a revisión" }, "⚠ Sin repositorio ",
          can("owner") ? h("button", { class: "ghost", onclick: () => openGitSetup(S.project), title: "Abre el asistente: traer un repo de GitHub, crear uno nuevo o dejarlo solo en la máquina" }, "🐙 Crear o conectar")
            : h("button", { class: "ghost", onclick: () => requestRepo(S.project.id), title: "Les avisa a los owners del equipo para que creen el repositorio" }, "🙋 Pedir que lo creen")),
        S.project.repo && h("button", { class: "ghost", onclick: openIntegrar, title: "Revisar y juntar el trabajo que mandó el equipo (Pull Requests)  ·  git diff / git merge" }, "🔀 Integrar"),
        S.project.repo && (repoUrl(S.project.repo)
          ? h("a", { class: "repo-link", href: repoUrl(S.project.repo), target: "_blank", rel: "noopener", title: "Abrir el repositorio en una pestaña nueva" }, "🐙 " + S.project.repo.replace(/^https?:\/\/(www\.)?github\.com\//, "").replace(/\.git$/, "") + " ↗")
          : h("span", { title: "Repositorio" }, "🔗 " + S.project.repo))),
      S.terms ? termsView() : S.portfolio ? portfolioView() : S.project ? (S.view === "calendar" ? calendarView() : S.view === "gantt" ? ganttView() : S.view === "canvas" ? canvasView() : S.view === "activity" ? activityView() : h("div", { class: "board" }, STATUS.map(column)))
        : S.team ? h("div", { class: "empty" }, "Creá un proyecto con + en la barra lateral") : welcomeView(),
      barBottom.length > 0 && h("div", { class: "bars bottom" }, barBottom)),
    S.panel && S.team && teamPanel(), S.chatOpen && S.team && chatPanel(), dock);
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
      : h("div", { class: "act-list" }, S.pf.ai_recent.map((a) => h("div", { class: "act-row" + (a.task_id ? " clickable" : ""), title: a.task_id ? "Abrir la tarea y ver toda la conversación" : "", onclick: a.task_id ? guard(async () => openTask(await api("GET", `/api/tasks/${a.task_id}`))) : null }, avatar(a.who),
        h("span", { class: "grow" }, h("b", {}, a.who), " ", a.text, h("small", { class: "inl" }, " · " + a.project)), h("small", {}, ago(a.at)), a.task_id && h("small", { class: "hint" }, "💬 ver")))));
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
      : h("div", { class: "act-list" }, items.map((a) => h("div", { class: "act-row" + (a.task_id ? " clickable" : ""), title: a.task_id ? "Abrir la tarea: conversación, respuestas y responsables" : "", onclick: a.task_id ? guard(async () => openTask(await api("GET", `/api/tasks/${a.task_id}`))) : null }, avatar(a.username || "?"),
        h("span", { class: "grow" }, h("b", {}, a.username || "alguien"), " ", a.text), h("small", {}, new Date(a.created_at).toLocaleString()), a.task_id && h("small", { class: "hint" }, "💬 ver")))));
}

// ---------- configuración ----------
const IS_WEB = /^https?:$/.test(location.protocol) && location.hostname !== "tauri.localhost";
const VIEWS = [["board", "Tablero"], ["calendar", "Calendario"], ["gantt", "Gantt"], ["canvas", "Pizarra"], ["activity", "Actividad"]];

// Ayuda de cada fila de Configuración: qué hace (usuario) y qué hay que tener en el servidor (admin del servidor)
const CFG_HELP = {
  user: ["Usuario", "Es tu nombre en la app. Tus compañeros te buscan por él cuando un admin te agrega a un equipo.", null],
  email: ["Email para avisos", "Tocá «Configurar», escribí tu email y poné el código de 6 dígitos que te llega. Solo los emails verificados reciben avisos: asignaciones, menciones, recordatorios y mensajes del equipo.",
    "El servidor necesita una cuenta de correo saliente (Gmail, Outlook, Brevo, SendGrid, Mailgun, SES o uno propio). Creá el archivo ~/.config/tm-server/mail.env (permisos 600) con TM_SMTP_URL, TM_SMTP_USER, TM_SMTP_PASSWORD y TM_SMTP_FROM, agregá EnvironmentFile=-%h/.config/tm-server/mail.env al servicio de tm-server y reinicialo. Sin eso el servidor no manda emails y acá no se puede verificar. Los datos de cada proveedor están en el manual."],
  github: ["GitHub", "Tocá «Conectar», abrí la página que se muestra, escribí el código y autorizá. Queda vinculada tu cuenta, con tus propios permisos: la app no ve nada que vos no puedas ver. Podés desconectar cuando quieras.",
    "Creá una OAuth App en github.com → Settings → Developer settings → OAuth Apps, tildá «Enable Device Flow» y copiá su Client ID (no hace falta el secreto). Arrancá tm-server con TM_GITHUB_CLIENT_ID=<ese id> (por ejemplo en ~/.config/tm-server/github.env). Por defecto se piden los permisos «read:user repo»; TM_GITHUB_SCOPE los cambia."],
  lang: ["Idioma", "Cambia el idioma de la app para vos (español, inglés o portugués). Los demás del equipo siguen con el suyo.", null],
  license: ["Licencia", "La app es software libre (AGPL-3.0): podés usarla gratis cumpliendo esa licencia. Si tu empresa prefiere no cumplirla, existe una licencia comercial anual. Acá ves el estado y, si sos la primera persona registrada, podés comprarla.", null],
  guide: ["Guía de inicio", "Un asistente corto con dibujos que te lleva a dejar todo andando: equipo, proyecto e IA. Se abre solo la primera vez y la podés repetir cuando quieras.", null],
  cmds: ["Comandos para todas las IAs", "Son los comandos que cualquier IA del servidor puede usar sin pedir permiso (permitidos) y los que nunca puede usar (prohibidos). Se suman a la lista propia de cada IA, que está en su ficha. Lo prohibido siempre gana. Un comando se escribe así: git clone, cargo build o Bash(npm test:*).", "Solo la primera persona registrada edita estas listas. Los archivos whitelist.txt y blacklist.txt del agente son la base; estas listas se suman a ellas."],
  digest: ["Cómo recibís los avisos", "Resumen: los avisos por Telegram y email llegan juntos en un solo mensaje, una vez por día a la hora que elijas o cada 1, 2, 3, 6 o 12 horas. Uno por uno: cada aviso sale al momento. 🔕 Silencioso: no te llega nada por Telegram ni email (ni siquiera los pedidos 🙋 de las IAs). ⏸ Pausar: silencio por un rato (1 hora a 1 semana) y después vuelve solo a como lo tenías. La bandeja de la app siempre se actualiza al instante. Fuera del silencio, los pedidos de ayuda de una IA (🙋) salen siempre al momento.", null],
  reports: ["Informes automáticos", "Un resumen diario o semanal del equipo: qué terminaron las personas y las IAs, cuánto gastaron las IAs y qué quedó trabado. Te llega a la bandeja y, si querés, por Telegram y email.", null],
  telegram: ["Telegram", "Tocá «Vincular», abrí el bot que se muestra y mandale /start CÓDIGO. Después recibís las novedades en Telegram y podés usar /tareas, /hecho N (marca la tarea N como hecha) y /stop (desvincula).",
    "Creá un bot con @BotFather en Telegram y arrancá tm-server con TM_TELEGRAM_TOKEN=<token>. No necesita URL pública (usa long polling). Sin el token, esta fila dice «el servidor no tiene el bot configurado»."],
  inbox: ["Bandeja de novedades", "Reúne lo que te pasó: asignaciones, comentarios, menciones (@usuario), recordatorios y mensajes. Se marcan como leídas al cerrarla. También abre con la campana 🔔 o Alt+K n.",
    "Los recordatorios de tareas por vencer salen solos; la frecuencia se ajusta con TM_REMINDER_SECS (300 segundos por defecto)."],
  logout: ["Cerrar sesión", "Sale de la app en este dispositivo. Tus datos quedan en el servidor.", null],
  logoutAll: ["Cerrar todas las sesiones", "Úsalo si perdiste un dispositivo o sospechás que alguien más entró: invalida todas tus sesiones abiertas y tenés que volver a iniciar sesión en cada dispositivo.", null],
  role: ["Tu rol", "Lector: solo mira. Miembro: crea, edita y comenta tareas y escribe al equipo. Admin: además agrega miembros, cambia roles y genera invitaciones. Dueño: todo lo anterior y es quien creó el equipo. Si necesitás otro permiso, pedíselo a un admin.",
    "El rol es por equipo, no del servidor: administrar el servidor (variables de entorno, base de datos) es independiente de ser admin o dueño de un equipo."],
  message: ["Mensaje al equipo", "Escribe un aviso que le llega a todos los miembros del equipo por bandeja, y por email y Telegram a quienes lo tengan conectado.", null],
  members: ["Miembros e invitaciones", "Mirá quiénes están en el equipo. Si sos admin o dueño: buscá a alguien ya registrado y agregalo, o generá un código de invitación (y «Copiar invitación» arma el texto con el enlace de instalación para pegar en un chat).", null],
  agents: ["Agentes de IA (tm-agent)", "Para que una IA trabaje sola cuando le asignás una tarea, instalás tm-agent en la máquina donde está la carpeta del proyecto. Tocá «Ver instructivo» para los pasos y el comando listo.",
    "tm-agent no es parte del servidor: es otro programa que se conecta al servidor con el token de la IA. Se instala con cargo install --git https://github.com/polblancoo/crewhelm crewhelm-agent y se deja como servicio (systemd, launchd o el Programador de tareas)."],
  join: ["Unirme con un código", "Si un admin te pasó un código de invitación, pegalo acá para entrar a su equipo.", null],
  newTeam: ["Crear un equipo", "Creás un equipo nuevo y quedás como Dueño. Después invitás a la gente desde «Miembros, roles e invitaciones».", null],
  project: ["Proyecto", "Un proyecto agrupa las tareas del equipo (tablero, calendario, Gantt y pizarra). Creá uno con «Nuevo proyecto».", null],
  perfil: ["Perfil de uso", "Prepara la interfaz según para qué usan el equipo: cambia los nombres de las columnas y oculta lo técnico (terminales, carpeta del código) si no hace falta. «Cargar ejemplo» crea un proyecto de muestra con tareas; «Eliminar ejemplo» lo borra entero y deja todo limpio. No toca tus proyectos reales.", null],
  duplicate: ["Usar como plantilla", "Crea un proyecto nuevo copiando las tareas, etiquetas y dependencias del actual, para repetir un proceso sin armarlo de cero.", null],
  export: ["Exportar", "Descarga todas las tareas del proyecto en CSV (para planilla) o JSON (para copia de seguridad o scripts).", null],
  integrations: ["Integraciones", "Sumá herramientas externas (por ejemplo una videollamada con Jitsi) como botones en la barra de arriba. Con el asistente queda lista en tres pasos, con salida a internet o con tu propio servidor. Las configura un administrador y las usa todo el equipo.", null],
  bars: ["Posición de los botones", "Elegí si los botones del proyecto (Programadas, Consumo, GitHub, Proyecto, Equipo…) y las vistas con el buscador (Tablero, Calendario, Gantt…) van arriba o abajo de la pantalla. Se guarda solo en este dispositivo.", null],
  chatExts: ["Archivos del chat", "Qué tipos de archivo se pueden mandar por el chat (hasta 5 MB cada uno). Rige para todo el servidor y lo cambia el dueño de un equipo. Los programas, scripts, HTML y SVG nunca se pueden enviar. Los comprimidos (zip, rar…) vienen apagados porque pueden esconder programas.", null],
  avatar: ["Tu avatar", "El dibujito (emoji) que ven los demás junto a tu nombre: en el chat, el equipo y las tareas. Si no elegís, se ven tus iniciales.", null],
  bubbleSecs: ["Burbuja del chat", "Cuánto queda visible la burbuja que avisa de un mensaje nuevo del chat. Tocarla abre la conversación. Se guarda solo en este dispositivo.", null],
  toastSecs: ["Aviso emergente", "El cartelito que aparece abajo cuando llega un aviso. Elegí cuánto queda visible, o que se quede hasta que lo toques. Si el aviso es de una tarea, tocarlo abre la tarea para leerlo completo; si no, lo cierra. Se guarda solo en este dispositivo.", null],
  herdr: ["Terminal con herdr", "Al abrir la terminal de una tarea se abre herdr (un panel de terminales) en la carpeta del proyecto, si la máquina lo tiene. Apagalo si preferís una terminal común. Se guarda solo en este dispositivo.", null],
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
// GitHub por Device Flow: se muestra un código para escribir en github.com/login/device y se espera la autorización
const linkGithub = guard(async () => {
  const d = await api("POST", "/api/github/link");
  const note = h("p", { class: "hint" }, "Esperando que autorices en GitHub…");
  let alive = true;
  const close = () => { alive = false; box.remove(); };
  const box = h("div", { class: "overlay", onmousedown: (e) => e.target === box && close() },
    h("div", { class: "modal small" }, h("h3", {}, "Conectar GitHub"),
      h("p", {}, "1. Abrí esta página e iniciá sesión en GitHub:"),
      h("p", {}, h("a", { href: d.verification_uri, target: "_blank", rel: "noopener" }, d.verification_uri)),
      h("p", {}, "2. Escribí este código:"),
      h("code", { class: "bot-token", style: "font-size:1.6em;letter-spacing:.15em;text-align:center" }, d.user_code),
      note,
      h("div", { class: "row" }, h("button", { onclick: () => copy(d.user_code, "Código copiado") }, "Copiar código"), h("span", { class: "spacer" }), h("button", { onclick: close }, "Cancelar"))));
  document.body.append(box);
  let wait = Math.max(d.interval, 3) * 1000;
  const t0 = Date.now();
  while (alive && Date.now() - t0 < d.expires_in * 1000) {
    await new Promise((r) => setTimeout(r, wait));
    if (!alive) return;
    const r = await api("POST", "/api/github/poll").catch(() => ({ status: "error" }));
    if (r.status === "linked") { close(); toast("GitHub conectado como @" + r.login); openSettings(); return; }
    if (r.status === "slow_down") wait += 5000;
    if (["expired", "denied", "none"].includes(r.status)) { note.textContent = r.status === "denied" ? "❌ Rechazaste el acceso en GitHub." : "⌛ El código venció. Cerrá y volvé a intentar."; return; }
  }
});
// GitHub por proyecto: se indica el repositorio y se traen sus issues abiertos como tareas
const githubProject = guard(async () => {
  const p = S.project; if (!p) return;
  const info = await api("GET", `/api/projects/${p.id}/github`);
  if (!info.available) { toast("El servidor no tiene GitHub configurado (TM_GITHUB_CLIENT_ID)", true); return; }
  if (!info.linked || (!p.repo && can("owner"))) {
    const shut = () => document.getElementById("gh-intro")?.remove();
    document.body.append(h("div", { class: "overlay", id: "gh-intro", onmousedown: (e) => e.target.id === "gh-intro" && shut() },
      h("div", { class: "modal" }, h("h3", {}, "🐙 GitHub de «" + p.name + "»"),
        h("p", { class: "hint" }, "Acá hay dos cosas distintas:"),
        h("p", {}, h("b", {}, "1. El repositorio del proyecto"), p.repo ? ": " + p.repo + " ✓" : ": todavía no tiene. Es lo que permite que el equipo trabaje en ramas y mande su trabajo a revisión, sin que cada persona necesite cuenta de GitHub."),
        !p.repo && can("owner") && h("div", { class: "row" }, h("button", { class: "primary", onclick: () => { shut(); openGitSetup(p); } }, "🐙 Crear o conectar el repositorio")),
        h("p", {}, h("b", {}, "2. Tu cuenta personal de GitHub"), info.linked ? ": conectada ✓" : ": sin conectar. Sirve solo para traer issues como tareas y ver el estado de los PR y las pruebas. No hace falta para trabajar."),
        !info.linked && h("div", { class: "row" }, h("button", { onclick: () => { shut(); linkGithub(); } }, "Conectar mi cuenta")),
        h("div", { class: "actions" }, h("button", { class: "ghost", onclick: shut }, info.linked ? "Seguir" : "Cerrar")))));
    return;
  }
  const close = () => box.remove();
  // si el proyecto ya tiene «Repositorio» cargado en ⚙ Proyecto, se usa ese (usuario/repo o URL de GitHub)
  const fromProject = (p.repo || "").trim().replace(/^https?:\/\/github\.com\//, "").replace(/\.git$/, "").replace(/\/$/, "");
  if (info.repo !== fromProject && /^[\w.-]+\/[\w.-]+$/.test(fromProject)) {
    await api("PUT", `/api/projects/${p.id}/github`, { repo: fromProject }); info.repo = fromProject;
  }
  const repo = h("input", { value: info.repo || "", placeholder: "dueño/repositorio, ej. polblancoo/crewhelm", maxLength: 200 });
  const list = h("div", { class: "subs" });
  const picked = new Set();
  const load = guard(async () => {
    list.replaceChildren(h("small", { class: "hint" }, "Buscando issues abiertos…"));
    const r = await api("GET", `/api/projects/${p.id}/github/issues`);
    if (!r.issues.length) { list.replaceChildren(h("small", { class: "hint" }, "No hay issues abiertos en " + r.repo)); return; }
    list.replaceChildren(...r.issues.map((i) => h("label", { class: "sub" },
      h("input", { type: "checkbox", disabled: i.imported, onchange: (e) => (e.target.checked ? picked.add(i.number) : picked.delete(i.number)) }),
      h("span", {}, `#${i.number} ${i.title}` + (i.imported ? " (ya es tarea)" : "")))));
  });
  // ajustes del proyecto: cada casilla prende o apaga una función; lo automático usa tu conexión de GitHub
  const cfg = await api("GET", `/api/projects/${p.id}/github/settings`).catch(() => ({}));
  const OPTS = [
    ["auto_sync", "Revisar GitHub cada 5 minutos", "Pasa a Hecha la tarea cuando se cierra su issue o se integra su Pull Request, y muestra el estado del PR."],
    ["sync_comments", "Pasar comentarios entre la tarea y el issue", "Lo que se escribe en uno aparece en el otro. Ojo: si el repositorio es público, esos comentarios los ve cualquiera."],
    ["publish_issues", "Publicar como issue las tareas nuevas", "Desde que prendas esto, cada tarea nueva se crea también como issue en el repositorio."],
    ["ai_pr", "Avisarme cuando una IA termina para abrir el Pull Request", "La IA trabaja en la rama tm/tarea-N. Nunca se abre un PR sin que lo toques vos."],
    ["ci_status", "Mostrar si las pruebas (CI) pasan", "Un ✅ o ❌ junto al botón 🐙 GitHub, y un aviso si fallan."],
    ["activity", "Anotar los commits en Actividad", "Cada commit nuevo del repositorio queda en la bitácora del proyecto."],
    ["webhook", "Avisos al instante (webhook)", "GitHub avisa en el acto en vez de esperar 5 minutos. Solo sirve si tu servidor es alcanzable desde internet."],
  ];
  const checks = Object.fromEntries(OPTS.map(([k]) => [k, h("input", { type: "checkbox", checked: !!cfg[k], disabled: !can("admin") })]));
  const hookInfo = h("small", { class: "hint" });
  const drawHook = () => {
    hookInfo.textContent = checks.webhook.checked && cfg.webhook_secret
      ? `En GitHub: Settings → Webhooks → Add webhook. URL: ${location.origin}${cfg.webhook_path} · tipo application/json · Secret: ${cfg.webhook_secret} · eventos: issues, pull requests, comments, push y check runs.`
      : "";
  };
  checks.webhook.onchange = drawHook; drawHook();
  const settingsBox = h("details", { class: "gh-cfg", open: Object.keys(checks).some((k) => checks[k].checked) },
    h("summary", {}, "Ajustes de GitHub para este proyecto" + (cfg.last_sync ? ` · última revisión ${new Date(cfg.last_sync).toLocaleString()}` : "")),
    OPTS.map(([k, l, d]) => h("label", { class: "row", title: d }, checks[k], h("span", {}, l), h("small", { class: "hint" }, d))),
    hookInfo,
    can("admin") && h("div", { class: "row" },
      h("button", { class: "primary", onclick: guard(async () => {
        const r = await api("PUT", `/api/projects/${p.id}/github/settings`, Object.fromEntries(OPTS.map(([k]) => [k, checks[k].checked])));
        Object.assign(cfg, r); drawHook(); S.ghCi = r.ci_status ? r.ci_state : ""; render(); toast("Ajustes de GitHub guardados");
      }) }, "Guardar ajustes"),
      h("button", { onclick: guard(async () => {
        const r = await api("POST", `/api/projects/${p.id}/github/now`);
        S.tasks = await api("GET", `/api/projects/${p.id}/tasks`); render(); toast("Sincronizado: " + (r.summary || "sin cambios"));
      }) }, "Sincronizar ahora")));
  const box = h("div", { class: "overlay", onmousedown: (e) => e.target === box && close() },
    h("div", { class: "modal wide" }, h("h3", {}, `GitHub · ${p.name}`),
      h("label", {}, "Repositorio de este proyecto", repo),
      h("div", { class: "row" },
        h("button", { onclick: guard(async () => { await api("PUT", `/api/projects/${p.id}/github`, { repo: repo.value }); toast(repo.value.trim() ? "Repositorio guardado" : "Repositorio quitado"); if (repo.value.trim()) load(); else list.replaceChildren(); }) }, "Guardar"),
        h("button", { onclick: guard(async () => { await api("PUT", `/api/projects/${p.id}/github`, { repo: repo.value }); load(); }) }, "Ver issues abiertos")),
      list,
      settingsBox,
      h("div", { class: "row" },
        h("button", { onclick: guard(async () => {
          if (!picked.size) { toast("Tildá al menos un issue", true); return; }
          const r = await api("POST", `/api/projects/${p.id}/github/import`, { numbers: [...picked] });
          S.tasks = await api("GET", `/api/projects/${p.id}/tasks`); render(); close(); toast(`${r.created} tarea(s) creadas desde GitHub`);
        }) }, "Traer los tildados como tareas"),
        h("button", { onclick: guard(async () => {
          const r = await api("POST", `/api/projects/${p.id}/github/sync`);
          S.tasks = await api("GET", `/api/projects/${p.id}/tasks`); render(); toast(r.closed ? `${r.closed} tarea(s) pasaron a Hecha porque su issue se cerró` : "Todo al día: ningún issue importado se cerró");
        }) }, "Actualizar estados"),
        h("span", { class: "spacer" }), h("button", { onclick: close }, "Cerrar"))));
  document.body.append(box);
  if (info.repo) load();
});
// ---------- integraciones: videollamada (Jitsi) y otras herramientas, en una pestaña flotante o en ventana nueva ----------
async function loadInteg() {
  const id = S.team?.id; if (!id) { S.integ = null; return; }
  try { const r = await api("GET", `/api/teams/${id}/integrations`); if (S.team?.id === id) { S.integ = r; render(true); if (CHAT.el) chatDraw(); } } catch { S.integ = null; }
}
const integUrl = (u, room) => u.replaceAll("{sala}", encodeURIComponent(room || S.integ?.room || "")).replaceAll("{equipo}", encodeURIComponent((S.integ?.team || "").replace(/\s+/g, "-"))).replaceAll("{usuario}", encodeURIComponent(S.user?.username || ""));
const DOCK = { tabs: [], active: null };
function openInteg(it) {
  if (it.mode !== "panel") { window.open(integUrl(it.url), "_blank", "noopener,noreferrer"); return; }
  if (!DOCK.tabs.some((t) => t.id === it.id)) DOCK.tabs.push({ id: it.id, name: it.name, icon: it.icon, src: integUrl(it.url) });
  DOCK.active = it.id; drawDock();
}
// ---------- llamadas desde el chat: sala propia y aleatoria, la invitación llega solo a quienes ven esa conversación ----------
const CALL_RE = /\[\[call:([a-f0-9]{16,40})\]\]/;
const callItem = () => (S.integ?.items || []).find((i) => i.url.includes("{sala}"));
function openCall(room) {
  const it = callItem();
  if (!it) { toast("Falta configurar la videollamada (Configuración → Integraciones)", true); return; }
  const src = integUrl(it.url, "crewhelm-" + room), id = "call-" + room;
  if (it.mode !== "panel") { window.open(src, "_blank", "noopener,noreferrer"); return; }
  if (!DOCK.tabs.some((t) => t.id === id)) DOCK.tabs.push({ id, name: "Llamada", icon: "📞", src });
  DOCK.active = id; document.getElementById("integ-dock")?.classList.remove("min"); drawDock();
}
const chatCall = guard(async () => {
  if (!callItem()) { toast(can("admin") ? "Primero configurá la videollamada en Configuración → Integraciones" : "El administrador todavía no configuró la videollamada", true); return; }
  const room = [...crypto.getRandomValues(new Uint8Array(10))].map((b) => b.toString(16).padStart(2, "0")).join("");
  const c = CHAT.chan, payload = { body: `📞 ${S.user.username} inició una videollamada [[call:${room}]]` };
  if (c.t === "dm") payload.to = c.id; else if (c.t === "project") payload.project_id = c.id;
  const m = await api("POST", `/api/teams/${S.team.id}/chat`, payload);
  chatAdd(m); openCall(room);
});
// invitar a alguien de afuera (proveedor, cliente): sala propia, código único con vencimiento, copiar o mandar por email
const inviteModal = guard(async () => {
  if (!callItem()) { toast(can("admin") ? "Primero configurá la videollamada en Configuración → Integraciones" : "El administrador todavía no configuró la videollamada", true); return; }
  document.getElementById("invite-modal")?.remove();
  const label = h("input", { placeholder: "Para quién (opcional): Proveedor X, cliente…", maxLength: 60 });
  const hours = h("select", {}, [["1", "1 hora"], ["4", "4 horas"], ["24", "24 horas"], ["168", "7 días"]].map(([v, t]) => h("option", { value: v, selected: v === "24" }, "Vale " + t)));
  const out = h("div", {}), list = h("div", { class: "inv-list" }), box = h("div", { class: "overlay", id: "invite-modal", onmousedown: (e) => e.target === box && box.remove() });
  const link = (code) => `${location.origin}/j/${code}`;
  const showList = async () => {
    const r = await api("GET", `/api/teams/${S.team.id}/call-invites`);
    list.replaceChildren(r.items.length > 0 && h("h5", {}, "Invitaciones vigentes"), r.items.map((i) => h("div", { class: "cfg-line" },
      h("span", { class: "grow" }, `${i.label || "Sin nombre"} · ${i.by} · vence ${i.expires_at.slice(5, 16)} UTC · usada ${i.uses}×`),
      h("button", { class: "ghost", title: "Copiar el enlace", onclick: () => copy(link(i.code), "Enlace copiado") }, "📋"),
      h("button", { class: "ghost", title: "Anular: el enlace deja de funcionar", onclick: guard(async () => { await api("DELETE", `/api/teams/${S.team.id}/call-invites/${i.code}`); showList(); }) }, "🚫"))));
    return r;
  };
  const crear = guard(async () => {
    const r = await api("POST", `/api/teams/${S.team.id}/call-invites`, { label: label.value, hours: +hours.value });
    const url = link(r.code), mail = h("input", { type: "email", placeholder: "email de la persona invitada", maxLength: 120 });
    out.replaceChildren(h("p", { class: "hint" }, "Quien tenga este enlace entra a esta sala, sin cuenta y sin ver nada más de CrewHelm. Anulalo cuando termine."),
      h("input", { readonly: true, value: url, onfocus: (e) => e.target.select() }),
      h("div", { class: "row" }, h("button", { class: "primary", onclick: () => copy(url, "Enlace copiado") }, "📋 Copiar enlace"), h("button", { onclick: () => { openCall(r.room); box.remove(); } }, "📞 Entrar yo ahora")),
      r.email_available ? h("div", { class: "row" }, mail, h("button", { onclick: guard(async () => { await api("POST", `/api/teams/${S.team.id}/call-invites/${r.code}/email`, { to: mail.value, origin: location.origin }); toast("Invitación enviada por email"); }) }, "✉ Enviar por email"))
        : h("small", { class: "hint" }, "Este servidor no tiene el envío de emails configurado: copiá el enlace y mandalo por tu medio."));
    showList();
  });
  box.append(h("div", { class: "modal small" }, h("h3", {}, "🔗 Invitar a alguien de afuera"),
    h("p", { class: "hint" }, "Genera una sala nueva y un enlace único con vencimiento para un proveedor, cliente o cualquier persona que no es del equipo."),
    label, hours, h("div", { class: "row" }, h("button", { class: "primary", onclick: crear }, "Crear invitación")), out, list,
    h("div", { class: "row" }, h("button", { onclick: () => box.remove() }, "Cerrar"))));
  document.body.append(box); showList();
});
function drawDock() {
  let d = document.getElementById("integ-dock");
  if (!DOCK.tabs.length) { d?.remove(); return; }
  if (!d) { d = h("div", { id: "integ-dock", class: store.get("dockMax") === "1" ? "max" : "" }); document.body.append(d); }
  const frames = d.querySelector(".dock-frames") || h("div", { class: "dock-frames" });
  // los marcos se conservan entre redibujos para no cortar la llamada
  for (const t of DOCK.tabs) {
    let f = frames.querySelector(`[data-tab="${t.id}"]`);
    if (!f) { f = h("iframe", { "data-tab": t.id, src: t.src, allow: "camera; microphone; display-capture; fullscreen; autoplay; clipboard-write", sandbox: "allow-scripts allow-same-origin allow-forms allow-popups allow-modals allow-downloads", referrerpolicy: "no-referrer" }); frames.append(f); }
    f.hidden = t.id !== DOCK.active;
  }
  for (const f of [...frames.children]) if (!DOCK.tabs.some((t) => String(t.id) === f.dataset.tab)) f.remove();
  const cur = DOCK.tabs.find((t) => t.id === DOCK.active);
  const head = h("div", { class: "dock-head" },
    DOCK.tabs.map((t) => h("button", { class: "dock-tab" + (t.id === DOCK.active ? " on" : ""), onclick: () => { DOCK.active = t.id; drawDock(); } }, t.icon + " " + t.name,
      h("span", { class: "dock-x", title: "Cerrar esta pestaña (corta la llamada)", onclick: (e) => { e.stopPropagation(); DOCK.tabs = DOCK.tabs.filter((x) => x.id !== t.id); DOCK.active = DOCK.tabs[0]?.id ?? null; drawDock(); } }, "✕"))),
    h("span", { class: "spacer" }),
    cur && h("button", { class: "ghost", title: "Abrir en una ventana aparte", onclick: () => window.open(cur.src, "_blank", "noopener,noreferrer") }, "↗"),
    h("button", { class: "ghost", title: "Agrandar o achicar", onclick: () => { const on = d.classList.toggle("max"); store.set("dockMax", on ? "1" : "0"); } }, "⤢"),
    h("button", { class: d.classList.contains("min") ? "primary" : "ghost", title: d.classList.contains("min") ? "Volver a ver la llamada" : "Minimizar (la llamada sigue)", onclick: () => { d.classList.toggle("min"); drawDock(); } }, d.classList.contains("min") ? "▲ Volver a la llamada" : "–"));
  d.replaceChildren(head, frames);
}

// administración: lista editable y asistente paso a paso para la videollamada estándar
const integAdmin = guard(async () => {
  const r = await api("GET", `/api/teams/${S.team.id}/integrations`);
  let items = r.items.map((i) => ({ ...i }));
  const box = h("div", { class: "overlay", id: "integ-modal", onmousedown: (e) => e.target.id === "integ-modal" && box.remove() });
  const body = h("div", { class: "integ-body" });
  const save = guard(async () => {
    const res = await api("PUT", `/api/teams/${S.team.id}/integrations`, { items });
    S.integ = res; toast("Integraciones guardadas"); box.remove(); render();
  });
  const draw = () => body.replaceChildren(...[
    h("p", { class: "hint" }, "Herramientas externas que se abren desde la barra de arriba. En la dirección podés usar {sala} (código secreto del equipo), {equipo} y {usuario}."),
    !items.length && h("p", { class: "hint" }, "Todavía no hay ninguna. Probá el asistente de videollamada."),
    items.map((it, i) => h("div", { class: "integ-row" },
      h("input", { value: it.icon, maxLength: 4, class: "integ-ico", title: "Ícono (un emoji)", oninput: (e) => (it.icon = e.target.value) }),
      h("input", { value: it.name, maxLength: 40, placeholder: "Nombre", title: "Nombre del botón", oninput: (e) => (it.name = e.target.value) }),
      h("input", { value: it.url, maxLength: 400, placeholder: "https://…", class: "integ-url", title: "Dirección que se abre", oninput: (e) => (it.url = e.target.value) }),
      h("select", { title: "Cómo se abre: pestaña flotante dentro de la app (podés seguir usando el tablero) o ventana nueva", onchange: (e) => (it.mode = e.target.value) },
        h("option", { value: "panel", selected: it.mode === "panel" }, "Pestaña en la app"), h("option", { value: "tab", selected: it.mode !== "panel" }, "Ventana nueva")),
      h("button", { class: "ghost danger", title: "Quitar", onclick: () => { items.splice(i, 1); draw(); } }, "✕"))),
    h("div", { class: "row" },
      h("button", { onclick: () => { items.push({ name: "", icon: "🔌", url: "https://", mode: "tab" }); draw(); } }, "+ Agregar"),
      h("button", { onclick: () => wizard() }, "🧙 Asistente: videollamada"),
      h("span", { class: "spacer" }), h("button", { onclick: () => box.remove() }, "Cancelar"), h("button", { class: "primary", onclick: save }, "Guardar")),
  ].flat().filter(Boolean));
  function wizard() {
    const st = { step: 1, where: "internet", domain: "", email: "", name: "Videollamada", mode: "tab" };
    const own = () => st.where !== "internet";
    const host = () => {
      let h0 = st.domain.replace(/^https?:\/\//, "").replace(/\/+$/, "");
      if (st.where === "lan" && !/:\d+$/.test(h0)) h0 += ":8443";
      return h0;
    };
    const url = () => (st.where === "internet" ? "https://meet.jit.si" : "https://" + host()) + "/crewhelm-{sala}";
    const nsteps = () => (own() ? ["Dónde", "Datos", "Instalar", "Probar"] : ["Dónde", "Datos", "Probar"]);
    const testStep = () => nsteps().length;
    // un solo bloque para pegar en la máquina del Jitsi: baja, configura y levanta todo
    const script = () => {
      const lan = st.where === "lan", h0 = host(), ip = lan ? h0.split(":")[0] : "";
      return [
        "set -e",
        "# Requiere Docker con el plugin compose",
        "git clone --depth 1 https://github.com/jitsi/docker-jitsi-meet && cd docker-jitsi-meet",
        "cp env.example .env && ./gen-passwords.sh",
        "mkdir -p ~/.jitsi-meet-cfg/{web,transcripts,prosody/config,prosody/prosody-plugins-custom,jicofo,jvb}",
        "cat >> .env <<'EOF'",
        `PUBLIC_URL=https://${h0}`,
        lan ? `DOCKER_HOST_ADDRESS=${ip}` : "# DOCKER_HOST_ADDRESS se completa abajo con la IP pública",
        lan ? "HTTP_PORT=8000\nHTTPS_PORT=8443\nENABLE_LETSENCRYPT=0\nENABLE_AUTH=0" : `HTTP_PORT=80\nHTTPS_PORT=443\nENABLE_LETSENCRYPT=1\nLETSENCRYPT_DOMAIN=${h0}\nLETSENCRYPT_EMAIL=${st.email}\nENABLE_AUTH=1\nENABLE_GUESTS=1\nAUTH_TYPE=internal`,
        "EOF",
        ...(lan ? [] : ['echo "DOCKER_HOST_ADDRESS=$(curl -s https://ifconfig.me)" >> .env']),
        "docker compose up -d",
        ...(lan ? [] : ['PASS=$(openssl rand -hex 8)', 'sleep 20 && docker compose exec -T prosody prosodyctl --config /config/prosody.cfg.lua register crewhelm meet.jitsi "$PASS"', 'echo "Usuario para crear salas: crewhelm / $PASS"']),
      ].join("\n");
    };
    const dd = () => {
      const steps = nsteps();
      const head = h("div", { class: "wiz-steps" }, steps.map((x, i) => h("span", { class: "wiz-step" + (st.step === i + 1 ? " on" : st.step > i + 1 ? " ok" : "") }, `${i + 1}. ${x}`)));
      const opt = (v, mode, title, desc) => h("label", { class: "wiz-opt" }, h("input", { type: "radio", name: "w", checked: st.where === v, onchange: () => { st.where = v; st.mode = mode; } }), h("div", {}, h("b", {}, title), h("small", {}, desc)));
      let main;
      if (st.step === 1) main = [
        h("p", {}, "¿Dónde va a correr la videollamada?"),
        opt("internet", "tab", "🌐 Jitsi público de internet (lo más rápido)", "Usa meet.jit.si, gratis y sin instalar nada: listo en un minuto. El audio y el video pasan por servidores de Jitsi; la sala tiene un código secreto que no se puede adivinar."),
        opt("lan", "panel", "🏢 Servidor propio, solo en la red interna", "Instalás un Jitsi en una máquina de tu red. Nada sale a internet y funciona sin conexión. Te damos el bloque listo para pegar."),
        opt("net", "panel", "🔒 Servidor propio, abierto a internet (equipos remotos)", "Tu propio Jitsi con dominio y certificado real, con usuario para crear salas. Es lo que conviene si el equipo trabaja remoto y querés control. Te damos el bloque listo para pegar."),
      ];
      if (st.step === 2) { const dom = h("input", { value: st.domain, placeholder: st.where === "lan" ? "192.168.1.50" : "meet.tuempresa.com", maxLength: 120, oninput: (e) => (st.domain = e.target.value.trim()) }); main = [
        own() && h("label", {}, st.where === "lan" ? "IP (o nombre) de la máquina donde va a correr" : "Dominio público (ya apuntando a esa máquina)", dom,
          h("small", { class: "hint" }, st.where === "lan" ? "Ejemplo: 192.168.1.50. Usa el puerto 8443." : "Ejemplo: meet.tuempresa.com. Usa los puertos 80 y 443.")),
        st.where === "net" && h("label", {}, "Tu email (para el certificado gratuito Let's Encrypt)", h("input", { type: "email", value: st.email, maxLength: 120, oninput: (e) => (st.email = e.target.value.trim()) })),
        h("label", {}, "Nombre del botón", h("input", { value: st.name, maxLength: 40, oninput: (e) => (st.name = e.target.value) })),
        st.where === "internet" && h("p", { class: "hint" }, "Ojo: el Jitsi público corta las llamadas a los 5 minutos cuando va incrustado en otra página, y a veces pide que quien abre la sala inicie sesión. Por eso se abre en «Ventana nueva»."),
        h("label", {}, "¿Cómo se abre?", h("select", { onchange: (e) => (st.mode = e.target.value) },
          h("option", { value: "panel", selected: st.mode === "panel" }, "Pestaña flotante dentro de la app (seguís viendo el tablero)"), h("option", { value: "tab", selected: st.mode === "tab" }, "Ventana nueva del navegador"))),
      ]; }
      if (own() && st.step === 3) { const ports = st.where === "lan" ? "8443/tcp y 10000/udp" : "80/tcp, 443/tcp y 10000/udp"; main = [
        h("p", {}, "En la máquina del Jitsi (Linux con Docker), pegá este bloque. Baja, configura y levanta todo:"),
        h("pre", { class: "wiz-script" }, script()),
        h("button", { onclick: () => copy(script(), "Bloque copiado") }, "📋 Copiar bloque"),
        h("ul", { class: "wiz-list" },
          h("li", {}, "Abrí en el firewall" + (st.where === "net" ? " y en el router" : "") + ": " + ports + "."),
          st.where === "lan" ? h("li", {}, `Cada persona abre una vez https://${host()} y acepta el aviso del certificado (es propio). Sin eso el navegador no deja usar cámara ni micrófono.`)
            : h("li", {}, "El bloque muestra al final un usuario y clave para crear salas: guardalos. Quien solo se une no necesita cuenta."),
          h("li", {}, "Cuando termine, seguí con «Probar».")),
      ]; }
      if (st.step === testStep()) main = [
        h("p", {}, "Así quedaría el botón:"), h("p", { class: "wiz-url" }, "📹 " + st.name + " → " + url()),
        h("p", { class: "hint" }, "La primera persona que entra crea la sala; el resto entra con el mismo botón. El navegador va a pedir permiso de cámara y micrófono."),
        h("button", { onclick: () => window.open(url().replace("{sala}", encodeURIComponent(r.room)), "_blank", "noopener,noreferrer") }, "▶ Probar ahora (ventana nueva)"),
      ];
      const next = guard(async () => {
        if (st.step === 2) {
          if (own() && !/^[\w-]+(\.[\w-]+)*(:\d{2,5})?$/i.test(host())) throw new Error(st.where === "lan" ? "escribí la IP o el nombre de la máquina, por ejemplo 192.168.1.50" : "escribí el dominio, por ejemplo meet.tuempresa.com");
          if (st.where === "net" && !/^\S+@\S+\.\S+$/.test(st.email)) throw new Error("falta un email válido para el certificado");
          if (!st.name.trim()) throw new Error("falta el nombre");
        }
        if (st.step < testStep()) { st.step++; dd(); return; }
        items = items.filter((i) => !i.url.includes("{sala}") || !/jit/i.test(i.name + i.url)).concat({ name: st.name.trim(), icon: "📹", url: url(), mode: st.mode });
        wz.remove(); draw();
      });
      wz.querySelector(".modal").replaceChildren(h("h3", {}, "🧙 Videollamada del equipo"), head, h("div", { class: "wiz-main" }, main),
        h("div", { class: "row" }, st.step > 1 && h("button", { onclick: () => { st.step--; dd(); } }, "← Atrás"), h("span", { class: "spacer" }), h("button", { onclick: () => wz.remove() }, "Cancelar"),
          h("button", { class: "primary", onclick: next }, st.step < testStep() ? "Siguiente →" : "Agregar a la lista")));
    };
    const wz = h("div", { class: "overlay", style: "z-index:10000", onmousedown: (e) => e.target === wz && wz.remove() }, h("div", { class: "modal small wiz" }));
    document.body.append(wz); dd();
  }
  box.append(h("div", { class: "modal wide" }, h("h3", {}, "🔌 Integraciones del equipo"), body)); document.body.append(box); draw();
});

const openSettings = guard(async () => {
  document.getElementById("settings-modal")?.remove();
  S.me = await api("GET", "/api/me");
  const me = S.me, close = () => document.getElementById("settings-modal")?.remove();
  const go = (fn) => () => { close(); fn(); };
  const sec = (title, ...kids) => kids;
  const line = (label, ...ctl) => {
    const [text, key] = Array.isArray(label) ? label : [label, null];
    const help = key && CFG_HELP[key] ? CFG_HELP[key][1] : "";
    const el = h("div", { class: "cfg-item" },
      h("div", { class: "cfg-line" }, h("span", { class: "grow" }, text),
        key && h("button", { class: "ghost help", title: "Explicación completa", onclick: () => (key === "google" ? googleHelp() : cfgHelp(key)) }, "?"), ...ctl),
      help && h("small", { class: "cfg-help" }, help.length > 220 ? help.slice(0, 217) + "…" : help));
    el.dataset.key = key || "";
    return el;
  };

  const view = h("select", { onchange: (e) => { store.set("view", e.target.value); S.view = e.target.value; render(); } },
    VIEWS.map(([v, l]) => h("option", { value: v, selected: S.view === v }, l)));
  const server = h("input", { value: S.server, disabled: IS_WEB, maxLength: 200 });
  const tgState = !me.telegram_available ? "el servidor no tiene el bot configurado" : me.telegram_linked ? "✓ vinculado" : "sin vincular";
  const gc = await api("GET", "/api/google").catch(() => ({ available: false }));
  const gh = await api("GET", "/api/github").catch(() => ({ available: false }));
  const gcState = !gc.available ? "el servidor no tiene Google configurado" : gc.linked ? `✓ conectado (${gc.events} tareas con fecha en tu calendario)` : "sin conectar";
  const emailState = !me.email ? "sin cargar" : me.email_verified ? `✓ ${me.email}` : `${me.email} (sin verificar)`;

  // cómo llegan los avisos por Telegram y email: resumen a una hora fija (lo normal) o uno por uno
  const np = S.notifyPrefs || { mode: "digest", hour: 9, every_h: 0 };
  if (!S.notifyPrefs) api("GET", "/api/me/notify-prefs").then((v) => { S.notifyPrefs = v; if (document.getElementById("settings-modal")) openSettings(); }).catch(() => {});
  const muted = np.mute_until && new Date(np.mute_until) > new Date();
  const saveNp = guard(async (mute) => {
    const v = { mode: npMode.value, hour: +npHour.value, every_h: +npEvery.value, tz_offset: -new Date().getTimezoneOffset(), mute_hours: mute === undefined ? (muted ? Math.max(1, Math.ceil((new Date(np.mute_until) - new Date()) / 36e5)) : 0) : mute };
    S.notifyPrefs = await api("PUT", "/api/me/notify-prefs", v).then(() => api("GET", "/api/me/notify-prefs"));
    toast(v.mute_hours ? `Avisos pausados ${v.mute_hours} h` : v.mode === "silent" ? "Modo silencioso: Telegram y email no te molestan" : v.mode === "digest" ? (v.every_h ? `Resumen cada ${v.every_h} h` : `Resumen diario a las ${String(v.hour).padStart(2, "0")}:00`) : "Avisos uno por uno");
    if (document.getElementById("settings-modal")) openSettings();
  });
  const opt = (val, cur, txt) => h("option", { value: val, selected: val === cur }, txt);
  const npMode = h("select", { title: "Cómo te llegan los avisos por Telegram y email (la bandeja de la app siempre se llena)", onchange: () => saveNp() },
    opt("digest", np.mode, "Resumen"), opt("instant", np.mode, "Uno por uno"), opt("silent", np.mode, "🔕 Silencioso"));
  const npEvery = h("select", { disabled: np.mode !== "digest", title: "Cada cuánto sale el resumen", onchange: () => saveNp() },
    opt(0, np.every_h || 0, "una vez por día"), ...[1, 2, 3, 6, 12].map((n) => opt(n, np.every_h, `cada ${n} h`)));
  const npHour = h("select", { disabled: np.mode !== "digest" || !!np.every_h, title: "Hora del resumen diario", onchange: () => saveNp() },
    Array.from({ length: 24 }, (_, i) => opt(i, np.hour, String(i).padStart(2, "0") + ":00")));
  const npPause = h("select", { title: "Pausa temporal: no llega nada por Telegram ni email hasta que pase el tiempo", onchange: (e) => { const v = +e.target.value; if (v >= 0) saveNp(v); } },
    h("option", { value: -1, selected: true }, muted ? "⏸ hasta " + new Date(np.mute_until).toLocaleString([], { weekday: "short", hour: "2-digit", minute: "2-digit" }) : "⏸ Pausar…"),
    ...(muted ? [h("option", { value: 0 }, "▶ Reanudar ahora")] : []),
    h("option", { value: 1 }, "1 hora"), h("option", { value: 4 }, "4 horas"), h("option", { value: 12 }, "12 horas"), h("option", { value: 24 }, "1 día"), h("option", { value: 168 }, "1 semana"));

  const items = [
      S.team && sec("", line(["Informes automáticos del equipo (resumen diario o semanal)", "reports"], h("button", { onclick: go(openReports) }, "Configurar"))),
      sec("Cuenta",
        line([`Usuario: ${me.username}`, "user"]),
        line(["Archivos permitidos en el chat", "chatExts"], h("button", { onclick: () => { close(); chooseChatExts(); } }, "Elegir tipos")),
        line(["Tu avatar", "avatar"], avatar(me.username), h("button", { onclick: () => { close(); chooseAvatar(); } }, "Elegir")),
        line([`Email para avisos: ${emailState}`, "email"], h("button", { onclick: go(openProfile) }, "Configurar")),
        line([`Telegram: ${tgState}`, "telegram"], me.telegram_available && (me.telegram_linked
          ? h("button", { class: "danger", onclick: guard(async () => { await api("DELETE", "/api/me/telegram"); close(); openSettings(); }) }, "Desvincular")
          : h("button", { onclick: () => { close(); linkTelegram(); } }, "Vincular"))),
        line(["Cómo recibís los avisos (Telegram y email)", "digest"], npMode, npEvery, npHour, npPause),
        (can("admin") || gh.linked) && line([`GitHub: ${!gh.available ? "el servidor no tiene GitHub configurado" : gh.linked ? "✓ conectado como @" + gh.login : "sin conectar"}`, "github"], gh.available && (gh.linked
          ? h("button", { class: "danger", onclick: guard(async () => { if (!(await ask("¿Desconectar tu cuenta de GitHub?"))) return; await api("DELETE", "/api/github"); close(); openSettings(); }) }, "Desconectar")
          : h("button", { onclick: () => { close(); linkGithub(); } }, "Conectar"))),
        (gc.available || gc.linked) && line([`Google Calendar: ${gcState}`, "google"], gc.available && (gc.linked
          ? [h("button", { onclick: guard(async () => { const r = await api("POST", "/api/google/sync"); toast(`Calendario al día (${r.created} nuevos, ${r.updated} cambiados, ${r.deleted} quitados)`); close(); openSettings(); }) }, "Sincronizar"),
            h("button", { class: "danger", onclick: guard(async () => { if (!(await ask("¿Desconectar y quitar de tu calendario los eventos creados por la app?"))) return; await api("DELETE", "/api/google"); close(); openSettings(); }) }, "Desconectar")]
          : h("button", { onclick: guard(async () => { const l = await api("POST", "/api/google/link"); window.open(l.url, "_blank", "noopener"); toast("Autorizá en la pestaña de Google y después volvé a abrir Configuración"); }) }, "Conectar"))),
        line(["Bandeja de novedades", "inbox"], h("button", { onclick: go(() => loadInbox(false)) }, S.unread ? `Abrir (${S.unread})` : "Abrir")),
        line(["Cerrar sesión en este dispositivo", "logout"], h("button", { onclick: () => { close(); logout(); } }, "Salir")),
        line(["Cerrar sesión en todos los dispositivos", "logoutAll"], h("button", { class: "danger", onclick: () => { close(); logoutAll(); } }, "Cerrar todas"))),
      S.team && sec("Proyecto",
        line([S.project ? `Proyecto actual: ${S.project.name}` + (S.project.repo ? ` · 🔗 ${S.project.repo}` : " · sin repositorio") + (S.project.work_dir ? ` · 📁 ${S.project.work_dir}` : "") : "Sin proyectos", "project"],
          h("button", { onclick: go(createProject) }, "Nuevo proyecto"),
          S.project && can("admin") && h("button", { onclick: go(editProject), title: "Nombre, carpeta de trabajo y repositorio" }, "⚙ Datos del proyecto"),
          S.project && can("member") && h("button", { onclick: go(githubProject), title: "Traer los issues abiertos del repositorio como tareas" }, "🐙 Traer issues")),
        S.project && can("member") && line(["Usar como plantilla (copia tareas, etiquetas y dependencias)", "duplicate"], h("button", { onclick: go(duplicateProject) }, "Duplicar")),
        S.project && line(["Exportar las tareas", "export"], h("button", { onclick: guard(() => saveFile(`/api/projects/${S.project.id}/export?format=csv`, `${S.project.name}.csv`)) }, "CSV"),
          h("button", { onclick: guard(() => saveFile(`/api/projects/${S.project.id}/export?format=json`, `${S.project.name}.json`)) }, "JSON"))),
      S.team && sec(`Equipo «${S.team.name}»`,
        line([`Tu rol: ${ROLES.find(([r]) => r === myRole())?.[1]}`, "role"]),
        can("member") && line(["Enviar un mensaje al equipo", "message"], h("button", { onclick: go(sendTeamMessage) }, "Escribir")),
        line(["Miembros, roles e invitaciones", "members"], h("button", { onclick: go(openTeam) }, "Administrar")),
        can("admin") && line(["Agentes de IA: que trabajen solos (tm-agent)", "agents"], h("button", { onclick: go(agentsGuide) }, "Ver instructivo")),
        can("admin") && (S.bots || []).length > 0 && line(["Permisos de cada IA: perfil confiable, autorización total y listas de comandos", "cmds"], (() => {
          const sel = h("select", {}, (S.bots || []).map((b) => h("option", { value: b.id }, b.username + (b.full_access ? " 🔓" : ""))));
          const pick = () => (S.bots || []).find((b) => String(b.id) === sel.value);
          const btn = (label, title, fn, cls) => h("button", { class: cls || "", title, onclick: () => { const b = pick(); if (b) { close(); fn(b); } } }, label);
          return h("div", { class: "row wrap" }, sel,
            btn("⚡ Perfil confiable", "Autorización total con una lista corta de cosas peligrosas prohibidas: la IA deja de pedir permisos", trustProfile, "primary"),
            btn("🔒 Autorización total", "Activar o apagar: la IA puede todo salvo lo prohibido", toggleFull),
            btn("📋 Comandos habituales", "Lo que la IA pidió antes y un catálogo para permitir", commandsDialog),
            btn("✅ Permitidos", "Lista de comandos permitidos de la IA", editAllow),
            btn("🚫 Prohibidos", "Lista de comandos prohibidos de la IA", editDeny),
            btn("💬 Chat", "Que esta IA conteste en el chat del equipo, con topes de gasto", botChatModal),
            btn("Ficha", "Abrir la ficha de la IA", botModal));
        })()),
        line(["Unirme a otro equipo con un código", "join"], h("button", { onclick: go(joinWithCode) }, "Unirme")),
        line(["Crear un equipo nuevo", "newTeam"], h("button", { onclick: go(createTeam) }, "Crear"))),
      S.team && sec("Perfil de uso",
        line([`Perfil actual: ${S.profile ? PROFILES[S.profile].name : "sin elegir"}`, "perfil"],
          can("admin") && (() => {
            const sel = h("select", {}, h("option", { value: "" }, "Elegir…"), Object.entries(PROFILES).map(([id, p]) => h("option", { value: id, selected: S.profile === id }, p.name)));
            sel.onchange = guard(async () => {
              if (!sel.value) return;
              await api("PUT", `/api/teams/${S.team.id}/profile`, { profile: sel.value }); applyProfile(sel.value);
              if (PROFILES[sel.value].view) S.view = PROFILES[sel.value].view;
              close(); render(); toast(`Perfil «${PROFILES[sel.value].name}» aplicado. ${PROFILES[sel.value].desc}`);
            });
            return sel;
          })()),
        S.isExample
          ? line(["Este es un equipo de ejemplo: borrarlo deja todo limpio. Tu equipo real no se toca", "perfil"],
              h("button", { onclick: guard(async () => {
                const real = S.teams.find((t) => t.id !== S.team.id && !t.is_example && !t.name.startsWith("Ejemplo · "));
                if (real) { close(); await selectTeam(real); toast("Volviste a tu equipo"); }
                else toast("No tenés otro equipo todavía: creá uno desde «Crear un equipo nuevo»", true);
              }) }, "Volver a mi equipo"),
              can("owner") && h("button", { class: "danger", onclick: guard(async () => {
                if (!(await ask("Se borra este equipo de ejemplo con sus proyectos y tareas. Tus otros equipos no se tocan. ¿Eliminar el ejemplo?", "Eliminar ejemplo"))) return;
                const id = S.team.id;
                await api("DELETE", `/api/examples/${id}`);
                S.teams = S.teams.filter((t) => t.id !== id); close();
                await selectTeam(S.teams[0] || null); toast("Ejemplo eliminado");
              }) }, "Eliminar ejemplo"))
          : can("member") && line(["Ver un equipo de ejemplo ya armado (separado del tuyo, se borra con un botón)", "perfil"], (() => {
              const sel = h("select", {}, Object.entries(PROFILES).map(([id, p]) => h("option", { value: id, selected: S.profile === id }, p.name)));
              return [sel, h("button", { onclick: guard(async () => {
                const r = await api("POST", "/api/examples", { profile: sel.value });
                S.teams = await api("GET", "/api/teams"); close(); await selectTeam(S.teams.find((t) => t.id === r.team.id) || S.teams[S.teams.length - 1]);
                toast("Equipo de ejemplo creado. Para volver al tuyo: Configuración → Equipo y proyecto → «Volver a mi equipo»");
              }) }, "Crear ejemplo")];
            })())),
      sec("Licencia",
        S.me?.server_role === "admin" && line(["Comandos permitidos y prohibidos para todas las IAs", "cmds"], h("button", { onclick: guard(async () => {
          const g = await api("GET", "/api/commands");
          if (!g.can_edit) { toast("Solo la primera persona registrada edita las listas generales. Cada IA tiene la suya en su ficha."); return; }
          const f = await form("Comandos de todas las IAs", [
            { name: "allow", label: "✅ Permitidos para todas (uno por línea, ej. git clone · cargo build · Bash(npm test:*))", type: "textarea", value: g.allow, max: 4000 },
            { name: "deny", label: "🚫 Prohibidos para todas (uno por línea, ej. Bash(rm:*)). Lo prohibido siempre gana.", type: "textarea", value: g.deny, max: 4000 },
            { name: "net_allow", label: "🌐 Destinos de red para las IAs con sandbox, además de los de fábrica (Anthropic, OpenAI, GitHub, crates.io, npm, PyPI…). Uno por línea, solo dominios (ej. api.miempresa.com)", type: "textarea", value: g.net_allow, max: 4000 }], "Guardar"); if (!f) return;
          await api("PUT", "/api/commands", { allow: f.allow, deny: f.deny, net_allow: f.net_allow }); toast("Listas guardadas");
        }) }, "Editar")),
        line(["Uso gratis (AGPL-3.0) o licencia comercial para empresas", "license"], h("button", { onclick: go(openLicense) }, "🏢 Licencia")),),
      sec("Preferencias",
        line(["Idioma / Language", "lang"], h("select", { onchange: (e) => tr.set(e.target.value) },
          Object.entries(tr.langs).map(([k, n]) => h("option", { value: k, selected: tr.lang === k }, n)))),
        line(["Vista inicial", "view"], view),
        line(["Dónde van los botones (Programadas, Consumo, GitHub, Proyecto…)", "bars"], (() => {
          const sel = h("select", { onchange: (e) => { store.set("barActs", e.target.value); render(); } },
            [["top", "Arriba"], ["bottom", "Abajo"]].map(([v, l]) => h("option", { value: v, selected: (store.get("barActs") === "bottom" ? "bottom" : "top") === v }, l)));
          return sel;
        })()),
        line(["Dónde van Tablero / Calendario / Gantt y el buscador", "bars"], (() => {
          const sel = h("select", { onchange: (e) => { store.set("barViews", e.target.value); render(); } },
            [["top", "Arriba"], ["bottom", "Abajo"]].map(([v, l]) => h("option", { value: v, selected: (store.get("barViews") === "bottom" ? "bottom" : "top") === v }, l)));
          return sel;
        })()),
        line(["Abrir herdr al entrar a la terminal de una tarea", "herdr"], (() => {
          const cb = h("input", { type: "checkbox", checked: store.get("autoHerdr") !== "0" });
          cb.onchange = () => { store.set("autoHerdr", cb.checked ? "1" : "0"); api("PUT", "/api/me/term-prefs", { autoHerdr: cb.checked }).catch(() => {}); toast(cb.checked ? "herdr automático: sí" : "herdr automático: no"); };
          return cb;
        })()),
        line(["Cuánto queda visible el aviso emergente", "toastSecs"], (() => {
          const cur = parseFloat(store.get("toastSecs")); const v = cur >= 0 ? cur : 6;
          return h("select", { onchange: (e) => { store.set("toastSecs", e.target.value); toast("Así de largo se verá este aviso"); } },
            [[3.5, "3 segundos"], [6, "6 segundos (normal)"], [12, "12 segundos"], [30, "30 segundos"], [0, "Hasta que lo toque"]].map(([n, l]) => h("option", { value: n, selected: n === v }, l)));
        })()),
        line(["Cuánto queda visible la burbuja del chat", "bubbleSecs"], (() => {
          const cur = parseFloat(store.get("bubbleSecs")); const v = cur >= 0 ? cur : 10;
          return h("select", { onchange: (e) => { store.set("bubbleSecs", e.target.value); toast("Así de larga se verá la burbuja"); } },
            [[5, "5 segundos"], [10, "10 segundos (normal)"], [20, "20 segundos"], [60, "1 minuto"], [0, "Hasta que la toque"]].map(([n, l]) => h("option", { value: n, selected: n === v }, l)));
        })()),
        line(["Servidor", "server"], server),
        IS_WEB ? h("small", {}, "Estás usando la web del propio servidor: la dirección no se cambia desde acá.")
          : h("div", { class: "row" }, h("button", { onclick: guard(async () => {
            const url = server.value.trim().replace(/\/$/, ""); if (!/^https?:\/\//.test(url)) throw new Error("la dirección debe empezar con http:// o https://");
            S.server = url; store.set("server", url); toast("Servidor guardado: volvé a iniciar sesión"); close(); logout();
          }) }, "Guardar servidor y reiniciar sesión"))),
      can("admin") && S.team && sec("Integraciones", line(["Videollamada y otras herramientas del equipo", "integrations"], h("button", { onclick: go(integAdmin) }, "Administrar"))),
      ["cto", "admin"].includes(S.me?.server_role) && sec("Procesos", procPanel()),
      S.me?.server_role === "admin" && sec("Roles y límites del servidor", rolesPanel(), limitsPanel()),
      sec("Ayuda", line(["Manual con buscador e índice por temas", "manual"], h("button", { onclick: go(() => openManual()) }, "Abrir el manual")),
        line(["Atajos de teclado (también con ?)", "shortcuts"], h("button", { onclick: go(helpModal) }, "Ver atajos")),
        line(["Guía de inicio paso a paso", "guide"], h("button", { onclick: go(openWizard) }, "🧭 Guía de inicio"))),
  ].flat(Infinity).filter(Boolean);
  // solapas: cada línea se ubica por su clave; lo que no tiene clave (botones sueltos) queda con la línea anterior
  const TABS = [
    ["cuenta", "👤 Cuenta", ["user", "inbox", "logout", "logoutAll"]],
    ["conexiones", "🔔 Avisos y conexiones", ["email", "telegram", "digest", "github", "google", "reports"]],
    ["equipo", "👥 Equipo y proyecto", ["role", "message", "members", "join", "newTeam", "perfil", "project", "duplicate", "export"]],
    ["ias", "🤖 IAs", ["agents", "cmds"]],
    ["integraciones", "🔌 Integraciones", ["integrations"]],
    ["procesos", "🖥 Procesos", ["procesos"]],
    ["srvroles", "🛡 Roles y límites", ["srvroles", "srvlimits"]],
    ["licencia", "🏢 Licencia", ["license"]],
    ["prefs", "⚙ Preferencias y ayuda", ["lang", "view", "bars", "herdr", "server", "manual", "shortcuts", "guide"]],
  ];
  const tabOf = new Map(TABS.flatMap(([id, , keys]) => keys.map((k) => [k, id])));
  const groups = Object.fromEntries(TABS.map(([id]) => [id, []]));
  let curTab = "cuenta";
  for (const it of items) { const t = tabOf.get(it.dataset?.key || ""); if (t) curTab = t; groups[curTab].push(it); }
  let active = store.get("cfgTab"); if (!groups[active]?.length) active = "cuenta";
  const panel = h("div", { class: "cfg-panel" });
  const nav = h("div", { class: "cfg-nav" });
  const show = (id) => {
    active = id; store.set("cfgTab", id);
    panel.replaceChildren(h("h2", {}, TABS.find((x) => x[0] === id)[1]), ...groups[id]);
    nav.replaceChildren(...TABS.filter(([tid]) => groups[tid].length).map(([tid, label]) => h("button", { class: "cfg-tab" + (tid === id ? " active" : ""), onclick: () => show(tid) }, label)));
  };
  show(active);
  document.body.append(h("div", { class: "overlay", id: "settings-modal", onmousedown: (e) => e.target.id === "settings-modal" && close() },
    h("div", { class: "modal settings" },
      h("div", { class: "cfg-head" }, h("h2", {}, "Configuración"), h("span", { class: "spacer" }), h("button", { class: "primary", onclick: close }, "Listo")),
      h("div", { class: "cfg-body" }, nav, panel))));
});

// ---------- ayuda con buscador y atajos de teclado ----------
const SHORTCUTS = [
  ["?", "Abrir esta ayuda"], ["/", "Ir al buscador"], ["n", "Nueva tarea (en Por hacer)"],
  ["1", "Vista Tablero"], ["2", "Vista Calendario"], ["3", "Vista Gantt"], ["4", "Vista Pizarra"], ["5", "Vista Actividad"], ["@usuario", "En un comentario: menciona y avisa a esa persona"],
  ["Esc", "Cerrar ventana"], [",", "Configuración"], ["Alt+K c", "Configuración"], ["Alt+K s", "Enviar un mensaje al equipo"], ["Alt+K g", "Vincular Telegram"],
  ["Alt+K m", "Equipo: miembros y agregar con búsqueda"], ["Alt+K i", "Equipo: generar código de invitación"],
  ["Alt+K j", "Unirse a un equipo con código"], ["Alt+K t", "Crear equipo"], ["Alt+K p", "Crear proyecto"],
  ["Alt+K e", "Mi email para recibir avisos"], ["Alt+K n", "Bandeja de novedades y mis tareas"],
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
  if ((e.altKey && e.code === "KeyK") || (e.ctrlKey && e.key.toLowerCase() === "t")) { e.preventDefault(); S.prefix = true; toast("Alt+K ▸ m miembros · i invitar · j unirse · t panel del equipo · p proyecto · e email · n bandeja · s mensaje · g Telegram · c config"); return; }
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

// ---------- guía de inicio: pasos con ilustraciones para dejar todo andando ----------
const WIZ_ART = (() => {
  const wrap = (body) => `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 560 190" font-family="system-ui,sans-serif">
    <defs><linearGradient id="g1" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#2a2f4a"/><stop offset="1" stop-color="#171826"/></linearGradient>
    <radialGradient id="glow" cx=".5" cy=".5" r=".5"><stop offset="0" stop-color="#7aa2f7" stop-opacity=".35"/><stop offset="1" stop-color="#7aa2f7" stop-opacity="0"/></radialGradient></defs>
    <rect width="560" height="190" rx="16" fill="url(#g1)"/>${body}</svg>`;
  const card = (x, y, c, w = 70) => `<rect x="${x}" y="${y}" width="${w}" height="22" rx="6" fill="#24283b"/><rect x="${x}" y="${y}" width="4" height="22" rx="2" fill="${c}"/><rect x="${x + 12}" y="${y + 8}" width="${w - 28}" height="5" rx="2.5" fill="#414868"/>`;
  const col = (x, c, n, cards) => `<rect x="${x}" y="34" width="86" height="136" rx="10" fill="#1a1b26" stroke="#2f334d"/><circle cx="${x + 14}" cy="48" r="4" fill="${c}"/><rect x="${x + 24}" y="45" width="34" height="6" rx="3" fill="#565f89"/>${cards.map((y) => card(x + 8, y, c, 70)).join("")}`;
  const spokes = [0, 45, 90, 135].map((a) => `<g transform="rotate(${a} 440 95)"><line x1="440" y1="38" x2="440" y2="152" stroke="#e0af68" stroke-width="7" stroke-linecap="round"/><circle cx="440" cy="34" r="8" fill="#bb9af7"/><circle cx="440" cy="156" r="8" fill="#bb9af7"/></g>`).join("");
  const av = (x, y, c, t) => `<circle cx="${x}" cy="${y}" r="26" fill="${c}"/><text x="${x}" y="${y + 8}" text-anchor="middle" font-size="22" font-weight="700" fill="#15161e">${t}</text>`;
  const robot = (x, y, s = 1) => `<g transform="translate(${x} ${y}) scale(${s})"><line x1="0" y1="-38" x2="0" y2="-26" stroke="#7dcfff" stroke-width="3"/><circle cx="0" cy="-41" r="5" fill="#f7768e"/><rect x="-30" y="-26" width="60" height="48" rx="14" fill="#7dcfff"/><rect x="-22" y="-16" width="44" height="26" rx="9" fill="#1a1b26"/><circle cx="-10" cy="-3" r="5" fill="#9ece6a"/><circle cx="10" cy="-3" r="5" fill="#9ece6a"/><rect x="-10" y="14" width="20" height="3" rx="1.5" fill="#1a1b26"/></g>`;
  const chip = (x, y, w, t, c) => `<rect x="${x}" y="${y}" width="${w}" height="26" rx="13" fill="#1a1b26" stroke="${c}"/><text x="${x + w / 2}" y="${y + 17}" text-anchor="middle" font-size="12" fill="${c}">${t}</text>`;
  return {
    welcome: wrap(`<circle cx="440" cy="95" r="95" fill="url(#glow)"/>${col(30, "#7aa2f7", "", [62, 90])}${col(126, "#e0af68", "", [62])}${col(222, "#9ece6a", "", [62, 90, 118])}
      ${spokes}<circle cx="440" cy="95" r="46" fill="none" stroke="#e0af68" stroke-width="9"/><circle cx="440" cy="95" r="15" fill="#e0af68"/><circle cx="440" cy="95" r="6" fill="#1a1b26"/>${robot(525, 150, .45)}`),
    team: wrap(`<g stroke="#414868" stroke-width="2" stroke-dasharray="5 5" fill="none"><path d="M120 95 C170 60 210 60 250 85"/><path d="M200 140 C230 120 250 110 270 100"/><path d="M440 95 C400 60 360 60 320 85"/><path d="M360 140 C330 120 310 110 295 100"/></g>
      <rect x="235" y="68" width="90" height="56" rx="14" fill="#24283b" stroke="#7aa2f7" stroke-width="2"/><text x="280" y="102" text-anchor="middle" font-size="15" font-weight="700" fill="#7aa2f7">Equipo</text>
      ${av(90, 90, "#7aa2f7", "A")}${av(190, 145, "#bb9af7", "M")}${av(470, 90, "#9ece6a", "L")}${robot(370, 140, .8)}${chip(60, 30, 80, "dueño", "#e0af68")}${chip(420, 30, 80, "miembro", "#9ece6a")}`),
    project: wrap(`<path d="M50 62 h56 l14 16 h90 a10 10 0 0 1 10 10 v70 a10 10 0 0 1 -10 10 h-160 a10 10 0 0 1 -10 -10 v-86 a10 10 0 0 1 10 -10z" fill="#e0af68"/><rect x="40" y="86" width="180" height="72" rx="10" fill="#f0c674"/>
      <text x="130" y="128" text-anchor="middle" font-size="14" font-weight="700" fill="#3b2f12">~/proyectos/app</text>
      <g stroke="#7aa2f7" stroke-width="4" fill="none"><path d="M300 150 H500"/><path d="M340 150 C360 150 360 70 390 70 H460"/></g>
      ${[310, 360, 420, 480].map((x) => `<circle cx="${x}" cy="150" r="9" fill="#9ece6a"/>`).join("")}${[395, 450].map((x) => `<circle cx="${x}" cy="70" r="9" fill="#bb9af7"/>`).join("")}${chip(300, 28, 120, "github.com/tu/repo", "#7dcfff")}`),
    ia: wrap(`<circle cx="280" cy="95" r="90" fill="url(#glow)"/>${robot(280, 98, 1.5)}
      <rect x="60" y="40" width="130" height="34" rx="12" fill="#24283b" stroke="#7aa2f7"/><text x="125" y="62" text-anchor="middle" font-size="13" fill="#c0caf5" font-family="monospace">git clone …</text>
      <rect x="370" y="52" width="130" height="34" rx="12" fill="#24283b" stroke="#9ece6a"/><text x="435" y="74" text-anchor="middle" font-size="13" fill="#c0caf5" font-family="monospace">cargo test ✓</text>
      ${chip(40, 150, 80, "Claude", "#e0af68")}${chip(130, 150, 80, "Gemini", "#7aa2f7")}${chip(350, 150, 80, "OpenAI", "#9ece6a")}${chip(440, 150, 80, "Local", "#bb9af7")}`),
    listo: wrap(`<circle cx="280" cy="92" r="88" fill="url(#glow)"/><circle cx="280" cy="92" r="52" fill="#9ece6a"/><path d="M254 94 l18 18 l34 -38" stroke="#15161e" stroke-width="11" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
      ${[[70, 40, "#f7768e"], [120, 130, "#e0af68"], [170, 60, "#7aa2f7"], [400, 50, "#bb9af7"], [450, 140, "#7dcfff"], [500, 70, "#f7768e"], [360, 150, "#e0af68"], [210, 150, "#9ece6a"]].map(([x, y, c], i) => (i % 2 ? `<rect x="${x}" y="${y}" width="12" height="12" rx="3" fill="${c}" transform="rotate(${i * 20} ${x} ${y})"/>` : `<circle cx="${x}" cy="${y}" r="6" fill="${c}"/>`)).join("")}`),
  };
})();
// el SVG es texto fijo de esta aplicación (nunca datos de usuarios)
const wizArt = (name) => { const el = document.createElement("div"); el.className = "wiz-art"; el.append(document.importNode(new DOMParser().parseFromString(WIZ_ART[name] || WIZ_ART.project, "image/svg+xml").documentElement, true)); return el; };

async function openWizard() {
  document.getElementById("wiz-modal")?.remove();
  const W = { i: 0, team: S.team, project: S.project, ai: false };
  const STEPS = ["welcome", "team", "perfil", "project", "ia", "listo"];
  const body = h("div", { class: "wiz-body" }), dots = h("div", { class: "wiz-dots" }), foot = h("div", { class: "wiz-foot" });
  const close = (done) => { if (done) store.set("wizDone", "1"); document.getElementById("wiz-modal")?.remove(); render(); };
  const teamIn = h("input", { placeholder: "Ej.: Mi empresa, Backend, Equipo A", maxLength: 60, value: S.team?.name || "" });
  const projIn = h("input", { placeholder: "Ej.: App de ventas", maxLength: 60 });
  const dirIn = h("input", { placeholder: "Carpeta del código, en la máquina del servidor (opcional; si no existe, la IA la crea)", maxLength: 300 });
  const sample = h("input", { type: "checkbox", checked: true });
  const profSel = h("select", {}, h("option", { value: "" }, "Elegir más tarde"), Object.entries(PROFILES).map(([id, p]) => h("option", { value: id, selected: S.profile === id }, p.name)));
  const profEx = h("input", { type: "checkbox" });
  const profNote = h("small", { class: "hint" }, "");
  profSel.onchange = () => { profNote.textContent = PROFILES[profSel.value]?.desc || ""; };
  const next = async () => {
    try {
      if (STEPS[W.i] === "perfil" && W.team && profSel.value && profSel.value !== S.profile) {
        await api("PUT", `/api/teams/${W.team.id}/profile`, { profile: profSel.value }); applyProfile(profSel.value);
      }
      if (STEPS[W.i] === "perfil" && profSel.value && profEx.checked && !W.exampleDone) {
        const r = await api("POST", "/api/examples", { profile: profSel.value });
        S.teams = await api("GET", "/api/teams"); W.exampleDone = true; render();
      }
      if (STEPS[W.i] === "team" && !W.team) {
        const n = teamIn.value.trim(); if (!n) return toast("Escribí un nombre para el equipo", true);
        W.team = await api("POST", "/api/teams", { name: n }); S.teams.push(W.team); await selectTeam(W.team);
      }
      if (STEPS[W.i] === "project" && !W.project) {
        const n = projIn.value.trim(); if (!n) return toast("Escribí un nombre para el proyecto", true);
        W.project = await api("POST", `/api/teams/${W.team.id}/projects`, { name: n, work_dir: dirIn.value.trim(), repo: "" });
        if (!S.projects.some((x) => x.id === W.project.id)) S.projects.push(W.project);
        await selectProject(W.project);
      }
      if (W.i === STEPS.length - 1) {
        if (sample.checked && W.project) await api("POST", `/api/projects/${W.project.id}/tasks`, { title: "👋 Mi primera tarea: arrastrame a «En curso»", description: "Las tareas se mueven entre columnas arrastrándolas. Asignala a una persona o a una IA desde acá." }).catch(() => {});
        if (W.project) await selectProject(W.project);
        return close(true);
      }
      W.i++; paint();
    } catch (e) { toast(e.message, true); }
  };
  const pick = (el) => el;
  const paint = () => {
    const k = STEPS[W.i];
    const T = {
      welcome: ["Bienvenida/o a " + APP_NAME, "Acá coordinás a tu equipo humano y a tus IAs en un solo tablero: repartís tareas, ves quién hace qué y las IAs trabajan por vos con permisos que controlás. En 5 pasos dejamos todo andando."],
      team: ["Tu equipo", W.team ? `Ya tenés el equipo «${W.team.name}». Seguimos.` : "Un equipo agrupa a las personas y a las IAs que trabajan juntas. Vos quedás como dueño. Después invitás a quien quieras."],
      perfil: ["¿Para qué lo van a usar?", "Elegí un perfil y la interfaz se prepara sola: nombres de las columnas y qué partes técnicas se ven. Podés cambiarlo cuando quieras en Configuración → Equipo y proyecto."],
      project: ["Tu primer proyecto", W.project ? `Proyecto «${W.project.name}» listo.` : "Un proyecto es un tablero de tareas. Si querés que las IAs trabajen sobre código, indicá la carpeta donde vive (en la máquina del servidor)."],
      ia: ["Sumá una IA (opcional)", W.ai ? "¡IA creada! Aparece en la barra lateral con su luz 🟢/🔴." : "Una IA es un asistente de código (Claude, Gemini, ChatGPT o un modelo propio) que toma tareas del tablero y pide permiso por la web para lo delicado. Podés hacerlo ahora o más tarde desde «IAs → +»."],
      listo: ["¡Todo listo!", "Ya podés usar el tablero. Tips: tecla ? muestra los atajos · la campanita 🔔 te lleva a lo nuevo · ⚙ Configuración tiene avisos, informes y comandos de las IAs · «🧭 Guía de inicio» la volvés a abrir cuando quieras."],
    }[k];
    const fields = {
      team: !W.team && h("label", {}, "Nombre del equipo", teamIn),
      perfil: [h("label", {}, "Perfil de uso", profSel, profNote), h("label", { class: "row" }, profEx, h("span", {}, "Crear también un equipo de ejemplo ya armado (separado del tuyo, se borra con un botón)"))],
      project: !W.project && [h("label", {}, "Nombre del proyecto", projIn), h("label", {}, "Carpeta de trabajo", dirIn)],
      ia: h("div", { class: "row" }, h("button", { class: W.ai ? "" : "primary", onclick: guard(async () => { const before = (S.bots || []).length; await createBot(); await boot(); W.ai = (S.bots || []).length > before || W.ai; paint(); }) }, W.ai ? "➕ Agregar otra IA" : "🤖 Agregar una IA")),
      listo: h("label", { class: "row" }, sample, h("span", {}, "Crear una tarea de ejemplo en el tablero")),
    }[k];
    body.replaceChildren(wizArt(k), h("h2", {}, T[0]), h("p", {}, T[1]), ...[fields].flat().filter(Boolean));
    dots.replaceChildren(...STEPS.map((_, n) => h("span", { class: "wiz-dot" + (n === W.i ? " on" : n < W.i ? " done" : "") })));
    const last = W.i === STEPS.length - 1;
    foot.replaceChildren(
      h("button", { class: "ghost", onclick: () => close(true) }, "Saltar la guía"), h("span", { class: "spacer" }),
      W.i > 0 ? h("button", { onclick: () => { W.i--; paint(); } }, "Atrás") : null,
      h("button", { class: "primary", onclick: next }, last ? "Empezar 🚀" : k === "ia" && !W.ai ? "Más tarde →" : "Siguiente →"));
    (teamIn.isConnected ? teamIn : projIn.isConnected ? projIn : null)?.focus();
  };
  document.body.append(h("div", { class: "overlay wiz", id: "wiz-modal" }, h("div", { class: "modal wiz-card" }, dots, body, foot)));
  paint();
}
// primera vez: sin ninguna persona registrada la pantalla de entrada invita a crear la cuenta del dueño
async function checkFirstRun() {
  try { const r = await fetch(S.server + "/api/setup"); if (r.ok) { S.setup = await r.json(); if (S.setup.first_run && !S.token) { S.register = true; render(); } } } catch {}
}

if (IS_WEB && "serviceWorker" in navigator) navigator.serviceWorker.register("/sw.js").catch(() => {});
// ---------- zoom de la letra: A− / A+ flotante en todas las pantallas (se recuerda en este dispositivo) ----------
(function zoomControl() {
  const STEPS = [0.8, 0.9, 1, 1.1, 1.25, 1.4, 1.6, 1.8, 2];
  let z = +store.get("zoom") || 1;
  const apply = () => { document.documentElement.style.zoom = z === 1 ? "" : String(z); lbl.textContent = Math.round(z * 100) + "%"; store.set("zoom", String(z)); };
  const step = (d) => { const i = Math.max(0, STEPS.findIndex((x) => x >= z - 0.001)); z = STEPS[Math.min(STEPS.length - 1, Math.max(0, i + d))]; apply(); };
  const lbl = h("button", { class: "zoom-lbl", title: "Volver al tamaño normal", onclick: () => { z = 1; apply(); } }, "100%");
  const box = h("div", { class: "zoom-ctl", title: "Tamaño de la letra (también Ctrl + Alt + rueda del mouse)" },
    h("button", { title: "Letra más chica", onclick: () => step(-1) }, "A−"), lbl, h("button", { title: "Letra más grande", onclick: () => step(1) }, "A+"));
  document.addEventListener("wheel", (e) => { if (e.ctrlKey && e.altKey) { e.preventDefault(); step(e.deltaY < 0 ? 1 : -1); } }, { passive: false });
  document.body.append(box); apply();
})();

render();
if (S.token) boot(true).catch(() => { S.token = null; render(); }); else checkFirstRun();


// ---------- terminal SSH integrada (solo owners): el servidor abre SSH con clave a la máquina del proyecto ----------
const loadScript = (src) => new Promise((ok, no) => { const el = document.createElement("script"); el.src = src; el.onload = ok; el.onerror = () => no(new Error("no pude cargar " + src)); document.head.append(el); });

// interruptores para el equipo: la terminal de las personas y la entrada por SSH (el agente se activa solo, sin variables ni consola)
function peopleBox(pid) {
  const box = h("div", { class: "peoplebox" });
  const people = h("input", { type: "checkbox" }), ssh = h("input", { type: "checkbox" });
  const save = guard(async () => {
    if (!pid) return;
    const r = await api("PUT", `/api/projects/${pid}/term/settings`, { people: people.checked, ssh: ssh.checked });
    people.checked = r.people; ssh.checked = r.ssh; ssh.disabled = !r.people;
    toast(r.people ? "Listo: en un minuto las personas del equipo pueden abrir su terminal" : "Terminal para las personas: apagada");
  });
  people.onchange = () => { if (!people.checked) ssh.checked = false; ssh.disabled = !people.checked; save(); };
  ssh.onchange = save;
  box.append(
    h("label", { class: "chk" }, people, " Permitir la terminal a las personas del equipo",
      h("small", { class: "hint" }, "Cada persona tiene su espacio propio en el servidor, que se conserva entre sesiones. Quienes no son owners trabajan dentro de un sandbox: no ven el resto de la máquina ni los espacios de otros.")),
    h("label", { class: "chk" }, ssh, " También permitir entrar por ssh con su clave",
      h("small", { class: "hint" }, "Cada persona carga su clave pública en su perfil («🔑 Entrar por SSH»).")));
  ssh.disabled = true;
  if (pid) api("GET", `/api/projects/${pid}/term/settings`).then((r) => { people.checked = !!r.people; ssh.checked = !!r.ssh; ssh.disabled = !r.people; if (!r.agent_id) box.append(h("small", { class: "hint" }, "Primero guardá la conexión con el agente (botón «▶ Conectar»).")); }).catch(() => {});
  return box;
}

// asistente de git del proyecto: traer un repo de GitHub, crear uno nuevo o dejarlo solo en la máquina (lo hace el agente de esa máquina)
function gitBox(pid) {
  const box = h("fieldset", { class: "gitbox" }, h("legend", { title: "Deja la carpeta del proyecto lista para trabajar en equipo: repositorio git, conexión con GitHub y clave para subir cambios" }, "🐙 Git y GitHub"));
  const body = h("div", {}); box.append(body);
  const repoOf = (url) => (String(url || "").match(/[:/]([\w.-]+\/[\w.-]+?)(?:\.git)?$/) || [])[1] || "";
  const run = async (mode, extra, label) => {
    body.replaceChildren(h("p", { class: "hint" }, "⏳ " + label + " (puede tardar hasta un minuto)…"));
    try {
      const r = await api("POST", `/api/projects/${pid}/git/setup`, { mode, ...extra });
      toast("Listo");
      if (r.repo && S.project?.id === pid && !S.project.repo) { S.project.repo = r.repo; render(); }
      draw(r);
    } catch (e) { toast(e.message, true); draw(null, e.message); }
  };
  let ghi = null; api("GET", "/api/github").then((i) => { ghi = i; }).catch(() => {});
  const draw = (st, err) => {
    const parts = [];
    if (ghi && ghi.available && !ghi.linked) parts.push(h("div", { class: "gitopt" }, h("b", {}, "🔑 Falta conectar tu cuenta de GitHub"), h("small", { class: "hint" }, "Para traer o crear un repositorio hace falta tu cuenta (se usa solo mientras se hace, no se guarda en la carpeta). «Solo en esta máquina» no la necesita."),
      h("button", { onclick: () => { document.getElementById("git-setup")?.remove(); linkGithub(); } }, "Conectar mi GitHub")));
    if (ghi && !ghi.available) parts.push(h("p", { class: "hint" }, "Este servidor no tiene GitHub configurado: solo se puede crear el repositorio local."));
    if (err) parts.push(h("p", { class: "err" }, "⚠ " + err));
    if (st?.steps?.length) parts.push(h("ul", { class: "gitsteps" }, st.steps.map((x) => h("li", {}, "✔ " + x))));
    if (!st) { parts.push(h("button", { class: "ghost", onclick: load }, "↻ Revisar de nuevo")); return body.replaceChildren(...parts); }
    const repoIn = h("input", { placeholder: "usuario/repositorio", title: "Por ejemplo polblancoo/crewhelm" });
    const nameIn = h("input", { placeholder: "nombre del repositorio nuevo", value: "", title: "Letras, números, guiones y puntos" });
    const priv = h("input", { type: "checkbox", checked: true });
    const newBlock = h("div", { class: "gitopt" }, h("b", {}, "✨ Crear un repositorio nuevo en GitHub"),
      h("small", { class: "hint" }, "Sube lo que haya en la carpeta (menos claves y carpetas pesadas, que se dejan afuera con un .gitignore)."),
      nameIn, h("label", { class: "chk", title: "Privado: solo lo ven las personas que invites en GitHub. Público: lo puede ver cualquiera (escribir siempre requiere permiso)." }, priv, " Privado"),
      h("button", { title: "Crea el repositorio en tu cuenta de GitHub, sube la carpeta y deja lista la clave para subir cambios", onclick: () => nameIn.value.trim() ? run("new", { name: nameIn.value.trim(), private: priv.checked }, "Creando el repositorio") : toast("Escribí un nombre", true) }, "✨ Crear y subir"));
    if (st.is_repo && st.origin) {
      const r = repoOf(st.origin);
      parts.push(h("p", {}, "✔ Conectado a ", h("b", {}, r || st.origin), st.branch ? " · rama " + st.branch : "", st.dirty ? " · hay cambios sin guardar" : ""));
      parts.push(h("button", { class: "ghost", title: "Vuelve a registrar la clave con la que este servidor sube cambios a GitHub (si dejó de funcionar)", onclick: () => run("attach", { repo: r }, "Renovando la clave de subida") }, "🔑 Renovar la clave de subida"));
    } else if (st.is_repo) {
      parts.push(h("p", {}, "La carpeta ya es un repositorio git, pero no está conectada con GitHub."));
      parts.push(h("div", { class: "gitopt" }, h("b", {}, "🔗 Conectar con un repositorio que ya existe"), repoIn,
        h("button", { title: "Registra una clave para subir cambios y apunta la carpeta a ese repositorio", onclick: () => repoIn.value.trim() ? run("attach", { repo: repoIn.value.trim() }, "Conectando con GitHub") : toast("Escribí usuario/repositorio", true) }, "🔗 Conectar")), newBlock);
    } else {
      parts.push(h("p", {}, st.exists ? (st.empty ? "La carpeta del proyecto está vacía." : "La carpeta del proyecto tiene archivos pero todavía no es un repositorio.") : "La carpeta del proyecto todavía no existe."));
      if (!st.exists || st.empty) parts.push(h("div", { class: "gitopt" }, h("b", {}, "⬇ Traer un repositorio de GitHub"), repoIn,
        h("button", { title: "Descarga el repositorio a la carpeta del proyecto y deja lista la clave para subir cambios", onclick: () => repoIn.value.trim() ? run("clone", { repo: repoIn.value.trim() }, "Trayendo el repositorio") : toast("Escribí usuario/repositorio", true) }, "⬇ Traer")));
      parts.push(newBlock);
      parts.push(h("div", { class: "gitopt" }, h("b", {}, "💻 Solo en esta máquina"), h("small", { class: "hint" }, "Crea el repositorio git sin GitHub. Se puede conectar más tarde."),
        h("button", { class: "ghost", title: "git init + primer commit, sin subir nada a internet", onclick: () => run("local", {}, "Creando el repositorio") }, "💻 Crear solo local")));
    }
    body.replaceChildren(...parts);
  };
  const load = async () => {
    body.replaceChildren(h("p", { class: "hint" }, "⏳ Mirando la carpeta del proyecto…"));
    try {
      const gh = await api("GET", "/api/github").catch(() => ({}));
      if (!gh.available) return body.replaceChildren(h("p", { class: "hint" }, "GitHub no está configurado en este servidor (lo activa quien administra el servidor)."));
      if (!gh.linked) return body.replaceChildren(h("p", { class: "hint" }, "Primero conectá tu cuenta de GitHub en Configuración → Avisos y conexiones."));
      draw(await api("POST", `/api/projects/${pid}/git/setup`, { mode: "status" }));
    } catch (e) { body.replaceChildren(h("p", { class: "hint" }, "No pude revisar la carpeta: " + e.message)); }
  };
  load();
  return box;
}

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
    h("small", { class: "hint" }, "Alcanza con que el agente de esa IA esté corriendo en la máquina del proyecto: se conecta solo hacia el servidor, sin abrir puertos, copiar claves ni cargar IPs. Funciona también detrás de NAT y en Windows."),
    can("owner") && peopleBox(pid), can("owner") && gitBox(pid));
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
    ws.onopen = () => { ws.send(JSON.stringify({ token: S.token, cols: term.cols, rows: term.rows })); setCfg(true); };
    ws.onmessage = (e) => {
      if (typeof e.data === "string") {
        const m = JSON.parse(e.data);
        if (m.error) { term.write("\r\n\x1b[31m" + m.error + "\x1b[0m\r\n"); setCfg(false); }
        if (m.info) term.write("\x1b[33m" + m.info + "\x1b[0m\r\n");
      } else term.write(new Uint8Array(e.data));
    };
    ws.onclose = () => term && term.write("\r\n\x1b[90m[sesión cerrada]\x1b[0m\r\n");
    const enc = new TextEncoder();
    term.onData((d) => ws.readyState === 1 && ws.send(enc.encode(d)));
    term.onResize(({ cols, rows }) => ws.readyState === 1 && ws.send(JSON.stringify({ resize: [cols, rows] })));
    ro = new ResizeObserver(() => { try { fit.fit(); } catch {} }); ro.observe(box);
    term.focus();
    // la máquina del proyecto no tiene herdr: se propone instalarlo (el comando queda escrito en la terminal; lo confirma la persona con Enter)
    const ag = mode.value === "agent" ? (await api("GET", `/api/projects/${pid}/term/agents`).catch(() => [])).find((x) => String(x.id) === agentSel.value) : null;
    drawTools(ag);
    if (ag && ag.online && !ag.herdr && store.get("no_herdr") !== "1") {
      const bar = h("div", { class: "row", style: "background:var(--bg2);border-radius:8px;padding:8px 10px;margin-top:6px" },
        h("span", { class: "grow" }, "Esta máquina no tiene herdr: un panel que muestra qué hace cada IA (trabajando, esperando o bloqueada) y deja las sesiones corriendo si cerrás la terminal."),
        h("button", { class: "primary", onclick: () => { ws.readyState === 1 && ws.send(enc.encode("curl -fsSL https://herdr.dev/install.sh | sh")); bar.remove(); toast("Revisá el comando en la terminal y tocá Enter para instalarlo"); term.focus(); } }, "Instalar"),
        h("button", { onclick: () => bar.remove() }, "Ahora no"),
        h("button", { class: "ghost", onclick: () => { store.set("no_herdr", "1"); bar.remove(); } }, "No preguntar más"));
      tools.after(bar);
    }
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
  // todo lo de la conexión se puede ocultar para dejarle el lugar a la terminal (se oculta solo al conectar)
  const cfgWrap = h("div", {},
    h("label", {}, "Conexión", mode),
    agentBox,
    sshBox,
    h("div", { class: "row" },
      h("button", { class: "primary", onclick: connect }, "▶ Conectar"),
      h("button", { title: "Usa esta misma conexión en todos los demás proyectos del equipo", onclick: copyAll }, "📋 Usar en todos los proyectos")));
  const tools = h("div", { class: "row term-tools" });
  const setCfg = (hide) => { cfgWrap.hidden = hide; modal.querySelector(".term-modal")?.classList.toggle("cfg-hidden", hide); const b = tools.querySelector(".cfg-toggle"); if (b) b.textContent = hide ? "⚙ Mostrar conexión" : "⚙ Ocultar conexión"; setTimeout(() => { try { fit?.fit(); } catch {} }, 60); };
  const AI_BIN = { claude: "claude", gemini: "gemini", openai: "codex" };
  // con una tarea, la IA arranca leyendo el contexto que el servidor dejó en .tm/ (sin comillas ni símbolos que la shell interprete)
  const aiCmdFor = (ai) => {
    if (!task.id) return ai;
    const prompt = `Leé el archivo .tm/terminal-contexto-${task.id}.md: es el contexto de la tarea ${task.id}. Resumime en dos líneas qué hay que hacer y esperá mis indicaciones.`;
    return ai === "gemini" ? `gemini -i "${prompt}"` : `${ai} "${prompt}"`;
  };
  const typeCmd = (text) => { if (sock && sock.readyState === 1) { sock.send(new TextEncoder().encode(text + "\r")); term?.focus(); } else toast("Conectate primero", true); };
  const drawTools = (ag) => {
    const ai = ag && AI_BIN[(ag.model || "").split(":")[0].toLowerCase()];
    tools.replaceChildren(h("button", { class: "ghost cfg-toggle", onclick: () => setCfg(!cfgWrap.hidden) }, cfgWrap.hidden ? "⚙ Mostrar conexión" : "⚙ Ocultar conexión"),
      ag && ag.herdr && h("button", { title: "Abre herdr en la carpeta del proyecto: panel con el estado de cada IA", onclick: () => typeCmd("herdr") }, "▶ Abrir herdr"),
      ai && h("button", { title: task.id ? `Abre ${ai} ya enterado de esta tarea (título, descripción y lo último que se habló), en la carpeta del proyecto` : `Abre ${ai} en la carpeta del proyecto`, onclick: () => typeCmd(aiCmdFor(ai)) }, "🤖 Abrir " + ai + (task.id ? " con la tarea" : "")));
  };
  const close = async () => {
    stop(); closeModal();
    // si venía desde la tarea, vuelve a ella con la terminal abajo (solo si la conexión quedó lista)
    if (S.dockAfter && S.dockAfter === task.id) {
      S.dockAfter = null;
      const c = await api("GET", `/api/projects/${task.project_id}/ssh`).catch(() => ({}));
      const t = S.tasks.find((x) => x.id === task.id);
      if (t) { S.dockOpen = c.host || c.agent_id ? t.id : null; openTask(t); }
    }
  };
  const modal = h("div", { class: "overlay", id: "modal", onmousedown: (e) => e.target.id === "modal" && close() },
    h("div", { class: "modal term-modal" + (store.get("termMax") === "1" ? " max" : "") },
      h("h3", { class: "task-head" }, "💻 Terminal · " + (task.id ? "tarea #" + task.id : "configuración de la conexión"), h("span", { class: "spacer" }),
        h("button", { class: "ghost max-btn", title: "Agranda la terminal para trabajar más cómodo", onclick: (e) => { const m = e.currentTarget.closest(".modal"); const on = m.classList.toggle("max"); store.set("termMax", on ? "1" : "0"); e.currentTarget.textContent = on ? "⤡ Restaurar" : "⤢ Maximizar"; setTimeout(() => { try { fit?.fit(); } catch {} }, 80); } }, store.get("termMax") === "1" ? "⤡ Restaurar" : "⤢ Maximizar"),
        h("button", { class: "ghost", onclick: close }, "✕")),
      cfgWrap,
      tools,
      box,
      h("small", { class: "hint" }, "Cada sesión queda anotada en el historial de la tarea. Se cierra sola a los 30 min sin uso.")));
  closeModal(); S.open = null;
  document.body.append(modal);
}


// ---------- vista Terminales: varias terminales SSH en paneles + estado de las IAs (solo owners) ----------
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
      h("button", { title: "Abre un panel por cada tarea abierta de este proyecto, con el título de la tarea y su responsable", onclick: openProjectPanes }, "📋 Abrir las tareas"),
      h("button", { class: store.get("tvZen") === "1" ? "on" : "", title: "Oculta el menú lateral y el panel del equipo para tener toda la pantalla para las terminales", onclick: () => { store.set("tvZen", store.get("tvZen") === "1" ? "0" : "1"); render(); startPanes(); } }, store.get("tvZen") === "1" ? "⛶ Mostrar barras" : "⛶ Ocultar barras"),
      h("button", { class: store.get("tvSide") === "0" ? "on" : "", title: "Muestra u oculta la lista de IAs de la izquierda", onclick: () => { store.set("tvSide", store.get("tvSide") === "0" ? "1" : "0"); render(); startPanes(); } }, store.get("tvSide") === "0" ? "◧ Mostrar IAs" : "◧ Ocultar IAs"),
      can("owner") && h("fieldset", { class: "tv-admin", title: "Herramientas de administración de las terminales: solo las ven los owners" },
        h("legend", {}, "🛠 Administración"),
        (() => {
        const sel = h("select", { title: "Ver espacio: abre en solo lectura la carpeta personal de otra persona o IA en el servidor, para auditar o ayudar. No se puede modificar nada", onchange: (e) => { const v = e.target.value; e.target.value = ""; if (v) addPane(S.tvProject, null, "👁 espacio de " + v, v); } },
          h("option", { value: "" }, "👁 Ver espacio de…"), ...(S.members || []).filter((m) => m.id !== S.me?.id).map((m) => h("option", { value: m.username }, (botInfo(m.username) ? "🤖 " : "👤 ") + m.username)));
        return sel;
      })(),
        h("button", { title: "Conexión: cómo llega la terminal a la máquina del proyecto (agente o SSH) y si las personas del equipo pueden usarla", onclick: guard(() => openTerminal({ id: null, project_id: S.tvProject })) }, "⚙ Conexión"),
        can("owner") && h("button", { title: "Repositorio git del proyecto: traer uno de GitHub, crear uno nuevo o dejarlo solo local", onclick: () => { const p = S.projects.find((x) => x.id === S.tvProject); if (p) openGitSetup(p); } }, "🐙 Repositorio")),
      h("small", { class: "hint" }, "Hasta 8 a la vez. Lo que hagas queda anotado en .tm/ de la carpeta del proyecto para que la IA retome.")),
    h("div", { class: "tv-body" }, store.get("tvSide") !== "0" && tvSide,
      h("div", { class: "tv-grid" + (PANES.length === 1 ? " one" : ""), style: `--cols:${c}` }, PANES.length ? PANES.map((p) => p.el) : h("div", { class: "empty" }, "Tocá «+ Terminal» o elegí una IA de la izquierda."))));
}
// barra de acciones de git bajo la terminal de una persona (no owner): escribe en la terminal el comando `cw …`, así se ve qué se hizo y se aprende
function gitBar(pane) {
  if (can("owner") || pane.space) return null;
  const run = (cmd) => { if (pane.ws?.readyState === 1) { pane.ws.send(new TextEncoder().encode(cmd + "\r")); pane.term?.focus(); } else toast("La terminal todavía no está conectada", true); };
  const b = (txt, tip, cmd) => h("button", { class: "ghost", title: tip, onclick: () => run(cmd) }, txt);
  const bar = h("div", { class: "gitbar" },
    h("span", { class: "gb-t", title: "Atajos para trabajar sin saber comandos. Cada botón escribe el comando en la terminal; usalos con la terminal en el símbolo normal (no dentro de un editor)." }, "🧭 Tu trabajo"),
    b("🔄 Actualizar", "Trae a tu copia las novedades que hizo el resto del equipo.\nEn git: git fetch + git merge --ff-only", "cw actualizar"),
    b("🌿 Empezar trabajo", "Crea tu propia rama para un trabajo nuevo: te pregunta cómo se llama.\nEn git: git switch -c nombre-de-la-rama", "cw rama"),
    b("📋 Qué cambié", "Muestra en qué rama estás y qué archivos tocaste.\nEn git: git status", "cw estado"),
    b("💾 Guardar", "Guarda tus cambios con un mensaje corto (te lo pregunta).\nEn git: git add -A + git commit -m \"mensaje\"", "cw guardar"),
    b("🚀 Enviar a revisión", "Sube tu trabajo y abre el Pull Request para que otra persona lo revise.\nEn git: git push origin tu-rama (y el Pull Request se abre en GitHub)", "cw enviar"),
    b("❓ Ayuda", "Lista de comandos y el orden normal: empezar → editar → guardar → enviar", "cw ayuda"));
  // si el proyecto todavía no tiene repositorio, los botones no sirven: se ofrece pedirlo
  api("GET", `/api/projects/${pane.pid}/git/state`).then((s) => {
    if (s.is_repo) return;
    bar.replaceChildren(h("span", { class: "gb-t warn" }, "⚠ Este proyecto todavía no tiene repositorio: no se puede editar ni enviar trabajo."),
      h("button", { title: "Les avisa a los owners del equipo para que creen el repositorio del proyecto", onclick: () => requestRepo(pane.pid) }, "🙋 Pedir repositorio"));
  }).catch(() => {});
  return bar;
}
// la terminal de una tarea se abre como un panel de «🖥️ Terminales»: sigue viva aunque mires otra tarea, la configuración o lo que sea
const paneTitle = (t) => {
  const m = (S.members || []).find((x) => x.id === t.assignee_id);
  return `#${t.id} · ${t.title.length > 42 ? t.title.slice(0, 42) + "…" : t.title} · ${m ? (botInfo(m.username) ? "🤖 " : "👤 ") + m.username : "sin responsable"}`;
};
const openTaskTerminal = guard(async (task) => {
  const c0 = await api("GET", `/api/projects/${task.project_id}/ssh`);
  if (!c0.host && !c0.agent_id) return openTerminal(task);
  closeModal(); S.open = null;
  S.tvProject = task.project_id;
  await openTerms();
  await addPane(task.project_id, task.id, paneTitle(task));
});
// abre un panel por cada tarea abierta del proyecto (en curso primero), hasta completar 8
const openProjectPanes = guard(async () => {
  await loadTermTasks();
  const list = (S.tvTasks || []).filter((t) => !t.parent_id && t.status !== "done" && !/^🙋 Pedido de #/.test(t.title))
    .sort((x, y) => (x.status === "doing" ? 0 : 1) - (y.status === "doing" ? 0 : 1));
  let n = 0;
  for (const t of list) { if (PANES.length >= 8) break; if (PANES.some((p) => p.tid === t.id)) continue; await addPane(t.project_id, t.id, paneTitle(t)); n++; }
  toast(n ? `Abiertas ${n} terminales, una por tarea` : "No hay tareas abiertas sin terminal (máximo 8 a la vez)");
});
async function addPane(pid, tid, title, space) {
  if (PANES.length >= 8) return toast("Máximo 8 terminales abiertas", true);
  if (tid && PANES.some((p) => p.tid === tid)) return toast("Esa tarea ya tiene su terminal abierta acá");
  try {
    const c0 = can("owner") ? await api("GET", `/api/projects/${pid}/ssh`) : { agent_id: 1 }; if (!c0.host && !c0.agent_id) { toast("Este proyecto todavía no tiene conexión a su máquina: configurala primero"); return openTerminal({ id: null, project_id: pid }); }
  } catch (e) { return toast(e.message, true); }
  const box = h("div", { class: "term-box" }), st = h("span", { class: "tv-st" }, "⏳");
  const pane = { pid, tid, box, st, started: false, space };
  pane.el = h("div", { class: "tv-pane" }, h("div", { class: "tv-head" }, st, h("b", {}, title), h("span", { class: "spacer" }),
    h("button", { class: "ghost", title: "Agranda esta terminal a toda la pantalla (el mismo botón para volver)", onclick: (e) => { const on = pane.el.classList.toggle("full"); e.currentTarget.textContent = on ? "⤡" : "⤢"; setTimeout(() => { try { pane.fit?.fit(); pane.term?.focus(); } catch {} }, 80); } }, "⤢"),
    h("button", { class: "ghost", title: "Cerrar esta terminal (guarda la nota .md)", onclick: () => closePane(pane) }, "✕")), box, gitBar(pane));
  PANES.push(pane); render(); startPanes();
}
function closePane(p) { try { p.ws?.close(); } catch {} p.ro?.disconnect(); p.term?.dispose(); PANES.splice(PANES.indexOf(p), 1); p.el.remove(); render(); startPanes(); }
function startPanes() { requestAnimationFrame(() => PANES.forEach(startPane)); }
// arranca la sesión de un panel (de «Terminales» o la acoplada en una tarea): mismo WebSocket, mismas preferencias
function startPane(p) {
  {
    if (p.started || !p.el.isConnected) { try { p.fit?.fit(); } catch {} return; }
    p.started = true;
    const term = new Terminal({ cursorBlink: true, fontSize: 13, fontFamily: "monospace", theme: { background: "#0b0d14" } });
    const fit = new FitAddon.FitAddon(); term.loadAddon(fit); term.open(p.box); fit.fit();
    Object.assign(p, { term, fit });
    const ws = new WebSocket(S.server.replace(/^http/, "ws") + `/api/projects/${p.pid}/term${p.tid ? "?task=" + p.tid : ""}`); p.ws = ws; ws.binaryType = "arraybuffer";
    ws.onopen = () => { ws.send(JSON.stringify({ token: S.token, cols: term.cols, rows: term.rows, space: p.space || undefined })); p.st.textContent = "🟢"; };
    // por defecto, la terminal de una tarea abre herdr en la carpeta del proyecto (si la máquina lo tiene; se apaga con store autoHerdr=0)
    if (p.tid && store.get("autoHerdr") !== "0") {
      let armed = true;
      setTimeout(() => { if (armed && ws.readyState === 1) { armed = false; ws.send(new TextEncoder().encode("command -v herdr >/dev/null 2>&1 && herdr\r")); } }, 1800);
    }
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
  }
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
    const cmd = h("input", { value: found[0] || "", maxLength: 1000, placeholder: "ej.: cargo build" });
    const why = h("div", { class: "gd-why" });
    const scope = h("select", {}, h("option", { value: "always" }, "Siempre: se agrega a la lista de comandos permitidos de esta IA (recomendado)"), h("option", { value: "once" }, "Solo esta vez (la próxima ejecución de esta tarea)"));
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

const netName = (r) => ({ tron: "Tron (TRC-20)", solana: "Solana (SPL)", ethereum: "Ethereum (ERC-20)" }[r] || r);

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
    if (l.exceeds_plan) kids.push(h("p", { class: "lic-warn" }, `⚠️ Tu plan «${l.license.plan}» incluye hasta ${l.plan_limits.humans ?? "—"} personas y ${l.plan_limits.ais ?? "—"} IAs, y hoy hay ${l.humans} personas y ${l.ais} IAs. Para seguir en regla pasá al plan que corresponde y pagá solo la diferencia.`));
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
      h("h4", {}, `Pagá exactamente ${o.monto} ${o.cripto} en la red ${netName(o.red)}`),
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
  // términos que se aceptan antes de pagar: el texto viene del servicio de licencias y queda registrada la versión aceptada
  const termsBox = (t) => {
    if (!t) return { nodes: [h("p", { class: "lic-warn" }, "Los términos todavía no están publicados; no se puede generar el pago.")], ok: () => false };
    const chk = h("input", { type: "checkbox" });
    return { nodes: [h("label", {}, "Términos de la licencia comercial (versión " + t.version + ")", h("textarea", { readOnly: true, rows: 9, class: "lic-terms" }, t.texto)),
      h("label", { class: "row" }, chk, h("span", {}, "Leí y acepto los términos"))], ok: () => chk.checked };
  };
  const planText = (p) => `${p.nombre}: hasta ${p.max_usuarios} personas y ${p.max_ias} IAs · USD ${p.precio_usd} por ${p.dias >= 365 ? "año" : p.dias + " días"}` + (p.fundador ? ` · 🎖 precio fundador (lista USD ${p.precio_lista})` : "");
  const orderBtn = (pl, getBody, terms, email) => h("button", { class: "primary", onclick: guard(async () => {
    if (!terms.ok()) return toast("Tenés que aceptar los términos", true);
    const o = await api("POST", "/api/license/order", { ...getBody(), email: email.value, accept: pl.terminos.version, lang: tr.lang });
    showOrder(o, email.value.trim());
  }) }, "Generar el pago");
  const buy = async () => {
    const pl = await api("GET", "/api/license/plans?lang=" + encodeURIComponent(tr.lang || "es"));
    if (!pl.planes.length || !pl.metodos.length) { body.replaceChildren(h("p", {}, "Todavía no hay planes o formas de pago disponibles. Probá más tarde."), h("button", { onclick: openLicense }, "← Volver")); return; }
    const email = h("input", { type: "email", placeholder: "tu@email.com (acá te llega el código)", maxLength: 120 });
    const plan = h("select", {}, pl.planes.map((p) => h("option", { value: p.plan }, planText(p))));
    const met = h("select", {}, pl.metodos.map((m, i) => h("option", { value: i }, `${m.cripto} en ${netName(m.red)}`)));
    const cont = h("small", { class: "hint" });
    const upd = () => { const m = pl.metodos[+met.value]; cont.replaceChildren("Contrato del token: ", h("code", {}, m.contrato), " ", link(m.explorador_token, "ver en el explorador")); };
    met.onchange = upd; upd();
    const fund = pl.planes.find((p) => p.fundador && p.cupos_restantes > 0);
    const terms = termsBox(pl.terminos);
    body.replaceChildren(h("h4", {}, "Comprar una licencia comercial"),
      fund && h("p", { class: "lic-ok" }, `🎖 Precio fundador: quedan ${fund.cupos_restantes} cupos. Lo conservás de por vida en las renovaciones.`),
      h("label", {}, "Plan", plan), h("label", {}, "Pagar con", met), cont, h("label", {}, "Email", email),
      h("small", { class: "hint" }, "El email se usa solo para mandarte el código y el comprobante. Si después sumás más personas o IAs de las que incluye tu plan, pagás la diferencia con el plan que corresponde."),
      ...terms.nodes,
      h("div", { class: "row" }, orderBtn(pl, () => { const m = pl.metodos[+met.value]; return { plan: plan.value, cripto: m.cripto, red: m.red }; }, terms, email), h("button", { onclick: openLicense }, "← Volver")));
  };
  const improve = async () => {
    const [u, pl] = await Promise.all([api("GET", "/api/license/upgrade"), api("GET", "/api/license/plans?lang=" + encodeURIComponent(tr.lang || "es"))]);
    if (!u.necesita) { toast("Tu uso entra en el plan actual"); return; }
    if (!u.destino) { body.replaceChildren(h("h4", {}, "Mejorar el plan"), h("p", {}, u.motivo || "Escribinos para un acuerdo a medida."), h("button", { onclick: openLicense }, "← Volver")); return; }
    const email = h("input", { type: "email", placeholder: "tu@email.com (comprobante)", maxLength: 120 });
    const met = h("select", {}, pl.metodos.map((m, i) => h("option", { value: i }, `${m.cripto} en ${netName(m.red)}`)));
    const terms = termsBox(pl.terminos);
    body.replaceChildren(h("h4", {}, `Pasar al plan ${u.destino.nombre}`),
      h("p", {}, `Uso actual: ${u.uso.usuarios} personas y ${u.uso.ias} IAs (tu plan incluye ${u.limites.usuarios} y ${u.limites.ias}). El plan ${u.destino.nombre} incluye hasta ${u.destino.max_usuarios} personas y ${u.destino.max_ias} IAs.`),
      h("p", { class: "lic-ok" }, `Precio del plan USD ${u.precio_destino} − ya pagado USD ${u.pagado} = a pagar ahora USD ${u.diferencia}. Tu vencimiento no cambia.`),
      h("label", {}, "Pagar con", met), h("label", {}, "Email", email), ...terms.nodes,
      h("div", { class: "row" }, orderBtn(pl, () => { const m = pl.metodos[+met.value]; return { plan: u.destino.plan, cripto: m.cripto, red: m.red, upgrade: true }; }, terms, email), h("button", { onclick: openLicense }, "← Volver")));
  };
  const code = h("input", { placeholder: "TM-XXXX-XXXX-XXXX", maxLength: 40 });
  const act = guard(async () => { await api("POST", "/api/license/activate", { codigo: code.value }); toast("Licencia activada"); openLicense(); loadLicense(); });
  const main = () => body.replaceChildren(...[...head(), ...settings(),
    l.admin && h("div", { class: "row" }, l.exceeds_plan && h("button", { class: "primary", onclick: guard(improve) }, "⬆ Mejorar el plan (pagar la diferencia)"),
      h("button", { class: l.exceeds_plan ? "" : "primary", onclick: guard(buy) }, l.licensed ? "Renovar o comprar otro plan" : "Comprar una licencia comercial")),
    l.admin && h("label", {}, "¿Ya tenés un código?", h("div", { class: "row" }, code, h("button", { onclick: act }, "Activar")))].filter(Boolean));
  main();
  document.body.append(h("div", { class: "overlay", id: "lic-modal", onmousedown: (e) => e.target.id === "lic-modal" && close() },
    h("div", { class: "modal lic-modal" }, h("h3", {}, "🏢 Licencia", h("span", { class: "spacer" }), h("button", { class: "ghost", onclick: close }, "✕")), body)));
  const saved = (() => { try { return JSON.parse(store.get("lic_order") || "null"); } catch { return null; } })();
  if (saved && l.admin && !l.licensed) {
    body.append(h("button", { onclick: guard(async () => { const r = await api("GET", `/api/license/order/${saved.id}?token=${encodeURIComponent(saved.token)}`); toast(r.estado === "pendiente" ? "Pedido pendiente: generá uno nuevo si hace falta" : "Estado del pedido: " + r.estado); }) }, "Ver mi último pedido"));
  }
});

// ---------- tareas programadas ----------
// A una hora fija (cada día, semana o mes) el servidor crea una tarea ya asignada. Distinto de «Repetición»,
// que crea la siguiente cuando se completa la anterior.
const DIAS = ["Lunes", "Martes", "Miércoles", "Jueves", "Viernes", "Sábado", "Domingo"];
const hhmm = (min) => String(Math.floor(min / 60)).padStart(2, "0") + ":" + String(min % 60).padStart(2, "0");
const schedText = (s) => s.freq === "daily" ? `Cada día a las ${hhmm(s.at_minute)}`
  : s.freq === "weekly" ? `Cada ${DIAS[s.weekday].toLowerCase()} a las ${hhmm(s.at_minute)}`
  : `Cada mes, el día ${s.monthday}, a las ${hhmm(s.at_minute)}`;

// informes automáticos del equipo (bandeja, Telegram y email)
const openReports = guard(async () => {
  if (!S.team) return;
  closeModal();
  const r = await api("GET", `/api/teams/${S.team.id}/report`);
  const cur = r.sub;
  const freq = h("select", {}, [["daily", "Todos los días"], ["weekly", "Una vez por semana"]].map(([v, l]) => h("option", { value: v, selected: (cur?.freq || "daily") === v }, l)));
  const wd = h("select", {}, DIAS.map((d, i) => h("option", { value: i, selected: (cur?.weekday ?? 0) === i }, d)));
  const hhmm = (m) => `${String(Math.floor(m / 60)).padStart(2, "0")}:${String(m % 60).padStart(2, "0")}`;
  const at = h("input", { type: "time", value: hhmm(cur?.at_minute ?? 540), style: "max-width:130px" });
  const wdL = h("label", {}, "Día", wd);
  const sync = () => { wdL.style.display = freq.value === "weekly" ? "" : "none"; };
  freq.onchange = sync; sync();
  const preview = h("pre", { class: "bot-cfg", style: "display:none;white-space:pre-wrap" });
  const tg = h("input", { type: "checkbox", checked: r.telegram && (cur?.via_telegram ?? true), disabled: !r.telegram });
  const em = h("input", { type: "checkbox", checked: r.email && (cur?.via_email ?? true), disabled: !r.email });
  const body = (active = true) => { const [H, M] = (at.value || "09:00").split(":").map(Number); return { freq: freq.value, weekday: +wd.value, at_minute: H * 60 + M, tz_offset: -new Date().getTimezoneOffset(), active, via_telegram: tg.checked, via_email: em.checked }; };
  document.body.append(h("div", { class: "overlay", id: "modal", onmousedown: (e) => e.target.id === "modal" && closeModal() },
    h("div", { class: "modal" },
      h("h3", {}, "📊 Informes · " + S.team.name, h("span", { class: "spacer" }), h("button", { class: "ghost", onclick: closeModal }, "✕")),
      h("p", { class: "hint" }, "Un resumen con lo que terminaron las personas y las IAs, cuánto gastaron las IAs y qué quedó trabado (en curso hace más de un día, pedidos 🙋 sin responder, vencidas). Siempre queda en la bandeja de la app. Elegí además dónde querés recibirlo:"),
      h("label", { class: "row" }, tg, h("span", {}, r.telegram ? "Telegram" : "Telegram (no lo tenés vinculado: Configuración → Telegram)")),
      h("label", { class: "row" }, em, h("span", {}, r.email ? "Email" : "Email (sin email verificado o el servidor no tiene correo: Configuración → Email)")),
      cur && h("p", {}, cur.active ? "✓ Suscripto. Próximo: " + new Date(cur.next_run).toLocaleString() : "En pausa"),
      h("div", { class: "row" }, h("label", {}, "Frecuencia", freq), wdL, h("label", {}, "Hora", at)),
      h("div", { class: "row" },
        h("button", { class: "primary", onclick: guard(async () => { const x = await api("PUT", `/api/teams/${S.team.id}/report`, body()); toast("Listo. Próximo informe: " + new Date(x.next_run).toLocaleString()); openReports(); }) }, cur ? "Guardar" : "Suscribirme"),
        cur && h("button", { onclick: guard(async () => { await api("PUT", `/api/teams/${S.team.id}/report`, body(!cur.active)); openReports(); }) }, cur.active ? "⏸ Pausar" : "▶ Reanudar"),
        cur && h("button", { class: "danger", onclick: guard(async () => { await api("DELETE", `/api/teams/${S.team.id}/report`); openReports(); }) }, "Quitar"),
        h("span", { class: "spacer" }),
        h("button", { title: "Arma el informe de ahora y te lo manda para que lo veas", onclick: guard(async () => { const x = await api("POST", `/api/teams/${S.team.id}/report/now`, { freq: freq.value, tz_offset: -new Date().getTimezoneOffset(), via_telegram: tg.checked, via_email: em.checked }); toast("Enviado a: " + x.sent.join(", ")); preview.textContent = x.text; preview.style.display = ""; }) }, "Enviarme uno ahora")),
      preview)));
});
const openSchedules = guard(async () => {
  if (!S.project) return;
  closeModal();
  const list = await api("GET", `/api/projects/${S.project.id}/schedules`);
  const assignable = can("admin") ? S.members.filter((m) => RANK[m.role] >= 1) : S.members.filter((m) => m.id === S.user?.id && can("member"));
  const reload = () => openSchedules();
  const body = (s, over = {}) => ({ title: s.title, description: s.description, priority: s.priority, assignee_id: s.assignee_id, freq: s.freq, weekday: s.weekday, monthday: s.monthday, at_minute: s.at_minute, tz_offset: s.tz_offset, skip_if_open: s.skip_if_open, active: s.active, ...over });
  const rows = list.map((s) => h("div", { class: "sched-row" + (s.active ? "" : " off") },
    h("div", {}, h("b", {}, s.title), s.assignee && h("small", { class: "inl" }, " · 👤 " + s.assignee)),
    h("small", { class: "hint" }, schedText(s) + (s.skip_if_open ? " · si la anterior sigue abierta, no crea otra" : "")),
    h("small", { class: "hint" }, s.active ? "Próxima: " + new Date(s.next_run).toLocaleString() : "En pausa"),
    can("member") && h("div", { class: "row" },
      h("button", { title: "Crea la tarea ahora, sin esperar ni mover el próximo horario", onclick: guard(async () => { const r = await api("POST", `/api/schedules/${s.id}/run`); toast(r.task_id ? "Tarea creada" : "No se creó la tarea"); reload(); }) }, "▶ Ejecutar ahora"),
      h("button", { title: "Cambiar el título, la descripción, el responsable o el horario", onclick: () => edit(s) }, "✏ Editar"),
      h("button", { onclick: guard(async () => { await api("PUT", `/api/schedules/${s.id}`, body(s, { active: !s.active })); reload(); }) }, s.active ? "⏸ Pausar" : "▶ Activar"),
      h("button", { class: "danger", onclick: guard(async () => { if (!(await ask("¿Borrar la tarea programada?"))) return; await api("DELETE", `/api/schedules/${s.id}`); reload(); }) }, "🗑 Borrar"))));
  const title = h("input", { placeholder: "Título de la tarea (ej. Chequear resultados)", maxLength: 200 });
  const desc = h("textarea", { placeholder: "Descripción: qué tiene que hacer (opcional)", rows: 3 });
  const who = h("select", {}, h("option", { value: "" }, "Sin asignar"), assignable.map((m) => h("option", { value: m.id }, m.id === S.user?.id && !can("admin") ? `Yo (${m.username})` : label(m.username))));
  const freq = h("select", { onchange: () => sync() }, [["daily", "Cada día"], ["weekly", "Cada semana"], ["monthly", "Cada mes"]].map(([v, l]) => h("option", { value: v }, l)));
  const wd = h("select", {}, DIAS.map((d, i) => h("option", { value: i }, d)));
  const md = h("input", { type: "number", min: 1, max: 31, value: 1, style: "max-width:90px" });
  const at = h("input", { type: "time", value: "09:00", style: "max-width:130px" });
  const skip = h("input", { type: "checkbox", checked: true });
  const wdL = h("label", {}, "Día de la semana", wd), mdL = h("label", {}, "Día del mes (si el mes es más corto, el último día)", md);
  const sync = () => { wdL.style.display = freq.value === "weekly" ? "" : "none"; mdL.style.display = freq.value === "monthly" ? "" : "none"; };
  sync();
  // edición: carga la tarea programada en el formulario de abajo y «Guardar cambios» la actualiza (conserva si está activa o en pausa)
  let editing = null;
  const heading = h("h4", {}, "Programar una tarea nueva");
  const okBtn = h("button", { class: "primary", onclick: () => add() }, "Programar");
  const cancelBtn = h("button", { style: "display:none", onclick: () => reload() }, "Cancelar edición");
  const edit = (s) => {
    editing = s;
    title.value = s.title; desc.value = s.description; who.value = s.assignee_id ?? ""; freq.value = s.freq; wd.value = s.weekday; md.value = s.monthday;
    at.value = hhmm(s.at_minute); skip.checked = !!s.skip_if_open; sync();
    heading.textContent = `Editando: ${s.title}`; okBtn.textContent = "Guardar cambios"; cancelBtn.style.display = "";
    title.scrollIntoView({ block: "center" }); title.focus();
  };
  const add = guard(async () => {
    const [H, M] = (at.value || "09:00").split(":").map(Number);
    const data = { title: title.value, description: desc.value, assignee_id: who.value ? +who.value : null, freq: freq.value, weekday: +wd.value, monthday: +md.value || 1, at_minute: H * 60 + M, tz_offset: -new Date().getTimezoneOffset(), skip_if_open: skip.checked };
    if (editing) {
      const r = await api("PUT", `/api/schedules/${editing.id}`, { ...data, priority: editing.priority, active: editing.active });
      toast("Cambios guardados. Próxima: " + new Date(r.next_run).toLocaleString()); return reload();
    }
    const created = await api("POST", `/api/projects/${S.project.id}/schedules`, data);
    toast("Tarea programada. Se crea el " + new Date(created.next_run).toLocaleString()); reload();
  });
  document.body.append(h("div", { class: "overlay", id: "modal", onmousedown: (e) => e.target.id === "modal" && closeModal() },
    h("div", { class: "modal usage-modal" },
      h("h3", {}, "⏰ Tareas programadas · " + S.project.name, h("span", { class: "spacer" }), h("button", { class: "ghost", onclick: closeModal }, "✕")),
      h("p", { class: "hint" }, "A la hora indicada el servidor crea la tarea ya asignada (por ejemplo, para que una IA chequee resultados cada lunes). La hora es la de tu zona horaria."),
      rows.length ? h("div", { class: "sched-list" }, rows) : h("small", { class: "hint" }, "Todavía no hay tareas programadas en este proyecto."),
      can("member") && heading,
      can("member") && title, can("member") && desc,
      can("member") && h("label", {}, "Responsable", who),
      can("member") && h("div", { class: "row" }, h("label", {}, "Frecuencia", freq), wdL, mdL, h("label", {}, "Hora", at)),
      can("member") && h("label", { class: "row" }, skip, h("span", {}, "Si la tarea anterior sigue abierta, no crear otra")),
      can("member") && h("div", { class: "row" }, okBtn, cancelBtn))));
});

// ventanas: botón para ampliar/restaurar en todas (y se pueden redimensionar desde la esquina)
new MutationObserver((muts) => {
  for (const m of muts) for (const n of m.addedNodes) {
    if (n.nodeType !== 1) continue;
    for (const modal of n.matches?.(".overlay > .modal") ? [n] : n.querySelectorAll?.(".overlay > .modal") || []) {
      if (modal.classList.contains("term-modal") || modal.querySelector(".xbtn")) continue;
      const head = modal.querySelector("h3");
      if (!head) continue;
      const b = h("button", { class: "ghost xbtn", title: "Ampliar o restaurar la ventana", onclick: () => { const on = modal.classList.toggle("xmax"); b.textContent = on ? "⤡" : "⤢"; } }, "⤢");
      const sp = head.querySelector(".spacer");
      sp ? sp.after(b) : head.append(b);
    }
  }
}).observe(document.body, { childList: true, subtree: true });
