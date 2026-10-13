use std::process::ExitCode;

use anstream::println;
use clap::Subcommand;
use crossterm::style::Stylize;
use eyre::Result;
use serde_json::json;
use tab_integrations::Integration as _;
use tab_integrations::shell::ShellExt;
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

#[allow(unused_mut)]
async fn install(integration: Integration, silent: bool) -> Result<()> {
    let mut installed = false;
    let mut errored = false;
    let mut status: Option<&str> = None;

    let result = match integration {
        Integration::All => Ok(()),
        Integration::Dotfiles { shell } => {
            let shells = match shell {
                Some(shell) => vec![shell],
                None => vec![Shell::Bash, Shell::Zsh, Shell::Fish],
            };

            let mut errs: Vec<String> = vec![];
            for shell in &shells {
                match shell.get_shell_integrations(&Env::new()) {
                    Ok(integrations) => {
                        for integration in integrations {
                            match integration.is_installed().await {
                                Ok(_) => {
                                    debug!("Skipping {}", integration.describe());
                                },
                                Err(_) => {
                                    installed = true;
                                    if let Err(e) = integration.install().await {
                                        errs.push(format!(
                                            "{}: {}",
                                            integration.describe().bold(),
                                            e.verbose_message()
                                        ));
                                    }
                                },
                            }
                        }
                    },
                    Err(e) => {
                        errs.push(format!("{shell}: {e}"));
                    },
                }
            }

            if errs.is_empty() {
                Ok(())
            } else {
                Err(eyre::eyre!("\n\n{}", errs.join("\n\n")))
            }
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
            let shells = match shell {
                Some(shell) => vec![shell],
                None => vec![Shell::Bash, Shell::Zsh, Shell::Fish],
            };

            let mut errs: Vec<String> = vec![];
            for shell in &shells {
                match shell.get_shell_integrations(&Env::new()) {
                    Ok(integrations) => {
                        for integration in integrations {
                            match integration.is_installed().await {
                                Ok(_) => {
                                    uninstalled = true;
                                    if let Err(e) = integration.uninstall().await {
                                        errs.push(format!(
                                            "{}: {}",
                                            integration.describe().bold(),
                                            e.verbose_message()
                                        ));
                                    }
                                },
                                Err(_) => {
                                    debug!("Skipping {}", integration.describe());
                                },
                            }
                        }
                    },
                    Err(e) => {
                        errs.push(format!("{shell}: {e}"));
                    },
                }
            }

            if errs.is_empty() {
                Ok(())
            } else {
                Err(eyre::eyre!("\n\n{}", errs.join("\n\n")))
            }
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

            for shell in &[Shell::Bash, Shell::Zsh, Shell::Fish] {
                match shell.get_shell_integrations(&Env::new()) {
                    Ok(integrations) => {
                        for integration in integrations {
                            all_integrations.push((
                                integration.is_installed().await.is_ok(),
                                integration.describe(),
                                integration.get_shell(),
                                integration.file_name().to_owned(),
                            ));
                        }
                    },
                    Err(e) => {
                        errors.push((shell.to_string(), e.verbose_message()));
                    },
                }
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
