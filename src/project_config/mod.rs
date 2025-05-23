use crate::Language;
use serde;
use std::{
    io::Write,
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
/// A struct representing a particular project's config, this config contains the root directory
/// structure and the
pub struct ProjectConfig {
    /// name for the current project
    name: String,
    /// the directory assigned to each target language
    lang_dirs: Vec<LangDir>,
    /// the master directory that the files are copied and translated from
    src_dir: Option<LangDir>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
/// A master directory for a language that copies the master one
pub struct LangDir {
    dir: Directory,
    language: Language,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
/// A config representation of a directory
pub struct Directory {
    /// name of the directory
    name: String,
    /// path to the directory
    path: PathBuf,
    /// directory that this one contains
    dirs: Vec<Directory>,
    /// files that this directory contains
    files: Vec<File>,
}

impl Directory {
    fn new(name: &str, path: PathBuf) -> Self {
        Directory {
            name: name.to_string(),
            path,
            dirs: vec![],
            files: vec![],
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
/// A config for a file
pub struct File {
    /// name of the file
    name: String,
    /// path to file
    path: PathBuf,
    /// if the file is translatable (false is not, true if it is)
    translatable: bool,
}

impl ProjectConfig {
    fn new(proj_name: &str) -> Self {
        ProjectConfig {
            name: proj_name.to_string(),
            lang_dirs: Vec::new(),
            src_dir: None,
        }
    }
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

/// Build a `Directory` tree rooted at `root`.
pub fn build_tree<P: AsRef<Path>>(root: P) -> std::io::Result<Directory> {
    fn recurse(path: &Path) -> std::io::Result<Directory> {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| String::from("/"));

        let mut dir = Directory::new(&name, path.to_path_buf());

        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let meta = entry.metadata()?;

            if meta.is_symlink() {
                continue;
            }

            if meta.is_dir() {
                dir.dirs.push(recurse(&entry.path())?);
            } else if meta.is_file() {
                let file_name = entry.file_name().to_string_lossy().into_owned();
                dir.files.push(File {
                    name: file_name.clone(),
                    path: entry.path(),
                    translatable: false,
                });
            }
        }

        Ok(dir)
    }

    recurse(root.as_ref())
}

// init project for translation
// we should go through the directory recursively and parse dir-file tree
// ideas: files_to_llm
pub fn init(proj_name: &str, path: PathBuf) -> Result<(), InitProjectError> {
    if !path.exists() {
        return Err(InitProjectError::InvalidPath);
    }
    let config_filename = "trans_conf.json";
    let config_file_fullpath = path.join(config_filename);
    if config_file_fullpath.exists() {
        return Err(InitProjectError::ProjectAlreadyInitialized);
    }

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .read(true)
        .open(config_file_fullpath)
        .map_err(InitProjectError::FileCreatingError)?;
    let conf = ProjectConfig::new(proj_name);
    let serialized = serde_json::to_string(&conf)
        .map_err(|e| InitProjectError::SerialisationError(e.to_string()))?;
    file.write_fmt(format_args!("{}", serialized))
        .map_err(InitProjectError::FileCreatingError)?;
    Ok(())
}

// commands
//pub fn add_lang_dir(dir_name: &str, lang: Language) -> Result<(), Box<dyn std::error::Error>> {
//    todo!()
//}
