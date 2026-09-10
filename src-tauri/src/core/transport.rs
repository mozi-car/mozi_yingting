use crate::core::error::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::mpsc::Receiver;

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TransportKind {
    Can,
    CanFd,
    Lin,
    Serial,
    Doip,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct OpenConfig {
    pub device_id: String,
    pub kind: Option<TransportKind>,
    pub parameters: BTreeMap<String, String>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TransportHandle(pub u64);

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Frame {
    pub channel: u8,
    pub id: u32,
    pub data: Vec<u8>,
    pub timestamp_us: Option<u64>,
    pub flags: u32,
}

pub trait Transport: Send + Sync {
    fn kind(&self) -> TransportKind;
    fn open(&self, config: &OpenConfig) -> CoreResult<TransportHandle>;
    fn close(&self, handle: TransportHandle) -> CoreResult<()>;
    fn send(&self, handle: TransportHandle, frame: Frame) -> CoreResult<()>;
    fn subscribe(&self, handle: TransportHandle) -> CoreResult<Receiver<Frame>>;
}

#[derive(Default)]
pub struct TransportRegistry;

impl TransportRegistry {
    pub fn new() -> Self {
        Self
    }

    pub fn unsupported(&self, kind: TransportKind) -> CoreError {
        CoreError::new(
            "transport_backend_missing",
            format!("no Rust Core backend registered for {kind:?}"),
        )
    }
}
