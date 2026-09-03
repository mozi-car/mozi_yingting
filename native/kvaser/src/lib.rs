use napi::bindgen_prelude::*;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi_derive::napi;
use std::collections::HashMap;
use std::ffi::{c_void, CString};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{FreeLibrary, HMODULE};
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA, SetDllDirectoryA};

type Status = i32;
type Init = unsafe extern "system" fn();
type Unload = unsafe extern "system" fn() -> Status;
type Open = unsafe extern "system" fn(i32, i32) -> i32;
type Close = unsafe extern "system" fn(i32) -> Status;
type Bus = unsafe extern "system" fn(i32) -> Status;
type Params = unsafe extern "system" fn(i32, i64, u32, u32, u32, u32, u32) -> Status;
type ParamsFd = unsafe extern "system" fn(i32, i64, u32, u32, u32) -> Status;
type Output = unsafe extern "system" fn(i32, i32) -> Status;
type Write = unsafe extern "system" fn(i32, i64, *mut u8, u32, u32) -> Status;
type Read =
    unsafe extern "system" fn(i32, *mut i64, *mut u8, *mut u32, *mut u32, *mut u64) -> Status;
type Version = unsafe extern "system" fn(i32) -> u32;
type ErrorText = unsafe extern "system" fn(Status, *mut i8, u32) -> Status;
type IoCtl = unsafe extern "system" fn(i32, u32, *mut c_void, u32) -> Status;
type NumChannels = unsafe extern "system" fn(*mut i32) -> Status;
type ChannelData = unsafe extern "system" fn(i32, i32, *mut c_void, usize) -> Status;
type ReadStatus = unsafe extern "system" fn(i32, *mut u64) -> Status;

struct Api(HMODULE);
unsafe impl Send for Api {}
unsafe impl Sync for Api {}
impl Drop for Api {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { FreeLibrary(self.0) };
        }
    }
}
fn sym<T: Copy>(api: &Api, name: &[u8]) -> Result<T> {
    let p = unsafe { GetProcAddress(api.0, name.as_ptr() as _) }.ok_or_else(|| {
        Error::from_reason(format!(
            "Kvaser symbol missing: {}",
            String::from_utf8_lossy(name)
        ))
    })?;
    Ok(unsafe { std::mem::transmute_copy(&p) })
}

#[napi]
pub struct KvaserApi {
    api: Mutex<Option<Api>>,
}
#[napi]
impl KvaserApi {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            api: Mutex::new(None),
        }
    }
    #[napi(js_name = "LoadDll")]
    pub fn load_dll(&self, path: String) -> Result<()> {
        let path = if path.to_ascii_lowercase().ends_with(".dll") { path } else { format!("{}\\canlib32.dll", path.trim_end_matches(|c| c == '\\' || c == '/')) };
        let p = CString::new(path).map_err(|_| Error::from_reason("NUL in DLL path"))?;
        if let Some(parent) = Path::new(p.to_str().unwrap_or_default()).parent() {
            if let Ok(directory) = CString::new(parent.to_string_lossy().as_bytes()) { unsafe { SetDllDirectoryA(directory.as_ptr() as _); } }
        }
        let h = unsafe { LoadLibraryA(p.as_ptr() as _) };
        if h.is_null() {
            return Err(Error::from_reason("failed to load Kvaser CANlib DLL"));
        };
        *self
            .api
            .lock()
            .map_err(|_| Error::from_reason("lock poisoned"))? = Some(Api(h));
        Ok(())
    }
    #[napi(js_name = "IsLoaded")]
    pub fn is_loaded(&self) -> Result<bool> {
        Ok(self
            .api
            .lock()
            .map_err(|_| Error::from_reason("lock poisoned"))?
            .is_some())
    }
    fn with<T: Copy>(&self, name: &[u8]) -> Result<T> {
        let g = self
            .api
            .lock()
            .map_err(|_| Error::from_reason("lock poisoned"))?;
        sym(
            g.as_ref()
                .ok_or_else(|| Error::from_reason("Kvaser DLL is not loaded"))?,
            name,
        )
    }
    #[napi(js_name = "canInitializeLibrary")]
    pub fn initialize(&self) -> Result<()> {
        unsafe { self.with::<Init>(b"canInitializeLibrary\0")?() };
        Ok(())
    }
    #[napi(js_name = "canUnloadLibrary")]
    pub fn unload(&self) -> Result<i32> {
        Ok(unsafe { self.with::<Unload>(b"canUnloadLibrary\0")?() })
    }
    #[napi(js_name = "canOpenChannel")]
    pub fn open(&self, ch: i32, flags: i32) -> Result<i32> {
        Ok(unsafe { self.with::<Open>(b"canOpenChannel\0")?(ch, flags) })
    }
    #[napi(js_name = "canClose")]
    pub fn close(&self, h: i32) -> Result<i32> {
        Ok(unsafe { self.with::<Close>(b"canClose\0")?(h) })
    }
    #[napi(js_name = "canBusOn")]
    pub fn bus_on(&self, h: i32) -> Result<i32> {
        Ok(unsafe { self.with::<Bus>(b"canBusOn\0")?(h) })
    }
    #[napi(js_name = "canBusOff")]
    pub fn bus_off(&self, h: i32) -> Result<i32> {
        Ok(unsafe { self.with::<Bus>(b"canBusOff\0")?(h) })
    }
    #[napi(js_name = "canSetBusParams")]
    pub fn set_params(
        &self,
        h: i32,
        f: i64,
        t1: u32,
        t2: u32,
        sjw: u32,
        samp: u32,
        sync: u32,
    ) -> Result<i32> {
        Ok(unsafe { self.with::<Params>(b"canSetBusParams\0")?(h, f, t1, t2, sjw, samp, sync) })
    }
    #[napi(js_name = "canSetBusParamsFd")]
    pub fn set_params_fd(&self, h: i32, f: i64, t1: u32, t2: u32, sjw: u32) -> Result<i32> {
        Ok(unsafe { self.with::<ParamsFd>(b"canSetBusParamsFd\0")?(h, f, t1, t2, sjw) })
    }
    #[napi(js_name = "canSetBusOutputControl")]
    pub fn output(&self, h: i32, mode: i32) -> Result<i32> {
        Ok(unsafe { self.with::<Output>(b"canSetBusOutputControl\0")?(h, mode) })
    }
    #[napi(js_name = "canWrite")]
    pub fn write(&self, h: i32, id: i64, data: &ByteArray, dlc: u32, flags: u32) -> Result<i32> {
        let mut d = data
            .data
            .lock()
            .map_err(|_| Error::from_reason("buffer lock poisoned"))?;
        Ok(unsafe {
            self.with::<Write>(b"canWrite\0")?(
                h,
                id,
                d.as_mut_ptr(),
                dlc.min(d.len() as u32),
                flags,
            )
        })
    }
    #[napi(js_name = "canRead")]
    pub fn read(
        &self,
        h: i32,
        id: &mut JSINT64,
        data: &ByteArray,
        dlc: &mut JSUINT32,
        flags: &mut JSUINT32,
        time: &mut JSUINT64,
    ) -> Result<i32> {
        let f = self.with::<Read>(b"canRead\0")?;
        let mut idv = 0i64;
        let mut dlcv = 0u32;
        let mut flagsv = 0u32;
        let mut timev = 0u64;
        let mut d = data
            .data
            .lock()
            .map_err(|_| Error::from_reason("buffer lock poisoned"))?;
        let status = unsafe {
            f(
                h,
                &mut idv,
                d.as_mut_ptr(),
                &mut dlcv,
                &mut flagsv,
                &mut timev,
            )
        };
        id.value = idv;
        dlc.value = dlcv;
        flags.value = flagsv;
        time.value = timev as i64;
        Ok(status)
    }
    #[napi(js_name = "canGetErrorText")]
    pub fn error_text(&self, status: i32) -> Result<Buffer> {
        let mut b = vec![0u8; 1024];
        unsafe {
            self.with::<ErrorText>(b"canGetErrorText\0")?(
                status,
                b.as_mut_ptr() as _,
                b.len() as u32,
            )
        };
        Ok(Buffer::from(b))
    }
    #[napi(js_name = "canIoCtl")]
    pub fn ioctl(&self, h: i32, func: u32, mut value: Buffer) -> Result<i32> {
        Ok(unsafe {
            self.with::<IoCtl>(b"canIoCtl\0")?(h, func, value.as_mut_ptr() as _, value.len() as u32)
        })
    }
    #[napi(js_name = "canReadStatus")]
    pub fn read_status(&self, h: i32, flags: &mut JSUINT64) -> Result<i32> {
        let mut value = 0u64;
        let s = unsafe { self.with::<ReadStatus>(b"canReadStatus\0")?(h, &mut value) };
        flags.value = value as i64;
        Ok(s)
    }
    #[napi(js_name = "canGetVersionEx")]
    pub fn version(&self, kind: i32) -> Result<i32> {
        Ok(unsafe { self.with::<Version>(b"canGetVersionEx\0")?(kind) as i32 })
    }
    #[napi(js_name = "canGetNumberOfChannels")]
    pub fn channels(&self) -> Result<i32> {
        let mut n = 0;
        let status = unsafe { self.with::<NumChannels>(b"canGetNumberOfChannels\0")?(&mut n) };
        if status < 0 {
            Err(Error::from_reason(format!(
                "canGetNumberOfChannels failed: {status}"
            )))
        } else {
            Ok(n)
        }
    }
    #[napi(js_name = "canGetChannelData")]
    pub fn channel_data(&self, ch: i32, item: i32, mut b: Buffer) -> Result<i32> {
        Ok(unsafe {
            self.with::<ChannelData>(b"canGetChannelData\0")?(
                ch,
                item,
                b.as_mut_ptr() as _,
                b.len(),
            )
        })
    }
}

#[napi(js_name = "canOK")]
pub const can_ok: i32 = 0;
#[napi(js_name = "canERR_NOMSG")]
pub const can_err_nomsg: i32 = -2;
#[napi(js_name = "canMSG_STD")]
pub const can_msg_std: u32 = 0x0002;
#[napi(js_name = "canMSG_EXT")]
pub const can_msg_ext: u32 = 0x0004;
#[napi(js_name = "canMSG_RTR")]
pub const can_msg_rtr: u32 = 0x0001;
#[napi(js_name = "canMSG_TXACK")]
pub const can_msg_txack: u32 = 0x0040;
#[napi(js_name = "canFDMSG_FDF")]
pub const can_fdmsg_fdf: u32 = 0x0100;
#[napi(js_name = "canFDMSG_BRS")]
pub const can_fdmsg_brs: u32 = 0x0200;
#[napi(js_name = "canOPEN_ACCEPT_VIRTUAL")]
pub const can_open_accept_virtual: i32 = 0x0020;
#[napi(js_name = "canOPEN_CAN_FD")]
pub const can_open_can_fd: i32 = 0x0200;
#[napi(js_name = "canDRIVER_NORMAL")]
pub const can_driver_normal: i32 = 4;
#[napi(js_name = "canDRIVER_SILENT")]
pub const can_driver_silent: i32 = 1;
#[napi(js_name = "canMSG_ERROR_FRAME")]
pub const can_msg_error_frame: u32 = 0x0020;
#[napi(js_name = "canSTAT_BUS_OFF")]
pub const can_stat_bus_off: i64 = 0x0001;
#[napi(js_name = "canSTAT_ERROR_ACTIVE")]
pub const can_stat_error_active: i64 = 0x0002;
#[napi(js_name = "canSTAT_ERROR_PASSIVE")]
pub const can_stat_error_passive: i64 = 0x0004;
#[napi(js_name = "canSTAT_ERROR_WARNING")]
pub const can_stat_error_warning: i64 = 0x0008;
#[napi(js_name = "canSTAT_TX_PENDING")]
pub const can_stat_tx_pending: i64 = 0x0010;
#[napi(js_name = "canSTAT_OVERRUN")]
pub const can_stat_overrun: i64 = 0x0020;
#[napi(js_name = "canCHANNEL_CAP_LIN_HYBRID")]
pub const can_channel_cap_lin_hybrid: i64 = 0x04000000;
#[napi(js_name = "canCHANNELDATA_CHANNEL_NAME")]
pub const can_channeldata_channel_name: i32 = 1;
#[napi(js_name = "canCHANNELDATA_CARD_SERIAL_NO")]
pub const can_channeldata_card_serial_no: i32 = 6;
#[napi(js_name = "canIOCTL_SET_TXACK")]
pub const can_ioctl_set_txack: u32 = 7;
#[napi(js_name = "canIOCTL_SET_TIMER_SCALE")]
pub const can_ioctl_set_timer_scale: u32 = 8;
#[napi(js_name = "canVERSION_CANLIB32_PRODVER")]
pub fn can_version_canlib32_prodver() -> i32 {
    4
}
#[napi(js_name = "canOPEN_ACCEPT_VIRTUAL")]
pub fn can_open_accept_virtual_value() -> i32 {
    can_open_accept_virtual
}
#[napi(js_name = "canOPEN_CAN_FD")]
pub fn can_open_can_fd_value() -> i32 {
    can_open_can_fd
}
#[napi(js_name = "canDRIVER_NORMAL")]
pub fn can_driver_normal_value() -> i32 {
    can_driver_normal
}
#[napi(js_name = "canDRIVER_SILENT")]
pub fn can_driver_silent_value() -> i32 {
    can_driver_silent
}
#[napi(js_name = "canERR_NOMSG")]
pub fn can_err_nomsg_value() -> i32 {
    can_err_nomsg
}
#[napi(js_name = "canMSG_STD")]
pub fn can_msg_std_value() -> u32 {
    can_msg_std
}
#[napi(js_name = "canMSG_EXT")]
pub fn can_msg_ext_value() -> u32 {
    can_msg_ext
}
#[napi(js_name = "canMSG_RTR")]
pub fn can_msg_rtr_value() -> u32 {
    can_msg_rtr
}
#[napi(js_name = "canMSG_TXACK")]
pub fn can_msg_txack_value() -> u32 {
    can_msg_txack
}
#[napi(js_name = "canMSG_ERROR_FRAME")]
pub fn can_msg_error_frame_value() -> u32 {
    can_msg_error_frame
}
#[napi(js_name = "canFDMSG_FDF")]
pub fn can_fdmsg_fdf_value() -> u32 {
    can_fdmsg_fdf
}
#[napi(js_name = "canFDMSG_BRS")]
pub fn can_fdmsg_brs_value() -> u32 {
    can_fdmsg_brs
}
#[napi(js_name = "canSTAT_BUS_OFF")]
pub fn can_stat_bus_off_value() -> i64 {
    can_stat_bus_off
}
#[napi(js_name = "canSTAT_ERROR_ACTIVE")]
pub fn can_stat_error_active_value() -> i64 {
    can_stat_error_active
}
#[napi(js_name = "canSTAT_ERROR_PASSIVE")]
pub fn can_stat_error_passive_value() -> i64 {
    can_stat_error_passive
}
#[napi(js_name = "canSTAT_ERROR_WARNING")]
pub fn can_stat_error_warning_value() -> i64 {
    can_stat_error_warning
}
#[napi(js_name = "canSTAT_TX_PENDING")]
pub fn can_stat_tx_pending_value() -> i64 {
    can_stat_tx_pending
}
#[napi(js_name = "canSTAT_OVERRUN")]
pub fn can_stat_overrun_value() -> i64 {
    can_stat_overrun
}
#[napi(js_name = "canCHANNEL_CAP_LIN_HYBRID")]
pub fn can_channel_cap_lin_hybrid_value() -> i64 {
    can_channel_cap_lin_hybrid
}
#[napi(js_name = "canCHANNELDATA_CHANNEL_NAME")]
pub fn can_channeldata_channel_name_value() -> i32 {
    can_channeldata_channel_name
}
#[napi(js_name = "canCHANNELDATA_CARD_SERIAL_NO")]
pub fn can_channeldata_card_serial_no_value() -> i32 {
    can_channeldata_card_serial_no
}
#[napi(js_name = "canIOCTL_SET_TXACK")]
pub fn can_ioctl_set_txack_value() -> u32 {
    can_ioctl_set_txack
}
#[napi(js_name = "canIOCTL_SET_TIMER_SCALE")]
pub fn can_ioctl_set_timer_scale_value() -> u32 {
    can_ioctl_set_timer_scale
}
#[napi(js_name = "canGetVersionEx")]
pub fn can_get_version_ex(kind: i32) -> Result<i32> {
    default_api().version(kind)
}

static DEFAULT_API: OnceLock<KvaserApi> = OnceLock::new();
fn default_api() -> &'static KvaserApi {
    DEFAULT_API.get_or_init(KvaserApi::new)
}

#[napi(js_name = "LoadDll")]
pub fn load_dll(path: String) -> Result<()> {
    default_api().load_dll(path)
}
#[napi(js_name = "canInitializeLibrary")]
pub fn can_initialize_library() -> Result<()> {
    default_api().initialize()
}
#[napi(js_name = "canUnloadLibrary")]
pub fn can_unload_library() -> Result<i32> {
    default_api().unload()
}
#[napi(js_name = "canOpenChannel")]
pub fn can_open_channel(ch: i32, flags: i32) -> Result<i32> {
    default_api().open(ch, flags)
}
#[napi(js_name = "canClose")]
pub fn can_close(h: i32) -> Result<i32> {
    default_api().close(h)
}
#[napi(js_name = "canBusOn")]
pub fn can_bus_on(h: i32) -> Result<i32> {
    default_api().bus_on(h)
}
#[napi(js_name = "canBusOff")]
pub fn can_bus_off(h: i32) -> Result<i32> {
    default_api().bus_off(h)
}
#[napi(js_name = "canSetBusOutputControl")]
pub fn can_set_output(h: i32, m: i32) -> Result<i32> {
    default_api().output(h, m)
}
#[napi(js_name = "canSetBusParams")]
pub fn can_set_params(h: i32, f: i64, t1: u32, t2: u32, s: u32, n: u32, sy: u32) -> Result<i32> {
    default_api().set_params(h, f, t1, t2, s, n, sy)
}
#[napi(js_name = "canSetBusParamsFd")]
pub fn can_set_params_fd(h: i32, f: i64, t1: u32, t2: u32, s: u32) -> Result<i32> {
    default_api().set_params_fd(h, f, t1, t2, s)
}
#[napi(js_name = "canWrite")]
pub fn can_write(h: i32, id: i64, data: &ByteArray, dlc: u32, flags: u32) -> Result<i32> {
    default_api().write(h, id, data, dlc, flags)
}
#[napi(js_name = "canGetErrorText")]
pub fn can_get_error_text(status: i32) -> Result<Buffer> {
    default_api().error_text(status)
}
#[napi(js_name = "canGetNumberOfChannels")]
pub fn can_get_number_of_channels() -> Result<i32> {
    default_api().channels()
}
#[napi(js_name = "canGetChannelData")]
pub fn can_get_channel_data(ch: i32, item: i32, mut b: Buffer) -> Result<i32> {
    default_api().channel_data(ch, item, b)
}
#[napi(js_name = "canIoCtl")]
pub fn can_ioctl(h: i32, f: u32, b: Buffer) -> Result<i32> {
    default_api().ioctl(h, f, b)
}
#[napi(js_name = "canRead")]
pub fn can_read(
    h: i32,
    id: &mut JSINT64,
    data: &ByteArray,
    dlc: &mut JSUINT32,
    flags: &mut JSUINT32,
    time: &mut JSUINT64,
) -> Result<i32> {
    default_api().read(h, id, data, dlc, flags, time)
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
pub fn create_tsfn(handle: i32, id: String, callback: Function<'static>) -> Result<()> {
    free_tsfn(id.clone())?;
    let tsfn: ThreadsafeFunction<()> = callback
        .build_threadsafe_function()
        .callee_handled()
        .build_callback(|_| Ok(()))?;
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let join = std::thread::spawn(move || {
        while !flag.load(Ordering::Acquire) {
            let mut can_id = JSINT64::new(None);
            let mut data = ByteArray {
                data: Mutex::new(vec![0; 64]),
            };
            let mut dlc = JSUINT32::new(None);
            let mut flags = JSUINT32::new(None);
            let mut time = JSUINT64::new(None);
            match default_api().read(handle, &mut can_id, &data, &mut dlc, &mut flags, &mut time) {
                Ok(status) if status == can_err_nomsg => {
                    std::thread::sleep(Duration::from_millis(2))
                }
                Ok(status) if status >= 0 => {
                    let _ = tsfn.call(Ok(()), ThreadsafeFunctionCallMode::NonBlocking);
                }
                _ => std::thread::sleep(Duration::from_millis(10)),
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
struct PeriodTask {
    stop: Arc<AtomicBool>,
    data: Arc<Mutex<Vec<u8>>>,
    join: Option<JoinHandle<()>>,
}
static PERIODS: OnceLock<Mutex<HashMap<String, PeriodTask>>> = OnceLock::new();
fn periods() -> &'static Mutex<HashMap<String, PeriodTask>> {
    PERIODS.get_or_init(|| Mutex::new(HashMap::new()))
}
#[napi(js_name = "StartPeriodSend")]
pub fn start_period_send(id: String, msg: Object, period: f64, duration: f64) -> Result<String> {
    if !default_api().is_loaded()? {
        return Err(Error::from_reason("Kvaser CANlib DLL is not loaded"));
    }
    let handle: i32 = id
        .parse()
        .map_err(|_| Error::from_reason("Kvaser context id must be channel handle"))?;
    let can_id: i64 = msg.get("id")?.unwrap_or(0i64);
    let extend: bool = msg.get("extendId")?.unwrap_or(false);
    let remote: bool = msg.get("remoteFrame")?.unwrap_or(false);
    let canfd: bool = msg.get("canfd")?.unwrap_or(false);
    let brs: bool = msg.get("brs")?.unwrap_or(false);
    let values: Vec<u8> = msg.get::<Vec<u8>>("data")?.unwrap_or_default();
    let task_id = uuid::Uuid::new_v4().to_string();
    let stop = Arc::new(AtomicBool::new(false));
    let data = Arc::new(Mutex::new(values));
    let loop_stop = stop.clone();
    let loop_data = data.clone();
    let interval = Duration::from_secs_f64(period.max(0.000001));
    let deadline = if duration > 0.0 {
        Some(Instant::now() + Duration::from_secs_f64(duration))
    } else {
        None
    };
    let join = std::thread::spawn(move || {
        while !loop_stop.load(Ordering::Acquire) {
            if let Some(end) = deadline {
                if Instant::now() >= end {
                    break;
                }
            }
            let payload = loop_data.lock().map(|v| v.clone()).unwrap_or_default();
            let b = ByteArray {
                data: Mutex::new(payload),
            };
            let mut flags = if extend { can_msg_ext } else { can_msg_std };
            if remote {
                flags |= can_msg_rtr
            }
            if canfd {
                flags |= can_fdmsg_fdf;
                if brs {
                    flags |= can_fdmsg_brs
                }
            }
            let _ = default_api().write(
                handle,
                can_id,
                &b,
                b.data.lock().map(|v| v.len() as u32).unwrap_or(0),
                flags,
            );
            std::thread::sleep(interval);
        }
    });
    periods()
        .lock()
        .map_err(|_| Error::from_reason("period lock poisoned"))?
        .insert(
            task_id.clone(),
            PeriodTask {
                stop,
                data,
                join: Some(join),
            },
        );
    Ok(task_id)
}
#[napi(js_name = "StopPeriodSend")]
pub fn stop_period_send(id: String) -> Result<()> {
    if let Some(mut task) = periods()
        .lock()
        .map_err(|_| Error::from_reason("period lock poisoned"))?
        .remove(&id)
    {
        task.stop.store(true, Ordering::Release);
        if let Some(j) = task.join.take() {
            let _ = j.join();
        }
    }
    Ok(())
}
#[napi(js_name = "ChangeData")]
pub fn change_data(id: String, data: Vec<u8>) -> Result<()> {
    let tasks = periods()
        .lock()
        .map_err(|_| Error::from_reason("period lock poisoned"))?;
    let task = tasks
        .get(&id)
        .ok_or_else(|| Error::from_reason("period task not found"))?;
    *task
        .data
        .lock()
        .map_err(|_| Error::from_reason("period data lock poisoned"))? = data;
    Ok(())
}
#[napi(js_name = "canReadStatus")]
pub fn can_read_status(h: i32, flags: &mut JSUINT64) -> Result<i32> {
    default_api().read_status(h, flags)
}

#[napi(js_name = "JSINT32")]
pub struct JSINT32 {
    pub value: i32,
}
#[napi]
impl JSINT32 {
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
#[napi(js_name = "JSINT64")]
pub struct JSINT64 {
    pub value: i64,
}
#[napi]
impl JSINT64 {
    #[napi(constructor)]
    pub fn new(value: Option<i64>) -> Self {
        Self {
            value: value.unwrap_or(0),
        }
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
#[napi(js_name = "JSUINT32")]
pub struct JSUINT32 {
    pub value: u32,
}
#[napi]
impl JSUINT32 {
    #[napi(constructor)]
    pub fn new(value: Option<u32>) -> Self {
        Self {
            value: value.unwrap_or(0),
        }
    }
    #[napi]
    pub fn cast(&self) -> u32 {
        self.value
    }
    #[napi]
    pub fn value(&self) -> u32 {
        self.value
    }
}
#[napi(js_name = "JSUINT64")]
pub struct JSUINT64 {
    pub value: i64,
}
#[napi]
impl JSUINT64 {
    #[napi(constructor)]
    pub fn new(value: Option<i64>) -> Self {
        Self {
            value: value.unwrap_or(0),
        }
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
#[napi]
pub struct ByteArray {
    pub data: Mutex<Vec<u8>>,
}
#[napi]
impl ByteArray {
    #[napi(constructor)]
    pub fn new(length: u32) -> Self {
        Self {
            data: Mutex::new(vec![0; length as usize]),
        }
    }
    #[napi]
    pub fn cast(&self) -> i64 {
        self.data.lock().map(|v| v.as_ptr() as i64).unwrap_or(0)
    }
    #[napi]
    pub fn setitem(&self, index: u32, value: u8) -> Result<()> {
        let mut data = self
            .data
            .lock()
            .map_err(|_| Error::from_reason("buffer lock poisoned"))?;
        if let Some(slot) = data.get_mut(index as usize) {
            *slot = value;
        }
        Ok(())
    }
    #[napi]
    pub fn getitem(&self, index: u32) -> Result<u8> {
        Ok(self
            .data
            .lock()
            .map_err(|_| Error::from_reason("buffer lock poisoned"))?
            .get(index as usize)
            .copied()
            .unwrap_or(0))
    }
}
