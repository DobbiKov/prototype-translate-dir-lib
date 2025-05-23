use thiserror::Error;

#[derive(Error, Debug)]
pub enum LoadConfigError {
    #[error("open config file error {0}")]
    OpenConfigFileError(std::io::Error),
    #[error("incorrect config file format")]
    IncorrectConfigFileFormat,
}
