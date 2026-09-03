use crate::device::{DeviceBackend, DeviceRef};
use crate::frame::{CandleBittiming, CandleFrame};
use napi::bindgen_prelude::*;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi_derive::napi;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

struct CallbackTask {
    stop: Arc<std::sync::atomic::AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}
#[napi(object)]
pub struct CandleReceived {
    #[napi(js_name = "ID")]
    pub id: u32,
    #[napi(js_name = "FrameType")]
    pub frame_type: u8,
    #[napi(js_name = "Flags")]
    pub flags: u8,
    #[napi(js_name = "Data")]
    pub data: Buffer,
    #[napi(js_name = "TimeStamp")]
    pub timestamp: u32,
}

static CALLBACKS: OnceLock<Mutex<HashMap<String, CallbackTask>>> = OnceLock::new();
fn callbacks() -> &'static Mutex<HashMap<String, CallbackTask>> {
    CALLBACKS.get_or_init(|| Mutex::new(HashMap::new()))
}

static DEVICES: OnceLock<Mutex<HashMap<String, DeviceRef>>> = OnceLock::new();
fn devices() -> &'static Mutex<HashMap<String, DeviceRef>> {
    DEVICES.get_or_init(|| Mutex::new(HashMap::new()))
}
fn device_from_object(device: &Object) -> Result<DeviceRef> {
    let path: String = device.get("path")?.unwrap_or_default();
    devices()
        .lock()
        .map_err(|_| Error::from_reason("device registry poisoned"))?
        .get(&path)
        .cloned()
        .ok_or_else(|| Error::from_reason("device is not registered"))
}

#[napi(object)]
#[derive(Clone)]
pub struct DeviceInfo {
    pub path: String,
    pub friendly_name: String,
    #[napi(js_name = "interfaceNumber")]
    pub interface_number: u8,
}

#[napi(js_name = "scanDevices")]
pub fn scan_devices() -> Result<Vec<DeviceInfo>> {
    let found = crate::device::scan_devices().map_err(Error::from_reason)?;
    let mut registry = devices()
        .lock()
        .map_err(|_| Error::from_reason("device registry poisoned"))?;
    registry.clear();
    let mut result = Vec::with_capacity(found.len());
    for item in found {
        let info = DeviceInfo {
            path: item.path.clone(),
            friendly_name: item.friendly_name.clone(),
            interface_number: item.interface_number,
        };
        registry.insert(item.path.clone(), Arc::new(Mutex::new(item)));
        result.push(info);
    }
    Ok(result)
}
#[napi(js_name = "candle_bittiming_t")]
pub struct candle_bittiming_t {
    #[napi]
    pub prop_seg: u32,
    pub phase_seg1: u32,
    pub phase_seg2: u32,
    pub sjw: u32,
    pub brp: u32,
}
#[napi]
impl candle_bittiming_t {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            prop_seg: 0,
            phase_seg1: 0,
            phase_seg2: 0,
            sjw: 0,
            brp: 0,
        }
    }
}

#[napi(js_name = "candle_frame_t")]
pub struct candle_frame_t {
    pub can_id: u32,
    pub can_dlc: u8,
    pub channel: u8,
    pub flags: u8,
    pub data: Vec<u8>,
}
#[napi]
impl candle_frame_t {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            can_id: 0,
            can_dlc: 0,
            channel: 0,
            flags: 0,
            data: vec![0; 64],
        }
    }
}

#[napi(js_name = "candle_list_t")]
pub struct candle_list_t {
    #[napi]
    pub num_devices: u8,
}
#[napi]
impl candle_list_t {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self { num_devices: 0 }
    }
}

#[napi]
pub struct Uint8Array {
    pub data: Vec<u8>,
}
#[napi]
impl Uint8Array {
    #[napi(constructor)]
    pub fn new(length: u32) -> Self {
        Self {
            data: vec![0; length as usize],
        }
    }
    #[napi]
    pub fn setitem(&mut self, index: u32, value: u8) {
        if let Some(item) = self.data.get_mut(index as usize) {
            *item = value;
        }
    }
    #[napi]
    pub fn getitem(&self, index: u32) -> u8 {
        self.data.get(index as usize).copied().unwrap_or(0)
    }
    #[napi]
    pub fn cast(&self) -> Buffer {
        Buffer::from(self.data.clone())
    }
}

#[napi(js_name = "candle_list_scan")]
pub fn candle_list_scan(list: &mut candle_list_t) -> bool {
    list.num_devices = 0;
    true
}
#[napi(js_name = "candle_dev_open")]
pub fn candle_dev_open(device: Object) -> bool {
    device_from_object(&device)
        .and_then(|d| DeviceBackend::open(d).map_err(Error::from_reason))
        .is_ok()
}
#[napi(js_name = "candle_dev_close")]
pub fn candle_dev_close(device: Object) -> bool {
    device_from_object(&device)
        .and_then(|d| {
            DeviceBackend { device: d }
                .close()
                .map_err(Error::from_reason)
        })
        .is_ok()
}
#[napi(js_name = "candle_channel_set_timing")]
pub fn candle_channel_set_timing(device: Object, channel: u8, timing: &candle_bittiming_t) -> bool {
    let raw = CandleBittiming {
        prop_seg: timing.prop_seg,
        phase_seg1: timing.phase_seg1,
        phase_seg2: timing.phase_seg2,
        sjw: timing.sjw,
        brp: timing.brp,
    };
    device_from_object(&device)
        .and_then(|d| {
            DeviceBackend { device: d }
                .set_timing(channel, &raw)
                .map_err(Error::from_reason)
        })
        .is_ok()
}
#[napi(js_name = "candle_channel_set_data_timing")]
pub fn candle_channel_set_data_timing(
    device: Object,
    channel: u8,
    timing: &candle_bittiming_t,
) -> bool {
    let raw = CandleBittiming {
        prop_seg: timing.prop_seg,
        phase_seg1: timing.phase_seg1,
        phase_seg2: timing.phase_seg2,
        sjw: timing.sjw,
        brp: timing.brp,
    };
    device_from_object(&device)
        .and_then(|d| {
            DeviceBackend { device: d }
                .set_data_timing(channel, &raw)
                .map_err(Error::from_reason)
        })
        .is_ok()
}
#[napi(js_name = "candle_channel_start")]
pub fn candle_channel_start(device: Object, channel: u8, flags: u32) -> bool {
    device_from_object(&device)
        .and_then(|d| {
            DeviceBackend { device: d }
                .start_channel(channel, flags)
                .map_err(Error::from_reason)
        })
        .is_ok()
}
#[napi(js_name = "candle_channel_stop")]
pub fn candle_channel_stop(device: Object, channel: u8) -> bool {
    device_from_object(&device)
        .and_then(|d| {
            DeviceBackend { device: d }
                .stop_channel(channel)
                .map_err(Error::from_reason)
        })
        .is_ok()
}
#[napi(js_name = "candle_channel_set_interfacenumber_endpoints")]
pub fn candle_channel_set_interfacenumber_endpoints(device: Object, channel: u8) -> bool {
    device_from_object(&device)
        .and_then(|d| {
            DeviceBackend { device: d }
                .set_endpoints(channel)
                .map_err(Error::from_reason)
        })
        .is_ok()
}
#[napi(js_name = "candle_channel_get_can_resister_enable_state")]
pub fn candle_channel_get_can_resister_enable_state(
    device: Object,
    channel: u8,
    mut value: Buffer,
) -> bool {
    device_from_object(&device)
        .and_then(|d| DeviceBackend { device: d }.termination(channel, None).map_err(Error::from_reason))
        .map(|enabled| {
            if !value.is_empty() { value[0] = u8::from(enabled); }
        })
        .is_ok()
}
#[napi(js_name = "candle_channel_set_can_resister_enable_state")]
pub fn candle_channel_set_can_resister_enable_state(
    device: Object,
    channel: u8,
    value: Buffer,
) -> bool {
    let enabled = value.first().copied().unwrap_or(0) != 0;
    device_from_object(&device)
        .and_then(|d| DeviceBackend { device: d }.termination(channel, Some(enabled)).map_err(Error::from_reason))
        .is_ok()
}
#[napi(js_name = "candle_dev_get_timestamp_us")]
pub fn candle_dev_get_timestamp_us(device: Object, mut timestamp: Buffer) -> bool {
    device_from_object(&device)
        .and_then(|d| {
            DeviceBackend { device: d }
                .timestamp()
                .map_err(Error::from_reason)
        })
        .map(|value| {
            for (index, byte) in value.to_le_bytes().iter().enumerate() {
                if index < timestamp.len() { timestamp[index] = *byte; }
            }
        })
        .is_ok()
}
#[napi(js_name = "SetContextDevice")]
pub fn SetContextDevice(name: String, device: Object) -> bool {
    let path: String = device.get("path").ok().flatten().unwrap_or_default();
    devices()
        .lock()
        .map(|mut map| {
            if let Some(value) = map.remove(&path) {
                map.insert(name, value);
                true
            } else {
                false
            }
        })
        .unwrap_or(false)
}
#[napi(js_name = "SendCANMsg")]
pub fn SendCANMsg(name: String, channel: u8, frame: &candle_frame_t) -> bool {
    let result = devices()
        .lock()
        .ok()
        .and_then(|registry| registry.get(&name).cloned())
        .and_then(|device| {
            let mut raw = CandleFrame::default();
            raw.can_id = frame.can_id;
            raw.can_dlc = frame.can_dlc;
            raw.flags = frame.flags;
            let length = crate::frame::data_length(frame.can_dlc).min(frame.data.len()).min(64);
            raw.data[..length].copy_from_slice(&frame.data[..length]);
            DeviceBackend { device }.send(channel, &raw).ok()
        });
    result.is_some()
}
#[napi(js_name = "GetDevicePath")]
pub fn GetDevicePath(device: Object) -> String {
    device.get("path").ok().flatten().unwrap_or_default()
}
#[napi(js_name = "GetDeviceFriendlyName")]
pub fn GetDeviceFriendlyName(device: Object) -> String {
    device
        .get("friendlyName")
        .ok()
        .flatten()
        .unwrap_or_default()
}
#[napi(js_name = "CreateTSFN")]
pub fn CreateTSFN(
    channel: u8,
    name: String,
    callback: Function<'static>,
    _error_callback: Function<'static>,
) -> Result<()> {
    let tsfn: ThreadsafeFunction<CandleReceived> = callback
        .build_threadsafe_function()
        .callee_handled()
        .build_callback(|ctx| Ok(ctx.value))?;
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = stop.clone();
    let device = devices()
        .lock()
        .map_err(|_| Error::from_reason("device registry poisoned"))?
        .get(&name)
        .cloned()
        .ok_or_else(|| Error::from_reason("device not found"))?;
    let worker = thread::spawn(move || {
        while !flag.load(std::sync::atomic::Ordering::Acquire) {
            if let Ok(backend) = (DeviceBackend {
                device: device.clone(),
            })
            .receive(100)
            {
                if let Some(mut frame) = backend {
                    frame.channel = channel;
                    let length = crate::frame::data_length(frame.can_dlc).min(frame.data.len());
                    let received = CandleReceived {
                        id: frame.can_id,
                        frame_type: if frame.echo_id != 0 { 2 } else { 1 },
                        flags: frame.flags,
                        data: Buffer::from(frame.data[..length].to_vec()),
                        timestamp: frame.timestamp_us,
                    };
                    let _ = tsfn.call(Ok(received), ThreadsafeFunctionCallMode::NonBlocking);
                }
            }
            thread::sleep(Duration::from_millis(1));
        }
    });
    callbacks()
        .lock()
        .map_err(|_| Error::from_reason("callback registry poisoned"))?
        .insert(
            name,
            CallbackTask {
                stop,
                worker: Some(worker),
            },
        );
    Ok(())
}
#[napi(js_name = "FreeTSFN")]
pub fn FreeTSFN(name: String) -> Result<()> {
    if let Some(mut task) = callbacks()
        .lock()
        .map_err(|_| Error::from_reason("callback registry poisoned"))?
        .remove(&name)
    {
        task.stop.store(true, std::sync::atomic::Ordering::Release);
        if let Some(worker) = task.worker.take() {
            let _ = worker.join();
        }
    }
    Ok(())
}
