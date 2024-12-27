use crate::app::App;
use crate::error::Result;
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Clone)]
pub enum ExportFormat {
    JSON,
    HTML,
    CSV,
}

impl std::fmt::Display for ExportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportFormat::JSON => write!(f, "JSON"),
            ExportFormat::HTML => write!(f, "HTML"),
            ExportFormat::CSV => write!(f, "CSV"),
        }
    }
}

pub fn export_data(app: &App, format: ExportFormat) -> Result<(String, String)> {
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let mut path = PathBuf::from(home);
    
    let (filename, content) = match format {
        ExportFormat::JSON => {
            let data = serde_json::to_string_pretty(&app.sections)?;
            ("rust-tui-export.json", data)
        },
        ExportFormat::HTML => {
            let mut html = String::from("<!DOCTYPE html><html><head><title>Rust TUI Export</title></head><body>");
            html.push_str("<h1>Exported Sections</h1>");
            
            for section in &app.sections {
                html.push_str(&format!("<h2>{}</h2>", section.title));
                html.push_str("<ul>");
                for detail in &section.details {
                    html.push_str(&format!(
                        "<li><strong>{}</strong><br>{}<br><small>{}</small></li>",
                        detail.title, detail.description, detail.created_at
                    ));
                }
                html.push_str("</ul>");
            }
            
            html.push_str("</body></html>");
            ("rust-tui-export.html", html)
        },
        ExportFormat::CSV => {
            let mut csv = String::from("Section,Title,Description,Created At\n");
            for section in &app.sections {
                for detail in &section.details {
                    csv.push_str(&format!(
                        "\"{}\",\"{}\",\"{}\",\"{}\"\n",
                        section.title, detail.title, detail.description, detail.created_at
                    ));
                }
            }
            ("rust-tui-export.csv", csv)
        },
    };

    let source_path = env::current_dir()?.join("data.json");
    path.push(&filename);
    fs::write(&path, content)?;
    
    Ok((
        source_path.to_string_lossy().into_owned(),
        path.to_string_lossy().into_owned()
    ))
}
