use std::process::ExitCode;

use anstream::println;
use clap::Subcommand;
use crossterm::style::Stylize;
use eyre::Result;
use serde_json::json;
use tab_integrations::Integration as _;
use tab_integrations::shell::{
    all_shell_integrations,
    shell_integrations,
};
use tab_integrations::ssh::SshIntegration;
use tab_os_shim::Env;
use tab_util::Shell;
use tracing::debug;

use super::OutputFormat;

#[derive(Debug, PartialEq, Eq, Subcommand)]
pub enum IntegrationsSubcommands {
    Install {
        /// Integration to install
        #[command(subcommand)]
        integration: Integration,
        /// Suppress status messages
        #[arg(long, short)]
        silent: bool,
    },
    Uninstall {
        /// Integration to uninstall
        #[command(subcommand)]
        integration: Integration,
        /// Suppress status messages
        #[arg(long, short)]
        silent: bool,
    },
    Reinstall {
        /// Integration to reinstall
        #[command(subcommand)]
        integration: Integration,
        /// Suppress status messages
        #[arg(long, short)]
        silent: bool,
    },
    Status {
        /// Integration to check status of
        #[command(subcommand)]
        integration: Integration,
        #[arg(long, short, value_enum, default_value_t)]
        format: OutputFormat,
    },
}

#[derive(Debug, Subcommand, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Integration {
    Dotfiles {
        /// Only install the integrations for a single shell
        #[arg(value_enum)]
        shell: Option<Shell>,
    },
    Ssh,
    InputMethod,
    #[command(name = "vscode")]
    VSCode,
    #[doc(hidden)]
    All,
}

impl IntegrationsSubcommands {
    pub async fn execute(self) -> Result<ExitCode> {
        match self {
            IntegrationsSubcommands::Install { integration, silent } => {
                if let Integration::All = integration {
                    install(Integration::Dotfiles { shell: None }, silent).await?;
                    install(Integration::Ssh, silent).await?;
                    install(Integration::InputMethod, silent).await?;
                } else {
                    install(integration, silent).await?;
                }
                Ok(ExitCode::SUCCESS)
            },
            IntegrationsSubcommands::Uninstall { integration, silent } => {
                if let Integration::All = integration {
                    uninstall(Integration::Dotfiles { shell: None }, silent).await?;
                    uninstall(Integration::Ssh, silent).await?;
                    uninstall(Integration::InputMethod, silent).await?;
                } else {
                    uninstall(integration, silent).await?;
                }
                Ok(ExitCode::SUCCESS)
            },
            IntegrationsSubcommands::Status { integration, format } => status(integration, format).await,
            IntegrationsSubcommands::Reinstall { integration, silent } => {
                if let Integration::All = integration {
                    uninstall(Integration::Dotfiles { shell: None }, silent).await?;
                    uninstall(Integration::Ssh, silent).await?;
                    uninstall(Integration::InputMethod, silent).await?;
                    install(Integration::Dotfiles { shell: None }, silent).await?;
                    install(Integration::Ssh, silent).await?;
                    install(Integration::InputMethod, silent).await?;
                } else {
                    uninstall(integration, silent).await?;
                    install(integration, silent).await?;
                }
                Ok(ExitCode::SUCCESS)
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DotfileAction {
    Install,
    Uninstall,
}

/// Installs or uninstalls the dotfile integrations for `shell`, or for every shell if `shell` is
/// `None`. Returns `true` if it tried to install or uninstall any integration, even if that
/// attempt failed.
async fn apply_dotfiles(shell: Option<Shell>, action: DotfileAction) -> (bool, Result<()>) {
    let env = Env::new();
    let (integrations, shell_errors) = match shell {
        Some(shell) => shell_integrations(&[shell], &env),
        None => all_shell_integrations(&env),
    };

    let mut changed = false;
    let mut errs: Vec<String> = vec![];
    for integration in integrations {
        let is_installed = integration.is_installed().await.is_ok();
        let result = match (action, is_installed) {
            (DotfileAction::Install, false) => integration.install().await,
            (DotfileAction::Uninstall, true) => integration.uninstall().await,
            _ => {
                debug!("Skipping {}", integration.describe());
                continue;
            },
        };
        changed = true;
        if let Err(e) = result {
            errs.push(format!("{}: {}", integration.describe().bold(), e.verbose_message()));
        }
    }
    errs.extend(shell_errors.into_iter().map(|(shell, e)| format!("{shell}: {e}")));

    let result = if errs.is_empty() {
        Ok(())
    } else {
        Err(eyre::eyre!("\n\n{}", errs.join("\n\n")))
    };
    (changed, result)
}

#[allow(unused_mut)]
async fn install(integration: Integration, silent: bool) -> Result<()> {
    let mut installed = false;
    let mut errored = false;
    let mut status: Option<&str> = None;

    let result = match integration {
        Integration::All => Ok(()),
        Integration::Dotfiles { shell } => {
            let (changed, result) = apply_dotfiles(shell, DotfileAction::Install).await;
            installed = changed;
            result
        },
        Integration::Ssh => {
            let ssh_integration = SshIntegration::new()?;
            if ssh_integration.is_installed().await.is_err() {
                installed = true;
                ssh_integration.install().await.map_err(eyre::Report::from)
            } else {
                Ok(())
            }
        },
        Integration::InputMethod => {
            tab_settings::state::set_value("input-method.enabled", true).ok();
            tab_integrations::input_method::InputMethod::default().install().await?;
            installed = true;
            status = Some("You must restart your terminal to finish installing the input method.");
            Ok(())
        },
        Integration::VSCode => {
            let variants = tab_integrations::vscode::variants_installed();
            installed = !variants.is_empty();
            for variant in variants {
                tab_integrations::vscode::VSCodeIntegration { variant }
                    .install()
                    .await?;
            }
            Ok(())
        },
    };

    if installed && result.is_ok() && !silent {
        println!("Installed!");

        if let Some(status) = status {
            println!("{status}");
        }
    }

    if !errored && !installed && !silent {
        println!("Already installed");
    }

    result
}

async fn uninstall(integration: Integration, silent: bool) -> Result<()> {
    let mut uninstalled = false;

    let result = match integration {
        Integration::All => Ok(()),
        Integration::Dotfiles { shell } => {
            let (changed, result) = apply_dotfiles(shell, DotfileAction::Uninstall).await;
            uninstalled = changed;
            result
        },
        Integration::Ssh => {
            let ssh_integration = SshIntegration::new()?;
            if ssh_integration.is_installed().await.is_ok() {
                uninstalled = true;
                ssh_integration.uninstall().await.map_err(eyre::Report::from)
            } else {
                Ok(())
            }
        },
        Integration::InputMethod => {
            tab_integrations::input_method::InputMethod::default()
                .uninstall()
                .await?;
            uninstalled = true;
            Ok(())
        },
        Integration::VSCode => {
            for variant in tab_integrations::vscode::variants_installed() {
                let integration = tab_integrations::vscode::VSCodeIntegration { variant };
                if integration.is_installed().await.is_ok() {
                    integration.uninstall().await?;
                    uninstalled = true;
                }
            }
            println!("Warning: VSCode integrations are automatically reinstalled on launch");
            Ok(())
        },
    };

    if uninstalled && result.is_ok() && !silent {
        println!("Uninstalled!");
    }

    if !uninstalled && !silent {
        println!("Not installed");
    }

    result
}

async fn status(integration: Integration, format: OutputFormat) -> Result<ExitCode> {
    match integration {
        Integration::All => Err(eyre::eyre!(
            "Checking the status for all integrations is currently not supported"
        )),
        Integration::Ssh => {
            let ssh_integration = SshIntegration::new()?;
            let installed = ssh_integration.is_installed().await.is_ok();
            format.print(
                || if installed { "Installed" } else { "Not installed" },
                || {
                    json!({
                        "installed": installed,
                    })
                },
            );
            Ok(ExitCode::SUCCESS)
        },
        Integration::Dotfiles { .. } => {
            let mut all_integrations = vec![];
            let mut errors = vec![];

            let (integrations, shell_errors) = all_shell_integrations(&Env::new());
            for integration in integrations {
                all_integrations.push((
                    integration.is_installed().await.is_ok(),
                    integration.describe(),
                    integration.get_shell(),
                    integration.file_name().to_owned(),
                ));
            }
            for (shell, e) in shell_errors {
                errors.push((shell.to_string(), e.verbose_message()));
            }

            format.print(
                || {
                    let mut s = String::new();
                    for (installed, describe, _, _) in &all_integrations {
                        s.push_str(&if *installed {
                            "✔ ".green().to_string()
                        } else {
                            "✘ ".red().to_string()
                        });
                        s.push_str(describe);
                        s.push('\n');
                    }

                    for (shell, error) in &errors {
                        s.push_str(&format!("{shell}: {error}\n"));
                    }

                    s
                },
                || {
                    let integrations = all_integrations
                        .iter()
                        .map(|(installed, describe, shell, file_name)| {
                            json!({
                                "installed": installed,
                                "description": describe,
                                "shell": shell,
                                "file_name": file_name,
                            })
                        })
                        .collect::<Vec<_>>();

                    let errors = errors
                        .iter()
                        .map(|(shell, error)| {
                            json!({
                                "shell": shell,
                                "error": error,
                            })
                        })
                        .collect::<Vec<_>>();

                    json!({
                        "integrations": integrations,
                        "errors": errors,
                    })
                },
            );

            Ok(ExitCode::SUCCESS)
        },
        Integration::InputMethod => {
            let input_method = tab_integrations::input_method::InputMethod::default();
            let installed = input_method.is_installed().await.is_ok();
            format.print(
                || if installed { "Installed" } else { "Not installed" },
                || {
                    json!({
                        "installed": installed,
                    })
                },
            );
            Ok(ExitCode::SUCCESS)
        },
        Integration::VSCode => {
            let variants = tab_integrations::vscode::variants_installed();
            for variant in variants {
                let integration = tab_integrations::vscode::VSCodeIntegration { variant };
                match integration.is_installed().await {
                    Ok(_) => {
                        println!("{}: Installed", integration.variant.application_name);
                    },
                    Err(_) => {
                        println!("{}: Not installed", integration.variant.application_name);
                    },
                }
            }
            Ok(ExitCode::SUCCESS)
        },
    }
}
