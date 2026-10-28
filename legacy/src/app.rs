use crate::search::SearchResult;
use crate::search::SearchSource;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};
use arboard::Clipboard;
use crate::export::ExportFormat;
use crate::languages::Language;
use crate::code_handler::CodeHandler;
use crate::db::Database;
use std::sync::mpsc;
//use tokio::sync::mpsc;
use std::process::Stdio;
use crate::export;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Section {
    pub id: usize,
    pub title: String,
    pub details: Vec<Detail>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Detail {
    pub id: usize,
    pub title: String,
    pub description: String,
    pub code_path: Option<String>,
    pub language: Language,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum SearchTarget {
    Local,
    CratesIo,
    CheatsRs,
    All,
}

impl std::fmt::Display for SearchTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchTarget::Local => write!(f, "📝 Local"),
            SearchTarget::CratesIo => write!(f, "📦 Crates.io"),
            SearchTarget::CheatsRs => write!(f, "📚 Cheats.sh"),
            SearchTarget::All => write!(f, "🔍 Todas las fuentes"),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum Focus {
    Sections,
    Details,
    Search,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum Mode {
    Normal,
    Adding,
    Editing,
    Viewing,
    Searching,
    Help,
    Exporting,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum PopupFocus {
    Title,
    Description,
    Code,
}

pub struct App {
    pub sections: Vec<Section>,
    pub selected_section: Option<usize>,
    pub selected_detail: Option<usize>,
    pub focus: Focus,
    pub mode: Mode,
    pub search_query: String,
    pub search_results: Vec<SearchResult>,
    pub layout_sizes: LayoutSizes,
    pub input_buffer: String,
    pub description_buffer: String,
    pub search_target: SearchTarget,
    pub popup_focus: PopupFocus,
    pub searching: bool,
    search_tx: Option<mpsc::Sender<String>>,
    results_rx: Option<mpsc::Receiver<Vec<SearchResult>>>,
    pub search_scroll: usize,
    pub selected_link: Option<usize>,
    pub links: Vec<String>,
    pub clipboard: Option<Clipboard>,
    pub copying: bool,
    pub copy_result: Option<SearchResult>,
    pub selected_export_format: usize,
    pub export_formats: Vec<ExportFormat>,
    pub export_message: Option<String>,
    pub code_buffer: String,
    pub code_cursor: usize,
    pub code_scroll: usize,
    pub code_scroll_vertical: usize,
    pub selected_language: Language,
    pub code_handler: CodeHandler,
    pub db: Database,
    pub details_scroll: usize,
    pub selection_start: Option<usize>,
    pub selection_end: Option<usize>,
    pub title_cursor: usize,
    pub description_cursor: usize,
}

impl Clone for App {
    fn clone(&self) -> Self {
        Self {
            sections: self.sections.clone(),
            selected_section: self.selected_section,
            selected_detail: self.selected_detail,
            focus: self.focus.clone(),
            mode: self.mode.clone(),
            search_query: self.search_query.clone(),
            search_results: self.search_results.clone(),
            layout_sizes: self.layout_sizes.clone(),
            input_buffer: self.input_buffer.clone(),
            description_buffer: self.description_buffer.clone(),
            search_target: self.search_target.clone(),
            popup_focus: self.popup_focus.clone(),
            searching: self.searching,
            search_tx: self.search_tx.clone(),
            results_rx: None,
            search_scroll: self.search_scroll,
            selected_link: self.selected_link,
            links: self.links.clone(),
            clipboard: None,
            copying: self.copying,
            copy_result: self.copy_result.clone(),
            selected_export_format: self.selected_export_format,
            export_formats: self.export_formats.clone(),
            export_message: self.export_message.clone(),
            code_buffer: self.code_buffer.clone(),
            code_cursor: self.code_cursor,
            code_scroll: self.code_scroll,
            code_scroll_vertical: self.code_scroll_vertical,
            selected_language: self.selected_language.clone(),
            code_handler: self.code_handler.clone(),
            db: self.db.clone(),
            details_scroll: self.details_scroll,
            selection_start: self.selection_start,
            selection_end: self.selection_end,
            title_cursor: self.title_cursor,
            description_cursor: self.description_cursor,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LayoutSizes {
    pub left_panel_width: u16,
    pub right_panel_width: u16,
}

impl App {
    pub fn new() -> Self {
        let config = crate::config::Config::new().unwrap_or_else(|e| {
            eprintln!("Error creando configuración: {}", e);
            std::process::exit(1);
        });
        
        let db = Database::new(config.db_path.to_str().unwrap()).unwrap_or_else(|e| {
            eprintln!("Error conectando a la base de datos: {}", e);
            std::process::exit(1);
        });

        Self {
            sections: Vec::new(),
            selected_section: None,
            selected_detail: None,
            focus: Focus::Sections,
            mode: Mode::Normal,
            search_query: String::new(),
            search_results: Vec::new(),
            layout_sizes: LayoutSizes {
                left_panel_width: 30,
                right_panel_width: 70,
            },
            input_buffer: String::new(),
            description_buffer: String::new(),
            search_target: SearchTarget::Local,
            popup_focus: PopupFocus::Title,
            searching: false,
            search_tx: None,
            results_rx: None,
            search_scroll: 0,
            selected_link: None,
            links: Vec::new(),
            clipboard: Clipboard::new().ok(),
            copying: false,
            copy_result: None,
            selected_export_format: 0,
            export_formats: vec![
                ExportFormat::JSON,
                ExportFormat::HTML,
                ExportFormat::CSV,
            ],
            export_message: None,
            code_buffer: String::new(),
            code_cursor: 0,
            code_scroll: 0,
            code_scroll_vertical: 0,
            selected_language: Language::default(),
            code_handler: CodeHandler::new(),
            db,
            details_scroll: 0,
            selection_start: None,
            selection_end: None,
            title_cursor: 0,
            description_cursor: 0,
        }
    }

    pub fn from_sections(sections: Vec<Section>) -> Self {
        let mut app = Self::new();
        app.sections = sections;
        if !app.sections.is_empty() {
            app.selected_section = Some(0);
            if !app.sections[0].details.is_empty() {
                app.selected_detail = Some(0);
            }
        }
        app
    }

    pub fn handle_input(&mut self, key: KeyEvent) {
        match self.mode {
            Mode::Normal => {
                match key.code {
                    KeyCode::Char('s') => self.start_search(),
                    KeyCode::Char('x') => self.start_export(),
                    KeyCode::Up => self.move_selection_up(),
                    KeyCode::Down => self.move_selection_down(),
                    KeyCode::Char('a') => {
                        self.mode = Mode::Adding;
                        self.input_buffer.clear();
                        self.description_buffer.clear();
                        self.code_buffer.clear();
                        self.title_cursor = 0;
                        self.description_cursor = 0;
                        self.code_cursor = 0;
                        self.popup_focus = PopupFocus::Title;
                    },
                    KeyCode::Char('e') => {
                        if let Some(section_idx) = self.selected_section {
                            if self.focus == Focus::Sections {
                                self.mode = Mode::Editing;
                                if let Some(section) = self.sections.get(section_idx) {
                                    self.input_buffer = section.title.clone();
                                }
                            } else if let Some(detail_idx) = self.selected_detail {
                                if let Some(section) = self.sections.get(section_idx) {
                                    if let Some(detail) = section.details.get(detail_idx) {
                                        self.mode = Mode::Editing;
                                        self.input_buffer = detail.title.clone();
                                        self.description_buffer = detail.description.clone();
                                        if let Some(ref path) = detail.code_path {
                                            self.code_buffer = std::fs::read_to_string(path)
                                                .unwrap_or_default();
                                        }
                                        self.selected_language = detail.language.clone();
                                        self.popup_focus = PopupFocus::Title;
                                    }
                                }
                            }
                        }
                    },
                    KeyCode::Char('d') => {
                        if let Some(section_idx) = self.selected_section {
                            if self.focus == Focus::Sections {
                                // Eliminar sección
                                if let Some(section) = self.sections.get(section_idx) {
                                    // Primero eliminar de la base de datos usando el ID correcto
                                    self.db.delete_section(section.id).ok();
                                    
                                    // Luego eliminar del vector en memoria
                                    self.sections.remove(section_idx);
                                    
                                    // Actualizar la selección
                                    if section_idx >= self.sections.len() {
                                        self.selected_section = self.sections.len().checked_sub(1);
                                    }
                                    self.selected_detail = None;
                                }
                            } else if let Some(detail_idx) = self.selected_detail {
                                // Eliminar detalle
                                if let Some(section) = self.sections.get_mut(section_idx) {
                                    if let Some(detail) = section.details.get(detail_idx) {
                                        // Eliminar archivo de código si existe
                                        if let Some(ref path) = detail.code_path {
                                            self.code_handler.delete_code(path).ok();
                                        }
                                    }
                                    section.details.remove(detail_idx);
                                    if detail_idx >= section.details.len() {
                                        self.selected_detail = section.details.len().checked_sub(1);
                                    }
                                    // Guardar cambios en la base de datos
                                    self.db.save_section(section).ok();
                                }
                            }
                        }
                    },
                    KeyCode::Char('h') => {
                        self.mode = Mode::Help;
                    },
                    KeyCode::Tab => {
                        if key.modifiers.contains(KeyModifiers::SHIFT) {
                            self.previous_focus();
                        } else {
                            self.next_focus();
                        }
                    },
                    KeyCode::Enter => {
                        if self.focus == Focus::Details {
                            self.view_current_detail();
                        }
                    },
                    _ => {}
                }
            },
            Mode::Adding | Mode::Editing => {
                match key.code {
                    KeyCode::Esc => {
                        self.mode = Mode::Normal;
                        self.input_buffer.clear();
                        self.description_buffer.clear();
                        self.code_buffer.clear();
                        self.title_cursor = 0;
                        self.description_cursor = 0;
                        self.code_cursor = 0;
                    },
                    KeyCode::Tab => {
                        if self.focus == Focus::Details {
                            self.popup_focus = match self.popup_focus {
                                PopupFocus::Title => PopupFocus::Description,
                                PopupFocus::Description => PopupFocus::Code,
                                PopupFocus::Code => PopupFocus::Title,
                            };
                        }
                    },
                    KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if self.focus == Focus::Sections {
                            // Guardar sección
                            if !self.input_buffer.is_empty() {
                                let section = Section {
                                    id: if self.mode == Mode::Adding { 0 } else { 
                                        self.selected_section.unwrap_or(0) 
                                    },
                                    title: self.input_buffer.clone(),
                                    details: Vec::new(),
                                };
                                
                                // Guardar en la base de datos primero
                                self.db.save_section(&section).ok();
                                
                                // Luego actualizar el vector de secciones
                                if self.mode == Mode::Adding {
                                    self.sections.push(section.clone());
                                    self.selected_section = Some(self.sections.len() - 1);
                                } else if let Some(idx) = self.selected_section {
                                    self.sections[idx] = section;
                                }
                            }
                        } else {
                            // Guardar detalle
                            if let Some(section_idx) = self.selected_section {
                                let detail = Detail {
                                    id: if self.mode == Mode::Adding { 0 } else {
                                        self.selected_detail.unwrap_or(0)
                                    },
                                    title: self.input_buffer.clone(),
                                    description: self.description_buffer.clone(),
                                    code_path: if self.code_buffer.is_empty() {
                                        None
                                    } else {
                                        Some(self.code_handler.save_code(
                                            &self.code_buffer,
                                            &self.selected_language
                                        ).unwrap_or_default())
                                    },
                                    language: self.selected_language.clone(),
                                    created_at: chrono::Local::now().to_string(),
                                };

                                if let Some(section) = self.sections.get_mut(section_idx) {
                                    if self.mode == Mode::Adding {
                                        section.details.push(detail);
                                        self.selected_detail = Some(section.details.len() - 1);
                                    } else if let Some(detail_idx) = self.selected_detail {
                                        section.details[detail_idx] = detail;
                                    }
                                    self.db.save_section(section).ok();
                                }
                            }
                        }
                        self.mode = Mode::Normal;
                        self.title_cursor = 0;
                        self.description_cursor = 0;
                        self.code_cursor = 0;
                    },
                    KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        // Pegar solo en el buffer activo
                        if let Ok(mut clipboard) = Clipboard::new() {
                            if let Ok(text) = clipboard.get_text() {
                                match self.popup_focus {
                                    PopupFocus::Title => self.input_buffer.push_str(&text),
                                    PopupFocus::Description => self.description_buffer.push_str(&text),
                                    PopupFocus::Code => {
                                        // Para el código, insertar en la posición del cursor
                                        self.code_buffer.insert_str(self.code_cursor, &text);
                                        self.code_cursor += text.len();
                                    }
                                }
                            }
                        }
                    },
                    _ => self.handle_edit_mode(key),
                }
            },
            Mode::Searching => self.handle_search_mode(key),
            Mode::Viewing => self.handle_view_mode(key),
            Mode::Help => {
                if key.code == KeyCode::Esc {
                    self.mode = Mode::Normal;
                }
            },
            Mode::Exporting => self.handle_export_mode(key),
            _ => {}
        }
    }

    pub fn check_search_results(&mut self) {
        if let Some(rx) = &mut self.results_rx {
            if let Ok(results) = rx.try_recv() {
                self.search_results = results;
                self.searching = false;
            }
        }
    }

    fn view_current_detail(&mut self) {
        if let (Some(section_idx), Some(detail_idx)) = (self.selected_section, self.selected_detail) {
            if let Some(section) = self.sections.get(section_idx) {
                if let Some(detail) = section.details.get(detail_idx) {
                    self.mode = Mode::Viewing;
                    self.input_buffer = detail.title.clone();
                    self.description_buffer = detail.description.clone();
                    if let Some(ref path) = detail.code_path {
                        self.code_buffer = std::fs::read_to_string(path).unwrap_or_default();
                    }
                    self.selected_language = detail.language.clone();
                }
            }
        }
    }

    pub fn start_search(&mut self) {
        self.mode = Mode::Searching;
        self.search_query.clear();
        self.search_results.clear();
        self.search_scroll = 0;
        self.selected_link = None;
        self.links.clear();
    }

    pub fn start_export(&mut self) {
        self.mode = Mode::Exporting;
        self.selected_export_format = 0;
        self.export_message = None;
    }

    pub fn move_selection_up(&mut self) {
        match self.focus {
            Focus::Sections => {
                if let Some(current) = self.selected_section {
                    self.selected_section = if current > 0 {
                        Some(current - 1)
                    } else {
                        Some(self.sections.len() - 1)
                    };
                } else if !self.sections.is_empty() {
                    self.selected_section = Some(0);
                }
            }
            Focus::Details => {
                if let Some(section_idx) = self.selected_section {
                    if let Some(section) = self.sections.get(section_idx) {
                        if let Some(current) = self.selected_detail {
                            self.selected_detail = if current > 0 {
                                Some(current - 1)
                            } else {
                                Some(section.details.len() - 1)
                            };
                        } else if !section.details.is_empty() {
                            self.selected_detail = Some(0);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub fn move_selection_down(&mut self) {
        match self.focus {
            Focus::Sections => {
                if let Some(current) = self.selected_section {
                    self.selected_section = if current + 1 < self.sections.len() {
                        Some(current + 1)
                    } else {
                        Some(0)
                    };
                } else if !self.sections.is_empty() {
                    self.selected_section = Some(0);
                }
            }
            Focus::Details => {
                if let Some(section_idx) = self.selected_section {
                    if let Some(section) = self.sections.get(section_idx) {
                        if let Some(current) = self.selected_detail {
                            self.selected_detail = if current + 1 < section.details.len() {
                                Some(current + 1)
                            } else {
                                Some(0)
                            };
                        } else if !section.details.is_empty() {
                            self.selected_detail = Some(0);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_search_mode(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Tab => {
                self.search_target = match self.search_target {
                    SearchTarget::Local => SearchTarget::CratesIo,
                    SearchTarget::CratesIo => SearchTarget::CheatsRs,
                    SearchTarget::CheatsRs => SearchTarget::All,
                    SearchTarget::All => SearchTarget::Local,
                };
                self.perform_search();
            }
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.search_query.clear();
                self.search_results.clear();
                self.search_scroll = 0;
                self.selected_link = None;
                self.links.clear();
            }
            KeyCode::Enter => {
                if self.search_target == SearchTarget::Local {
                    if let Some(result) = self.search_results.get(self.search_scroll) {
                        for (section_idx, section) in self.sections.iter().enumerate() {
                            for (detail_idx, detail) in section.details.iter().enumerate() {
                                if detail.title == result.title {
                                    self.selected_section = Some(section_idx);
                                    self.selected_detail = Some(detail_idx);
                                    self.mode = Mode::Viewing;
                                    self.input_buffer = detail.title.clone();
                                    self.description_buffer = detail.description.clone();
                                    if let Some(ref path) = detail.code_path {
                                        self.code_buffer = std::fs::read_to_string(path)
                                            .unwrap_or_default();
                                    }
                                    self.selected_language = detail.language.clone();
                                    self.code_cursor = 0;
                                    self.code_scroll = 0;
                                    self.code_scroll_vertical = 0;
                                    self.selection_start = None;
                                    self.selection_end = None;
                                    return;
                                }
                            }
                        }
                    }
                } else if let Some(selected) = self.selected_link {
                    if let Some(url) = self.links.get(selected) {
                        let _ = open::that(url);
                    }
                }
            }
            KeyCode::Up => {
                if self.search_scroll > 0 {
                    self.search_scroll -= 1;
                }
            }
            KeyCode::Down => {
                if self.search_scroll + 1 < self.search_results.len() {
                    self.search_scroll += 1;
                }
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
                self.perform_search();
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                self.perform_search();
            }
            _ => {}
        }
    }

    fn perform_search(&mut self) {
        self.search_results.clear();
        if self.search_query.is_empty() {
            return;
        }

        if self.search_target == SearchTarget::Local {
            if let Ok(results) = self.db.search_local(&self.search_query) {
                for (_section, detail) in results {
                    self.search_results.push(SearchResult {
                        title: detail.title,
                        description: detail.description,
                        source: SearchSource::Local,
                    });
                }
            }
        }
    }

    fn handle_view_mode(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.input_buffer.clear();
                self.description_buffer.clear();
                self.code_buffer.clear();
                self.selection_start = None;
                self.selection_end = None;
                self.code_cursor = 0;
                self.code_scroll = 0;
                self.code_scroll_vertical = 0;
            },
            KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down => {
                if !key.modifiers.contains(KeyModifiers::CONTROL) {
                    match key.code {
                        KeyCode::Left => if self.code_cursor > 0 { self.code_cursor -= 1 },
                        KeyCode::Right => if self.code_cursor < self.code_buffer.len() { self.code_cursor += 1 },
                        KeyCode::Up => {
                            let current_line = self.code_cursor_line();
                            if current_line > 0 {
                                let current_column = self.code_cursor_column();
                                self.move_cursor(current_line - 1, current_column);
                            }
                        },
                        KeyCode::Down => {
                            let current_line = self.code_cursor_line();
                            let total_lines = self.code_buffer.lines().count();
                            if current_line < total_lines - 1 {
                                let current_column = self.code_cursor_column();
                                self.move_cursor(current_line + 1, current_column);
                            }
                        },
                        _ => {}
                    }
                } else {
                    if self.selection_start.is_none() {
                        self.selection_start = Some(self.code_cursor);
                    }
                    match key.code {
                        KeyCode::Left => {
                            if self.code_cursor > 0 {
                                self.code_cursor -= 1;
                            }
                        },
                        KeyCode::Right => {
                            if self.code_cursor < self.code_buffer.len() {
                                self.code_cursor += 1;
                            }
                        },
                        KeyCode::Up => {
                            let current_line = self.code_cursor_line();
                            if current_line > 0 {
                                let current_column = self.code_cursor_column();
                                self.move_cursor(current_line - 1, current_column);
                            }
                        },
                        KeyCode::Down => {
                            let current_line = self.code_cursor_line();
                            let total_lines = self.code_buffer.lines().count();
                            if current_line < total_lines - 1 {
                                let current_column = self.code_cursor_column();
                                self.move_cursor(current_line + 1, current_column);
                            }
                        },
                        _ => {}
                    }
                    self.selection_end = Some(self.code_cursor);
                }
            },
            KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let (Some(start), Some(end)) = (self.selection_start, self.selection_end) {
                    let (start_idx, end_idx) = if start <= end {
                        (start, end)
                    } else {
                        (end, start)
                    };
                    
                    let text = self.code_buffer[start_idx..=end_idx].to_string();
                    
                    // Intentar copiar usando xclip
                    use std::process::Command;
                    use std::io::Write;
                    
                    let success = Command::new("xclip")
                        .arg("-selection")
                        .arg("clipboard")
                        .stdin(Stdio::piped())
                        .spawn()
                        .map(|mut child| {
                            child.stdin.as_mut()
                                .unwrap()
                                .write_all(text.as_bytes())
                                .and_then(|_| child.wait())
                                .is_ok()
                        })
                        .unwrap_or(false);

                    if success {
                        self.selection_start = None;
                        self.selection_end = None;
                    } else {
                        // Intentar con arboard como fallback
                        if let Ok(mut clipboard) = Clipboard::new() {
                            if clipboard.set_text(&text).is_ok() {
                                self.selection_start = None;
                                self.selection_end = None;
                            }
                        }
                    }
                }
            },
            _ => {}
        }
    }

    pub fn next_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Sections => Focus::Details,
            Focus::Details => Focus::Search,
            Focus::Search => Focus::Sections,
        };
    }

    pub fn previous_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Sections => Focus::Search,
            Focus::Details => Focus::Sections,
            Focus::Search => Focus::Details,
        };
    }

    pub fn code_cursor_line(&self) -> usize {
        self.code_buffer[..self.code_cursor]
            .chars()
            .filter(|&c| c == '\n')
            .count()
    }

    pub fn code_cursor_column(&self) -> usize {
        let last_newline = self.code_buffer[..self.code_cursor]
            .rfind('\n')
            .map(|i| i + 1)
            .unwrap_or(0);
        self.code_cursor - last_newline
    }

    fn get_line_number(&self, pos: usize) -> usize {
        self.code_buffer[..pos]
            .chars()
            .filter(|&c| c == '\n')
            .count()
    }

    pub fn get_position_in_buffer(&self, line: usize, column: usize) -> usize {
        let mut pos = 0;
        let mut current_line = 0;

        for (i, c) in self.code_buffer.chars().enumerate() {
            if current_line == line {
                if pos == column {
                    return i;
                }
                pos += 1;
            }
            if c == '\n' {
                if current_line == line {
                    return i;
                }
                current_line += 1;
                pos = 0;
            }
        }

        // Si estamos en la última línea y no hay \n
        if current_line == line && pos == column {
            return self.code_buffer.len();
        }

        // Fallback seguro
        self.code_buffer.len().saturating_sub(1)
    }

    // Método auxiliar para manejar el cursor
    pub fn move_cursor(&mut self, line: usize, column: usize) {
        self.code_cursor = self.get_position_in_buffer(line, column);
    }

    // Método para obtener la posición actual del cursor como (línea, columna)
    pub fn get_cursor_position(&self) -> (usize, usize) {
        let line = self.code_cursor_line();
        let column = self.code_cursor_column();
        (line, column)
    }

    fn copy_to_clipboard(&mut self, text: &str) -> bool {
        // Primero intentar con xclip
        use std::process::Command;
        let xclip_success = Command::new("xclip")
            .args(["-selection", "clipboard"])
            .arg(text)
            .status()
            .map(|status| status.success())
            .unwrap_or(false);

        if xclip_success {
            return true;
        }

        // Si xclip falla, intentar con arboard
        if let Ok(mut clipboard) = Clipboard::new() {
            return clipboard.set_text(text).is_ok();
        }

        false
    }

    fn handle_edit_mode(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                match self.popup_focus {
                    PopupFocus::Description => {
                        self.description_buffer.insert(self.description_cursor, '\n');
                        self.description_cursor += 1;
                    },
                    PopupFocus::Code => {
                        self.code_buffer.insert(self.code_cursor, '\n');
                        self.code_cursor += 1;
                    },
                    _ => {}
                }
            },
            KeyCode::Backspace => {
                match self.popup_focus {
                    PopupFocus::Title => {
                        if self.title_cursor > 0 {
                            self.input_buffer.remove(self.title_cursor - 1);
                            self.title_cursor -= 1;
                        }
                    },
                    PopupFocus::Description => {
                        if self.description_cursor > 0 {
                            self.description_buffer.remove(self.description_cursor - 1);
                            self.description_cursor -= 1;
                        }
                    },
                    PopupFocus::Code => {
                        if self.code_cursor > 0 {
                            self.code_buffer.remove(self.code_cursor - 1);
                            self.code_cursor -= 1;
                        }
                    }
                }
            },
            KeyCode::Delete => {
                match self.popup_focus {
                    PopupFocus::Title => {
                        if self.title_cursor < self.input_buffer.len() {
                            self.input_buffer.remove(self.title_cursor);
                        }
                    },
                    PopupFocus::Description => {
                        if self.description_cursor < self.description_buffer.len() {
                            self.description_buffer.remove(self.description_cursor);
                        }
                    },
                    PopupFocus::Code => {
                        if self.code_cursor < self.code_buffer.len() {
                            self.code_buffer.remove(self.code_cursor);
                        }
                    }
                }
            },
            KeyCode::Left => {
                match self.popup_focus {
                    PopupFocus::Title => {
                        if self.title_cursor > 0 {
                            self.title_cursor -= 1;
                        }
                    },
                    PopupFocus::Description => {
                        if self.description_cursor > 0 {
                            self.description_cursor -= 1;
                        }
                    },
                    PopupFocus::Code => {
                        if self.code_cursor > 0 {
                            self.code_cursor -= 1;
                        }
                    }
                }
            },
            KeyCode::Right => {
                match self.popup_focus {
                    PopupFocus::Title => {
                        if self.title_cursor < self.input_buffer.len() {
                            self.title_cursor += 1;
                        }
                    },
                    PopupFocus::Description => {
                        if self.description_cursor < self.description_buffer.len() {
                            self.description_cursor += 1;
                        }
                    },
                    PopupFocus::Code => {
                        if self.code_cursor < self.code_buffer.len() {
                            self.code_cursor += 1;
                        }
                    }
                }
            },
            KeyCode::Char(c) => {
                match self.popup_focus {
                    PopupFocus::Title => {
                        self.input_buffer.insert(self.title_cursor, c);
                        self.title_cursor += 1;
                    },
                    PopupFocus::Description => {
                        self.description_buffer.insert(self.description_cursor, c);
                        self.description_cursor += 1;
                    },
                    PopupFocus::Code => {
                        self.code_buffer.insert(self.code_cursor, c);
                        self.code_cursor += 1;
                    }
                }
            },
            KeyCode::Up => {
                match self.popup_focus {
                    PopupFocus::Code => {
                        let current_line = self.code_cursor_line();
                        if current_line > 0 {
                            let current_column = self.code_cursor_column();
                            self.move_cursor(current_line - 1, current_column);
                            
                            // Actualizar scroll vertical si es necesario
                            if current_line <= self.code_scroll_vertical {
                                self.code_scroll_vertical = current_line.saturating_sub(1);
                            }
                        }
                    },
                    PopupFocus::Description => {
                        let current_line = self.description_buffer[..self.description_cursor]
                            .chars()
                            .filter(|&c| c == '\n')
                            .count();
                        if current_line > 0 {
                            let current_column = self.description_cursor - 
                                self.description_buffer[..self.description_cursor]
                                    .rfind('\n')
                                    .map(|i| i + 1)
                                    .unwrap_or(0);
                            
                            let prev_line_start = self.description_buffer[..self.description_cursor]
                                .rfind('\n')
                                .map(|i| {
                                    self.description_buffer[..i]
                                        .rfind('\n')
                                        .map(|j| j + 1)
                                        .unwrap_or(0)
                                })
                                .unwrap_or(0);
                            
                            self.description_cursor = prev_line_start + 
                                current_column.min(
                                    self.description_buffer[prev_line_start..]
                                        .find('\n')
                                        .unwrap_or_else(|| self.description_buffer[prev_line_start..].len())
                                );
                        }
                    },
                    _ => {}
                }
            },
            KeyCode::Down => {
                match self.popup_focus {
                    PopupFocus::Code => {
                        let current_line = self.code_cursor_line();
                        let total_lines = self.code_buffer.lines().count();
                        if current_line < total_lines - 1 {
                            let current_column = self.code_cursor_column();
                            self.move_cursor(current_line + 1, current_column);
                            
                            // Actualizar scroll vertical si es necesario
                            if current_line >= self.code_scroll_vertical + 10 {
                                self.code_scroll_vertical = current_line - 9;
                            }
                        }
                    },
                    PopupFocus::Description => {
                        let next_newline = self.description_buffer[self.description_cursor..]
                            .find('\n')
                            .map(|i| self.description_cursor + i);
                        
                        if let Some(newline_pos) = next_newline {
                            let current_column = self.description_cursor - 
                                self.description_buffer[..self.description_cursor]
                                    .rfind('\n')
                                    .map(|i| i + 1)
                                    .unwrap_or(0);
                            
                            let next_line_start = newline_pos + 1;
                            let next_line_end = self.description_buffer[next_line_start..]
                                .find('\n')
                                .map(|i| next_line_start + i)
                                .unwrap_or_else(|| self.description_buffer.len());
                            
                            self.description_cursor = next_line_start + 
                                current_column.min(next_line_end - next_line_start);
                        }
                    },
                    _ => {}
                }
            },
            _ => {}
        }
    }

    fn handle_export_mode(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.export_message = None;
            },
            KeyCode::Up => {
                if self.selected_export_format > 0 {
                    self.selected_export_format -= 1;
                }
            },
            KeyCode::Down => {
                if self.selected_export_format < self.export_formats.len() - 1 {
                    self.selected_export_format += 1;
                }
            },
            KeyCode::Enter => {
                if self.export_message.is_none() {
                    // Mostrar mensaje de proceso
                    self.export_message = Some("⏳ Exportando datos...".to_string());
                    
                    // Realizar la exportación
                    let format = self.export_formats[self.selected_export_format].clone();
                    match export::export_data(self, format) {
                        Ok((source, dest)) => {
                            self.export_message = Some(format!(
                                "✅ Datos exportados correctamente\nOrigen: {}\nDestino: {}",
                                source, dest
                            ));
                        },
                        Err(e) => {
                            self.export_message = Some(format!("❌ Error al exportar: {}", e));
                        }
                    }
                } else {
                    // Si hay un mensaje, Enter cierra el popup
                    self.mode = Mode::Normal;
                    self.export_message = None;
                }
            },
            _ => {}
        }
    }
}

