use thiserror::Error;

#[derive(Error, Debug)]
pub enum LoadConfigError {
    #[error("open config file error {0}")]
    OpenConfigFileError(std::io::Error),
    #[error("incorrect config file format")]
    IncorrectConfigFileFormat,
}

#[derive(Error, Debug)]
pub enum WriteConfigError {
    #[error("io error: {0}")]
    IoError(std::io::Error),
    #[error("serialisation error: {0}")]
    SerialisationError(String),
}
