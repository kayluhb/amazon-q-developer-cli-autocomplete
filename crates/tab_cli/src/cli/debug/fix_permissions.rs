use std::fs::OpenOptions;
use std::io::ErrorKind;
use std::os::unix::fs::{
    MetadataExt as _,
    OpenOptionsExt as _,
    PermissionsExt,
    fchown,
};
use std::path::Path;

use anstream::println;
use color_eyre::owo_colors::OwoColorize;
use eyre::{
    Context,
    ContextCompat,
    Result,
    bail,
};
use nix::libc::uid_t;
use nix::unistd::{
    Gid,
    Uid,
};
use tab_integrations::shell::ShellExt as _;
use tab_os_shim::Env;
use tab_util::directories::home_dir;
use tab_util::{
    CLI_BINARY_NAME,
    GITHUB_REPO_NAME,
    Shell,
};
use tracing::info;

pub fn fix_permissions(env: &Env) -> Result<()> {
    let Ok(sudo_uid_str) = std::env::var("SUDO_UID") else {
        bail!("This command must be run with sudo");
    };

    let sudo_uid: uid_t = sudo_uid_str.parse().context("Invalid SUDO_UID")?;
    let user = nix::unistd::User::from_uid(sudo_uid.into())
        .context("Failed to get user from SUDO_UID")?
        .context("Failed to find user from SUDO_UID")?;

    info!(?user, "Loaded user");

    let home_dir = home_dir()?;

    let user_uid = user.uid;
    let user_gid = user.gid;

    let mut updated = false;

    for shell in Shell::all() {
        let shell_dir = shell.get_config_directory(env)?;

        // Only fix if the shell dir is not the home dir
        if shell_dir.is_dir() && shell_dir != home_dir {
            for entry in walkdir::WalkDir::new(shell_dir) {
                let entry = entry?;
                updated |= fix_permissions_for_path(entry.path(), &user_uid, &user_gid)?;
            }
        }

        if let Ok(integrations) = shell.get_shell_integrations(env) {
            for integration in integrations {
                updated |= fix_permissions_for_path(&integration.path(), &user_uid, &user_gid)?;
            }
        }
    }

    println!(
        "\n{}\n\nIf you continue to experience issues:\n  1. Run {}\n  2. Report the issue at {}\n",
        if updated {
            "The permissions should be fixed now!".bold()
        } else {
            "The permissions have not been modified".bold()
        },
        format!("{CLI_BINARY_NAME} doctor").magenta(),
        format!("https://github.com/{GITHUB_REPO_NAME}/issues").magenta(),
    );

    Ok(())
}

/// Fixes the owner and mode of `path`. Never follows a symlink at `path`.
///
/// The owner and mode change through the open file descriptor, so a swap of `path` to a symlink
/// after the check has no effect.
fn fix_permissions_for_path(path: &Path, user_uid: &Uid, user_gid: &Gid) -> Result<bool> {
    // `symlink_metadata` describes the link itself, not its target.
    let link_metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(err.into()),
    };

    // Skip symlinks, sockets, fifos and devices
    if !link_metadata.is_file() && !link_metadata.is_dir() {
        return Ok(false);
    }

    let file = match OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK)
        .open(path)
    {
        Ok(file) => file,
        // `O_NOFOLLOW` gives `ELOOP` when `path` is now a symlink
        Err(err) if err.raw_os_error() == Some(nix::libc::ELOOP) => return Ok(false),
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(err.into()),
    };
    let metadata = file.metadata()?;

    info!(
        "{}: permission {:o}, uid: {}, gid: {}",
        path.display(),
        metadata.permissions().mode(),
        metadata.uid(),
        metadata.gid()
    );

    let mut updated = false;

    // ensure owner is correct
    if metadata.uid() != user_uid.as_raw() || metadata.gid() != user_gid.as_raw() {
        println!("Fixing owner for {}", path.display().bold());
        fchown(&file, Some(user_uid.as_raw()), Some(user_gid.as_raw()))?;
        updated = true;
    }

    if metadata.is_dir() && metadata.mode() & 0o700 != 0o700 {
        println!("Fixing permissions for {}", path.display().bold());
        let mut permissions = metadata.permissions();
        permissions.set_mode(permissions.mode() | 0o700);
        file.set_permissions(permissions)?;
        updated = true;
    }

    if metadata.is_file() && metadata.mode() & 0o600 != 0o600 {
        println!("Fixing permissions for {}", path.display().bold());
        let mut permissions = metadata.permissions();
        permissions.set_mode(permissions.mode() | 0o600);
        file.set_permissions(permissions)?;
        updated = true;
    }

    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skips_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target");
        std::fs::write(&target, "").unwrap();
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o400)).unwrap();
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        let uid = Uid::current();
        let gid = Gid::current();
        assert!(!fix_permissions_for_path(&link, &uid, &gid).unwrap());
        assert_eq!(std::fs::metadata(&target).unwrap().mode() & 0o777, 0o400);

        // A regular file is fixed
        assert!(fix_permissions_for_path(&target, &uid, &gid).unwrap());
        assert_eq!(std::fs::metadata(&target).unwrap().mode() & 0o777, 0o600);
    }

    #[test]
    fn test_missing_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing");
        assert!(!fix_permissions_for_path(&path, &Uid::current(), &Gid::current()).unwrap());
    }
}
