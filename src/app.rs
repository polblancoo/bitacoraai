use crate::search::SearchResult;
use crate::search::SearchSource;
use chrono::Local;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};
use crate::search::cheats::CheatsRsSearch;
use crate::search::crates_io::CratesIoSearch;
use crate::search::SearchProvider;
use tokio::sync::mpsc;
use arboard::Clipboard;
use open;
use crate::export::{export_data, ExportFormat};
use crate::languages::Language;
use crate::code_handler::{CodeHandler, MAX_CODE_SIZE};
use crate::db::Database;
use std::fs;

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
    pub selected_language: Language,
    pub code_handler: CodeHandler,
    pub db: Database,
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
            selected_language: self.selected_language.clone(),
            code_handler: self.code_handler.clone(),
            db: self.db.clone(),
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
            selected_language: Language::default(),
            code_handler: CodeHandler::new(),
            db,
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

    pub fn handle_input(&mut self, key: KeyEvent) {
        match self.mode {
            Mode::Adding | Mode::Editing => {
                match key.code {
                    KeyCode::Char(c) => {
                        if key.modifiers.contains(KeyModifiers::CONTROL) {
                            match c {
                                's' => {
                                    if self.mode == Mode::Editing {
                                        self.handle_edit_submit();
                                    } else {
                                        self.handle_add_submit();
                                    }
                                    self.mode = Mode::Normal;
                                }
                                'l' => {
                                    if self.popup_focus == PopupFocus::Code {
                                        self.cycle_language();
                                    }
                                }
                                'v' => {
                                    if let Some(clipboard) = &mut self.clipboard {
                                        if let Ok(text) = clipboard.get_text() {
                                            match self.popup_focus {
                                                PopupFocus::Code => {
                                                    let new_size = self.code_buffer.len() + text.len();
                                                    if new_size > MAX_CODE_SIZE {
                                                        let remaining = MAX_CODE_SIZE - self.code_buffer.len();
                                                        let truncated = text.chars().take(remaining).collect::<String>();
                                                        self.code_buffer.insert_str(self.code_cursor, &truncated);
                                                        self.code_cursor += truncated.len();
                                                        self.show_warning("⚠️ Código truncado: excede el límite de tamaño");
                                                    } else {
                                                        self.code_buffer.insert_str(self.code_cursor, &text);
                                                        self.code_cursor += text.len();
                                                    }
                                                }
                                                PopupFocus::Title => {
                                                    self.input_buffer.push_str(&text);
                                                }
                                                PopupFocus::Description => {
                                                    self.description_buffer.push_str(&text);
                                                }
                                            }
                                        }
                                    }
                                }
                                'c' => {
                                    if let Some(clipboard) = &mut self.clipboard {
                                        match self.popup_focus {
                                            PopupFocus::Code => {
                                                let _ = clipboard.set_text(&self.code_buffer);
                                            }
                                            PopupFocus::Title => {
                                                let _ = clipboard.set_text(&self.input_buffer);
                                            }
                                            PopupFocus::Description => {
                                                let _ = clipboard.set_text(&self.description_buffer);
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                        } else {
                            // Manejo normal de caracteres
                            match self.focus {
                                Focus::Details => {
                                    match self.popup_focus {
                                        PopupFocus::Code => {
                                            self.code_buffer.push(c);
                                        }
                                        PopupFocus::Title => self.input_buffer.push(c),
                                        PopupFocus::Description => self.description_buffer.push(c),
                                    }
                                }
                                Focus::Sections => {
                                    self.input_buffer.push(c);
                                }
                                _ => {}
                            }
                        }
                    }
                    KeyCode::Tab => {
                        if self.focus == Focus::Details {
                            if key.modifiers.contains(KeyModifiers::SHIFT) {
                                // Si es Shift+Tab, retroceder
                                self.popup_focus = match self.popup_focus {
                                    PopupFocus::Title => PopupFocus::Code,
                                    PopupFocus::Description => PopupFocus::Title,
                                    PopupFocus::Code => PopupFocus::Description,
                                };
                            } else if self.popup_focus == PopupFocus::Code {
                                // Si estamos en el código, insertar tab
                                self.code_buffer.push_str("    ");
                            } else {
                                // Avanzar al siguiente campo
                                self.popup_focus = match self.popup_focus {
                                    PopupFocus::Title => PopupFocus::Description,
                                    PopupFocus::Description => PopupFocus::Code,
                                    PopupFocus::Code => PopupFocus::Title,
                                };
                            }
                        }
                    }
                    KeyCode::Enter => {
                        if self.popup_focus == PopupFocus::Code {
                            self.code_buffer.push('\n');
                        }
                    }
                    KeyCode::Backspace => {
                        match self.focus {
                            Focus::Sections => {
                                let _ = self.input_buffer.pop();
                            }
                            Focus::Details => {
                                match self.popup_focus {
                                    PopupFocus::Title => { let _ = self.input_buffer.pop(); }
                                    PopupFocus::Description => { let _ = self.description_buffer.pop(); }
                                    PopupFocus::Code => {
                                        if self.code_cursor > 0 {
                                            self.code_cursor -= 1;
                                            self.code_buffer.remove(self.code_cursor);
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    KeyCode::Left => {
                        if self.popup_focus == PopupFocus::Code && self.code_cursor > 0 {
                            self.code_cursor -= 1;
                        }
                    }
                    KeyCode::Right => {
                        if self.popup_focus == PopupFocus::Code && self.code_cursor < self.code_buffer.len() {
                            self.code_cursor += 1;
                        }
                    }
                    KeyCode::Esc => {
                        self.mode = Mode::Normal;
                        self.input_buffer.clear();
                        self.description_buffer.clear();
                        self.code_buffer.clear();
                        self.popup_focus = PopupFocus::Title;
                    }
                    KeyCode::Delete => {
                        if self.popup_focus == PopupFocus::Code && self.code_cursor < self.code_buffer.len() {
                            self.code_buffer.remove(self.code_cursor);
                        }
                    }
                    KeyCode::Up => {
                        if self.popup_focus == PopupFocus::Code {
                            let lines: Vec<&str> = self.code_buffer.lines().collect();
                            let current_line = self.code_cursor_line();
                            if current_line > 0 {
                                let current_column = self.code_cursor_column();
                                let mut new_pos = 0;
                                let target_line = current_line - 1;
                                
                                // Encontrar la posición de la línea anterior
                                for _ in 0..target_line {
                                    if let Some(idx) = self.code_buffer[new_pos..].find('\n') {
                                        new_pos += idx + 1;
                                    }
                                }
                                
                                // Calcular la nueva posición del cursor
                                let line_length = lines.get(target_line)
                                    .map(|line| line.len())
                                    .unwrap_or(0);
                                
                                self.code_cursor = new_pos + current_column.min(line_length);
                                
                                // Ajustar scroll si es necesario
                                if self.code_scroll > 0 && current_line <= self.code_scroll {
                                    self.code_scroll -= 1;
                                }
                            }
                        }
                    }
                    KeyCode::Down => {
                        if self.popup_focus == PopupFocus::Code {
                            let lines: Vec<&str> = self.code_buffer.lines().collect();
                            let current_line = self.code_cursor_line();
                            if current_line < lines.len() - 1 {
                                let current_column = self.code_cursor_column();
                                let next_line_start = self.code_buffer[self.code_cursor..]
                                    .find('\n')
                                    .map(|i| self.code_cursor + i + 1)
                                    .unwrap_or(self.code_buffer.len());
                                let next_line_end = self.code_buffer[next_line_start..]
                                    .find('\n')
                                    .map(|i| next_line_start + i)
                                    .unwrap_or(self.code_buffer.len());
                                self.code_cursor = next_line_start + current_column.min(
                                    next_line_end - next_line_start
                                );
                                self.code_scroll += 1;
                            }
                        }
                    }
                    KeyCode::PageUp => {
                        if self.popup_focus == PopupFocus::Code {
                            let lines: Vec<&str> = self.code_buffer.lines().collect();
                            let current_line = self.code_cursor_line();
                            if current_line >= 5 {
                                let target_line = current_line - 5;
                                let mut pos = 0;
                                for _ in 0..target_line {
                                    if let Some(idx) = self.code_buffer[pos..].find('\n') {
                                        pos += idx + 1;
                                    }
                                }
                                self.code_cursor = pos;
                                self.code_scroll = self.code_scroll.saturating_sub(5);
                            } else {
                                self.code_cursor = 0;
                                self.code_scroll = 0;
                            }
                        }
                    }
                    KeyCode::PageDown => {
                        if self.popup_focus == PopupFocus::Code {
                            let lines: Vec<&str> = self.code_buffer.lines().collect();
                            let current_line = self.code_cursor_line();
                            if current_line + 5 < lines.len() {
                                let target_line = current_line + 5;
                                let mut pos = 0;
                                for _ in 0..target_line {
                                    if let Some(idx) = self.code_buffer[pos..].find('\n') {
                                        pos += idx + 1;
                                    } else {
                                        pos = self.code_buffer.len();
                                        break;
                                    }
                                }
                                self.code_cursor = pos;
                                self.code_scroll = self.code_scroll.saturating_add(5);
                            } else {
                                // Ir al final del código
                                self.code_cursor = self.code_buffer.len();
                                let total_lines = lines.len();
                                let visible_lines = 10;
                                self.code_scroll = total_lines.saturating_sub(visible_lines);
                            }
                        }
                    }
                    _ => {}
                }
            }
            Mode::Normal => {
                match key.code {
                    KeyCode::Char('a') => {
                        self.mode = Mode::Adding;
                        self.input_buffer.clear();
                        self.description_buffer.clear();
                        self.code_buffer.clear();
                        self.popup_focus = PopupFocus::Title;
                    }
                    KeyCode::Char('e') => self.start_editing(),
                    KeyCode::Char('d') => self.delete_current_item(),
                    KeyCode::Char('s') => self.start_search(),
                    KeyCode::Char('h') => self.mode = Mode::Help,
                    KeyCode::Char('x') => self.mode = Mode::Exporting,
                    KeyCode::Up => self.move_selection_up(),
                    KeyCode::Down => self.move_selection_down(),
                    KeyCode::Tab => self.next_focus(),
                    KeyCode::BackTab => self.previous_focus(),
                    KeyCode::Enter => {
                        if self.focus == Focus::Details {
                            if let (Some(section_idx), Some(detail_idx)) = (self.selected_section, self.selected_detail) {
                                if let Some(section) = self.sections.get(section_idx) {
                                    if let Some(detail) = section.details.get(detail_idx) {
                                        self.mode = Mode::Viewing;
                                        self.input_buffer = detail.title.clone();
                                        self.description_buffer = detail.description.clone();
                                        if let Some(ref path) = detail.code_path {
                                            self.code_buffer = fs::read_to_string(path).unwrap_or_default();
                                        }
                                        self.selected_language = detail.language.clone();
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Mode::Help | Mode::Exporting => {
                if key.code == KeyCode::Esc {
                    self.mode = Mode::Normal;
                }
            }
            Mode::Searching => self.handle_search_mode(key),
            Mode::Viewing => {
                match key.code {
                    KeyCode::Esc => {
                        self.mode = Mode::Normal;
                        self.input_buffer.clear();
                        self.description_buffer.clear();
                        self.code_buffer.clear();
                        self.code_cursor = 0;
                        self.code_scroll = 0;
                    }
                    KeyCode::Char('e') => {
                        self.mode = Mode::Editing;
                    }
                    KeyCode::Up => {
                        if self.code_scroll > 0 {
                            self.code_scroll -= 1;
                        }
                    }
                    KeyCode::Down => {
                        let total_lines = self.code_buffer.lines().count();
                        let visible_lines = 10;
                        if self.code_scroll < total_lines.saturating_sub(visible_lines) {
                            self.code_scroll += 1;
                        }
                    }
                    KeyCode::PageUp => {
                        self.code_scroll = self.code_scroll.saturating_sub(5);
                    }
                    KeyCode::PageDown => {
                        let total_lines = self.code_buffer.lines().count();
                        let visible_lines = 10;
                        let max_scroll = total_lines.saturating_sub(visible_lines);
                        self.code_scroll = (self.code_scroll + 5).min(max_scroll);
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if let Some(clipboard) = &mut self.clipboard {
                            let _ = clipboard.set_text(&self.code_buffer);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn handle_add_submit(&mut self) {
        match self.focus {
            Focus::Sections => {
                if !self.input_buffer.is_empty() {
                    let new_id = self.sections.last()
                        .map(|s| s.id + 1)
                        .unwrap_or(1);

                    let title = if !self.input_buffer.starts_with('📁') {
                        format!("📁 {}", self.input_buffer.trim())
                    } else {
                        self.input_buffer.clone()
                    };

                    let section = Section {
                        id: new_id,
                        title,
                        details: Vec::new(),
                    };

                    if let Err(e) = self.db.save_section(&section) {
                        eprintln!("Error guardando sección: {}", e);
                    } else {
                        self.sections.push(section);
                        self.selected_section = Some(self.sections.len() - 1);
                        println!("Sección guardada correctamente");
                    }
                }
            }
            Focus::Details => {
                if let Some(section_idx) = self.selected_section {
                    if !self.input_buffer.is_empty() {
                        if let Some(section) = self.sections.get_mut(section_idx) {
                            let new_id = section.details.len() + 1;
                            
                            let code_path = if !self.code_buffer.is_empty() {
                                match self.code_handler.save_code(&self.code_buffer, &self.selected_language) {
                                    Ok(path) => Some(path),
                                    Err(e) => {
                                        eprintln!("Error guardando código: {}", e);
                                        None
                                    }
                                }
                            } else {
                                None
                            };

                            let detail = Detail {
                                id: new_id,
                                title: self.input_buffer.clone(),
                                description: self.description_buffer.clone(),
                                code_path,
                                language: self.selected_language.clone(),
                                created_at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                            };

                            section.details.push(detail);
                            if let Err(e) = self.db.save_section(section) {
                                eprintln!("Error guardando detalle: {}", e);
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        // Limpiar estado y pantalla
        self.mode = Mode::Normal;
        self.input_buffer.clear();
        self.description_buffer.clear();
        self.code_buffer.clear();
        self.code_cursor = 0;
        self.code_scroll = 0;
    }

    fn handle_edit_submit(&mut self) {
        match self.focus {
            Focus::Sections => {
                if let Some(idx) = self.selected_section {
                    if let Some(section) = self.sections.get_mut(idx) {
                        section.title = if !self.input_buffer.starts_with('📁') {
                            format!("📁 {}", self.input_buffer.trim())
                        } else {
                            self.input_buffer.clone()
                        };
                        
                        // Guardar en la base de datos
                        if let Err(e) = self.db.save_section(section) {
                            eprintln!("Error guardando sección: {}", e);
                        }
                    }
                }
            }
            Focus::Details => {
                if let (Some(section_idx), Some(detail_idx)) = (self.selected_section, self.selected_detail) {
                    if let Some(section) = self.sections.get_mut(section_idx) {
                        if let Some(detail) = section.details.get_mut(detail_idx) {
                            // Actualizar datos básicos
                            detail.title = self.input_buffer.clone();
                            detail.description = self.description_buffer.clone();
                            
                            // Guardar código si hay contenido
                            if !self.code_buffer.is_empty() {
                                match self.code_handler.save_code(&self.code_buffer, &self.selected_language) {
                                    Ok(path) => {
                                        detail.code_path = Some(path);
                                        detail.language = self.selected_language.clone();
                                    }
                                    Err(e) => eprintln!("Error guardando código: {}", e),
                                }
                            }
                            
                            detail.created_at = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                            
                            // Guardar en la base de datos
                            if let Err(e) = self.db.save_section(section) {
                                eprintln!("Error guardando detalle: {}", e);
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        // Limpiar estado y volver a modo normal
        self.mode = Mode::Normal;
        self.input_buffer.clear();
        self.description_buffer.clear();
        self.code_buffer.clear();
        self.popup_focus = PopupFocus::Title;
    }

    pub fn handle_search_mode(&mut self, key: KeyEvent) {
        if self.copying {
            self.handle_copy_mode(key);
            return;
        }

        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.search_query.clear();
                self.search_results.clear();
                self.search_scroll = 0;
                self.selected_link = None;
                self.links.clear();
                self.searching = false;
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                if !self.search_query.is_empty() {
                    self.searching = true;
                    self.perform_search_based_on_target();
                }
            }
            KeyCode::Tab => {
                self.cycle_search_target();
                if !self.search_query.is_empty() {
                    self.searching = true;
                    self.perform_search_based_on_target();
                }
            }
            KeyCode::Up => {
                if self.search_scroll > 0 {
                    self.search_scroll -= 1;
                    if !self.links.is_empty() {
                        self.selected_link = Some(self.search_scroll);
                    }
                }
            }
            KeyCode::Down => {
                if self.search_scroll + 1 < self.search_results.len() {
                    self.search_scroll += 1;
                    if !self.links.is_empty() {
                        self.selected_link = Some(self.search_scroll);
                    }
                }
            }
            KeyCode::PageUp => {
                self.search_scroll = self.search_scroll.saturating_sub(5);
                if !self.links.is_empty() {
                    self.selected_link = Some(self.search_scroll);
                }
            }
            KeyCode::PageDown => {
                let max_scroll = self.search_results.len().saturating_sub(1);
                self.search_scroll = (self.search_scroll + 5).min(max_scroll);
                if !self.links.is_empty() {
                    self.selected_link = Some(self.search_scroll);
                }
            }
            KeyCode::Enter => {
                if let Some(selected) = self.selected_link {
                    if let Some(url) = self.links.get(selected) {
                        let _ = open::that(url);
                    }
                }
            }
            KeyCode::Char('c') => {
                if !self.search_results.is_empty() {
                    self.copying = true;
                    self.copy_result = Some(self.search_results[self.search_scroll].clone());
                }
            }
            KeyCode::Char(c) => {
                if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                    self.search_query.push(c);
                    self.searching = true;
                    self.perform_search_based_on_target();
                }
            }
            _ => {}
        }
    }

    fn setup_search_channel(&mut self) {
        let (tx, mut rx) = mpsc::channel(32);
        let (results_tx, results_rx) = mpsc::channel(32);
        self.search_tx = Some(tx);
        self.results_rx = Some(results_rx);

        let search_target = self.search_target.clone();

        // Spawn background task
        tokio::spawn(async move {
            let mut crates_search = CratesIoSearch::new();
            let mut cheats_search = CheatsRsSearch::new();

            while let Some(query) = rx.recv().await {
                let mut results = Vec::new();
                
                match search_target {
                    SearchTarget::CratesIo => {
                        if let Ok(search_results) = crates_search.search(&query).await {
                            results = search_results;
                        }
                    }
                    SearchTarget::CheatsRs => {
                        if let Ok(search_results) = cheats_search.search(&query).await {
                            results = search_results;
                        }
                    }
                    SearchTarget::All => {
                        if let Ok(crates_results) = crates_search.search(&query).await {
                            results.extend(crates_results);
                            if let Ok(cheats_results) = cheats_search.search(&query).await {
                                results.extend(cheats_results);
                            }
                        }
                    }
                    _ => {}
                }
                let _ = results_tx.send(results).await;
            }
        });
    }

    pub fn check_search_results(&mut self) {
        if let Some(rx) = &mut self.results_rx {
            match rx.try_recv() {
                Ok(results) => {
                    self.search_results = results;
                    self.searching = false;
                }
                Err(_) => {} // Ignorar errores de try_recv
            }
        }
    }

    pub fn perform_search_based_on_target(&mut self) {
        self.search_results.clear();
        self.links.clear();
        self.selected_link = None;
        
        if self.search_query.is_empty() {
            return;
        }

        match self.search_target {
            SearchTarget::Local => {
                self.perform_local_search(self.search_query.clone());
            }
            _ => {
                if self.search_tx.is_none() {
                    self.setup_search_channel();
                }
                
                if let Some(tx) = &self.search_tx {
                    self.searching = true;
                    let _ = tx.try_send(self.search_query.clone());
                }
            }
        }
    }

    pub fn handle_help_mode(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('h') => self.mode = Mode::Normal,
            _ => {}
        }
    }

    fn start_adding(&mut self) {
        self.mode = Mode::Adding;
        self.popup_focus = PopupFocus::Title;
        self.input_buffer.clear();
        self.description_buffer.clear();
        self.code_buffer.clear();
    }

    fn start_editing(&mut self) {
        match self.focus {
            Focus::Sections => {
                if let Some(idx) = self.selected_section {
                    if let Some(section) = self.sections.get(idx) {
                        self.input_buffer = section.title.clone();
                        if self.input_buffer.starts_with("📁 ") {
                            self.input_buffer = self.input_buffer[5..].to_string();
                        }
                        self.mode = Mode::Editing;
                    }
                }
            }
            Focus::Details => {
                if let (Some(section_idx), Some(detail_idx)) = (self.selected_section, self.selected_detail) {
                    if let Some(section) = self.sections.get(section_idx) {
                        if let Some(detail) = section.details.get(detail_idx) {
                            self.input_buffer = detail.title.clone();
                            self.description_buffer = detail.description.clone();
                            self.code_buffer = if let Some(ref path) = detail.code_path {
                                fs::read_to_string(path).unwrap_or_default()
                            } else {
                                String::new()
                            };
                            self.selected_language = detail.language.clone();
                            self.mode = Mode::Editing;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn delete_current_item(&mut self) {
        match self.focus {
            Focus::Sections => {
                if let Some(idx) = self.selected_section {
                    // Eliminar de la base de datos
                    if let Err(e) = self.db.delete_section(self.sections[idx].id) {
                        eprintln!("Error eliminando sección: {}", e);
                        return;
                    }
                    
                    // Eliminar del estado local
                    self.sections.remove(idx);
                    
                    // Actualizar selección
                    if !self.sections.is_empty() {
                        self.selected_section = Some(if idx >= self.sections.len() {
                            self.sections.len() - 1
                        } else {
                            idx
                        });
                    } else {
                        self.selected_section = None;
                    }
                    self.selected_detail = None;
                }
            }
            Focus::Details => {
                if let (Some(section_idx), Some(detail_idx)) = (self.selected_section, self.selected_detail) {
                    if let Some(section) = self.sections.get_mut(section_idx) {
                        // Eliminar de la base de datos
                        if let Err(e) = self.db.delete_detail(section.id, section.details[detail_idx].id) {
                            eprintln!("Error eliminando detalle: {}", e);
                            return;
                        }
                        
                        // Eliminar archivo de código si existe
                        if let Some(ref path) = section.details[detail_idx].code_path {
                            if let Err(e) = fs::remove_file(path) {
                                eprintln!("Error eliminando archivo de código: {}", e);
                            }
                        }
                        
                        // Eliminar del estado local
                        section.details.remove(detail_idx);
                        
                        // Actualizar selección
                        if !section.details.is_empty() {
                            self.selected_detail = Some(if detail_idx >= section.details.len() {
                                section.details.len() - 1
                            } else {
                                detail_idx
                            });
                        } else {
                            self.selected_detail = None;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn move_selection_up(&mut self) {
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

    fn move_selection_down(&mut self) {
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

    fn start_search(&mut self) {
        self.mode = Mode::Searching;
        self.search_query.clear();
        self.search_results.clear();
    }

    fn cycle_search_target(&mut self) {
        self.search_target = match self.search_target {
            SearchTarget::Local => SearchTarget::CratesIo,
            SearchTarget::CratesIo => SearchTarget::CheatsRs,
            SearchTarget::CheatsRs => SearchTarget::All,
            SearchTarget::All => SearchTarget::Local,
        };
    }

    fn perform_local_search(&mut self, query: String) {
        self.search_results.clear();
        
        if let Ok(results) = self.db.search_local(&query) {
            for (section, detail) in results {
                self.search_results.push(SearchResult {
                    title: format!("{} > {}", section.title, detail.title),
                    description: detail.description,
                    source: SearchSource::Local,
                });
            }
        }
    }

    fn toggle_popup_focus(&mut self) {
        if self.focus == Focus::Details {
            self.popup_focus = match self.popup_focus {
                PopupFocus::Title => PopupFocus::Description,
                PopupFocus::Description => PopupFocus::Code,
                PopupFocus::Code => PopupFocus::Title,
            };
        }
    }

    fn handle_copy_mode(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.copying = false;
                self.copy_result = None;
            }
            KeyCode::Enter => {
                if let Some(result) = self.copy_result.take() {
                    if let Some(section_idx) = self.selected_section {
                        if let Some(section) = self.sections.get_mut(section_idx) {
                            let new_id = section.details.len() + 1;
                            section.details.push(Detail {
                                id: new_id,
                                title: result.title,
                                description: result.description,
                                code_path: None,
                                language: Language::None,
                                created_at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                            });
                        }
                    }
                }
                self.copying = false;
            }
            KeyCode::Up | KeyCode::Down => {
                // Navegar entre resultados de búsqueda
                if let Some(current_idx) = self.search_results.iter()
                    .position(|r| Some(r) == self.copy_result.as_ref()) {
                    let new_idx = if key.code == KeyCode::Up {
                        if current_idx > 0 { current_idx - 1 } else { self.search_results.len() - 1 }
                    } else {
                        if current_idx < self.search_results.len() - 1 { current_idx + 1 } else { 0 }
                    };
                    self.copy_result = Some(self.search_results[new_idx].clone());
                }
            }
            _ => {}
        }
    }

    pub fn save_state(&self) -> Result<String, serde_json::Error> {
        // Solo guardar los campos necesarios
        let state = serde_json::json!({
            "sections": self.sections,
            "layout_sizes": self.layout_sizes,
            "search_target": self.search_target,
        });
        serde_json::to_string_pretty(&state)
    }

    pub fn load_state(&mut self, json: &str) -> Result<(), serde_json::Error> {
        let state: serde_json::Value = serde_json::from_str(json)?;
        
        if let Some(sections) = state.get("sections") {
            self.sections = serde_json::from_value(sections.clone())?;
        }
        
        if let Some(layout) = state.get("layout_sizes") {
            self.layout_sizes = serde_json::from_value(layout.clone())?;
        }
        
        if let Some(target) = state.get("search_target") {
            self.search_target = serde_json::from_value(target.clone())?;
        }
        
        Ok(())
    }

    pub fn handle_export_mode(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.export_message = None;
            }
            KeyCode::Up => {
                if self.selected_export_format > 0 {
                    self.selected_export_format -= 1;
                }
            }
            KeyCode::Down => {
                if self.selected_export_format < self.export_formats.len() - 1 {
                    self.selected_export_format += 1;
                }
            }
            KeyCode::Enter => {
                if let Some(format) = self.export_formats.get(self.selected_export_format) {
                    match export_data(self, format.clone()) {
                        Ok((source, dest)) => {
                            self.export_message = Some(format!(
                                "✅ Exportado desde:\n   {}\n   a:\n   {}", 
                                source, dest
                            ));
                        }
                        Err(e) => {
                            self.export_message = Some(format!("❌ Error al exportar: {}", e));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn cycle_language(&mut self) {
        self.selected_language = match self.selected_language {
            Language::None => Language::Rust,
            Language::Rust => Language::Python,
            Language::Python => Language::None,
        };
    }

    fn show_warning(&mut self, message: &str) {
        // TODO: Implementar sistema de mensajes/avisos
        eprintln!("{}", message);
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
}

