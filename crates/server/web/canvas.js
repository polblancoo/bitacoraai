"use strict";
// Vistas gráficas: Gantt con dependencias, pizarra por proyecto y mapa mental de una tarea.
// Se carga después de app.js y usa sus globales (S, h, api, guard, can, openTask…).

const SVGNS = "http://www.w3.org/2000/svg";
function sv(tag, attrs = {}, ...kids) {
  const el = document.createElementNS(SVGNS, tag);
  for (const [k, v] of Object.entries(attrs)) if (v != null && v !== false) el.setAttribute(k, v);
  for (const kid of kids.flat()) if (kid != null && kid !== false) el.append(kid);
  return el;
}
const DAY = 86400000;
const STATUS_COLOR = { todo: "var(--blue)", doing: "var(--yellow)", done: "var(--green)" };
const clip = (s, n) => (s.length > n ? s.slice(0, n - 1) + "…" : s);

// ---------- Gantt ----------
function ganttView() {
  const q = S.query.toLowerCase();
  const all = S.tasks.filter((t) => !t.parent_id && (!q || t.title.toLowerCase().includes(q)));
  const dated = all.filter((t) => t.due).sort((a, b) => new Date(a.start || a.created_at) - new Date(b.start || b.created_at));
  const undated = all.filter((t) => !t.due);
  if (!dated.length) {
    return h("div", { class: "empty" }, "Ninguna tarea tiene fecha de vencimiento. Ponele “Vence” (y opcionalmente “Empieza”) desde la tarea para verla acá.");
  }
  const begin = (t) => new Date(t.start || t.created_at);
  const end = (t) => new Date(t.due);
  const min = new Date(Math.min(...dated.map((t) => begin(t).getTime()), Date.now()));
  min.setHours(0, 0, 0, 0); min.setTime(min.getTime() - 2 * DAY);
  const max = new Date(Math.max(...dated.map((t) => end(t).getTime()), Date.now()) + 3 * DAY);
  const days = Math.ceil((max - min) / DAY);
  const DW = 30, RH = 34, LW = 190, HEAD = 36;
  const width = LW + days * DW, height = HEAD + dated.length * RH + 8;
  const xOf = (d) => LW + ((d - min) / DAY) * DW;
  const rowOf = new Map(dated.map((t, i) => [t.id, i]));
  const svg = sv("svg", { width, height, class: "gantt" });

  // cuadrícula y cabecera por día
  for (let i = 0; i <= days; i++) {
    const d = new Date(min.getTime() + i * DAY);
    const x = LW + i * DW;
    const weekend = d.getDay() === 0 || d.getDay() === 6;
    if (weekend) svg.append(sv("rect", { x, y: HEAD, width: DW, height: height - HEAD, class: "g-weekend" }));
    svg.append(sv("line", { x1: x, x2: x, y1: HEAD - 8, y2: height, class: "g-grid" }));
    if (d.getDate() === 1 || i === 0) svg.append(sv("text", { x: x + 3, y: 12, class: "g-month" }, d.toLocaleDateString(undefined, { month: "short", year: "2-digit" })));
    if (DW >= 24) svg.append(sv("text", { x: x + DW / 2, y: HEAD - 12, class: "g-day", "text-anchor": "middle" }, String(d.getDate())));
  }
  const tx = xOf(new Date());
  svg.append(sv("line", { x1: tx, x2: tx, y1: HEAD - 8, y2: height, class: "g-today" }));

  // flechas de dependencia (dibujadas bajo las barras)
  const arrows = sv("g", {});
  svg.append(sv("defs", {}, sv("marker", { id: "arr", viewBox: "0 0 8 8", refX: 7, refY: 4, markerWidth: 7, markerHeight: 7, orient: "auto" },
    sv("path", { d: "M0,0 L8,4 L0,8 z", fill: "var(--purple)" }))));
  svg.append(arrows);

  const drawArrows = (override = {}) => {
    arrows.replaceChildren();
    for (const t of dated) {
      for (const dep of t.depends_on || []) {
        const from = dated.find((x) => x.id === dep);
        if (!from) continue;
        const o1 = override[from.id] || 0, o2 = override[t.id] || 0;
        const x1 = xOf(end(from)) + DW / 2 + o1, y1 = HEAD + rowOf.get(from.id) * RH + RH / 2;
        const x2 = xOf(begin(t)) + o2, y2 = HEAD + rowOf.get(t.id) * RH + RH / 2;
        const mx = Math.max(x1 + 10, x2 - 10);
        const late = x2 < x1;
        arrows.append(sv("path", { d: `M${x1},${y1} H${mx} V${y2} H${x2 - 2}`, class: "g-arrow" + (late ? " bad" : ""), "marker-end": "url(#arr)" }));
      }
    }
  };

  dated.forEach((t, i) => {
    const y = HEAD + i * RH;
    const x1 = xOf(begin(t)), x2 = Math.max(xOf(end(t)) + DW / 2, x1 + 12);
    const g = sv("g", { class: "g-row" });
    g.append(sv("rect", { x: 0, y, width, height: RH, class: i % 2 ? "g-stripe" : "g-clear" }));
    const label = sv("text", { x: 8, y: y + RH / 2 + 4, class: "g-label" }, clip(t.title, 26));
    label.append(sv("title", {}, t.title));
    g.append(label);
    const late = t.status !== "done" && end(t) < new Date();
    const bar = sv("rect", { x: x1, y: y + 7, width: x2 - x1, height: RH - 14, rx: 6, class: "g-bar", fill: STATUS_COLOR[t.status], stroke: late ? "var(--red)" : "none", "stroke-width": 2 });
    const pct = t.subtasks_total ? t.subtasks_done / t.subtasks_total : t.status === "done" ? 1 : 0;
    const prog = sv("rect", { x: x1, y: y + 7, width: (x2 - x1) * pct, height: RH - 14, rx: 6, class: "g-prog" });
    const who = t.assignee ? sv("text", { x: x1 + 6, y: y + RH / 2 + 4, class: "g-who" }, clip(t.assignee, 12)) : null;
    bar.append(sv("title", {}, `${t.title}\n${begin(t).toLocaleDateString()} → ${end(t).toLocaleDateString()}${t.assignee ? "\n@" + t.assignee : ""}`));
    g.append(bar, prog, who);

    // arrastrar la barra = mover empieza y vence (solo quien puede editar)
    if (can("member")) {
      bar.style.cursor = "grab";
      bar.addEventListener("pointerdown", (e) => {
        e.preventDefault(); bar.setPointerCapture(e.pointerId);
        const x0 = e.clientX; let dx = 0; let moved = false;
        const move = (ev) => {
          dx = Math.round((ev.clientX - x0) / DW) * DW; moved = moved || Math.abs(ev.clientX - x0) > 3;
          for (const el of [bar, prog, who]) if (el) el.setAttribute("transform", `translate(${dx},0)`);
          drawArrows({ [t.id]: dx });
        };
        const up = guard(async () => {
          bar.removeEventListener("pointermove", move); bar.removeEventListener("pointerup", up);
          if (!moved) return openTask(t);
          const days = dx / DW;
          if (!days) return render();
          await patchTask(t.id, { start: new Date(begin(t).getTime() + days * DAY).toISOString(), due: new Date(end(t).getTime() + days * DAY).toISOString() });
        });
        bar.addEventListener("pointermove", move); bar.addEventListener("pointerup", up);
      });
    } else bar.addEventListener("click", () => openTask(t));
    svg.append(g);
  });
  // las flechas van encima de las filas pero bajo las barras: se re-insertan al final
  svg.append(arrows);
  drawArrows();

  return h("div", { class: "gantt-wrap" },
    h("div", { class: "gantt-scroll" }, svg),
    h("div", { class: "gantt-foot" },
      h("small", {}, "Arrastrá una barra para moverla · las flechas moradas son dependencias (rojas: empieza antes de que termine la anterior) · borde rojo: vencida"),
      undated.length > 0 && h("small", {}, `${undated.length} sin fecha: ${undated.slice(0, 6).map((t) => t.title).join(", ")}${undated.length > 6 ? "…" : ""}`)));
}

// ---------- Pizarra ----------
const NOTE_COLORS = ["#e0af68", "#9ece6a", "#7aa2f7", "#bb9af7", "#f7768e", "#7dcfff"];
const CV = { linking: null, drag: false };

async function loadCanvas() {
  if (!S.project) return;
  const id = S.project.id;
  const c = await api("GET", `/api/projects/${id}/canvas`);
  if (S.project?.id === id) { S.canvas = { ...c, project_id: id }; render(); }
}

function canvasView() {
  if (!S.canvas || S.canvas.project_id !== S.project.id) {
    S.canvas = { project_id: S.project.id, items: [], links: [], loading: true };
    loadCanvas().catch((e) => toast(e.message, true));
  }
  const edit = can("member");
  const { items, links } = S.canvas;
  const byId = new Map(items.map((i) => [i.id, i]));
  const board = h("div", { class: "cv-board" });
  const svg = sv("svg", { class: "cv-links", width: 3000, height: 2000 });
  svg.append(sv("defs", {}, sv("marker", { id: "cvarr", viewBox: "0 0 8 8", refX: 7, refY: 4, markerWidth: 8, markerHeight: 8, orient: "auto" },
    sv("path", { d: "M0,0 L8,4 L0,8 z", fill: "var(--fg)" }))));
  board.append(svg);

  const center = (it) => [it.x + it.w / 2, it.y + it.h / 2];
  const edge = (a, b) => {
    // punto del borde del rectángulo `a` en dirección al centro de `b`
    const [ax, ay] = center(a), [bx, by] = center(b);
    const dx = bx - ax, dy = by - ay;
    if (!dx && !dy) return [ax, ay];
    const k = Math.min((a.w / 2) / Math.abs(dx || 1e-9), (a.h / 2) / Math.abs(dy || 1e-9));
    return [ax + dx * k, ay + dy * k];
  };
  const lineEls = new Map();
  const drawLinks = () => {
    for (const l of links) {
      const a = byId.get(l.from_id), b = byId.get(l.to_id); if (!a || !b) continue;
      const [x1, y1] = edge(a, b), [x2, y2] = edge(b, a);
      let g = lineEls.get(l.id);
      if (!g) {
        g = sv("g", {}, sv("line", { class: "cv-line", "marker-end": "url(#cvarr)" }), sv("line", { class: "cv-hit" }));
        if (edit) g.lastChild.addEventListener("click", guard(async () => {
          if (await ask("¿Borrar la conexión?")) await api("DELETE", `/api/canvas/links/${l.id}`);
        }));
        svg.append(g); lineEls.set(l.id, g);
      }
      for (const ln of g.children) { ln.setAttribute("x1", x1); ln.setAttribute("y1", y1); ln.setAttribute("x2", x2); ln.setAttribute("y2", y2); }
    }
  };

  for (const it of items) {
    const task = it.kind === "task" ? S.tasks.find((t) => t.id === it.task_id) : null;
    const el = h("div", {
      class: "cv-item " + it.kind + (CV.linking === it.id ? " linking" : ""),
      style: `left:${it.x}px;top:${it.y}px;width:${it.w}px;height:${it.h}px;--c:${it.color}`,
    });
    if (task) {
      el.append(h("div", { class: "cv-task-title" }, task.title),
        h("div", { class: "meta" }, h("span", { class: `pill p${task.priority}` }, PRIO[task.priority]),
          h("span", { class: "pill st", style: `color:${STATUS_COLOR[task.status]}` }, STATUS.find((s) => s.id === task.status).label),
          task.assignee && avatar(task.assignee)));
    } else if (it.kind === "task") {
      el.append(h("div", { class: "cv-task-title dim" }, "(tarea borrada)"));
    } else {
      const ta = h("textarea", { value: it.text, readOnly: !edit, maxLength: 2000, placeholder: "Nota…" });
      ta.addEventListener("pointerdown", (e) => e.stopPropagation());
      ta.addEventListener("change", guard(() => api("PATCH", `/api/canvas/items/${it.id}`, { text: ta.value })));
      el.append(ta);
    }
    if (edit) {
      const bar = h("div", { class: "cv-tools" },
        h("button", { title: "Conectar con otro elemento", onclick: (e) => { e.stopPropagation(); CV.linking = CV.linking === it.id ? null : it.id; render(); } }, "↗"),
        it.kind === "note" && h("button", { title: "Cambiar color", onclick: guard(async (e) => {
          e.stopPropagation(); const i = NOTE_COLORS.indexOf(it.color);
          await api("PATCH", `/api/canvas/items/${it.id}`, { color: NOTE_COLORS[(i + 1) % NOTE_COLORS.length] });
        }) }, "◐"),
        h("button", { title: "Quitar de la pizarra", onclick: guard(async (e) => { e.stopPropagation(); await api("DELETE", `/api/canvas/items/${it.id}`); }) }, "✕"));
      el.append(bar);
      el.addEventListener("pointerdown", (e) => {
        if (e.button !== 0 || e.target.closest(".cv-tools")) return;
        if (CV.linking && CV.linking !== it.id) {
          const from = CV.linking; CV.linking = null;
          api("POST", `/api/projects/${S.project.id}/canvas/links`, { from_id: from, to_id: it.id }).catch((er) => toast(er.message, true));
          return;
        }
        const x0 = e.clientX, y0 = e.clientY, ix = it.x, iy = it.y; let moved = false;
        el.setPointerCapture(e.pointerId); CV.drag = true;
        const move = (ev) => {
          if (Math.abs(ev.clientX - x0) + Math.abs(ev.clientY - y0) > 3) moved = true;
          it.x = Math.max(0, ix + ev.clientX - x0); it.y = Math.max(0, iy + ev.clientY - y0);
          el.style.left = it.x + "px"; el.style.top = it.y + "px"; drawLinks();
        };
        const up = guard(async () => {
          CV.drag = false; el.removeEventListener("pointermove", move); el.removeEventListener("pointerup", up);
          if (moved) await api("PATCH", `/api/canvas/items/${it.id}`, { x: Math.round(it.x), y: Math.round(it.y) });
          else if (task) openTask(task);
        });
        el.addEventListener("pointermove", move); el.addEventListener("pointerup", up);
      });
    } else if (task) el.addEventListener("click", () => openTask(task));
    board.append(el);
  }
  drawLinks();
  if (edit) board.addEventListener("dblclick", guard(async (e) => {
    if (e.target !== board && e.target !== svg) return;
    const r = board.getBoundingClientRect();
    await api("POST", `/api/projects/${S.project.id}/canvas/items`, { kind: "note", x: Math.round(e.clientX - r.left - 100), y: Math.round(e.clientY - r.top - 50), text: "", color: NOTE_COLORS[items.length % NOTE_COLORS.length] });
  }));

  const addNote = guard(async () => {
    const n = items.filter((i) => i.kind === "note").length;
    await api("POST", `/api/projects/${S.project.id}/canvas/items`, { kind: "note", x: 40 + (n % 6) * 30, y: 40 + (n % 6) * 30, text: "", color: NOTE_COLORS[n % NOTE_COLORS.length] });
  });
  const onBoard = new Set(items.filter((i) => i.kind === "task").map((i) => i.task_id));
  const pick = h("select", { onchange: guard(async (e) => {
    if (!e.target.value) return;
    const n = items.length;
    await api("POST", `/api/projects/${S.project.id}/canvas/items`, { kind: "task", task_id: +e.target.value, x: 260 + (n % 5) * 40, y: 40 + (n % 5) * 40, color: "#7aa2f7", h: 80 });
  }) }, h("option", { value: "" }, "+ Tarjeta de tarea…"),
    S.tasks.filter((t) => !t.parent_id && !onBoard.has(t.id)).map((t) => h("option", { value: t.id }, clip(t.title, 40))));

  return h("div", { class: "cv" },
    h("div", { class: "cv-bar" },
      edit && h("button", { onclick: addNote }, "+ Nota"), edit && pick,
      CV.linking && h("span", { class: "cv-hint" }, "Elegí el elemento destino de la conexión (↗ otra vez cancela)"),
      h("span", { class: "spacer" }),
      h("small", {}, `${items.length} elementos · ${links.length} conexiones · se actualiza en vivo`)),
    h("div", { class: "cv-scroll" }, board));
}

// ---------- Mapa mental de una tarea ----------
function openMindmap(task) {
  document.getElementById("mm-modal")?.remove();
  const W = 900, H = 560, cx = W / 2, cy = H / 2;
  const subs = S.tasks.filter((t) => t.parent_id === task.id);
  const deps = (task.depends_on || []).map((id) => S.tasks.find((t) => t.id === id)).filter(Boolean);
  const dependents = S.tasks.filter((t) => (t.depends_on || []).includes(task.id));
  const branches = [
    { name: "Subtareas", color: "var(--green)", kids: subs.map((s) => ({ text: (s.status === "done" ? "☑ " : "☐ ") + s.title, task: s })) },
    { name: "Etiquetas", color: "var(--purple)", kids: task.labels.map((l) => ({ text: "#" + l.name })) },
    { name: "Depende de", color: "var(--red)", kids: deps.map((d) => ({ text: d.title, task: d })) },
    { name: "Bloquea a", color: "var(--yellow)", kids: dependents.map((d) => ({ text: d.title, task: d })) },
    { name: "Datos", color: "var(--cyan)", kids: [
      { text: "Estado: " + STATUS.find((s) => s.id === task.status).label }, { text: "Prioridad: " + PRIO[task.priority] },
      { text: "Responsable: " + (task.assignee || "sin asignar") },
      task.due && { text: "Vence: " + new Date(task.due).toLocaleDateString() },
      task.start && { text: "Empieza: " + new Date(task.start).toLocaleDateString() }].filter(Boolean) },
  ].filter((b) => b.kids.length);

  const svg = sv("svg", { viewBox: `0 0 ${W} ${H}`, class: "mm" });
  const node = (x, y, text, cls, color, onclick, anchor = "middle") => {
    const g = sv("g", { class: "mm-node " + cls, style: onclick ? "cursor:pointer" : null });
    const t = sv("text", { x, y: y + 4, "text-anchor": anchor }, clip(text, 34));
    g.append(sv("title", {}, text), t);
    svg.append(g);
    const bb = { w: Math.min(text.length, 34) * 6.6 + 18, h: 24 };
    const rx = anchor === "start" ? x - 9 : anchor === "end" ? x - bb.w + 9 : x - bb.w / 2;
    g.prepend(sv("rect", { x: rx, y: y - 12, width: bb.w, height: bb.h, rx: 12, stroke: color, fill: "var(--card)" }));
    if (onclick) g.addEventListener("click", onclick);
    return g;
  };
  const n = branches.length;
  branches.forEach((b, i) => {
    const ang = (-Math.PI / 2) + (i + 0.5) * (2 * Math.PI / n) - Math.PI / n;
    const bx = cx + Math.cos(ang) * 150 * 1.5, by = cy + Math.sin(ang) * 150;
    svg.prepend(sv("path", { d: `M${cx},${cy} Q${(cx + bx) / 2},${cy} ${bx},${by}`, class: "mm-edge", stroke: b.color }));
    const side = Math.cos(ang) >= 0 ? 1 : -1;
    b.kids.forEach((k, j) => {
      const span = Math.min(b.kids.length, 6);
      const ky = by + (j - (b.kids.length - 1) / 2) * Math.min(30, 150 / span + 8);
      const kx = bx + side * 70;
      svg.prepend(sv("path", { d: `M${bx},${by} C${bx + side * 40},${by} ${kx - side * 20},${ky} ${kx},${ky}`, class: "mm-edge", stroke: b.color }));
      node(kx, ky, k.text, "leaf", b.color, k.task ? () => { document.getElementById("mm-modal")?.remove(); openTask(k.task); } : null, side > 0 ? "start" : "end");
    });
    node(bx, by, b.name, "branch", b.color);
  });
  node(cx, cy, task.title, "root", "var(--blue)");
  if (!n) svg.append(sv("text", { x: cx, y: cy + 40, "text-anchor": "middle", class: "mm-empty" }, "Sin subtareas, etiquetas ni dependencias todavía"));

  document.body.append(h("div", { class: "overlay", id: "mm-modal", onmousedown: (e) => e.target.id === "mm-modal" && e.target.remove() },
    h("div", { class: "modal wide" }, h("h3", {}, "Mapa mental"), svg,
      h("div", { class: "row" }, h("button", { onclick: () => document.getElementById("mm-modal")?.remove() }, "Cerrar")))));
}
