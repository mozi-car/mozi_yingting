#[repr(C, packed)]
#[derive(Clone, Copy, Default)]
pub struct CandleBittiming {
    pub prop_seg: u32,
    pub phase_seg1: u32,
    pub phase_seg2: u32,
    pub sjw: u32,
    pub brp: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct CandleFrame {
    pub echo_id: u32,
    pub can_id: u32,
    pub can_dlc: u8,
    pub channel: u8,
    pub flags: u8,
    pub reserved: u8,
    pub data: [u8; 64],
    pub timestamp_us: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy, Default)]
pub struct CandleCapability {
    pub feature: u32,
    pub fclk_can: u32,
    pub tseg1_min: u32,
    pub tseg1_max: u32,
    pub tseg2_min: u32,
    pub tseg2_max: u32,
    pub sjw_max: u32,
    pub brp_min: u32,
    pub brp_max: u32,
    pub brp_inc: u32,
}
#[repr(C, packed)]
#[derive(Clone, Copy, Default)]
pub struct CandleDeviceConfig {
    pub reserved1: u8,
    pub reserved2: u8,
    pub reserved3: u8,
    pub icount: u8,
    pub sw_version: u32,
    pub hw_version: u32,
}
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct CandleDevice {
    pub interface_number: u8,
    pub opened: bool,
    pub dconf: CandleDeviceConfig,
    pub bt_const: CandleCapability,
    pub data_bt_const: CandleCapability,
    pub path: [u16; 256],
    pub friendly_name: [u8; 256],
}
impl Default for CandleDevice {
    fn default() -> Self {
        Self {
            interface_number: 0,
            opened: false,
            dconf: Default::default(),
            bt_const: Default::default(),
            data_bt_const: Default::default(),
            path: [0; 256],
            friendly_name: [0; 256],
        }
    }
}

impl Default for CandleFrame {
    fn default() -> Self {
        Self {
            echo_id: 0,
            can_id: 0,
            can_dlc: 0,
            channel: 0,
            flags: 0,
            reserved: 0,
            data: [0; 64],
            timestamp_us: 0,
        }
    }
}

pub fn data_length(dlc: u8) -> usize {
    match dlc {
        0..=8 => dlc as usize,
        9 => 12,
        10 => 16,
        11 => 20,
        12 => 24,
        13 => 32,
        14 => 48,
        15 => 64,
        _ => 0,
    }
}
