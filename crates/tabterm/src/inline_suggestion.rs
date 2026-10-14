//! Caches the autocomplete popup's top suggestion so the zsh inline suggestions plugin can show it
//! as ghost text.

use std::sync::LazyLock;
use std::time::Duration;

use tokio::sync::watch;

/// How long a lookup waits for the popup to send a suggestion for the buffer
pub const LOOKUP_TIMEOUT: Duration = Duration::from_millis(150);

pub static INLINE_SUGGESTION: LazyLock<InlineSuggestionCache> = LazyLock::new(InlineSuggestionCache::new);

#[derive(Debug)]
struct Entry {
    buffer: String,
    suffix: String,
}

#[derive(Debug)]
pub struct InlineSuggestionCache {
    entry: watch::Sender<Option<Entry>>,
}

impl InlineSuggestionCache {
    pub fn new() -> Self {
        Self {
            entry: watch::Sender::new(None),
        }
    }

    /// Replaces the cached suggestion and wakes any lookups waiting on it
    pub fn set(&self, buffer: String, suffix: String) {
        self.entry.send_replace(Some(Entry { buffer, suffix }));
    }

    /// Returns the suffix cached for `buffer`, waiting up to `timeout` for the popup to send one.
    /// Returns `None` on a timeout or when the popup has no suggestion for the buffer.
    pub async fn lookup(&self, buffer: &str, timeout: Duration) -> Option<String> {
        let mut rx = self.entry.subscribe();
        let matching = rx.wait_for(|entry| entry.as_ref().is_some_and(|entry| entry.buffer == buffer));
        match tokio::time::timeout(timeout, matching).await {
            Ok(Ok(entry)) => entry
                .as_ref()
                .map(|entry| entry.suffix.clone())
                .filter(|suffix| !suffix.is_empty()),
            _ => None,
        }
    }
}

impl Default for InlineSuggestionCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    const TIMEOUT: Duration = Duration::from_millis(50);

    #[tokio::test]
    async fn immediate_hit() {
        let cache = InlineSuggestionCache::new();
        cache.set("git che".into(), "ckout".into());
        assert_eq!(cache.lookup("git che", TIMEOUT).await.as_deref(), Some("ckout"));
    }

    #[tokio::test]
    async fn empty_suffix_is_none() {
        let cache = InlineSuggestionCache::new();
        cache.set("git che".into(), "".into());
        assert_eq!(cache.lookup("git che", Duration::from_secs(10)).await, None);
    }

    #[tokio::test]
    async fn mismatch_times_out() {
        let cache = InlineSuggestionCache::new();
        cache.set("git".into(), " ".into());
        assert_eq!(cache.lookup("git che", TIMEOUT).await, None);
    }

    #[tokio::test]
    async fn later_set_resolves_wait() {
        let cache = Arc::new(InlineSuggestionCache::new());
        let lookup = tokio::spawn({
            let cache = cache.clone();
            async move { cache.lookup("git che", Duration::from_secs(10)).await }
        });
        tokio::time::sleep(Duration::from_millis(10)).await;
        cache.set("git che".into(), "ckout".into());
        assert_eq!(lookup.await.unwrap().as_deref(), Some("ckout"));
    }

    #[tokio::test]
    async fn newer_set_replaces_entry() {
        let cache = InlineSuggestionCache::new();
        cache.set("git che".into(), "ckout".into());
        cache.set("git sta".into(), "tus".into());
        assert_eq!(cache.lookup("git che", TIMEOUT).await, None);
        assert_eq!(cache.lookup("git sta", TIMEOUT).await.as_deref(), Some("tus"));
    }
}
