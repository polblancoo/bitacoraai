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
    Searching,
    Help,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum PopupFocus {
    Title,
    Description,
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
    }

    fn start_editing(&mut self) {
        self.popup_focus = PopupFocus::Title;
        match self.focus {
            Focus::Sections => {
                if let Some(index) = self.selected_section {
                    if let Some(section) = self.sections.get(index) {
                        self.mode = Mode::Editing;
                        self.input_buffer = if section.title.starts_with("📁") {
                            section.title[4..].to_string()
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
            Focus::Search => {}
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
}

