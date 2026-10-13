//! Making it a little more convenient and safe to query whether
//! something is a terminal teletype or not.
//! This module defines the IsTty trait and the is_tty method to
//! return true if the item represents a terminal.
use std::os::unix::io::AsRawFd;

/// Adds the is_tty method to types that might represent a terminal
pub trait IsTty {
    /// Returns true if the instance is a terminal teletype, false
    /// otherwise.
    fn is_tty(&self) -> bool;
}

/// On unix, the `isatty()` library function returns true if a file
/// descriptor is a terminal.  Let's implement `IsTty` for anything
/// that has an associated raw file descriptor.
impl<S: AsRawFd> IsTty for S {
    fn is_tty(&self) -> bool {
        let fd = self.as_raw_fd();
        nix::unistd::isatty(fd).unwrap_or_default()
    }
}

#[cfg(test)]
mod test {
    use std::io::stdout;

    use super::IsTty;

    #[test]
    fn test_is_tty() {
        stdout().is_tty();
    }
}
