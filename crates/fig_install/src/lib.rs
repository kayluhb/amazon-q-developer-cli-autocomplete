#[cfg(target_os = "freebsd")]
mod freebsd;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(windows)]
mod windows;

use std::time::SystemTimeError;

#[cfg(target_os = "freebsd")]
use freebsd as os;
#[cfg(target_os = "linux")]
use linux as os;
#[cfg(target_os = "macos")]
use macos as os;
#[cfg(target_os = "macos")]
pub use os::uninstall_terminal_integrations;
use thiserror::Error;
use tracing::error;
#[cfg(windows)]
use windows as os;

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
    Util(#[from] fig_util::Error),
    #[error(transparent)]
    Integration(#[from] fig_integrations::Error),
    #[error(transparent)]
    Settings(#[from] fig_settings::Error),
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
    #[cfg(target_os = "macos")]
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

impl From<fig_util::directories::DirectoryError> for Error {
    fn from(err: fig_util::directories::DirectoryError) -> Self {
        fig_util::Error::Directory(err).into()
    }
}
