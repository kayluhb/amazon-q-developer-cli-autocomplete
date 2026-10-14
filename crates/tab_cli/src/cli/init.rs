use std::borrow::Cow;
use std::fmt::Display;
use std::io::{
    Write,
    stdout,
};
use std::path::Path;
use std::process::ExitCode;
use std::sync::LazyLock;

use clap::Args;
use crossterm::style::Stylize;
use eyre::Result;
use tab_integrations::shell::{
    ShellExt,
    When,
};
use tab_os_shim::Context;
use tab_util::env_var::TAB_SHELL;
use tab_util::{
    CLI_BINARY_NAME,
    PRODUCT_NAME,
    Shell,
    Terminal,
    get_parent_process_exe,
};

use super::internal::should_figterm_launch::should_figterm_launch_exit_status;
use crate::util::app_path_from_bundle_id;

const SHELL_INTEGRATIONS_ENABLED_STATE_KEY: &str = "shell-integrations.enabled";
const INLINE_SOURCE_SETTINGS_KEY: &str = "inline.source";
/// The `inline.source` value when the setting is not set
const INLINE_SOURCE_DEFAULT: &str = "history-specs";

static IS_SNAPSHOT_TEST: LazyLock<bool> = LazyLock::new(|| tab_os_shim::Env::new().q_init_snapshot_test());

#[derive(Debug, Args, PartialEq, Eq)]
pub struct InitArgs {
    /// The shell to generate the dotfiles for
    #[arg(value_enum)]
    shell: Shell,
    /// When to generate the dotfiles for
    #[arg(value_enum)]
    when: When,
    #[arg(long)]
    rcfile: Option<String>,
}

impl InitArgs {
    pub async fn execute(&self) -> Result<ExitCode> {
        let InitArgs { shell, when, rcfile } = self;
        match shell_init(shell, when, rcfile).await {
            Ok(source) => writeln!(stdout(), "{source}"),
            Err(err) => writeln!(stdout(), "# Could not load source: {err}"),
        }
        .ok();
        Ok(ExitCode::SUCCESS)
    }
}

#[derive(PartialEq, Eq)]
enum GuardAssignment {
    BeforeSourcing,
    AfterSourcing,
}

/// Quotes `value` as one literal string word for `shell`
fn shell_quote(shell: &Shell, value: &str) -> String {
    match shell {
        // Nothing is special inside single quotes, so `'` is closed, escaped and reopened
        Shell::Bash | Shell::Zsh => format!("'{}'", value.replace('\'', r"'\''")),
        // Inside fish single quotes only `\\` and `\'` are escapes
        Shell::Fish => format!("'{}'", value.replace('\\', r"\\").replace('\'', r"\'")),
        // Nu double quotes take backslash escapes and do not interpolate
        Shell::Nu => {
            let mut quoted = String::with_capacity(value.len() + 2);
            quoted.push('"');
            for character in value.chars() {
                match character {
                    '\\' => quoted.push_str(r"\\"),
                    '"' => quoted.push_str(r#"\""#),
                    '\n' => quoted.push_str(r"\n"),
                    '\r' => quoted.push_str(r"\r"),
                    '\t' => quoted.push_str(r"\t"),
                    character => quoted.push(character),
                }
            }
            quoted.push('"');
            quoted
        },
    }
}

fn assign_shell_variable(shell: &Shell, name: impl Display, value: impl Display, exported: bool) -> String {
    let value = shell_quote(shell, &value.to_string());
    match (shell, exported) {
        (Shell::Bash | Shell::Zsh, false) => format!("{name}={value}"),
        (Shell::Bash | Shell::Zsh, true) => format!("export {name}={value}"),
        (Shell::Fish, false) => format!("set -g {name} {value}"),
        (Shell::Fish, true) => format!("set -gx {name} {value}"),
        (Shell::Nu, _) => format!("let-env {name} = {value};"),
    }
}

fn guard_source(
    shell: &Shell,
    export: bool,
    guard_var: impl Display,
    assignment: GuardAssignment,
    source: impl Into<Cow<'static, str>>,
) -> String {
    let mut output: Vec<Cow<'static, str>> = Vec::with_capacity(4);
    output.push(match shell {
        Shell::Bash | Shell::Zsh => format!("if [ -z \"${{{guard_var}}}\" ]; then").into(),
        Shell::Fish => format!("if test -z \"${guard_var}\"").into(),
        Shell::Nu => format!("if env | any name == '{guard_var}' {{").into(),
    });

    let shell_var = assign_shell_variable(shell, guard_var, "1", export);
    match assignment {
        GuardAssignment::BeforeSourcing => {
            // If script may trigger rc file to be rerun, guard assignment must happen first to avoid recursion
            output.push(format!("  {shell_var}").into());
            for line in source.into().lines() {
                output.push(format!("  {line}").into());
            }
        },
        GuardAssignment::AfterSourcing => {
            for line in source.into().lines() {
                output.push(format!("  {line}").into());
            }
            output.push(format!("  {shell_var}").into());
        },
    }

    output.push(
        match shell {
            Shell::Bash | Shell::Zsh => "fi\n",
            Shell::Fish => "end\n",
            Shell::Nu => "}",
        }
        .into(),
    );

    output.join("\n")
}

async fn shell_init(shell: &Shell, when: &When, rcfile: &Option<String>) -> Result<String> {
    // Do not print any shell integrations for `.profile` as it can cause issues on launch
    if std::env::consts::OS == "linux" && matches!(rcfile.as_deref(), Some("profile")) {
        return Ok("".into());
    }

    if !tab_settings::state::get_bool_or(SHELL_INTEGRATIONS_ENABLED_STATE_KEY, true) {
        return Ok(shell_integrations_disabled_code(*shell));
    }

    let mut to_source = Vec::new();

    if let Some(parent_process) = get_parent_process_exe() {
        to_source.push(assign_shell_variable(
            shell,
            TAB_SHELL,
            if *IS_SNAPSHOT_TEST {
                Path::new("/bin/zsh").display()
            } else {
                parent_process.display()
            },
            false,
        ));
    };

    if when == &When::Pre {
        let status = if *IS_SNAPSHOT_TEST {
            0
        } else {
            should_figterm_launch_exit_status(&Context::new(), true)
        };
        to_source.push(assign_shell_variable(shell, "SHOULD_TABTERM_LAUNCH", status, false));
    }

    let is_jetbrains_terminal = Terminal::is_jetbrains_terminal();

    if when == &When::Pre && shell == &Shell::Bash && is_jetbrains_terminal {
        // JediTerm does not launch as a 'true' login shell, so our normal "shopt -q login_shell" check does
        // not work. Thus, TAB_IS_LOGIN_SHELL will be incorrect. We must manually set it so the
        // user's bash_profile is sourced. https://github.com/JetBrains/intellij-community/blob/master/plugins/terminal/resources/jediterm-bash.in
        to_source.push("TAB_IS_LOGIN_SHELL=1".into());
    }

    if let Some(path) = tab_settings::settings::get_string_opt("qterm.path") {
        to_source.push(assign_shell_variable(shell, "TAB_TERM_PATH", path, false));
    }

    let shell_integration_source = shell.get_fig_integration_source(when);
    to_source.push(shell_integration_source);

    if when == &When::Pre && is_jetbrains_terminal {
        // Manually call JetBrains shell integration after exec-ing to figterm.
        // This may recursively call out to bashrc/zshrc so make sure to assign guard variable first.

        let get_jetbrains_source = if let Some(bundle_id) = std::env::var_os("__CFBundleIdentifier") {
            if let Some(bundle) = app_path_from_bundle_id(bundle_id) {
                // The source for JetBrains shell integrations can be found here.
                // https://github.com/JetBrains/intellij-community/tree/master/plugins/terminal/resources

                // We source both the old and new location of these integrations.
                // In theory, they shouldn't both exist since they come with the app bundle itself.
                // As of writing, the bash path change isn't live, but we source it anyway.
                let source_if_exists = |relative_paths: [&str; 2]| {
                    relative_paths
                        .iter()
                        .map(|relative_path| {
                            let path =
                                shell_quote(shell, &format!("{bundle}/Contents/plugins/terminal/{relative_path}"));
                            format!("[ -f {path} ] && source {path}\n")
                        })
                        .collect::<String>()
                };
                match shell {
                    Shell::Bash => Some(source_if_exists(["jediterm-bash.in", "bash/jediterm-bash.in"])),
                    Shell::Zsh => Some(source_if_exists([".zshenv", "zsh/.zshenv"])),
                    Shell::Fish => Some(source_if_exists(["fish/config.fish", "fish/init.fish"])),
                    Shell::Nu => None,
                }
            } else {
                None
            }
        } else {
            None
        };

        if let Some(source) = get_jetbrains_source {
            to_source.push(guard_source(
                shell,
                false,
                "TAB_JETBRAINS_SHELL_INTEGRATION",
                GuardAssignment::BeforeSourcing,
                source,
            ));
        }
    }

    if should_source_inline_plugin(shell, when, rcfile.as_deref()) && !*IS_SNAPSHOT_TEST {
        if let Some(source) = inline_plugin_source(Some(&inline_source_setting())) {
            to_source.push(source);
        }
    }

    if when == &When::Post
        && !tab_integrations::input_method::InputMethod::default()
            .is_enabled()
            .unwrap_or(false)
    {
        if let Some(terminal) = Terminal::parent_terminal(&Context::new()) {
            let prompt_state_key = format!("prompt.input-method.{}.count", terminal.internal_id());
            let prompt_count = tab_settings::state::get_int_or(&prompt_state_key, 0);
            if terminal.supports_macos_input_method() && prompt_count < 2 {
                let _ = tab_settings::state::set_value(&prompt_state_key, prompt_count + 1);
                to_source.push(input_method_prompt_code(*shell, &terminal));
            }
        }
    }

    Ok(to_source.join("\n"))
}

/// The `inline.source` setting, or the default when it is not set
pub(super) fn inline_source_setting() -> String {
    tab_settings::settings::get_string_opt(INLINE_SOURCE_SETTINGS_KEY).unwrap_or_else(|| INLINE_SOURCE_DEFAULT.into())
}

/// The inline suggestions plugin is zsh-only and loads once, from the zshrc post block
fn should_source_inline_plugin(shell: &Shell, when: &When, rcfile: Option<&str>) -> bool {
    shell == &Shell::Zsh && when == &When::Post && rcfile != Some("zprofile")
}

/// The plugin's strategies for an `inline.source` setting, or `None` when it is off or invalid
pub(super) fn inline_strategy(inline_source: Option<&str>) -> Option<&'static str> {
    match inline_source? {
        "history" => Some("match_prev_cmd history"),
        "specs" => Some("specs"),
        "history-specs" => Some("match_prev_cmd history specs"),
        _ => None,
    }
}

/// The zsh code that loads the inline suggestions plugin for an `inline.source` setting, or `None`
/// when it is off
fn inline_plugin_source(inline_source: Option<&str>) -> Option<String> {
    let strategy = inline_strategy(inline_source)?;

    Some(guard_source(
        &Shell::Zsh,
        false,
        "TAB_INLINE_PLUGIN_SOURCED",
        GuardAssignment::AfterSourcing,
        format!(
            "TAB_AUTOSUGGEST_STRATEGY=({strategy})\n{}",
            tab_integrations::shell::inline_plugin::zsh_script()
        ),
    ))
}

fn shell_integrations_disabled_code(shell: Shell) -> String {
    guard_source(
        &shell,
        false,
        "TAB_SHELL_INTEGRATION_DISABLED",
        GuardAssignment::AfterSourcing,
        format!(
            "printf '{PRODUCT_NAME} shell integration is disabled.\\nRe-enable by running: {}\\n'",
            format!("{CLI_BINARY_NAME} _ local-state -d {SHELL_INTEGRATIONS_ENABLED_STATE_KEY}").magenta()
        ),
    )
}

#[allow(dead_code)]
fn input_method_prompt_code(shell: Shell, terminal: &Terminal) -> String {
    guard_source(
        &shell,
        false,
        "TAB_INPUT_METHOD_PROMPT",
        GuardAssignment::AfterSourcing,
        format!(
            "printf '\\n🚀 {PRODUCT_NAME} supports {terminal}!\\n\\nEnable integrations with {terminal} by \
             running:\\n  {}\\n\\n'\n",
            format!("{CLI_BINARY_NAME} integrations install input-method").magenta()
        ),
    )
}

#[cfg(test)]
mod tests {
    use std::process::{
        Command,
        Stdio,
    };

    use super::*;

    fn run_shell_stdout(shell: &Shell, text: &str) -> String {
        let mut child = Command::new(shell.as_str())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        let stdin = child.stdin.as_mut().unwrap();
        // Since these are all guarded we run the code twice to double check
        stdin.write_all(text.as_bytes()).unwrap();
        stdin.write_all(text.as_bytes()).unwrap();
        stdin.flush().unwrap();

        let output = child.wait_with_output().unwrap();
        String::from_utf8(output.stdout).unwrap()
    }

    #[test]
    fn test_shell_quote() {
        assert_eq!(shell_quote(&Shell::Zsh, "a b"), "'a b'");
        assert_eq!(shell_quote(&Shell::Bash, "it's"), r"'it'\''s'");
        assert_eq!(shell_quote(&Shell::Fish, r"it's \ x"), r"'it\'s \\ x'");
        assert_eq!(shell_quote(&Shell::Nu, "a\"b\\c\nd"), r#""a\"b\\c\nd""#);
    }

    #[test]
    fn test_assign_shell_variable_is_literal() {
        let values = [
            "plain",
            "/path with spaces/bin",
            "it's",
            "$(echo pwned)",
            "`echo pwned`",
            "\"; echo pwned; \"",
            "'; echo pwned; '",
            r"back\slash\",
            r"$HOME ${HOME} %s \' \\'",
            "new\nline",
        ];
        for shell in Shell::all_test() {
            for value in values {
                let code = format!(
                    "{}\nprintf '%s' \"$TAB_TEST_VAR\"\n",
                    assign_shell_variable(&shell, "TAB_TEST_VAR", value, false)
                );
                let output = Command::new(shell.as_str()).arg("-c").arg(&code).output().unwrap();
                assert_eq!(String::from_utf8(output.stdout).unwrap(), value, "{shell:?}: {code}");
            }
        }
    }

    #[test]
    fn test_inline_plugin_source() {
        for off in [None, Some("off"), Some(""), Some("bogus")] {
            assert_eq!(inline_plugin_source(off), None, "{off:?}");
        }

        for (source, strategy) in [
            ("history", "TAB_AUTOSUGGEST_STRATEGY=(match_prev_cmd history)\n"),
            ("specs", "TAB_AUTOSUGGEST_STRATEGY=(specs)\n"),
            (
                "history-specs",
                "TAB_AUTOSUGGEST_STRATEGY=(match_prev_cmd history specs)\n",
            ),
        ] {
            let code = inline_plugin_source(Some(source)).unwrap();
            assert!(code.starts_with("if [ -z \"${TAB_INLINE_PLUGIN_SOURCED}\" ]; then\n"));
            assert!(code.contains(strategy), "{source}");
        }
    }

    #[test]
    fn test_should_source_inline_plugin() {
        assert!(should_source_inline_plugin(&Shell::Zsh, &When::Post, Some("zshrc")));
        assert!(should_source_inline_plugin(&Shell::Zsh, &When::Post, None));
        assert!(!should_source_inline_plugin(&Shell::Zsh, &When::Post, Some("zprofile")));
        assert!(!should_source_inline_plugin(&Shell::Zsh, &When::Pre, Some("zshrc")));
        assert!(!should_source_inline_plugin(&Shell::Bash, &When::Post, Some("bashrc")));
    }

    #[test]
    fn test_prompts() {
        for shell in Shell::all_test() {
            let shell_integrations_disabled_output = run_shell_stdout(&shell, &shell_integrations_disabled_code(shell));

            println!("=== shell_integrations_disabled_code {shell:?} ===");
            println!("{shell_integrations_disabled_output}");
            println!("===");

            assert_eq!(
                shell_integrations_disabled_output,
                format!(
                    "{PRODUCT_NAME} shell integration is disabled.\nRe-enable by running: {}\n",
                    format!("{CLI_BINARY_NAME} _ local-state -d {SHELL_INTEGRATIONS_ENABLED_STATE_KEY}").magenta()
                )
            );

            let terminal = Terminal::Iterm;
            let input_method_prompt_output = run_shell_stdout(&shell, &input_method_prompt_code(shell, &terminal));

            println!("=== input_method_prompt {shell:?} ===");
            println!("{input_method_prompt_output}");
            println!("===");

            assert_eq!(
                input_method_prompt_output,
                format!(
                    "\n🚀 {PRODUCT_NAME} supports {terminal}!\n\nEnable integrations with {terminal} by running:\n  {}\n\n",
                    format!("{CLI_BINARY_NAME} integrations install input-method").magenta()
                )
            );
        }
    }
}
