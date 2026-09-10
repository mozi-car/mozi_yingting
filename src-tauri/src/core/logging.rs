use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Clone, Debug, Serialize)]
pub struct LogRecord {
    pub level: LogLevel,
    pub component: String,
    pub message: String,
    pub timestamp_ms: u128,
}

#[derive(Clone, Debug)]
pub struct CoreLogger {
    component: String,
}

impl CoreLogger {
    pub fn new(component: impl Into<String>) -> Self {
        Self { component: component.into() }
    }

    pub fn record(&self, level: LogLevel, message: impl Into<String>) -> LogRecord {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or_default();
        LogRecord { level, component: self.component.clone(), message: message.into(), timestamp_ms }
    }
}
