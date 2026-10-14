use std::path::{
    Path,
    PathBuf,
};
use std::{
    env,
    fs,
};

/// `build-scripts/build.py` packs the VS Code extension to this path before it builds the crates.
const VSIX_DIR: &str = "src/vscode";
const VSIX_NAME: &str = "vscode-plugin.vsix";

fn main() {
    // Watch the folder, not the file: a missing file would make cargo rerun this script on every build.
    println!("cargo:rerun-if-changed={VSIX_DIR}");

    let src = Path::new(VSIX_DIR).join(VSIX_NAME);
    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join(VSIX_NAME);

    // A cargo build without the npm step has no extension. Embed an empty file so the crate still
    // builds; the integration refuses to install it.
    if src.exists() {
        fs::copy(&src, &out).unwrap();
    } else {
        fs::write(&out, []).unwrap();
    }
}
