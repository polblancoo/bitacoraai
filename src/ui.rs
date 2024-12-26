use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Line},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap, Clear},
    Frame,
};
use crate::app::{App, Focus, Mode, SearchTarget, PopupFocus};
use crate::helper::KeyBindings;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(3),
        ].as_ref())
        .split(frame.size());

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(app.layout_sizes.left_panel_width),
            Constraint::Percentage(app.layout_sizes.right_panel_width),
        ].as_ref())
        .split(chunks[0]);

    draw_sections(frame, app, main_chunks[0]);
    draw_details(frame, app, main_chunks[1]);
    draw_shortcuts(frame, app, chunks[1]);

    match app.mode {
        Mode::Adding | Mode::Editing => draw_edit_popup(frame, app),
        Mode::Help => draw_help_popup(frame, app),
        Mode::Searching => draw_search_popup(frame, app),
        _ => {}
    }
}

fn draw_sections(frame: &mut Frame, app: &App, area: Rect) {
    let mut state = ListState::default();
    state.select(app.selected_section);

    let block = Block::default()
        .title(Span::styled(
            "📚 Secciones",
            Style::default().fg(if app.focus == Focus::Sections {
                Color::Yellow
            } else {
                Color::White
            })
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if app.focus == Focus::Sections {
            Color::Yellow
        } else {
            Color::White
        }));

    let items: Vec<ListItem> = app.sections.iter()
        .map(|section| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    &section.title,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(if app.focus == Focus::Sections {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        }),
                )
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD)
        );

    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_details(frame: &mut Frame, app: &App, area: Rect) {
    let mut state = ListState::default();
    state.select(app.selected_detail);

    let block = Block::default()
        .title("Detalles")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if app.focus == Focus::Details {
            Color::Yellow
        } else {
            Color::White
        }));

    let items: Vec<ListItem> = if let Some(section_idx) = app.selected_section {
        if let Some(section) = app.sections.get(section_idx) {
            section.details.iter()
                .map(|detail| {
                    ListItem::new(vec![
                        // Primera línea: Título
                        Line::from(vec![
                            Span::styled(
                                &detail.title,
                                Style::default()
                                    .fg(Color::Green)
                                    .add_modifier(Modifier::BOLD)
                            ),
                        ]),
                        // Segunda línea: Descripción
                        Line::from(vec![
                            Span::raw("  "),  // Indentación
                            Span::styled(
                                &detail.description,
                                Style::default().fg(Color::White)
                            ),
                        ]),
                        // Tercera línea: Fecha de creación
                        Line::from(vec![
                            Span::raw("  "),  // Indentación
                            Span::styled(
                                &detail.created_at,
                                Style::default()
                                    .fg(Color::DarkGray)
                                    .add_modifier(Modifier::ITALIC)
                            ),
                        ]),
                        // Línea en blanco para separar entradas
                        Line::from(""),
                    ])
                })
                .collect()
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD)
        );

    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_shortcuts(frame: &mut Frame, app: &App, area: Rect) {
    let bindings = KeyBindings::new(app.focus.clone());
    let shortcuts = Line::from(
        bindings
            .commands
            .iter()
            .flat_map(|(key, desc)| {
                vec![
                    Span::styled(
                        key,
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::raw(": "),
                    Span::raw(desc),
                    Span::raw(" | "),
                ]
            })
            .collect::<Vec<Span>>()
    );

    let paragraph = Paragraph::new(shortcuts)
        .block(Block::default().borders(Borders::ALL))
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

fn draw_edit_popup(frame: &mut Frame, app: &App) {
    let area = centered_rect(70, 50, frame.size());
    
    let title = match app.focus {
        Focus::Sections => "Nueva Sección".to_string(),
        Focus::Details => {
            if let Some(section_idx) = app.selected_section {
                if let Some(section) = app.sections.get(section_idx) {
                    format!("Nuevo Detalle en \"{}\"", section.title)
                } else {
                    "Nuevo Detalle".to_string()
                }
            } else {
                "Nuevo Detalle".to_string()
            }
        }
        _ => "Nuevo".to_string(),
    };

    let title_style = if app.popup_focus == PopupFocus::Title {
        Style::default().bg(Color::Yellow).fg(Color::Black)
    } else {
        Style::default().fg(Color::White)
    };

    let desc_style = if app.popup_focus == PopupFocus::Description {
        Style::default().bg(Color::Yellow).fg(Color::Black)
    } else {
        Style::default().fg(Color::White)
    };

    let content = match app.focus {
        Focus::Sections => {
            let input_text = format!("{}_", app.input_buffer);
            vec![
                Line::from(""),
                Line::from(vec![
                    Span::raw("Título: "),
                    Span::styled(
                        input_text,
                        Style::default()
                            .bg(Color::Yellow)
                            .fg(Color::Black)
                    ),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::raw("(El ícono 📁 se agregará automáticamente si no lo incluyes)")
                ]),
            ]
        }
        Focus::Details => {
            vec![
                Line::from(""),
                Line::from(vec![
                    Span::raw("Título: "),
                    Span::styled(
                        format!("{}_", app.input_buffer),
                        title_style
                    ),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::raw("Descripción: "),
                    Span::styled(
                        format!("{}_", app.description_buffer),
                        desc_style
                    ),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Tab", Style::default().fg(Color::Green)),
                    Span::raw(" para cambiar entre campos"),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Enter", Style::default().fg(Color::Green)),
                    Span::raw(" para guardar, "),
                    Span::styled("Esc", Style::default().fg(Color::Green)),
                    Span::raw(" para cancelar"),
                ]),
            ]
        }
        _ => vec![],
    };

    let edit_message = Paragraph::new(content)
        .block(Block::default()
            .title(Span::styled(
                title,
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)))
        .alignment(ratatui::layout::Alignment::Left)
        .wrap(Wrap { trim: true });

    let clear = Clear;
    frame.render_widget(clear, area);
    frame.render_widget(edit_message, area);
}

fn draw_help_popup(frame: &mut Frame, _app: &App) {
    let area = centered_rect(60, 60, frame.size());
    
    let help_text = vec![
        Line::from(vec![
            Span::styled("Comandos Disponibles", Style::default().fg(Color::Yellow))
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("h", Style::default().fg(Color::Green)),
            Span::raw(" - Muestra esta ayuda"),
        ]),
        Line::from(vec![
            Span::styled("a", Style::default().fg(Color::Green)),
            Span::raw(" - Agrega una nueva sección/detalle"),
        ]),
        Line::from(vec![
            Span::styled("e", Style::default().fg(Color::Green)),
            Span::raw(" - Edita la sección/detalle seleccionado"),
        ]),
        Line::from(vec![
            Span::styled("d", Style::default().fg(Color::Green)),
            Span::raw(" - Elimina la sección/detalle seleccionado"),
        ]),
        Line::from(vec![
            Span::styled("Tab", Style::default().fg(Color::Green)),
            Span::raw(" - Cambia el foco entre paneles"),
        ]),
        Line::from(vec![
            Span::styled("Ctrl + ←/→", Style::default().fg(Color::Green)),
            Span::raw(" - Ajusta el tamaño de los paneles"),
        ]),
        Line::from(vec![
            Span::styled("s", Style::default().fg(Color::Green)),
            Span::raw(" - Buscar en secciones y detalles"),
        ]),
        Line::from(vec![
            Span::styled("q", Style::default().fg(Color::Green)),
            Span::raw(" - Salir"),
        ]),
    ];

    let help_message = Paragraph::new(help_text)
        .block(Block::default()
            .title("Ayuda")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)))
        .wrap(Wrap { trim: true });

    let clear = Clear;
    frame.render_widget(clear, area);
    frame.render_widget(help_message, area);
}

// Función helper para centrar el popup
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn draw_search_popup(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 70, frame.size());
    
    let search_text = format!("{}_", app.search_query);
    let mut content = vec![
        // Mostrar el selector de fuente
        Line::from(vec![
            Span::raw("Fuente: "),
            Span::styled(
                match app.search_target {
                    SearchTarget::Local => "Local",
                    SearchTarget::CratesIo => "Crates.io",
                    SearchTarget::CheatsRs => "Cheats.rs",
                    SearchTarget::All => "Todas",
                },
                Style::default().fg(Color::Green),
            ),
            Span::raw("  "),
            Span::styled("[Tab]", Style::default().fg(Color::Yellow)),
            Span::raw(" para cambiar"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("Búsqueda: "),
            Span::styled(
                search_text,
                Style::default()
                    .bg(Color::Yellow)
                    .fg(Color::Black)
            ),
        ]),
        Line::from(""),
    ];

    // Agregar resultados de búsqueda
    if !app.search_results.is_empty() {
        content.push(Line::from(vec![
            Span::styled("Resultados:", Style::default().fg(Color::Green))
        ]));
        content.push(Line::from(""));

        for result in &app.search_results {
            content.push(Line::from(vec![
                Span::styled(&result.title, Style::default().fg(Color::Cyan)),
            ]));
            content.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(&result.description, Style::default().fg(Color::White)),
            ]));
            // Agregar la fuente del resultado
            content.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!("[{}]", result.source),
                    Style::default().fg(Color::DarkGray)
                ),
            ]));
            content.push(Line::from("")); // Línea en blanco para separar resultados
        }
    } else if !app.search_query.is_empty() {
        content.push(Line::from(vec![
            Span::styled("No se encontraron resultados", Style::default().fg(Color::Red))
        ]));
    }

    let search_message = Paragraph::new(content)
        .block(Block::default()
            .title("Búsqueda")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)))
        .wrap(Wrap { trim: true });

    let clear = Clear;
    frame.render_widget(clear, area);
    frame.render_widget(search_message, area);
}
