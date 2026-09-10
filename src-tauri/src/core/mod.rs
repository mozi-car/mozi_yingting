pub mod config;
pub mod device;
pub mod error;
pub mod event_bus;
pub mod task;
pub mod transport;

use config::CoreConfig;
use device::DeviceManager;
use event_bus::EventBus;
use std::sync::{Arc, RwLock};
use task::TaskSupervisor;
use transport::TransportRegistry;

#[derive(Clone)]
pub struct Core {
    pub config: Arc<RwLock<CoreConfig>>,
    pub events: EventBus,
    pub tasks: TaskSupervisor,
    pub devices: DeviceManager,
    pub transports: Arc<TransportRegistry>,
}

impl Core {
    pub fn new() -> Self {
        let events = EventBus::new();
        Self {
            config: Arc::new(RwLock::new(CoreConfig::default())),
            events: events.clone(),
            tasks: TaskSupervisor::new(),
            devices: DeviceManager::new(events),
            transports: Arc::new(TransportRegistry::new()),
        }
    }
}

impl Default for Core {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::Core;

    #[test]
    fn core_starts_with_empty_device_and_task_state() {
        let core = Core::new();
        assert!(core.devices.list().is_empty());
        assert_eq!(core.tasks.active_count(), 0);
    }
}
