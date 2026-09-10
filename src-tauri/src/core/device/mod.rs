use crate::core::error::{CoreError, CoreResult};
use crate::core::event_bus::{CoreEvent, EventBus};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

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

#[derive(Clone, Debug, Serialize)]
pub struct DeviceSnapshot {
    pub info: DeviceInfo,
    pub state: DeviceState,
}

#[derive(Clone, Default)]
pub struct DeviceManager {
    devices: Arc<RwLock<HashMap<DeviceId, DeviceSnapshot>>>,
    events: EventBus,
}

impl DeviceManager {
    pub fn new(events: EventBus) -> Self {
        Self { devices: Arc::new(RwLock::new(HashMap::new())), events }
    }

    pub fn register(&self, info: DeviceInfo) -> bool {
        let id = info.id.clone();
        let mut devices = self.devices.write().expect("device manager poisoned");
        let inserted = !devices.contains_key(&id);
        devices.insert(id, DeviceSnapshot { info: info.clone(), state: DeviceState::Discovered });
        drop(devices);
        if inserted {
            self.events.publish(CoreEvent { topic: "device.added".into(), payload: serde_json::to_value(info).unwrap_or_default() });
        }
        inserted
    }

    pub fn remove(&self, id: &str) -> Option<DeviceSnapshot> {
        let removed = self.devices.write().expect("device manager poisoned").remove(id);
        if removed.is_some() {
            self.events.publish(CoreEvent { topic: "device.removed".into(), payload: serde_json::json!({ "id": id }) });
        }
        removed
    }

    pub fn set_state(&self, id: &str, state: DeviceState) -> CoreResult<()> {
        let mut devices = self.devices.write().expect("device manager poisoned");
        let snapshot = devices.get_mut(id).ok_or_else(|| CoreError::not_found(format!("device not found: {id}")))?;
        snapshot.state = state.clone();
        drop(devices);
        self.events.publish(CoreEvent { topic: "device.state".into(), payload: serde_json::json!({ "id": id, "state": state }) });
        Ok(())
    }

    pub fn open(&self, id: &str) -> CoreResult<()> {
        self.set_state(id, DeviceState::Opening)?;
        self.set_state(id, DeviceState::Open)
    }

    pub fn close(&self, id: &str) -> CoreResult<()> {
        self.set_state(id, DeviceState::Closing)?;
        self.set_state(id, DeviceState::Closed)
    }

    pub fn get(&self, id: &str) -> Option<DeviceSnapshot> {
        self.devices.read().expect("device manager poisoned").get(id).cloned()
    }

    pub fn list(&self) -> Vec<DeviceSnapshot> {
        self.devices.read().expect("device manager poisoned").values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{DeviceInfo, DeviceKind, DeviceManager, DeviceState};
    use crate::core::event_bus::EventBus;

    fn manager() -> (DeviceManager, std::sync::mpsc::Receiver<crate::core::event_bus::CoreEvent>) {
        let events = EventBus::new();
        let receiver = events.subscribe();
        (DeviceManager::new(events), receiver)
    }

    #[test]
    fn device_open_close_is_typed_and_evented() {
        let (manager, receiver) = manager();
        assert!(manager.register(DeviceInfo { id: "can-1".into(), name: "CAN 1".into(), kind: DeviceKind::Can, vendor: Some("test".into()), path: None }));
        manager.open("can-1").unwrap();
        assert_eq!(manager.get("can-1").unwrap().state, DeviceState::Open);
        manager.close("can-1").unwrap();
        assert_eq!(manager.get("can-1").unwrap().state, DeviceState::Closed);
        assert_eq!(manager.list().len(), 1);
        assert!(receiver.try_iter().count() >= 5);
    }

    #[test]
    fn unknown_device_returns_core_error() {
        let (manager, _) = manager();
        assert!(manager.open("missing").unwrap_err().code == "not_found");
    }
}
