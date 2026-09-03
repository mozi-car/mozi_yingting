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

type U8 = unsafe extern "system" fn(i32) -> u8;
type Scan = unsafe extern "system" fn(*mut i32) -> i32;
type CanInit = unsafe extern "system" fn(i32, u8, *const CanInitConfig) -> i32;
type CanInit2 = unsafe extern "system" fn(i32, u8, i32, u8) -> i32;
type CanBus = unsafe extern "system" fn(i32, u8) -> i32;
type CanSend = unsafe extern "system" fn(i32, u8, *const CanMsg, u32) -> i32;
type CanGet = unsafe extern "system" fn(i32, u8, *mut CanMsg, i32) -> i32;
type CanGetOne = unsafe extern "system" fn(i32, u8, *mut CanMsg) -> i32;
type FdInit = unsafe extern "system" fn(i32, u8, *const CanFdConfig) -> i32;
type FdInit2 = unsafe extern "system" fn(i32, u8, i32, i32, u8, u8) -> i32;
type FdSend = unsafe extern "system" fn(i32, u8, *const CanFdMsg, u32) -> i32;
type FdGet = unsafe extern "system" fn(i32, u8, *mut CanFdMsg, i32) -> i32;
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct CanMsg {
    pub id: u32,
    pub timestamp: u32,
    pub remote: u8,
    pub external: u8,
    pub data_len: u8,
    pub data: [u8; 8],
    pub timestamp_high: u8,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct CanInitConfig {
    pub brp: u32,
    pub sjw: u8,
    pub bs1: u8,
    pub bs2: u8,
    pub mode: u8,
    pub abom: u8,
    pub nart: u8,
    pub rflm: u8,
    pub txfp: u8,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct CanFdConfig {
    pub mode: u8,
    pub iso_crc: u8,
    pub retry: u8,
    pub resistor: u8,
    pub nbt_brp: u8,
    pub nbt_seg1: u8,
    pub nbt_seg2: u8,
    pub nbt_sjw: u8,
    pub dbt_brp: u8,
    pub dbt_seg1: u8,
    pub dbt_seg2: u8,
    pub dbt_sjw: u8,
    pub tdc: u8,
    pub reserved: [u8; 7],
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CanFdMsg {
    pub id: u32,
    pub timestamp: u32,
    pub flags: u8,
    pub data_len: u8,
    pub data: [u8; 64],
    pub timestamp_high: u8,
}
impl Default for CanFdMsg {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
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
            "Toomoss symbol missing: {}",
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
        .map_err(|_| Error::from_reason("Toomoss API lock poisoned"))?;
    sym(
        g.as_ref()
            .ok_or_else(|| Error::from_reason("Toomoss DLL is not loaded"))?,
        n,
    )
}
#[napi(js_name = "LoadDll")]
pub fn load_dll(path: String) -> Result<()> {
    let p = if path.to_ascii_lowercase().ends_with(".dll") {
        path
    } else {
        format!("{}\\controlcan.dll", path)
    };
    let c = CString::new(p.clone()).map_err(|_| Error::from_reason("NUL in DLL path"))?;
    if let Some(parent) = Path::new(&p).parent() {
        if let Ok(directory) = CString::new(parent.to_string_lossy().as_bytes()) { unsafe { SetDllDirectoryA(directory.as_ptr() as _); } }
    }
    let h = unsafe { LoadLibraryA(c.as_ptr() as _) };
    if h.is_null() {
        return Err(Error::from_reason("failed to load Toomoss DLL"));
    }
    *api()
        .lock()
        .map_err(|_| Error::from_reason("lock poisoned"))? = Some(Api(h));
    Ok(())
}
#[napi(js_name = "IsLoaded")]
pub fn is_loaded() -> Result<bool> {
    Ok(api()
        .lock()
        .map_err(|_| Error::from_reason("lock poisoned"))?
        .is_some())
}
#[napi(js_name = "USB_ScanDevice")]
pub fn scan(handles: &mut I32Array) -> Result<i32> {
    let mut v = handles
        .values
        .lock()
        .map_err(|_| Error::from_reason("array lock poisoned"))?;
    Ok(unsafe { loaded::<Scan>(b"USB_ScanDevice\0")?(v.as_mut_ptr()) })
}
#[napi(js_name = "USB_OpenDevice")]
pub fn open(h: i32) -> Result<u8> {
    Ok(unsafe { loaded::<U8>(b"USB_OpenDevice\0")?(h) })
}
#[napi(js_name = "USB_CloseDevice")]
pub fn close(h: i32) -> Result<u8> {
    Ok(unsafe { loaded::<U8>(b"USB_CloseDevice\0")?(h) })
}
#[napi(js_name = "USB_ResetDevice")]
pub fn reset(h: i32) -> Result<u8> {
    Ok(unsafe { loaded::<U8>(b"USB_ResetDevice\0")?(h) })
}
#[napi(js_name = "CAN_Init")]
pub fn can_init(h: i32, ch: u8, c: &CanInitConfigJs) -> Result<i32> {
    let r = c.raw();
    Ok(unsafe { loaded::<CanInit>(b"CAN_Init\0")?(h, ch, &r) })
}
#[napi(js_name = "CAN_Init2")]
pub fn can_init2(h: i32, ch: u8, bps: i32, res: u8) -> Result<i32> {
    Ok(unsafe { loaded::<CanInit2>(b"CAN_Init2\0")?(h, ch, bps, res) })
}
#[napi(js_name = "CAN_StartGetMsg")]
pub fn can_start(h: i32, ch: u8) -> Result<i32> {
    Ok(unsafe { loaded::<CanBus>(b"CAN_StartGetMsg\0")?(h, ch) })
}
#[napi(js_name = "CAN_StopGetMsg")]
pub fn can_stop_get(h: i32, ch: u8) -> Result<i32> {
    Ok(unsafe { loaded::<CanBus>(b"CAN_StopGetMsg\0")?(h, ch) })
}
#[napi(js_name = "CAN_Stop")]
pub fn can_stop(h: i32, ch: u8) -> Result<i32> {
    Ok(unsafe { loaded::<CanBus>(b"CAN_Stop\0")?(h, ch) })
}
#[napi(js_name = "CAN_ClearMsg")]
pub fn can_clear(h: i32, ch: u8) -> Result<i32> {
    Ok(unsafe { loaded::<CanBus>(b"CAN_ClearMsg\0")?(h, ch) })
}
#[napi(js_name = "CAN_SendMsg")]
pub fn can_send(h: i32, ch: u8, msg: Object, count: u32) -> Result<i32> {
    let mut r = CanMsg::default();
    r.id = msg.get("ID")?.unwrap_or(0);
    r.remote = msg.get("RemoteFlag")?.unwrap_or(0);
    r.external = msg.get("ExternFlag")?.unwrap_or(0);
    r.data_len = msg.get("DataLen")?.unwrap_or(0);
    let d: Vec<u8> = msg.get("Data")?.unwrap_or_default();
    r.data[..d.len().min(8)].copy_from_slice(&d[..d.len().min(8)]);
    Ok(unsafe { loaded::<CanSend>(b"CAN_SendMsg\0")?(h, ch, &r, count) })
}
#[napi(js_name = "CAN_GetMsg")]
pub fn can_get(h: i32, ch: u8, count: Option<i32>) -> Result<Vec<CanMsgJs>> {
    let n = count.unwrap_or(64).max(1);
    let mut v = vec![CanMsg::default(); n as usize];
    let r = unsafe { loaded::<CanGet>(b"CAN_GetMsgWithSize\0")?(h, ch, v.as_mut_ptr(), n) };
    Ok(v.into_iter()
        .take(r.max(0) as usize)
        .map(CanMsgJs::from_raw)
        .collect())
}
#[napi(js_name = "CANFD_Init")]
pub fn fd_init(h: i32, ch: u8, c: &CanFdConfigJs) -> Result<i32> {
    let r = c.raw();
    Ok(unsafe { loaded::<FdInit>(b"CANFD_Init\0")?(h, ch, &r) })
}
#[napi(js_name = "CANFD_Init2")]
pub fn fd_init2(h: i32, ch: u8, a: i32, d: i32, r: u8, crc: u8) -> Result<i32> {
    Ok(unsafe { loaded::<FdInit2>(b"CANFD_Init2\0")?(h, ch, a, d, r, crc) })
}
#[napi(js_name = "CAN_ResetStartTime")]
pub fn can_reset_time(h: i32, ch: u8) -> Result<i32> {
    Ok(unsafe { loaded::<CanBus>(b"CAN_ResetStartTime\0")?(h, ch) })
}
#[napi(js_name = "CANFD_ResetStartTime")]
pub fn fd_reset_time(h: i32, ch: u8) -> Result<i32> {
    Ok(unsafe { loaded::<CanBus>(b"CANFD_ResetStartTime\0")?(h, ch) })
}
#[napi(js_name = "CANFD_StartGetMsg")]
pub fn fd_start(h: i32, ch: u8) -> Result<i32> {
    Ok(unsafe { loaded::<CanBus>(b"CANFD_StartGetMsg\0")?(h, ch) })
}
#[napi(js_name = "CANFD_Stop")]
pub fn fd_stop(h: i32, ch: u8) -> Result<i32> {
    Ok(unsafe { loaded::<CanBus>(b"CANFD_Stop\0")?(h, ch) })
}
#[napi(js_name = "CANFD_ClearMsg")]
pub fn fd_clear(h: i32, ch: u8) -> Result<i32> {
    Ok(unsafe { loaded::<CanBus>(b"CANFD_ClearMsg\0")?(h, ch) })
}
#[napi(js_name = "CANFD_SendMsg")]
pub fn fd_send(h: i32, ch: u8, msg: Object, count: u32) -> Result<i32> {
    let mut r = CanFdMsg::default();
    r.id = msg.get("ID")?.unwrap_or(0);
    r.data_len = msg.get("DataLen")?.unwrap_or(0);
    let d: Vec<u8> = msg.get("Data")?.unwrap_or_default();
    r.data[..d.len().min(64)].copy_from_slice(&d[..d.len().min(64)]);
    Ok(unsafe { loaded::<FdSend>(b"CANFD_SendMsg\0")?(h, ch, &r, count) })
}
#[napi(js_name = "CANFD_GetMsg")]
pub fn fd_get(h: i32, ch: u8, count: Option<i32>) -> Result<Vec<CanFdMsgJs>> {
    let n = count.unwrap_or(64).max(1);
    let mut v = vec![CanFdMsg::default(); n as usize];
    let r = unsafe { loaded::<FdGet>(b"CANFD_GetMsg\0")?(h, ch, v.as_mut_ptr(), n) };
    Ok(v.into_iter()
        .take(r.max(0) as usize)
        .map(CanFdMsgJs::from_raw)
        .collect())
}
#[napi(js_name = "I32Array")]
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
#[napi(js_name = "CAN_INIT_CONFIG")]
pub struct CanInitConfigJs {
    #[napi(js_name = "CAN_BRP")] pub can_brp: u32,
    #[napi(js_name = "CAN_SJW")] pub can_sjw: u8,
    #[napi(js_name = "CAN_BS1")] pub can_bs1: u8,
    #[napi(js_name = "CAN_BS2")] pub can_bs2: u8,
    #[napi(js_name = "CAN_Mode")] pub can_mode: u8,
    #[napi(js_name = "CAN_ABOM")] pub can_abom: u8,
    #[napi(js_name = "CAN_NART")] pub can_nart: u8,
    #[napi(js_name = "CAN_RFLM")] pub can_rflm: u8,
    #[napi(js_name = "CAN_TXFP")] pub can_txfp: u8,
}
impl CanInitConfigJs {
    fn raw(&self) -> CanInitConfig {
        CanInitConfig {
            brp: self.can_brp,
            sjw: self.can_sjw,
            bs1: self.can_bs1,
            bs2: self.can_bs2,
            mode: self.can_mode,
            abom: self.can_abom,
            nart: self.can_nart,
            rflm: self.can_rflm,
            txfp: self.can_txfp,
        }
    }
}
#[napi]
impl CanInitConfigJs {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            can_brp: 0,
            can_sjw: 1,
            can_bs1: 0,
            can_bs2: 0,
            can_mode: 0,
            can_abom: 0,
            can_nart: 0,
            can_rflm: 0,
            can_txfp: 0,
        }
    }
}
#[napi(js_name = "CANFD_INIT_CONFIG")]
pub struct CanFdConfigJs {
    #[napi(js_name = "Mode")] pub mode: u8,
    #[napi(js_name = "ISOCRCEnable")] pub iso_crc_enable: u8,
    #[napi(js_name = "RetrySend")] pub retry_send: u8,
    #[napi(js_name = "ResEnable")] pub res_enable: u8,
    #[napi(js_name = "NBT_BRP")] pub nbt_brp: u8,
    #[napi(js_name = "NBT_SEG1")] pub nbt_seg1: u8,
    #[napi(js_name = "NBT_SEG2")] pub nbt_seg2: u8,
    #[napi(js_name = "NBT_SJW")] pub nbt_sjw: u8,
    #[napi(js_name = "DBT_BRP")] pub dbt_brp: u8,
    #[napi(js_name = "DBT_SEG1")] pub dbt_seg1: u8,
    #[napi(js_name = "DBT_SEG2")] pub dbt_seg2: u8,
    #[napi(js_name = "DBT_SJW")] pub dbt_sjw: u8,
    #[napi(js_name = "TDC")] pub tdc: u8,
}
impl CanFdConfigJs {
    fn raw(&self) -> CanFdConfig {
        CanFdConfig {
            mode: self.mode,
            iso_crc: self.iso_crc_enable,
            retry: self.retry_send,
            resistor: self.res_enable,
            nbt_brp: self.nbt_brp,
            nbt_seg1: self.nbt_seg1,
            nbt_seg2: self.nbt_seg2,
            nbt_sjw: self.nbt_sjw,
            dbt_brp: self.dbt_brp,
            dbt_seg1: self.dbt_seg1,
            dbt_seg2: self.dbt_seg2,
            dbt_sjw: self.dbt_sjw,
            tdc: self.tdc,
            reserved: [0; 7],
        }
    }
}
#[napi]
impl CanFdConfigJs {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            mode: 0,
            iso_crc_enable: 1,
            retry_send: 0,
            res_enable: 0,
            nbt_brp: 0,
            nbt_seg1: 0,
            nbt_seg2: 0,
            nbt_sjw: 0,
            dbt_brp: 0,
            dbt_seg1: 0,
            dbt_seg2: 0,
            dbt_sjw: 0,
            tdc: 0,
        }
    }
}
#[napi(object)]
pub struct CanMsgJs {
    #[napi(js_name = "ID")] pub id: u32,
    #[napi(js_name = "TimeStamp")] pub timestamp: u32,
    #[napi(js_name = "RemoteFlag")] pub remote_flag: u8,
    #[napi(js_name = "ExternFlag")] pub extern_flag: u8,
    #[napi(js_name = "DataLen")] pub data_len: u8,
    #[napi(js_name = "Data")] pub data: Buffer,
    #[napi(js_name = "TimeStampHigh")] pub timestamp_high: u8,
}
impl CanMsgJs {
    fn raw(&self) -> CanMsg {
        let mut d = [0; 8];
        let n = self.data.len().min(8);
        d[..n].copy_from_slice(&self.data[..n]);
        CanMsg {
            id: self.id,
            timestamp: self.timestamp,
            remote: self.remote_flag,
            external: self.extern_flag,
            data_len: self.data_len.min(8),
            data: d,
            timestamp_high: self.timestamp_high,
        }
    }
    fn from_raw(x: CanMsg) -> Self {
        Self {
            id: x.id,
            timestamp: x.timestamp,
            remote_flag: x.remote,
            extern_flag: x.external,
            data_len: x.data_len,
            data: Buffer::from(x.data[..x.data_len.min(8) as usize].to_vec()),
            timestamp_high: x.timestamp_high,
        }
    }
}
#[napi(object)]
pub struct CanFdMsgJs {
    #[napi(js_name = "ID")] pub id: u32,
    #[napi(js_name = "TimeStamp")] pub timestamp: u32,
    #[napi(js_name = "Flags")] pub flags: u8,
    #[napi(js_name = "DataLen")] pub data_len: u8,
    #[napi(js_name = "Data")] pub data: Buffer,
    #[napi(js_name = "TimeStampHigh")] pub timestamp_high: u8,
}
impl CanFdMsgJs {
    fn raw(&self) -> CanFdMsg {
        let mut d = [0; 64];
        let n = self.data.len().min(64);
        d[..n].copy_from_slice(&self.data[..n]);
        CanFdMsg {
            id: self.id,
            timestamp: self.timestamp,
            flags: self.flags,
            data_len: self.data_len.min(64),
            data: d,
            timestamp_high: self.timestamp_high,
        }
    }
    fn from_raw(x: CanFdMsg) -> Self {
        Self {
            id: x.id,
            timestamp: x.timestamp,
            flags: x.flags,
            data_len: x.data_len,
            data: Buffer::from(x.data[..x.data_len.min(64) as usize].to_vec()),
            timestamp_high: x.timestamp_high,
        }
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
pub fn create_tsfn(handle: i32, id: String, callback: Function<'static>) -> Result<()> {
    free_tsfn(id.clone())?;
    let tsfn: ThreadsafeFunction<CanMsgJs> = callback
        .build_threadsafe_function()
        .callee_handled()
        .build_callback(|ctx| Ok(ctx.value))?;
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let join = std::thread::spawn(move || {
        while !flag.load(Ordering::Acquire) {
            let _ = can_get(handle, 0, Some(1)).map(|frames| {
                frames.into_iter().for_each(|frame| {
                    let _ = tsfn.call(Ok(frame), ThreadsafeFunctionCallMode::NonBlocking);
                })
            });
            std::thread::sleep(std::time::Duration::from_millis(2));
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
#[napi(js_name = "DEV_ResetTimestamp")]
pub fn reset_timestamp(h: i32) -> Result<u8> {
    Ok(unsafe { loaded::<U8>(b"DEV_ResetTimestamp\0")?(h) })
}
#[napi(js_name = "SendCANMsg")]
pub fn send_can_msg(h: i32, ch: u8, fd: bool, _id: String, _cmd: i32, msg: Object) -> Result<i32> {
    if fd {
        fd_send(h, ch, msg, 1)
    } else {
        can_send(h, ch, msg, 1)
    }
}
