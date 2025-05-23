use crate::{helper, project_config::LoadConfigError, Language};
use std::{io::Read, path::PathBuf};
use thiserror::Error;

use crate::project_config::ProjectConfig;

#[derive(Debug)]
/// Struct representing the full project for translation
pub struct Project {
    /// Absolute path to the root directory of the project
    path_to_root: PathBuf,
    /// Config of the project
    config: ProjectConfig,
}

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

/// Initialize project for translation
pub fn init(name: &str, path: PathBuf) -> Result<(), InitProjectError> {
    if !path.is_dir() {
        return Err(InitProjectError::InvalidPath);
    }
    let path = std::fs::canonicalize(path).map_err(|_| InitProjectError::InvalidPath)?;

    let conf = crate::project_config::init(name, path)?;

    Ok(())
}

/// Load project from the given path (even if the path is a child of the project directory)
pub fn load(path: PathBuf) -> Result<Project, LoadProjectError> {
    let conf_file_path = match helper::find_file_upwards(path, "trans_conf.json") {
        None => return Err(LoadProjectError::NoConfig),
        Some(r) => r,
    };
    let root = {
        // Yeah, I wrote the same thing in two different ways
        if let Some(p) = conf_file_path.clone().parent() {
            p.to_path_buf()
        } else {
            return Err(LoadProjectError::NoConfig);
        }
    };

    let conf = crate::project_config::load_config_from_file(conf_file_path)
        .map_err(LoadProjectError::LoadConfigError)?;

    Ok(Project {
        path_to_root: root,
        config: conf,
    })
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

impl Project {
    pub fn get_root_path(&self) -> std::path::PathBuf {
        self.path_to_root.clone()
    }
    pub fn get_config(&self) -> ProjectConfig {
        self.config.clone()
    }
    /// Set source directory that the contents will be translated of
    pub fn set_source_dir(
        &mut self,
        dir_name: &str,
        lang: Language,
    ) -> Result<(), SetSourceDirError> {
        let full_dir_path = self.get_root_path().join(dir_name);
        if !full_dir_path.exists() {
            return Err(SetSourceDirError::DirectoryDoesNotExist);
        }
        if !full_dir_path.is_dir() {
            return Err(SetSourceDirError::NotDirectory);
        }

        //set as src dir
        let _ = self
            .config
            .set_src_dir(full_dir_path, lang)
            .map_err(SetSourceDirError::AnalyzeDirError);

        Ok(())
    }
}
