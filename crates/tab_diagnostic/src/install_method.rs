use std::sync::LazyLock;

use serde::{
    Deserialize,
    Serialize,
};

static INSTALL_METHOD: LazyLock<InstallMethod> = LazyLock::new(|| {
    if let Ok(current_exe) = std::env::current_exe() {
        if current_exe.components().any(|c| c.as_os_str() == ".toolbox") {
            return InstallMethod::Toolbox;
        }
    }

    InstallMethod::Unknown
});

/// The method of installation that Tab was installed with
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstallMethod {
    Toolbox,
    Unknown,
}

impl std::fmt::Display for InstallMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            InstallMethod::Toolbox => "toolbox",
            InstallMethod::Unknown => "unknown",
        })
    }
}

pub fn get_install_method() -> InstallMethod {
    *INSTALL_METHOD
}
