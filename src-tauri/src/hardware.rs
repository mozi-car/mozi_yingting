//! Event-driven Windows PnP hardware discovery.
//!
//! This module intentionally only discovers device interfaces. It does not open
//! devices, query vendor SDKs, configure buses, or send traffic.

use serde::Serialize;
use std::collections::HashMap;
use std::sync::{mpsc, Arc, Mutex};
use tauri::{AppHandle, Emitter};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HardwareDevice {
    pub id: String,
    pub vendor: Option<String>,
    pub product: Option<String>,
    pub vid: Option<String>,
    pub pid: Option<String>,
    pub device_instance_id: String,
    pub interface_path: String,
}

#[derive(Default)]
struct HardwareRegistry {
    devices: HashMap<String, HardwareDevice>,
}

impl HardwareRegistry {
    fn add(&mut self, device: HardwareDevice) -> Option<HardwareDevice> {
        if self.devices.contains_key(&device.id) {
            return None;
        }
        self.devices.insert(device.id.clone(), device.clone());
        Some(device)
    }

    fn remove(&mut self, id: &str) -> Option<HardwareDevice> {
        self.devices.remove(id)
    }
}

#[derive(Clone, Copy)]
enum PnpAction {
    Arrival,
    Removal,
}

struct PnpEvent {
    action: PnpAction,
    interface_path: String,
}

pub fn start(app: &AppHandle) {
    #[cfg(windows)]
    {
        if let Err(error) = start_windows(app.clone()) {
            log::error!("hardware PnP monitor failed to start: {error}");
        }
    }

    #[cfg(not(windows))]
    let _ = app;
}

#[cfg(windows)]
fn start_windows(app: AppHandle) -> Result<(), String> {
    let (sender, receiver) = mpsc::channel::<PnpEvent>();
    let registration = WindowsPnpRegistration::register(sender)?;

    // Register first, then enumerate once. This avoids missing an arrival while
    // startup enumeration is in progress.
    let initial = enumerate_usb_interfaces()?;
    let registry = Arc::new(Mutex::new(HardwareRegistry::default()));
    let worker_registry = Arc::clone(&registry);

    std::thread::spawn(move || {
        let _registration = registration;
        for path in initial {
            process_event(
                &app,
                &worker_registry,
                PnpEvent {
                    action: PnpAction::Arrival,
                    interface_path: path,
                },
            );
        }
        while let Ok(event) = receiver.recv() {
            process_event(&app, &worker_registry, event);
        }
    });
    Ok(())
}

#[cfg(windows)]
fn process_event(app: &AppHandle, registry: &Mutex<HardwareRegistry>, event: PnpEvent) {
    let id = event.interface_path.clone();
    match event.action {
        PnpAction::Arrival => {
            let device = device_from_path(event.interface_path);
            if let Some(device) = registry
                .lock()
                .expect("hardware registry poisoned")
                .add(device)
            {
                let _ = app.emit("hardware-added", serde_json::json!({ "device": device }));
            }
        }
        PnpAction::Removal => {
            if registry
                .lock()
                .expect("hardware registry poisoned")
                .remove(&id)
                .is_some()
            {
                let _ = app.emit("hardware-removed", serde_json::json!({ "id": id }));
            }
        }
    }
}

#[cfg(windows)]
fn device_from_path(interface_path: String) -> HardwareDevice {
    let upper = interface_path.to_ascii_uppercase();
    let vid = extract_token(&upper, "VID=");
    let pid = extract_token(&upper, "PID=");
    HardwareDevice {
        id: interface_path.clone(),
        vendor: None,
        product: None,
        vid,
        pid,
        device_instance_id: interface_path.clone(),
        interface_path,
    }
}

#[cfg(windows)]
fn extract_token(value: &str, prefix: &str) -> Option<String> {
    value.split_once(prefix).and_then(|(_, tail)| {
        let token: String = tail.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
        (!token.is_empty()).then_some(token)
    })
}

#[cfg(windows)]
mod windows_pnp {
    use super::{PnpAction, PnpEvent};
    use std::ffi::c_void;
    use std::mem::size_of;
    use std::ptr;
    use std::sync::mpsc::Sender;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Guid {
        data1: u32,
        data2: u16,
        data3: u16,
        data4: [u8; 8],
    }

    // GUID_DEVINTERFACE_USB_DEVICE.
    const USB_DEVICE_GUID: Guid = Guid {
        data1: 0xa5dcbf10,
        data2: 0x6530,
        data3: 0x11d2,
        data4: [0x90, 0x1f, 0x00, 0xc0, 0x4f, 0xb9, 0x51, 0xed],
    };

    const CM_GET_DEVICE_INTERFACE_LIST_PRESENT: u32 = 0;
    const CR_SUCCESS: u32 = 0;
    const CM_NOTIFY_FILTER_TYPE_DEVICEINTERFACE: u32 = 0;
    pub const CM_NOTIFY_ACTION_DEVICEINTERFACEARRIVAL: u32 = 0;
    pub const CM_NOTIFY_ACTION_DEVICEINTERFACEREMOVAL: u32 = 1;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct NotifyFilterDeviceInterface {
        class_guid: Guid,
    }
    #[repr(C)]
    #[derive(Clone, Copy)]
    union NotifyFilterUnion {
        device_interface: NotifyFilterDeviceInterface,
    }
    #[repr(C)]
    struct NotifyFilter {
        cb_size: u32,
        flags: u32,
        filter_type: u32,
        filter: NotifyFilterUnion,
    }
    #[repr(C)]
    struct NotifyEventDataHeader {
        cb_size: u32,
        action: u32,
        event_data_size: u32,
    }

    #[link(name = "cfgmgr32")]
    extern "system" {
        fn CM_Get_Device_Interface_List_SizeW(
            size: *mut u32,
            guid: *const Guid,
            id: *const u16,
            flags: u32,
        ) -> u32;
        fn CM_Get_Device_Interface_ListW(
            guid: *const Guid,
            id: *const u16,
            buffer: *mut u16,
            length: u32,
            flags: u32,
        ) -> u32;
        fn CM_Register_Notification(
            filter: *const NotifyFilter,
            context: *const c_void,
            callback: unsafe extern "system" fn(*const c_void, *const c_void, u32) -> u32,
            notification: *mut *mut c_void,
        ) -> u32;
        fn CM_Unregister_Notification(notification: *mut c_void) -> u32;
    }

    pub struct WindowsPnpRegistration {
        handle: *mut c_void,
        context: *mut Sender<PnpEvent>,
    }

    unsafe impl Send for WindowsPnpRegistration {}

    impl WindowsPnpRegistration {
        pub fn register(sender: Sender<PnpEvent>) -> Result<Self, String> {
            let context = Box::into_raw(Box::new(sender));
            let filter = NotifyFilter {
                cb_size: size_of::<NotifyFilter>() as u32,
                flags: 0,
                filter_type: CM_NOTIFY_FILTER_TYPE_DEVICEINTERFACE,
                filter: NotifyFilterUnion {
                    device_interface: NotifyFilterDeviceInterface {
                        class_guid: USB_DEVICE_GUID,
                    },
                },
            };
            let mut handle = ptr::null_mut();
            let result =
                unsafe { CM_Register_Notification(&filter, context.cast(), callback, &mut handle) };
            if result != CR_SUCCESS {
                unsafe {
                    drop(Box::from_raw(context));
                }
                return Err(format!("CM_Register_Notification failed: {result}"));
            }
            Ok(Self { handle, context })
        }
    }

    impl Drop for WindowsPnpRegistration {
        fn drop(&mut self) {
            unsafe {
                let _ = CM_Unregister_Notification(self.handle);
                drop(Box::from_raw(self.context));
            }
        }
    }

    unsafe extern "system" fn callback(
        context: *const c_void,
        event: *const c_void,
        _reserved: u32,
    ) -> u32 {
        if context.is_null() || event.is_null() {
            return 0;
        }
        let header = &*(event as *const NotifyEventDataHeader);
        if header.action != CM_NOTIFY_ACTION_DEVICEINTERFACEARRIVAL
            && header.action != CM_NOTIFY_ACTION_DEVICEINTERFACEREMOVAL
        {
            return 0;
        }
        // The symbolic-link WCHAR array starts immediately after the fixed
        // header plus the interface class GUID in CM_NOTIFY_EVENT_DATA.
        let path_ptr = (event as *const u8)
            .add(size_of::<NotifyEventDataHeader>() + size_of::<Guid>())
            as *const u16;
        let mut length = 0;
        while *path_ptr.add(length) != 0 {
            length += 1;
        }
        let path = String::from_utf16_lossy(std::slice::from_raw_parts(path_ptr, length));
        let action = if header.action == CM_NOTIFY_ACTION_DEVICEINTERFACEARRIVAL {
            PnpAction::Arrival
        } else {
            PnpAction::Removal
        };
        let _ = (&*(context as *const Sender<PnpEvent>)).send(PnpEvent {
            action,
            interface_path: path,
        });
        0
    }

    pub fn enumerate_usb_interfaces() -> Result<Vec<String>, String> {
        let mut size = 0;
        let result = unsafe {
            CM_Get_Device_Interface_List_SizeW(
                &mut size,
                &USB_DEVICE_GUID,
                ptr::null(),
                CM_GET_DEVICE_INTERFACE_LIST_PRESENT,
            )
        };
        if result != CR_SUCCESS {
            return Err(format!(
                "CM_Get_Device_Interface_List_SizeW failed: {result}"
            ));
        }
        if size == 0 {
            return Ok(Vec::new());
        }
        let mut buffer = vec![0u16; size as usize];
        let result = unsafe {
            CM_Get_Device_Interface_ListW(
                &USB_DEVICE_GUID,
                ptr::null(),
                buffer.as_mut_ptr(),
                size,
                CM_GET_DEVICE_INTERFACE_LIST_PRESENT,
            )
        };
        if result != CR_SUCCESS {
            return Err(format!("CM_Get_Device_Interface_ListW failed: {result}"));
        }
        let mut result = Vec::new();
        let mut start = 0;
        for index in 0..buffer.len() {
            if buffer[index] == 0 {
                if index == start {
                    break;
                }
                result.push(String::from_utf16_lossy(&buffer[start..index]));
                start = index + 1;
            }
        }
        Ok(result)
    }
}

#[cfg(windows)]
use windows_pnp::{enumerate_usb_interfaces, WindowsPnpRegistration};
