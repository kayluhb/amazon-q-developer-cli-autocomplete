use std::fmt::Display;

use anstream::adapter::strip_str;
use tab_integrations::Integration;
use tab_integrations::shell::ShellExt;
use tab_integrations::ssh::SshIntegration;
use tab_os_shim::{
    ContextArcProvider,
    ContextProvider,
    EnvProvider,
};
use tab_proto::fig::install_response::{
    InstallationStatus,
    Response,
};
use tab_proto::fig::result::Result as ProtoResultEnum;
use tab_proto::fig::server_originated_message::Submessage as ServerOriginatedSubMessage;
use tab_proto::fig::{
    InstallAction,
    InstallComponent,
    InstallRequest,
    InstallResponse,
    Result as ProtoResult,
};
use tab_settings::settings::SettingsProvider;
use tab_settings::state::StateProvider;
use tab_util::Shell;

use super::RequestResult;

async fn integration_status(integration: impl tab_integrations::Integration) -> ServerOriginatedSubMessage {
    ServerOriginatedSubMessage::InstallResponse(InstallResponse {
        response: Some(Response::InstallationStatus(match integration.is_installed().await {
            Ok(_) => InstallationStatus::Installed.into(),
            Err(_) => InstallationStatus::NotInstalled.into(),
        })),
    })
}

fn integration_result(result: Result<(), impl Display>) -> ServerOriginatedSubMessage {
    ServerOriginatedSubMessage::InstallResponse(InstallResponse {
        response: Some(Response::Result(match result {
            Ok(()) => ProtoResult {
                result: ProtoResultEnum::Ok.into(),
                error: None,
            },
            Err(err) => ProtoResult {
                result: ProtoResultEnum::Error.into(),
                error: Some(err.to_string()),
            },
        })),
    })
}

pub async fn install<Ctx>(request: InstallRequest, ctx: &Ctx) -> RequestResult
where
    Ctx: SettingsProvider + StateProvider + ContextProvider + ContextArcProvider + Send + Sync,
{
    let response = match (request.component(), request.action()) {
        (InstallComponent::Dotfiles, action) => {
            let mut errs: Vec<String> = vec![];
            for shell in Shell::all() {
                match shell.get_shell_integrations(ctx.env()) {
                    Ok(integrations) => {
                        for integration in integrations {
                            let res = match action {
                                InstallAction::Install => integration.install().await,
                                InstallAction::Uninstall => integration.uninstall().await,
                                InstallAction::Status => integration.is_installed().await,
                            };

                            if let Err(err) = res {
                                errs.push(format!(
                                    "{integration}: {}",
                                    strip_str(&err.verbose_message().to_string())
                                ));
                            }
                        }
                    },
                    Err(err) => {
                        errs.push(format!("{shell}: {}", strip_str(&err.verbose_message().to_string())));
                    },
                }
            }

            match action {
                InstallAction::Install | InstallAction::Uninstall => integration_result(match &errs[..] {
                    [] => Ok(()),
                    errs => Err(errs.join("\n\n")),
                }),
                InstallAction::Status => ServerOriginatedSubMessage::InstallResponse(InstallResponse {
                    response: Some(Response::InstallationStatus(
                        if errs.is_empty() {
                            InstallationStatus::Installed
                        } else {
                            InstallationStatus::NotInstalled
                        }
                        .into(),
                    )),
                }),
            }
        },
        (InstallComponent::Ssh, action) => match SshIntegration::new() {
            Ok(ssh_integration) => match action {
                InstallAction::Install => integration_result(ssh_integration.install().await),
                InstallAction::Uninstall => integration_result(ssh_integration.uninstall().await),
                InstallAction::Status => integration_status(ssh_integration).await,
            },
            Err(err) => integration_result(Err(err)),
        },
        (InstallComponent::Ibus, _) => integration_result(Err("IBus install is legacy")),
        (InstallComponent::Accessibility, InstallAction::Install) => {
            use macos_utils::accessibility::{
                accessibility_is_enabled,
                open_accessibility,
            };

            if !accessibility_is_enabled() {
                open_accessibility();
            }

            integration_result(Ok::<(), &str>(()))
        },
        (InstallComponent::Accessibility, InstallAction::Status) => {
            use macos_utils::accessibility::accessibility_is_enabled;

            ServerOriginatedSubMessage::InstallResponse(InstallResponse {
                response: Some(Response::InstallationStatus(if accessibility_is_enabled() {
                    InstallationStatus::Installed.into()
                } else {
                    InstallationStatus::NotInstalled.into()
                })),
            })
        },
        (InstallComponent::Accessibility, InstallAction::Uninstall) => integration_result(Ok::<(), &str>(())),
        (InstallComponent::InputMethod, InstallAction::Install) => {
            use tab_integrations::Integration;
            use tab_integrations::input_method::InputMethod;

            integration_result(match InputMethod::default().install().await {
                Ok(_) => Ok(()),
                Err(err) => Err(format!("Could not install input method: {err}")),
            })
        },
        (InstallComponent::InputMethod, InstallAction::Uninstall) => {
            use tab_integrations::input_method::{
                InputMethod,
                InputMethodError,
            };
            use tab_integrations::{
                Error,
                Integration,
            };

            integration_result(match InputMethod::default().uninstall().await {
                Ok(_) | Err(Error::InputMethod(InputMethodError::CouldNotListInputSources)) => Ok(()),
                Err(err) => Err(format!("Could not uninstall input method: {err}")),
            })
        },
        (InstallComponent::InputMethod, InstallAction::Status) => {
            use tab_integrations::input_method::InputMethod;

            integration_status(InputMethod::default()).await
        },
        (InstallComponent::DesktopEntry | InstallComponent::AutostartEntry | InstallComponent::GnomeExtension, _) => {
            integration_result(Err("Not supported on macOS."))
        },
    };

    RequestResult::Ok(Box::new(response))
}
