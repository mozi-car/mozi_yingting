use napi::bindgen_prelude::*;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi_derive::napi;
use std::ffi::CString;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;
use windows_sys::Win32::Foundation::{FreeLibrary, HMODULE};
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA, SetDllDirectoryA};

type Status = i16;
type OpenDriver = unsafe extern "system" fn() -> Status;
type CloseDriver = unsafe extern "system" fn() -> Status;
type Mask = unsafe extern "system" fn(i32, i32, i32) -> u64;
type OpenPort =
    unsafe extern "system" fn(*mut i32, *const i8, u64, *mut u64, u32, u32, u32) -> Status;
type ClosePort = unsafe extern "system" fn(i32) -> Status;
type Activate = unsafe extern "system" fn(i32, u64, u32, u32) -> Status;
type Deactivate = unsafe extern "system" fn(i32, u64) -> Status;
type ReceiveEvent = unsafe extern "system" fn(i32, *mut u32, *mut RawEvent) -> Status;
type CanReceive = unsafe extern "system" fn(i32, *mut RawRx) -> Status;
type Transmit = unsafe extern "system" fn(i32, u64, *mut u32, *mut u8) -> Status;
type TransmitEx = unsafe extern "system" fn(i32, u64, u32, *mut u32, *mut RawTx) -> Status;
type SetParams = unsafe extern "system" fn(i32, u64, *mut RawChip) -> Status;
type SetOutput = unsafe extern "system" fn(i32, u64, i32) -> Status;
type SetMode = unsafe extern "system" fn(i32, u64, i32, i32) -> Status;
type SetReceiveMode = unsafe extern "system" fn(i32, u8, u8) -> Status;
type LinSetParams = unsafe extern "system" fn(i32, u64, RawLinStat) -> Status;
type LinSetDlc = unsafe extern "system" fn(i32, u64, *const u8) -> Status;
type LinSetSlave = unsafe extern "system" fn(i32, u64, u8, *const u8, u8, u16) -> Status;
type LinSendRequest = unsafe extern "system" fn(i32, u64, u8, u32) -> Status;
type LinSwitchSlave = unsafe extern "system" fn(i32, u64, u8, u8) -> Status;
type LinWakeup = unsafe extern "system" fn(i32, u64) -> Status;
type FlushReceiveQueue = unsafe extern "system" fn(i32) -> Status;
type SetTransceiver = unsafe extern "system" fn(i32, u64, i32, i32, i32) -> Status;
type SetFdConfig = unsafe extern "system" fn(i32, u64, *mut RawFdConf) -> Status;
type GetConfig = unsafe extern "system" fn(*mut RawDriverConfig) -> Status;
type ErrorString = unsafe extern "system" fn(Status) -> *const i8;
struct Api(HMODULE);
unsafe impl Send for Api {}
unsafe impl Sync for Api {}
impl Drop for Api {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                FreeLibrary(self.0);
            }
        }
    }
}
fn sym<T: Copy>(a: &Api, n: &[u8]) -> Result<T> {
    let p = unsafe { GetProcAddress(a.0, n.as_ptr() as _) }.ok_or_else(|| {
        Error::from_reason(format!(
            "Vector symbol missing: {}",
            String::from_utf8_lossy(n)
        ))
    })?;
    Ok(unsafe { std::mem::transmute_copy(&p) })
}
static API: OnceLock<Mutex<Option<Api>>> = OnceLock::new();
static CHANNELS: OnceLock<Mutex<Vec<ChannelConfig>>> = OnceLock::new();
fn channels() -> &'static Mutex<Vec<ChannelConfig>> {
    CHANNELS.get_or_init(|| Mutex::new(Vec::new()))
}
fn api() -> &'static Mutex<Option<Api>> {
    API.get_or_init(|| Mutex::new(None))
}
fn load<T: Copy>(n: &[u8]) -> Result<T> {
    let g = api()
        .lock()
        .map_err(|_| Error::from_reason("Vector lock poisoned"))?;
    sym(
        g.as_ref()
            .ok_or_else(|| Error::from_reason("Vector DLL not loaded"))?,
        n,
    )
}
#[napi(js_name = "LoadDll")]
pub fn load_dll(path: String) -> Result<()> {
    let path = if path.to_ascii_lowercase().ends_with(".dll") { path } else { format!("{}\\vxlapi64.dll", path.trim_end_matches(|c| c == '\\' || c == '/')) };
    let p = CString::new(path).map_err(|_| Error::from_reason("NUL path"))?;
    if let Some(parent) = Path::new(p.to_str().unwrap_or_default()).parent() {
        if let Ok(directory) = CString::new(parent.to_string_lossy().as_bytes()) { unsafe { SetDllDirectoryA(directory.as_ptr() as _); } }
    }
    let h = unsafe { LoadLibraryA(p.as_ptr() as _) };
    if h.is_null() {
        return Err(Error::from_reason("failed to load Vector XL DLL"));
    };
    *api()
        .lock()
        .map_err(|_| Error::from_reason("lock poisoned"))? = Some(Api(h));
    Ok(())
}
#[napi(js_name = "dllLoaded")]
pub fn dll_loaded() -> bool {
    api().lock().map(|g| g.is_some()).unwrap_or(false)
}
#[napi(js_name = "driverLoad")]
pub fn driver_load() -> Result<i32> {
    Ok(unsafe { load::<OpenDriver>(b"xlOpenDriver\0")?().into() })
}
#[napi(js_name = "xlOpenDriver")]
pub fn xl_open_driver() -> Result<i32> {
    driver_load()
}
#[napi(js_name = "xlCloseDriver")]
pub fn xl_close_driver() -> Result<i32> {
    Ok(unsafe { load::<CloseDriver>(b"xlCloseDriver\0")?().into() })
}
#[napi(js_name = "xlGetChannelMask")]
pub fn channel_mask(hw: i32, index: i32, ch: i32) -> Result<i64> {
    Ok(unsafe { load::<Mask>(b"xlGetChannelMask\0")?(hw, index, ch) as i64 })
}
#[napi(js_name = "listCanChannels")]
pub fn list_can_channels() -> Result<Vec<ChannelConfig>> {
    let all = channels()
        .lock()
        .map_err(|_| Error::from_reason("channel lock poisoned"))?;
    Ok(all
        .iter()
        .filter(|c| c.hw_type != 1 && (c.channel_bus_capabilities & 0x00010001) == 0x00010001)
        .cloned()
        .collect())
}
#[napi(js_name = "listEthernetChannels")]
pub fn list_ethernet_channels() -> Result<Vec<ChannelConfig>> {
    let mut config = XlDriverConfig::new();
    let status = get_driver_config(&mut config);
    if status != 0 { return Err(Error::from_reason(format!("xlGetDriverConfig failed: {status}"))); }
    Ok(config.channels.into_iter().filter(|channel| (channel.channel_bus_capabilities & 0x0000_1000) != 0).collect())
}
#[napi(js_name = "xlOpenPort")]
pub fn open_port(
    port: &mut XlPortHandle,
    name: String,
    mask: i64,
    permission: &mut XlAccess,
    queue: u32,
    version: u32,
    bus: u32,
) -> Result<i32> {
    let n = CString::new(name).map_err(|_| Error::from_reason("NUL app name"))?;
    let mut p = 0i32;
    let mut perm = permission.value;
    let s = unsafe {
        load::<OpenPort>(b"xlOpenPort\0")?(
            &mut p,
            n.as_ptr(),
            mask as u64,
            (&mut perm as *mut i64).cast::<u64>(),
            queue,
            version,
            bus,
        )
    };
    port.value = p as i64;
    permission.value = perm as i64;
    Ok(s.into())
}
#[napi(js_name = "xlClosePort")]
pub fn close_port(port: i64) -> Result<i32> {
    Ok(unsafe { load::<ClosePort>(b"xlClosePort\0")?(port as i32).into() })
}
#[napi(js_name = "xlActivateChannel")]
pub fn activate(port: i64, mask: i64) -> Result<i32> {
    Ok(unsafe { load::<Activate>(b"xlActivateChannel\0")?(port as i32, mask as u64, 1, 0).into() })
}
#[napi(js_name = "xlDeactivateChannel")]
pub fn deactivate(port: i64, mask: i64) -> Result<i32> {
    Ok(unsafe { load::<Deactivate>(b"xlDeactivateChannel\0")?(port as i32, mask as u64).into() })
}
#[napi(js_name = "xlGetDriverConfig")]
pub fn get_driver_config(config: &mut XlDriverConfig) -> i32 {
    let mut raw = unsafe { std::mem::zeroed::<RawDriverConfig>() };
    let status = unsafe {
        match load::<GetConfig>(b"xlGetDriverConfig\0") {
            Ok(f) => f(&mut raw),
            Err(_) => return -1,
        }
    };
    if status == 0 {
        config.dll_version = raw.dll_version;
        config.channel_count = raw.channel_count.min(64);
        config.channels = raw.channels[..config.channel_count as usize]
            .iter()
            .map(ChannelConfig::from_raw)
            .collect();
        if let Ok(mut all) = channels().lock() {
            *all = config.channels.clone();
        }
        config.channel = 1;
    }
    status as i32
}
#[napi(js_name = "xlReceive")]
pub fn receive(port: i64) -> Result<i32> {
    let mut count = 1u32;
    let mut event = RawEvent::default();
    Ok(
        unsafe {
            load::<ReceiveEvent>(b"xlReceive\0")?(port as i32, &mut count, &mut event).into()
        },
    )
}
#[napi(object)]
pub struct XlLinEvent { pub tag: u8, pub time_stamp: i64, pub id: u8, pub dlc: u8, pub flags: u16, pub data: Vec<u8>, pub crc: u8 }
#[napi(js_name = "receiveLinEvent")]
pub fn receive_lin_event(port: i64) -> Result<XlLinEvent> {
    let mut count = 1u32;
    let mut event = RawEvent::default();
    let status = unsafe { load::<ReceiveEvent>(b"xlReceive\0")?(port as i32, &mut count, &mut event) };
    if status != 0 { return Err(Error::from_reason(format!("xlReceive status {status}"))); }
    let lin = unsafe { &*((&event as *const RawEvent as *const u8).add(16) as *const RawLinMsg) };
    Ok(XlLinEvent { tag: event.tag, time_stamp: event.time_stamp as i64, id: lin.id, dlc: lin.dlc.min(8), flags: lin.flags, data: lin.data[..lin.dlc.min(8) as usize].to_vec(), crc: lin.crc })
}
#[napi(js_name = "receiveClassicEvent")]
pub fn receive_classic_event(port: i64) -> Result<XlEvent> {
    let mut count = 1u32;
    let mut event = RawEvent::default();
    let status =
        unsafe { load::<ReceiveEvent>(b"xlReceive\0")?(port as i32, &mut count, &mut event) };
    if status != 0 {
        return Err(Error::from_reason(format!("xlReceive status {}", status)));
    }
    let n = event.msg.dlc.min(8) as usize;
    Ok(XlEvent {
        tag: event.tag,
        time_stamp: event.time_stamp as i64,
        can_id: event.msg.can_id,
        flags: event.msg.flags as u32,
        dlc: event.msg.dlc as u32,
        data: event.msg.data[..n].to_vec(),
    })
}
#[napi(js_name = "xlCanTransmitEx")]
pub fn can_transmit_ex(
    port: i64,
    mask: i64,
    message_count: u32,
    sent: u32,
    data: &XlCanTxEvent,
) -> Result<i32> {
    if message_count == 0 {
        return Err(Error::from_reason("message_count must be positive"));
    }

    let mut count = sent.max(1).min(message_count);
    let mut raw = RawTx::default();
    raw.tag = data.tag;
    raw.msg.can_id = data.can_id;
    raw.msg.flags = data.msg_flags;
    raw.msg.dlc = data.dlc;
    let length = if data.msg_flags & 0x0001 != 0 {
        match data.dlc {
            0..=8 => data.dlc as usize,
            9 => 12,
            10 => 16,
            11 => 20,
            12 => 24,
            13 => 32,
            14 => 48,
            _ => 64,
        }
    } else {
        data.dlc.min(8) as usize
    };
    raw.msg.data[..length].copy_from_slice(&data.data[..length]);

    Ok(unsafe {
        load::<TransmitEx>(b"xlCanTransmitEx\0")?(
            port as i32,
            mask as u64,
            message_count,
            &mut count,
            &mut raw,
        )
        .into()
    })
}
#[napi(js_name = "xlCanReceive")]
pub fn can_receive(port: i64) -> Result<i32> {
    let mut event = RawRx::default();
    Ok(unsafe { load::<CanReceive>(b"xlCanReceive\0")?(port as i32, &mut event).into() })
}
#[napi(js_name = "receiveCanEvent")]
pub fn receive_can_event(port: i64) -> Result<XlCanRxEvent> {
    let mut event = RawRx::default();
    let status = unsafe { load::<CanReceive>(b"xlCanReceive\0")?(port as i32, &mut event) };
    if status != 0 {
        return Err(Error::from_reason(format!(
            "xlCanReceive status {}",
            status
        )));
    }
    let n = event.msg.dlc.min(64) as usize;
    Ok(XlCanRxEvent {
        tag: event.tag,
        can_id: event.msg.can_id,
        msg_flags: event.msg.flags,
        dlc: event.msg.dlc,
        data: event.msg.data[..n].to_vec(),
        time_stamp_sync: event.timestamp as i64,
    })
}
#[napi(js_name = "xlCanTransmit")]
pub fn can_transmit(port: i64, mask: i64, _event_count: u32, event: &SxEvent) -> Result<i32> {
    let mut count = 1u32;
    let mut raw = RawEvent::default();
    raw.tag = event.tag;
    raw.time_stamp = event.time_stamp as u64;
    raw.msg.can_id = event.can_id;
    raw.msg.dlc = event.dlc as u16;
    raw.msg.flags = event.flags as u16;
    let n = event.data.len().min(8);
    raw.msg.data[..n].copy_from_slice(&event.data[..n]);
    Ok(unsafe {
        load::<Transmit>(b"xlCanTransmit\0")?(
            port as i32,
            mask as u64,
            &mut count,
            (&mut raw as *mut RawEvent).cast::<u8>(),
        )
        .into()
    })
}
#[napi(js_name = "JSxlGetErrorString")]
pub fn error_string(status: i32) -> Result<String> {
    let p = unsafe { load::<ErrorString>(b"xlGetErrorString\0")?(status as i16) };
    if p.is_null() {
        Ok(format!("Vector XL status {status}"))
    } else {
        Ok(unsafe { std::ffi::CStr::from_ptr(p) }
            .to_string_lossy()
            .into_owned())
    }
}
#[napi(js_name = "XLPORTHANDLE")]
pub struct XlPortHandle {
    pub value: i64,
}
#[napi]
impl XlPortHandle {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self { value: 0 }
    }
    #[napi]
    pub fn cast(&self) -> i64 {
        self.value
    }
    #[napi]
    pub fn value(&self) -> i64 {
        self.value
    }
}
#[napi(js_name = "XLACCESS")]
pub struct XlAccess {
    pub value: i64,
}
#[napi]
impl XlAccess {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self { value: 0 }
    }
    #[napi]
    pub fn assign(&mut self, v: i64) {
        self.value = v
    }
    #[napi]
    pub fn cast(&self) -> i64 {
        self.value
    }
    #[napi]
    pub fn value(&self) -> i64 {
        self.value
    }
}
#[napi(js_name = "UINT32")]
pub struct Uint32 {
    pub value: u32,
}
#[napi]
impl Uint32 {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self { value: 0 }
    }
    #[napi]
    pub fn assign(&mut self, value: u32) { self.value = value; }
    #[napi]
    pub fn cast(&self) -> u32 {
        self.value
    }
    #[napi]
    pub fn value(&self) -> u32 {
        self.value
    }
}
#[napi(js_name = "UINT8ARRAY")]
pub struct Uint8Array {
    pub values: Mutex<Vec<u8>>,
}
#[napi]
impl Uint8Array {
    #[napi(constructor)]
    pub fn new(n: u32) -> Self {
        Self {
            values: Mutex::new(vec![0; n as usize]),
        }
    }
    #[napi]
    pub fn cast(&self) -> i64 {
        0
    }
    #[napi]
    pub fn getitem(&self, i: u32) -> Result<u8> {
        Ok(self
            .values
            .lock()
            .map_err(|_| Error::from_reason("lock"))?
            .get(i as usize)
            .copied()
            .unwrap_or(0))
    }
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct RawFdConf {
    arbitration: u32,
    sjw_abr: u32,
    tseg1_abr: u32,
    tseg2_abr: u32,
    data: u32,
    sjw_dbr: u32,
    tseg1_dbr: u32,
    tseg2_dbr: u32,
    reserved: u8,
    options: u8,
    reserved1: [u8; 2],
    reserved2: u32,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct RawChip {
    bit_rate: u32,
    sjw: u8,
    tseg1: u8,
    tseg2: u8,
    sam: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct RawCanMsg {
    can_id: u32,
    flags: u32,
    dlc: u8,
    reserved: [u8; 7],
    data: [u8; 64],
}
impl Default for RawCanMsg {
    fn default() -> Self {
        Self {
            can_id: 0,
            flags: 0,
            dlc: 0,
            reserved: [0; 7],
            data: [0; 64],
        }
    }
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct RawClassicMsg {
    can_id: u32,
    flags: u16,
    dlc: u16,
    res1: u64,
    data: [u8; 8],
    res2: u64,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct RawTx {
    tag: u16,
    trans_id: u16,
    channel: u8,
    reserved: [u8; 3],
    msg: RawCanMsg,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct RawLinMsg { id: u8, dlc: u8, flags: u16, data: [u8; 8], crc: u8, reserved: [u8; 3] }
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct RawEvent {
    tag: u8,
    channel: u8,
    trans_id: u16,
    port: u16,
    flags: u8,
    reserved: u8,
    time_stamp: u64,
    msg: RawClassicMsg,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct RawRx {
    size: u32,
    tag: u16,
    channel: u16,
    user: u32,
    flags: u16,
    reserved: u16,
    reserved1: u64,
    timestamp: u64,
    msg: RawCanMsg,
}
#[repr(C, packed(1))]
#[derive(Clone, Copy)]
struct RawChannel {
    name: [u8; 32],
    hw_type: u8,
    hw_index: u8,
    hw_channel: u8,
    transceiver_type: u16,
    transceiver_state: u16,
    config_error: u16,
    channel_index: u8,
    channel_mask: u64,
    channel_capabilities: u32,
    channel_bus_capabilities: u32,
    is_on_bus: u8,
    connected_bus_type: u32,
    bus_params: [u8; 32],
    do_not_use: u32,
    driver_version: u32,
    interface_version: u32,
    raw_data: [u32; 10],
    serial_number: u32,
    article_number: u32,
    transceiver_name: [u8; 32],
    special_cab_flags: u32,
    dominant_timeout: u32,
    dominant_recessive_delay: u8,
    recessive_dominant_delay: u8,
    connection_info: u8,
    currently_available_timestamps: u8,
    minimal_supply_voltage: u16,
    maximal_supply_voltage: u16,
    maximal_baudrate: u32,
    fpga_core_capabilities: u8,
    special_device_status: u8,
    channel_bus_active_capabilities: u16,
    break_offset: u16,
    delimiter_offset: u16,
    reserved: [u32; 3],
}
#[repr(C, packed(1))]
struct RawDriverConfig {
    dll_version: u32,
    channel_count: u32,
    reserved: [u32; 10],
    channels: [RawChannel; 64],
}
impl Default for RawChannel {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct RawLinStat { lin_mode: u32, baudrate: i32, lin_version: u32, reserved: u32 }

#[napi(js_name = "XLlinStatPar")]
pub struct XlLinStatPar { #[napi(js_name = "LINMode")] pub lin_mode: u32, pub baudrate: i32, #[napi(js_name = "LINVersion")] pub lin_version: u32 }
#[napi]
impl XlLinStatPar { #[napi(constructor)] pub fn new() -> Self { Self { lin_mode: 1, baudrate: 19200, lin_version: 3 } } }
impl XlLinStatPar { fn raw(&self) -> RawLinStat { RawLinStat { lin_mode: self.lin_mode, baudrate: self.baudrate, lin_version: self.lin_version, reserved: 0 } } }

#[napi(js_name = "s_xl_lin_msg")]
pub struct SxLinMsg { pub id: u8, pub dlc: u8, pub flags: u16, pub data: Vec<u8>, pub crc: u8 }
#[napi]
impl SxLinMsg { #[napi(constructor)] pub fn new() -> Self { Self { id: 0, dlc: 0, flags: 0, data: vec![0; 8], crc: 0 } } }

#[napi(js_name = "UINT16")]
pub struct Uint16 { pub value: u16 }
#[napi]
impl Uint16 { #[napi(constructor)] pub fn new() -> Self { Self { value: 0 } } #[napi] pub fn assign(&mut self, value: u16) { self.value = value } #[napi] pub fn value(&self) -> u16 { self.value } }

#[napi(js_name = "XLcanFdConf")]
pub struct XlCanFdConf {
    pub arbitration_bit_rate: u32,
    pub sjw_abr: u32,
    pub tseg1_abr: u32,
    pub tseg2_abr: u32,
    pub data_bit_rate: u32,
    pub sjw_dbr: u32,
    pub tseg1_dbr: u32,
    pub tseg2_dbr: u32,
    pub options: u8,
}
#[napi]
impl XlCanFdConf {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            arbitration_bit_rate: 0,
            sjw_abr: 0,
            tseg1_abr: 0,
            tseg2_abr: 0,
            data_bit_rate: 0,
            sjw_dbr: 0,
            tseg1_dbr: 0,
            tseg2_dbr: 0,
            options: 0,
        }
    }
    #[napi]
    pub fn cast(&self) -> i64 {
        self as *const _ as i64
    }
    fn raw(&self) -> RawFdConf {
        RawFdConf {
            arbitration: self.arbitration_bit_rate,
            sjw_abr: self.sjw_abr,
            tseg1_abr: self.tseg1_abr,
            tseg2_abr: self.tseg2_abr,
            data: self.data_bit_rate,
            sjw_dbr: self.sjw_dbr,
            tseg1_dbr: self.tseg1_dbr,
            tseg2_dbr: self.tseg2_dbr,
            options: self.options,
            ..Default::default()
        }
    }
}
#[napi(js_name = "XLchipParams")]
pub struct XlChipParams {
    pub bit_rate: u32,
    pub sjw: u8,
    pub tseg1: u8,
    pub tseg2: u8,
    pub sam: u8,
}
#[napi]
impl XlChipParams {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            bit_rate: 0,
            sjw: 0,
            tseg1: 0,
            tseg2: 0,
            sam: 0,
        }
    }
    #[napi]
    pub fn cast(&self) -> i64 {
        self as *const _ as i64
    }
    fn raw(&self) -> RawChip {
        RawChip {
            bit_rate: self.bit_rate,
            sjw: self.sjw,
            tseg1: self.tseg1,
            tseg2: self.tseg2,
            sam: self.sam,
        }
    }
}
#[napi(js_name = "XLcanTxEvent")]
pub struct XlCanTxEvent {
    pub tag: u16,
    pub can_id: u32,
    pub msg_flags: u32,
    pub dlc: u8,
    pub data: Vec<u8>,
}
#[napi]
impl XlCanTxEvent {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            tag: 0,
            can_id: 0,
            msg_flags: 0,
            dlc: 0,
            data: vec![0; 64],
        }
    }
    #[napi]
    pub fn cast(&self) -> i64 {
        self as *const _ as i64
    }
}
#[napi(js_name = "XLCANRXEVENT")]
pub struct XlCanRxEvent {
    pub tag: u16,
    pub can_id: u32,
    pub msg_flags: u32,
    pub dlc: u8,
    pub data: Vec<u8>,
    pub time_stamp_sync: i64,
}
#[napi]
impl XlCanRxEvent {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            tag: 0,
            can_id: 0,
            msg_flags: 0,
            dlc: 0,
            data: vec![0; 64],
            time_stamp_sync: 0,
        }
    }
    #[napi]
    pub fn cast(&self) -> i64 {
        self as *const _ as i64
    }
    #[napi]
    pub fn getitem(&self, _i: u32) -> XlCanRxEvent {
        XlCanRxEvent {
            tag: self.tag,
            can_id: self.can_id,
            msg_flags: self.msg_flags,
            dlc: self.dlc,
            data: self.data.clone(),
            time_stamp_sync: self.time_stamp_sync,
        }
    }
}
#[napi(js_name = "XL_DRIVER_CONFIG")]
pub struct XlDriverConfig {
    pub dll_version: u32,
    pub channel_count: u32,
    pub channel: i64,
    channels: Vec<ChannelConfig>,
}
#[napi]
impl XlDriverConfig {
    #[napi(getter, js_name = "dllVersion")]
    pub fn dll_version_get(&self) -> u32 {
        self.dll_version
    }
    #[napi(getter, js_name = "channelCount")]
    pub fn channel_count_get(&self) -> u32 {
        self.channel_count
    }
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            dll_version: 0,
            channel_count: 0,
            channel: 0,
            channels: Vec::new(),
        }
    }
    #[napi]
    pub fn cast(&self) -> i64 {
        self as *const _ as i64
    }
    #[napi]
    pub fn getitem(&self, index: u32) -> Result<ChannelConfig> {
        self.channels
            .get(index as usize)
            .cloned()
            .ok_or_else(|| Error::from_reason("channel index out of range"))
    }
}
#[napi]
#[derive(Clone)]
pub struct ChannelConfig {
    pub name: String,
    pub hw_type: u32,
    pub hw_index: u32,
    pub hw_channel: u32,
    pub channel_index: u32,
    pub channel_mask: i64,
    pub transceiver_type: u32,
    pub config_error: u32,
    pub channel_capabilities: u32,
    pub channel_bus_capabilities: u32,
    pub channel_bus_active_capabilities: u32,
    pub is_on_bus: bool,
    pub connected_bus_type: u32,
    pub serial_number: u32,
    pub bus_type: u32,
}
#[napi]
impl ChannelConfig {
    #[napi(getter, js_name = "name")]
    pub fn name_get(&self) -> String {
        self.name.clone()
    }
    #[napi(getter, js_name = "hwType")]
    pub fn hw_type_get(&self) -> u32 {
        self.hw_type
    }
    #[napi(getter, js_name = "hwIndex")]
    pub fn hw_index_get(&self) -> u32 {
        self.hw_index
    }
    #[napi(getter, js_name = "hwChannel")]
    pub fn hw_channel_get(&self) -> u32 {
        self.hw_channel
    }
    #[napi(getter, js_name = "channelCapabilities")]
    pub fn caps_get(&self) -> u32 {
        self.channel_capabilities
    }
    #[napi(getter, js_name = "channelIndex")]
    pub fn channel_index_get(&self) -> u32 {
        self.channel_index
    }
    #[napi(getter, js_name = "channelMask")]
    pub fn channel_mask_get(&self) -> i64 {
        self.channel_mask
    }
    #[napi(getter, js_name = "transceiverType")]
    pub fn transceiver_type_get(&self) -> u32 {
        self.transceiver_type
    }
    #[napi(getter, js_name = "configError")]
    pub fn config_error_get(&self) -> u32 {
        self.config_error
    }
    #[napi(getter, js_name = "channelBusCapabilities")]
    pub fn bus_caps_get(&self) -> u32 {
        self.channel_bus_capabilities
    }
    #[napi(getter, js_name = "channelBusActiveCapabilities")]
    pub fn active_bus_caps_get(&self) -> u32 {
        self.channel_bus_active_capabilities
    }
    #[napi(getter, js_name = "isOnBus")]
    pub fn is_on_bus_get(&self) -> bool {
        self.is_on_bus
    }
    #[napi(getter, js_name = "connectedBusType")]
    pub fn connected_bus_type_get(&self) -> u32 {
        self.connected_bus_type
    }
    #[napi(getter, js_name = "serialNumber")]
    pub fn serial_get(&self) -> u32 {
        self.serial_number
    }
    #[napi(getter, js_name = "busParams")]
    pub fn bus_params_get(&self) -> BusParams {
        BusParams {
            bus_type: self.bus_type,
        }
    }
    #[napi(js_name = "frompointer")]
    pub fn from_pointer(p: i64) -> i64 {
        p
    }
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            name: String::new(),
            hw_type: 0,
            hw_index: 0,
            hw_channel: 0,
            channel_index: 0,
            channel_mask: 0,
            transceiver_type: 0,
            config_error: 0,
            channel_capabilities: 0,
            channel_bus_capabilities: 0,
            channel_bus_active_capabilities: 0,
            is_on_bus: false,
            connected_bus_type: 0,
            serial_number: 0,
            bus_type: 0,
        }
    }
    fn from_raw(raw: &RawChannel) -> Self {
        let end = raw
            .name
            .iter()
            .position(|x| *x == 0)
            .unwrap_or(raw.name.len());
        Self {
            name: String::from_utf8_lossy(&raw.name[..end]).into_owned(),
            hw_type: raw.hw_type as u32,
            hw_index: raw.hw_index as u32,
            hw_channel: raw.hw_channel as u32,
            channel_index: raw.channel_index as u32,
            channel_mask: raw.channel_mask as i64,
            transceiver_type: raw.transceiver_type as u32,
            config_error: raw.config_error as u32,
            channel_capabilities: raw.channel_capabilities,
            channel_bus_capabilities: raw.channel_bus_capabilities,
            channel_bus_active_capabilities: raw.channel_bus_active_capabilities as u32,
            is_on_bus: raw.is_on_bus != 0,
            connected_bus_type: raw.connected_bus_type,
            serial_number: raw.serial_number,
            bus_type: if (raw.channel_bus_capabilities & 0x0001) != 0 { 1 } else if (raw.channel_bus_capabilities & 0x0002) != 0 { 2 } else if (raw.channel_bus_capabilities & 0x0000_1000) != 0 { 3 } else { raw.connected_bus_type },
        }
    }
}
pub struct ChannelConfigArray {
    pub values: Vec<ChannelConfig>,
}
#[napi(js_name = "XLEVENT")]
pub struct XlEvent {
    pub tag: u8,
    pub time_stamp: i64,
    pub can_id: u32,
    pub flags: u32,
    pub dlc: u32,
    pub data: Vec<u8>,
}
#[napi]
impl XlEvent {
    #[napi(constructor)]
    pub fn new(_n: Option<u32>) -> Self {
        Self {
            tag: 0,
            time_stamp: 0,
            can_id: 0,
            flags: 0,
            dlc: 0,
            data: vec![0; 8],
        }
    }
    #[napi]
    pub fn cast(&self) -> i64 {
        self as *const _ as i64
    }
    #[napi]
    pub fn getitem(&self, _i: u32) -> XlEvent {
        XlEvent {
            tag: self.tag,
            time_stamp: self.time_stamp,
            can_id: self.can_id,
            flags: self.flags,
            dlc: self.dlc,
            data: self.data.clone(),
        }
    }
}
#[napi(js_name = "s_xl_event")]
pub struct SxEvent {
    pub tag: u8,
    pub time_stamp: i64,
    pub can_id: u32,
    pub flags: u8,
    pub dlc: u8,
    pub data: Vec<u8>,
}
#[napi]
impl SxEvent {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            tag: 10,
            time_stamp: 0,
            can_id: 0,
            flags: 0,
            dlc: 0,
            data: vec![0; 8],
        }
    }
    #[napi]
    pub fn cast(&self) -> i64 {
        self as *const _ as i64
    }
}
#[napi(object)]
#[derive(Clone)]
pub struct BusParams {
    pub bus_type: u32,
}
#[napi(js_name = "xlFlushReceiveQueue")]
pub fn flush_receive_queue(port: i64) -> Result<i32> { Ok(unsafe { load::<FlushReceiveQueue>(b"xlFlushReceiveQueue\0")?(port as i32).into() }) }
#[napi(js_name = "xlLinSetChannelParams")]
pub fn lin_set_channel_params(port: i64, mask: i64, params: &XlLinStatPar) -> Result<i32> { Ok(unsafe { load::<LinSetParams>(b"xlLinSetChannelParams\0")?(port as i32, mask as u64, params.raw()).into() }) }
#[napi(js_name = "xlLinSetDLC")]
pub fn lin_set_dlc(port: i64, mask: i64, dlc: &Uint8Array) -> Result<i32> { let values = dlc.values.lock().map_err(|_| Error::from_reason("LIN DLC lock poisoned"))?; let mut raw = [0u8; 64]; raw[..values.len().min(64)].copy_from_slice(&values[..values.len().min(64)]); Ok(unsafe { load::<LinSetDlc>(b"xlLinSetDLC\0")?(port as i32, mask as u64, raw.as_ptr()).into() }) }
#[napi(js_name = "xlLinSetSlave")]
pub fn lin_set_slave(port: i64, mask: i64, id: u8, data: Vec<u8>, dlc: u8, checksum: u16) -> Result<i32> { let mut raw = [0u8; 8]; raw[..data.len().min(8)].copy_from_slice(&data[..data.len().min(8)]); Ok(unsafe { load::<LinSetSlave>(b"xlLinSetSlave\0")?(port as i32, mask as u64, id, raw.as_ptr(), dlc, checksum).into() }) }
#[napi(js_name = "xlLinSwitchSlave")]
pub fn lin_switch_slave(port: i64, mask: i64, id: u8, mode: u8) -> Result<i32> { Ok(unsafe { load::<LinSwitchSlave>(b"xlLinSwitchSlave\0")?(port as i32, mask as u64, id, mode).into() }) }
#[napi(js_name = "xlLinSendRequest")]
pub fn lin_send_request(port: i64, mask: i64, id: u8, flags: u32) -> Result<i32> { Ok(unsafe { load::<LinSendRequest>(b"xlLinSendRequest\0")?(port as i32, mask as u64, id, flags).into() }) }
#[napi(js_name = "wakeup")]
pub fn lin_wakeup(port: i64, mask: i64) -> Result<i32> { Ok(unsafe { load::<LinWakeup>(b"xlLinWakeUp\0")?(port as i32, mask as u64).into() }) }

#[napi(js_name = "xlCanFdSetConfiguration")]
pub fn fd_config(p: i64, m: i64, c: &XlCanFdConf) -> Result<i32> {
    let mut raw = c.raw();
    Ok(unsafe {
        load::<SetFdConfig>(b"xlCanFdSetConfiguration\0")?(p as i32, m as u64, &mut raw).into()
    })
}
#[napi(js_name = "xlCanSetChannelParams")]
pub fn channel_params(p: i64, m: i64, c: &XlChipParams) -> Result<i32> {
    let mut raw = c.raw();
    Ok(unsafe {
        load::<SetParams>(b"xlCanSetChannelParams\0")?(p as i32, m as u64, &mut raw).into()
    })
}
#[napi(js_name = "xlCanSetChannelOutput")]
pub fn channel_output(p: i64, m: i64, v: i32) -> Result<i32> {
    Ok(unsafe { load::<SetOutput>(b"xlCanSetChannelOutput\0")?(p as i32, m as u64, v).into() })
}
#[napi(js_name = "xlCanSetChannelMode")]
pub fn channel_mode(p: i64, m: i64, a: i32, b: i32) -> Result<i32> {
    Ok(unsafe { load::<SetMode>(b"xlCanSetChannelMode\0")?(p as i32, m as u64, a, b).into() })
}
#[napi(js_name = "xlCanSetChannelTransceiver")]
pub fn transceiver(p: i64, m: i64, t: i32, line: i32, res: i32) -> Result<i32> {
    Ok(unsafe {
        load::<SetTransceiver>(b"xlCanSetChannelTransceiver\0")?(p as i32, m as u64, t, line, res)
            .into()
    })
}
#[napi(js_name = "xlCanSetReceiveMode")]
pub fn receive_mode(p: i64, error: u8, chip: u8) -> Result<i32> {
    Ok(unsafe { load::<SetReceiveMode>(b"xlSetReceiveMode\0")?(p as i32, error, chip).into() })
}
struct Period {
    stop: Arc<AtomicBool>,
    data: Arc<Mutex<Vec<u8>>>,
    join: Option<std::thread::JoinHandle<()>>,
}
static PERIODS: OnceLock<Mutex<std::collections::HashMap<String, Period>>> = OnceLock::new();
static CALLBACKS: OnceLock<
    Mutex<std::collections::HashMap<String, (Arc<AtomicBool>, std::thread::JoinHandle<()>)>>,
> = OnceLock::new();
fn periods() -> &'static Mutex<std::collections::HashMap<String, Period>> {
    PERIODS.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}
fn callbacks(
) -> &'static Mutex<std::collections::HashMap<String, (Arc<AtomicBool>, std::thread::JoinHandle<()>)>>
{
    CALLBACKS.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}
#[napi(js_name = "ZCAN_StartCAN")]
pub fn zcan_start() -> Result<i32> {
    Err(Error::from_reason(
        "ZCAN_StartCAN is not part of Vector XL API",
    ))
}
#[napi(js_name = "ZCAN_ResetCAN")]
pub fn zcan_reset() -> Result<i32> {
    Err(Error::from_reason(
        "ZCAN_ResetCAN is not part of Vector XL API",
    ))
}
#[napi(js_name = "CreateTSFN")]
pub fn create_tsfn(
    port: i64,
    name: String,
    callback: Function<'static>,
    mask: i64,
    canfd: bool,
) -> Result<()> {
    if !dll_loaded() { return Err(Error::from_reason("Vector XL DLL is not loaded")); }
    let tsfn: ThreadsafeFunction<()> = callback
        .build_threadsafe_function()
        .callee_handled()
        .build_callback(|_| Ok(()))?;
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let _ = (port, mask, canfd);
    let worker = thread::spawn(move || {
        // The JS callback owns the receive call. Polling the vendor queue here
        // would consume the event before either CAN or LIN TypeScript handler.
        while !flag.load(Ordering::Acquire) {
            let _ = tsfn.call(Ok(()), ThreadsafeFunctionCallMode::NonBlocking);
            thread::sleep(Duration::from_millis(2));
        }
    });
    let mut map = callbacks()
        .lock()
        .map_err(|_| Error::from_reason("callback lock poisoned"))?;
    if let Some((old, join)) = map.remove(&name) {
        old.store(true, Ordering::Release);
        let _ = join.join();
    }
    map.insert(name, (stop, worker));
    Ok(())
}
#[napi(js_name = "FreeTSFN")]
pub fn free_tsfn(name: String) -> Result<()> {
    if let Some((stop, join)) = callbacks()
        .lock()
        .map_err(|_| Error::from_reason("callback lock poisoned"))?
        .remove(&name)
    {
        stop.store(true, Ordering::Release);
        let _ = join.join();
    }
    Ok(())
}
#[napi(js_name = "StartPeriodSend")]
pub fn start_period(name: String, message: Object, period: f64, duration: f64) -> Result<String> {
    if !dll_loaded() {
        return Err(Error::from_reason("Vector XL DLL is not loaded"));
    }
    if !period.is_finite() || period <= 0.0 {
        return Err(Error::from_reason("period must be positive"));
    }
    let port: i64 = message.get("port")?.unwrap_or(0);
    let mask: i64 = message.get("mask")?.unwrap_or(0);
    if port < 0 || mask == 0 { return Err(Error::from_reason("Vector periodic send requires an open port and channel mask")); }
    let can_id: u32 = message.get("id")?.unwrap_or(0);
    let extend: bool = message.get("extendId")?.unwrap_or(false);
    let remote: bool = message.get("remoteFrame")?.unwrap_or(false);
    let canfd: bool = message.get("canfd")?.unwrap_or(false);
    let brs: bool = message.get("brs")?.unwrap_or(false);
    let id = uuid::Uuid::new_v4().to_string();
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let data = Arc::new(Mutex::new(
        message.get::<Vec<u8>>("data")?.unwrap_or_default(),
    ));
    let shared = data.clone();
    let deadline = if duration > 0.0 {
        Some(std::time::Instant::now() + Duration::from_secs_f64(duration))
    } else {
        None
    };
    let join = thread::spawn(move || {
        while !flag.load(Ordering::Acquire) {
            if deadline
                .map(|d| std::time::Instant::now() >= d)
                .unwrap_or(false)
            {
                break;
            }
            let bytes = shared.lock().map(|v| v.clone()).unwrap_or_default();
            let mut id_value = can_id;
            if extend { id_value |= 0x8000_0000; }
            if remote { id_value |= 0x4000_0000; }
            if canfd {
                let mut ev = XlCanTxEvent::new();
                ev.tag = 0x0440;
                ev.can_id = id_value;
                ev.msg_flags = 0x0001 | if brs { 0x0002 } else { 0 } | if remote { 0x0010 } else { 0 };
                ev.dlc = match bytes.len() { 0..=8 => bytes.len() as u8, 9..=12 => 9, 13..=16 => 10, 17..=20 => 11, 21..=24 => 12, 25..=32 => 13, 33..=48 => 14, _ => 15 };
                ev.data = bytes;
                let _ = can_transmit_ex(port, mask, 1, 1, &ev);
            } else {
                let mut ev = SxEvent::new();
                ev.can_id = id_value;
                ev.flags = if remote { 0x10 } else { 0 };
                ev.data = bytes;
                ev.dlc = ev.data.len().min(8) as u8;
                let _ = can_transmit(port, mask, 1, &ev);
            }
            thread::sleep(Duration::from_secs_f64(period));
        }
    });
    periods()
        .lock()
        .map_err(|_| Error::from_reason("period lock poisoned"))?
        .insert(
            id.clone(),
            Period {
                stop,
                data,
                join: Some(join),
            },
        );
    let _ = name;
    Ok(id)
}
#[napi(js_name = "StopPeriodSend")]
pub fn stop_period(id: String) -> Result<()> {
    if let Some(mut p) = periods()
        .lock()
        .map_err(|_| Error::from_reason("period lock poisoned"))?
        .remove(&id)
    {
        p.stop.store(true, Ordering::Release);
        if let Some(j) = p.join.take() {
            let _ = j.join();
        }
        Ok(())
    } else {
        Err(Error::from_reason("period task not found"))
    }
}
#[napi(js_name = "ChangeData")]
pub fn change_data(id: String, data: Vec<u8>) -> Result<()> {
    let map = periods()
        .lock()
        .map_err(|_| Error::from_reason("period lock poisoned"))?;
    let p = map
        .get(&id)
        .ok_or_else(|| Error::from_reason("period task not found"))?;
    *p.data
        .lock()
        .map_err(|_| Error::from_reason("period data lock poisoned"))? = data;
    Ok(())
}
