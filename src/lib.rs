pub mod errors;
pub mod helper;
pub mod lib_config;
pub mod project;
pub mod project_config;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum Language {
    French,
    English,
    German,
    Spanish,
    Ukrainian,
}

impl Language {
    pub fn get_dir_suffix(&self) -> &str {
        match self {
            Language::French => "_fr",
            Language::English => "_en",
            Language::German => "_de",
            Language::Spanish => "_sp",
            Language::Ukrainian => "_ua",
        }
    }
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
