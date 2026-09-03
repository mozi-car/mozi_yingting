use napi_derive::napi;
use std::ffi::c_void;

// ABI structs mirror PCAN-ISO-TP_2016.h. They are never exposed to JS.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct RawMsgInfo { pub size: u32, pub flags: u32, pub extra: *mut c_void }
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct RawCanInfo { pub can_id: u32, pub can_msgtype: u32, pub dlc: u8, pub _padding: [u8; 3] }
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct RawNetaddr { pub msgtype: u32, pub format: u32, pub target_type: u32, pub source_addr: u16, pub target_addr: u16, pub extension_addr: u8, pub _padding: [u8; 3] }
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct RawOptionList { pub buffer: *mut c_void, pub count: u32, pub _padding: [u8; 4] }
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct RawMsgData { pub flags: u32, pub length: u32, pub data: *mut u8, pub netstatus: u32, pub options: *mut RawOptionList }
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct RawCanData { pub flags: u32, pub length: u32, pub data: *mut u8, pub netstatus: u32, pub options: *mut RawOptionList, pub data_max: [u8; 8] }
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RawCanFdData { pub flags: u32, pub length: u32, pub data: *mut u8, pub netstatus: u32, pub options: *mut RawOptionList, pub data_max: [u8; 64] }
impl Default for RawCanFdData { fn default() -> Self { unsafe { std::mem::zeroed() } } }
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct RawIsoTpData { pub flags: u32, pub length: u32, pub data: *mut u8, pub netstatus: u32, pub options: *mut RawOptionList, pub netaddrinfo: RawNetaddr, pub reserved: *mut c_void }
#[repr(C)]
#[derive(Clone, Copy)]
pub union RawMsgDataUnion { pub any: *mut RawMsgData, pub can: *mut RawCanData, pub canfd: *mut RawCanFdData, pub isotp: *mut RawIsoTpData }
impl Default for RawMsgDataUnion { fn default() -> Self { Self { any: std::ptr::null_mut() } } }
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct RawCantpMsg { pub msg_type: u32, pub reserved: RawMsgInfo, pub can_info: RawCanInfo, pub msgdata: RawMsgDataUnion }
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct RawCantpProgress { pub state: u32, pub percentage: u8, pub _padding: [u8; 3], pub buffer: *mut RawCantpMsg }
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct RawMapping { pub uid: usize, pub can_id: u32, pub can_id_flow_ctrl: u32, pub can_msgtype: u32, pub can_tx_dlc: u8, pub _padding: [u8; 3], pub netaddrinfo: RawNetaddr }

#[napi(object)]
#[derive(Clone, Copy, Default)]
pub struct CanInfo {
    #[napi(js_name = "can_id")] pub can_id: u32,
    #[napi(js_name = "can_msgtype")] pub can_msgtype: u32,
    pub dlc: u8
}
#[napi(object)]
#[derive(Clone, Copy, Default)]
pub struct CantpNetaddrinfo {
    pub msgtype: u32,
    pub format: u32,
    #[napi(js_name = "target_type")] pub target_type: u32,
    #[napi(js_name = "source_addr")] pub source_addr: u16,
    #[napi(js_name = "target_addr")] pub target_addr: u16,
    #[napi(js_name = "extension_addr")] pub extension_addr: u8
}
impl CantpNetaddrinfo {
    pub fn raw(&self) -> RawNetaddr { RawNetaddr { msgtype: self.msgtype, format: self.format, target_type: self.target_type, source_addr: self.source_addr, target_addr: self.target_addr, extension_addr: self.extension_addr, _padding: [0; 3] } }
    pub fn from_raw(v: RawNetaddr) -> Self { Self { msgtype: v.msgtype, format: v.format, target_type: v.target_type, source_addr: v.source_addr, target_addr: v.target_addr, extension_addr: v.extension_addr } }
}

#[napi(object)]
#[derive(Clone)]
pub struct CantpMsgData { pub flags: u32, pub length: u32, pub data: Vec<u8>, pub netstatus: u32 }
#[napi(object)]
#[derive(Clone)]
pub struct CantpMsgDataIsoTp { pub flags: u32, pub length: u32, pub data: Vec<u8>, pub netstatus: u32, pub netaddrinfo: CantpNetaddrinfo }
#[napi(object)]
#[derive(Clone)]
pub struct CantpMsgDataCan { pub flags: u32, pub length: u32, pub data: Vec<u8>, pub netstatus: u32 }

#[napi(js_name = "cantp_msg")]
pub struct CantpMsg {
    #[napi(js_name = "type")]
    pub msg_type: u32,
    pub flags: u32,
    pub length: u32,
    pub data: Vec<u8>,
    pub netstatus: u32,
    can_info_value: CanInfo,
    pub(crate) raw: Box<RawCantpMsg>,
}
#[napi]
impl CantpMsg {
    #[napi(constructor)]
    pub fn new() -> Self { Self { msg_type: 0, flags: 0, length: 0, data: Vec::new(), netstatus: 0, can_info_value: CanInfo::default(), raw: Box::new(RawCantpMsg::default()) } }
    #[napi(getter, js_name = "can_info")]
    pub fn can_info(&self) -> CanInfo { self.can_info_value }
    #[napi(setter, js_name = "can_info")]
    pub fn set_can_info(&mut self, value: CanInfo) { self.can_info_value = value; }
}
impl CantpMsg {
    pub fn raw_ptr(&mut self) -> *mut RawCantpMsg { self.raw.as_mut() as *mut _ }
    pub fn sync_to_raw(&mut self) { self.raw.msg_type = self.msg_type; self.raw.can_info = RawCanInfo { can_id: self.can_info_value.can_id, can_msgtype: self.can_info_value.can_msgtype, dlc: self.can_info_value.dlc, _padding: [0; 3] }; }
    pub fn sync_from_raw(&mut self) { self.msg_type = self.raw.msg_type; self.can_info_value = CanInfo { can_id: self.raw.can_info.can_id, can_msgtype: self.raw.can_info.can_msgtype, dlc: self.raw.can_info.dlc }; }
}

#[napi(js_name = "cantp_mapping")]
pub struct CantpMapping {
    pub uid: i64,
    #[napi(js_name = "can_id")] pub can_id: u32,
    #[napi(js_name = "can_id_flow_ctrl")] pub can_id_flow_ctrl: u32,
    #[napi(js_name = "can_msgtype")] pub can_msgtype: u32,
    #[napi(js_name = "can_tx_dlc")] pub can_tx_dlc: u8,
    netaddrinfo_value: CantpNetaddrinfo
}
#[napi]
impl CantpMapping {
    #[napi(constructor)]
    pub fn new() -> Self { Self { uid: 0, can_id: 0, can_id_flow_ctrl: 0, can_msgtype: 0, can_tx_dlc: 0, netaddrinfo_value: CantpNetaddrinfo::default() } }
    #[napi(getter)]
    pub fn netaddrinfo(&self) -> CantpNetaddrinfo { self.netaddrinfo_value }
    #[napi(setter)]
    pub fn set_netaddrinfo(&mut self, value: CantpNetaddrinfo) { self.netaddrinfo_value = value; }
}
impl CantpMapping { pub fn raw(&self) -> RawMapping { RawMapping { uid: self.uid.max(0) as usize, can_id: self.can_id, can_id_flow_ctrl: self.can_id_flow_ctrl, can_msgtype: self.can_msgtype, can_tx_dlc: self.can_tx_dlc, _padding: [0; 3], netaddrinfo: self.netaddrinfo_value.raw() } } pub fn update_from_raw(&mut self, raw: &RawMapping) { self.uid = raw.uid as i64; } }

#[napi(js_name = "TimeStamp")]
pub struct TimeStamp { pub value: i64 }
#[napi]
impl TimeStamp { #[napi(constructor)] pub fn new() -> Self { Self { value: 0 } } }

#[napi(js_name = "cantp_msgprogress")]
pub struct CantpMsgprogress { pub state: u32, pub percentage: u8, pub buffer: i64 }
#[napi]
impl CantpMsgprogress { #[napi(constructor)] pub fn new() -> Self { Self { state: 0, percentage: 0, buffer: 0 } } }
impl CantpMsgprogress { pub fn raw_ptr(&mut self) -> RawCantpProgress { RawCantpProgress { state: self.state, percentage: self.percentage, _padding: [0; 3], buffer: std::ptr::null_mut() } } pub fn sync_from_raw(&mut self, raw: &RawCantpProgress) { self.state = raw.state; self.percentage = raw.percentage; self.buffer = raw.buffer as i64; } }

// Constants used by the TypeScript PCAN transport layer.
#[napi]
pub const PCANTP_STATUS_OK: i32 = 0;
#[napi]
pub const PCANTP_STATUS_NO_MESSAGE: i32 = 7;
#[napi]
pub const PCANTP_MSGTYPE_CAN: u32 = 1;
#[napi]
pub const PCANTP_MSGTYPE_CANFD: u32 = 2;
#[napi]
pub const PCANTP_MSGTYPE_ISOTP: u32 = 4;
#[napi]
pub const PCANTP_MSGTYPE_CANINFO: u32 = 8;
#[napi]
pub const PCANTP_MSGTYPE_FRAME: u32 = 3;
#[napi]
pub const PCANTP_MSGTYPE_ANY: u32 = 0xffff_ffff;
#[napi]
pub const PCANTP_CAN_MSGTYPE_STANDARD: u32 = 0;
#[napi]
pub const PCANTP_CAN_MSGTYPE_RTR: u32 = 1;
#[napi]
pub const PCANTP_CAN_MSGTYPE_EXTENDED: u32 = 2;
#[napi]
pub const PCANTP_CAN_MSGTYPE_FD: u32 = 4;
#[napi]
pub const PCANTP_CAN_MSGTYPE_BRS: u32 = 8;
#[napi]
pub const PCANTP_PARAMETER_API_VERSION: u32 = 0x101;
#[napi]
pub const PCANTP_PARAMETER_CHANNEL_CONDITION: u32 = 0x102;
#[napi]
pub const PCANTP_PARAMETER_RECEIVE_EVENT: u32 = 0x104;
#[napi]
pub const PCANTP_PARAMETER_CAN_TX_DL: u32 = 0x106;
#[napi]
pub const PCANTP_PARAMETER_CAN_DATA_PADDING: u32 = 0x107;
#[napi]
pub const PCANTP_PARAMETER_CAN_PADDING_VALUE: u32 = 0x108;
#[napi]
pub const PCANTP_PARAMETER_RESET_HARD: u32 = 0x11f;
#[napi]
pub const PCANTP_PARAMETER_ALLOW_MSGTYPE_CANINFO: u32 = 0x124;
#[napi]
pub const PCANTP_CHANNEL_AVAILABLE: u8 = 1;
#[napi]
pub const PCANTP_CHANNEL_OCCUPIED: u8 = 2;
#[napi]
pub const PCANTP_MSGDIRECTION_RX: u32 = 0;
#[napi]
pub const PCANTP_MSGDIRECTION_TX: u32 = 1;
#[napi]
pub const PCANTP_MSGFLAG_LOOPBACK: u32 = 1;
#[napi]
pub const PCANTP_MSGPROGRESS_STATE_COMPLETED: u32 = 2;
#[napi]
pub const PCANTP_ISOTP_MSGTYPE_DIAGNOSTIC: u32 = 1;
#[napi]
pub const PCANTP_ISOTP_MSGTYPE_REMOTE_DIAGNOSTIC: u32 = 2;
#[napi]
pub const PCANTP_ISOTP_MSGTYPE_FLAG_INDICATION_RX: u32 = 0x10;
#[napi]
pub const PCANTP_ISOTP_MSGTYPE_FLAG_INDICATION_TX: u32 = 0x20;
#[napi]
pub const PCANTP_ISOTP_FORMAT_NORMAL: u32 = 1;
#[napi]
pub const PCANTP_ISOTP_FORMAT_FIXED_NORMAL: u32 = 2;
#[napi]
pub const PCANTP_ISOTP_FORMAT_EXTENDED: u32 = 3;
#[napi]
pub const PCANTP_ISOTP_FORMAT_MIXED: u32 = 4;
#[napi]
pub const PCANTP_ISOTP_FORMAT_ENHANCED: u32 = 5;
#[napi]
pub const PCANTP_ISOTP_ADDRESSING_PHYSICAL: u32 = 1;
#[napi]
pub const PCANTP_ISOTP_ADDRESSING_FUNCTIONAL: u32 = 2;
#[napi]
pub const PCAN_NONEBUS: u32 = 0;
#[napi]
pub const PCAN_DNGBUS1: u32 = 0x31;
#[napi]
pub const PCAN_PCCBUS1: u32 = 0x61;
#[napi]
pub const PCAN_PCCBUS2: u32 = 0x62;
#[napi]
pub const PCAN_BUSOFF_AUTORESET: u32 = 7;
#[napi] pub const PCAN_ISABUS1: u32 = 0x21;
#[napi] pub const PCAN_ISABUS2: u32 = 0x22;
#[napi] pub const PCAN_ISABUS3: u32 = 0x23;
#[napi] pub const PCAN_ISABUS4: u32 = 0x24;
#[napi] pub const PCAN_ISABUS5: u32 = 0x25;
#[napi] pub const PCAN_ISABUS6: u32 = 0x26;
#[napi] pub const PCAN_ISABUS7: u32 = 0x27;
#[napi] pub const PCAN_ISABUS8: u32 = 0x28;
#[napi] pub const PCAN_PCIBUS1: u32 = 0x41;
#[napi] pub const PCAN_PCIBUS2: u32 = 0x42;
#[napi] pub const PCAN_PCIBUS3: u32 = 0x43;
#[napi] pub const PCAN_PCIBUS4: u32 = 0x44;
#[napi] pub const PCAN_PCIBUS5: u32 = 0x45;
#[napi] pub const PCAN_PCIBUS6: u32 = 0x46;
#[napi] pub const PCAN_PCIBUS7: u32 = 0x47;
#[napi] pub const PCAN_PCIBUS8: u32 = 0x48;
#[napi] pub const PCAN_PCIBUS9: u32 = 0x409;
#[napi] pub const PCAN_PCIBUS10: u32 = 0x40a;
#[napi] pub const PCAN_PCIBUS11: u32 = 0x40b;
#[napi] pub const PCAN_PCIBUS12: u32 = 0x40c;
#[napi] pub const PCAN_PCIBUS13: u32 = 0x40d;
#[napi] pub const PCAN_PCIBUS14: u32 = 0x40e;
#[napi] pub const PCAN_PCIBUS15: u32 = 0x40f;
#[napi] pub const PCAN_PCIBUS16: u32 = 0x410;
#[napi] pub const PCAN_USBBUS1: u32 = 0x51;
#[napi] pub const PCAN_USBBUS2: u32 = 0x52;
#[napi] pub const PCAN_USBBUS3: u32 = 0x53;
#[napi] pub const PCAN_USBBUS4: u32 = 0x54;
#[napi] pub const PCAN_USBBUS5: u32 = 0x55;
#[napi] pub const PCAN_USBBUS6: u32 = 0x56;
#[napi] pub const PCAN_USBBUS7: u32 = 0x57;
#[napi] pub const PCAN_USBBUS8: u32 = 0x58;
#[napi] pub const PCAN_USBBUS9: u32 = 0x509;
#[napi] pub const PCAN_USBBUS10: u32 = 0x50a;
#[napi] pub const PCAN_USBBUS11: u32 = 0x50b;
#[napi] pub const PCAN_USBBUS12: u32 = 0x50c;
#[napi] pub const PCAN_USBBUS13: u32 = 0x50d;
#[napi] pub const PCAN_USBBUS14: u32 = 0x50e;
#[napi] pub const PCAN_USBBUS15: u32 = 0x50f;
#[napi] pub const PCAN_USBBUS16: u32 = 0x510;
#[napi] pub const PCAN_LANBUS1: u32 = 0x801;
#[napi] pub const PCAN_LANBUS2: u32 = 0x802;
#[napi] pub const PCAN_LANBUS3: u32 = 0x803;
#[napi] pub const PCAN_LANBUS4: u32 = 0x804;
#[napi] pub const PCAN_LANBUS5: u32 = 0x805;
#[napi] pub const PCAN_LANBUS6: u32 = 0x806;
#[napi] pub const PCAN_LANBUS7: u32 = 0x807;
#[napi] pub const PCAN_LANBUS8: u32 = 0x808;
#[napi] pub const PCAN_LANBUS9: u32 = 0x809;
#[napi] pub const PCAN_LANBUS10: u32 = 0x80a;
#[napi] pub const PCAN_LANBUS11: u32 = 0x80b;
#[napi] pub const PCAN_LANBUS12: u32 = 0x80c;
#[napi] pub const PCAN_LANBUS13: u32 = 0x80d;
#[napi] pub const PCAN_LANBUS14: u32 = 0x80e;
#[napi] pub const PCAN_LANBUS15: u32 = 0x80f;
#[napi] pub const PCAN_LANBUS16: u32 = 0x810;
#[napi]
pub const PCANTP_BAUDRATE_1M: u32 = 0x14;
#[napi]
pub const PCANTP_BAUDRATE_800K: u32 = 0x16;
#[napi]
pub const PCANTP_BAUDRATE_500K: u32 = 0x1c;
#[napi]
pub const PCANTP_BAUDRATE_250K: u32 = 0x11c;
#[napi]
pub const PCANTP_BAUDRATE_125K: u32 = 0x31c;
#[napi]
pub const PCANTP_BAUDRATE_100K: u32 = 0x432f;
