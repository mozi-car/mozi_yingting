use napi::bindgen_prelude::*;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi_derive::napi;
use std::ffi::{c_char, c_void, CString};
use std::path::Path;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;
use windows_sys::Win32::Foundation::{FreeLibrary, HMODULE};
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA, SetDllDirectoryA};

type ErrorCode = u32;
type Register = unsafe extern "system" fn(*const c_char, u32, *mut u8) -> ErrorCode;
type Remove = unsafe extern "system" fn(u8) -> ErrorCode;
type Connect = unsafe extern "system" fn(u8, u16) -> ErrorCode;
type Read = unsafe extern "system" fn(u8, *mut TlinRcvMsg) -> ErrorCode;
type Write = unsafe extern "system" fn(u8, u16, *const TlinMsg) -> ErrorCode;
type InitHw = unsafe extern "system" fn(u8, u16, u8, u16) -> ErrorCode;
type Available = unsafe extern "system" fn(*mut u16, u16, *mut i32) -> ErrorCode;
type RegisterFrame = unsafe extern "system" fn(u8, u16, u8, u8) -> ErrorCode;
type FrameEntry = unsafe extern "system" fn(u8, u16, *mut TlinFrameEntry) -> ErrorCode;
type Status = unsafe extern "system" fn(u16, *mut TlinHardwareStatus) -> ErrorCode;
type Wake = unsafe extern "system" fn(u8, u16) -> ErrorCode;
type Version = unsafe extern "system" fn(*mut TlinVersion) -> ErrorCode;
type ErrorText = unsafe extern "system" fn(ErrorCode, u8, *mut c_char, u16) -> ErrorCode;
type Checksum = unsafe extern "system" fn(*mut TlinMsg) -> ErrorCode;
type Update = unsafe extern "system" fn(u8, u16, u8, u8, u8, *mut u8) -> ErrorCode;
type HardwareParam = unsafe extern "system" fn(u16, u16, *mut c_void, u16) -> ErrorCode;
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct TlinVersion {
    pub major: i16,
    pub minor: i16,
    pub revision: i16,
    pub build: i16,
}
#[repr(C, packed(1))]
#[derive(Clone, Copy)]
pub struct TlinMsg {
    pub frame_id: u8,
    pub length: u8,
    pub direction: u8,
    pub checksum_type: u8,
    pub data: [u8; 8],
    pub checksum: u8,
}
impl Default for TlinMsg {
    fn default() -> Self {
        Self {
            frame_id: 0,
            length: 0,
            direction: 0,
            checksum_type: 0,
            data: [0; 8],
            checksum: 0,
        }
    }
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TlinRcvMsg {
    pub msg_type: u8,
    pub frame_id: u8,
    pub length: u8,
    pub direction: u8,
    pub checksum_type: u8,
    pub data: [u8; 8],
    pub checksum: u8,
    pub error_flags: i32,
    pub timestamp: u64,
    pub hw: u16,
}
impl Default for TlinRcvMsg {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TlinFrameEntry {
    pub frame_id: u8,
    pub length: u8,
    pub direction: u8,
    pub checksum_type: u8,
    pub flags: u16,
    pub initial_data: [u8; 8],
}
impl Default for TlinFrameEntry {
    fn default() -> Self {
        Self {
            frame_id: 0,
            length: 0,
            direction: 0,
            checksum_type: 0,
            flags: 0,
            initial_data: [0; 8],
        }
    }
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct TlinHardwareStatus {
    pub mode: u8,
    pub status: u8,
    pub free_send: u8,
    pub reserved: u8,
    pub free_schedule: u16,
    pub overrun: u16,
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
fn sym<T: Copy>(a: &Api, n: &[u8]) -> Result<T> {
    let p = unsafe { GetProcAddress(a.0, n.as_ptr() as _) }.ok_or_else(|| {
        Error::from_reason(format!(
            "PEAK LIN symbol missing: {}",
            String::from_utf8_lossy(n)
        ))
    })?;
    Ok(unsafe { std::mem::transmute_copy(&p) })
}
static API: OnceLock<Mutex<Option<Api>>> = OnceLock::new();
fn api() -> &'static Mutex<Option<Api>> {
    API.get_or_init(|| Mutex::new(None))
}
struct CallbackTask { stop: Arc<std::sync::atomic::AtomicBool>, worker: Option<thread::JoinHandle<()>> }
static CALLBACKS: OnceLock<Mutex<HashMap<String, CallbackTask>>> = OnceLock::new();
fn callbacks() -> &'static Mutex<HashMap<String, CallbackTask>> { CALLBACKS.get_or_init(|| Mutex::new(HashMap::new())) }

#[napi(js_name = "CreateTSFN")]
pub fn create_tsfn(client: u8, name: String, callback: Function<'static>) -> Result<()> {
    free_tsfn(name.clone())?;
    let tsfn: ThreadsafeFunction<()> = callback.build_threadsafe_function().callee_handled().build_callback(|_| Ok(()))?;
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let stop_worker = stop.clone();
    let worker = thread::spawn(move || {
        while !stop_worker.load(std::sync::atomic::Ordering::Acquire) {
            let mut msg = TlinRcvMsg::default();
            let status = loaded::<Read>(b"LIN_Read\\0").ok().map(|f| unsafe { f(client, &mut msg) });
            if status == Some(0) { let _ = tsfn.call(Ok(()), ThreadsafeFunctionCallMode::NonBlocking); }
            thread::sleep(Duration::from_millis(2));
        }
    });
    callbacks().lock().map_err(|_| Error::from_reason("PEAK LIN callback lock poisoned"))?.insert(name, CallbackTask { stop, worker: Some(worker) });
    Ok(())
}
#[napi(js_name = "FreeTSFN")]
pub fn free_tsfn(name: String) -> Result<()> {
    if let Some(mut task) = callbacks().lock().map_err(|_| Error::from_reason("PEAK LIN callback lock poisoned"))?.remove(&name) {
        task.stop.store(true, std::sync::atomic::Ordering::Release);
        if let Some(worker) = task.worker.take() { let _ = worker.join(); }
    }
    Ok(())
}

fn loaded<T: Copy>(n: &[u8]) -> Result<T> {
    let g = api()
        .lock()
        .map_err(|_| Error::from_reason("PEAK LIN lock poisoned"))?;
    sym(
        g.as_ref()
            .ok_or_else(|| Error::from_reason("PEAK LIN DLL is not loaded"))?,
        n,
    )
}
#[napi(js_name = "LoadDll")]
pub fn load_dll(path: String) -> Result<()> {
    let path = if path.to_ascii_lowercase().ends_with(".dll") { path } else { format!("{}\\PLinApi.dll", path.trim_end_matches(|c| c == '\\' || c == '/')) };
    let c = CString::new(path).map_err(|_| Error::from_reason("NUL in DLL path"))?;
    if let Some(parent) = Path::new(c.to_str().unwrap_or_default()).parent() {
        if let Ok(directory) = CString::new(parent.to_string_lossy().as_bytes()) { unsafe { SetDllDirectoryA(directory.as_ptr() as _); } }
    }
    let h = unsafe { LoadLibraryA(c.as_ptr() as _) };
    if h.is_null() {
        return Err(Error::from_reason("failed to load PEAK LIN DLL"));
    }
    *api()
        .lock()
        .map_err(|_| Error::from_reason("lock poisoned"))? = Some(Api(h));
    Ok(())
}
#[napi(js_name = "HLINCLIENT_JS")]
pub struct HlinClient { value: u8 }
#[napi]
impl HlinClient {
    #[napi(constructor)] pub fn new() -> Self { Self { value: 0 } }
    #[napi] pub fn cast(&self) -> u8 { self.value }
    #[napi] pub fn value(&self) -> u8 { self.value }
}
#[napi(js_name = "LIN_RegisterClient")]
pub fn register_client(name: String, window: u32, mut output: Option<&mut HlinClient>) -> Result<u32> {
    let c = CString::new(name).map_err(|_| Error::from_reason("NUL in client name"))?;
    let mut h = 0;
    let status = unsafe { loaded::<Register>(b"LIN_RegisterClient\0")?(c.as_ptr(), window, &mut h) };
    if status == 0 { if let Some(out) = output.as_mut() { out.value = h; } }
    Ok(status)
}
#[napi(js_name = "LIN_RemoveClient")]
pub fn remove_client(h: u8) -> Result<u32> {
    Ok(unsafe { loaded::<Remove>(b"LIN_RemoveClient\0")?(h) })
}
#[napi(js_name = "LIN_ConnectClient")]
pub fn connect(h: u8, hw: u16) -> Result<u32> {
    Ok(unsafe { loaded::<Connect>(b"LIN_ConnectClient\0")?(h, hw) })
}
#[napi(js_name = "LIN_DisconnectClient")]
pub fn disconnect(h: u8, hw: u16) -> Result<u32> {
    Ok(unsafe { loaded::<Connect>(b"LIN_DisconnectClient\0")?(h, hw) })
}
#[napi(js_name = "LIN_InitializeHardware")]
pub fn initialize(h: u8, hw: u16, mode: u8, baud: u16) -> Result<u32> {
    Ok(unsafe { loaded::<InitHw>(b"LIN_InitializeHardware\0")?(h, hw, mode, baud) })
}
#[napi(js_name = "LIN_GetAvailableHardware")]
pub fn available() -> Result<Vec<u16>> {
    let mut v = vec![0u16; 64];
    let mut n = 0;
    let s = unsafe {
        loaded::<Available>(b"LIN_GetAvailableHardware\0")?(
            v.as_mut_ptr(),
            (v.len() * 2) as u16,
            &mut n,
        )
    };
    if s != 0 {
        Err(Error::from_reason(format!("PEAK LIN error {s}")))
    } else {
        Ok(v.into_iter().take(n.max(0) as usize).collect())
    }
}
#[napi(js_name = "LIN_RegisterFrameId")]
pub fn register_frame(c: u8, hw: u16, from: u8, to: u8) -> Result<u32> {
    Ok(unsafe { loaded::<RegisterFrame>(b"LIN_RegisterFrameId\0")?(c, hw, from, to) })
}
#[napi(js_name = "LIN_Read")]
pub fn read(c: u8) -> Result<TlinRcvMsgJs> {
    let mut m = TlinRcvMsg::default();
    let s = unsafe { loaded::<Read>(b"LIN_Read\0")?(c, &mut m) };
    if s != 0 {
        Err(Error::from_reason(format!("PEAK LIN error {s}")))
    } else {
        Ok(TlinRcvMsgJs::from_raw(m))
    }
}
#[napi(js_name = "LIN_Write")]
pub fn write(c: u8, hw: u16, msg: &TlinMsgJs) -> Result<u32> {
    let raw = msg.raw();
    Ok(unsafe { loaded::<Write>(b"LIN_Write\0")?(c, hw, &raw) })
}
#[napi(js_name = "LIN_SetFrameEntry")]
pub fn set_entry(c: u8, hw: u16, e: &TlinFrameEntryJs) -> Result<u32> {
    let mut x = e.raw();
    Ok(unsafe { loaded::<FrameEntry>(b"LIN_SetFrameEntry\0")?(c, hw, &mut x) })
}
#[napi(js_name = "LIN_GetHardwareParam")]
pub fn get_hardware_param(hw: u16, param: u16, mut buffer: Buffer) -> Result<u32> {
    let s = unsafe { loaded::<HardwareParam>(b"LIN_GetHardwareParam\0")?(hw, param, buffer.as_mut_ptr().cast(), buffer.len() as u16) };
    Ok(s)
}
#[napi(js_name = "LIN_GetStatus")]
pub fn get_status(hw: u16) -> Result<TlinHardwareStatusJs> {
    let mut s = TlinHardwareStatus::default();
    let r = unsafe { loaded::<Status>(b"LIN_GetStatus\0")?(hw, &mut s) };
    if r != 0 {
        Err(Error::from_reason(format!("PEAK LIN error {r}")))
    } else {
        Ok(TlinHardwareStatusJs::from_raw(s))
    }
}
#[napi(js_name = "LIN_ResetHardwareConfig")]
pub fn reset_config(c: u8, hw: u16) -> Result<u32> {
    Ok(unsafe { loaded::<Connect>(b"LIN_ResetHardwareConfig\0")?(c, hw) })
}
#[napi(js_name = "LIN_XmtWakeUp")]
pub fn wake(c: u8, hw: u16) -> Result<u32> {
    Ok(unsafe { loaded::<Wake>(b"LIN_XmtWakeUp\0")?(c, hw) })
}
#[napi(js_name = "LIN_CalculateChecksum")]
pub fn checksum(msg: &mut TlinMsgJs) -> Result<u32> {
    let mut raw = msg.raw();
    let s = unsafe { loaded::<Checksum>(b"LIN_CalculateChecksum\0")?(&mut raw) };
    msg.checksum = raw.checksum;
    Ok(s)
}
#[napi(js_name = "LIN_GetVersion")]
pub fn version() -> Result<TlinVersionJs> {
    let mut v = TlinVersion::default();
    let s = unsafe { loaded::<Version>(b"LIN_GetVersion\0")?(&mut v) };
    if s != 0 {
        Err(Error::from_reason(format!("PEAK LIN error {s}")))
    } else {
        Ok(TlinVersionJs {
            major: v.major,
            minor: v.minor,
            revision: v.revision,
            build: v.build,
        })
    }
}
#[napi(js_name = "LIN_GetErrorText")]
pub fn error_text(error: u32, language: u8) -> Result<Buffer> {
    let mut b = vec![0u8; 256];
    unsafe {
        loaded::<ErrorText>(b"LIN_GetErrorText\0")?(
            error,
            language,
            b.as_mut_ptr() as _,
            b.len() as u16,
        )
    };
    Ok(Buffer::from(b))
}
#[napi(js_name = "LIN_UpdateByteArray")]
pub fn update_bytes(client: u8, hw: u16, frame_id: u8, index: u8, length: u8, data: Buffer) -> Result<u32> {
    let mut data = data.to_vec();
    let length = usize::from(length).min(data.len());
    Ok(unsafe { loaded::<Update>(b"LIN_UpdateByteArray\0")?(client, hw, frame_id, index, length as u8, data.as_mut_ptr()) })
}
#[allow(non_upper_case_globals)]
#[napi]
pub const hwpName: u16 = 1;
#[napi(js_name = "FRAME_FLAG_RESPONSE_ENABLE")]
pub const FRAME_FLAG_RESPONSE_ENABLE: u16 = 1;
#[napi(js_name = "FRAME_FLAG_IGNORE_INIT_DATA")]
pub const FRAME_FLAG_IGNORE_INIT_DATA: u16 = 4;
#[napi(js_name = "TLINVersion")]
pub struct TlinVersionJs {
    #[napi(js_name = "Major")] pub major: i16,
    #[napi(js_name = "Minor")] pub minor: i16,
    #[napi(js_name = "Revision")] pub revision: i16,
    #[napi(js_name = "Build")] pub build: i16,
}
#[napi]
impl TlinVersionJs {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            major: 0,
            minor: 0,
            revision: 0,
            build: 0,
        }
    }
}
#[napi(js_name = "TLINMsg")]
pub struct TlinMsgJs {
    #[napi(js_name = "FrameId")]
    pub frame_id: u8,
    #[napi(js_name = "Length")]
    pub length: u8,
    #[napi(js_name = "Direction")]
    pub direction: u8,
    #[napi(js_name = "ChecksumType")]
    pub checksum_type: u8,
    #[napi(js_name = "Data")]
    pub data: Vec<u8>,
    #[napi(js_name = "Checksum")]
    pub checksum: u8,
}
impl TlinMsgJs {
    fn raw(&self) -> TlinMsg {
        let mut d = [0; 8];
        d[..self.data.len().min(8)].copy_from_slice(&self.data[..self.data.len().min(8)]);
        TlinMsg {
            frame_id: self.frame_id,
            length: self.length,
            direction: self.direction,
            checksum_type: self.checksum_type,
            data: d,
            checksum: self.checksum,
        }
    }
}
#[napi]
impl TlinMsgJs {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            frame_id: 0,
            length: 0,
            direction: 0,
            checksum_type: 0,
            data: vec![0; 8],
            checksum: 0,
        }
    }
}
#[napi(js_name = "TLINFrameEntry")]
pub struct TlinFrameEntryJs {
    #[napi(js_name = "FrameId")]
    pub frame_id: u8,
    #[napi(js_name = "Length")]
    pub length: u8,
    #[napi(js_name = "Direction")]
    pub direction: u8,
    #[napi(js_name = "ChecksumType")]
    pub checksum_type: u8,
    #[napi(js_name = "Flags")]
    pub flags: u16,
    #[napi(js_name = "InitialData")]
    pub initial_data: Vec<u8>,
}
impl TlinFrameEntryJs {
    fn raw(&self) -> TlinFrameEntry {
        let mut d = [0; 8];
        d[..self.initial_data.len().min(8)]
            .copy_from_slice(&self.initial_data[..self.initial_data.len().min(8)]);
        TlinFrameEntry {
            frame_id: self.frame_id,
            length: self.length,
            direction: self.direction,
            checksum_type: self.checksum_type,
            flags: self.flags,
            initial_data: d,
        }
    }
}
#[napi]
impl TlinFrameEntryJs {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            frame_id: 0,
            length: 0,
            direction: 0,
            checksum_type: 0,
            flags: 0,
            initial_data: vec![0; 8],
        }
    }
}
#[napi(js_name = "TLINRcvMsg")]
pub struct TlinRcvMsgJs {
    #[napi(js_name = "Type")] pub msg_type: u8,
    #[napi(js_name = "FrameId")] pub frame_id: u8,
    #[napi(js_name = "Length")] pub length: u8,
    #[napi(js_name = "Direction")] pub direction: u8,
    #[napi(js_name = "ChecksumType")] pub checksum_type: u8,
    #[napi(js_name = "Data")] pub data: Vec<u8>,
    #[napi(js_name = "Checksum")] pub checksum: u8,
    #[napi(js_name = "ErrorFlags")] pub error_flags: i32,
    #[napi(js_name = "TimeStamp")] pub timestamp: i64,
    #[napi(js_name = "Hw")] pub hw: u16,
}
impl TlinRcvMsgJs {
    fn from_raw(x: TlinRcvMsg) -> Self {
        Self {
            msg_type: x.msg_type,
            frame_id: x.frame_id,
            length: x.length,
            direction: x.direction,
            checksum_type: x.checksum_type,
            data: x.data[..x.length.min(8) as usize].to_vec(),
            checksum: x.checksum,
            error_flags: x.error_flags,
            timestamp: x.timestamp as i64,
            hw: x.hw,
        }
    }
}
#[napi]
impl TlinRcvMsgJs {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            msg_type: 0,
            frame_id: 0,
            length: 0,
            direction: 0,
            checksum_type: 0,
            data: vec![0; 8],
            checksum: 0,
            error_flags: 0,
            timestamp: 0,
            hw: 0,
        }
    }
}
#[napi(js_name = "TLINHardwareStatus")]
pub struct TlinHardwareStatusJs {
    #[napi(js_name = "Mode")] pub mode: u8,
    #[napi(js_name = "Status")] pub status: u8,
    #[napi(js_name = "FreeOnSendQueue")] pub free_on_send_queue: u8,
    #[napi(js_name = "FreeOnSchedulePool")] pub free_on_schedule_pool: u16,
    #[napi(js_name = "ReceiveBufferOverrun")] pub receive_buffer_overrun: u16,
}
impl TlinHardwareStatusJs {
    fn from_raw(x: TlinHardwareStatus) -> Self {
        Self {
            mode: x.mode,
            status: x.status,
            free_on_send_queue: x.free_send,
            free_on_schedule_pool: x.free_schedule,
            receive_buffer_overrun: x.overrun,
        }
    }
}
#[napi]
impl TlinHardwareStatusJs {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            mode: 0,
            status: 0,
            free_on_send_queue: 0,
            free_on_schedule_pool: 0,
            receive_buffer_overrun: 0,
        }
    }
}
#[napi(js_name = "ByteArray")]
pub struct ByteArray {
    pub data: Vec<u8>,
}
#[napi]
impl ByteArray {
    #[napi(constructor)]
    pub fn new(n: u32) -> Self {
        Self {
            data: vec![0; n as usize],
        }
    }
    #[napi]
    pub fn getitem(&self, i: u32) -> u8 {
        self.data.get(i as usize).copied().unwrap_or(0)
    }
    #[napi]
    pub fn setitem(&mut self, i: u32, v: u8) {
        if let Some(x) = self.data.get_mut(i as usize) {
            *x = v;
        }
    }
    #[napi(js_name = "frompointer")]
    pub fn from_pointer(_p: i64) -> Self {
        Self::new(8)
    }
}
#[napi]
pub fn migration_status() -> String {
    "PEAK LIN PLIN-API ABI and Rust compatibility wrappers implemented".to_string()
}
