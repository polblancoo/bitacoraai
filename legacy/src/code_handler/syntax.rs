use syntect::easy::HighlightLines;
use syntect::highlighting::{ThemeSet, Style};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

pub fn highlight_code(code: &str, language: &str) -> String {
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    
    let syntax = match language {
        "rust" => ps.find_syntax_by_extension("rs"),
        "python" => ps.find_syntax_by_extension("py"),
        _ => ps.find_syntax_by_extension("txt"),
    }.unwrap_or_else(|| ps.find_syntax_plain_text());
    
    let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);
    
    let mut highlighted = String::new();
    for line in LinesWithEndings::from(code) {
        let ranges: Vec<(Style, &str)> = h.highlight_line(line, &ps).unwrap();
        let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
        highlighted.push_str(&escaped);
    }
    
    highlighted
} 