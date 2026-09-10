use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CoreConfig {
    pub project_root: Option<PathBuf>,
    pub project_name: Option<String>,
    pub values: BTreeMap<String, String>,
}

#[derive(Clone, Default)]
pub struct ConfigStore {
    value: Arc<RwLock<CoreConfig>>,
}

impl ConfigStore {
    pub fn new(value: CoreConfig) -> Self { Self { value: Arc::new(RwLock::new(value)) } }
    pub fn snapshot(&self) -> CoreConfig { self.value.read().expect("config poisoned").clone() }
    pub fn replace(&self, value: CoreConfig) { *self.value.write().expect("config poisoned") = value; }
    pub fn set(&self, key: impl Into<String>, value: impl Into<String>) {
        self.value.write().expect("config poisoned").values.insert(key.into(), value.into());
    }
    pub fn get(&self, key: &str) -> Option<String> {
        self.value.read().expect("config poisoned").values.get(key).cloned()
    }
}
