use std::process::ExitCode;

use anstream::println;
use crossterm::style::Stylize;
use eyre::Result;
use fig_os_shim::Context;
use fig_util::{
    CLI_BINARY_NAME,
    PRODUCT_NAME,
};

use crate::util::dialoguer_theme;

pub async fn uninstall_command(no_confirm: bool) -> Result<ExitCode> {
    if !no_confirm {
        println!(
            "\nIs {PRODUCT_NAME} not working? Try running {}\n",
            format!("{CLI_BINARY_NAME} doctor").bold().magenta()
        );
        let should_continue = dialoguer::Select::with_theme(&dialoguer_theme())
            .with_prompt(format!("Are you sure want to continue uninstalling {PRODUCT_NAME}?"))
            .items(&["Yes", "No"])
            .default(0)
            .interact_opt()?;

        if should_continue == Some(0) {
            println!("Uninstalling {PRODUCT_NAME}");
        } else {
            println!("Cancelled");
            return Ok(ExitCode::FAILURE);
        }
    };

    let ctx = Context::new();
    uninstall(ctx).await?;

    Ok(ExitCode::SUCCESS)
}

async fn uninstall(ctx: std::sync::Arc<fig_os_shim::Context>) -> Result<()> {
    fig_install::uninstall(fig_install::InstallComponents::all(), ctx).await?;

    Ok(())
}

// #[cfg(target_os = "linux")]
// mod linux {
//     use eyre::Result;
//
//     pub async fn uninstall_apt(pkg: String) -> Result<()> {
//         tokio::process::Command::new("apt")
//             .arg("remove")
//             .arg("-y")
//             .arg(pkg)
//             .status()
//             .await?;
//         std::fs::remove_file("/etc/apt/sources.list.d/fig.list")?;
//         std::fs::remove_file("/etc/apt/keyrings/fig.gpg")?;
//
//         Ok(())
//     }
//
//     pub async fn uninstall_dnf(pkg: String) -> Result<()> {
//         tokio::process::Command::new("dnf")
//             .arg("remove")
//             .arg("-y")
//             .arg(pkg)
//             .status()
//             .await?;
//         std::fs::remove_file("/etc/yum.repos.d/fig.repo")?;
//
//         Ok(())
//     }
//
//     pub async fn uninstall_pacman(pkg: String) -> Result<()> {
//         tokio::process::Command::new("pacman")
//             .arg("-Rs")
//             .arg(pkg)
//             .status()
//             .await?;
//
//         Ok(())
//     }
// }
