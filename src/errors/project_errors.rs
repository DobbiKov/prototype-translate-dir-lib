use std::path::StripPrefixError;

use crate::errors::project_config_errors::LoadConfigError;
use thiserror::Error;

use super::project_config_errors::WriteConfigError;

#[derive(Error, Debug)]
pub enum InitProjectError {
    #[error("invalid path")]
    InvalidPath,
    #[error("the project is already initialized")]
    ProjectAlreadyInitialized,
    #[error("config writing error {0}")]
    ConfigWritingError(WriteConfigError),
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

#[derive(Error, Debug)]
pub enum SyncFilesError {
    #[error("can't set translate language without source language")]
    NoSourceLang,
    #[error("no languages to translate into")]
    NoTransLangs,
    #[error("copy error: {0}")]
    CopyError(CopyFileDirError),
    #[error("building config error: {0}")]
    BuildingConfigError(std::io::Error),
    #[error("config writing error {0}")]
    ConfigWritingError(WriteConfigError),
}

#[derive(Error, Debug)]
pub enum CopyFileDirError {
    #[error("io error: {0}")]
    IoError(std::io::Error),
    #[error("strip path error: {0}")]
    StripPathError(StripPrefixError),
}

#[derive(Error, Debug)]
pub enum AddTranslatableFileError {
    #[error("can't set translate language without source language")]
    NoSourceLang,
    #[error("there is no such file")]
    NoFile,
    #[error("config writing error {0}")]
    ConfigWritingError(WriteConfigError),
}

#[derive(Error, Debug)]
pub enum GetTranslatableFilesError {
    #[error("can't set translate language without source language")]
    NoSourceLang,
}
