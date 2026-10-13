use std::borrow::Cow;
use std::io::ErrorKind;
use std::path::{
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

async fn merged_index_json(ctx: &Context) -> SpecIndex {
    let mut cache = INDEX_CACHE.lock().await;
    if let Some(index) = cache.as_ref() {
        return index.clone();
    }

    let mut completions = FnvHashSet::default();
    let mut diff_versioned_completions = FnvHashSet::default();
    for index in spec_dirs(ctx).iter().filter_map(|dir| read_index(dir)) {
        completions.extend(index.completions);
        diff_versioned_completions.extend(index.diff_versioned_completions);
    }

    let mut completions: Vec<_> = completions.into_iter().collect();
    completions.sort();
    let mut diff_versioned_completions: Vec<_> = diff_versioned_completions.into_iter().collect();
    diff_versioned_completions.sort();

    let index = SpecIndex {
        completions,
        diff_versioned_completions,
    };
    *cache = Some(index.clone());
    index
}

/// Resolve `relative` inside `dir`, refusing paths that escape it
fn resolve(dir: &Path, relative: &str) -> Option<PathBuf> {
    let root = dir.canonicalize().ok()?;
    let path = root.join(relative).canonicalize().ok()?;
    (path.starts_with(&root) && path.is_file()).then_some(path)
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
    let path = request.uri().path();

    if path == "/index.json" {
        let index = merged_index_json(&ctx).await;
        return Ok(res_ok(
            serde_json::to_vec(&index)?,
            "application/json".try_into().unwrap(),
        ));
    }

    let relative = path.strip_prefix('/').unwrap_or(path);
    for dir in spec_dirs(&ctx) {
        if let Some(file) = resolve(&dir, relative) {
            return Ok(res_ok(tokio::fs::read(&file).await?, content_type(&file)));
        }
    }

    Ok(res_404())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_rejects_escape() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("git.js"), "").unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        assert!(resolve(dir.path(), "git.js").is_some());
        assert!(resolve(dir.path(), "missing.js").is_none());
        assert!(resolve(&dir.path().join("sub"), "../git.js").is_none());
    }
}
