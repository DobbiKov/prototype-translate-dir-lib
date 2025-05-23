use crate::helper;
use std::{io::Read, path::PathBuf};
use thiserror::Error;

use crate::project_config::ProjectConfig;

#[derive(Debug)]
pub struct Project {
    path_to_root: PathBuf,
    config: ProjectConfig,
}

#[derive(Error, Debug)]
pub enum LoadProjectError {
    #[error("there's no config to load from")]
    NoConfig,
    #[error("open config file error {0}")]
    OpenConfigFileError(std::io::Error),
    #[error("incorrect config file format")]
    IncorrectConfigFileFormat,
}

impl Project {
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

        let mut conf_file = std::fs::OpenOptions::new()
            .read(true)
            .open(&conf_file_path)
            .map_err(LoadProjectError::OpenConfigFileError)?;
        let mut contents = String::new();
        conf_file
            .read_to_string(&mut contents)
            .map_err(LoadProjectError::OpenConfigFileError)?;
        let conf: ProjectConfig = serde_json::from_str(contents.as_str())
            .map_err(|_| LoadProjectError::IncorrectConfigFileFormat)?;

        Ok(Project {
            path_to_root: root,
            config: conf,
        })
    }
}
