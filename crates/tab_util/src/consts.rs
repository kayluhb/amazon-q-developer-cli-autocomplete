pub const APP_BUNDLE_ID: &str = "com.kayluhb.tab";
pub const APP_BUNDLE_NAME: &str = "Tab.app";

pub const APP_PROCESS_NAME: &str = "tab_desktop";

/// The name configured under `"package.productName"` in the tauri.conf.json file.
pub const TAURI_PRODUCT_NAME: &str = "tab_desktop";

pub const CLI_BINARY_NAME: &str = "tab";
pub const CLI_BINARY_NAME_MINIMAL: &str = "tab-minimal";
pub const PTY_BINARY_NAME: &str = "tabterm";

pub const CLI_CRATE_NAME: &str = "tab_cli";

pub const URL_SCHEMA: &str = "tab";

pub const PRODUCT_NAME: &str = "Tab";

pub const RUNTIME_DIR_NAME: &str = "tabrun";

/// Data directory name used in paths like ~/.local/share/{DATA_DIR_NAME}
pub const DATA_DIR_NAME: &str = "tab";

/// Backup directory name
pub const BACKUP_DIR_NAME: &str = ".tab.dotfiles.bak";

pub const GITHUB_REPO_NAME: &str = "kayluhb/amazon-q-developer-cli-autocomplete";

pub mod url {
    pub const USER_MANUAL: &str = "https://docs.aws.amazon.com/amazonq/latest/qdeveloper-ug/command-line.html";
    pub const AUTOCOMPLETE_SSH_WIKI: &str =
        "https://docs.aws.amazon.com/amazonq/latest/qdeveloper-ug/command-line-autocomplete-ssh.html";
}

/// Build time env vars
pub mod build {
    /// The target of the current build, e.g. "aarch64-unknown-linux-musl"
    pub const TARGET_TRIPLE: Option<&str> = option_env!("TAB_BUILD_TARGET_TRIPLE");

    /// The variant of the current build
    pub const VARIANT: Option<&str> = option_env!("TAB_BUILD_VARIANT");

    /// A git full sha hash of the current build
    pub const HASH: Option<&str> = option_env!("TAB_BUILD_HASH");

    /// The datetime in rfc3339 format of the current build
    pub const DATETIME: Option<&str> = option_env!("TAB_BUILD_DATETIME");

    /// If `fish` tests should be skipped
    pub const SKIP_FISH_TESTS: bool = option_env!("TAB_BUILD_SKIP_FISH_TESTS").is_some();

    /// If `shellcheck` tests should be skipped
    pub const SKIP_SHELLCHECK_TESTS: bool = option_env!("TAB_BUILD_SKIP_SHELLCHECK_TESTS").is_some();
}

/// macOS specific constants
pub mod macos {
    pub const BUNDLE_CONTENTS_MACOS_PATH: &str = "Contents/MacOS";
    pub const BUNDLE_CONTENTS_RESOURCE_PATH: &str = "Contents/Resources";
    pub const BUNDLE_CONTENTS_HELPERS_PATH: &str = "Contents/Helpers";
    pub const BUNDLE_CONTENTS_INFO_PLIST_PATH: &str = "Contents/Info.plist";
}

pub mod env_var {
    macro_rules! define_env_vars {
        ($($(#[$meta:meta])* $ident:ident = $name:expr),*) => {
            $(
                $(#[$meta])*
                pub const $ident: &str = $name;
            )*

            pub const ALL: &[&str] = &[$($ident),*];
        }
    }

    define_env_vars! {
        /// The UUID of the current parent qterm instance
        TABTERM_SESSION_ID = "TABTERM_SESSION_ID",

        /// The current parent socket to connect to
        TAB_PARENT = "TAB_PARENT",

        /// Set the [`TAB_PARENT`] parent socket to connect to
        TAB_SET_PARENT = "TAB_SET_PARENT",

        /// Guard for the [`TAB_SET_PARENT`] check
        TAB_SET_PARENT_CHECK = "TAB_SET_PARENT_CHECK",

        /// Set if qterm is running, contains the version
        TAB_TERM = "TAB_TERM",

        /// Sets the current log level
        TAB_LOG_LEVEL = "TAB_LOG_LEVEL",

        /// Overrides the ZDOTDIR environment variable
        TAB_ZDOTDIR = "TAB_ZDOTDIR",

        /// Indicates a process was launched by Tab
        PROCESS_LAUNCHED_BY_TAB = "PROCESS_LAUNCHED_BY_TAB",

        /// The shell to use in qterm
        TAB_SHELL = "TAB_SHELL",

        /// Indicates the user is debugging the shell
        TAB_DEBUG_SHELL = "TAB_DEBUG_SHELL",

        /// Indicates the user is using zsh autosuggestions which disables Inline
        TAB_USING_ZSH_AUTOSUGGESTIONS = "TAB_USING_ZSH_AUTOSUGGESTIONS",

        /// Overrides the path to the bundle metadata released with certain desktop builds.
        TAB_BUNDLE_METADATA_PATH = "TAB_BUNDLE_METADATA_PATH"
    }
}

pub mod system_paths {
    /// System installation paths
    pub const APPLICATIONS_DIR: &str = "/Applications";
    pub const USR_LOCAL_BIN: &str = "/usr/local/bin";
    pub const USR_SHARE: &str = "/usr/share";
    pub const OPT_HOMEBREW_BIN: &str = "/opt/homebrew/bin";
}

#[cfg(test)]
mod tests {
    use time::OffsetDateTime;
    use time::format_description::well_known::Rfc3339;

    use super::*;

    #[test]
    fn test_build_envs() {
        if let Some(build_variant) = build::VARIANT {
            println!("build_variant: {build_variant}");
            assert!(["full", "minimal"].contains(&&*build_variant.to_ascii_lowercase()));
        }

        if let Some(build_hash) = build::HASH {
            println!("build_hash: {build_hash}");
            assert!(!build_hash.is_empty());
        }

        if let Some(build_datetime) = build::DATETIME {
            println!("build_datetime: {build_datetime}");
            println!("{}", OffsetDateTime::parse(build_datetime, &Rfc3339).unwrap());
        }
    }
}
