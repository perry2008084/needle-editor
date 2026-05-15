use std::collections::BTreeMap;

use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct Settings {
    values: BTreeMap<String, Value>,
}

impl Settings {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.values.get(key)
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(Value::as_bool)
    }

    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(Value::as_str)
    }

    pub fn set(&mut self, key: impl Into<String>, value: Value) {
        self.values.insert(key.into(), value);
    }

    pub fn erase(&mut self, key: &str) -> Option<Value> {
        self.values.remove(key)
    }

    pub fn has(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }

    pub fn as_map(&self) -> &BTreeMap<String, Value> {
        &self.values
    }
}
