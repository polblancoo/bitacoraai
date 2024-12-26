use crate::search::SearchResult;
use crate::search::SearchSource;
use chrono::Local;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};
use crate::search::cheats::CheatsRsSearch;
use crate::search::crates_io::CratesIoSearch;
use crate::search::SearchProvider;

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
    pub created_at: String,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Focus {
    Sections,
    Details,
    Search,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SearchTarget {
    Local,
    CratesIo,
    CheatsRs,
    All,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Mode {
    Normal,
    Adding,
    Editing,
    Searching,
    Help,
}

#[derive(Debug, PartialEq, Clone)]
pub enum PopupFocus {
    Title,
    Description,
}

#[derive(Clone)]
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
}

#[derive(Clone)]
pub struct LayoutSizes {
    pub left_panel_width: u16,
    pub right_panel_width: u16,
}

impl App {
    pub fn new() -> Self {
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

    pub fn handle_input(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Tab {
            match self.mode {
                Mode::Normal => self.next_focus(),
                Mode::Adding | Mode::Editing => {
                    if self.focus == Focus::Details {
                        self.popup_focus = match self.popup_focus {
                            PopupFocus::Title => PopupFocus::Description,
                            PopupFocus::Description => PopupFocus::Title,
                        };
                    }
                }
                Mode::Searching => {
                    self.cycle_search_target();
                    if !self.search_query.is_empty() {
                        self.perform_local_search(self.search_query.clone());
                    }
                }
                _ => {}
            }
            return;
        }

        match self.mode {
            Mode::Normal => {
                match key.code {
                    KeyCode::BackTab => self.previous_focus(),
                    _ => self.handle_normal_mode(key),
                }
            }
            Mode::Adding => self.handle_adding_mode(key),
            Mode::Editing => self.handle_editing_mode(key),
            Mode::Searching => self.handle_search_mode(key),
            Mode::Help => self.handle_help_mode(key),
        }
    }

    fn handle_normal_mode(&mut self, key: KeyEvent) {
        match (key.code, key.modifiers) {
            (KeyCode::Char('q'), _) => {}, // Salir
            (KeyCode::Char('h'), _) => self.mode = Mode::Help, // Ayuda
            (KeyCode::Char('a'), _) => self.start_adding(), // Agregar
            (KeyCode::Char('e'), _) => self.start_editing(), // Editar
            (KeyCode::Char('d'), _) => self.delete_current_item(), // Eliminar
            (KeyCode::Char('s'), _) => self.start_search(), // Buscar
            (KeyCode::Left, KeyModifiers::CONTROL) => self.resize_panel(true), // Redimensionar
            (KeyCode::Right, KeyModifiers::CONTROL) => self.resize_panel(false), // Redimensionar
            (KeyCode::Up, _) => self.move_selection_up(), // Mover arriba
            (KeyCode::Down, _) => self.move_selection_down(), // Mover abajo
            _ => {}
        }
    }

    fn resize_panel(&mut self, decrease: bool) {
        match self.focus {
            Focus::Sections => {
                if decrease && self.layout_sizes.left_panel_width > 20 {
                    self.layout_sizes.left_panel_width -= 5;
                    self.layout_sizes.right_panel_width += 5;
                } else if !decrease && self.layout_sizes.left_panel_width < 50 {
                    self.layout_sizes.left_panel_width += 5;
                    self.layout_sizes.right_panel_width -= 5;
                }
            }
            Focus::Details => {
                if decrease && self.layout_sizes.right_panel_width > 40 {
                    self.layout_sizes.right_panel_width -= 5;
                    self.layout_sizes.left_panel_width += 5;
                } else if !decrease && self.layout_sizes.right_panel_width < 80 {
                    self.layout_sizes.right_panel_width += 5;
                    self.layout_sizes.left_panel_width -= 5;
                }
            }
            Focus::Search => {}
        }
    }

    pub fn handle_adding_mode(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.input_buffer.clear();
                self.description_buffer.clear();
                self.popup_focus = PopupFocus::Title;
            }
            KeyCode::Enter => {
                self.handle_add_submit();
                self.mode = Mode::Normal;
                self.input_buffer.clear();
                self.description_buffer.clear();
                self.popup_focus = PopupFocus::Title;
            }
            KeyCode::Char(c) => {
                match self.popup_focus {
                    PopupFocus::Title => self.input_buffer.push(c),
                    PopupFocus::Description => self.description_buffer.push(c),
                }
            }
            KeyCode::Backspace => {
                match self.popup_focus {
                    PopupFocus::Title => { self.input_buffer.pop(); }
                    PopupFocus::Description => { self.description_buffer.pop(); }
                }
            }
            _ => {}
        }
    }

    fn handle_add_submit(&mut self) {
        match self.focus {
            Focus::Sections => {
                if !self.input_buffer.is_empty() {
                    let new_id = self.sections.len() + 1;
                    let title = if !self.input_buffer.starts_with("📁") {
                        format!("📁 {}", self.input_buffer.trim())
                    } else {
                        self.input_buffer.clone()
                    };
                    self.sections.push(Section {
                        id: new_id,
                        title,
                        details: Vec::new(),
                    });
                    self.mode = Mode::Normal;
                    self.input_buffer.clear();
                }
            }
            Focus::Details => {
                if !self.input_buffer.is_empty() && self.selected_section.is_some() {
                    if let Some(section) = self.sections.get_mut(self.selected_section.unwrap()) {
                        let new_id = section.details.len() + 1;
                        section.details.push(Detail {
                            id: new_id,
                            title: self.input_buffer.clone(),
                            description: self.description_buffer.clone(),
                            created_at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                        });
                        self.mode = Mode::Normal;
                        self.input_buffer.clear();
                        self.description_buffer.clear();
                    }
                }
            }
            _ => {}
        }
    }

    pub fn handle_editing_mode(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.input_buffer.clear();
                self.description_buffer.clear();
                self.popup_focus = PopupFocus::Title;
            }
            KeyCode::Enter => {
                self.handle_edit_submit();
                self.mode = Mode::Normal;
                self.input_buffer.clear();
                self.description_buffer.clear();
                self.popup_focus = PopupFocus::Title;
            }
            KeyCode::Char(c) => {
                match self.popup_focus {
                    PopupFocus::Title => self.input_buffer.push(c),
                    PopupFocus::Description => self.description_buffer.push(c),
                }
            }
            KeyCode::Backspace => {
                match self.popup_focus {
                    PopupFocus::Title => { self.input_buffer.pop(); }
                    PopupFocus::Description => { self.description_buffer.pop(); }
                }
            }
            _ => {}
        }
    }

    fn handle_edit_submit(&mut self) {
        match self.focus {
            Focus::Sections => {
                if let Some(index) = self.selected_section {
                    if !self.input_buffer.is_empty() {
                        if let Some(section) = self.sections.get_mut(index) {
                            let title = if !self.input_buffer.starts_with("📁") {
                                format!("📁 {}", self.input_buffer.trim())
                            } else {
                                self.input_buffer.clone()
                            };
                            section.title = title;
                        }
                    }
                }
            }
            Focus::Details => {
                if let (Some(section_idx), Some(detail_idx)) = (self.selected_section, self.selected_detail) {
                    if !self.input_buffer.is_empty() {
                        if let Some(section) = self.sections.get_mut(section_idx) {
                            if let Some(detail) = section.details.get_mut(detail_idx) {
                                detail.title = self.input_buffer.clone();
                                detail.description = self.description_buffer.clone();
                                detail.created_at = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                            }
                        }
                    }
                }
            }
            Focus::Search => {}
        }
    }

    pub fn handle_search_mode(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.search_query.clear();
                self.search_results.clear();
            }
            KeyCode::Tab => {
                self.cycle_search_target();
                if !self.search_query.is_empty() {
                    tokio::spawn({
                        let mut app = self.clone();
                        async move {
                            app.perform_search().await;
                        }
                    });
                }
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
                tokio::spawn({
                    let mut app = self.clone();
                    async move {
                        app.perform_search().await;
                    }
                });
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                tokio::spawn({
                    let mut app = self.clone();
                    async move {
                        app.perform_search().await;
                    }
                });
            }
            _ => {}
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
    }

    fn start_editing(&mut self) {
        self.popup_focus = PopupFocus::Title;
        match self.focus {
            Focus::Sections => {
                if let Some(index) = self.selected_section {
                    if let Some(section) = self.sections.get(index) {
                        self.mode = Mode::Editing;
                        // Cargar el título sin el ícono para edición
                        self.input_buffer = if section.title.starts_with("📁") {
                            section.title[4..].to_string() // Saltar "📁 "
                        } else {
                            section.title.clone()
                        };
                    }
                }
            }
            Focus::Details => {
                if let (Some(section_idx), Some(detail_idx)) = (self.selected_section, self.selected_detail) {
                    if let Some(section) = self.sections.get(section_idx) {
                        if let Some(detail) = section.details.get(detail_idx) {
                            self.mode = Mode::Editing;
                            self.input_buffer = detail.title.clone();
                            self.description_buffer = detail.description.clone();
                        }
                    }
                }
            }
            Focus::Search => {}
        }
    }

    fn delete_current_item(&mut self) {
        match self.focus {
            Focus::Sections => {
                if let Some(index) = self.selected_section {
                    self.sections.remove(index);
                    if self.sections.is_empty() {
                        self.selected_section = None;
                    } else if index >= self.sections.len() {
                        self.selected_section = Some(self.sections.len() - 1);
                    }
                }
            }
            Focus::Details => {
                if let (Some(section_idx), Some(detail_idx)) =
                    (self.selected_section, self.selected_detail)
                {
                    if let Some(section) = self.sections.get_mut(section_idx) {
                        section.details.remove(detail_idx);
                        if section.details.is_empty() {
                            self.selected_detail = None;
                        } else if detail_idx >= section.details.len() {
                            self.selected_detail = Some(section.details.len() - 1);
                        }
                    }
                }
            }
            Focus::Search => {} // No deletion in search mode
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
        if !self.search_query.is_empty() {
            self.perform_local_search(self.search_query.clone());
        }
    }

    async fn perform_crates_io_search(&self, query: &str) -> crate::error::Result<Vec<SearchResult>> {
        let mut searcher = CratesIoSearch::new();
        searcher.search(query).await
    }

    async fn perform_cheats_rs_search(&self, query: &str) -> crate::error::Result<Vec<SearchResult>> {
        let mut searcher = CheatsRsSearch::new();
        searcher.search(query).await
    }

    async fn perform_search(&mut self) {
        self.search_results.clear();
        let query = self.search_query.clone();
        
        if query.is_empty() {
            return;
        }

        match self.search_target {
            SearchTarget::Local => {
                self.perform_local_search(query);
            }
            SearchTarget::CratesIo => {
                if let Ok(results) = self.perform_crates_io_search(&query).await {
                    self.search_results.extend(results);
                }
            }
            SearchTarget::CheatsRs => {
                if let Ok(results) = self.perform_cheats_rs_search(&query).await {
                    self.search_results.extend(results);
                }
            }
            SearchTarget::All => {
                self.perform_local_search(query.clone());
                
                if let Ok(results) = self.perform_crates_io_search(&query).await {
                    self.search_results.extend(results);
                }
                if let Ok(results) = self.perform_cheats_rs_search(&query).await {
                    self.search_results.extend(results);
                }
            }
        }
    }

    fn perform_local_search(&mut self, query: String) {
        self.search_results.clear();
        let query = query.to_lowercase();
        
        if query.is_empty() {
            return;
        }

        for section in &self.sections {
            if section.title.to_lowercase().contains(&query) {
                self.search_results.push(SearchResult {
                    title: section.title.clone(),
                    description: format!("Sección encontrada"),
                    source: SearchSource::Local,
                });
            }

            for detail in &section.details {
                if detail.title.to_lowercase().contains(&query) 
                   || detail.description.to_lowercase().contains(&query) {
                    self.search_results.push(SearchResult {
                        title: detail.title.clone(),
                        description: detail.description.clone(),
                        source: SearchSource::Local,
                    });
                }
            }
        }
    }

    fn toggle_popup_focus(&mut self) {
        if self.focus == Focus::Details {
            self.popup_focus = match self.popup_focus {
                PopupFocus::Title => PopupFocus::Description,
                PopupFocus::Description => PopupFocus::Title,
            };
        }
    }
}

