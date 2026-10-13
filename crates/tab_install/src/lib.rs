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
    #[error("unsupported platform")]
    UnsupportedPlatform,
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
    #[error("could not determine app version")]
    UnclearVersion,
    #[error("please update from your package manager")]
    PackageManaged,
    #[error("failed to update: `{0}`")]
    UpdateFailed(String),
    #[error("failed to update: `{0}`")]
    UpdateFailedPermissions(String),
    #[error("failed to update due to auth error: `{0}`")]
    SecurityFramework(#[from] security_framework::base::Error),
    #[error("your system is not supported on this channel")]
    SystemNotOnChannel,
    #[error("manifest not found")]
    ManifestNotFound,
    #[error("Update in progress")]
    UpdateInProgress,
    #[error("could not convert path to cstring")]
    Nul(#[from] std::ffi::NulError),
    #[error("failed to get system id")]
    SystemIdNotFound,
    #[error("unable to find the bundled metadata")]
    BundleMetadataNotFound,
    #[error("unsupported variant: {0}")]
    UnsupportedVariant(String),
}

impl From<tab_util::directories::DirectoryError> for Error {
    fn from(err: tab_util::directories::DirectoryError) -> Self {
        tab_util::Error::Directory(err).into()
    }
}
