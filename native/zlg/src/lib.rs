use napi::bindgen_prelude::*;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi_derive::napi;
use std::collections::HashMap;
use std::ffi::CString;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::JoinHandle;
use std::time::Duration;
use windows_sys::Win32::Foundation::{FreeLibrary, HMODULE};
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA, SetDllDirectoryA};

type Device = usize;
type Channel = usize;
type OpenDevice = unsafe extern "system" fn(u32, u32, u32) -> Device;
type CloseDevice = unsafe extern "system" fn(Device) -> u32;
type InitCan = unsafe extern "system" fn(Device, u32, *const u8) -> Channel;
type ChannelFn = unsafe extern "system" fn(Channel) -> u32;
type SetValue = unsafe extern "system" fn(Device, *const i8, *const i8) -> u32;
type Transmit = unsafe extern "system" fn(Channel, *const CanTransmit, u32) -> u32;
type TransmitFd = unsafe extern "system" fn(Channel, *const CanTransmitFd, u32) -> u32;
type Receive = unsafe extern "system" fn(Channel, *mut CanReceive, u32, i32) -> u32;
type ReceiveFd = unsafe extern "system" fn(Channel, *mut CanReceiveFd, u32, i32) -> u32;
type ReadErr = unsafe extern "system" fn(Channel, *mut ChannelErr) -> u32;
type GetReceiveNum = unsafe extern "system" fn(Channel, u8) -> u32;

#[repr(C)]
#[derive(Clone, Copy)]
struct CanFrame {
    id: u32,
    dlc: u8,
    pad: u8,
    r0: u8,
    r1: u8,
    data: [u8; 8],
}
#[repr(C)]
#[derive(Clone, Copy)]
struct CanFdFrame {
    id: u32,
    len: u8,
    flags: u8,
    r0: u8,
    r1: u8,
    data: [u8; 64],
}
#[repr(C)]
struct CanTransmit {
    frame: CanFrame,
    transmit_type: u32,
}
#[repr(C)]
struct CanTransmitFd {
    frame: CanFdFrame,
    transmit_type: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct CanReceive {
    frame: CanFrame,
    timestamp: u64,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct CanReceiveFd {
    frame: CanFdFrame,
    timestamp: u64,
}
#[repr(C)]
#[derive(Default)]
struct ChannelErr {
    error_code: u32,
    passive: [u8; 3],
    lost: u8,
}

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
fn proc<T: Copy>(api: &Api, name: &[u8]) -> Result<T> {
    let p = unsafe { GetProcAddress(api.0, name.as_ptr() as _) }.ok_or_else(|| {
        Error::from_reason(format!(
            "ZLG symbol missing: {}",
            String::from_utf8_lossy(name)
        ))
    })?;
    Ok(unsafe { std::mem::transmute_copy(&p) })
}
static API: OnceLock<Mutex<Option<Api>>> = OnceLock::new();
fn api() -> &'static Mutex<Option<Api>> {
    API.get_or_init(|| Mutex::new(None))
}
fn loaded<T: Copy>(name: &[u8]) -> Result<T> {
    let g = api()
        .lock()
        .map_err(|_| Error::from_reason("ZLG API lock poisoned"))?;
    proc(
        g.as_ref()
            .ok_or_else(|| Error::from_reason("ZLG DLL is not loaded"))?,
        name,
    )
}

#[napi(js_name = "LoadDll")]
pub fn load_dll(path: String) -> Result<()> {
    let mut p = path;
    if !p.to_ascii_lowercase().ends_with(".dll") {
        p.push_str("\\zlgcan.dll")
    };
    let c = CString::new(p.clone()).map_err(|_| Error::from_reason("NUL in DLL path"))?;
    if let Some(parent) = Path::new(&p).parent() {
        if let Ok(directory) = CString::new(parent.to_string_lossy().as_bytes()) { unsafe { SetDllDirectoryA(directory.as_ptr() as _); } }
    }
    let h = unsafe { LoadLibraryA(c.as_ptr() as _) };
    if h.is_null() {
        return Err(Error::from_reason("failed to load ZLG DLL"));
    };
    *api()
        .lock()
        .map_err(|_| Error::from_reason("ZLG API lock poisoned"))? = Some(Api(h));
    Ok(())
}
#[napi(js_name = "IsLoaded")]
pub fn is_loaded() -> Result<bool> {
    Ok(api()
        .lock()
        .map_err(|_| Error::from_reason("ZLG API lock poisoned"))?
        .is_some())
}
#[napi(js_name = "ZCAN_OpenDevice")]
pub fn open_device(t: u32, i: u32, r: u32) -> Result<i64> {
    Ok(unsafe { loaded::<OpenDevice>(b"ZCAN_OpenDevice\0")?(t, i, r) } as i64)
}
#[napi(js_name = "ZCAN_CloseDevice")]
pub fn close_device(h: i64) -> Result<u32> {
    Ok(unsafe { loaded::<CloseDevice>(b"ZCAN_CloseDevice\0")?(h as Device) })
}
#[napi(js_name = "ZCAN_InitCAN")]
pub fn init_can(h: i64, index: u32, cfg: &ZcanChannelInitConfig) -> Result<i64> {
    Ok(
        unsafe { loaded::<InitCan>(b"ZCAN_InitCAN\0")?(h as Device, index, cfg.raw.as_ptr()) }
            as i64,
    )
}
#[napi(js_name = "ZCAN_StartCAN")]
pub fn start_can(h: i64) -> Result<u32> {
    Ok(unsafe { loaded::<ChannelFn>(b"ZCAN_StartCAN\0")?(h as Channel) })
}
#[napi(js_name = "ZCAN_ResetCAN")]
pub fn reset_can(h: i64) -> Result<u32> {
    Ok(unsafe { loaded::<ChannelFn>(b"ZCAN_ResetCAN\0")?(h as Channel) })
}
#[napi(js_name = "ZCAN_ClearBuffer")]
pub fn clear_buffer(h: i64) -> Result<u32> {
    Ok(unsafe { loaded::<ChannelFn>(b"ZCAN_ClearBuffer\0")?(h as Channel) })
}
#[napi(js_name = "ZCAN_CloseCAN")]
pub fn close_can(h: i64) -> Result<u32> {
    Ok(unsafe { loaded::<ChannelFn>(b"ZCAN_CloseCAN\0")?(h as Channel) })
}
#[napi(js_name = "ZCAN_StopCAN")]
pub fn stop_can(h: i64) -> Result<u32> {
    Ok(unsafe { loaded::<ChannelFn>(b"ZCAN_StopCAN\0")?(h as Channel) })
}
#[napi(js_name = "ZCAN_SetValue")]
pub fn set_value(h: i64, path: String, value: String) -> Result<u32> {
    let p = CString::new(path).map_err(|_| Error::from_reason("NUL in path"))?;
    let v = CString::new(value).map_err(|_| Error::from_reason("NUL in value"))?;
    Ok(unsafe { loaded::<SetValue>(b"ZCAN_SetValue\0")?(h as Device, p.as_ptr(), v.as_ptr()) })
}
#[napi(js_name = "ZCAN_Transmit")]
pub fn transmit(h: i64, frame: &ZcanTransmitData, _len: u32) -> Result<u32> {
    Ok(unsafe { loaded::<Transmit>(b"ZCAN_Transmit\0")?(h as Channel, &frame.raw, 1) })
}
#[napi(js_name = "ZCAN_TransmitFD")]
pub fn transmit_fd(h: i64, frame: &ZcanTransmitFdData, _len: u32) -> Result<u32> {
    Ok(unsafe { loaded::<TransmitFd>(b"ZCAN_TransmitFD\0")?(h as Channel, &frame.raw, 1) })
}
#[napi(js_name = "ZCAN_Receive")]
pub fn receive(h: i64, output: &mut ReceiveDataArray, count: u32, wait: i32) -> Result<u32> {
    let mut frames = output.values.lock().map_err(|_| Error::from_reason("receive array poisoned"))?;
    frames.resize(count as usize, CanReceive { frame: CanFrame { id: 0, dlc: 0, pad: 0, r0: 0, r1: 0, data: [0; 8] }, timestamp: 0 });
    Ok(unsafe { loaded::<Receive>(b"ZCAN_Receive\0")?(h as Channel, frames.as_mut_ptr(), count, wait) })
}
#[allow(dead_code)]
fn receive_legacy(h: i64, _output: i64, count: u32, wait: i32) -> Result<u32> {
    let mut frames = vec![
        CanReceive {
            frame: CanFrame {
                id: 0,
                dlc: 0,
                pad: 0,
                r0: 0,
                r1: 0,
                data: [0; 8]
            },
            timestamp: 0
        };
        count as usize
    ];
    let n = unsafe {
        loaded::<Receive>(b"ZCAN_Receive\0")?(h as Channel, frames.as_mut_ptr(), count, wait)
    };
    Ok(n)
}
#[napi(js_name = "ZCAN_ReceiveFD")]
pub fn receive_fd(h: i64, output: &mut ReceiveFdDataArray, count: u32, wait: i32) -> Result<u32> {
    let mut frames = output.values.lock().map_err(|_| Error::from_reason("receive FD array poisoned"))?;
    frames.resize(count as usize, CanReceiveFd { frame: CanFdFrame { id: 0, len: 0, flags: 0, r0: 0, r1: 0, data: [0; 64] }, timestamp: 0 });
    Ok(unsafe { loaded::<ReceiveFd>(b"ZCAN_ReceiveFD\0")?(h as Channel, frames.as_mut_ptr(), count, wait) })
}
#[allow(dead_code)]
fn receive_fd_legacy(h: i64, _output: i64, count: u32, wait: i32) -> Result<u32> {
    let mut frames = vec![
        CanReceiveFd {
            frame: CanFdFrame {
                id: 0,
                len: 0,
                flags: 0,
                r0: 0,
                r1: 0,
                data: [0; 64]
            },
            timestamp: 0
        };
        count as usize
    ];
    let n = unsafe {
        loaded::<ReceiveFd>(b"ZCAN_ReceiveFD\0")?(h as Channel, frames.as_mut_ptr(), count, wait)
    };
    Ok(n)
}
#[napi(js_name = "ZCAN_GetReceiveNum")]
pub fn get_receive_num(h: i64, kind: u8) -> Result<u32> {
    Ok(unsafe { loaded::<GetReceiveNum>(b"ZCAN_GetReceiveNum\0")?(h as Channel, kind) })
}
#[napi(js_name = "ZCAN_ReadChannelErrInfo")]
pub fn read_error(h: i64, mut info: Option<&mut ZcanChannelErrInfo>) -> Result<u32> {
    let mut e = ChannelErr::default();
    let status = unsafe { loaded::<ReadErr>(b"ZCAN_ReadChannelErrInfo\0")?(h as Channel, &mut e) };
    if let Some(output) = info.as_mut() { output.error_code = e.error_code; }
    Ok(status)
}

#[napi(js_name = "ZCAN_CHANNEL_INIT_CONFIG")]
pub struct ZcanChannelInitConfig {
    pub raw: Vec<u8>,
    #[napi(js_name = "can_type")] pub can_type: u32,
    #[napi(js_name = "can_acc_code")] pub can_acc_code: u32,
    #[napi(js_name = "can_acc_mask")] pub can_acc_mask: u32,
    #[napi(js_name = "can_abit_timing")] pub can_abit_timing: u32,
    #[napi(js_name = "can_dbit_timing")] pub can_dbit_timing: u32,
    #[napi(js_name = "can_baud_prescaler")] pub can_baud_prescaler: u32,
    #[napi(js_name = "can_filter")] pub can_filter: u8,
    #[napi(js_name = "can_mode")] pub can_mode: u8,
}
#[napi]
impl ZcanChannelInitConfig {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            raw: vec![0; 256],
            can_type: 0,
            can_acc_code: 0,
            can_acc_mask: 0,
            can_abit_timing: 0,
            can_dbit_timing: 0,
            can_baud_prescaler: 0,
            can_filter: 0,
            can_mode: 0,
        }
    }
    #[napi]
    pub fn sync(&mut self) {
        // ZCAN_CHANNEL_INIT_CONFIG is: can_type (u32), followed by the
        // CAN/CAN-FD union. The vendor ABI uses native little-endian fields.
        self.raw = vec![0; 32];
        self.raw[0..4].copy_from_slice(&self.can_type.to_ne_bytes());
        self.raw[4..8].copy_from_slice(&self.can_acc_code.to_ne_bytes());
        self.raw[8..12].copy_from_slice(&self.can_acc_mask.to_ne_bytes());
        self.raw[12..16].copy_from_slice(&self.can_abit_timing.to_ne_bytes());
        self.raw[16..20].copy_from_slice(&self.can_dbit_timing.to_ne_bytes());
        self.raw[20..24].copy_from_slice(&self.can_baud_prescaler.to_ne_bytes());
        self.raw[24] = self.can_filter;
        self.raw[25] = self.can_mode;
    }
}
#[napi(js_name = "ZCAN_Transmit_Data")]
pub struct ZcanTransmitData {
    raw: CanTransmit,
}
#[napi]
impl ZcanTransmitData {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            raw: CanTransmit {
                frame: CanFrame {
                    id: 0,
                    dlc: 0,
                    pad: 0,
                    r0: 0,
                    r1: 0,
                    data: [0; 8],
                },
                transmit_type: 0,
            },
        }
    }
    #[napi]
    pub fn set_frame(&mut self, id: u32, data: Buffer, pad: u8) -> Result<()> {
        self.raw.frame.id = id;
        self.raw.frame.dlc = data.len().min(8) as u8;
        self.raw.frame.pad = pad;
        self.raw.frame.data[..self.raw.frame.dlc as usize]
            .copy_from_slice(&data[..self.raw.frame.dlc as usize]);
        Ok(())
    }
}
#[napi(js_name = "ZCAN_TransmitFD_Data")]
pub struct ZcanTransmitFdData {
    raw: CanTransmitFd,
}
#[napi]
impl ZcanTransmitFdData {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            raw: CanTransmitFd {
                frame: CanFdFrame {
                    id: 0,
                    len: 0,
                    flags: 0,
                    r0: 0,
                    r1: 0,
                    data: [0; 64],
                },
                transmit_type: 0,
            },
        }
    }
    #[napi]
    pub fn set_frame(&mut self, id: u32, data: Buffer, flags: u8) -> Result<()> {
        self.raw.frame.id = id;
        self.raw.frame.len = data.len().min(64) as u8;
        self.raw.frame.flags = flags;
        self.raw.frame.data[..self.raw.frame.len as usize]
            .copy_from_slice(&data[..self.raw.frame.len as usize]);
        Ok(())
    }
}
#[napi(object)]
pub struct ZcanReceiveData {
    pub can_id: u32,
    pub data: Buffer,
    pub flags: u32,
    pub timestamp: i64,
}
impl ZcanReceiveData {
    fn from_raw(x: CanReceive) -> Self {
        Self {
            can_id: x.frame.id,
            data: Buffer::from(x.frame.data[..x.frame.dlc.min(8) as usize].to_vec()),
            flags: x.frame.pad as u32,
            timestamp: x.timestamp as i64,
        }
    }
}
#[napi(object)]
pub struct ZcanReceiveFdData {
    pub can_id: u32,
    pub data: Buffer,
    pub flags: u32,
    pub timestamp: i64,
}
#[napi(js_name = "ZCAN_CHANNEL_ERR_INFO")]
pub struct ZcanChannelErrInfo {
    pub error_code: u32,
}
#[napi]
impl ZcanChannelErrInfo {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self { error_code: 0 }
    }
}
impl ZcanReceiveFdData {
    fn from_raw(x: CanReceiveFd) -> Self {
        Self {
            can_id: x.frame.id,
            data: Buffer::from(x.frame.data[..x.frame.len.min(64) as usize].to_vec()),
            flags: x.frame.flags as u32,
            timestamp: x.timestamp as i64,
        }
    }
}
#[napi]
pub struct U32Array {
    pub values: Vec<u32>,
}
#[napi]
impl U32Array {
    #[napi(constructor)]
    pub fn new(length: u32) -> Self {
        Self {
            values: vec![0; length as usize],
        }
    }
    #[napi]
    pub fn cast(&self) -> i64 {
        self.values.as_ptr() as i64
    }
    #[napi]
    pub fn getitem(&self, index: u32) -> Result<u32> {
        Ok(self.values.get(index as usize).copied().unwrap_or(0))
    }
}
#[napi]
pub struct ByteArray {
    pub values: Mutex<Vec<u8>>,
    pointer: i64,
}
#[napi]
impl ByteArray {
    #[napi(constructor)]
    pub fn new(length: u32) -> Self {
        Self {
            values: Mutex::new(vec![0; length as usize]),
            pointer: 0,
        }
    }
    #[napi(js_name = "frompointer")]
    pub fn from_pointer(pointer: i64) -> Self {
        Self { values: Mutex::new(vec![0; 64]), pointer }
    }
    #[napi]
    pub fn cast(&self) -> i64 {
        if self.pointer != 0 { self.pointer } else { self.values.lock().map(|v| v.as_ptr() as i64).unwrap_or(0) }
    }
    #[napi]
    pub fn getitem(&self, index: u32) -> Result<u8> {
        if self.pointer != 0 { return Ok(unsafe { (self.pointer as *const u8).add(index as usize).read() }); }
        Ok(self.values.lock().map_err(|_| Error::from_reason("array lock poisoned"))?.get(index as usize).copied().unwrap_or(0))
    }
    #[napi]
    pub fn setitem(&self, index: u32, value: u8) -> Result<()> {
        if self.pointer != 0 { unsafe { (self.pointer as *mut u8).add(index as usize).write(value); } return Ok(()); }
        if let Some(v) = self.values.lock().map_err(|_| Error::from_reason("array lock poisoned"))?.get_mut(index as usize) { *v = value; }
        Ok(())
    }
}
#[napi]
pub struct ReceiveDataArray {
    values: Mutex<Vec<CanReceive>>,
}
#[napi]
impl ReceiveDataArray {
    #[napi(constructor)]
    pub fn new(length: u32) -> Self {
        Self {
            values: Mutex::new(vec![
                CanReceive {
                    frame: CanFrame {
                        id: 0,
                        dlc: 0,
                        pad: 0,
                        r0: 0,
                        r1: 0,
                        data: [0; 8]
                    },
                    timestamp: 0
                };
                length as usize
            ]),
        }
    }
    #[napi]
    pub fn cast(&self) -> i64 {
        self.values.lock().map(|v| v.as_ptr() as i64).unwrap_or(0)
    }
    #[napi]
    pub fn getitem(&self, index: u32) -> Result<ZcanReceiveData> {
        self.values
            .lock()
            .map_err(|_| Error::from_reason("array lock poisoned"))?
            .get(index as usize)
            .copied()
            .ok_or_else(|| Error::from_reason("array index out of bounds"))
            .map(ZcanReceiveData::from_raw)
    }
}
#[napi(js_name = "ReceiveFDDataArray")]
pub struct ReceiveFdDataArray {
    values: Mutex<Vec<CanReceiveFd>>,
}
#[napi]
impl ReceiveFdDataArray {
    #[napi(constructor)]
    pub fn new(length: u32) -> Self {
        Self {
            values: Mutex::new(vec![
                CanReceiveFd {
                    frame: CanFdFrame {
                        id: 0,
                        len: 0,
                        flags: 0,
                        r0: 0,
                        r1: 0,
                        data: [0; 64]
                    },
                    timestamp: 0
                };
                length as usize
            ]),
        }
    }
    #[napi]
    pub fn cast(&self) -> i64 {
        self.values.lock().map(|v| v.as_ptr() as i64).unwrap_or(0)
    }
    #[napi]
    pub fn getitem(&self, index: u32) -> Result<ZcanReceiveFdData> {
        self.values
            .lock()
            .map_err(|_| Error::from_reason("array lock poisoned"))?
            .get(index as usize)
            .copied()
            .ok_or_else(|| Error::from_reason("array index out of bounds"))
            .map(ZcanReceiveFdData::from_raw)
    }
}
#[napi(js_name = "handleConver")]
pub fn handle_conver(handle: i64, out: i64) -> Result<()> {
    if out == 0 {
        return Err(Error::from_reason("output pointer is null"));
    }
    let value = handle as u64;
    unsafe {
        let target = out as *mut u32;
        target.write(value as u32);
        target.add(1).write((value >> 32) as u32);
    }
    Ok(())
}
struct CallbackTask {
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}
static CALLBACKS: OnceLock<Mutex<HashMap<String, CallbackTask>>> = OnceLock::new();
fn callbacks() -> &'static Mutex<HashMap<String, CallbackTask>> {
    CALLBACKS.get_or_init(|| Mutex::new(HashMap::new()))
}
#[napi(js_name = "CreateTSFN")]
pub fn create_tsfn(
    device: i64,
    channel: i64,
    name: String,
    fd_name: String,
    callback: Function<'static>,
    callback_fd: Function<'static>,
    _error: String,
    error_callback: Function<'static>,
) -> Result<()> {
    free_tsfn(name.clone())?;
    let tsfn: ThreadsafeFunction<()> = callback
        .build_threadsafe_function()
        .callee_handled()
        .build_callback(|_| Ok(()))?;
    let fd_tsfn: ThreadsafeFunction<()> = callback_fd
        .build_threadsafe_function()
        .callee_handled()
        .build_callback(|_| Ok(()))?;
    let err_tsfn: ThreadsafeFunction<()> = error_callback
        .build_threadsafe_function()
        .callee_handled()
        .build_callback(|_| Ok(()))?;
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let join = std::thread::spawn(move || {
        while !flag.load(Ordering::Acquire) {
            let result = if fd_name.is_empty() {
                let mut frames = ReceiveDataArray::new(64);
                receive(channel, &mut frames, 64, 20)
            } else {
                let mut frames = ReceiveFdDataArray::new(64);
                receive_fd(channel, &mut frames, 64, 20)
            };
            match result {
                Ok(n) if n > 0 => {
                    let _ = if fd_name.is_empty() {
                        tsfn.call(Ok(()), ThreadsafeFunctionCallMode::NonBlocking)
                    } else {
                        fd_tsfn.call(Ok(()), ThreadsafeFunctionCallMode::NonBlocking)
                    };
                }
                Ok(_) => {}
                Err(_) => {
                    let _ = err_tsfn.call(Ok(()), ThreadsafeFunctionCallMode::NonBlocking);
                    break;
                }
            }
        }
        let _ = device;
    });
    callbacks()
        .lock()
        .map_err(|_| Error::from_reason("callback lock poisoned"))?
        .insert(
            name,
            CallbackTask {
                stop,
                join: Some(join),
            },
        );
    Ok(())
}
#[napi(js_name = "FreeTSFN")]
pub fn free_tsfn(name: String) -> Result<()> {
    if let Some(mut task) = callbacks()
        .lock()
        .map_err(|_| Error::from_reason("callback lock poisoned"))?
        .remove(&name)
    {
        task.stop.store(true, Ordering::Release);
        if let Some(join) = task.join.take() {
            let _ = join.join();
        }
    }
    Ok(())
}
#[napi]
pub fn migration_status() -> String {
    "ZLG Rust DLL FFI and callback lifecycle implemented".to_string()
}
#[napi(js_name = "ZCAN_USBCANFD_200U")]
pub const ZCAN_USBCANFD_200U: i32 = 41;
#[napi(js_name = "ZCAN_USBCANFD_100U")]
pub const ZCAN_USBCANFD_100U: i32 = 42;
#[napi(js_name = "ZCAN_USBCANFD_400U")]
pub const ZCAN_USBCANFD_400U: i32 = 43;
#[napi(js_name = "ZCAN_USBCANFD_MINI")]
pub const ZCAN_USBCANFD_MINI: i32 = 44;
#[napi(js_name = "ZCAN_USBCAN_E_U")]
pub const ZCAN_USBCAN_E_U: i32 = 3;
#[napi(js_name = "ZCAN_USBCAN2")]
pub const ZCAN_USBCAN2: i32 = 4;
#[napi(js_name = "ZCAN_USBCAN_2E_U")]
pub const ZCAN_USBCAN_2E_U: i32 = 5;
