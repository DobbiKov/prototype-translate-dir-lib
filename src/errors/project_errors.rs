use crate::errors::project_config_errors::LoadConfigError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum InitProjectError {
    #[error("file creating error")]
    FileCreatingError(std::io::Error),
    #[error("invalid path")]
    InvalidPath,
    #[error("the project is already initialized")]
    ProjectAlreadyInitialized,
    #[error("project tree parsing error: {0}")]
    TreeParsingError(std::io::Error),
    #[error("serialisation error {0}")]
    SerialisationError(String),
}

#[derive(Error, Debug)]
pub enum LoadProjectError {
    #[error("there's no config to load from")]
    NoConfig,
    #[error("load config error {0}")]
    LoadConfigError(LoadConfigError),
}
#[derive(Error, Debug)]
pub enum SetSourceDirError {
    #[error("directory doesn't exist")]
    DirectoryDoesNotExist,
    #[error("incorrect path")]
    IncorrectPath,
    #[error("provided path is not directory")]
    NotDirectory,
    #[error("couldn't analyze directory {0}")]
    AnalyzeDirError(std::io::Error),
}

#[derive(Error, Debug)]
pub enum AddLanguageError {
    #[error("language already in the project")]
    LangAlreadyInTheProj,
    #[error("io error {0}")]
    IoError(std::io::Error),
    #[error("can't set translate language without source language")]
    NoSourceLang,
    #[error("language directory already exists")]
    LangDirExists,
}
