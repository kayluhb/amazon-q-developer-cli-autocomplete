#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Failed to open URL")]
    Failed,
}

#[allow(unexpected_cfgs)]
fn open_macos(url_str: impl AsRef<str>) -> Result<(), Error> {
    use objc2::ClassType;
    use objc2_foundation::{
        NSString,
        NSURL,
    };

    let url_nsstring = NSString::from_str(url_str.as_ref());
    let nsurl = unsafe { NSURL::initWithString(NSURL::alloc(), &url_nsstring) }.ok_or(Error::Failed)?;
    let res = unsafe { objc2_app_kit::NSWorkspace::sharedWorkspace().openURL(&nsurl) };
    res.then_some(()).ok_or(Error::Failed)
}

/// Returns bool indicating whether the URL was opened successfully
pub fn open_url(url: impl AsRef<str>) -> Result<(), Error> {
    open_macos(url)
}

/// Returns bool indicating whether the URL was opened successfully
pub async fn open_url_async(url: impl AsRef<str>) -> Result<(), Error> {
    open_macos(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[ignore]
    #[test]
    fn test_open_url() {
        open_url("https://fig.io").unwrap();
    }
}
