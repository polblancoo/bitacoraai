use serde::{Serialize, Deserialize};
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Rust,
    Python,
    #[serde(rename = "none")]
    None,
}

impl FromStr for Language {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "rust" => Ok(Language::Rust),
            "python" => Ok(Language::Python),
            "none" | "" => Ok(Language::None),
            _ => Err(format!("Lenguaje no válido: {}", s))
        }
    }
}

impl Default for Language {
    fn default() -> Self {
        Language::None
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Language::Rust => write!(f, "🦀 Rust"),
            Language::Python => write!(f, "🐍 Python"),
            Language::None => write!(f, "📝 Texto"),
        }
    }
}

pub struct LanguageSupport {
    pub language: Language,
    pub extension: &'static str,
    pub icon: &'static str,
}

impl Language {
    pub fn get_support(&self) -> LanguageSupport {
        match self {
            Language::Rust => LanguageSupport {
                language: self.clone(),
                extension: "rs",
                icon: "🦀",
            },
            Language::Python => LanguageSupport {
                language: self.clone(),
                extension: "py",
                icon: "🐍",
            },
            Language::None => LanguageSupport {
                language: self.clone(),
                extension: "txt",
                icon: "📝",
            },
        }
    }
} 