use super::config::{ConfigStore, CoreConfig};
use super::device::DeviceManager;
use super::event_bus::EventBus;
use super::logging::CoreLogger;
use super::task::TaskRuntime;

#[derive(Clone)]
pub struct CoreRuntime {
    pub config: ConfigStore,
    pub events: EventBus,
    pub tasks: TaskRuntime,
    pub logger: CoreLogger,
    pub devices: DeviceManager,
}

impl CoreRuntime {
    pub fn new(component: impl Into<String>) -> Self {
        let events = EventBus::new();
        Self {
            config: ConfigStore::new(CoreConfig::default()),
            events: events.clone(),
            tasks: TaskRuntime::new(),
            logger: CoreLogger::new(component),
            devices: DeviceManager::new(events),
        }
    }
}

impl Default for CoreRuntime {
    fn default() -> Self { Self::new("rust-core") }
}

#[cfg(test)]
mod tests {
    use super::CoreRuntime;
    use crate::core::event_bus::CoreEvent;
    use crate::core::logging::LogLevel;
    use serde_json::json;

    #[test]
    fn infrastructure_supports_config_events_tasks_and_logging() {
        let runtime = CoreRuntime::default();
        runtime.config.set("mode", "test");
        assert_eq!(runtime.config.get("mode").as_deref(), Some("test"));
        let rx = runtime.events.subscribe();
        runtime.events.publish(CoreEvent { topic: "test".into(), payload: json!({"ok": true}) });
        assert_eq!(rx.recv().unwrap().topic, "test");
        let token = runtime.tasks.register();
        assert!(!token.is_cancelled());
        runtime.tasks.cancel_all();
        assert!(token.is_cancelled());
        assert!(runtime.logger.record(LogLevel::Info, "ok").timestamp_ms > 0);
    }
}
