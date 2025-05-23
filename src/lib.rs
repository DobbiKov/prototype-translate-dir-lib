pub mod helper;
pub mod lib_config;
pub mod project;
pub mod project_config;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Language {
    French,
    English,
    German,
    Spanish,
    Ukrainian,
}

impl From<Language> for &str {
    fn from(value: Language) -> Self {
        match value {
            Language::French => "French",
            Language::English => "English",
            Language::German => "German",
            Language::Spanish => "Spanish",
            Language::Ukrainian => "Ukrainian",
        }
    }
}
