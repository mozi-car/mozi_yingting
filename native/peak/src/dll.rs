use napi::bindgen_prelude::*;
use std::ffi::{c_char, CString};
use std::path::Path;
use windows_sys::Win32::Foundation::{FreeLibrary, HMODULE};
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA, SetDllDirectoryA};

pub struct Api {
    pub module: HMODULE,
}
unsafe impl Send for Api {}
unsafe impl Sync for Api {}
impl Drop for Api {
    fn drop(&mut self) {
        if !self.module.is_null() {
            unsafe { FreeLibrary(self.module) };
        }
    }
}

pub type Status = i32;
pub type Initialize = unsafe extern "system" fn(u32, u32, u32, u32, u16) -> Status;
pub type InitializeFd = unsafe extern "system" fn(u32, *const c_char) -> Status;
pub type Uninitialize = unsafe extern "system" fn(u32) -> Status;
use crate::types::{RawCantpMsg, RawCantpProgress, RawMapping};
pub type Read = unsafe extern "system" fn(u32, *mut RawCantpMsg, *mut u64, u32) -> Status;
pub type Write = unsafe extern "system" fn(u32, *mut RawCantpMsg) -> Status;
pub type Progress = unsafe extern "system" fn(u32, *mut RawCantpMsg, u32, *mut RawCantpProgress) -> Status;
pub type AddMapping = unsafe extern "system" fn(u32, *mut RawMapping) -> Status;
pub type RemoveMapping = unsafe extern "system" fn(u32, usize) -> Status;
pub type Reset = unsafe extern "system" fn(u32) -> Status;
pub type ReadFull = unsafe extern "system" fn(u32, *mut RawCantpMsg, *mut u64, u32) -> Status;
pub type GetValue = unsafe extern "system" fn(u32, u32, *mut u8, u32) -> Status;
pub type SetValue = unsafe extern "system" fn(u32, u32, *mut u8, u32) -> Status;
pub type ErrorText = unsafe extern "system" fn(i32, u16, *mut c_char, u32) -> Status;

pub fn load(path: &str) -> Result<Api> {
    let path = if path.to_ascii_lowercase().ends_with(".dll") { path.to_owned() } else { format!("{}\\PCAN-ISO-TP.dll", path.trim_end_matches(|c| c == '\\' || c == '/')) };
    let path = CString::new(path).map_err(|_| Error::from_reason("DLL path contains NUL"))?;
    if let Some(parent) = Path::new(path.to_str().unwrap_or_default()).parent() {
        if let Ok(directory) = CString::new(parent.to_string_lossy().as_bytes()) {
            unsafe { SetDllDirectoryA(directory.as_ptr() as *const u8); }
        }
    }
    let module = unsafe { LoadLibraryA(path.as_ptr() as *const u8) };
    if module.is_null() {
        return Err(Error::from_reason("failed to load PCAN DLL"));
    }
    Ok(Api { module })
}

pub fn symbol<T: Copy>(api: &Api, name: &[u8]) -> Result<T> {
    let ptr =
        unsafe { GetProcAddress(api.module, name.as_ptr() as *const u8) }.ok_or_else(|| {
            Error::from_reason(format!(
                "PCAN symbol not found: {}",
                String::from_utf8_lossy(name)
            ))
        })?;
    Ok(unsafe { std::mem::transmute_copy(&ptr) })
}
