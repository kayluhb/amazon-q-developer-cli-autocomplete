pub mod macos;

use std::time::SystemTimeError;

use macos as os;
pub use os::uninstall_terminal_integrations;
use thiserror::Error;
use tracing::error;

mod common;
pub use common::{
    InstallComponents,
    install,
    uninstall,
};

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Util(#[from] tab_util::Error),
    #[error(transparent)]
    Integration(#[from] tab_integrations::Error),
    #[error(transparent)]
    Settings(#[from] tab_settings::Error),
    #[error("error converting path")]
    PathConversionError(#[from] camino::FromPathBufError),
    #[error(transparent)]
    Semver(#[from] semver::Error),
    #[error(transparent)]
    SystemTime(#[from] SystemTimeError),
    #[error(transparent)]
    Strum(#[from] strum::ParseError),
    #[error("failed to update: `{0}`")]
    UpdateFailed(String),
}

impl From<tab_util::directories::DirectoryError> for Error {
    fn from(err: tab_util::directories::DirectoryError) -> Self {
        tab_util::Error::Directory(err).into()
    }
}
