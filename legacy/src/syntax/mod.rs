use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};
use crate::languages::Language;

pub fn highlight_code(code: &str, language: &Language) -> Vec<Line<'static>> {
    match language {
        Language::Rust => highlight_rust(code),
        Language::Python => highlight_python(code),
        Language::None => vec![Line::from(code.to_string())],
    }
}

fn highlight_python(code: &str) -> Vec<Line<'static>> {
    let keywords = vec!["def", "class", "if", "else", "elif", "for", "while", "import", "from", "return"];
    let types = vec!["str", "int", "float", "bool", "list", "dict", "tuple", "set"];

    code.lines().map(|line| {
        let mut spans = Vec::new();
        let mut current_word = String::new();
        let mut chars = line.chars().peekable();

        while let Some(c) = chars.next() {
            match c {
                '#' => {
                    // Comentario
                    let comment: String = std::iter::once(c).chain(chars).collect();
                    spans.push(Span::styled(comment.to_string(), Style::default().fg(Color::DarkGray)));
                    break;
                }
                '"' | '\'' => {
                    // String literal
                    let quote = c;
                    let mut string = String::from(c);
                    while let Some(next) = chars.next() {
                        string.push(next);
                        if next == quote { break; }
                    }
                    spans.push(Span::styled(string.to_string(), Style::default().fg(Color::Green)));
                }
                c if c.is_whitespace() => {
                    if !current_word.is_empty() {
                        let style = if keywords.contains(&current_word.as_str()) {
                            Style::default().fg(Color::Yellow)
                        } else if types.contains(&current_word.as_str()) {
                            Style::default().fg(Color::Cyan)
                        } else {
                            Style::default()
                        };
                        spans.push(Span::styled(current_word.clone(), style));
                        current_word.clear();
                    }
                    spans.push(Span::raw(c.to_string()));
                }
                _ => current_word.push(c),
            }
        }

        if !current_word.is_empty() {
            let style = if keywords.contains(&current_word.as_str()) {
                Style::default().fg(Color::Yellow)
            } else if types.contains(&current_word.as_str()) {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            };
            spans.push(Span::styled(current_word.to_string(), style));
        }

        Line::from(spans)
    }).collect()
}

fn highlight_rust(code: &str) -> Vec<Line<'static>> {
    let keywords = vec!["fn", "let", "mut", "pub", "struct", "enum", "impl", "use", "mod"];
    let types = vec!["String", "Vec", "Option", "Result", "i32", "u32", "bool"];

    code.lines().map(|line| {
        let mut spans = Vec::new();
        let mut current_word = String::new();
        let mut chars = line.chars().peekable();

        while let Some(c) = chars.next() {
            match c {
                '"' => {
                    // String literal
                    let mut string = String::from(c);
                    while let Some(next) = chars.next() {
                        string.push(next);
                        if next == '"' { break; }
                    }
                    spans.push(Span::styled(string.to_string(), Style::default().fg(Color::Green)));
                }
                '/' if chars.peek() == Some(&'/') => {
                    // Comentario
                    let comment: String = std::iter::once(c).chain(chars).collect();
                    spans.push(Span::styled(comment.to_string(), Style::default().fg(Color::DarkGray)));
                    break;
                }
                c if c.is_whitespace() => {
                    if !current_word.is_empty() {
                        let style = if keywords.contains(&current_word.as_str()) {
                            Style::default().fg(Color::Yellow)
                        } else if types.contains(&current_word.as_str()) {
                            Style::default().fg(Color::Cyan)
                        } else {
                            Style::default()
                        };
                        spans.push(Span::styled(current_word.clone(), style));
                        current_word.clear();
                    }
                    spans.push(Span::raw(c.to_string()));
                }
                _ => current_word.push(c),
            }
        }

        if !current_word.is_empty() {
            let style = if keywords.contains(&current_word.as_str()) {
                Style::default().fg(Color::Yellow)
            } else if types.contains(&current_word.as_str()) {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            };
            spans.push(Span::styled(current_word.to_string(), style));
        }

        Line::from(spans)
    }).collect()
} 