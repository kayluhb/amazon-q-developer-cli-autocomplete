use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::{
    Map,
    Result,
};

/// A store of JSON values by key, with typed getters.
///
/// Implementors supply the four raw operations. The typed getters are the same for every store,
/// so they are provided here.
pub trait ValueStore {
    /// Returns every key and value in the store.
    fn all(&self) -> Result<Map>;

    /// Returns the raw value for `key`.
    fn get_value(&self, key: impl AsRef<str>) -> Result<Option<Value>>;

    /// Sets `key` to `value`.
    fn set_value(&self, key: impl AsRef<str>, value: impl Into<Value>) -> Result<()>;

    /// Removes `key`.
    fn remove_value(&self, key: impl AsRef<str>) -> Result<()>;

    /// Deserializes the value for `key` into `T`.
    fn get<T: DeserializeOwned>(&self, key: impl AsRef<str>) -> Result<Option<T>> {
        Ok(self.get_value(key)?.map(serde_json::from_value).transpose()?)
    }

    fn get_bool(&self, key: impl AsRef<str>) -> Result<Option<bool>> {
        Ok(self.get_value(key)?.and_then(|value| value.as_bool()))
    }

    fn get_bool_or(&self, key: impl AsRef<str>, default: bool) -> bool {
        self.get_bool(key).ok().flatten().unwrap_or(default)
    }

    fn get_string(&self, key: impl AsRef<str>) -> Result<Option<String>> {
        Ok(self.get_value(key)?.and_then(|value| match value {
            Value::String(s) => Some(s),
            _ => None,
        }))
    }

    fn get_string_opt(&self, key: impl AsRef<str>) -> Option<String> {
        self.get_string(key).ok().flatten()
    }

    fn get_string_or(&self, key: impl AsRef<str>, default: impl Into<String>) -> String {
        self.get_string(key).ok().flatten().unwrap_or_else(|| default.into())
    }

    fn get_int(&self, key: impl AsRef<str>) -> Result<Option<i64>> {
        Ok(self.get_value(key)?.and_then(|value| value.as_i64()))
    }

    fn get_int_or(&self, key: impl AsRef<str>, default: i64) -> i64 {
        self.get_int(key).ok().flatten().unwrap_or(default)
    }
}
