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
        Mode::Help => draw_help(f),
        _ => {}
    }
}

fn draw_login(f: &mut Frame, app: &App) {
    let area = centered(f.area(), 52, 13);
    let labels = ["Servidor", "Usuario", "Contraseña"];
    let mut lines = vec![Line::from(Span::styled(
        if app.register { "Crear cuenta" } else { "Iniciar sesión" },
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
    lines.push(Line::from(Span::styled("Enter entrar · Tab campo · F2 login/registro · Esc salir", Style::default().fg(DIM))));
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
        spans.push(Span::styled("sin proyectos — P para crear uno", Style::default().fg(DIM)));
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
    let title = format!("{who} · equipo: {}", members.join(", "));
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
        let title = format!("{} ({})", st.label(), tasks.len());
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
        "n tarea · Enter detalle · H/L mover · +/- prioridad · a asignarme · e editar · d borrar · Tab proyecto · [ ] equipo · ? ayuda · q salir".to_string()
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
            Span::styled(format!("{sym} prioridad {}", t.priority), Style::default().fg(col)),
            Span::raw("  ·  "),
            Span::raw(t.status.label()),
            Span::raw("  ·  "),
            Span::styled(t.assignee.as_deref().map(|a| format!("@{a}")).unwrap_or_else(|| "sin asignar".into()), Style::default().fg(PURPLE)),
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
        lines.push(Line::from(Span::styled(format!("Subtareas ({}/{})", t.subtasks_done, t.subtasks_total), Style::default().fg(YELLOW))));
        for (i, st) in subs.iter().enumerate() {
            let done = st.status == Status::Done;
            let mut style = Style::default().fg(if done { DIM } else { FG });
            if i == app.sub { style = style.bg(Color::Rgb(41, 46, 66)); }
            lines.push(Line::from(Span::styled(format!(" {} {}", if done { "☑" } else { "☐" }, st.title), style)));
        }
        lines.push(Line::raw(""));
    }
    lines.push(Line::from(Span::styled(format!("Comentarios ({})", app.comments.len()), Style::default().fg(YELLOW))));
    for c in &app.comments {
        lines.push(Line::from(vec![
            Span::styled(format!("{} ", c.created_at.format("%d/%m %H:%M")), Style::default().fg(DIM)),
            Span::styled(format!("{}: ", c.username), Style::default().fg(GREEN)),
            Span::raw(c.body.clone()),
        ]));
    }
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled("s subtarea · j/k elegir · Espacio marcar · d borrar subtarea · l etiqueta · c comentar · Esc volver", Style::default().fg(DIM))));
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }).block(block("Detalle", true)), area);
}

fn draw_prompt(f: &mut Frame, app: &App, kind: PromptKind) {
    if app.return_mode_detail {
        draw_detail(f, app);
    }
    let title = match kind {
        PromptKind::NewTask => "Nueva tarea",
        PromptKind::NewProject => "Nuevo proyecto",
        PromptKind::NewTeam => "Nuevo equipo",
        PromptKind::AddMember => "Agregar miembro: usuario [rol]",
        PromptKind::Comment => "Comentario",
        PromptKind::EditTitle => "Editar título",
        PromptKind::Subtask => "Nueva subtarea",
        PromptKind::JoinCode => "Código de invitación",
        PromptKind::SetRole => "Cambiar rol: usuario rol (viewer | member | admin)",
        PromptKind::Label => "Etiqueta (existente: alterna · nueva: se crea)",
    };
    let area = centered(f.area(), 60, 3);
    f.render_widget(Clear, area);
    f.render_widget(Paragraph::new(format!("{}▏", app.input)).block(block(title, true)), area);
}

fn draw_help(f: &mut Frame) {
    let rows = [
        ("h/l ←/→", "cambiar columna"), ("j/k ↑/↓", "mover selección"), ("Enter", "detalle y comentarios"),
        ("n", "nueva tarea"), ("e", "editar título"), ("d", "borrar tarea"),
        ("H / L", "mover tarea de estado"), ("+ / -", "prioridad"), ("a", "asignarme / quitar"),
        ("Tab / S-Tab", "proyecto siguiente / anterior"), ("[ / ]", "equipo anterior / siguiente"),
        ("P", "nuevo proyecto"), ("T", "nuevo equipo"), ("m", "agregar miembro: usuario [rol] (admin+)"), ("R", "cambiar rol de un miembro (admin+)"),
        ("I", "generar código de invitación (admin+)"), ("J", "unirse con un código"),
        ("l (detalle)", "poner/quitar etiqueta (la crea si no existe)"), ("s (detalle)", "nueva subtarea"),
        ("Espacio", "marcar subtarea (detalle)"), ("X", "cerrar sesión en todos los dispositivos"),
        ("q", "salir"),
    ];
    let lines: Vec<Line> = rows
        .iter()
        .map(|(k, d)| Line::from(vec![Span::styled(format!("{k:>12}  "), Style::default().fg(BLUE)), Span::raw(*d)]))
        .collect();
    let area = centered(f.area(), 56, rows.len() as u16 + 2);
    f.render_widget(Clear, area);
    f.render_widget(Paragraph::new(lines).block(block("Atajos", true)), area);
}
