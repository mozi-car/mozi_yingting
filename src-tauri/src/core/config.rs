use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CoreConfig {
    pub project_root: Option<PathBuf>,
    pub project_name: Option<String>,
    pub values: BTreeMap<String, String>,
}

impl CoreConfig {
    pub fn with_project(root: impl Into<PathBuf>, name: impl Into<String>) -> Self {
        Self {
            project_root: Some(root.into()),
            project_name: Some(name.into()),
            values: BTreeMap::new(),
        }
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.values.insert(key.into(), value.into());
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }
}
