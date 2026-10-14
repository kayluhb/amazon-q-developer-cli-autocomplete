use std::path::PathBuf;
use std::sync::Arc;

use tab_integrations::Integration;
use tab_integrations::shell::all_shell_integrations;
use tab_integrations::ssh::SshIntegration;
use tab_os_shim::{
    Context,
    Env,
};
use tab_util::{
    CLI_BINARY_NAME,
    PTY_BINARY_NAME,
    Shell,
    directories,
};

use crate::Error;

bitflags::bitflags! {
    /// The different components that can be installed.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct InstallComponents: u64 {
        /// Removal of the integrations from user's dotfiles
        const SHELL_INTEGRATIONS    = 0b00000001;
        /// This handles the removal of the CLI and pty binaries as well as legacy copies
        const BINARY                = 0b00000010;
        /// Removal of the ssh integration from the ~/.ssh/config file
        const SSH                   = 0b00000100;
        const DESKTOP_APP           = 0b00001000;
        const INPUT_METHOD          = 0b00010000;
        const DESKTOP_ENTRY         = 0b00100000;
        const GNOME_SHELL_EXTENSION = 0b01000000;
    }
}

pub async fn uninstall(components: InstallComponents, ctx: Arc<Context>) -> Result<(), Error> {
    let ssh_result = if components.contains(InstallComponents::SSH) {
        SshIntegration::new()?.uninstall().await
    } else {
        Ok(())
    };

    let shell_integration_result: Result<(), Error> = {
        let (integrations, errors) = all_shell_integrations(ctx.env());
        for integration in integrations {
            integration.uninstall().await?;
        }
        match errors.into_iter().next() {
            Some((_, err)) => Err(err.into()),
            None => Ok(()),
        }
    };

    if components.contains(InstallComponents::BINARY) {
        let remove_binary = |path: PathBuf| async move {
            match tokio::fs::remove_file(&path).await {
                Ok(_) => tracing::info!("Removed binary: {path:?}"),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {},
                Err(err) => tracing::warn!(%err, "Failed to remove binary: {path:?}"),
            }
        };

        // let folders = [directories::home_local_bin()?, Path::new("/usr/local/bin").into()];
        let folders = [directories::home_local_bin()?];

        let all_binary_names = [CLI_BINARY_NAME, PTY_BINARY_NAME];

        for folder in folders {
            for binary_name in &all_binary_names {
                let binary_path = folder.join(binary_name);
                remove_binary(binary_path).await;
            }

            for shell in Shell::all() {
                let pty_path = folder.join(format!("{shell} ({PTY_BINARY_NAME})"));
                remove_binary(pty_path).await;
            }
        }
    }

    let daemon_result = Ok(());

    if components.contains(InstallComponents::INPUT_METHOD) {
        use tab_integrations::Error;
        use tab_integrations::input_method::{
            InputMethod,
            InputMethodError,
        };

        match InputMethod::default().uninstall().await {
            Ok(_) | Err(Error::InputMethod(InputMethodError::CouldNotListInputSources)) => {},
            Err(err) => return Err(err.into()),
        }
    }

    if components.contains(InstallComponents::DESKTOP_APP) {
        super::os::uninstall_desktop(&ctx).await?;
        // Must be last -- this will kill the running desktop process if this is
        // called from the desktop app.
        let quit_res = tokio::process::Command::new("killall")
            .args([tab_util::consts::APP_PROCESS_NAME])
            .output()
            .await;
        if let Err(err) = quit_res {
            tracing::warn!("Failed to quit running Fig app: {err}");
        }
    }

    daemon_result
        .and(shell_integration_result)
        .and(ssh_result.map_err(|e| e.into()))
}

pub async fn install(components: InstallComponents, env: &Env) -> Result<(), Error> {
    if components.contains(InstallComponents::SHELL_INTEGRATIONS) {
        let (integrations, errors) = all_shell_integrations(env);
        let mut errs: Vec<Error> = errors.into_iter().map(|(_, e)| e.into()).collect();
        for integration in integrations {
            if let Err(e) = integration.install().await {
                errs.push(e.into());
            }
        }

        if let Some(err) = errs.pop() {
            return Err(err);
        }
    }

    if components.contains(InstallComponents::SSH) {
        SshIntegration::new()?.install().await?;
    }

    if components.contains(InstallComponents::INPUT_METHOD) {
        use tab_integrations::input_method::InputMethod;
        InputMethod::default().install().await?;
    }

    Ok(())
}
