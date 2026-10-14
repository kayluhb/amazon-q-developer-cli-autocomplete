use serde::de::DeserializeOwned;
use serde_json::{
    Map,
    Value,
};

use crate::sqlite::{
    Db,
    database,
};
use crate::{
    Result,
    ValueStore,
};

#[derive(Debug, Clone, Default)]
pub struct State(inner::Inner);

mod inner {
    #[derive(Debug, Clone, Default)]
    pub enum Inner {
        #[default]
        Real,
        Fake(crate::sqlite::Db),
    }
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_fake() -> Self {
        let db = Db::mock();
        db.migrate().unwrap();
        Self(inner::Inner::Fake(db))
    }

    pub fn from_slice(slice: &[(&str, Value)]) -> Self {
        let fake = Self::new_fake();
        for (key, value) in slice {
            fake.set_value(key, value.clone()).unwrap();
        }
        fake
    }

    fn database(&self) -> Result<&Db> {
        match &self.0 {
            inner::Inner::Real => Ok(database()?),
            inner::Inner::Fake(db) => Ok(db),
        }
    }

    // Atomic style operations

    pub fn atomic_bool_or(&self, key: impl AsRef<str>, or: bool) -> Result<bool> {
        self.database()?.atomic_bool_or(key, or)
    }
}

impl ValueStore for State {
    fn all(&self) -> Result<Map<String, Value>> {
        self.database()?.all_state_values()
    }

    fn get_value(&self, key: impl AsRef<str>) -> Result<Option<Value>> {
        self.database()?.get_state_value(key)
    }

    fn set_value(&self, key: impl AsRef<str>, value: impl Into<Value>) -> Result<()> {
        self.database()?.set_state_value(key, value)?;
        Ok(())
    }

    fn remove_value(&self, key: impl AsRef<str>) -> Result<()> {
        self.database()?.unset_state_value(key)?;
        Ok(())
    }
}

pub trait StateProvider {
    fn state(&self) -> &State;
}

impl StateProvider for State {
    fn state(&self) -> &State {
        self
    }
}

pub fn all() -> Result<Map<String, Value>> {
    State::new().all()
}

pub fn set_value(key: impl AsRef<str>, value: impl Into<Value>) -> Result<()> {
    State::new().set_value(key, value)
}

pub fn remove_value(key: impl AsRef<str>) -> Result<()> {
    State::new().remove_value(key)
}

pub fn get_value(key: impl AsRef<str>) -> Result<Option<Value>> {
    State::new().get_value(key)
}

pub fn get<T: DeserializeOwned>(key: impl AsRef<str>) -> Result<Option<T>> {
    State::new().get(key)
}

pub fn get_bool(key: impl AsRef<str>) -> Result<Option<bool>> {
    State::new().get_bool(key)
}

pub fn get_bool_or(key: impl AsRef<str>, default: bool) -> bool {
    State::new().get_bool_or(key, default)
}

pub fn get_string(key: impl AsRef<str>) -> Result<Option<String>> {
    State::new().get_string(key)
}

pub fn get_string_opt(key: impl AsRef<str>) -> Option<String> {
    State::new().get_string_opt(key)
}

pub fn get_string_or(key: impl AsRef<str>, default: impl Into<String>) -> String {
    State::new().get_string_or(key, default)
}

pub fn get_int(key: impl AsRef<str>) -> Result<Option<i64>> {
    State::new().get_int(key)
}

pub fn get_int_or(key: impl AsRef<str>, default: i64) -> i64 {
    State::new().get_int_or(key, default)
}

#[cfg(test)]
mod tests {
    use super::{
        Result,
        State,
        ValueStore,
    };

    /// General read/write state test
    #[test]
    fn test_state() -> Result<()> {
        let state = State::new_fake();

        assert!(state.get_value("test").unwrap().is_none());
        assert!(state.get::<String>("test").unwrap().is_none());
        state.set_value("test", "hello :)")?;
        assert!(state.get_value("test").unwrap().is_some());
        assert!(state.get::<String>("test").unwrap().is_some());
        state.remove_value("test")?;
        assert!(state.get_value("test").unwrap().is_none());
        assert!(state.get::<String>("test").unwrap().is_none());

        assert!(!state.get_bool_or("bool", false));
        state.set_value("bool", true).unwrap();
        assert!(state.get_bool("bool").unwrap().unwrap());

        assert_eq!(state.get_string_or("string", "hi"), "hi");
        state.set_value("string", "hi").unwrap();
        assert_eq!(state.get_string("string").unwrap().unwrap(), "hi");

        assert_eq!(state.get_int_or("int", 32), 32);
        state.set_value("int", 32).unwrap();
        assert_eq!(state.get_int("int").unwrap().unwrap(), 32);

        Ok(())
    }
}
