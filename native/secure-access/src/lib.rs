#![allow(clippy::missing_safety_doc)]

use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::ffi::{c_char, CString};
use std::path::Path;
use std::ptr;
use std::sync::Mutex;
use windows_sys::Win32::Foundation::{FreeLibrary, HMODULE};
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA, SetDllDirectoryA};

type GenerateKeyExOptFn = unsafe extern "system" fn(
    *const u8,
    u32,
    u32,
    *const c_char,
    *const c_char,
    *mut u8,
    u32,
    *mut u32,
) -> i32;

type GenerateKeyExFn =
    unsafe extern "system" fn(*const u8, u32, u32, *const c_char, *mut u8, u32, *mut u32) -> i32;

struct SeedKeyState {
    module: HMODULE,
}

impl Drop for SeedKeyState {
    fn drop(&mut self) {
        if !self.module.is_null() {
            // SAFETY: module is a handle returned by LoadLibraryA and is owned by this state.
            unsafe { FreeLibrary(self.module) };
            self.module = ptr::null_mut();
        }
    }
}

#[napi]
pub struct SeedKey {
    state: Mutex<SeedKeyState>,
}

#[napi]
impl SeedKey {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            state: Mutex::new(SeedKeyState {
                module: ptr::null_mut(),
            }),
        }
    }

    #[napi(js_name = "LoadDLL")]
    pub fn load_dll(&self, path: String) -> Result<()> {
        let path = CString::new(path).map_err(|_| Error::from_reason("DLL path contains NUL"))?;
        if let Ok(path_text) = path.to_str() {
            if let Some(parent) = Path::new(path_text).parent() {
                if let Ok(directory) = CString::new(parent.to_string_lossy().as_bytes()) {
                    unsafe { SetDllDirectoryA(directory.as_ptr() as *const u8); }
                }
            }
        }
        let module = unsafe { LoadLibraryA(path.as_ptr() as *const u8) };
        if module.is_null() {
            return Err(Error::from_reason("failed to load SecureAccess DLL"));
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| Error::from_reason("SeedKey state poisoned"))?;
        if !state.module.is_null() {
            unsafe { FreeLibrary(state.module) };
        }
        state.module = module;
        Ok(())
    }

    #[napi(js_name = "IsLoaded")]
    pub fn is_loaded(&self) -> Result<bool> {
        let state = self
            .state
            .lock()
            .map_err(|_| Error::from_reason("SeedKey state poisoned"))?;
        Ok(!state.module.is_null())
    }

    #[napi(js_name = "GenerateKeyExOpt")]
    #[allow(clippy::too_many_arguments)]
    pub fn generate_key_ex_opt(
        &self,
        seed: Buffer,
        security_level: u32,
        variant: Buffer,
        options: Buffer,
        key: Buffer,
    ) -> Result<Buffer> {
        let state = self
            .state
            .lock()
            .map_err(|_| Error::from_reason("SeedKey state poisoned"))?;
        let function = get_function::<GenerateKeyExOptFn>(state.module, b"GenerateKeyExOpt\0")?;
        let variant = c_string_bytes(&variant)?;
        let options = c_string_bytes(&options)?;
        let mut output = key.to_vec();
        let mut actual_size = 0u32;
        let result = unsafe {
            function(
                seed.as_ref().as_ptr(),
                seed.len() as u32,
                security_level,
                variant.as_ptr() as *const c_char,
                options.as_ptr() as *const c_char,
                output.as_mut_ptr(),
                output.len() as u32,
                &mut actual_size,
            )
        };
        if result != 0 {
            return Err(Error::from_reason(format!(
                "GenerateKeyExOpt failed with error code {result}"
            )));
        }
        output.truncate(actual_size.min(output.len() as u32) as usize);
        Ok(output.into())
    }

    #[napi(js_name = "GenerateKeyEx")]
    pub fn generate_key_ex(
        &self,
        seed: Buffer,
        security_level: u32,
        variant: Buffer,
        key: Buffer,
    ) -> Result<Buffer> {
        let state = self
            .state
            .lock()
            .map_err(|_| Error::from_reason("SeedKey state poisoned"))?;
        let function = get_function::<GenerateKeyExFn>(state.module, b"GenerateKeyEx\0")?;
        let variant = c_string_bytes(&variant)?;
        let mut output = key.to_vec();
        let mut actual_size = 0u32;
        let result = unsafe {
            function(
                seed.as_ref().as_ptr(),
                seed.len() as u32,
                security_level,
                variant.as_ptr() as *const c_char,
                output.as_mut_ptr(),
                output.len() as u32,
                &mut actual_size,
            )
        };
        if result != 0 {
            return Err(Error::from_reason(format!(
                "GenerateKeyEx failed with error code {result}"
            )));
        }
        output.truncate(actual_size.min(output.len() as u32) as usize);
        Ok(output.into())
    }
}

fn c_string_bytes(value: &[u8]) -> Result<Vec<u8>> {
    let mut result = value.to_vec();
    if result.contains(&0) {
        return Err(Error::from_reason("string argument contains NUL"));
    }
    result.push(0);
    Ok(result)
}

fn get_function<T>(module: HMODULE, name: &[u8]) -> Result<T> {
    if module.is_null() {
        return Err(Error::from_reason("DLL not loaded"));
    }
    // SAFETY: module is a valid loaded module and name is NUL-terminated.
    let address = unsafe { GetProcAddress(module, name.as_ptr() as *const u8) };
    let address = address.ok_or_else(|| Error::from_reason("function not found in DLL"))?;
    // SAFETY: caller selects T to match the documented DLL export signature.
    Ok(unsafe { std::mem::transmute_copy::<unsafe extern "system" fn() -> isize, T>(&address) })
}
