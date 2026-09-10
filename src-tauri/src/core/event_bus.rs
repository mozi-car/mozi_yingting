use crate::core::device::{DeviceInfo, DeviceState};
use crate::core::transport::Frame;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub enum CoreEvent {
    DeviceAdded(DeviceInfo),
    DeviceRemoved { id: String },
    DeviceStateChanged { id: String, state: DeviceState },
    Frame(Frame),
    Diagnostic { correlation_id: String, payload: Vec<u8> },
    Error { code: String, message: String },
}

#[derive(Clone, Default)]
pub struct EventBus {
    subscribers: Arc<Mutex<Vec<Sender<CoreEvent>>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn subscribe(&self) -> Receiver<CoreEvent> {
        let (sender, receiver) = mpsc::channel();
        self.subscribers
            .lock()
            .expect("event bus poisoned")
            .push(sender);
        receiver
    }

    pub fn publish(&self, event: CoreEvent) {
        let mut subscribers = self.subscribers.lock().expect("event bus poisoned");
        subscribers.retain(|sender| sender.send(event.clone()).is_ok());
    }

    pub fn subscriber_count(&self) -> usize {
        self.subscribers.lock().expect("event bus poisoned").len()
    }
}
