use std::borrow::Cow;
use std::io::ErrorKind;
use std::path::{
    Component,
    Path,
    PathBuf,
};
use std::sync::Arc;

use fnv::FnvHashSet;
use serde::{
    Deserialize,
    Serialize,
};
use tab_os_shim::Context;
use tokio::sync::Mutex;
use tracing::error;
use wry::http::header::CONTENT_TYPE;
use wry::http::{
    HeaderValue,
    Request,
    Response,
    StatusCode,
};

use crate::webview::WindowId;

const APPLICATION_JAVASCRIPT: HeaderValue = HeaderValue::from_static("application/javascript");

fn res_404() -> Response<Cow<'static, [u8]>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header(CONTENT_TYPE, "text/plain")
        .body(b"Not Found".as_ref().into())
        .unwrap()
}

fn res_ok(bytes: Vec<u8>, content_type: HeaderValue) -> Response<Cow<'static, [u8]>> {
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, content_type)
        .body(bytes.into())
        .unwrap()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SpecIndex {
    completions: Vec<String>,
    #[serde(default)]
    diff_versioned_completions: Vec<String>,
}

/// Spec directories in priority order: user specs override the bundled ones
fn spec_dirs(ctx: &Context) -> Vec<PathBuf> {
    let mut dirs = vec![];
    if let Ok(dir) = tab_util::directories::autocomplete_specs_dir() {
        dirs.push(dir);
    }
    if let Ok(dir) = tab_util::directories::resources_path_ctx(ctx) {
        dirs.push(dir.join("specs"));
    }
    dirs
}

static INDEX_CACHE: Mutex<Option<SpecIndex>> = Mutex::const_new(None);

pub async fn clear_index_cache() {
    *INDEX_CACHE.lock().await = None;
}

fn read_index(dir: &Path) -> Option<SpecIndex> {
    let bytes = match std::fs::read(dir.join("index.json")) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == ErrorKind::NotFound => return None,
        Err(err) => {
            error!(%err, ?dir, "Failed to read spec index");
            return None;
        },
    };
    match serde_json::from_slice(&bytes) {
        Ok(index) => Some(index),
        Err(err) => {
            error!(%err, ?dir, "Failed to parse spec index");
            None
        },
    }
}

/// Merge the indexes of `dirs`. The first directory that lists a spec decides whether it is
/// diff-versioned, so a user's single-file spec overrides a bundled diff-versioned one
fn merge_indexes(dirs: &[PathBuf]) -> SpecIndex {
    let mut seen = FnvHashSet::default();
    let mut completions = vec![];
    let mut diff_versioned_completions = vec![];
    for index in dirs.iter().filter_map(|dir| read_index(dir)) {
        let diff_versioned: FnvHashSet<_> = index.diff_versioned_completions.into_iter().collect();
        for name in index.completions {
            if !seen.insert(name.clone()) {
                continue;
            }
            if diff_versioned.contains(&name) {
                diff_versioned_completions.push(name.clone());
            }
            completions.push(name);
        }
    }

    completions.sort();
    diff_versioned_completions.sort();
    SpecIndex {
        completions,
        diff_versioned_completions,
    }
}

async fn merged_index_json(ctx: &Context) -> SpecIndex {
    let mut cache = INDEX_CACHE.lock().await;
    if let Some(index) = cache.as_ref() {
        return index.clone();
    }

    let index = merge_indexes(&spec_dirs(ctx));
    *cache = Some(index.clone());
    index
}

/// Resolve `relative` inside `dir`, refusing `..` and absolute paths. Symlinks are followed, so
/// users can link specs in from elsewhere
fn resolve(dir: &Path, relative: &str) -> Option<PathBuf> {
    let relative = Path::new(relative);
    if !relative
        .components()
        .all(|component| matches!(component, Component::Normal(_)))
    {
        return None;
    }
    let path = dir.join(relative);
    path.is_file().then_some(path)
}

/// Find `relative` in the first of `dirs` that has it
fn find_spec(dirs: &[PathBuf], relative: &str) -> Option<PathBuf> {
    dirs.iter().find_map(|dir| resolve(dir, relative))
}

fn content_type(path: &Path) -> HeaderValue {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("json") => HeaderValue::from_static("application/json"),
        Some("png") => HeaderValue::from_static("image/png"),
        Some("svg") => HeaderValue::from_static("image/svg+xml"),
        _ => APPLICATION_JAVASCRIPT,
    }
}

// handle `spec://localhost/spec.js`
pub async fn handle(
    ctx: Arc<Context>,
    request: Request<Vec<u8>>,
    _: WindowId,
) -> anyhow::Result<Response<Cow<'static, [u8]>>> {
    if request.uri().host() != Some("localhost") {
        return Ok(res_404());
    }

    let path = request.uri().path();
    if path == "/index.json" {
        let index = merged_index_json(&ctx).await;
        return Ok(res_ok(
            serde_json::to_vec(&index)?,
            "application/json".try_into().unwrap(),
        ));
    }

    let relative = path.strip_prefix('/').unwrap_or(path);
    match find_spec(&spec_dirs(&ctx), relative) {
        Some(file) => Ok(res_ok(tokio::fs::read(&file).await?, content_type(&file))),
        None => Ok(res_404()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_index(dir: &Path, completions: &[&str], diff_versioned: &[&str]) {
        let index = serde_json::json!({
            "completions": completions,
            "diffVersionedCompletions": diff_versioned,
        });
        std::fs::write(dir.join("index.json"), index.to_string()).unwrap();
    }

    #[test]
    fn resolve_rejects_escape() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("git.js"), "").unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        assert!(resolve(dir.path(), "git.js").is_some());
        assert!(resolve(dir.path(), "missing.js").is_none());
        assert!(resolve(dir.path(), "sub").is_none());
        assert!(resolve(&dir.path().join("sub"), "../git.js").is_none());
        assert!(resolve(dir.path(), "/etc/hosts").is_none());
    }

    #[test]
    fn resolve_follows_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        std::fs::write(elsewhere.path().join("foo.js"), "").unwrap();
        std::os::unix::fs::symlink(elsewhere.path().join("foo.js"), dir.path().join("foo.js")).unwrap();
        assert!(resolve(dir.path(), "foo.js").is_some());
    }

    #[test]
    fn find_spec_prefers_user_dir() {
        let user = tempfile::tempdir().unwrap();
        let bundled = tempfile::tempdir().unwrap();
        std::fs::write(user.path().join("git.js"), "").unwrap();
        std::fs::write(bundled.path().join("git.js"), "").unwrap();
        std::fs::write(bundled.path().join("ls.js"), "").unwrap();
        let dirs = [user.path().to_path_buf(), bundled.path().to_path_buf()];

        assert_eq!(find_spec(&dirs, "git.js"), Some(user.path().join("git.js")));
        assert_eq!(find_spec(&dirs, "ls.js"), Some(bundled.path().join("ls.js")));
        assert_eq!(find_spec(&dirs, "cd.js"), None);
    }

    #[test]
    fn merge_indexes_user_dir_wins() {
        let user = tempfile::tempdir().unwrap();
        let bundled = tempfile::tempdir().unwrap();
        // The user overrides the diff-versioned `aws` spec with a single file
        write_index(user.path(), &["aws", "mytool"], &[]);
        write_index(bundled.path(), &["aws", "git", "kubectl"], &["aws", "kubectl"]);

        let index = merge_indexes(&[user.path().to_path_buf(), bundled.path().to_path_buf()]);
        assert_eq!(index.completions, ["aws", "git", "kubectl", "mytool"]);
        assert_eq!(index.diff_versioned_completions, ["kubectl"]);
    }

    #[test]
    fn merge_indexes_skips_missing_and_invalid() {
        let missing = tempfile::tempdir().unwrap();
        let invalid = tempfile::tempdir().unwrap();
        let bundled = tempfile::tempdir().unwrap();
        std::fs::write(invalid.path().join("index.json"), "not json").unwrap();
        // Older indexes have no diffVersionedCompletions
        std::fs::write(bundled.path().join("index.json"), r#"{"completions":["git"]}"#).unwrap();

        let index = merge_indexes(&[
            missing.path().to_path_buf(),
            invalid.path().to_path_buf(),
            bundled.path().to_path_buf(),
        ]);
        assert_eq!(index.completions, ["git"]);
        assert!(index.diff_versioned_completions.is_empty());
    }

    #[test]
    fn content_type_by_extension() {
        assert_eq!(content_type(Path::new("index.json")), "application/json");
        assert_eq!(content_type(Path::new("git.png")), "image/png");
        assert_eq!(content_type(Path::new("git.svg")), "image/svg+xml");
        assert_eq!(content_type(Path::new("git.js")), "application/javascript");
    }
}
