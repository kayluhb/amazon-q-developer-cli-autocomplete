//! Trusted origins for the webviews.
//!
//! Only content from these origins may navigate the webview, send IPC messages,
//! or call the custom protocols.

use url::Url;

use super::autocomplete::RESOURCE_URL;
use super::{
    AUTOCOMPLETE_ID,
    DASHBOARD_ID,
    WindowId,
    autocomplete,
    dashboard,
};

const FILE_SCHEME: &str = "file";

/// The URLs that a window loads. The bundled resource URL is always trusted. The current
/// URL can differ in development (`DASHBOARD_URL`, `developer.dashboard.host`, etc).
pub fn trusted_urls(window_id: &WindowId) -> Vec<Url> {
    let mut urls = vec![RESOURCE_URL.clone()];
    if *window_id == DASHBOARD_ID {
        urls.push(dashboard::url());
    } else if *window_id == AUTOCOMPLETE_ID {
        urls.push(autocomplete::url());
    }
    urls
}

/// The (scheme, host, port) tuple of a URL. Returns `None` for `file:` URLs and URLs without
/// a host.
fn origin_tuple(url: &Url) -> Option<(&str, &str, Option<u16>)> {
    if url.scheme() == FILE_SCHEME {
        return None;
    }
    let host = url.host_str().filter(|host| !host.is_empty())?;
    Some((url.scheme(), host, url.port_or_known_default()))
}

/// Returns true if `url` has the same scheme, host and port as one of `trusted`.
pub fn is_trusted_origin(url: &str, trusted: &[Url]) -> bool {
    let Ok(url) = Url::parse(url) else {
        return false;
    };
    let Some(origin) = origin_tuple(&url) else {
        return false;
    };
    trusted.iter().any(|trusted| origin_tuple(trusted) == Some(origin))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trusted() -> Vec<Url> {
        vec![
            Url::parse("qcliresource://localhost").unwrap(),
            Url::parse("http://localhost:5173/some/path").unwrap(),
        ]
    }

    #[test]
    fn allows_exact_origins() {
        let trusted = trusted();
        assert!(is_trusted_origin("qcliresource://localhost", &trusted));
        assert!(is_trusted_origin("qcliresource://localhost/index.html?a=b#c", &trusted));
        assert!(is_trusted_origin("http://localhost:5173", &trusted));
        assert!(is_trusted_origin("http://localhost:5173/other", &trusted));
    }

    #[test]
    fn rejects_other_ports() {
        let trusted = trusted();
        assert!(!is_trusted_origin("http://localhost", &trusted));
        assert!(!is_trusted_origin("http://localhost:8080/", &trusted));
    }

    #[test]
    fn rejects_other_schemes() {
        let trusted = trusted();
        assert!(!is_trusted_origin("https://localhost:5173", &trusted));
        assert!(!is_trusted_origin("api://localhost", &trusted));
        assert!(!is_trusted_origin("file://localhost/etc/passwd", &trusted));
        assert!(!is_trusted_origin("file:///etc/passwd", &trusted));
    }

    #[test]
    fn rejects_other_hosts() {
        let trusted = trusted();
        assert!(!is_trusted_origin("qcliresource://evil.localhost", &trusted));
        assert!(!is_trusted_origin("http://evil.localhost:5173", &trusted));
        assert!(!is_trusted_origin("http://127.0.0.1:5173", &trusted));
        assert!(!is_trusted_origin("https://example.com", &trusted));
    }

    #[test]
    fn rejects_invalid_values() {
        let trusted = trusted();
        assert!(!is_trusted_origin("", &trusted));
        assert!(!is_trusted_origin("null", &trusted));
        assert!(!is_trusted_origin("about:blank", &trusted));
        assert!(!is_trusted_origin("data:text/html,hi", &trusted));
    }

    #[test]
    fn rejects_lookalike_urls() {
        let trusted = trusted();
        assert!(!is_trusted_origin("http://localhost:5173@evil.com", &trusted));
        assert!(!is_trusted_origin("http://localhost.:5173", &trusted));
        assert!(!is_trusted_origin("http://[::1]:5173", &trusted));
        assert!(!is_trusted_origin("qcliresource://localhost:1234", &trusted));
    }

    #[test]
    fn host_case_follows_url_normalization() {
        let trusted = trusted();
        // Special schemes such as http lowercase the host. Custom schemes keep the case.
        assert!(is_trusted_origin("HTTP://LOCALHOST:5173", &trusted));
        assert!(!is_trusted_origin("qcliresource://LOCALHOST", &trusted));
    }

    #[test]
    fn file_urls_are_never_trusted() {
        let trusted = vec![Url::parse("file:///tmp/index.html").unwrap()];
        assert!(!is_trusted_origin("file:///tmp/index.html", &trusted));
    }

    #[test]
    fn default_port_matches_explicit_port() {
        let trusted = vec![Url::parse("http://localhost:80").unwrap()];
        assert!(is_trusted_origin("http://localhost/", &trusted));
    }

    #[test]
    fn trusted_urls_contain_resource_url() {
        assert!(trusted_urls(&WindowId("other".into())).contains(&RESOURCE_URL));
    }
}
