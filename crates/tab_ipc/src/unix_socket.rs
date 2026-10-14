use std::path::Path;
use std::time::Duration;

use tokio::net::{
    UnixListener,
    UnixStream,
};
use tracing::{
    debug,
    error,
    trace,
    warn,
};

use crate::{
    BufferedReader,
    ConnectError,
};

struct OctalU32(u32);

impl std::fmt::Display for OctalU32 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:o}", self.0)
    }
}

pub async fn validate_socket(socket: impl AsRef<Path>) -> Result<(), ConnectError> {
    use std::os::unix::fs::PermissionsExt;

    let socket = socket.as_ref();

    match tokio::fs::metadata(socket).await {
        Ok(metadata) => {
            let mode = metadata.permissions().mode();
            if validate_mode_bits(mode, 0o600) {
                debug!(?socket, mode =% OctalU32(mode), "Socket permissions are 0o600");
                return Ok(());
            }
            warn!(?socket, mode =% OctalU32(mode), "Socket permissions are not 0o600");
        },
        Err(err) => {
            warn!(%err, ?socket, "Failed to get socket metadata, checking parent folder permissions");
        },
    }

    if let Some(parent_path) = socket.parent() {
        let metadata = tokio::fs::metadata(parent_path).await?;
        let mode = metadata.permissions().mode();
        if validate_mode_bits(mode, 0o700) {
            debug!(?socket, mode =% OctalU32(mode), "Socket folder permissions are 0o700");
            return Ok(());
        }
        warn!(?socket, mode =% OctalU32(mode), "Socket folder permissions are not 0o700");
    }

    error!(?socket, "Incorrect socket permissions, not connecting to socket");
    Err(ConnectError::IncorrectSocketPermissions)
}

/// Connects to a unix socket
pub async fn socket_connect(socket_path: impl AsRef<Path>) -> Result<UnixStream, ConnectError> {
    let socket_path = socket_path.as_ref();

    validate_socket(&socket_path).await?;

    let stream = match UnixStream::connect(&socket_path).await {
        Ok(stream) => stream,
        Err(err) => {
            error!(%err, ?socket_path, "Failed to connect");
            return Err(err.into());
        },
    };

    // Set lower permissions bits to 0o600
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = tokio::fs::metadata(&socket_path).await {
            let mut permissions = metadata.permissions();
            let new_mode = 0o600 | (permissions.mode() & !0o777);
            if permissions.mode() != new_mode {
                debug!(?socket_path, mode =% OctalU32(new_mode), "Setting socket permissions to 0o600");
                permissions.set_mode(new_mode);
                if let Err(err) = tokio::fs::set_permissions(&socket_path, permissions).await {
                    warn!(%err, ?socket_path, "Failed to set socket permissions");
                }
            }
        }
    }

    trace!(?socket_path, "Connected");

    Ok(stream)
}

/// Connects to a unix socket with a timeout
pub async fn socket_connect_timeout(socket: impl AsRef<Path>, timeout: Duration) -> Result<UnixStream, ConnectError> {
    let socket = socket.as_ref();
    match tokio::time::timeout(timeout, socket_connect(&socket)).await {
        Ok(Ok(conn)) => Ok(conn),
        Ok(Err(err)) => Err(err),
        Err(_) => {
            error!(?socket, ?timeout, "Timeout while connecting");
            Err(ConnectError::Timeout)
        },
    }
}

/// Binds a unix socket listener at `socket_path`.
///
/// Creates the parent folder if it does not exist and sets its permissions to 0o700, which
/// [validate_socket] expects. Removes an old socket file at `socket_path` before the bind.
pub async fn socket_bind(socket_path: impl AsRef<Path>) -> std::io::Result<UnixListener> {
    use std::fs::Permissions;
    use std::os::unix::fs::PermissionsExt;

    // Adds the failed step and its path to an io error, so the caller can tell the steps apart.
    fn step_error(step: &str, path: &Path) -> impl FnOnce(std::io::Error) -> std::io::Error {
        let message = format!("{step} {}", path.display());
        move |err| std::io::Error::new(err.kind(), format!("{message}: {err}"))
    }

    let socket_path = socket_path.as_ref();
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent).map_err(step_error("Failed creating socket folder", parent))?;
        std::fs::set_permissions(parent, Permissions::from_mode(0o700))
            .map_err(step_error("Failed setting permissions on socket folder", parent))?;
    }

    tokio::fs::remove_file(socket_path).await.ok();
    UnixListener::bind(socket_path).map_err(step_error("Failed binding socket", socket_path))
}

/// Checks all the lower bits of a permission with the mask 0o777
fn validate_mode_bits(left: u32, right: u32) -> bool {
    left & 0o777 == right & 0o777
}

pub type BufferedUnixStream = BufferedReader<UnixStream>;

impl BufferedUnixStream {
    /// Connect to a unix socket
    pub async fn connect(socket: impl AsRef<Path>) -> Result<Self, ConnectError> {
        Ok(Self::new(socket_connect(socket).await?))
    }

    /// Connect to a unix socket with a timeout
    pub async fn connect_timeout(socket: impl AsRef<Path>, timeout: Duration) -> Result<Self, ConnectError> {
        Ok(Self::new(socket_connect_timeout(socket, timeout).await?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_validate_socket() {
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("socket.sock");

        let socket_thread = tokio::spawn({
            let socket_path = socket_path.clone();
            async move {
                let listener = tokio::net::UnixListener::bind(&socket_path).unwrap();
                listener.accept().await.unwrap();
            }
        });

        assert!(socket_connect(&socket_path).await.is_err());

        {
            use std::fs::Permissions;
            use std::os::unix::fs::PermissionsExt;
            tokio::fs::set_permissions(&dir, Permissions::from_mode(0o700))
                .await
                .unwrap();
        }

        // Wait some time for the socket to start listening.
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;

        let _stream = socket_connect(&socket_path).await.expect("Failed to validate socket");

        socket_thread.abort();
    }

    #[tokio::test]
    async fn test_socket_bind() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("nested").join("socket.sock");

        let listener = socket_bind(&socket_path).await.unwrap();
        let mode = std::fs::metadata(socket_path.parent().unwrap())
            .unwrap()
            .permissions()
            .mode();
        assert!(validate_mode_bits(mode, 0o700));
        drop(listener);

        // A second bind replaces the old socket file.
        let _listener = socket_bind(&socket_path).await.unwrap();
        validate_socket(&socket_path).await.unwrap();
    }

    #[tokio::test]
    async fn test_socket_bind_restricts_existing_parent() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let parent = dir.path().join("open");
        std::fs::create_dir(&parent).unwrap();
        std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o755)).unwrap();

        let _listener = socket_bind(parent.join("socket.sock")).await.unwrap();
        let mode = std::fs::metadata(&parent).unwrap().permissions().mode();
        assert!(validate_mode_bits(mode, 0o700));
    }

    /// If this test fails, we need to reevaluate the permissions model design around our sockets
    /// and double check with security
    #[test]
    fn test_validate_mode_bits() {
        let valid = 0o700;

        for i in 0..0o700 {
            assert!(!validate_mode_bits(i, valid));
        }

        for i in 0o701..0o777 {
            assert!(!validate_mode_bits(i, valid));
        }
    }
}
