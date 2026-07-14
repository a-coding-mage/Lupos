//! linux-parity: partial
//! linux-source: vendor/linux/drivers/gpu/drm/drm_mipi_dsi.c
//! linux-source: vendor/linux/include/drm/drm_mipi_dsi.h
//! Linux MIPI DSI helper ABI selected as built-in DRM core by vendor i915.

extern crate alloc;

use alloc::vec::Vec;
use core::ffi::c_void;
use core::ptr;

use crate::include::uapi::errno::{EINVAL, ENOMEM, ENOSYS};
use crate::kernel::module::{export_symbol, find_symbol};

const MIPI_DSI_MSG_USE_LPM: u16 = 1 << 1;
const MIPI_DSI_MODE_LPM: usize = 1 << 11;

const MIPI_DSI_V_SYNC_START: u8 = 0x01;
const MIPI_DSI_V_SYNC_END: u8 = 0x11;
const MIPI_DSI_H_SYNC_START: u8 = 0x21;
const MIPI_DSI_H_SYNC_END: u8 = 0x31;
const MIPI_DSI_COMPRESSION_MODE: u8 = 0x07;
const MIPI_DSI_END_OF_TRANSMISSION: u8 = 0x08;
const MIPI_DSI_COLOR_MODE_OFF: u8 = 0x02;
const MIPI_DSI_COLOR_MODE_ON: u8 = 0x12;
const MIPI_DSI_SHUTDOWN_PERIPHERAL: u8 = 0x22;
const MIPI_DSI_TURN_ON_PERIPHERAL: u8 = 0x32;
const MIPI_DSI_GENERIC_SHORT_WRITE_0_PARAM: u8 = 0x03;
const MIPI_DSI_GENERIC_SHORT_WRITE_1_PARAM: u8 = 0x13;
const MIPI_DSI_GENERIC_SHORT_WRITE_2_PARAM: u8 = 0x23;
const MIPI_DSI_GENERIC_READ_REQUEST_0_PARAM: u8 = 0x04;
const MIPI_DSI_GENERIC_READ_REQUEST_1_PARAM: u8 = 0x14;
const MIPI_DSI_GENERIC_READ_REQUEST_2_PARAM: u8 = 0x24;
const MIPI_DSI_DCS_SHORT_WRITE: u8 = 0x05;
const MIPI_DSI_DCS_SHORT_WRITE_PARAM: u8 = 0x15;
const MIPI_DSI_DCS_READ: u8 = 0x06;
const MIPI_DSI_EXECUTE_QUEUE: u8 = 0x16;
const MIPI_DSI_SET_MAXIMUM_RETURN_PACKET_SIZE: u8 = 0x37;
const MIPI_DSI_NULL_PACKET: u8 = 0x09;
const MIPI_DSI_BLANKING_PACKET: u8 = 0x19;
const MIPI_DSI_GENERIC_LONG_WRITE: u8 = 0x29;
const MIPI_DSI_DCS_LONG_WRITE: u8 = 0x39;
const MIPI_DSI_PICTURE_PARAMETER_SET: u8 = 0x0a;
const MIPI_DSI_COMPRESSED_PIXEL_STREAM: u8 = 0x0b;
const MIPI_DSI_LOOSELY_PACKED_PIXEL_STREAM_YCBCR20: u8 = 0x0c;
const MIPI_DSI_PACKED_PIXEL_STREAM_YCBCR24: u8 = 0x1c;
const MIPI_DSI_PACKED_PIXEL_STREAM_YCBCR16: u8 = 0x2c;
const MIPI_DSI_PACKED_PIXEL_STREAM_30: u8 = 0x0d;
const MIPI_DSI_PACKED_PIXEL_STREAM_36: u8 = 0x1d;
const MIPI_DSI_PACKED_PIXEL_STREAM_YCBCR12: u8 = 0x3d;
const MIPI_DSI_PACKED_PIXEL_STREAM_16: u8 = 0x0e;
const MIPI_DSI_PACKED_PIXEL_STREAM_18: u8 = 0x1e;
const MIPI_DSI_PIXEL_STREAM_3BYTE_18: u8 = 0x2e;
const MIPI_DSI_PACKED_PIXEL_STREAM_24: u8 = 0x3e;

const MIPI_DCS_NOP: u8 = 0x00;
const DRM_DSC_PICTURE_PARAMETER_SET_SIZE: usize = 128;

// x86_64 vendor defconfig layout, verified with the vendor build headers:
//   sizeof(struct device) == 760
//   offsetof(struct mipi_dsi_device, attached) == 768
//   offsetof(struct mipi_dsi_device, channel) == 792
//   offsetof(struct mipi_dsi_device, mode_flags) == 808
const MIPI_DSI_DEVICE_HOST_OFFSET: usize = 0;
const MIPI_DSI_DEVICE_ATTACHED_OFFSET: usize = 768;
const MIPI_DSI_DEVICE_CHANNEL_OFFSET: usize = 792;
const MIPI_DSI_DEVICE_MODE_FLAGS_OFFSET: usize = 808;

#[repr(C)]
pub struct MipiDsiMsg {
    pub channel: u8,
    pub type_: u8,
    pub flags: u16,
    pub tx_len: usize,
    pub tx_buf: *const c_void,
    pub rx_len: usize,
    pub rx_buf: *mut c_void,
}

#[repr(C)]
pub struct MipiDsiPacket {
    pub size: usize,
    pub header: [u8; 4],
    pub payload_length: usize,
    pub payload: *const u8,
}

#[repr(C)]
pub struct MipiDsiHostOps {
    pub attach: Option<unsafe extern "C" fn(*mut MipiDsiHost, *mut MipiDsiDevice) -> i32>,
    pub detach: Option<unsafe extern "C" fn(*mut MipiDsiHost, *mut MipiDsiDevice) -> i32>,
    pub transfer: Option<unsafe extern "C" fn(*mut MipiDsiHost, *const MipiDsiMsg) -> isize>,
}

#[repr(C)]
pub struct MipiDsiHost {
    pub dev: *mut c_void,
    pub ops: *const MipiDsiHostOps,
    pub list_next: *mut c_void,
    pub list_prev: *mut c_void,
}

#[repr(C)]
pub struct MipiDsiDevice {
    _opaque: [u8; 0],
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "mipi_dsi_packet_format_is_short",
        mipi_dsi_packet_format_is_short as usize,
        false,
    );
    export_symbol_once(
        "mipi_dsi_packet_format_is_long",
        mipi_dsi_packet_format_is_long as usize,
        false,
    );
    export_symbol_once(
        "mipi_dsi_create_packet",
        mipi_dsi_create_packet as usize,
        false,
    );
    export_symbol_once("mipi_dsi_attach", mipi_dsi_attach as usize, false);
    export_symbol_once(
        "mipi_dsi_set_maximum_return_packet_size",
        mipi_dsi_set_maximum_return_packet_size as usize,
        false,
    );
    export_symbol_once(
        "mipi_dsi_compression_mode",
        mipi_dsi_compression_mode as usize,
        false,
    );
    export_symbol_once(
        "mipi_dsi_picture_parameter_set",
        mipi_dsi_picture_parameter_set as usize,
        false,
    );
    export_symbol_once(
        "mipi_dsi_generic_write",
        mipi_dsi_generic_write as usize,
        false,
    );
    export_symbol_once(
        "mipi_dsi_dcs_write_buffer",
        mipi_dsi_dcs_write_buffer as usize,
        false,
    );
    export_symbol_once("mipi_dsi_dcs_write", mipi_dsi_dcs_write as usize, false);
    export_symbol_once("mipi_dsi_dcs_read", mipi_dsi_dcs_read as usize, false);
    export_symbol_once("mipi_dsi_dcs_nop", mipi_dsi_dcs_nop as usize, false);
}

#[inline]
unsafe fn read_dsi_host(dsi: *const MipiDsiDevice) -> *mut MipiDsiHost {
    if dsi.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        ptr::read(
            dsi.cast::<u8>()
                .add(MIPI_DSI_DEVICE_HOST_OFFSET)
                .cast::<*mut MipiDsiHost>(),
        )
    }
}

#[inline]
unsafe fn read_dsi_channel(dsi: *const MipiDsiDevice) -> u8 {
    if dsi.is_null() {
        return 0;
    }
    unsafe {
        ptr::read(
            dsi.cast::<u8>()
                .add(MIPI_DSI_DEVICE_CHANNEL_OFFSET)
                .cast::<u32>(),
        ) as u8
    }
}

#[inline]
unsafe fn read_dsi_mode_flags(dsi: *const MipiDsiDevice) -> usize {
    if dsi.is_null() {
        return 0;
    }
    unsafe {
        ptr::read(
            dsi.cast::<u8>()
                .add(MIPI_DSI_DEVICE_MODE_FLAGS_OFFSET)
                .cast::<usize>(),
        )
    }
}

#[inline]
unsafe fn write_dsi_attached(dsi: *mut MipiDsiDevice, attached: bool) {
    if !dsi.is_null() {
        unsafe {
            ptr::write(
                dsi.cast::<u8>().add(MIPI_DSI_DEVICE_ATTACHED_OFFSET),
                u8::from(attached),
            );
        }
    }
}

#[inline]
fn ret_zero_or_error(ret: isize) -> i32 {
    if ret < 0 { ret as i32 } else { 0 }
}

unsafe fn mipi_dsi_device_transfer(dsi: *mut MipiDsiDevice, msg: &mut MipiDsiMsg) -> isize {
    let host = unsafe { read_dsi_host(dsi) };
    if host.is_null() {
        return -(ENOSYS as isize);
    }

    let ops = unsafe { (*host).ops };
    if ops.is_null() {
        return -(ENOSYS as isize);
    }

    let Some(transfer) = (unsafe { (*ops).transfer }) else {
        return -(ENOSYS as isize);
    };

    if unsafe { read_dsi_mode_flags(dsi) } & MIPI_DSI_MODE_LPM != 0 {
        msg.flags |= MIPI_DSI_MSG_USE_LPM;
    }

    unsafe { transfer(host, msg as *const MipiDsiMsg) }
}

pub extern "C" fn mipi_dsi_packet_format_is_short(type_: u8) -> bool {
    matches!(
        type_,
        MIPI_DSI_V_SYNC_START
            | MIPI_DSI_V_SYNC_END
            | MIPI_DSI_H_SYNC_START
            | MIPI_DSI_H_SYNC_END
            | MIPI_DSI_COMPRESSION_MODE
            | MIPI_DSI_END_OF_TRANSMISSION
            | MIPI_DSI_COLOR_MODE_OFF
            | MIPI_DSI_COLOR_MODE_ON
            | MIPI_DSI_SHUTDOWN_PERIPHERAL
            | MIPI_DSI_TURN_ON_PERIPHERAL
            | MIPI_DSI_GENERIC_SHORT_WRITE_0_PARAM
            | MIPI_DSI_GENERIC_SHORT_WRITE_1_PARAM
            | MIPI_DSI_GENERIC_SHORT_WRITE_2_PARAM
            | MIPI_DSI_GENERIC_READ_REQUEST_0_PARAM
            | MIPI_DSI_GENERIC_READ_REQUEST_1_PARAM
            | MIPI_DSI_GENERIC_READ_REQUEST_2_PARAM
            | MIPI_DSI_DCS_SHORT_WRITE
            | MIPI_DSI_DCS_SHORT_WRITE_PARAM
            | MIPI_DSI_DCS_READ
            | MIPI_DSI_EXECUTE_QUEUE
            | MIPI_DSI_SET_MAXIMUM_RETURN_PACKET_SIZE
    )
}

pub extern "C" fn mipi_dsi_packet_format_is_long(type_: u8) -> bool {
    matches!(
        type_,
        MIPI_DSI_NULL_PACKET
            | MIPI_DSI_BLANKING_PACKET
            | MIPI_DSI_GENERIC_LONG_WRITE
            | MIPI_DSI_DCS_LONG_WRITE
            | MIPI_DSI_PICTURE_PARAMETER_SET
            | MIPI_DSI_COMPRESSED_PIXEL_STREAM
            | MIPI_DSI_LOOSELY_PACKED_PIXEL_STREAM_YCBCR20
            | MIPI_DSI_PACKED_PIXEL_STREAM_YCBCR24
            | MIPI_DSI_PACKED_PIXEL_STREAM_YCBCR16
            | MIPI_DSI_PACKED_PIXEL_STREAM_30
            | MIPI_DSI_PACKED_PIXEL_STREAM_36
            | MIPI_DSI_PACKED_PIXEL_STREAM_YCBCR12
            | MIPI_DSI_PACKED_PIXEL_STREAM_16
            | MIPI_DSI_PACKED_PIXEL_STREAM_18
            | MIPI_DSI_PIXEL_STREAM_3BYTE_18
            | MIPI_DSI_PACKED_PIXEL_STREAM_24
    )
}

pub unsafe extern "C" fn mipi_dsi_create_packet(
    packet: *mut MipiDsiPacket,
    msg: *const MipiDsiMsg,
) -> i32 {
    if packet.is_null() || msg.is_null() {
        return -EINVAL;
    }

    let msg = unsafe { &*msg };
    if !mipi_dsi_packet_format_is_short(msg.type_) && !mipi_dsi_packet_format_is_long(msg.type_) {
        return -EINVAL;
    }

    if msg.channel > 3 {
        return -EINVAL;
    }

    if msg.tx_len != 0 && msg.tx_buf.is_null() {
        return -EINVAL;
    }

    unsafe {
        ptr::write_bytes(packet, 0, 1);
        (*packet).header[0] = ((msg.channel & 0x3) << 6) | (msg.type_ & 0x3f);

        if mipi_dsi_packet_format_is_long(msg.type_) {
            (*packet).header[1] = (msg.tx_len & 0xff) as u8;
            (*packet).header[2] = ((msg.tx_len >> 8) & 0xff) as u8;
            (*packet).payload_length = msg.tx_len;
            (*packet).payload = msg.tx_buf.cast::<u8>();
        } else {
            let tx = msg.tx_buf.cast::<u8>();
            (*packet).header[1] = if msg.tx_len > 0 { *tx } else { 0 };
            (*packet).header[2] = if msg.tx_len > 1 { *tx.add(1) } else { 0 };
        }

        (*packet).size = core::mem::size_of_val(&(*packet).header) + (*packet).payload_length;
    }

    0
}

pub unsafe extern "C" fn mipi_dsi_attach(dsi: *mut MipiDsiDevice) -> i32 {
    let host = unsafe { read_dsi_host(dsi) };
    if host.is_null() {
        return -ENOSYS;
    }

    let ops = unsafe { (*host).ops };
    if ops.is_null() {
        return -ENOSYS;
    }

    let Some(attach) = (unsafe { (*ops).attach }) else {
        return -ENOSYS;
    };

    let ret = unsafe { attach(host, dsi) };
    if ret != 0 {
        return ret;
    }

    unsafe { write_dsi_attached(dsi, true) };
    0
}

pub unsafe extern "C" fn mipi_dsi_set_maximum_return_packet_size(
    dsi: *mut MipiDsiDevice,
    value: u16,
) -> i32 {
    let tx = [(value & 0xff) as u8, (value >> 8) as u8];
    let mut msg = MipiDsiMsg {
        channel: unsafe { read_dsi_channel(dsi) },
        type_: MIPI_DSI_SET_MAXIMUM_RETURN_PACKET_SIZE,
        flags: 0,
        tx_len: tx.len(),
        tx_buf: tx.as_ptr().cast::<c_void>(),
        rx_len: 0,
        rx_buf: ptr::null_mut(),
    };
    let ret = unsafe { mipi_dsi_device_transfer(dsi, &mut msg) };
    ret_zero_or_error(ret)
}

pub unsafe extern "C" fn mipi_dsi_compression_mode(dsi: *mut MipiDsiDevice, enable: bool) -> i32 {
    let tx = [u8::from(enable), 0];
    let mut msg = MipiDsiMsg {
        channel: unsafe { read_dsi_channel(dsi) },
        type_: MIPI_DSI_COMPRESSION_MODE,
        flags: 0,
        tx_len: tx.len(),
        tx_buf: tx.as_ptr().cast::<c_void>(),
        rx_len: 0,
        rx_buf: ptr::null_mut(),
    };
    let ret = unsafe { mipi_dsi_device_transfer(dsi, &mut msg) };
    ret_zero_or_error(ret)
}

pub unsafe extern "C" fn mipi_dsi_picture_parameter_set(
    dsi: *mut MipiDsiDevice,
    pps: *const c_void,
) -> i32 {
    let mut msg = MipiDsiMsg {
        channel: unsafe { read_dsi_channel(dsi) },
        type_: MIPI_DSI_PICTURE_PARAMETER_SET,
        flags: 0,
        tx_len: DRM_DSC_PICTURE_PARAMETER_SET_SIZE,
        tx_buf: pps,
        rx_len: 0,
        rx_buf: ptr::null_mut(),
    };
    let ret = unsafe { mipi_dsi_device_transfer(dsi, &mut msg) };
    ret_zero_or_error(ret)
}

pub unsafe extern "C" fn mipi_dsi_generic_write(
    dsi: *mut MipiDsiDevice,
    payload: *const c_void,
    size: usize,
) -> isize {
    let type_ = match size {
        0 => MIPI_DSI_GENERIC_SHORT_WRITE_0_PARAM,
        1 => MIPI_DSI_GENERIC_SHORT_WRITE_1_PARAM,
        2 => MIPI_DSI_GENERIC_SHORT_WRITE_2_PARAM,
        _ => MIPI_DSI_GENERIC_LONG_WRITE,
    };
    let mut msg = MipiDsiMsg {
        channel: unsafe { read_dsi_channel(dsi) },
        type_,
        flags: 0,
        tx_len: size,
        tx_buf: payload,
        rx_len: 0,
        rx_buf: ptr::null_mut(),
    };
    unsafe { mipi_dsi_device_transfer(dsi, &mut msg) }
}

pub unsafe extern "C" fn mipi_dsi_dcs_write_buffer(
    dsi: *mut MipiDsiDevice,
    data: *const c_void,
    len: usize,
) -> isize {
    let type_ = match len {
        0 => return -(EINVAL as isize),
        1 => MIPI_DSI_DCS_SHORT_WRITE,
        2 => MIPI_DSI_DCS_SHORT_WRITE_PARAM,
        _ => MIPI_DSI_DCS_LONG_WRITE,
    };
    let mut msg = MipiDsiMsg {
        channel: unsafe { read_dsi_channel(dsi) },
        type_,
        flags: 0,
        tx_len: len,
        tx_buf: data,
        rx_len: 0,
        rx_buf: ptr::null_mut(),
    };
    unsafe { mipi_dsi_device_transfer(dsi, &mut msg) }
}

pub unsafe extern "C" fn mipi_dsi_dcs_write(
    dsi: *mut MipiDsiDevice,
    cmd: u8,
    data: *const c_void,
    len: usize,
) -> isize {
    let Some(size) = len.checked_add(1) else {
        return -(EINVAL as isize);
    };

    let mut stack_tx = [0u8; 8];
    if size <= stack_tx.len() {
        stack_tx[0] = cmd;
        if !data.is_null() && len != 0 {
            unsafe {
                ptr::copy_nonoverlapping(data.cast::<u8>(), stack_tx.as_mut_ptr().add(1), len);
            }
        }
        return unsafe { mipi_dsi_dcs_write_buffer(dsi, stack_tx.as_ptr().cast::<c_void>(), size) };
    }

    let mut tx = Vec::<u8>::new();
    if tx.try_reserve_exact(size).is_err() {
        return -(ENOMEM as isize);
    }
    tx.resize(size, 0);
    tx[0] = cmd;
    if !data.is_null() && len != 0 {
        unsafe {
            ptr::copy_nonoverlapping(data.cast::<u8>(), tx.as_mut_ptr().add(1), len);
        }
    }
    unsafe { mipi_dsi_dcs_write_buffer(dsi, tx.as_ptr().cast::<c_void>(), size) }
}

pub unsafe extern "C" fn mipi_dsi_dcs_read(
    dsi: *mut MipiDsiDevice,
    cmd: u8,
    data: *mut c_void,
    len: usize,
) -> isize {
    let mut msg = MipiDsiMsg {
        channel: unsafe { read_dsi_channel(dsi) },
        type_: MIPI_DSI_DCS_READ,
        flags: 0,
        tx_len: 1,
        tx_buf: (&cmd as *const u8).cast::<c_void>(),
        rx_len: len,
        rx_buf: data,
    };
    unsafe { mipi_dsi_device_transfer(dsi, &mut msg) }
}

pub unsafe extern "C" fn mipi_dsi_dcs_nop(dsi: *mut MipiDsiDevice) -> i32 {
    let ret = unsafe { mipi_dsi_dcs_write(dsi, MIPI_DCS_NOP, ptr::null(), 0) };
    ret_zero_or_error(ret)
}

const _: () = assert!(core::mem::size_of::<MipiDsiMsg>() == 40);
const _: () = assert!(core::mem::offset_of!(MipiDsiMsg, type_) == 1);
const _: () = assert!(core::mem::offset_of!(MipiDsiMsg, tx_len) == 8);
const _: () = assert!(core::mem::offset_of!(MipiDsiMsg, tx_buf) == 16);
const _: () = assert!(core::mem::offset_of!(MipiDsiMsg, rx_len) == 24);
const _: () = assert!(core::mem::offset_of!(MipiDsiMsg, rx_buf) == 32);
const _: () = assert!(core::mem::size_of::<MipiDsiPacket>() == 32);
const _: () = assert!(core::mem::offset_of!(MipiDsiPacket, header) == 8);
const _: () = assert!(core::mem::offset_of!(MipiDsiPacket, payload_length) == 16);
const _: () = assert!(core::mem::offset_of!(MipiDsiPacket, payload) == 24);
const _: () = assert!(core::mem::size_of::<MipiDsiHostOps>() == 24);
const _: () = assert!(core::mem::offset_of!(MipiDsiHostOps, detach) == 8);
const _: () = assert!(core::mem::offset_of!(MipiDsiHostOps, transfer) == 16);
const _: () = assert!(core::mem::size_of::<MipiDsiHost>() == 32);
const _: () = assert!(core::mem::offset_of!(MipiDsiHost, ops) == 8);
