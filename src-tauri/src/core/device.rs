use crate::core::error::{CoreError, CoreResult};
use crate::core::event_bus::{CoreEvent, EventBus};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub type DeviceId = String;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DeviceKind {
    Can,
    Lin,
    Serial,
    Ethernet,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub id: DeviceId,
    pub name: String,
    pub kind: DeviceKind,
    pub vendor: Option<String>,
    pub path: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DeviceState {
    Discovered,
    Opening,
    Open,
    Closing,
    Closed,
    Error,
}

#[derive(Clone, Default)]
pub struct DeviceManager {
    devices: Arc<Mutex<HashMap<DeviceId, (DeviceInfo, DeviceState)>>>,
    events: EventBus,
}

impl DeviceManager {
    pub fn new(events: EventBus) -> Self {
        Self {
            devices: Arc::new(Mutex::new(HashMap::new())),
            events,
        }
    }

    pub fn upsert(&self, device: DeviceInfo) -> bool {
        let id = device.id.clone();
        let mut devices = self.devices.lock().expect("device manager poisoned");
        let is_new = !devices.contains_key(&id);
        devices.insert(id, (device.clone(), DeviceState::Discovered));
        drop(devices);
        if is_new {
            self.events.publish(CoreEvent::DeviceAdded(device));
        }
        is_new
    }

    pub fn remove(&self, id: &str) -> Option<DeviceInfo> {
        let removed = self
            .devices
            .lock()
            .expect("device manager poisoned")
            .remove(id)
            .map(|(device, _)| device);
        if removed.is_some() {
            self.events
                .publish(CoreEvent::DeviceRemoved { id: id.to_string() });
        }
        removed
    }

    pub fn set_state(&self, id: &str, state: DeviceState) -> CoreResult<()> {
        let mut devices = self.devices.lock().expect("device manager poisoned");
        let (_, current) = devices
            .get_mut(id)
            .ok_or_else(|| CoreError::not_found(format!("device not found: {id}")))?;
        *current = state.clone();
        drop(devices);
        self.events.publish(CoreEvent::DeviceStateChanged {
            id: id.to_string(),
            state,
        });
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<(DeviceInfo, DeviceState)> {
        self.devices
            .lock()
            .expect("device manager poisoned")
            .get(id)
            .cloned()
    }

    pub fn list(&self) -> Vec<(DeviceInfo, DeviceState)> {
        self.devices
            .lock()
            .expect("device manager poisoned")
            .values()
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{DeviceInfo, DeviceKind, DeviceManager, DeviceState};
    use crate::core::event_bus::EventBus;

    #[test]
    fn device_lifecycle_publishes_typed_events() {
        let events = EventBus::new();
        let receiver = events.subscribe();
        let manager = DeviceManager::new(events);
        manager.upsert(DeviceInfo {
            id: "can-1".into(),
            name: "CAN 1".into(),
            kind: DeviceKind::Can,
            vendor: Some("test".into()),
            path: None,
        });
        manager.set_state("can-1", DeviceState::Open).unwrap();
        assert!(matches!(receiver.recv().unwrap(), crate::core::event_bus::CoreEvent::DeviceAdded(_)));
        assert!(matches!(receiver.recv().unwrap(), crate::core::event_bus::CoreEvent::DeviceStateChanged { .. }));
    }
}
