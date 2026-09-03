use napi::bindgen_prelude::*;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi_derive::napi;
use std::collections::HashMap;
use std::ffi::CString;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::{Mutex, OnceLock};
use windows_sys::Win32::Foundation::{FreeLibrary, HMODULE};
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA, SetDllDirectoryA};
type U8 = unsafe extern "system" fn(i32) -> u8;
type Scan = unsafe extern "system" fn(*mut i32) -> i32;
type Init = unsafe extern "system" fn(i32, u8, u32, u8) -> i32;
type Stop = unsafe extern "system" fn(i32, u8) -> i32;
type GetMsg = unsafe extern "system" fn(i32, u8, *mut LinExMsg) -> i32;
type GetStatus = unsafe extern "system" fn(i32, u8, *mut LinExStatus) -> i32;
type Power = unsafe extern "system" fn(i32, u8, u8) -> i32;
type Slave = unsafe extern "system" fn(i32, u8, *const LinExMsg, u32) -> i32;
type Send = unsafe extern "system" fn(i32, u8, i32, *const u8, *const LinExMsg) -> i32;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinExMsg {
    pub msg_type: u8,
    pub check_type: u8,
    pub pid: u8,
    pub data_len: u8,
    pub data: [u8; 8],
    pub check: u8,
    pub break_bits: u8,
    pub reserve: [u8; 2],
}
impl Default for LinExMsg {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct LinExStatus {
    pub error: u32,
    pub tx_count: u32,
    pub rx_count: u32,
    pub reserved: [u8; 8],
}
#[napi(object)]
pub struct LinExStatusJs {
    pub error: u32,
    pub tx_count: u32,
    pub rx_count: u32,
}
struct Api(HMODULE);
unsafe impl std::marker::Send for Api {}
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
            "Toomoss LIN symbol missing: {}",
            String::from_utf8_lossy(n)
        ))
    })?;
    Ok(unsafe { std::mem::transmute_copy(&p) })
}
static API: OnceLock<Mutex<Option<Api>>> = OnceLock::new();
fn api() -> &'static Mutex<Option<Api>> {
    API.get_or_init(|| Mutex::new(None))
}
fn loaded<T: Copy>(n: &[u8]) -> Result<T> {
    let g = api()
        .lock()
        .map_err(|_| Error::from_reason("Toomoss LIN lock poisoned"))?;
    sym(
        g.as_ref()
            .ok_or_else(|| Error::from_reason("Toomoss LIN DLL is not loaded"))?,
        n,
    )
}
#[napi(js_name = "LoadDll")]
pub fn load_dll(path: String) -> Result<()> {
    let c = CString::new(path).map_err(|_| Error::from_reason("NUL in DLL path"))?;
    if let Some(parent) = Path::new(c.to_str().unwrap_or_default()).parent() {
        if let Ok(directory) = CString::new(parent.to_string_lossy().as_bytes()) { unsafe { SetDllDirectoryA(directory.as_ptr() as _); } }
    }
    let h = unsafe { LoadLibraryA(c.as_ptr() as _) };
    if h.is_null() {
        return Err(Error::from_reason("failed to load Toomoss LIN DLL"));
    }
    *api()
        .lock()
        .map_err(|_| Error::from_reason("lock poisoned"))? = Some(Api(h));
    Ok(())
}
#[napi(js_name = "USB_ScanDevice")]
pub fn scan(a: &mut I32Array) -> Result<i32> {
    Ok(unsafe {
        loaded::<Scan>(b"USB_ScanDevice\0")?(
            a.values
                .lock()
                .map_err(|_| Error::from_reason("lock poisoned"))?
                .as_mut_ptr(),
        )
    })
}
#[napi(js_name = "USB_OpenDevice")]
pub fn open(h: i32) -> Result<u8> {
    Ok(unsafe { loaded::<U8>(b"USB_OpenDevice\0")?(h) })
}
#[napi(js_name = "USB_CloseDevice")]
pub fn close(h: i32) -> Result<u8> {
    Ok(unsafe { loaded::<U8>(b"USB_CloseDevice\0")?(h) })
}
#[napi(js_name = "DEV_ResetTimestamp")]
pub fn reset_timestamp(h: i32) -> u8 {
    match loaded::<unsafe extern "system" fn(i32) -> u8>(b"DEV_ResetTimestamp\0") {
        Ok(f) => unsafe { f(h) },
        Err(_) => 0,
    }
}
#[napi(js_name = "LIN_EX_Init")]
pub fn init(h: i32, ch: u8, baud: u32, master: u8) -> Result<i32> {
    Ok(unsafe { loaded::<Init>(b"LIN_EX_Init\0")?(h, ch, baud, master) })
}
#[napi(js_name = "LIN_EX_Stop")]
pub fn stop(h: i32, ch: u8) -> Result<i32> {
    Ok(unsafe { loaded::<Stop>(b"LIN_EX_Stop\0")?(h, ch) })
}
#[napi(js_name = "LIN_EX_CtrlPowerOut")]
pub fn power(h: i32, ch: u8, v: u8) -> Result<i32> {
    Ok(unsafe { loaded::<Power>(b"LIN_EX_CtrlPowerOut\0")?(h, ch, v) })
}
#[napi(js_name = "LIN_EX_GetMsg")]
pub fn get_msg(h: i32, ch: u8) -> Result<LinExMsgJs> {
    let mut raw = LinExMsg::default();
    let result = unsafe { loaded::<GetMsg>(b"LIN_EX_GetMsg\0")?(h, ch, &mut raw) };
    if result < 0 {
        return Err(Error::from_reason(format!(
            "LIN_EX_GetMsg failed: {result}"
        )));
    }
    Ok(LinExMsgJs::from_raw(raw, result))
}
#[napi(js_name = "LIN_EX_GetStatus")]
pub fn get_status(h: i32, ch: u8) -> Result<LinExStatusJs> {
    let mut raw = LinExStatus::default();
    let result = unsafe { loaded::<GetStatus>(b"LIN_EX_GetStatus\0")?(h, ch, &mut raw) };
    if result < 0 {
        return Err(Error::from_reason(format!(
            "LIN_EX_GetStatus failed: {result}"
        )));
    }
    Ok(LinExStatusJs {
        error: raw.error,
        tx_count: raw.tx_count,
        rx_count: raw.rx_count,
    })
}
#[napi(js_name = "LIN_EX_SlaveSetIDMode")]
pub fn slave(h: i32, ch: u8, m: &LinExMsgJs, mode: u32) -> Result<i32> {
    let mut d = [0; 8];
    let n = m.data.len().min(8);
    d[..n].copy_from_slice(&m.data[..n]);
    let r = LinExMsg {
        msg_type: m.msg_type,
        check_type: m.check_type,
        pid: m.pid,
        data_len: m.data_len,
        data: d,
        check: m.check,
        break_bits: m.break_bits,
        reserve: [0; 2],
    };
    Ok(unsafe { loaded::<Slave>(b"LIN_EX_SlaveSetIDMode\0")?(h, ch, &r, mode) })
}
#[napi(js_name = "SendLinMsg")]
pub fn send(h: i32, ch: u8, id: i32, name: String, m: &LinExMsgJs) -> Result<i32> {
    let c = CString::new(name).map_err(|_| Error::from_reason("NUL in id"))?;
    let mut d = [0; 8];
    let n = m.data.len().min(8);
    d[..n].copy_from_slice(&m.data[..n]);
    let r = LinExMsg {
        msg_type: m.msg_type,
        check_type: m.check_type,
        pid: m.pid,
        data_len: m.data_len,
        data: d,
        check: m.check,
        break_bits: m.break_bits,
        reserve: [0; 2],
    };
    Ok(unsafe { loaded::<Send>(b"SendLinMsg\0")?(h, ch, id, c.as_ptr() as _, &r) })
}
#[napi(js_name = "LIN_EX_MSG")]
pub struct LinExMsgJs {
    #[napi(js_name = "MsgType")] pub msg_type: u8,
    #[napi(js_name = "CheckType")] pub check_type: u8,
    #[napi(js_name = "PID")] pub pid: u8,
    #[napi(js_name = "DataLen")] pub data_len: u8,
    #[napi(js_name = "Data")] pub data: Vec<u8>,
    #[napi(js_name = "Check")] pub check: u8,
    #[napi(js_name = "BreakBits")] pub break_bits: u8,
}
#[napi]
impl LinExMsgJs {
    fn from_raw(raw: LinExMsg, result: i32) -> Self {
        Self {
            msg_type: raw.msg_type,
            check_type: raw.check_type,
            pid: raw.pid,
            data_len: raw.data_len,
            data: raw.data[..raw.data_len.min(8) as usize].to_vec(),
            check: raw.check,
            break_bits: raw.break_bits,
        }
    }
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            msg_type: 0,
            check_type: 0,
            pid: 0,
            data_len: 0,
            data: vec![0; 8],
            check: 0,
            break_bits: 0,
        }
    }
}
#[napi]
pub struct I32Array {
    pub values: Mutex<Vec<i32>>,
}
#[napi]
impl I32Array {
    #[napi(constructor)]
    pub fn new(n: u32) -> Self {
        Self {
            values: Mutex::new(vec![0; n as usize]),
        }
    }
    #[napi]
    pub fn getitem(&self, i: u32) -> Result<i32> {
        Ok(self
            .values
            .lock()
            .map_err(|_| Error::from_reason("lock poisoned"))?
            .get(i as usize)
            .copied()
            .unwrap_or(0))
    }
}
#[napi(js_name = "ByteArray")]
pub struct ByteArray;
#[napi]
impl ByteArray {
    #[napi(js_name = "frompointer")]
    pub fn from_pointer(_p: i64) -> Self {
        Self
    }
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
pub fn create_tsfn(
    handle: i32,
    channel: u8,
    _fd: bool,
    id: String,
    callback: Function<'static>,
) -> Result<()> {
    free_tsfn(id.clone())?;
    let tsfn: ThreadsafeFunction<LinExMsgJs> = callback
        .build_threadsafe_function()
        .callee_handled()
        .build_callback(|ctx| Ok(ctx.value))?;
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let join = std::thread::spawn(move || {
        while !flag.load(Ordering::Acquire) {
            match get_msg(handle, channel) {
                Ok(message) if message.data_len > 0 || message.msg_type != 0 => {
                    let _ = tsfn.call(Ok(message), ThreadsafeFunctionCallMode::NonBlocking);
                }
                Ok(_) => std::thread::sleep(std::time::Duration::from_millis(1)),
                Err(_) => std::thread::sleep(std::time::Duration::from_millis(5)),
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
pub fn migration_status() -> String {
    "Toomoss LIN vendor ABI and callback lifecycle implemented in Rust".to_string()
}
