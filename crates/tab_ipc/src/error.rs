use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Connect(#[from] ConnectError),
    #[error(transparent)]
    Send(#[from] SendError),
    #[error(transparent)]
    Recv(#[from] RecvError),
    #[error("timeout")]
    Timeout,
    #[error(transparent)]
    Dir(#[from] tab_util::directories::DirectoryError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Nix(#[from] nix::Error),
}

#[derive(Debug, Error)]
pub enum ConnectError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("timeout connecting to socket")]
    Timeout,
    #[error("invalid permissions on socket dir")]
    IncorrectSocketPermissions,
}

#[derive(Debug, Error)]
pub enum SendError {
    #[error(transparent)]
    Encode(#[from] tab_proto::FigMessageEncodeError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum RecvError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Parse(#[from] tab_proto::FigMessageParseError),
    #[error(transparent)]
    Decode(#[from] tab_proto::FigMessageDecodeError),
    #[error("invalid message type")]
    InvalidMessageType,
}

impl RecvError {
    pub fn is_disconnect(&self) -> bool {
        if let RecvError::Io(io) = self {
            matches!(io.kind(), std::io::ErrorKind::ConnectionAborted)
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_disconnect() {
        assert!(!RecvError::Decode(tab_proto::FigMessageDecodeError::NameNotValid("test".to_string())).is_disconnect());
        assert!(RecvError::Io(std::io::Error::new(std::io::ErrorKind::ConnectionAborted, "error")).is_disconnect());
        assert!(!RecvError::Io(std::io::Error::new(std::io::ErrorKind::WouldBlock, "error")).is_disconnect());
    }
}
