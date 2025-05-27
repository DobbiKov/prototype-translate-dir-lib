use crate::errors::project_config_errors::{LoadConfigError, WriteConfigError};
use crate::errors::project_errors::{
    AddTranslatableFileError, GetTranslatableFilesError, InitProjectError, UpdateSourceDirConfig,
};
use crate::Language;
use queues::*;
use serde;
use std::collections::HashMap;
use std::{
    io::{Read, Write},
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
impl LangDir {
    pub(crate) fn new(dir: Directory, lang: Language) -> Self {
        Self {
            dir,
            language: lang,
        }
    }
    pub fn get_lang(&self) -> Language {
        self.language.clone()
    }
    pub fn get_dir_as_ref(&self) -> &Directory {
        &self.dir
    }
    pub(crate) fn set_dir(&mut self, dir: Directory) {
        self.dir = dir;
    }
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
    fn new(path: PathBuf) -> Self {
        let name = match path.file_name() {
            None => String::new(),
            Some(r) => r.to_owned().into_string().unwrap_or(String::new()),
        };

        Directory {
            name,
            path,
            dirs: vec![],
            files: vec![],
        }
    }
    pub fn get_dir_name(&self) -> String {
        self.name.clone()
    }
    pub fn get_path(&self) -> PathBuf {
        self.path.clone()
    }
    pub fn get_files_as_ref(&self) -> &Vec<File> {
        &self.files
    }
    pub fn get_dirs_as_ref(&self) -> &Vec<Directory> {
        &self.dirs
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

impl File {
    pub fn get_name(&self) -> String {
        self.name.clone()
    }
    pub fn get_path(&self) -> PathBuf {
        self.path.clone()
    }
    pub fn is_translatable(&self) -> bool {
        self.translatable
    }
}

impl ProjectConfig {
    fn new(proj_name: &str) -> Self {
        ProjectConfig {
            name: proj_name.to_string(),
            lang_dirs: Vec::new(),
            src_dir: None,
        }
    }
    pub fn get_name(&self) -> String {
        self.name.clone()
    }
    pub fn get_src_dir_as_ref(&self) -> &Option<LangDir> {
        &self.src_dir
    }
    pub fn get_lang_dirs_as_ref(&self) -> &Vec<LangDir> {
        &self.lang_dirs
    }
    pub fn get_src_dir_path(&self) -> Option<PathBuf> {
        self.src_dir
            .as_ref()
            .map(|dir| dir.get_dir_as_ref().get_path())
    }
    pub fn get_tgt_dir_path_by_lang(&self, lang: &Language) -> Option<PathBuf> {
        for dir in &self.lang_dirs {
            if dir.get_lang() == *lang {
                return Some(dir.get_dir_as_ref().get_path());
            }
        }
        None
    }
    pub(crate) fn set_src_dir(&mut self, dir_path: PathBuf, lang: Language) -> std::io::Result<()> {
        let dir = build_tree(dir_path)?;
        let lang_dir = LangDir::new(dir, lang);

        self.src_dir = Some(lang_dir);
        Ok(())
    }
    pub(crate) fn add_lang(&mut self, dir_path: PathBuf, lang: Language) -> std::io::Result<()> {
        let dir = build_tree(dir_path)?;
        let lang_dir = LangDir::new(dir, lang);
        self.lang_dirs.push(lang_dir);
        Ok(())
    }
    pub(crate) fn remove_lang(&mut self, lang: Language) {
        let mut idx: Option<usize> = None;
        for (temp_id, l_dir) in self.lang_dirs.iter().enumerate() {
            if l_dir.get_lang() == lang {
                idx = Some(temp_id);
            }
        }
        if let Some(id) = idx {
            self.lang_dirs.remove(id);
        }
    }
    pub(crate) fn analyze_lang_dirs(&mut self) -> std::io::Result<()> {
        for dir in &mut self.lang_dirs {
            let path = dir.get_dir_as_ref().get_path();
            let tree = build_tree(path)?;
            dir.set_dir(tree);
        }
        Ok(())
    }

    pub fn make_translatable_file(
        &mut self,
        path: PathBuf,
    ) -> Result<(), AddTranslatableFileError> {
        let mut func = |f: &mut File| {
            f.translatable = true;
        };
        let src_dir = &mut match &mut self.src_dir {
            Some(r) => r,
            None => {
                return Err(AddTranslatableFileError::NoSourceLang);
            }
        }
        .dir;
        let res = find_file_and_apply(src_dir, &path, &mut func);
        match res {
            true => Ok(()),
            false => Err(AddTranslatableFileError::NoFile),
        }
    }

    pub fn make_untranslatable_file(
        &mut self,
        path: PathBuf,
    ) -> Result<(), AddTranslatableFileError> {
        let mut func = |f: &mut File| {
            f.translatable = false;
        };
        let src_dir = &mut match &mut self.src_dir {
            Some(r) => r,
            None => {
                return Err(AddTranslatableFileError::NoSourceLang);
            }
        }
        .dir;
        let res = find_file_and_apply(src_dir, &path, &mut func);
        match res {
            true => Ok(()),
            false => Err(AddTranslatableFileError::NoFile),
        }
    }
    pub fn get_translatable_files(&self) -> Result<Vec<PathBuf>, GetTranslatableFilesError> {
        let mut res = Vec::<PathBuf>::new();
        let mut queue = Queue::<&Directory>::new();
        let src_dir = match &self.src_dir {
            Some(d) => &d.dir,
            None => return Err(GetTranslatableFilesError::NoSourceLang),
        }; //verified that it exists upper
        let _ = queue.add(src_dir);
        while let Ok(dir) = queue.remove() {
            for file in &dir.files {
                if file.is_translatable() {
                    res.push(file.get_path());
                }
            }
            for sub_dir in &dir.dirs {
                let _ = queue.add(sub_dir);
            }
        }
        Ok(res)
    }

    /// Updates a config file according to the source directory structure
    pub fn update_source_dir_config(&mut self) -> Result<(), UpdateSourceDirConfig> {
        let src_dir_lang = self
            .get_src_dir_as_ref()
            .as_ref()
            .ok_or(UpdateSourceDirConfig::NoSourceLang)?;

        let old_dir = src_dir_lang.get_dir_as_ref();
        let new_dir =
            build_tree(old_dir.get_path()).map_err(UpdateSourceDirConfig::AnalyzeDirError)?;

        let res_dir = compare_and_submit_dir_structs(old_dir, &new_dir);
        self.src_dir = Some(LangDir {
            dir: res_dir,
            language: src_dir_lang.get_lang(),
        });
        Ok(())
    }
}

/// Searches recursively for file in the given directory and if it finds the file it applies the
/// given function and returns true, otherwise returns false
fn find_file_and_apply<F>(dir: &mut Directory, path: &Path, func: &mut F) -> bool
where
    F: FnMut(&mut File),
{
    for file in &mut dir.files {
        if file.get_path() == *path {
            (func)(file);
            return true;
        }
    }
    for sub_dir in &mut dir.dirs {
        if find_file_and_apply(sub_dir, path, func) {
            return true;
        }
    }
    false
}

/// Build a `Directory` tree rooted at `root`.
pub fn build_tree<P: AsRef<Path>>(root: P) -> std::io::Result<Directory> {
    fn recurse(path: &Path) -> std::io::Result<Directory> {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| String::from("/"));

        let mut dir = Directory::new(path.to_path_buf());

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

/// Init project config with it's file
pub(crate) fn init(proj_name: &str, path: PathBuf) -> Result<(), InitProjectError> {
    if !path.exists() {
        return Err(InitProjectError::InvalidPath);
    }
    let config_filename = "trans_conf.json";
    let config_file_fullpath = path.join(config_filename);
    if config_file_fullpath.exists() {
        return Err(InitProjectError::ProjectAlreadyInitialized);
    }

    let conf = ProjectConfig::new(proj_name);
    let _ = write_conf(config_file_fullpath, &conf).map_err(InitProjectError::ConfigWritingError);
    Ok(())
}

pub(crate) fn write_conf(path: PathBuf, conf: &ProjectConfig) -> Result<(), WriteConfigError> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .read(true)
        .open(path)
        .map_err(WriteConfigError::IoError)?;

    let serialized = serde_json::to_string(conf)
        .map_err(|e| WriteConfigError::SerialisationError(e.to_string()))?;
    file.write_fmt(format_args!("{}", serialized))
        .map_err(WriteConfigError::IoError)?;
    Ok(())
}

pub(crate) fn load_config_from_file(path: PathBuf) -> Result<ProjectConfig, LoadConfigError> {
    let mut conf_file = std::fs::OpenOptions::new()
        .read(true)
        .open(&path)
        .map_err(LoadConfigError::OpenConfigFileError)?;
    let mut contents = String::new();
    let _ = conf_file
        .read_to_string(&mut contents)
        .map_err(LoadConfigError::OpenConfigFileError)?;
    let conf: ProjectConfig = serde_json::from_str(contents.as_str())
        .map_err(|_| LoadConfigError::IncorrectConfigFileFormat)?;

    Ok(conf)
}

/// Compares old directory structure and new one and returns the merge of both. If it encounters a
/// file or a directory in both structures, it keeps the one from the old structure, if it
/// encounters a directory or a file present in the new structure but not in the old, it will add
/// it to the result.
fn compare_and_submit_dir_structs(old_dir: &Directory, new_dir: &Directory) -> Directory {
    let mut new_model = Directory::new(new_dir.get_path().to_path_buf());

    // --- Process Files ---
    // Create a HashMap of old files for efficient lookup
    let old_files_map: HashMap<PathBuf, &File> = old_dir
        .files
        .iter()
        .map(|f| -> (PathBuf, &File) { (f.get_path(), f) })
        .collect();

    for new_file in &new_dir.files {
        // Check if the new_file's path exists in the old_files_map
        if let Some(old_file_to_keep) = old_files_map.get(&new_file.get_path()) {
            // If found in old structure, keep the old one
            new_model.files.push((*old_file_to_keep).clone());
        } else {
            // If it's a new file, add it
            new_model.files.push(new_file.clone());
        }
    }

    // --- Process Subdirectories ---
    // Create a HashMap of old subdirectories for efficient lookup
    let old_dirs_map: HashMap<PathBuf, &Directory> =
        old_dir.dirs.iter().map(|d| (d.get_path(), d)).collect();

    for new_subdir in &new_dir.dirs {
        if let Some(old_subdir_to_compare) = old_dirs_map.get(&new_subdir.get_path()) {
            // If found in old structure, analyze recursively
            let res_subdir = compare_and_submit_dir_structs(old_subdir_to_compare, new_subdir);
            new_model.dirs.push(res_subdir);
        } else {
            // If it's a new directory, add it
            new_model.dirs.push(new_subdir.clone());
        }
    }

    new_model
}
