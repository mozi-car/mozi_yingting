use crate::types::{CanInfo, CantpMsg, CantpMsgData, CantpMsgDataCan, CantpMsgDataIsoTp, RawCantpMsg};
use std::slice;

pub fn init(msg: &mut CantpMsg, can_id: u32, msg_type: u32, data: &[u8]) {
    msg.msg_type = if data.len() > 8 { 2 } else { 1 };
    msg.set_can_info(CanInfo { can_id, can_msgtype: msg_type, dlc: data.len().min(64) as u8 });
    msg.length = data.len().min(64) as u32;
    msg.data = data[..data.len().min(64)].to_vec();
    msg.sync_to_raw();
}
unsafe fn bytes(ptr: *const u8, length: u32, max: usize) -> Vec<u8> {
    if ptr.is_null() || length == 0 { return Vec::new(); }
    slice::from_raw_parts(ptr, (length as usize).min(max)).to_vec()
}
pub unsafe fn from_raw(msg: &RawCantpMsg) -> Option<CantpMsgData> {
    let (ptr, max): (*const crate::types::RawCanData, usize) = match msg.msg_type {
        2 => (msg.msgdata.canfd as *const crate::types::RawCanFdData as *const crate::types::RawCanData, 64),
        _ => (msg.msgdata.can, 8),
    };
    if ptr.is_null() { return None; }
    // The common prefix of CAN and CAN-FD data structures is identical.
    let d = &*(ptr as *const crate::types::RawCanData);
    Some(CantpMsgData { flags: d.flags, length: d.length, data: bytes(d.data, d.length, max), netstatus: d.netstatus })
}
pub unsafe fn iso_from_raw(msg: &RawCantpMsg) -> Option<CantpMsgDataIsoTp> {
    let p = msg.msgdata.isotp;
    if p.is_null() { return None; }
    let d = &*p;
    Some(CantpMsgDataIsoTp { flags: d.flags, length: d.length, data: bytes(d.data, d.length, usize::MAX), netstatus: d.netstatus, netaddrinfo: crate::types::CantpNetaddrinfo::from_raw(d.netaddrinfo) })
}
pub unsafe fn can_from_raw(msg: &RawCantpMsg) -> Option<CantpMsgDataCan> {
    let p: *const crate::types::RawCanData = if msg.msg_type == 2 {
        msg.msgdata.canfd as *const crate::types::RawCanFdData as *const crate::types::RawCanData
    } else { msg.msgdata.can };
    if p.is_null() { return None; }
    let common = &*(p as *const crate::types::RawCanData);
    Some(CantpMsgDataCan { flags: common.flags, length: common.length, data: bytes(common.data, common.length, if msg.msg_type == 2 { 64 } else { 8 }), netstatus: common.netstatus })
}
