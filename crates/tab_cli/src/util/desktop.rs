use std::process::Command;

use eyre::{
    Result,
    eyre,
};
use tab_util::{
    PRODUCT_NAME,
    directories,
    manifest,
    system_info,
};

pub struct LaunchArgs {
    /// Should we wait for the socket to continue execution
    pub wait_for_socket: bool,
    /// Should we open the dashboard right away
    ///
    /// Note that this won't open the dashboard if the app is already running
    pub open_dashboard: bool,
    /// Should we do the first update check or skip it
    pub immediate_update: bool,
    /// Print output to user
    pub verbose: bool,
}

pub fn desktop_app_running() -> bool {
    use std::ffi::OsString;

    use objc2_app_kit::NSRunningApplication;
    use objc2_foundation::ns_string;
    use sysinfo::{
        ProcessRefreshKind,
        RefreshKind,
        System,
    };
    use tab_util::consts::{
        APP_BUNDLE_ID,
        APP_PROCESS_NAME,
    };

    let bundle_id = ns_string!(APP_BUNDLE_ID);
    let running_applications = unsafe { NSRunningApplication::runningApplicationsWithBundleIdentifier(bundle_id) };

    if !running_applications.is_empty() {
        return true;
    }

    // Fallback to process name check
    let app_process_name = OsString::from(APP_PROCESS_NAME);
    let system = System::new_with_specifics(RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing()));
    let mut processes = system.processes_by_exact_name(&app_process_name);
    processes.next().is_some()
}

pub fn launch_fig_desktop(args: LaunchArgs) -> Result<()> {
    if manifest::is_minimal() {
        return Err(eyre!(
            "launching {PRODUCT_NAME} from minimal installs is not yet supported"
        ));
    }

    if system_info::is_remote() {
        return Err(eyre!(
            "launching {PRODUCT_NAME} from remote installs is not yet supported"
        ));
    }

    match desktop_app_running() {
        true => return Ok(()),
        false => {
            if args.verbose {
                println!("Launching {PRODUCT_NAME}...");
            }
        },
    }

    std::fs::remove_file(directories::desktop_socket_path()?).ok();

    let mut common_args = vec![];
    if !args.open_dashboard {
        common_args.push("--no-dashboard");
    }
    if !args.immediate_update {
        common_args.push("--ignore-immediate-update");
    }

    let output = Command::new("open")
        .args(["-g", "-b", tab_util::consts::APP_BUNDLE_ID, "--args"])
        .args(common_args)
        .output()?;

    if !output.status.success() {
        eyre::bail!("failed to launch: {}", String::from_utf8_lossy(&output.stderr));
    }

    if !args.wait_for_socket {
        return Ok(());
    }

    if !desktop_app_running() {
        return Err(eyre!("{PRODUCT_NAME} was unable launch successfully"));
    }

    // Wait for socket to exist
    let path = directories::desktop_socket_path()?;

    for _ in 0..30 {
        // Wait for socket to exist
        if path.exists() {
            return Ok(());
        }

        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    Err(eyre!("failed to connect to socket".to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "not in ci"]
    fn test_e2e_desktop_app_running() {
        println!("{}", desktop_app_running());
    }

    #[test]
    #[ignore = "not in ci"]
    fn test_e2e_launch_fig_desktop() {
        launch_fig_desktop(LaunchArgs {
            wait_for_socket: true,
            open_dashboard: true,
            immediate_update: false,
            verbose: true,
        })
        .unwrap();
    }
}
