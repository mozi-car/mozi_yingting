use napi::bindgen_prelude::*;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi_derive::napi;
use std::collections::HashMap;
use std::ffi::{c_void, CString};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use windows_sys::Win32::Foundation::{FreeLibrary, HMODULE};
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA, SetDllDirectoryA};

type Status = i32;
type Init = unsafe extern "system" fn();
type Unload = unsafe extern "system" fn();
type Open = unsafe extern "system" fn(i32, i32) -> i32;
type Close = unsafe extern "system" fn(i32) -> Status;
type Version = unsafe extern "system" fn(*mut i32, *mut i32, *mut i32) -> Status;
type SetBitrate = unsafe extern "system" fn(i32, u32) -> Status;
type Bus = unsafe extern "system" fn(i32) -> Status;
type Write = unsafe extern "system" fn(i32, u32, *const c_void, u32) -> Status;
type Request = unsafe extern "system" fn(i32, u32) -> Status;
type Read = unsafe extern "system" fn(
    i32,
    *mut u32,
    *mut c_void,
    *mut u32,
    *mut u32,
    *mut LinMessageInfo,
) -> Status;
type ReadWait = unsafe extern "system" fn(
    i32,
    *mut u32,
    *mut c_void,
    *mut u32,
    *mut u32,
    *mut LinMessageInfo,
    u32,
) -> Status;
type Update = unsafe extern "system" fn(i32, u32, *const c_void, u32) -> Status;
type Wakeup = unsafe extern "system" fn(i32, u32, u32) -> Status;

#[repr(C, packed(1))]
#[derive(Clone, Copy, Default)]
pub struct LinMessageInfo {
    pub timestamp: u32,
    pub synch_break_length: u32,
    pub frame_length: u32,
    pub bitrate: u32,
    pub checksum: u8,
    pub id_parity: u8,
    pub reserved: u16,
    pub synch_edge_time: [u32; 4],
    pub byte_time: [u32; 8],
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
fn symbol<T: Copy>(api: &Api, name: &[u8]) -> Result<T> {
    let p = unsafe { GetProcAddress(api.0, name.as_ptr() as _) }.ok_or_else(|| {
        Error::from_reason(format!(
            "Kvaser LIN symbol missing: {}",
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
    let guard = api()
        .lock()
        .map_err(|_| Error::from_reason("Kvaser LIN API lock poisoned"))?;
    symbol(
        guard
            .as_ref()
            .ok_or_else(|| Error::from_reason("Kvaser LIN DLL is not loaded"))?,
        name,
    )
}

#[napi(js_name = "LoadDll")]
pub fn load_dll(path: String) -> Result<()> {
    let path = if path.to_ascii_lowercase().ends_with(".dll") { path } else { format!("{}\\linlib.dll", path.trim_end_matches(|c| c == '\\' || c == '/')) };
    let c = CString::new(path).map_err(|_| Error::from_reason("NUL in DLL path"))?;
    if let Some(parent) = Path::new(c.to_str().unwrap_or_default()).parent() {
        if let Ok(directory) = CString::new(parent.to_string_lossy().as_bytes()) { unsafe { SetDllDirectoryA(directory.as_ptr() as _); } }
    }
    let handle = unsafe { LoadLibraryA(c.as_ptr() as _) };
    if handle.is_null() {
        return Err(Error::from_reason("failed to load Kvaser LIN DLL"));
    }
    *api()
        .lock()
        .map_err(|_| Error::from_reason("Kvaser LIN API lock poisoned"))? = Some(Api(handle));
    Ok(())
}
#[napi(js_name = "IsLoaded")]
pub fn is_loaded() -> Result<bool> {
    Ok(api()
        .lock()
        .map_err(|_| Error::from_reason("lock poisoned"))?
        .is_some())
}
#[napi(js_name = "linInitializeLibrary")]
pub fn initialize() -> Result<()> {
    unsafe {
        loaded::<Init>(b"linInitializeLibrary\0")?();
    }
    Ok(())
}
#[napi(js_name = "linUnloadLibrary")]
pub fn unload() -> Result<()> {
    unsafe {
        loaded::<Unload>(b"linUnloadLibrary\0")?();
    }
    Ok(())
}
#[napi(js_name = "linOpenChannel")]
pub fn open_channel(channel: i32, flags: i32) -> Result<i32> {
    Ok(unsafe { loaded::<Open>(b"linOpenChannel\0")?(channel, flags) })
}
#[napi(js_name = "linClose")]
pub fn close(handle: i32) -> Result<i32> {
    Ok(unsafe { loaded::<Close>(b"linClose\0")?(handle) })
}
#[napi(js_name = "linGetVersion")]
pub fn get_version(
    major: &mut IntPointer,
    minor: &mut IntPointer,
    build: &mut IntPointer,
) -> Result<i32> {
    let mut a = 0;
    let mut b = 0;
    let mut c = 0;
    let s = unsafe { loaded::<Version>(b"linGetVersion\0")?(&mut a, &mut b, &mut c) };
    major.value = a;
    minor.value = b;
    build.value = c;
    Ok(s)
}
#[napi(js_name = "linSetBitrate")]
pub fn set_bitrate(handle: i32, bps: u32) -> Result<i32> {
    Ok(unsafe { loaded::<SetBitrate>(b"linSetBitrate\0")?(handle, bps) })
}
#[napi(js_name = "linBusOn")]
pub fn bus_on(handle: i32) -> Result<i32> {
    Ok(unsafe { loaded::<Bus>(b"linBusOn\0")?(handle) })
}
#[napi(js_name = "linBusOff")]
pub fn bus_off(handle: i32) -> Result<i32> {
    Ok(unsafe { loaded::<Bus>(b"linBusOff\0")?(handle) })
}
#[napi(js_name = "linWriteMessage")]
pub fn write_message(handle: i32, id: u32, data: Buffer, dlc: Option<u32>) -> Result<i32> {
    let n = dlc.unwrap_or(data.len() as u32).min(data.len() as u32);
    Ok(unsafe { loaded::<Write>(b"linWriteMessage\0")?(handle, id, data.as_ptr() as _, n) })
}
#[napi(js_name = "linRequestMessage")]
pub fn request_message(handle: i32, id: u32) -> Result<i32> {
    Ok(unsafe { loaded::<Request>(b"linRequestMessage\0")?(handle, id) })
}
#[napi(js_name = "linUpdateMessage")]
pub fn update_message(handle: i32, id: u32, data: Buffer, dlc: Option<u32>) -> Result<i32> {
    let n = dlc.unwrap_or(data.len() as u32).min(data.len() as u32);
    Ok(unsafe { loaded::<Update>(b"linUpdateMessage\0")?(handle, id, data.as_ptr() as _, n) })
}
#[napi(js_name = "linWriteWakeup")]
pub fn write_wakeup(handle: i32, count: u32, interval: u32) -> Result<i32> {
    Ok(unsafe { loaded::<Wakeup>(b"linWriteWakeup\0")?(handle, count, interval) })
}
fn read_impl(handle: i32, wait: Option<u32>) -> Result<LinReadResult> {
    let mut id = 0;
    let mut data = [0u8; 8];
    let mut dlc = 0;
    let mut flags = 0;
    let mut info = LinMessageInfo::default();
    let s = unsafe {
        match wait {
            Some(t) => loaded::<ReadWait>(b"linReadMessageWait\0")?(
                handle,
                &mut id,
                data.as_mut_ptr() as _,
                &mut dlc,
                &mut flags,
                &mut info,
                t,
            ),
            None => loaded::<Read>(b"linReadMessage\0")?(
                handle,
                &mut id,
                data.as_mut_ptr() as _,
                &mut dlc,
                &mut flags,
                &mut info,
            ),
        }
    };
    if s < 0 {
        return Err(Error::from_reason(format!("Kvaser LIN status {s}")));
    }
    Ok(LinReadResult {
        id,
        data: data[..dlc.min(8) as usize].to_vec(),
        dlc,
        flags,
        timestamp: info.timestamp,
    })
}
#[napi(js_name = "linReadMessage")]
pub fn read_message(handle: i32) -> Result<LinReadResult> {
    read_impl(handle, None)
}
#[napi(js_name = "linReadMessageWait")]
pub fn read_message_wait(handle: i32, timeout: u32) -> Result<LinReadResult> {
    read_impl(handle, Some(timeout))
}
struct CallbackTask {
    stop: Arc<AtomicBool>,
    join: Option<std::thread::JoinHandle<()>>,
}
static CALLBACKS: OnceLock<Mutex<HashMap<String, CallbackTask>>> = OnceLock::new();
fn callbacks() -> &'static Mutex<HashMap<String, CallbackTask>> {
    CALLBACKS.get_or_init(|| Mutex::new(HashMap::new()))
}
#[napi(js_name = "CreateTSFN")]
pub fn create_tsfn(handle: i32, id: String, callback: Function<'static>) -> Result<()> {
    free_tsfn(id.clone())?;
    let tsfn: ThreadsafeFunction<LinReadResult> = callback
        .build_threadsafe_function()
        .callee_handled()
        .build_callback(|ctx| Ok(ctx.value))?;
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let join = std::thread::spawn(move || {
        while !flag.load(Ordering::Acquire) {
            if let Ok(message) = read_message_wait(handle, 50) {
                let _ = tsfn.call(Ok(message), ThreadsafeFunctionCallMode::NonBlocking);
            }
        }
    });
    callbacks()
        .lock()
        .map_err(|_| Error::from_reason("callback lock poisoned"))?
        .insert(
            id,
            CallbackTask {
                stop,
                join: Some(join),
            },
        );
    Ok(())
}
#[napi(js_name = "linReadMessageLoop")]
pub fn read_message_loop(handle: i32) -> Result<LinReadResult> {
    read_impl(handle, None)
}
#[napi(js_name = "linGetErrorText")]
pub fn get_error_text(code: i32) -> String {
    format!("Kvaser LIN status {code}")
}
#[napi(js_name = "FreeTSFN")]
pub fn free_tsfn(id: String) -> Result<()> {
    if let Some(mut task) = callbacks()
        .lock()
        .map_err(|_| Error::from_reason("callback lock poisoned"))?
        .remove(&id)
    {
        task.stop.store(true, Ordering::Release);
        if let Some(join) = task.join.take() {
            let _ = join.join();
        }
    }
    Ok(())
}
#[napi]
pub struct IntPointer {
    pub value: i32,
}
#[napi]
impl IntPointer {
    #[napi(constructor)]
    pub fn new(value: Option<i32>) -> Self {
        Self {
            value: value.unwrap_or(0),
        }
    }
    #[napi]
    pub fn cast(&self) -> i32 {
        self.value
    }
    #[napi]
    pub fn value(&self) -> i32 {
        self.value
    }
}
#[napi(object)]
pub struct LinReadResult {
    pub id: u32,
    pub data: Vec<u8>,
    pub dlc: u32,
    pub flags: u32,
    pub timestamp: u32,
}
#[napi]
pub const LIN_MASTER: i32 = 1;
#[napi]
pub const LIN_SLAVE: i32 = 2;
#[napi]
pub const LIN_TX: u32 = 1;
#[napi]
pub const LIN_RX: u32 = 2;
#[napi]
pub const LIN_NODATA: u32 = 8;
#[napi]
pub const LIN_CSUM_ERROR: u32 = 16;
#[napi]
pub const LIN_PARITY_ERROR: u32 = 32;
#[napi]
pub fn migration_status() -> String {
    "Kvaser LIN vendor ABI and callback lifecycle implemented in Rust".to_string()
}
