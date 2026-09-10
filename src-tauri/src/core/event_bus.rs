use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CoreEvent {
    pub topic: String,
    pub payload: Value,
}

#[derive(Clone, Default)]
pub struct EventBus {
    subscribers: Arc<Mutex<Vec<Sender<CoreEvent>>>>,
}

impl EventBus {
    pub fn new() -> Self { Self::default() }
    pub fn subscribe(&self) -> Receiver<CoreEvent> {
        let (tx, rx) = mpsc::channel();
        self.subscribers.lock().expect("event bus poisoned").push(tx);
        rx
    }
    pub fn publish(&self, event: CoreEvent) {
        let mut subscribers = self.subscribers.lock().expect("event bus poisoned");
        subscribers.retain(|tx| tx.send(event.clone()).is_ok());
    }
    pub fn subscriber_count(&self) -> usize {
        self.subscribers.lock().expect("event bus poisoned").len()
    }
}
