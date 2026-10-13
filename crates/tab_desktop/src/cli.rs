use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Do not launch the dashboard when starting
    #[arg(long)]
    pub no_dashboard: bool,
    /// Checks the `app.launchOnStartup` setting before launching
    #[arg(long)]
    pub is_startup: bool,
    /// Kill old instances of `tab_desktop`
    #[arg(long)]
    pub kill_old: bool,
    /// Allow launching multiple instances of `tab_desktop`
    #[arg(long)]
    pub allow_multiple: bool,
    /// Accepted for compatibility with `tab launch`; the standalone build never updates
    #[arg(long, hide = true)]
    pub ignore_immediate_update: bool,
    /// Url to open
    pub url_link: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_app() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}
