mod app;
mod error;
mod helper;
mod search;
mod storage;
mod ui;

use crate::app::App;
use crate::error::Result;
use crate::storage::Storage;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;

#[tokio::main]
async fn main() -> Result<()> {
    let storage = Storage::new("data.json".to_string());
    
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and load data
    let mut app = App::new();
    
    // Cargar el estado guardado
    if let Ok(json) = storage.load() {
        if let Err(e) = app.load_state(&json) {
            eprintln!("Error loading state: {}", e);
        }
    }

    let res = run_app(&mut terminal, &mut app).await;

    // Guardar el estado antes de salir
    if let Err(e) = storage.save(&app) {
        eprintln!("Error saving state: {}", e);
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

async fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if let Event::Key(key) = event::read()? {
            if key.code == KeyCode::Char('q') {
                return Ok(());
            }
            app.handle_input(key);
        }

        // Verificar resultados de búsqueda
        app.check_search_results();
    }
}
