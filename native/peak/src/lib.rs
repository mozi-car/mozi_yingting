mod callback;
mod cyclic;
mod dll;
mod message;
mod types;

use crate::dll::{
    symbol, AddMapping, Api, ErrorText, GetValue, Initialize, InitializeFd, Progress, Read,
    RemoveMapping, Reset, SetValue, Uninitialize, Write,
};
use crate::types::{
    CantpMapping, CantpMsg, CantpMsgData, CantpMsgDataCan, CantpMsgDataIsoTp, CantpMsgprogress,
    CantpNetaddrinfo, TimeStamp, PCANTP_MSGTYPE_ANY, PCANTP_MSGTYPE_CAN,
};
use napi::bindgen_prelude::*;
use napi::threadsafe_function::ThreadsafeFunctionCallMode;
use napi_derive::napi;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[napi]
pub struct PeakApi {
    api: Mutex<Option<Api>>,
    tasks: cyclic::Tasks,
}

#[napi]
impl PeakApi {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            api: Mutex::new(None),
            tasks: Default::default(),
        }
    }
    #[napi(js_name = "LoadDll")]
    pub fn load_dll(&self, path: String) -> Result<()> {
        *self
            .api
            .lock()
            .map_err(|_| Error::from_reason("API lock poisoned"))? = Some(dll::load(&path)?);
        Ok(())
    }
    #[napi(js_name = "IsLoaded")]
    pub fn is_loaded(&self) -> Result<bool> {
        Ok(self
            .api
            .lock()
            .map_err(|_| Error::from_reason("API lock poisoned"))?
            .is_some())
    }
    #[napi(js_name = "CANTP_InitializeFD_2016")]
    pub fn initialize_fd(&self, h: u32, bitrate: String) -> Result<i32> {
        let api = self
            .api
            .lock()
            .map_err(|_| Error::from_reason("API lock poisoned"))?;
        let f = symbol::<InitializeFd>(
            api.as_ref()
                .ok_or_else(|| Error::from_reason("PCAN DLL is not loaded"))?,
            b"CANTP_InitializeFD_2016\0",
        )?;
        let s = std::ffi::CString::new(bitrate)
            .map_err(|_| Error::from_reason("bitrate contains NUL"))?;
        Ok(unsafe { f(h, s.as_ptr()) })
    }
    #[napi(js_name = "CANTP_Initialize_2016")]
    pub fn initialize(&self, h: u32, baud: u32, hw: u32, port: u32, irq: u16) -> Result<i32> {
        let api = self
            .api
            .lock()
            .map_err(|_| Error::from_reason("API lock poisoned"))?;
        let f = symbol::<Initialize>(
            api.as_ref()
                .ok_or_else(|| Error::from_reason("PCAN DLL is not loaded"))?,
            b"CANTP_Initialize_2016\0",
        )?;
        Ok(unsafe { f(h, baud, hw, port, irq) })
    }
    #[napi(js_name = "CANTP_Uninitialize_2016")]
    pub fn uninitialize(&self, h: u32) -> Result<i32> {
        let api = self
            .api
            .lock()
            .map_err(|_| Error::from_reason("API lock poisoned"))?;
        let f = symbol::<Uninitialize>(
            api.as_ref()
                .ok_or_else(|| Error::from_reason("PCAN DLL is not loaded"))?,
            b"CANTP_Uninitialize_2016\0",
        )?;
        Ok(unsafe { f(h) })
    }
    #[napi(js_name = "CANTP_GetValue_2016")]
    pub fn get_value(&self, h: u32, p: u32, mut b: Buffer) -> Result<i32> {
        let api = self
            .api
            .lock()
            .map_err(|_| Error::from_reason("API lock poisoned"))?;
        let f = symbol::<GetValue>(
            api.as_ref()
                .ok_or_else(|| Error::from_reason("PCAN DLL is not loaded"))?,
            b"CANTP_GetValue_2016\0",
        )?;
        Ok(unsafe { f(h, p, b.as_mut_ptr(), b.len() as u32) })
    }
    #[napi(js_name = "CANTP_SetValue_2016")]
    pub fn set_value(&self, h: u32, p: u32, b: Buffer) -> Result<i32> {
        let api = self
            .api
            .lock()
            .map_err(|_| Error::from_reason("API lock poisoned"))?;
        let f = symbol::<SetValue>(
            api.as_ref()
                .ok_or_else(|| Error::from_reason("PCAN DLL is not loaded"))?,
            b"CANTP_SetValue_2016\0",
        )?;
        let mut b = b.to_vec();
        Ok(unsafe { f(h, p, b.as_mut_ptr(), b.len() as u32) })
    }
    #[napi(js_name = "CANTP_GetErrorText_2016")]
    pub fn error_text(&self, error: i32, language: u16, mut buffer: Buffer) -> Result<i32> {
        let api = self
            .api
            .lock()
            .map_err(|_| Error::from_reason("API lock poisoned"))?;
        let f = symbol::<ErrorText>(
            api.as_ref()
                .ok_or_else(|| Error::from_reason("PCAN DLL is not loaded"))?,
            b"CANTP_GetErrorText_2016\0",
        )?;
        Ok(unsafe {
            f(
                error,
                language,
                buffer.as_mut_ptr() as *mut i8,
                buffer.len() as u32,
            )
        })
    }
    #[napi(js_name = "CANTP_StatusIsOk_2016")]
    pub fn status_is_ok(&self, status: i32, _ok: i32, strict: bool) -> bool {
        if strict {
            status == 0
        } else {
            status & 0xff == 0
        }
    }
    #[napi(js_name = "CANTP_MsgDataAlloc_2016")]
    pub fn msg_alloc(&self, msg: &mut CantpMsg, typ: u32) -> Result<i32> {
        let api = self
            .api
            .lock()
            .map_err(|_| Error::from_reason("API lock poisoned"))?;
        let f = symbol::<unsafe extern "system" fn(*mut core::ffi::c_void, u32) -> i32>(
            api.as_ref()
                .ok_or_else(|| Error::from_reason("PCAN DLL is not loaded"))?,
            b"CANTP_MsgDataAlloc_2016\0",
        )?;
        msg.sync_to_raw();
        let result = unsafe { f(msg.raw_ptr().cast(), typ) };
        if result == 0 {
            msg.msg_type = typ;
            msg.sync_from_raw();
        }
        Ok(result)
    }
    #[napi(js_name = "CANTP_MsgDataInit_2016")]
    pub fn msg_init(
        &self,
        msg: &mut CantpMsg,
        id: u32,
        typ: u32,
        data: Buffer,
        _addr: Option<Object>,
    ) -> Result<i32> {
        let api = self
            .api
            .lock()
            .map_err(|_| Error::from_reason("API lock poisoned"))?;
        let f = symbol::<
            unsafe extern "system" fn(
                *mut core::ffi::c_void,
                u32,
                u32,
                u32,
                *const u8,
                *mut core::ffi::c_void,
            ) -> i32,
        >(
            api.as_ref()
                .ok_or_else(|| Error::from_reason("PCAN DLL is not loaded"))?,
            b"CANTP_MsgDataInit_2016\0",
        )?;
        let p = data.as_ptr();
        let mut netaddr = if let Some(addr) = _addr {
            Some(crate::types::RawNetaddr {
                msgtype: addr.get("msgtype")?.unwrap_or(0),
                format: addr.get("format")?.unwrap_or(0),
                target_type: addr.get("target_type")?.unwrap_or(0),
                source_addr: addr.get("source_addr")?.unwrap_or(0),
                target_addr: addr.get("target_addr")?.unwrap_or(0),
                extension_addr: addr.get("extension_addr")?.unwrap_or(0),
                _padding: [0; 3],
            })
        } else { None };
        let netaddr_ptr: *mut crate::types::RawNetaddr = netaddr.as_mut().map_or(std::ptr::null_mut(), |v| v as *mut _);
        msg.sync_to_raw();
        let result = unsafe {
            f(msg.raw_ptr().cast(), id, typ, data.len() as u32, p, netaddr_ptr.cast())
        };
        if result == 0 { msg.sync_from_raw(); }
        Ok(result)
    }
    #[napi(js_name = "CANTP_MsgDataFree_2016")]
    pub fn msg_free(&self, msg: &mut CantpMsg) -> Result<i32> {
        let api = self
            .api
            .lock()
            .map_err(|_| Error::from_reason("API lock poisoned"))?;
        let f = symbol::<unsafe extern "system" fn(*mut core::ffi::c_void) -> i32>(
            api.as_ref()
                .ok_or_else(|| Error::from_reason("PCAN DLL is not loaded"))?,
            b"CANTP_MsgDataFree_2016\0",
        )?;
        let result = unsafe { f(msg.raw_ptr().cast()) };
        if result == 0 { msg.raw = Box::new(crate::types::RawCantpMsg::default()); }
        Ok(result)
    }
    #[napi(js_name = "CANTP_AddMapping_2016")]
    pub fn add_mapping(&self, handle: u32, mapping: &mut CantpMapping) -> Result<i32> {
        let api = self
            .api
            .lock()
            .map_err(|_| Error::from_reason("API lock poisoned"))?;
        let f = symbol::<AddMapping>(
            api.as_ref()
                .ok_or_else(|| Error::from_reason("PCAN DLL is not loaded"))?,
            b"CANTP_AddMapping_2016\0",
        )?;
        let mut raw = mapping.raw();
        let result = unsafe { f(handle, &mut raw) };
        if result == 0 { mapping.update_from_raw(&raw); }
        Ok(result)
    }
    #[napi(js_name = "CANTP_RemoveMapping_2016")]
    pub fn remove_mapping(&self, handle: u32, uid: i64) -> Result<i32> {
        let api = self
            .api
            .lock()
            .map_err(|_| Error::from_reason("API lock poisoned"))?;
        let f = symbol::<RemoveMapping>(
            api.as_ref()
                .ok_or_else(|| Error::from_reason("PCAN DLL is not loaded"))?,
            b"CANTP_RemoveMapping_2016\0",
        )?;
        Ok(unsafe { f(handle, uid as usize) })
    }
    #[napi(js_name = "CANTP_GetMsgProgress_2016")]
    pub fn progress(
        &self,
        handle: u32,
        msg: &mut CantpMsg,
        direction: u32,
        progress: &mut CantpMsgprogress,
    ) -> Result<i32> {
        let api = self
            .api
            .lock()
            .map_err(|_| Error::from_reason("API lock poisoned"))?;
        let f = symbol::<Progress>(
            api.as_ref()
                .ok_or_else(|| Error::from_reason("PCAN DLL is not loaded"))?,
            b"CANTP_GetMsgProgress_2016\0",
        )?;
        msg.sync_to_raw();
        let mut raw_progress = progress.raw_ptr();
        let result = unsafe { f(handle, msg.raw_ptr(), direction, &mut raw_progress) };
        if result == 0 { progress.sync_from_raw(&raw_progress); }
        Ok(result)
    }
    #[napi(js_name = "CANTP_Write_2016")]
    pub fn write(&self, handle: u32, msg: &mut CantpMsg) -> Result<i32> {
        let api = self
            .api
            .lock()
            .map_err(|_| Error::from_reason("API lock poisoned"))?;
        let f = symbol::<Write>(
            api.as_ref()
                .ok_or_else(|| Error::from_reason("PCAN DLL is not loaded"))?,
            b"CANTP_Write_2016\0",
        )?;
        msg.sync_to_raw();
        Ok(unsafe { f(handle, msg.raw_ptr()) })
    }
    #[napi(js_name = "CANTP_Read_2016")]
    pub fn read(&self, handle: u32, msg: &mut CantpMsg, timestamp: &mut TimeStamp, typ: u32) -> Result<i32> {
        let api = self
            .api
            .lock()
            .map_err(|_| Error::from_reason("API lock poisoned"))?;
        let f = symbol::<Read>(
            api.as_ref()
                .ok_or_else(|| Error::from_reason("PCAN DLL is not loaded"))?,
            b"CANTP_Read_2016\0",
        )?;
        msg.sync_to_raw();
        let result = unsafe { f(handle, msg.raw_ptr(), (&mut timestamp.value as *mut i64).cast(), typ) };
        if result == 0 { msg.sync_from_raw(); }
        Ok(result)
    }
    #[napi(js_name = "CANTP_MsgDataInitOptions_2016")]
    pub fn msg_options(&self, msg: &mut CantpMsg, count: u32) -> Result<i32> {
        type MsgOptions = unsafe extern "system" fn(*mut crate::types::RawCantpMsg, u32) -> i32;
        let api = self.api.lock().map_err(|_| Error::from_reason("API lock poisoned"))?;
        let f = symbol::<MsgOptions>(api.as_ref().ok_or_else(|| Error::from_reason("PCAN DLL is not loaded"))?, b"CANTP_MsgDataInitOptions_2016\\0")?;
        msg.sync_to_raw();
        Ok(unsafe { f(msg.raw_ptr(), count) })
    }
    #[napi(js_name = "CANTP_Reset_2016")]
    pub fn reset(&self, handle: u32) -> Result<i32> {
        let api = self
            .api
            .lock()
            .map_err(|_| Error::from_reason("API lock poisoned"))?;
        let f = symbol::<Reset>(
            api.as_ref()
                .ok_or_else(|| Error::from_reason("PCAN DLL is not loaded"))?,
            b"CANTP_Reset_2016\0",
        )?;
        Ok(unsafe { f(handle) })
    }
}

fn global_symbol<T: Copy>(name: &[u8]) -> Result<T> {
    let guard = global_api().lock().map_err(|_| Error::from_reason("API lock poisoned"))?;
    dll::symbol(guard.as_ref().ok_or_else(|| Error::from_reason("PCAN DLL is not loaded"))?, name)
}
#[napi(js_name = "CANTP_InitializeFD_2016")]
pub fn cantp_initialize_fd(handle: u32, bitrate: String) -> Result<i32> {
    let f = global_symbol::<InitializeFd>(b"CANTP_InitializeFD_2016\0")?;
    let bitrate = std::ffi::CString::new(bitrate).map_err(|_| Error::from_reason("bitrate contains NUL"))?;
    Ok(unsafe { f(handle, bitrate.as_ptr()) })
}
#[napi(js_name = "CANTP_Initialize_2016")]
pub fn cantp_initialize(handle: u32, baud: u32, hw: Option<u32>, port: Option<u32>, irq: Option<u16>) -> Result<i32> {
    Ok(unsafe { global_symbol::<Initialize>(b"CANTP_Initialize_2016\0")?(handle, baud, hw.unwrap_or(0), port.unwrap_or(0), irq.unwrap_or(0)) })
}
#[napi(js_name = "CANTP_Uninitialize_2016")]
pub fn cantp_uninitialize(handle: u32) -> Result<i32> { Ok(unsafe { global_symbol::<Uninitialize>(b"CANTP_Uninitialize_2016\0")?(handle) }) }
#[napi(js_name = "CANTP_Reset_2016")]
pub fn cantp_reset(handle: u32) -> Result<i32> { Ok(unsafe { global_symbol::<Reset>(b"CANTP_Reset_2016\0")?(handle) }) }
#[napi(js_name = "CANTP_GetValue_2016")]
pub fn cantp_get_value(handle: u32, parameter: u32, mut buffer: Buffer) -> Result<i32> {
    Ok(unsafe { global_symbol::<GetValue>(b"CANTP_GetValue_2016\0")?(handle, parameter, buffer.as_mut_ptr(), buffer.len() as u32) })
}
#[napi(js_name = "CANTP_SetValue_2016")]
pub fn cantp_set_value(handle: u32, parameter: u32, buffer: Buffer) -> Result<i32> {
    let mut buffer = buffer.to_vec();
    Ok(unsafe { global_symbol::<SetValue>(b"CANTP_SetValue_2016\0")?(handle, parameter, buffer.as_mut_ptr(), buffer.len() as u32) })
}
#[napi(js_name = "CANTP_GetErrorText_2016")]
pub fn cantp_error_text(error: i32, language: u16, mut buffer: Buffer) -> Result<i32> {
    Ok(unsafe { global_symbol::<ErrorText>(b"CANTP_GetErrorText_2016\0")?(error, language, buffer.as_mut_ptr().cast(), buffer.len() as u32) })
}
#[napi(js_name = "CANTP_MsgDataAlloc_2016")]
pub fn cantp_msg_alloc(msg: &mut CantpMsg, typ: u32) -> Result<i32> {
    msg.sync_to_raw();
    let f = global_symbol::<unsafe extern "system" fn(*mut crate::types::RawCantpMsg, u32) -> i32>(b"CANTP_MsgDataAlloc_2016\0")?;
    let result = unsafe { f(msg.raw_ptr(), typ) };
    if result == 0 { msg.msg_type = typ; msg.sync_from_raw(); }
    Ok(result)
}
#[napi(js_name = "CANTP_MsgDataInit_2016")]
pub fn cantp_msg_init(msg: &mut CantpMsg, id: u32, typ: u32, data: Buffer, addr: Option<Object>) -> Result<i32> {
    let mut raw_addr = addr.map(|a| crate::types::RawNetaddr { msgtype: a.get("msgtype").unwrap_or(Some(0)).unwrap_or(0), format: a.get("format").unwrap_or(Some(0)).unwrap_or(0), target_type: a.get("target_type").unwrap_or(Some(0)).unwrap_or(0), source_addr: a.get("source_addr").unwrap_or(Some(0)).unwrap_or(0), target_addr: a.get("target_addr").unwrap_or(Some(0)).unwrap_or(0), extension_addr: a.get("extension_addr").unwrap_or(Some(0)).unwrap_or(0), _padding: [0; 3] });
    let addr_ptr: *mut crate::types::RawNetaddr = raw_addr.as_mut().map_or(std::ptr::null_mut(), |v| v as *mut _);
    let f = global_symbol::<unsafe extern "system" fn(*mut crate::types::RawCantpMsg, u32, u32, u32, *const u8, *mut crate::types::RawNetaddr) -> i32>(b"CANTP_MsgDataInit_2016\0")?;
    let result = unsafe { f(msg.raw_ptr(), id, typ, data.len() as u32, data.as_ptr(), addr_ptr) };
    if result == 0 { msg.sync_from_raw(); }
    Ok(result)
}
#[napi(js_name = "CANTP_MsgDataInitOptions_2016")]
pub fn cantp_msg_options(msg: &mut CantpMsg, count: u32) -> Result<i32> { let f = global_symbol::<unsafe extern "system" fn(*mut crate::types::RawCantpMsg, u32) -> i32>(b"CANTP_MsgDataInitOptions_2016\0")?; Ok(unsafe { f(msg.raw_ptr(), count) }) }
#[napi(js_name = "CANTP_MsgDataFree_2016")]
pub fn cantp_msg_free(msg: &mut CantpMsg) -> Result<i32> { let f = global_symbol::<unsafe extern "system" fn(*mut crate::types::RawCantpMsg) -> i32>(b"CANTP_MsgDataFree_2016\0")?; let result = unsafe { f(msg.raw_ptr()) }; if result == 0 { msg.raw = Box::new(crate::types::RawCantpMsg::default()); } Ok(result) }
#[napi(js_name = "CANTP_AddMapping_2016")]
pub fn cantp_add_mapping(handle: u32, mapping: &mut CantpMapping) -> Result<i32> { let f = global_symbol::<AddMapping>(b"CANTP_AddMapping_2016\0")?; let mut raw = mapping.raw(); let result = unsafe { f(handle, &mut raw) }; if result == 0 { mapping.update_from_raw(&raw); } Ok(result) }
#[napi(js_name = "CANTP_RemoveMapping_2016")]
pub fn cantp_remove_mapping(handle: u32, uid: i64) -> Result<i32> { Ok(unsafe { global_symbol::<RemoveMapping>(b"CANTP_RemoveMapping_2016\0")?(handle, uid.max(0) as usize) }) }
#[napi(js_name = "CANTP_GetMsgProgress_2016")]
pub fn cantp_progress(handle: u32, msg: &mut CantpMsg, direction: u32, progress: &mut CantpMsgprogress) -> Result<i32> { let f = global_symbol::<Progress>(b"CANTP_GetMsgProgress_2016\0")?; msg.sync_to_raw(); let mut raw = progress.raw_ptr(); let result = unsafe { f(handle, msg.raw_ptr(), direction, &mut raw) }; if result == 0 { progress.sync_from_raw(&raw); } Ok(result) }
#[napi(js_name = "CANTP_Write_2016")]
pub fn cantp_write(handle: u32, msg: &mut CantpMsg) -> Result<i32> { let f = global_symbol::<Write>(b"CANTP_Write_2016\0")?; msg.sync_to_raw(); Ok(unsafe { f(handle, msg.raw_ptr()) }) }
#[napi(js_name = "CANTP_Read_2016")]
pub fn cantp_read(handle: u32, msg: &mut CantpMsg, timestamp: &mut TimeStamp, typ: Option<u32>) -> Result<i32> { let f = global_symbol::<Read>(b"CANTP_Read_2016\0")?; let result = unsafe { f(handle, msg.raw_ptr(), (&mut timestamp.value as *mut i64).cast(), typ.unwrap_or(PCANTP_MSGTYPE_ANY)) }; if result == 0 { msg.sync_from_raw(); } Ok(result) }
#[napi(js_name = "CANTP_StatusIsOk_2016")]
pub fn cantp_status_is_ok(status: i32, expected: Option<i32>, strict: Option<bool>) -> bool { let expected = expected.unwrap_or(0); if strict.unwrap_or(false) { status == expected } else { status & 0xff == expected & 0xff } }

static CALLBACKS: OnceLock<Mutex<HashMap<String, callback::CallbackState>>> = OnceLock::new();
fn callbacks() -> &'static Mutex<HashMap<String, callback::CallbackState>> {
    CALLBACKS.get_or_init(|| Mutex::new(HashMap::new()))
}
static GLOBAL_API: OnceLock<Mutex<Option<Api>>> = OnceLock::new();
static GLOBAL_TASKS: OnceLock<cyclic::Tasks> = OnceLock::new();
fn global_api() -> &'static Mutex<Option<Api>> {
    GLOBAL_API.get_or_init(|| Mutex::new(None))
}
fn global_tasks() -> &'static cyclic::Tasks {
    GLOBAL_TASKS.get_or_init(|| std::sync::Arc::new(Mutex::new(std::collections::HashMap::new())))
}
#[napi(js_name = "LoadDll")]
pub fn load_dll(path: String) -> Result<()> {
    *global_api()
        .lock()
        .map_err(|_| Error::from_reason("API lock poisoned"))? = Some(dll::load(&path)?);
    Ok(())
}
#[napi(js_name = "IsLoaded")]
pub fn is_loaded() -> Result<bool> {
    Ok(global_api()
        .lock()
        .map_err(|_| Error::from_reason("API lock poisoned"))?
        .is_some())
}
#[napi(js_name = "CreateTSFN")]
pub fn CreateTSFN(
    handle: u32,
    name: String,
    callback: Function<'static>,
    _canfd: bool,
) -> Result<()> {
    if !global_api().lock().map_err(|_| Error::from_reason("API lock poisoned"))?.is_some() {
        return Err(Error::from_reason("PCAN DLL is not loaded"));
    }
    FreeTSFN(name.clone())?;
    let state = callback::CallbackState::new(callback)?;
    let state_ref = state.callback.clone();
    let _ = handle;
    std::thread::spawn(move || {
        // PCAN-ISO-TP supports a receive-event parameter, but the existing
        // TypeScript contract performs CANTP_Read itself. Poll only the TSFN;
        // reading here would consume the message before JavaScript sees it.
        while let Ok(guard) = state_ref.lock() {
            if let Some(tsfn) = guard.as_ref() {
                let _ = tsfn.call(Ok(()), ThreadsafeFunctionCallMode::NonBlocking);
            } else {
                break;
            }
            drop(guard);
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    });
    callbacks()
        .lock()
        .map_err(|_| Error::from_reason("callback lock poisoned"))?
        .insert(name, state);
    Ok(())
}
#[napi(js_name = "FreeTSFN")]
pub fn FreeTSFN(name: String) -> Result<()> {
    if let Some(state) = callbacks()
        .lock()
        .map_err(|_| Error::from_reason("callback lock poisoned"))?
        .remove(&name)
    {
        if let Ok(mut callback) = state.callback.lock() {
            *callback = None;
        }
    }
    Ok(())
}
#[napi(js_name = "StartPeriodSend")]
pub fn start_period_send(_name: String, message: Object, period: f64) -> Result<String> {
    if !global_api().lock().map_err(|_| Error::from_reason("API lock poisoned"))?.is_some() {
        return Err(Error::from_reason("PCAN DLL is not loaded"));
    }
    if !period.is_finite() || period <= 0.0 {
        return Err(Error::from_reason("period must be positive"));
    }
    let handle: u32 = message.get("handle")?.unwrap_or(0);
    let can_id: u32 = message.get("id")?.unwrap_or(0);
    let data: Vec<u8> = message.get("data")?.unwrap_or_default();
    let id = uuid::Uuid::new_v4().to_string();
    let shared = std::sync::Arc::new(std::sync::Mutex::new(data));
    let loop_data = shared.clone();
    let task_id = id.clone();
    let worker = std::thread::spawn(move || loop {
        let bytes = loop_data.lock().map(|v| v.clone()).unwrap_or_default();
        let mut msg = CantpMsg::new();
        message::init(&mut msg, can_id, PCANTP_MSGTYPE_CAN, &bytes);
        let _ = global_api()
            .lock()
            .ok()
            .and_then(|api| {
                api.as_ref()
                    .map(|api| symbol::<Write>(api, b"CANTP_Write_2016\0").ok())
            })
            .flatten()
            .map(|f| unsafe { f(handle, msg.raw_ptr()) });
        std::thread::sleep(std::time::Duration::from_secs_f64(period / 1000.0));
        if global_tasks()
            .lock()
            .map(|m| !m.contains_key(&task_id))
            .unwrap_or(true)
        {
            break;
        }
    });
    if let Ok(mut tasks) = global_tasks().lock() {
        tasks.insert(
            id.clone(),
            cyclic::Task {
                data: shared,
                stop: std::sync::Arc::new(std::sync::Mutex::new(false)),
                worker: Some(worker),
            },
        );
    }
    Ok(id)
}
#[napi(js_name = "StopPeriodSend")]
pub fn stop_period_send(id: String) -> Result<()> {
    if cyclic::stop(global_tasks(), &id) {
        Ok(())
    } else {
        Err(Error::from_reason("period task not found"))
    }
}
#[napi(js_name = "ChangeData")]
pub fn change_data(id: String, data: Vec<u8>) -> Result<()> {
    let tasks = global_tasks()
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
#[napi(js_name = "GetMsgDataIsoTp")]
pub fn get_msg_data_iso_tp(msg: &CantpMsg) -> Result<CantpMsgDataIsoTp> {
    unsafe { message::iso_from_raw(&msg.raw).ok_or_else(|| Error::from_reason("message has no ISO-TP data")) }
}
#[napi(js_name = "GetMsgDataAny")]
pub fn get_msg_data_any(msg: &CantpMsg) -> Result<CantpMsgData> {
    unsafe { message::from_raw(&msg.raw).ok_or_else(|| Error::from_reason("message has no data")) }
}
#[napi(js_name = "GetMsgDataCan")]
pub fn get_msg_data_can(msg: &CantpMsg) -> Result<CantpMsgDataCan> {
    unsafe { message::can_from_raw(&msg.raw).ok_or_else(|| Error::from_reason("message has no CAN data")) }
}
