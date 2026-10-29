use crate::i18n::{t, tf};
use crate::{App, Mode, PromptKind};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use tm_core::Status;

const BG: Color = Color::Rgb(26, 27, 38);
const FG: Color = Color::Rgb(192, 202, 245);
const DIM: Color = Color::Rgb(86, 95, 137);
const BLUE: Color = Color::Rgb(122, 162, 247);
const GREEN: Color = Color::Rgb(158, 206, 106);
const YELLOW: Color = Color::Rgb(224, 175, 104);
const RED: Color = Color::Rgb(247, 118, 142);
const PURPLE: Color = Color::Rgb(187, 154, 247);

fn block(title: &str, active: bool) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(if active { BLUE } else { DIM }))
        .title(Span::styled(format!(" {title} "), Style::default().fg(if active { BLUE } else { FG }).add_modifier(Modifier::BOLD)))
        .style(Style::default().bg(BG).fg(FG))
}

fn prio(p: i64) -> (&'static str, Color) {
    match p {
        0 => ("▽", DIM),
        1 => ("●", BLUE),
        2 => ("▲", YELLOW),
        _ => ("‼", RED),
    }
}

fn hex(c: &str) -> Color {
    let c = c.trim_start_matches('#');
    match (u8::from_str_radix(c.get(0..2).unwrap_or("0"), 16), u8::from_str_radix(c.get(2..4).unwrap_or("0"), 16), u8::from_str_radix(c.get(4..6).unwrap_or("0"), 16)) {
        (Ok(r), Ok(g), Ok(b)) => Color::Rgb(r, g, b),
        _ => PURPLE,
    }
}

fn centered(area: Rect, w: u16, h: u16) -> Rect {
    let w = w.min(area.width);
    let h = h.min(area.height);
    Rect::new(area.x + (area.width - w) / 2, area.y + (area.height - h) / 2, w, h)
}

pub fn draw(f: &mut Frame, app: &App) {
    f.render_widget(Block::default().style(Style::default().bg(BG)), f.area());
    match app.mode {
        Mode::Login => return draw_login(f, app),
        _ => {}
    }

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5), Constraint::Length(1)])
        .split(f.area());

    draw_header(f, app, rows[0]);
    draw_board(f, app, rows[1]);
    draw_footer(f, app, rows[2]);

    match app.mode {
        Mode::Detail => draw_detail(f, app),
        Mode::Prompt(kind) => draw_prompt(f, app, kind),
        Mode::Help => draw_help(f, app),
        Mode::Inbox => draw_inbox(f, app),
        Mode::Config => draw_config(f, app),
        Mode::Prefix => {
            let area = Rect::new(0, f.area().height.saturating_sub(1), f.area().width, 1);
            f.render_widget(Clear, area);
            f.render_widget(
                Paragraph::new(Span::styled(
                    t(" Ctrl+T ▸ m miembro · r rol · i invitar · j unirse · t nuevo equipo · e email · v verificar · s mensaje · g Telegram · n bandeja · c config · Esc cancela"),
                    Style::default().fg(BG).bg(YELLOW),
                )),
                area,
            );
        }
        _ => {}
    }
}

fn draw_login(f: &mut Frame, app: &App) {
    let area = centered(f.area(), 52, 13);
    let labels = [t("Servidor"), t("Usuario"), t("Contraseña")];
    let mut lines = vec![Line::from(Span::styled(
        if app.register { t("Crear cuenta") } else { t("Iniciar sesión") },
        Style::default().fg(PURPLE).add_modifier(Modifier::BOLD),
    ))];
    lines.push(Line::raw(""));
    for (i, l) in labels.iter().enumerate() {
        let val = if i == 2 { "•".repeat(app.login[i].chars().count()) } else { app.login[i].clone() };
        let cur = if app.field == i { "▏" } else { "" };
        let st = if app.field == i { Style::default().fg(BLUE) } else { Style::default().fg(DIM) };
        lines.push(Line::from(vec![Span::styled(format!("{l:>11}  "), st), Span::raw(format!("{val}{cur}"))]));
    }
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(t("Enter entrar · Tab campo · F2 login/registro · Esc salir"),Style::default().fg(DIM))));
    if !app.status.is_empty() {
        lines.push(Line::from(Span::styled(app.status.clone(), Style::default().fg(RED))));
    }
    f.render_widget(Paragraph::new(lines).block(block("rust-tui-manager", true)).alignment(Alignment::Left), area);
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let team = app.teams.get(app.team).map(|t| t.name.as_str()).unwrap_or("—");
    let mut spans = vec![Span::styled(format!(" {team} "), Style::default().fg(BG).bg(PURPLE).add_modifier(Modifier::BOLD)), Span::raw("  ")];
    for (i, p) in app.projects.iter().enumerate() {
        let st = if i == app.project {
            Style::default().fg(BG).bg(BLUE).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(FG)
        };
        spans.push(Span::styled(format!(" {} ", p.name), st));
        spans.push(Span::raw(" "));
    }
    if app.projects.is_empty() {
        spans.push(Span::styled(t("sin proyectos — P para crear uno"),Style::default().fg(DIM)));
    }
    let who = app.me.as_ref().map(|u| u.username.as_str()).unwrap_or("");
    let dot = if app.online { Span::styled("● ", Style::default().fg(GREEN)) } else { Span::styled("○ ", Style::default().fg(RED)) };
    let members: Vec<String> = app
        .members
        .iter()
        .map(|m| match m.role {
            tm_core::Role::Owner => format!("{}★", m.username),
            tm_core::Role::Admin => format!("{}⚙", m.username),
            tm_core::Role::Viewer => format!("{}👁", m.username),
            tm_core::Role::Member => m.username.clone(),
        })
        .collect();
    let title = tf("{} · equipo: {}", &[who, &members.join(", ")]);
    f.render_widget(
        Paragraph::new(Line::from(spans)).block(block(&title, false).title_top(Line::from(dot).right_aligned())),
        area,
    );
}

fn draw_board(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 3); 3])
        .split(area);
    for (c, st) in Status::ALL.iter().enumerate() {
        let tasks = app.column(c);
        let items: Vec<ListItem> = tasks
            .iter()
            .map(|t| {
                let (sym, col) = prio(t.priority);
                let mut l = vec![Span::styled(format!("{sym} "), Style::default().fg(col)), Span::raw(t.title.clone())];
                if t.subtasks_total > 0 {
                    let c = if t.subtasks_done == t.subtasks_total { GREEN } else { DIM };
                    l.push(Span::styled(format!("  ☑ {}/{}", t.subtasks_done, t.subtasks_total), Style::default().fg(c)));
                }
                for lb in &t.labels {
                    l.push(Span::styled(format!(" #{}", lb.name), Style::default().fg(hex(&lb.color))));
                }
                if let Some(a) = &t.assignee {
                    l.push(Span::styled(format!("  @{a}"), Style::default().fg(PURPLE)));
                }
                ListItem::new(Line::from(l))
            })
            .collect();
        let active = app.col == c;
        let title = format!("{} ({})", t(st.label()), tasks.len());
        let list = List::new(items)
            .block(block(&title, active))
            .highlight_style(Style::default().bg(Color::Rgb(41, 46, 66)).add_modifier(Modifier::BOLD))
            .highlight_symbol("▌");
        let mut state = ListState::default();
        if active && !tasks.is_empty() {
            state.select(Some(app.row[c]));
        }
        f.render_stateful_widget(list, cols[c], &mut state);
    }
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let text = if app.status.is_empty() {
        {
            let bell = if app.unread > 0 { format!("🔔{} (N) · ", app.unread) } else { String::new() };
            format!("{bell}{}", t("n tarea · Enter detalle · H/L mover · +/- prioridad · a tomar/soltar · A asignar · C configuración · Ctrl+T equipo · ? ayuda · q salir"))
        }
    } else {
        app.status.clone()
    };
    let color = if app.status.is_empty() { DIM } else if app.status.starts_with('ℹ') { GREEN } else { RED };
    f.render_widget(Paragraph::new(Span::styled(text, Style::default().fg(color))), area);
}

fn draw_detail(f: &mut Frame, app: &App) {
    let Some(t) = app.selected() else { return };
    let area = centered(f.area(), 90, 26);
    f.render_widget(Clear, area);
    let (sym, col) = prio(t.priority);
    let mut lines = vec![
        Line::from(Span::styled(t.title.clone(), Style::default().fg(BLUE).add_modifier(Modifier::BOLD))),
        Line::from(vec![
            Span::styled(format!("{sym} {}", tf("prioridad {} ({})", &[&t.priority.to_string(), &crate::i18n::t(["baja", "normal", "alta", "urgente"][t.priority.clamp(0, 3) as usize])])), Style::default().fg(col)),
            Span::raw("  ·  "),
            Span::raw(crate::i18n::t(t.status.label())),
            Span::raw("  ·  "),
            Span::styled(t.assignee.as_deref().map(|a| format!("@{a}")).unwrap_or_else(|| crate::i18n::t("sin asignar")), Style::default().fg(PURPLE)),
        ]),
        Line::raw(""),
    ];
    if !t.labels.is_empty() {
        lines.push(Line::from(
            t.labels.iter().map(|l| Span::styled(format!("#{} ", l.name), Style::default().fg(hex(&l.color)))).collect::<Vec<_>>(),
        ));
    }
    if !t.description.is_empty() {
        lines.push(Line::raw(t.description.clone()));
        lines.push(Line::raw(""));
    }
    let subs = app.subtasks(t.id);
    if !subs.is_empty() {
        lines.push(Line::from(Span::styled(tf("Subtareas ({}/{})", &[&t.subtasks_done.to_string(), &t.subtasks_total.to_string()]), Style::default().fg(YELLOW))));
        for (i, st) in subs.iter().enumerate() {
            let done = st.status == Status::Done;
            let mut style = Style::default().fg(if done { DIM } else { FG });
            if i == app.sub { style = style.bg(Color::Rgb(41, 46, 66)); }
            lines.push(Line::from(Span::styled(format!(" {} {}", if done { "☑" } else { "☐" }, st.title), style)));
        }
        lines.push(Line::raw(""));
    }
    lines.push(Line::from(Span::styled(tf("Comentarios ({})", &[&app.comments.len().to_string()]), Style::default().fg(YELLOW))));
    for c in &app.comments {
        lines.push(Line::from(vec![
            Span::styled(format!("{} ", c.created_at.format("%d/%m %H:%M")), Style::default().fg(DIM)),
            Span::styled(format!("{}: ", c.username), Style::default().fg(GREEN)),
            Span::raw(c.body.clone()),
        ]));
    }
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(crate::i18n::t("s subtarea · j/k elegir · Espacio marcar · d borrar subtarea · l etiqueta · c comentar · Esc volver"),Style::default().fg(DIM))));
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }).block(block(&crate::i18n::t("Detalle"), true)), area);
}

fn draw_prompt(f: &mut Frame, app: &App, kind: PromptKind) {
    if app.return_mode_detail {
        draw_detail(f, app);
    }
    let title = match kind {
        PromptKind::NewTask => "Nueva tarea",
        PromptKind::NewProject => "Nuevo proyecto",
        PromptKind::NewTeam => "Nuevo equipo",
        PromptKind::AddMember => "Agregar miembro (escribí para buscar)",
        PromptKind::Comment => "Comentario",
        PromptKind::EditTitle => "Editar título",
        PromptKind::Subtask => "Nueva subtarea",
        PromptKind::JoinCode => "Código de invitación",
        PromptKind::SetRole => "Cambiar rol: usuario rol (viewer | member | admin)",
        PromptKind::AssignTo => "Asignar a (Tab completa · - quita la asignación)",
        PromptKind::SetEmail => "Tu email (para recibir avisos)",
        PromptKind::VerifyEmail => "Código de 6 dígitos que llegó a tu email",
        PromptKind::TeamMessage => "Mensaje para todo el equipo (bandeja, email y Telegram)",
        PromptKind::Label => "Etiqueta (existente: alterna · nueva: se crea)",
    };
    let sug = matches!(kind, PromptKind::AddMember | PromptKind::AssignTo);
    let extra = if sug { (app.suggest.len() as u16).min(8) + 1 } else { 0 };
    let area = centered(f.area(), 64, 3 + extra);
    f.render_widget(Clear, area);
    let mut lines = vec![Line::raw(format!("{}▏", app.input))];
    if sug {
        for (i, s) in app.suggest.iter().enumerate() {
            let st = if i == app.sug_sel { Style::default().fg(BG).bg(BLUE) } else { Style::default().fg(FG) };
            lines.push(Line::from(Span::styled(format!(" {s} "), st)));
        }
        let hint = if kind == PromptKind::AddMember {
            tf("↑/↓ elegir · Tab completa · Ctrl+R rol: {} · Enter agrega", &[&t(app.sug_role.label())])
        } else {
            t("↑/↓ elegir · Tab completa · Enter asigna")
        };
        lines.push(Line::from(Span::styled(hint, Style::default().fg(DIM))));
    }
    f.render_widget(Paragraph::new(lines).block(block(&t(title), true)), area);
}

fn draw_help(f: &mut Frame, app: &App) {
    let rows = [
        ("h/l ←/→", "cambiar columna"), ("j/k ↑/↓", "mover selección"), ("Enter", "detalle y comentarios"),
        ("n", "nueva tarea"), ("e", "editar título"), ("d", "borrar tarea"),
        ("H / L", "mover tarea de estado"), ("+ / -", "subir / bajar prioridad"),
        ("a", "tomar la tarea / soltar la mía"), ("A", "asignar a un miembro (admin o dueño)"),
        ("Tab / S-Tab", "proyecto siguiente / anterior"), ("[ / ]", "equipo anterior / siguiente"),
        ("P", "nuevo proyecto"), ("T", "nuevo equipo"),
        ("m", "agregar miembro con búsqueda (admin+)"), ("R", "cambiar rol de un miembro (admin+)"),
        ("I", "generar código de invitación (admin+)"), ("J", "unirse con un código"),
        ("Ctrl+T m", "agregar miembro (↑/↓ Tab Ctrl+R rol)"), ("Ctrl+T r", "cambiar rol"),
        ("Ctrl+T i", "código de invitación"), ("Ctrl+T j", "unirse con código"), ("Ctrl+T t", "nuevo equipo"),
        ("Ctrl+T e", "cargar mi email para recibir avisos"), ("Ctrl+T v", "verificar el código del email"),
        ("Ctrl+T n / N", "bandeja: novedades y mis tareas"),
        ("C / Ctrl+T c", "menú de configuración (email, Telegram, equipo, sesión)"),
        ("Ctrl+T s", "enviar un mensaje a todo el equipo"), ("Ctrl+T g", "vincular Telegram (te da el código para el bot)"),
        ("l (detalle)", "poner/quitar etiqueta (la crea si no existe)"), ("s (detalle)", "nueva subtarea"),
        ("Espacio", "marcar subtarea (detalle)"), ("c (detalle)", "comentar"),
        ("X", "cerrar sesión en todos los dispositivos"), ("?", "esta ayuda (escribí para buscar)"), ("q", "salir"),
    ];
    let q = app.help_q.to_lowercase();
    let rows: Vec<(String, String)> = rows.iter().map(|(k, d)| (t(k).replace("(detalle)", &t("(detalle)")).replace("Espacio", &t("Espacio")), t(d))).collect();
    let shown: Vec<&(String, String)> = rows.iter().filter(|(k, d)| q.is_empty() || k.to_lowercase().contains(&q) || d.to_lowercase().contains(&q)).collect();
    let mut lines = vec![Line::from(vec![Span::styled(t("buscar: "),Style::default().fg(DIM)), Span::raw(format!("{}▏", app.help_q))]), Line::raw("")];
    if shown.is_empty() {
        lines.push(Line::from(Span::styled(t("sin coincidencias"),Style::default().fg(DIM))));
    }
    lines.extend(shown.iter().map(|(k, d)| Line::from(vec![Span::styled(format!("{k:>14}  "), Style::default().fg(BLUE)), Span::raw(d.clone())])));
    let h = (lines.len() as u16 + 2).max(8);
    let area = centered(f.area(), 68, h);
    f.render_widget(Clear, area);
    f.render_widget(Paragraph::new(lines).block(block(&t("Atajos · Esc cierra"), true)), area);
}

fn draw_inbox(f: &mut Frame, app: &App) {
    let Some(inb) = &app.inbox else { return };
    let mut lines = vec![Line::from(Span::styled(
        tf("Novedades ({} sin leer)", &[&inb.unread.max(app.unread).to_string()]),
        Style::default().fg(YELLOW).add_modifier(Modifier::BOLD),
    ))];
    if inb.notifications.is_empty() {
        lines.push(Line::from(Span::styled(t("  nada nuevo"),Style::default().fg(DIM))));
    }
    for n in inb.notifications.iter().take(8) {
        let (mark, st) = if n.read { ("  ", Style::default().fg(DIM)) } else { ("● ", Style::default().fg(FG)) };
        lines.push(Line::from(vec![
            Span::styled(mark, Style::default().fg(GREEN)),
            Span::styled(format!("{} ", n.created_at.format("%d/%m %H:%M")), Style::default().fg(DIM)),
            Span::styled(n.text.clone(), st),
        ]));
    }
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(tf("Mis tareas abiertas ({})", &[&inb.tasks.len().to_string()]), Style::default().fg(YELLOW).add_modifier(Modifier::BOLD))));
    for (i, it) in inb.tasks.iter().enumerate() {
        let (sym, col) = prio(it.task.priority);
        let sel = if i == app.inbox_sel { Style::default().bg(Color::Rgb(41, 46, 66)).add_modifier(Modifier::BOLD) } else { Style::default() };
        lines.push(Line::from(vec![
            Span::styled(format!(" {sym} "), Style::default().fg(col)),
            Span::styled(it.task.title.clone(), sel),
            Span::styled(format!("  {} / {} · {}", it.team, it.project, t(it.task.status.label())), Style::default().fg(DIM)),
        ]));
    }
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(t("j/k elegir · Enter ir a la tarea · Esc cerrar (marca como leído)"),Style::default().fg(DIM))));
    let area = centered(f.area(), 90, (lines.len() as u16 + 2).min(f.area().height));
    f.render_widget(Clear, area);
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }).block(block(&t("Bandeja"), true)), area);
}

fn draw_config(f: &mut Frame, app: &App) {
    let me = app.email_info.as_ref();
    let email = match me {
        Some(m) if m.email.is_some() && m.email_verified => format!("✓ {}", m.email.clone().unwrap_or_default()),
        Some(m) if m.email.is_some() => tf("{} (sin verificar)", &[&m.email.clone().unwrap_or_default()]),
        _ => t("sin cargar"),
    };
    let tg = match me {
        Some(m) if !m.telegram_available => t("el servidor no tiene el bot"),
        Some(m) if m.telegram_linked => t("✓ vinculado · Enter desvincula"),
        _ => t("sin vincular · Enter vincula"),
    };
    let team = app.teams.get(app.team).map(|t| t.name.as_str()).unwrap_or("—");
    let rows: [(String, String); crate::CONFIG_ITEMS] = [
        (t("Email para avisos"), email),
        (t("Verificar el email"), t("código de 6 dígitos")),
        ("Telegram".into(), tg),
        (t("Bandeja de novedades"), tf("{} sin leer", &[&app.unread.to_string()])),
        (t("Enviar un mensaje al equipo"), team.into()),
        (t("Agregar un miembro"), t("con búsqueda")),
        (t("Cambiar el rol de un miembro"), String::new()),
        (t("Generar código de invitación"), String::new()),
        (t("Unirme a un equipo con código"), String::new()),
        (t("Crear un equipo nuevo"), String::new()),
        (t("Cerrar sesión en todos los dispositivos"), t("cierra el TUI")),
        (t("Atajos y ayuda"), "?".into()),
        (t("Idioma / Language"), format!("{} · {}", crate::i18n::name(), t("Enter cambia"))),
    ];
    let mut lines = vec![
        Line::from(Span::styled(tf("Usuario: {}   ·   Equipo: {}", &[app.me.as_ref().map(|u| u.username.as_str()).unwrap_or(""), team]), Style::default().fg(DIM))),
        Line::raw(""),
    ];
    for (i, (name, state)) in rows.iter().enumerate() {
        let sel = i == app.cfg_sel;
        let st = if sel { Style::default().fg(BG).bg(BLUE).add_modifier(Modifier::BOLD) } else { Style::default().fg(FG) };
        lines.push(Line::from(vec![
            Span::styled(format!(" {name:<42}"), st),
            Span::styled(format!(" {state}"), if sel { st } else { Style::default().fg(DIM) }),
        ]));
    }
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(t("j/k elegir · Enter abrir · Esc cerrar"),Style::default().fg(DIM))));
    let area = centered(f.area(), 92, (lines.len() as u16 + 2).min(f.area().height));
    f.render_widget(Clear, area);
    f.render_widget(Paragraph::new(lines).block(block(&t("Configuración"), true)), area);
}
