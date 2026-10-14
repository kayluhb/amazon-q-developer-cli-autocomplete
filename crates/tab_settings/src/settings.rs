use std::sync::{
    Arc,
    Mutex,
};

use serde::de::DeserializeOwned;
use serde_json::{
    Map,
    Value,
};

use crate::{
    JsonStore,
    OldSettings,
    Result,
    ValueStore,
};

#[derive(Debug, Clone, Default)]
pub struct Settings(inner::Inner);

mod inner {
    use std::sync::{
        Arc,
        Mutex,
    };

    use serde_json::{
        Map,
        Value,
    };

    #[derive(Debug, Clone, Default)]
    pub enum Inner {
        #[default]
        Real,
        Fake(Arc<Mutex<Map<String, Value>>>),
    }
}

impl Settings {
    pub fn new() -> Self {
        Self(inner::Inner::Real)
    }

    pub fn new_fake() -> Self {
        Self(inner::Inner::Fake(Arc::new(Mutex::new(Map::new()))))
    }

    pub fn from_slice(slice: &[(&str, Value)]) -> Self {
        Self(inner::Inner::Fake(Arc::new(Mutex::new(
            slice.iter().map(|(k, v)| ((*k).to_owned(), v.clone())).collect(),
        ))))
    }
}

impl ValueStore for Settings {
    fn all(&self) -> Result<Map<String, Value>> {
        match &self.0 {
            inner::Inner::Real => Ok(OldSettings::load()?.map().clone()),
            inner::Inner::Fake(map) => Ok(map.lock()?.clone()),
        }
    }

    fn get_value(&self, key: impl AsRef<str>) -> Result<Option<Value>> {
        match &self.0 {
            inner::Inner::Real => Ok(OldSettings::load()?.get(key.as_ref()).map(|v| v.clone())),
            inner::Inner::Fake(map) => Ok(map.lock()?.get(key.as_ref()).cloned()),
        }
    }

    fn set_value(&self, key: impl AsRef<str>, value: impl Into<Value>) -> Result<()> {
        match &self.0 {
            inner::Inner::Real => {
                let mut settings = OldSettings::load()?;
                settings.set(key.as_ref(), value);
                settings.save_to_file()?;
                Ok(())
            },
            inner::Inner::Fake(map) => {
                map.lock()?.insert(key.as_ref().to_owned(), value.into());
                Ok(())
            },
        }
    }

    fn remove_value(&self, key: impl AsRef<str>) -> Result<()> {
        match &self.0 {
            inner::Inner::Real => {
                let mut settings = OldSettings::load()?;
                settings.remove(key);
                settings.save_to_file()?;
                Ok(())
            },
            inner::Inner::Fake(map) => {
                map.lock()?.remove(key.as_ref());
                Ok(())
            },
        }
    }
}

pub trait SettingsProvider {
    fn settings(&self) -> &Settings;
}

impl SettingsProvider for Settings {
    fn settings(&self) -> &Settings {
        self
    }
}

pub fn init_global() -> Result<()> {
    OldSettings::load_into_global()
}

pub fn all() -> Result<Map<String, Value>> {
    Settings::new().all()
}

pub fn set_value(key: impl Into<String>, value: impl Into<serde_json::Value>) -> Result<()> {
    Settings::new().set_value(key.into(), value)
}

pub fn remove_value(key: impl AsRef<str>) -> Result<()> {
    Settings::new().remove_value(key)
}

pub fn get_value(key: impl AsRef<str>) -> Result<Option<serde_json::Value>> {
    Settings::new().get_value(key)
}

pub fn get<T: DeserializeOwned>(key: impl AsRef<str>) -> Result<Option<T>> {
    Settings::new().get(key)
}

pub fn get_bool(key: impl AsRef<str>) -> Result<Option<bool>> {
    Settings::new().get_bool(key)
}

pub fn get_bool_or(key: impl AsRef<str>, default: bool) -> bool {
    Settings::new().get_bool_or(key, default)
}

pub fn get_string(key: impl AsRef<str>) -> Result<Option<String>> {
    Settings::new().get_string(key)
}

pub fn get_string_opt(key: impl AsRef<str>) -> Option<String> {
    Settings::new().get_string_opt(key)
}

pub fn get_string_or(key: impl AsRef<str>, default: impl Into<String>) -> String {
    Settings::new().get_string_or(key, default)
}

pub fn get_int(key: impl AsRef<str>) -> Result<Option<i64>> {
    Settings::new().get_int(key)
}

pub fn get_int_or(key: impl AsRef<str>, default: i64) -> i64 {
    Settings::new().get_int_or(key, default)
}

#[cfg(test)]
mod test {
    use super::{
        Result,
        Settings,
        ValueStore,
    };

    /// General read/write settings test
    #[test]
    fn test_settings() -> Result<()> {
        let settings = Settings::from_slice(&[]);

        assert!(settings.get_value("test").unwrap().is_none());
        assert!(settings.get::<String>("test").unwrap().is_none());
        settings.set_value("test", "hello :)")?;
        assert!(settings.get_value("test").unwrap().is_some());
        assert!(settings.get::<String>("test").unwrap().is_some());
        settings.remove_value("test")?;
        assert!(settings.get_value("test").unwrap().is_none());
        assert!(settings.get::<String>("test").unwrap().is_none());

        assert!(!settings.get_bool_or("bool", false));
        settings.set_value("bool", true).unwrap();
        assert!(settings.get_bool("bool").unwrap().unwrap());

        assert_eq!(settings.get_string_or("string", "hi"), "hi");
        settings.set_value("string", "hi").unwrap();
        assert_eq!(settings.get_string("string").unwrap().unwrap(), "hi");

        assert_eq!(settings.get_int_or("int", 32), 32);
        settings.set_value("int", 32).unwrap();
        assert_eq!(settings.get_int("int").unwrap().unwrap(), 32);

        Ok(())
    }
}
