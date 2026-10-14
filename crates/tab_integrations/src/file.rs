use std::fs::Permissions;
use std::io::{
    ErrorKind,
    Write as _,
};
use std::os::unix::fs::{
    MetadataExt as _,
    PermissionsExt as _,
};
use std::path::{
    Path,
    PathBuf,
};

use async_trait::async_trait;
use tokio::fs;
use tracing::debug;

use crate::Integration;
use crate::error::{
    Error,
    ErrorExt,
    Result,
};

/// The permission bits of a file mode: rwx for all classes, plus setuid, setgid and sticky
const PERMISSION_BITS: u32 = 0o7777;

/// Replaces the contents of `path` with `contents` in one atomic step.
///
/// The data goes to a temp file in the same directory. The temp file is synced and then renamed
/// over the target. A crash thus leaves the old file or the new file, never a partial one.
///
/// If `path` is a symlink, the file it points to is replaced and the symlink stays. Dotfile
/// managers (for example stow, chezmoi or home-manager) link dotfiles into place, and a replaced
/// link would detach the file from the user's dotfile repository. A dangling symlink is an error.
///
/// If `mode` is `Some`, the file gets that mode. Otherwise an existing file keeps its mode and a
/// new file gets mode 0600. An existing file also keeps its owner when the process can set it (for
/// example under sudo).
pub(crate) fn write_atomic(path: &Path, contents: &[u8], mode: Option<u32>) -> std::io::Result<()> {
    let target = match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => std::fs::canonicalize(path)?,
        _ => path.to_path_buf(),
    };

    let existing = match std::fs::metadata(&target) {
        Ok(metadata) => Some(metadata),
        Err(err) if err.kind() == ErrorKind::NotFound => None,
        Err(err) => return Err(err),
    };

    let dir = match target.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir,
        _ => Path::new("."),
    };
    let file_name = target.file_name().unwrap_or_default().to_string_lossy();

    let mut temp = tempfile::Builder::new()
        .prefix(&format!(".{file_name}."))
        .suffix(".tmp")
        .tempfile_in(dir)?;
    temp.write_all(contents)?;

    let file = temp.as_file();
    if let Some(existing) = &existing {
        keep_owner(file, existing, &target)?;
    }

    let mode = mode.or_else(|| existing.as_ref().map(|metadata| metadata.mode() & PERMISSION_BITS));
    if let Some(mode) = mode {
        file.set_permissions(Permissions::from_mode(mode))?;
    }

    file.sync_all()?;
    temp.persist(&target).map_err(|err| err.error)?;

    // Make the rename durable. This is best effort, some file systems do not allow it.
    if let Ok(dir) = std::fs::File::open(dir) {
        let _ = dir.sync_all();
    }

    Ok(())
}

/// Gives `file` the owner of `existing`. Only a privileged process can do this. Otherwise the new
/// file has the caller's owner, like a file the caller creates.
fn keep_owner(file: &std::fs::File, existing: &std::fs::Metadata, target: &Path) -> std::io::Result<()> {
    let new = file.metadata()?;
    if existing.uid() == new.uid() && existing.gid() == new.gid() {
        return Ok(());
    }
    if let Err(err) = std::os::unix::fs::fchown(file, Some(existing.uid()), Some(existing.gid())) {
        debug!(%err, path =? target, "could not keep the file owner");
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct FileIntegration {
    pub path: PathBuf,
    pub contents: String,
    pub mode: Option<u32>,
}

impl FileIntegration {
    /// Sets `self.mode` on the existing file if its mode is different
    async fn fix_mode(&self) -> Result<()> {
        let Some(mode) = self.mode else {
            return Ok(());
        };
        let metadata = fs::metadata(&self.path).await.with_path(&self.path)?;
        if metadata.mode() & PERMISSION_BITS == mode {
            return Ok(());
        }
        fs::set_permissions(&self.path, Permissions::from_mode(mode))
            .await
            .with_path(&self.path)?;
        Ok(())
    }
}

#[async_trait]
impl Integration for FileIntegration {
    fn describe(&self) -> String {
        format!("File Integration @ {}", self.path.to_string_lossy())
    }

    async fn is_installed(&self) -> Result<()> {
        // Check for parent folder permissions issues
        {
            use nix::unistd::{
                AccessFlags,
                access,
            };

            let mut path = self.path.as_path();
            let mut res = Ok(());
            loop {
                if let Some(parent) = path.parent() {
                    match access(parent, AccessFlags::R_OK | AccessFlags::W_OK | AccessFlags::X_OK) {
                        Ok(_) => {
                            break;
                        },
                        Err(err) => {
                            res = Err(Error::PermissionDenied {
                                path: parent.into(),
                                inner: err.into(),
                            });
                            path = parent;
                        },
                    }
                }
            }
            res?;
        }

        let current_contents = match fs::read_to_string(&self.path).await.with_path(&self.path) {
            Ok(contents) => contents,
            Err(Error::Io(err)) if err.kind() == ErrorKind::NotFound => {
                return Err(Error::FileDoesNotExist(self.path.clone().into()));
            },
            Err(err) => return Err(err),
        };
        if current_contents.ne(&self.contents) {
            let message = format!("{} should contain:\n{}", self.path.display(), self.contents);
            return Err(Error::ImproperInstallation(message.into()));
        }
        Ok(())
    }

    async fn install(&self) -> Result<()> {
        if self.is_installed().await.is_ok() {
            // The contents are correct, but the mode can still be wrong
            return self.fix_mode().await;
        }

        let parent_dir = self
            .path
            .parent()
            .ok_or_else(|| Error::Custom("Could not get integration file directory".into()))?;

        if !parent_dir.is_dir() {
            fs::create_dir_all(&parent_dir).await.with_path(parent_dir)?;
        }

        debug!(path =? self.path, "Writing file integrations");
        write_atomic(&self.path, self.contents.as_bytes(), self.mode).with_path(&self.path)?;

        Ok(())
    }

    async fn uninstall(&self) -> Result<()> {
        match fs::remove_file(&self.path).await.with_path(&self.path) {
            Ok(_) => Ok(()),
            Err(Error::Io(err)) if err.kind() == ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_integration() {
        let tempdir = tempfile::tempdir().unwrap();
        let integration = FileIntegration {
            path: tempdir.path().join("integration.txt"),
            contents: "test".into(),
            // weird mode for testing
            mode: None,
        };

        assert_eq!(
            format!("File Integration @ {}/integration.txt", tempdir.path().display()),
            integration.describe()
        );

        // ensure no integration is marked as not installed
        assert!(matches!(
            integration.is_installed().await,
            Err(Error::FileDoesNotExist(_))
        ));

        // ensure the integration can be installed
        integration.install().await.unwrap();
        assert!(integration.is_installed().await.is_ok());

        // ensure the integration can be installed while already installed
        integration.install().await.unwrap();
        assert!(integration.is_installed().await.is_ok());

        // ensure the integration can be uninstalled
        integration.uninstall().await.unwrap();
        assert!(matches!(
            integration.is_installed().await,
            Err(Error::FileDoesNotExist(_))
        ));

        // write bad data to integration file
        fs::write(&integration.path, "bad data").await.unwrap();
        assert!(matches!(
            integration.is_installed().await,
            Err(Error::ImproperInstallation(_))
        ));

        // fix integration file
        integration.install().await.unwrap();
        assert!(integration.is_installed().await.is_ok());
    }

    fn mode_of(path: &Path) -> u32 {
        std::fs::metadata(path).unwrap().mode() & 0o7777
    }

    #[tokio::test]
    async fn test_mode_applies_to_existing_file() {
        let tempdir = tempfile::tempdir().unwrap();
        let integration = FileIntegration {
            path: tempdir.path().join("integration.txt"),
            contents: "test".into(),
            mode: Some(0o600),
        };

        // Correct contents, wrong mode
        std::fs::write(&integration.path, "test").unwrap();
        std::fs::set_permissions(&integration.path, Permissions::from_mode(0o644)).unwrap();
        integration.install().await.unwrap();
        assert_eq!(mode_of(&integration.path), 0o600);

        // Wrong contents, wrong mode
        std::fs::write(&integration.path, "bad").unwrap();
        std::fs::set_permissions(&integration.path, Permissions::from_mode(0o644)).unwrap();
        integration.install().await.unwrap();
        assert_eq!(mode_of(&integration.path), 0o600);
        assert_eq!(std::fs::read_to_string(&integration.path).unwrap(), "test");
    }

    #[test]
    fn test_write_atomic() {
        let tempdir = tempfile::tempdir().unwrap();
        let path = tempdir.path().join("file");

        // New file
        write_atomic(&path, b"one", None).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "one");
        assert_eq!(mode_of(&path), 0o600);

        // Existing file keeps its mode
        std::fs::set_permissions(&path, Permissions::from_mode(0o640)).unwrap();
        write_atomic(&path, b"two", None).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "two");
        assert_eq!(mode_of(&path), 0o640);

        // An explicit mode wins
        write_atomic(&path, b"three", Some(0o600)).unwrap();
        assert_eq!(mode_of(&path), 0o600);

        // No temp files are left behind
        assert_eq!(std::fs::read_dir(tempdir.path()).unwrap().count(), 1);
    }

    #[test]
    fn test_write_atomic_keeps_symlink() {
        let tempdir = tempfile::tempdir().unwrap();
        let target_dir = tempdir.path().join("dotfiles");
        std::fs::create_dir(&target_dir).unwrap();
        let target = target_dir.join("zshrc");
        std::fs::write(&target, "old").unwrap();
        let link = tempdir.path().join(".zshrc");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        write_atomic(&link, b"new", None).unwrap();

        assert!(std::fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "new");
        assert_eq!(std::fs::read_dir(&target_dir).unwrap().count(), 1);

        // A dangling symlink is an error and stays as it is
        std::fs::remove_file(&target).unwrap();
        assert!(write_atomic(&link, b"new", None).is_err());
        assert!(std::fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
    }
}
