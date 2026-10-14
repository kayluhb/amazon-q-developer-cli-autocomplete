use std::path::PathBuf;

use objc2_foundation::NSBundle;

// These match `tab_util::consts::macos`. tab_util depends on this crate, so this crate cannot use
// those constants.
const CONTENTS_MACOS_PATH: &str = "Contents/MacOS";
const CONTENTS_RESOURCES_PATH: &str = "Contents/Resources";

pub fn get_bundle_path() -> Option<PathBuf> {
    let main = NSBundle::mainBundle();
    let url = unsafe { main.bundleURL() };
    let path = unsafe { url.path() }?;
    Some(path.to_string().into())
}

pub fn get_bundle_path_for_executable(executable: &str) -> Option<PathBuf> {
    get_bundle_path().and_then(|path| {
        let full_path = path.join(CONTENTS_MACOS_PATH).join(executable);
        if full_path.exists() { Some(full_path) } else { None }
    })
}

pub fn get_bundle_resource_path() -> Option<PathBuf> {
    get_bundle_path().map(|path| path.join(CONTENTS_RESOURCES_PATH))
}
