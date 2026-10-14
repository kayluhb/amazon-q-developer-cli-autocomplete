//! Request handling that the settings and the state requests share.

use tab_settings::ValueStore;

use super::{
    Error,
    RequestResult,
    RequestResultImpl,
};

/// Returns the value for `key` as JSON, or the whole store when there is no key.
///
/// `name` is the store name for error messages, for example `settings`.
pub(super) fn get_json_blob(store: &impl ValueStore, name: &str, key: Option<String>) -> Result<String, Error> {
    let res = match key {
        Some(key) => serde_json::to_string(
            &store
                .get_value(&key)
                .map_err(|err| format!("Failed getting {name} value for {key}: {err}"))?
                .ok_or_else(|| format!("No value for key '{key}'"))?,
        ),
        None => store
            .all()
            .map(|map| serde_json::to_string(&map))
            .map_err(|err| format!("Failed getting {name}: {err}"))?,
    };

    Ok(res.map_err(|err| format!("Could not convert value for key to JSON: {err}"))?)
}

/// Sets `key` to `value`, or removes `key` when there is no value.
///
/// A value that is not valid JSON is stored as a JSON string.
pub(super) fn update(store: &impl ValueStore, key: Option<&str>, value: Option<String>) -> RequestResult {
    match (key, value) {
        (Some(key), Some(value)) => {
            let value = serde_json::from_str(&value).unwrap_or(serde_json::Value::String(value));
            store
                .set_value(key, value)
                .map_err(|err| format!("Failed setting {key}: {err}"))?;
        },
        (Some(key), None) => {
            store
                .remove_value(key)
                .map_err(|err| format!("Failed removing {key}: {err}"))?;
        },
        (None, _) => {
            return RequestResult::error("No key provided with request");
        },
    }

    RequestResult::success()
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use tab_settings::Settings;

    use super::*;
    use crate::requests::ServerOriginatedSubMessage;

    #[test]
    fn get_json_blob_returns_value_or_all() {
        let store = Settings::from_slice(&[("a", json!(1))]);
        assert_eq!(get_json_blob(&store, "settings", Some("a".into())).unwrap(), "1");
        assert_eq!(get_json_blob(&store, "settings", None).unwrap(), r#"{"a":1}"#);
        assert_eq!(
            get_json_blob(&store, "settings", Some("b".into()))
                .unwrap_err()
                .to_string(),
            "No value for key 'b'"
        );
    }

    #[test]
    fn update_sets_and_removes() {
        let store = Settings::new_fake();
        let key = "key";

        update(&store, Some(key), Some("true".into())).unwrap();
        assert_eq!(store.get_value(key).unwrap(), Some(json!(true)));

        update(&store, Some(key), Some("not json".into())).unwrap();
        assert_eq!(store.get_value(key).unwrap(), Some(json!("not json")));

        update(&store, Some(key), None).unwrap();
        assert_eq!(store.get_value(key).unwrap(), None);
    }

    #[test]
    fn update_without_key_fails() {
        let store = Settings::new_fake();
        for value in [Some("true".to_string()), None] {
            let response = *update(&store, None, value).unwrap();
            assert!(matches!(response, ServerOriginatedSubMessage::Error(_)), "{response:?}");
        }
        assert!(store.all().unwrap().is_empty());
    }
}
