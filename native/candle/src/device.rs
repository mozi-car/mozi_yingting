use crate::frame::{CandleBittiming, CandleFrame};
use std::ffi::c_void;
use std::ptr::null_mut;
use std::sync::{Arc, Mutex};
use windows_sys::core::GUID;
use windows_sys::Win32::Devices::DeviceAndDriverInstallation::{
    SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInterfaces, SetupDiGetClassDevsW,
    SetupDiGetDeviceInterfaceDetailW, SetupDiGetDeviceRegistryPropertyW, DIGCF_DEVICEINTERFACE,
    DIGCF_PRESENT, SPDRP_DEVICEDESC, SPDRP_FRIENDLYNAME, SP_DEVICE_INTERFACE_DATA,
    SP_DEVICE_INTERFACE_DETAIL_DATA_W, SP_DEVINFO_DATA,
};
use windows_sys::Win32::Devices::Usb::{
    WinUsb_ControlTransfer, WinUsb_Free, WinUsb_Initialize, WinUsb_QueryInterfaceSettings,
    WinUsb_QueryPipe, WinUsb_ReadPipe, WinUsb_SetPipePolicy, WinUsb_WritePipe,
    USB_ENDPOINT_DIRECTION_MASK, WINUSB_INTERFACE_HANDLE, WINUSB_PIPE_INFORMATION,
    WINUSB_SETUP_PACKET,
};
use windows_sys::Win32::Foundation::{
    CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_FLAG_OVERLAPPED, FILE_SHARE_READ, FILE_SHARE_WRITE,
    OPEN_EXISTING,
};
use windows_sys::Win32::System::IO::OVERLAPPED;

pub type DeviceRef = Arc<Mutex<Device>>;

pub fn scan_devices() -> Result<Vec<Device>, String> {
    let interface_guid = GUID {
        data1: 0xc15b4308,
        data2: 0x04d3,
        data3: 0x11e6,
        data4: [0xb3, 0xea, 0x60, 0x57, 0x18, 0x9e, 0x64, 0x43],
    };
    let info_set = unsafe {
        SetupDiGetClassDevsW(
            &interface_guid,
            std::ptr::null(),
            std::ptr::null_mut(),
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        )
    };
    if info_set == INVALID_HANDLE_VALUE as _ {
        return Err("SetupDiGetClassDevsW failed".into());
    }

    let mut devices = Vec::new();
    for index in 0..32u32 {
        let mut interface_data = SP_DEVICE_INTERFACE_DATA {
            cbSize: std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
            ..Default::default()
        };
        if unsafe {
            SetupDiEnumDeviceInterfaces(
                info_set,
                std::ptr::null(),
                &interface_guid,
                index,
                &mut interface_data,
            )
        } == 0
        {
            break;
        }

        let mut required = 0u32;
        unsafe {
            SetupDiGetDeviceInterfaceDetailW(
                info_set,
                &interface_data,
                std::ptr::null_mut(),
                0,
                &mut required,
                std::ptr::null_mut(),
            );
        }
        if required < std::mem::size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32 {
            continue;
        }
        let mut detail = vec![0u8; required as usize];
        let detail_ptr = detail.as_mut_ptr() as *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W;
        unsafe {
            (*detail_ptr).cbSize = if cfg!(target_arch = "x86") { 6 } else { 8 };
        }
        let mut dev_info = SP_DEVINFO_DATA {
            cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32,
            ..Default::default()
        };
        if unsafe {
            SetupDiGetDeviceInterfaceDetailW(
                info_set,
                &interface_data,
                detail_ptr,
                required,
                &mut required,
                &mut dev_info,
            )
        } == 0
        {
            continue;
        }
        let path = unsafe {
            let base = (*detail_ptr).DevicePath.as_ptr();
            let chars = std::slice::from_raw_parts(base, (required as usize / 2).saturating_sub(4));
            String::from_utf16_lossy(chars)
                .trim_end_matches('\0')
                .to_owned()
        };
        if path.is_empty() {
            continue;
        }
        let friendly_name = registry_string(info_set, &dev_info, SPDRP_FRIENDLYNAME)
            .or_else(|| registry_string(info_set, &dev_info, SPDRP_DEVICEDESC))
            .unwrap_or_else(|| "CandleLight Device".into());
        devices.push(Device {
            path,
            friendly_name,
            interface_number: index as u8,
            ..Default::default()
        });
    }
    unsafe { SetupDiDestroyDeviceInfoList(info_set) };
    Ok(devices)
}

fn registry_string(
    info_set: windows_sys::Win32::Devices::DeviceAndDriverInstallation::HDEVINFO,
    device_info: &SP_DEVINFO_DATA,
    property: u32,
) -> Option<String> {
    let mut data = [0u8; 512];
    let mut data_type = 0u32;
    let mut size = 0u32;
    if unsafe {
        SetupDiGetDeviceRegistryPropertyW(
            info_set,
            device_info,
            property,
            &mut data_type,
            data.as_mut_ptr(),
            data.len() as u32,
            &mut size,
        )
    } == 0
    {
        return None;
    }
    let wide =
        unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u16, size as usize / 2) };
    Some(
        String::from_utf16_lossy(wide)
            .trim_end_matches('\0')
            .to_owned(),
    )
}

const REQUEST_HOST_FORMAT: u8 = 0;
const REQUEST_BITTIMING: u8 = 1;
const REQUEST_MODE: u8 = 2;
const REQUEST_DEVICE_CONFIG: u8 = 5;
const REQUEST_TIMESTAMP: u8 = 6;
const REQUEST_DATA_BITTIMING: u8 = 10;
const REQUEST_SET_TERMINATION: u8 = 16;
const REQUEST_GET_TERMINATION: u8 = 15;
const REQUEST_INTERFACE_ENDPOINT: u8 = 17;
const CONTROL_IN: u8 = 0xC1;
const CONTROL_OUT: u8 = 0x41;

#[derive(Clone, Default)]
pub struct Device {
    pub path: String,
    pub friendly_name: String,
    pub interface_number: u8,
    pub opened: bool,
    pub winusb_handle: usize,
    pub device_handle: usize,
    pub bulk_in_pipe: u8,
    pub bulk_out_pipe: u8,
    pub timestamp_us: u32,
    pub started_channels: u32,
}

pub struct DeviceBackend {
    pub device: DeviceRef,
}

impl DeviceBackend {
    pub fn open(device: DeviceRef) -> Result<Self, String> {
        let mut value = device.lock().map_err(|_| "device lock poisoned")?;
        if value.path.is_empty() {
            return Err("device path is empty".into());
        }
        let wide: Vec<u16> = value.path.encode_utf16().chain(Some(0)).collect();
        let handle = unsafe {
            CreateFileW(
                wide.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                null_mut(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OVERLAPPED,
                null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err("CreateFileW failed".into());
        }
        let mut usb: WINUSB_INTERFACE_HANDLE = null_mut();
        if unsafe { WinUsb_Initialize(handle, &mut usb) } == 0 {
            unsafe { CloseHandle(handle) };
            return Err("WinUsb_Initialize failed".into());
        }
        let mut descriptor = Default::default();
        if unsafe { WinUsb_QueryInterfaceSettings(usb, 0, &mut descriptor) } == 0 {
            unsafe {
                WinUsb_Free(usb);
                CloseHandle(handle);
            }
            return Err("WinUsb_QueryInterfaceSettings failed".into());
        }
        for index in 0..descriptor.bNumEndpoints {
            let mut pipe = WINUSB_PIPE_INFORMATION::default();
            if unsafe { WinUsb_QueryPipe(usb, 0, index, &mut pipe) } != 0 && pipe.PipeType == 3 {
                if (pipe.PipeId as u32) & USB_ENDPOINT_DIRECTION_MASK != 0 {
                    value.bulk_in_pipe = pipe.PipeId;
                } else {
                    value.bulk_out_pipe = pipe.PipeId;
                }
            }
        }
        if value.bulk_in_pipe == 0 || value.bulk_out_pipe == 0 {
            unsafe {
                WinUsb_Free(usb);
                CloseHandle(handle);
            }
            return Err("CAN bulk endpoints not found".into());
        }
        let raw_io: u8 = 1;
        let _ = unsafe {
            WinUsb_SetPipePolicy(
                usb,
                value.bulk_in_pipe,
                0x01,
                1,
                (&raw_io as *const u8).cast::<c_void>(),
            )
        };
        value.device_handle = handle as usize;
        value.winusb_handle = usb as usize;
        value.interface_number = descriptor.bInterfaceNumber;
        value.opened = true;
        drop(value);
        let backend = Self {
            device: device.clone(),
        };
        backend.control_out(REQUEST_HOST_FORMAT, 1, 0, &0x0000_beefu32.to_le_bytes())?;
        backend.refresh_config()?;
        Ok(backend)
    }

    pub fn close(&self) -> Result<(), String> {
        let mut value = self.device.lock().map_err(|_| "device lock poisoned")?;
        if value.winusb_handle != 0 {
            unsafe { WinUsb_Free(value.winusb_handle as WINUSB_INTERFACE_HANDLE) };
        }
        if value.device_handle != 0 {
            unsafe { CloseHandle(value.device_handle as HANDLE) };
        }
        value.winusb_handle = 0;
        value.device_handle = 0;
        value.opened = false;
        Ok(())
    }

    pub fn control_out(
        &self,
        request: u8,
        value: u16,
        index: u16,
        data: &[u8],
    ) -> Result<(), String> {
        self.control(request, CONTROL_OUT, value, index, data.to_vec())
            .map(|_| ())
    }

    pub fn control_in(
        &self,
        request: u8,
        value: u16,
        index: u16,
        len: usize,
    ) -> Result<Vec<u8>, String> {
        self.control(request, CONTROL_IN, value, index, vec![0; len])
    }

    fn control(
        &self,
        request: u8,
        request_type: u8,
        value: u16,
        index: u16,
        mut data: Vec<u8>,
    ) -> Result<Vec<u8>, String> {
        let value_guard = self.device.lock().map_err(|_| "device lock poisoned")?;
        if value_guard.winusb_handle == 0 {
            return Err("device is not open".into());
        }
        let packet = WINUSB_SETUP_PACKET {
            RequestType: request_type,
            Request: request,
            Value: value,
            Index: index,
            Length: data.len() as u16,
        };
        let mut transferred = 0;
        if unsafe {
            WinUsb_ControlTransfer(
                value_guard.winusb_handle as WINUSB_INTERFACE_HANDLE,
                packet,
                data.as_mut_ptr(),
                data.len() as u32,
                &mut transferred,
                null_mut(),
            )
        } == 0
        {
            return Err(format!("WinUsb_ControlTransfer request {request} failed"));
        }
        Ok(data)
    }

    fn refresh_config(&self) -> Result<(), String> {
        let bytes = self.control_in(REQUEST_TIMESTAMP, 1, self.interface_number() as u16, 4)?;
        let timestamp = u32::from_le_bytes(bytes[..4].try_into().unwrap());
        self.device
            .lock()
            .map_err(|_| "device lock poisoned")?
            .timestamp_us = timestamp;
        Ok(())
    }

    fn interface_number(&self) -> u8 {
        self.device.lock().map(|d| d.interface_number).unwrap_or(0)
    }

    pub fn set_timing(&self, channel: u8, timing: &CandleBittiming) -> Result<(), String> {
        self.control_out(
            REQUEST_BITTIMING,
            channel as u16,
            self.interface_number() as u16,
            unsafe { std::slice::from_raw_parts((timing as *const CandleBittiming).cast(), 20) },
        )
        .map(|_| ())
    }

    pub fn set_data_timing(&self, channel: u8, timing: &CandleBittiming) -> Result<(), String> {
        self.control_out(
            REQUEST_DATA_BITTIMING,
            channel as u16,
            self.interface_number() as u16,
            unsafe { std::slice::from_raw_parts((timing as *const CandleBittiming).cast(), 20) },
        )
        .map(|_| ())
    }

    pub fn start(&self, channel: u8, flags: u32) -> Result<(), String> {
        let mut data = [0u8; 8];
        data[..4].copy_from_slice(&flags.to_le_bytes());
        self.control_out(
            REQUEST_MODE,
            channel as u16,
            self.interface_number() as u16,
            &data,
        )
        .map(|_| ())
    }

    pub fn stop(&self, channel: u8) -> Result<(), String> {
        self.start(channel, 0)
    }

    pub fn start_channel(&self, channel: u8, flags: u32) -> Result<(), String> {
        self.start(channel, flags)?;
        self.device
            .lock()
            .map_err(|_| "device lock poisoned")?
            .started_channels |= 1u32 << channel;
        Ok(())
    }

    pub fn stop_channel(&self, channel: u8) -> Result<(), String> {
        self.stop(channel)?;
        self.device
            .lock()
            .map_err(|_| "device lock poisoned")?
            .started_channels &= !(1u32 << channel);
        Ok(())
    }

    pub fn set_endpoints(&self, channel: u8) -> Result<(), String> {
        self.control_out(
            REQUEST_INTERFACE_ENDPOINT,
            channel as u16,
            self.interface_number() as u16,
            &[],
        )
        .map(|_| ())
    }

    pub fn termination(&self, channel: u8, enabled: Option<bool>) -> Result<bool, String> {
        let request = if enabled.is_some() {
            REQUEST_SET_TERMINATION
        } else {
            REQUEST_GET_TERMINATION
        };
        let data = vec![u8::from(enabled.unwrap_or(false))];
        if let Some(value) = enabled {
            self.control_out(
                request,
                channel as u16,
                self.interface_number() as u16,
                &data,
            )?;
            Ok(value)
        } else {
            Ok(
                self.control_in(request, channel as u16, self.interface_number() as u16, 1)?[0]
                    != 0,
            )
        }
    }

    pub fn timestamp(&self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(
            self.control_in(REQUEST_TIMESTAMP, 1, self.interface_number() as u16, 4)?[..4]
                .try_into()
                .unwrap(),
        ))
    }

    pub fn send(&self, channel: u8, frame: &CandleFrame) -> Result<(), String> {
        let value = self.device.lock().map_err(|_| "device lock poisoned")?;
        if !value.opened {
            return Err("device is not open".into());
        }
        let mut packet = *frame;
        packet.channel = channel;
        let size = 12
            + if packet.can_dlc <= 8 {
                8
            } else {
                super::frame::data_length(packet.can_dlc)
            };
        let mut written = 0;
        let ok = unsafe {
            WinUsb_WritePipe(
                value.winusb_handle as WINUSB_INTERFACE_HANDLE,
                value.bulk_out_pipe,
                (&packet as *const CandleFrame).cast(),
                size as u32,
                &mut written,
                null_mut(),
            )
        };
        if ok == 0 || written != size as u32 {
            return Err("WinUsb_WritePipe failed".into());
        }
        Ok(())
    }

    pub fn receive(&self, timeout_ms: u32) -> Result<Option<CandleFrame>, String> {
        let value = self.device.lock().map_err(|_| "device lock poisoned")?;
        let mut packet = CandleFrame::default();
        let mut read = 0;
        let mut overlapped = OVERLAPPED::default();
        let ok = unsafe {
            WinUsb_ReadPipe(
                value.winusb_handle as WINUSB_INTERFACE_HANDLE,
                value.bulk_in_pipe,
                (&mut packet as *mut CandleFrame).cast(),
                std::mem::size_of::<CandleFrame>() as u32,
                &mut read,
                &mut overlapped,
            )
        };
        let _ = timeout_ms;
        if ok == 0 || read == 0 {
            return Ok(None);
        }
        Ok(Some(packet))
    }
}
