use crate::{
    errors::project_errors::{
        AddLanguageError, AddTranslatableFileError, CopyFileDirError, GetTranslatableFilesError,
        InitProjectError, LoadProjectError, RemoveLangaugeError, SetSourceDirError, SyncFilesError,
        TranslateFileError, UpdateSourceDirConfig,
    },
    helper,
    project_config::{write_conf, Directory},
    Language,
};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

use crate::project_config::ProjectConfig;

#[derive(Debug)]
/// Struct representing the full project for translation
pub struct Project {
    /// Absolute path to the root directory of the project
    path_to_root: PathBuf,
    /// Config of the project
    config: ProjectConfig,
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

impl Project {
    /// returns the path to the root folder of the project
    pub fn get_root_path(&self) -> std::path::PathBuf {
        self.path_to_root.clone()
    }
    /// return the config
    pub fn get_config(&self) -> ProjectConfig {
        self.config.clone()
    }
    pub fn get_config_as_ref(&self) -> &ProjectConfig {
        &self.config
    }
    /// returns the path to the config file
    fn get_config_file_path(&self) -> PathBuf {
        self.get_root_path().join("trans_conf.json")
    }

    /// returns source language in an option or None if the source directory with a language isn't set
    fn get_src_lang(&self) -> Option<Language> {
        let conf = self.get_config();
        let src_dir = match conf.get_src_dir_as_ref() {
            Some(r) => r,
            None => {
                return None;
            }
        };
        let src_lang = &src_dir.get_lang();
        Some(src_lang.clone())
    }
    /// returns all the target languages from config
    fn get_tgt_langs(&self) -> Vec<Language> {
        let conf = self.get_config();
        conf.get_lang_dirs_as_ref()
            .iter()
            .map(|e| e.get_lang())
            .collect()
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

        let src_lang_op = self.get_src_lang();

        // verifying this lang isn't in the project
        if let Some(src_lang) = src_lang_op {
            if src_lang == lang {
                return Err(SetSourceDirError::LangAlreadyInTheProj);
            }
        }
        for lang_dir in self.config.get_lang_dirs_as_ref() {
            let t_lang = lang_dir.get_lang();
            if t_lang == lang {
                return Err(SetSourceDirError::LangAlreadyInTheProj);
            }
        }

        //set as src dir
        let _ = self
            .config
            .set_src_dir(full_dir_path, lang)
            .map_err(SetSourceDirError::AnalyzeDirError);

        let _ = write_conf(self.get_config_file_path(), &self.get_config());
        Ok(())
    }

    /// adds a language that the source directory will be translated into
    pub fn add_lang(&mut self, lang: Language) -> Result<(), AddLanguageError> {
        // verifying we can create a directory for the lang
        let mut dir_name = self.get_config().get_name().clone();
        dir_name.push_str(lang.get_dir_suffix());

        let new_path = self.get_root_path().join(dir_name);

        if new_path.exists() {
            return Err(AddLanguageError::LangDirExists);
        }

        // verifying there's a source language
        let conf = self.get_config();
        let src_lang = self.get_src_lang().ok_or(AddLanguageError::NoSourceLang)?;

        // verifying this lang isn't in the project
        if src_lang == lang {
            return Err(AddLanguageError::LangAlreadyInTheProj);
        }
        for lang_dir in conf.get_lang_dirs_as_ref() {
            let t_lang = lang_dir.get_lang();
            if t_lang == lang {
                return Err(AddLanguageError::LangAlreadyInTheProj);
            }
        }

        std::fs::create_dir(&new_path).map_err(AddLanguageError::IoError)?;

        self.config
            .add_lang(new_path, lang)
            .map_err(AddLanguageError::IoError)?;

        let _ = write_conf(self.get_config_file_path(), &self.get_config());

        Ok(())
    }

    /// removes the given language from the target languages and removes it's directory
    pub fn remove_lang(&mut self, lang: Language) -> Result<(), RemoveLangaugeError> {
        let tgt_lang_path = match self.config.get_tgt_dir_path_by_lang(&lang).as_ref() {
            None => return Err(RemoveLangaugeError::TargetLanguageNotInProject),
            Some(r) => r.to_path_buf(),
        };

        if !tgt_lang_path.exists() || !tgt_lang_path.is_dir() {
            return Err(RemoveLangaugeError::LangDirDoesNotExist);
        }

        self.config.remove_lang(lang);

        let _ = write_conf(self.get_config_file_path(), &self.get_config());
        std::fs::remove_dir_all(&tgt_lang_path).map_err(RemoveLangaugeError::IoError)?;
        Ok(())
    }

    /// Syncing untranslatable files from the source directory to the target directories
    pub fn sync_files(&mut self) -> Result<(), SyncFilesError> {
        let src_lang = self.get_src_lang().ok_or(SyncFilesError::NoSourceLang)?;

        self.update_project_structure()
            .map_err(SyncFilesError::UpdateStructureError)?;

        let conf = self.get_config_as_ref();
        let lang_dirs = conf.get_lang_dirs_as_ref();
        if lang_dirs.is_empty() {
            return Err(SyncFilesError::NoTransLangs);
        }

        let lang_dirs_names: Vec<String> = lang_dirs
            .iter()
            .map(|e| e.get_dir_as_ref().get_dir_name())
            .collect();

        let src_dir = conf.get_src_dir_as_ref();
        let src_dir_name = if let Some(l_dir) = src_dir {
            l_dir.get_dir_as_ref().get_dir_name()
        } else {
            panic!("impossible case")
        };

        let lang_src_dir = src_dir.clone().unwrap();
        let src_dir = lang_src_dir.get_dir_as_ref();

        // copy files
        for d_name in lang_dirs_names {
            remove_files_not_in_source_dir(
                &src_dir.get_path(),
                &self.get_root_path().join(&d_name),
                src_dir,
            )
            .map_err(SyncFilesError::RemoveUntrackedError)?;
            copy_untranslatable_files(&self.get_root_path(), &src_dir_name, &d_name, src_dir)
                .map_err(SyncFilesError::CopyError)?;
        }
        self.config
            .analyze_lang_dirs()
            .map_err(SyncFilesError::BuildingConfigError)?;
        write_conf(self.get_config_file_path(), &self.config)
            .map_err(SyncFilesError::ConfigWritingError)?;
        Ok(())
    }

    /// Makes the file by given path translatable (for the source directory)
    pub fn make_translatable_file(
        &mut self,
        path: PathBuf,
    ) -> Result<(), AddTranslatableFileError> {
        let path = std::fs::canonicalize(path).map_err(|_| AddTranslatableFileError::NoFile)?;
        self.config.make_translatable_file(path)?;
        write_conf(self.get_config_file_path(), &self.config)
            .map_err(AddTranslatableFileError::ConfigWritingError)?;
        Ok(())
    }

    /// Makes the file by given path untranslatable (for the source directory)
    pub fn make_untranslatable_file(
        &mut self,
        path: PathBuf,
    ) -> Result<(), AddTranslatableFileError> {
        let path = std::fs::canonicalize(path).map_err(|_| AddTranslatableFileError::NoFile)?;
        self.config.make_untranslatable_file(path)?;
        write_conf(self.get_config_file_path(), &self.config)
            .map_err(AddTranslatableFileError::ConfigWritingError)?;
        Ok(())
    }

    /// Returns a list of files of the source directory that are translatable
    pub fn get_translatable_files(&self) -> Result<Vec<PathBuf>, GetTranslatableFilesError> {
        let src_lang = match self.get_src_lang() {
            None => {
                return Err(GetTranslatableFilesError::NoSourceLang);
            }
            Some(s) => s,
        };
        self.config.get_translatable_files()
    }

    /// Makes the file by given path untranslatable (for the source directory)
    pub fn translate_file(&self, path: PathBuf, lang: Language) -> Result<(), TranslateFileError> {
        let path = std::fs::canonicalize(path).map_err(|_| TranslateFileError::FileNotExist)?;

        let src_lang = match self.get_src_lang() {
            None => {
                return Err(TranslateFileError::NoSourceLang);
            }
            Some(s) => s,
        };
        let tgt_langs = self.get_tgt_langs();
        if !tgt_langs.contains(&lang) {
            return Err(TranslateFileError::TargetLanguageNotInProject);
        }

        let trans_files = self
            .get_translatable_files()
            .map_err(TranslateFileError::TranslatableFilesError)?;

        if !trans_files.contains(&path) {
            return Err(TranslateFileError::UntranslatableFile);
        }

        // get new path in tgt_dir
        translate_file_helper(&path, &self.config, &lang)
    }

    /// Translates all translatable files
    pub fn translate_all(&self, lang: Language) -> Result<(), TranslateFileError> {
        let trans_files = self
            .get_translatable_files()
            .map_err(TranslateFileError::TranslatableFilesError)?;
        for file in &trans_files {
            translate_file_helper(file, &self.config, &lang)?;
        }
        Ok(())
    }

    /// Updates source directory structure (if for example it has been changed since the initialization of the project)
    pub fn update_project_structure(&mut self) -> Result<(), UpdateSourceDirConfig> {
        self.config.update_source_dir_config()
    }
}

/// Helper function to translate a file to a _lang_ language.
fn translate_file_helper(
    path: &PathBuf,
    conf: &ProjectConfig,
    lang: &Language,
) -> Result<(), TranslateFileError> {
    if !path.exists() || !path.is_file() {
        return Err(TranslateFileError::FileNotExist);
    }

    let src_dir_path = conf.get_src_dir_path().unwrap();
    let tgt_lang_path = conf.get_tgt_dir_path_by_lang(lang).unwrap();
    let relative_path = path
        .strip_prefix(src_dir_path)
        .map_err(|_| TranslateFileError::FileNotExist)?;
    let new_path = tgt_lang_path.join(relative_path);
    crate::translator::translate_file_to_file(path, new_path, lang)
        .map_err(TranslateFileError::IoError)?;
    thread::sleep(Duration::from_secs(8));
    Ok(())
}

pub fn copy_untranslatable_files(
    root_path: &Path,
    from_name: &str,
    to_name: &str,
    from_structure: &Directory,
) -> Result<(), CopyFileDirError> {
    let from_dir = root_path.join(from_name);
    let to_dir = root_path.join(to_name);
    copy_untranslatable_files_rec(&from_dir, &to_dir, from_structure)
}

fn copy_untranslatable_files_rec(
    from_dir: &Path,
    to_dir: &Path,
    dir: &Directory,
) -> Result<(), CopyFileDirError> {
    for file in dir.get_files_as_ref() {
        if file.is_translatable() {
            continue;
        }
        let full_path = file.get_path();
        let relative_path = full_path
            .strip_prefix(from_dir)
            .map_err(CopyFileDirError::StripPathError)?
            .to_path_buf();

        let new_path = to_dir.join(relative_path);
        let _ = std::fs::copy(full_path, new_path);
    }
    for sub_dir in dir.get_dirs_as_ref() {
        let full_path = sub_dir.get_path();
        let relative_path = full_path
            .strip_prefix(from_dir)
            .map_err(CopyFileDirError::StripPathError)?
            .to_path_buf();

        let new_path = to_dir.join(relative_path);
        if !&new_path.exists() {
            std::fs::create_dir(new_path).map_err(CopyFileDirError::IoError)?;
        }
        copy_untranslatable_files_rec(from_dir, to_dir, sub_dir)?;
    }
    Ok(())
}

/// Verifies and removes all the files and directories in the target directory that are not in the source directory.
///
/// - `from_dir_path`: The actual disk path of the current source directory being considered
///   (e.g., initially /path/to/project/src_en, then /path/to/project/src_en/subdir1, etc.).
/// - `to_dir_path`: The actual disk path of the current target directory being considered
///   (e.g., initially /path/to/project/target_fr, then /path/to/project/target_fr/subdir1, etc.).
/// - `source_dir_model`: The DirectoryModel representing the structure within `from_dir_path`.
///   Names within this model are relative to the current `from_dir_path`.
pub fn remove_files_not_in_source_dir(
    from_dir_path: &Path, // Path to the corresponding directory in the source structure
    to_dir_path: &Path,   // Path to the target directory to clean up
    source_dir_model: &Directory,
) -> std::io::Result<()> {
    // Collect names from the source model for efficient lookup.
    // These names are expected to be simple file/directory names, not paths.
    let model_file_names: HashSet<String> = source_dir_model
        .get_files_as_ref()
        .iter()
        .map(|f| f.get_name())
        .collect();

    let model_dir_names: HashSet<String> = source_dir_model
        .get_dirs_as_ref()
        .iter()
        .map(|d| d.get_dir_name())
        .collect();

    // Iterate over entries in the target directory on disk.
    for entry_result in std::fs::read_dir(to_dir_path)? {
        let entry = entry_result?;
        let entry_path = entry.path();
        let entry_name_os = entry.file_name(); // This is an OsString

        let entry_name_cow = entry_name_os.to_string_lossy();
        let entry_name_str = entry_name_cow.as_ref();

        let symlink_meta = std::fs::symlink_metadata(&entry_path)?;

        if symlink_meta.is_dir() {
            // Is an actual directory (not a symlink to one)
            if !model_dir_names.contains(entry_name_str) {
                // Directory exists in target but not in source model: remove it.
                if !symlink_meta.is_symlink() {
                    std::fs::remove_dir_all(&entry_path)?;
                }
            } else if !symlink_meta.is_symlink() {
                // Directory exists in both target and source model: recurse.
                // Find the corresponding Directory for this subdirectory.
                if let Some(sub_dir_model) = source_dir_model
                    .get_dirs_as_ref()
                    .iter()
                    .find(|dm| dm.get_dir_name() == entry_name_str)
                {
                    let next_from_dir_path = from_dir_path.join(&entry_name_os);
                    remove_files_not_in_source_dir(
                        &next_from_dir_path,
                        &entry_path,
                        sub_dir_model,
                    )?;
                } else {
                    // This case should ideally not be reached if model_dir_names.contains was true
                    // and get_dir_name() is consistent. Could indicate an issue or duplicate names.
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("Logic error: DirectoryModel for '{}' not found despite being in name set.", entry_name_str)
                    ));
                }
            }
        } else if symlink_meta.is_file() {
            // Is an actual file (not a symlink to one)
            if !model_file_names.contains(entry_name_str) {
                // File exists in target but not in source model: remove it.
                std::fs::remove_file(&entry_path)?;
            }
        }
    }

    Ok(())
}
