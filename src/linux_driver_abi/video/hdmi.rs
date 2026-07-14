//! linux-parity: partial
//! linux-source: vendor/linux/drivers/video/hdmi.c
//! HDMI infoframe helper exports built into Linux when `CONFIG_HDMI=y`.

use core::ffi::{c_char, c_void};

use crate::include::uapi::errno::{EINVAL, ENOSPC};
use crate::kernel::module::{export_symbol, find_symbol};

const HDMI_INFOFRAME_TYPE_VENDOR: i32 = 0x81;
const HDMI_INFOFRAME_TYPE_AVI: i32 = 0x82;
const HDMI_INFOFRAME_TYPE_SPD: i32 = 0x83;
const HDMI_INFOFRAME_TYPE_AUDIO: i32 = 0x84;
const HDMI_INFOFRAME_TYPE_DRM: i32 = 0x87;

const HDMI_IEEE_OUI: u32 = 0x000c03;
const HDMI_INFOFRAME_HEADER_SIZE: usize = 4;
const HDMI_AVI_INFOFRAME_SIZE: u8 = 13;
const HDMI_SPD_INFOFRAME_SIZE: u8 = 25;
const HDMI_AUDIO_INFOFRAME_SIZE: u8 = 10;
const HDMI_DRM_INFOFRAME_SIZE: u8 = 26;
const HDMI_VENDOR_INFOFRAME_SIZE: u8 = 4;

const HDMI_PICTURE_ASPECT_16_9: i32 = 2;
const HDMI_3D_STRUCTURE_INVALID: i32 = -1;
const HDMI_3D_STRUCTURE_SIDE_BY_SIDE_HALF: i32 = 8;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxHdmiAnyInfoframe {
    pub type_: i32,
    pub version: u8,
    pub length: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxHdmiAviInfoframe {
    pub type_: i32,
    pub version: u8,
    pub length: u8,
    pub itc: u8,
    pub pixel_repeat: u8,
    pub colorspace: i32,
    pub scan_mode: i32,
    pub colorimetry: i32,
    pub picture_aspect: i32,
    pub active_aspect: i32,
    pub extended_colorimetry: i32,
    pub quantization_range: i32,
    pub nups: i32,
    pub video_code: u8,
    pub _pad_video_code: [u8; 3],
    pub ycc_quantization_range: i32,
    pub content_type: i32,
    pub top_bar: u16,
    pub bottom_bar: u16,
    pub left_bar: u16,
    pub right_bar: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxHdmiSpdInfoframe {
    pub type_: i32,
    pub version: u8,
    pub length: u8,
    pub vendor: [u8; 8],
    pub product: [u8; 16],
    pub sdi: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxHdmiAudioInfoframe {
    pub type_: i32,
    pub version: u8,
    pub length: u8,
    pub channels: u8,
    pub _pad_channels: u8,
    pub coding_type: i32,
    pub sample_size: i32,
    pub sample_frequency: i32,
    pub coding_type_ext: i32,
    pub channel_allocation: u8,
    pub level_shift_value: u8,
    pub downmix_inhibit: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxHdmiVendorInfoframe {
    pub type_: i32,
    pub version: u8,
    pub length: u8,
    pub _pad_length: [u8; 2],
    pub oui: u32,
    pub vic: u8,
    pub _pad_vic: [u8; 3],
    pub s3d_struct: i32,
    pub s3d_ext_data: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxHdmiVendorAnyHeader {
    pub type_: i32,
    pub version: u8,
    pub length: u8,
    pub _pad_length: [u8; 2],
    pub oui: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxHdmiXy {
    pub x: u16,
    pub y: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxHdmiDrmInfoframe {
    pub type_: i32,
    pub version: u8,
    pub length: u8,
    pub _pad_length: [u8; 2],
    pub eotf: i32,
    pub metadata_type: i32,
    pub display_primaries: [LinuxHdmiXy; 3],
    pub white_point: LinuxHdmiXy,
    pub max_display_mastering_luminance: u16,
    pub min_display_mastering_luminance: u16,
    pub max_cll: u16,
    pub max_fall: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union LinuxHdmiVendorAnyInfoframe {
    pub any: LinuxHdmiVendorAnyHeader,
    pub hdmi: LinuxHdmiVendorInfoframe,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union LinuxHdmiInfoframe {
    pub any: LinuxHdmiAnyInfoframe,
    pub avi: LinuxHdmiAviInfoframe,
    pub spd: LinuxHdmiSpdInfoframe,
    pub vendor: LinuxHdmiVendorAnyInfoframe,
    pub audio: LinuxHdmiAudioInfoframe,
    pub drm: LinuxHdmiDrmInfoframe,
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "hdmi_avi_infoframe_init",
        linux_hdmi_avi_infoframe_init as usize,
        false,
    );
    export_symbol_once(
        "hdmi_avi_infoframe_check",
        linux_hdmi_avi_infoframe_check as usize,
        false,
    );
    export_symbol_once(
        "hdmi_avi_infoframe_pack_only",
        linux_hdmi_avi_infoframe_pack_only as usize,
        false,
    );
    export_symbol_once(
        "hdmi_avi_infoframe_pack",
        linux_hdmi_avi_infoframe_pack as usize,
        false,
    );
    export_symbol_once(
        "hdmi_spd_infoframe_init",
        linux_hdmi_spd_infoframe_init as usize,
        false,
    );
    export_symbol_once(
        "hdmi_spd_infoframe_check",
        linux_hdmi_spd_infoframe_check as usize,
        false,
    );
    export_symbol_once(
        "hdmi_spd_infoframe_pack_only",
        linux_hdmi_spd_infoframe_pack_only as usize,
        false,
    );
    export_symbol_once(
        "hdmi_spd_infoframe_pack",
        linux_hdmi_spd_infoframe_pack as usize,
        false,
    );
    export_symbol_once(
        "hdmi_audio_infoframe_init",
        linux_hdmi_audio_infoframe_init as usize,
        false,
    );
    export_symbol_once(
        "hdmi_audio_infoframe_check",
        linux_hdmi_audio_infoframe_check as usize,
        false,
    );
    export_symbol_once(
        "hdmi_audio_infoframe_pack_only",
        linux_hdmi_audio_infoframe_pack_only as usize,
        false,
    );
    export_symbol_once(
        "hdmi_audio_infoframe_pack",
        linux_hdmi_audio_infoframe_pack as usize,
        false,
    );
    export_symbol_once(
        "hdmi_vendor_infoframe_init",
        linux_hdmi_vendor_infoframe_init as usize,
        false,
    );
    export_symbol_once(
        "hdmi_vendor_infoframe_check",
        linux_hdmi_vendor_infoframe_check as usize,
        false,
    );
    export_symbol_once(
        "hdmi_vendor_infoframe_pack_only",
        linux_hdmi_vendor_infoframe_pack_only as usize,
        false,
    );
    export_symbol_once(
        "hdmi_vendor_infoframe_pack",
        linux_hdmi_vendor_infoframe_pack as usize,
        false,
    );
    export_symbol_once(
        "hdmi_drm_infoframe_init",
        linux_hdmi_drm_infoframe_init as usize,
        false,
    );
    export_symbol_once(
        "hdmi_drm_infoframe_check",
        linux_hdmi_drm_infoframe_check as usize,
        false,
    );
    export_symbol_once(
        "hdmi_drm_infoframe_pack_only",
        linux_hdmi_drm_infoframe_pack_only as usize,
        false,
    );
    export_symbol_once(
        "hdmi_drm_infoframe_pack",
        linux_hdmi_drm_infoframe_pack as usize,
        false,
    );
    export_symbol_once(
        "hdmi_drm_infoframe_unpack_only",
        linux_hdmi_drm_infoframe_unpack_only as usize,
        false,
    );
    export_symbol_once(
        "hdmi_infoframe_pack_only",
        linux_hdmi_infoframe_pack_only as usize,
        false,
    );
    export_symbol_once(
        "hdmi_infoframe_pack",
        linux_hdmi_infoframe_pack as usize,
        false,
    );
    export_symbol_once(
        "hdmi_infoframe_unpack",
        linux_hdmi_infoframe_unpack as usize,
        false,
    );
    export_symbol_once(
        "hdmi_infoframe_log",
        linux_hdmi_infoframe_log as usize,
        false,
    );
}

const fn neg_errno(errno: i32) -> isize {
    -(errno as isize)
}

unsafe fn zero_struct<T>(ptr: *mut T) {
    if !ptr.is_null() {
        unsafe {
            core::ptr::write_bytes(ptr.cast::<u8>(), 0, core::mem::size_of::<T>());
        }
    }
}

unsafe fn bytes_mut<'a>(ptr: *mut c_void, size: usize) -> Result<&'a mut [u8], isize> {
    if ptr.is_null() {
        return Err(neg_errno(EINVAL));
    }
    Ok(unsafe { core::slice::from_raw_parts_mut(ptr.cast::<u8>(), size) })
}

unsafe fn bytes<'a>(ptr: *const c_void, size: usize) -> Result<&'a [u8], i32> {
    if ptr.is_null() {
        return Err(-EINVAL);
    }
    Ok(unsafe { core::slice::from_raw_parts(ptr.cast::<u8>(), size) })
}

fn hdmi_infoframe_checksum(buf: &[u8]) -> u8 {
    let sum = buf.iter().fold(0u8, |acc, byte| acc.wrapping_add(*byte));
    0u8.wrapping_sub(sum)
}

fn hdmi_infoframe_set_checksum(buf: &mut [u8], length: usize) {
    buf[3] = 0;
    let checksum = hdmi_infoframe_checksum(&buf[..length]);
    buf[3] = checksum;
}

fn fill_header(buf: &mut [u8], type_: i32, version: u8, length: u8) {
    buf[0] = type_ as u8;
    buf[1] = version;
    buf[2] = length;
    buf[3] = 0;
}

fn c_strlen_bounded(ptr: *const c_char, limit: usize) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let mut len = 0usize;
    while len < limit {
        if unsafe { *ptr.add(len) } == 0 {
            break;
        }
        len += 1;
    }
    len
}

fn copy_c_string_trunc(dst: &mut [u8], src: *const c_char) {
    let len = c_strlen_bounded(src, 4096).min(dst.len());
    for (index, slot) in dst.iter_mut().take(len).enumerate() {
        *slot = unsafe { *src.add(index) } as u8;
    }
}

fn hdmi_avi_infoframe_check_only(frame: &LinuxHdmiAviInfoframe) -> i32 {
    if frame.type_ != HDMI_INFOFRAME_TYPE_AVI
        || frame.version != 2
        || frame.length != HDMI_AVI_INFOFRAME_SIZE
    {
        return -EINVAL;
    }
    if frame.picture_aspect > HDMI_PICTURE_ASPECT_16_9 {
        return -EINVAL;
    }
    0
}

pub unsafe extern "C" fn linux_hdmi_avi_infoframe_init(frame: *mut LinuxHdmiAviInfoframe) {
    if frame.is_null() {
        return;
    }
    unsafe {
        zero_struct(frame);
        (*frame).type_ = HDMI_INFOFRAME_TYPE_AVI;
        (*frame).version = 2;
        (*frame).length = HDMI_AVI_INFOFRAME_SIZE;
    }
}

pub unsafe extern "C" fn linux_hdmi_avi_infoframe_check(frame: *mut LinuxHdmiAviInfoframe) -> i32 {
    if frame.is_null() {
        return -EINVAL;
    }
    hdmi_avi_infoframe_check_only(unsafe { &*frame })
}

pub unsafe extern "C" fn linux_hdmi_avi_infoframe_pack_only(
    frame: *const LinuxHdmiAviInfoframe,
    buffer: *mut c_void,
    size: usize,
) -> isize {
    if frame.is_null() {
        return neg_errno(EINVAL);
    }
    let frame = unsafe { &*frame };
    let ret = hdmi_avi_infoframe_check_only(frame);
    if ret != 0 {
        return ret as isize;
    }

    let length = HDMI_INFOFRAME_HEADER_SIZE + frame.length as usize;
    if size < length {
        return neg_errno(ENOSPC);
    }
    let out = match unsafe { bytes_mut(buffer, size) } {
        Ok(out) => out,
        Err(err) => return err,
    };
    out.fill(0);
    fill_header(out, frame.type_, frame.version, frame.length);

    let payload = &mut out[HDMI_INFOFRAME_HEADER_SIZE..];
    payload[0] = (((frame.colorspace & 0x3) << 5) | (frame.scan_mode & 0x3)) as u8;
    if frame.active_aspect & 0xf != 0 {
        payload[0] |= 1 << 4;
    }
    if frame.top_bar != 0 || frame.bottom_bar != 0 {
        payload[0] |= 1 << 3;
    }
    if frame.left_bar != 0 || frame.right_bar != 0 {
        payload[0] |= 1 << 2;
    }

    payload[1] = (((frame.colorimetry & 0x3) << 6)
        | ((frame.picture_aspect & 0x3) << 4)
        | (frame.active_aspect & 0xf)) as u8;
    payload[2] = (((frame.extended_colorimetry & 0x7) << 4)
        | ((frame.quantization_range & 0x3) << 2)
        | (frame.nups & 0x3)) as u8;
    if frame.itc != 0 {
        payload[2] |= 1 << 7;
    }
    payload[3] = frame.video_code & 0x7f;
    payload[4] = (((frame.ycc_quantization_range & 0x3) << 6)
        | ((frame.content_type & 0x3) << 4)
        | (frame.pixel_repeat as i32 & 0xf)) as u8;
    payload[5] = frame.top_bar as u8;
    payload[6] = (frame.top_bar >> 8) as u8;
    payload[7] = frame.bottom_bar as u8;
    payload[8] = (frame.bottom_bar >> 8) as u8;
    payload[9] = frame.left_bar as u8;
    payload[10] = (frame.left_bar >> 8) as u8;
    payload[11] = frame.right_bar as u8;
    payload[12] = (frame.right_bar >> 8) as u8;

    hdmi_infoframe_set_checksum(out, length);
    length as isize
}

pub unsafe extern "C" fn linux_hdmi_avi_infoframe_pack(
    frame: *mut LinuxHdmiAviInfoframe,
    buffer: *mut c_void,
    size: usize,
) -> isize {
    let ret = unsafe { linux_hdmi_avi_infoframe_check(frame) };
    if ret != 0 {
        return ret as isize;
    }
    unsafe { linux_hdmi_avi_infoframe_pack_only(frame, buffer, size) }
}

fn hdmi_spd_infoframe_check_only(frame: &LinuxHdmiSpdInfoframe) -> i32 {
    if frame.type_ != HDMI_INFOFRAME_TYPE_SPD
        || frame.version != 1
        || frame.length != HDMI_SPD_INFOFRAME_SIZE
    {
        return -EINVAL;
    }
    0
}

pub unsafe extern "C" fn linux_hdmi_spd_infoframe_init(
    frame: *mut LinuxHdmiSpdInfoframe,
    vendor: *const c_char,
    product: *const c_char,
) -> i32 {
    if frame.is_null() {
        return -EINVAL;
    }
    unsafe {
        zero_struct(frame);
        (*frame).type_ = HDMI_INFOFRAME_TYPE_SPD;
        (*frame).version = 1;
        (*frame).length = HDMI_SPD_INFOFRAME_SIZE;
        copy_c_string_trunc(&mut (*frame).vendor, vendor);
        copy_c_string_trunc(&mut (*frame).product, product);
    }
    0
}

pub unsafe extern "C" fn linux_hdmi_spd_infoframe_check(frame: *mut LinuxHdmiSpdInfoframe) -> i32 {
    if frame.is_null() {
        return -EINVAL;
    }
    hdmi_spd_infoframe_check_only(unsafe { &*frame })
}

pub unsafe extern "C" fn linux_hdmi_spd_infoframe_pack_only(
    frame: *const LinuxHdmiSpdInfoframe,
    buffer: *mut c_void,
    size: usize,
) -> isize {
    if frame.is_null() {
        return neg_errno(EINVAL);
    }
    let frame = unsafe { &*frame };
    let ret = hdmi_spd_infoframe_check_only(frame);
    if ret != 0 {
        return ret as isize;
    }
    let length = HDMI_INFOFRAME_HEADER_SIZE + frame.length as usize;
    if size < length {
        return neg_errno(ENOSPC);
    }
    let out = match unsafe { bytes_mut(buffer, size) } {
        Ok(out) => out,
        Err(err) => return err,
    };
    out.fill(0);
    fill_header(out, frame.type_, frame.version, frame.length);
    let payload = &mut out[HDMI_INFOFRAME_HEADER_SIZE..];
    payload[..8].copy_from_slice(&frame.vendor);
    payload[8..24].copy_from_slice(&frame.product);
    payload[24] = frame.sdi as u8;
    hdmi_infoframe_set_checksum(out, length);
    length as isize
}

pub unsafe extern "C" fn linux_hdmi_spd_infoframe_pack(
    frame: *mut LinuxHdmiSpdInfoframe,
    buffer: *mut c_void,
    size: usize,
) -> isize {
    let ret = unsafe { linux_hdmi_spd_infoframe_check(frame) };
    if ret != 0 {
        return ret as isize;
    }
    unsafe { linux_hdmi_spd_infoframe_pack_only(frame, buffer, size) }
}

fn hdmi_audio_infoframe_check_only(frame: &LinuxHdmiAudioInfoframe) -> i32 {
    if frame.type_ != HDMI_INFOFRAME_TYPE_AUDIO
        || frame.version != 1
        || frame.length != HDMI_AUDIO_INFOFRAME_SIZE
    {
        return -EINVAL;
    }
    0
}

pub unsafe extern "C" fn linux_hdmi_audio_infoframe_init(
    frame: *mut LinuxHdmiAudioInfoframe,
) -> i32 {
    if frame.is_null() {
        return -EINVAL;
    }
    unsafe {
        zero_struct(frame);
        (*frame).type_ = HDMI_INFOFRAME_TYPE_AUDIO;
        (*frame).version = 1;
        (*frame).length = HDMI_AUDIO_INFOFRAME_SIZE;
    }
    0
}

pub unsafe extern "C" fn linux_hdmi_audio_infoframe_check(
    frame: *const LinuxHdmiAudioInfoframe,
) -> i32 {
    if frame.is_null() {
        return -EINVAL;
    }
    hdmi_audio_infoframe_check_only(unsafe { &*frame })
}

fn hdmi_audio_infoframe_pack_payload(frame: &LinuxHdmiAudioInfoframe, payload: &mut [u8]) {
    let channels = if frame.channels >= 2 {
        frame.channels - 1
    } else {
        0
    };
    payload[0] = (((frame.coding_type & 0xf) << 4) | (channels as i32 & 0x7)) as u8;
    payload[1] = (((frame.sample_frequency & 0x7) << 2) | (frame.sample_size & 0x3)) as u8;
    payload[2] = (frame.coding_type_ext & 0x1f) as u8;
    payload[3] = frame.channel_allocation;
    payload[4] = ((frame.level_shift_value & 0xf) << 3) as u8;
    if frame.downmix_inhibit != 0 {
        payload[4] |= 1 << 7;
    }
}

pub unsafe extern "C" fn linux_hdmi_audio_infoframe_pack_only(
    frame: *const LinuxHdmiAudioInfoframe,
    buffer: *mut c_void,
    size: usize,
) -> isize {
    if frame.is_null() {
        return neg_errno(EINVAL);
    }
    let frame = unsafe { &*frame };
    let ret = hdmi_audio_infoframe_check_only(frame);
    if ret != 0 {
        return ret as isize;
    }
    let length = HDMI_INFOFRAME_HEADER_SIZE + frame.length as usize;
    if size < length {
        return neg_errno(ENOSPC);
    }
    let out = match unsafe { bytes_mut(buffer, size) } {
        Ok(out) => out,
        Err(err) => return err,
    };
    out.fill(0);
    fill_header(out, frame.type_, frame.version, frame.length);
    hdmi_audio_infoframe_pack_payload(frame, &mut out[HDMI_INFOFRAME_HEADER_SIZE..]);
    hdmi_infoframe_set_checksum(out, length);
    length as isize
}

pub unsafe extern "C" fn linux_hdmi_audio_infoframe_pack(
    frame: *mut LinuxHdmiAudioInfoframe,
    buffer: *mut c_void,
    size: usize,
) -> isize {
    let ret = unsafe { linux_hdmi_audio_infoframe_check(frame) };
    if ret != 0 {
        return ret as isize;
    }
    unsafe { linux_hdmi_audio_infoframe_pack_only(frame, buffer, size) }
}

fn hdmi_vendor_infoframe_length(frame: &LinuxHdmiVendorInfoframe) -> u8 {
    if frame.s3d_struct >= HDMI_3D_STRUCTURE_SIDE_BY_SIDE_HALF {
        6
    } else if frame.vic != 0 || frame.s3d_struct != HDMI_3D_STRUCTURE_INVALID {
        5
    } else {
        4
    }
}

fn hdmi_vendor_infoframe_check_only(frame: &LinuxHdmiVendorInfoframe) -> i32 {
    if frame.type_ != HDMI_INFOFRAME_TYPE_VENDOR || frame.version != 1 || frame.oui != HDMI_IEEE_OUI
    {
        return -EINVAL;
    }
    if frame.vic != 0 && frame.s3d_struct != HDMI_3D_STRUCTURE_INVALID {
        return -EINVAL;
    }
    if frame.length != hdmi_vendor_infoframe_length(frame) {
        return -EINVAL;
    }
    0
}

pub unsafe extern "C" fn linux_hdmi_vendor_infoframe_init(
    frame: *mut LinuxHdmiVendorInfoframe,
) -> i32 {
    if frame.is_null() {
        return -EINVAL;
    }
    unsafe {
        zero_struct(frame);
        (*frame).type_ = HDMI_INFOFRAME_TYPE_VENDOR;
        (*frame).version = 1;
        (*frame).oui = HDMI_IEEE_OUI;
        (*frame).s3d_struct = HDMI_3D_STRUCTURE_INVALID;
        (*frame).length = HDMI_VENDOR_INFOFRAME_SIZE;
    }
    0
}

pub unsafe extern "C" fn linux_hdmi_vendor_infoframe_check(
    frame: *mut LinuxHdmiVendorInfoframe,
) -> i32 {
    if frame.is_null() {
        return -EINVAL;
    }
    unsafe {
        (*frame).length = hdmi_vendor_infoframe_length(&*frame);
        hdmi_vendor_infoframe_check_only(&*frame)
    }
}

pub unsafe extern "C" fn linux_hdmi_vendor_infoframe_pack_only(
    frame: *const LinuxHdmiVendorInfoframe,
    buffer: *mut c_void,
    size: usize,
) -> isize {
    if frame.is_null() {
        return neg_errno(EINVAL);
    }
    let frame = unsafe { &*frame };
    let ret = hdmi_vendor_infoframe_check_only(frame);
    if ret != 0 {
        return ret as isize;
    }
    let length = HDMI_INFOFRAME_HEADER_SIZE + frame.length as usize;
    if size < length {
        return neg_errno(ENOSPC);
    }
    let out = match unsafe { bytes_mut(buffer, size) } {
        Ok(out) => out,
        Err(err) => return err,
    };
    out.fill(0);
    fill_header(out, frame.type_, frame.version, frame.length);
    out[4] = 0x03;
    out[5] = 0x0c;
    out[6] = 0x00;
    if frame.s3d_struct != HDMI_3D_STRUCTURE_INVALID {
        out[7] = 0x2 << 5;
        out[8] = ((frame.s3d_struct & 0xf) << 4) as u8;
        if frame.s3d_struct >= HDMI_3D_STRUCTURE_SIDE_BY_SIDE_HALF {
            out[9] = ((frame.s3d_ext_data & 0xf) << 4) as u8;
        }
    } else if frame.vic != 0 {
        out[7] = 0x1 << 5;
        out[8] = frame.vic;
    } else {
        out[7] = 0;
    }
    hdmi_infoframe_set_checksum(out, length);
    length as isize
}

pub unsafe extern "C" fn linux_hdmi_vendor_infoframe_pack(
    frame: *mut LinuxHdmiVendorInfoframe,
    buffer: *mut c_void,
    size: usize,
) -> isize {
    let ret = unsafe { linux_hdmi_vendor_infoframe_check(frame) };
    if ret != 0 {
        return ret as isize;
    }
    unsafe { linux_hdmi_vendor_infoframe_pack_only(frame, buffer, size) }
}

fn hdmi_drm_infoframe_check_only(frame: &LinuxHdmiDrmInfoframe) -> i32 {
    if frame.type_ != HDMI_INFOFRAME_TYPE_DRM || frame.version != 1 {
        return -EINVAL;
    }
    if frame.length != HDMI_DRM_INFOFRAME_SIZE {
        return -EINVAL;
    }
    0
}

pub unsafe extern "C" fn linux_hdmi_drm_infoframe_init(frame: *mut LinuxHdmiDrmInfoframe) -> i32 {
    if frame.is_null() {
        return -EINVAL;
    }
    unsafe {
        zero_struct(frame);
        (*frame).type_ = HDMI_INFOFRAME_TYPE_DRM;
        (*frame).version = 1;
        (*frame).length = HDMI_DRM_INFOFRAME_SIZE;
    }
    0
}

pub unsafe extern "C" fn linux_hdmi_drm_infoframe_check(frame: *mut LinuxHdmiDrmInfoframe) -> i32 {
    if frame.is_null() {
        return -EINVAL;
    }
    hdmi_drm_infoframe_check_only(unsafe { &*frame })
}

pub unsafe extern "C" fn linux_hdmi_drm_infoframe_pack_only(
    frame: *const LinuxHdmiDrmInfoframe,
    buffer: *mut c_void,
    size: usize,
) -> isize {
    if frame.is_null() {
        return neg_errno(EINVAL);
    }
    let frame = unsafe { &*frame };
    let length = HDMI_INFOFRAME_HEADER_SIZE + frame.length as usize;
    if size < length {
        return neg_errno(ENOSPC);
    }
    let out = match unsafe { bytes_mut(buffer, size) } {
        Ok(out) => out,
        Err(err) => return err,
    };
    out.fill(0);
    fill_header(out, frame.type_, frame.version, frame.length);

    let payload = &mut out[HDMI_INFOFRAME_HEADER_SIZE..];
    let mut index = 0usize;
    payload[index] = frame.eotf as u8;
    index += 1;
    payload[index] = frame.metadata_type as u8;
    index += 1;
    for primary in &frame.display_primaries {
        payload[index] = primary.x as u8;
        payload[index + 1] = (primary.x >> 8) as u8;
        payload[index + 2] = primary.y as u8;
        payload[index + 3] = (primary.y >> 8) as u8;
        index += 4;
    }
    for value in [
        frame.white_point.x,
        frame.white_point.y,
        frame.max_display_mastering_luminance,
        frame.min_display_mastering_luminance,
        frame.max_cll,
        frame.max_fall,
    ] {
        payload[index] = value as u8;
        payload[index + 1] = (value >> 8) as u8;
        index += 2;
    }

    hdmi_infoframe_set_checksum(out, length);
    length as isize
}

pub unsafe extern "C" fn linux_hdmi_drm_infoframe_pack(
    frame: *mut LinuxHdmiDrmInfoframe,
    buffer: *mut c_void,
    size: usize,
) -> isize {
    let ret = unsafe { linux_hdmi_drm_infoframe_check(frame) };
    if ret != 0 {
        return ret as isize;
    }
    unsafe { linux_hdmi_drm_infoframe_pack_only(frame, buffer, size) }
}

fn hdmi_vendor_any_infoframe_check_only(frame: &LinuxHdmiVendorAnyInfoframe) -> i32 {
    let any = unsafe { frame.any };
    if any.type_ != HDMI_INFOFRAME_TYPE_VENDOR || any.version != 1 {
        return -EINVAL;
    }
    0
}

fn hdmi_vendor_any_infoframe_check(frame: *mut LinuxHdmiVendorAnyInfoframe) -> i32 {
    if frame.is_null() {
        return -EINVAL;
    }
    let ret = hdmi_vendor_any_infoframe_check_only(unsafe { &*frame });
    if ret != 0 {
        return ret;
    }
    let any = unsafe { (*frame).any };
    if any.oui != HDMI_IEEE_OUI {
        return -EINVAL;
    }
    unsafe { linux_hdmi_vendor_infoframe_check(frame.cast::<LinuxHdmiVendorInfoframe>()) }
}

unsafe fn hdmi_vendor_any_infoframe_pack_only(
    frame: *const LinuxHdmiVendorAnyInfoframe,
    buffer: *mut c_void,
    size: usize,
) -> isize {
    if frame.is_null() {
        return neg_errno(EINVAL);
    }
    let ret = hdmi_vendor_any_infoframe_check_only(unsafe { &*frame });
    if ret != 0 {
        return ret as isize;
    }
    let any = unsafe { (*frame).any };
    if any.oui != HDMI_IEEE_OUI {
        return neg_errno(EINVAL);
    }
    unsafe {
        linux_hdmi_vendor_infoframe_pack_only(
            frame.cast::<LinuxHdmiVendorInfoframe>(),
            buffer,
            size,
        )
    }
}

unsafe fn hdmi_vendor_any_infoframe_pack(
    frame: *mut LinuxHdmiVendorAnyInfoframe,
    buffer: *mut c_void,
    size: usize,
) -> isize {
    let ret = hdmi_vendor_any_infoframe_check(frame);
    if ret != 0 {
        return ret as isize;
    }
    unsafe { hdmi_vendor_any_infoframe_pack_only(frame, buffer, size) }
}

pub unsafe extern "C" fn linux_hdmi_infoframe_pack_only(
    frame: *const LinuxHdmiInfoframe,
    buffer: *mut c_void,
    size: usize,
) -> isize {
    if frame.is_null() {
        return neg_errno(EINVAL);
    }
    let type_ = unsafe { (*frame).any.type_ };
    match type_ {
        HDMI_INFOFRAME_TYPE_AVI => unsafe {
            linux_hdmi_avi_infoframe_pack_only(core::ptr::addr_of!((*frame).avi), buffer, size)
        },
        HDMI_INFOFRAME_TYPE_DRM => unsafe {
            linux_hdmi_drm_infoframe_pack_only(core::ptr::addr_of!((*frame).drm), buffer, size)
        },
        HDMI_INFOFRAME_TYPE_SPD => unsafe {
            linux_hdmi_spd_infoframe_pack_only(core::ptr::addr_of!((*frame).spd), buffer, size)
        },
        HDMI_INFOFRAME_TYPE_AUDIO => unsafe {
            linux_hdmi_audio_infoframe_pack_only(core::ptr::addr_of!((*frame).audio), buffer, size)
        },
        HDMI_INFOFRAME_TYPE_VENDOR => unsafe {
            hdmi_vendor_any_infoframe_pack_only(core::ptr::addr_of!((*frame).vendor), buffer, size)
        },
        _ => neg_errno(EINVAL),
    }
}

pub unsafe extern "C" fn linux_hdmi_infoframe_pack(
    frame: *mut LinuxHdmiInfoframe,
    buffer: *mut c_void,
    size: usize,
) -> isize {
    if frame.is_null() {
        return neg_errno(EINVAL);
    }
    let type_ = unsafe { (*frame).any.type_ };
    match type_ {
        HDMI_INFOFRAME_TYPE_AVI => unsafe {
            linux_hdmi_avi_infoframe_pack(core::ptr::addr_of_mut!((*frame).avi), buffer, size)
        },
        HDMI_INFOFRAME_TYPE_DRM => unsafe {
            linux_hdmi_drm_infoframe_pack(core::ptr::addr_of_mut!((*frame).drm), buffer, size)
        },
        HDMI_INFOFRAME_TYPE_SPD => unsafe {
            linux_hdmi_spd_infoframe_pack(core::ptr::addr_of_mut!((*frame).spd), buffer, size)
        },
        HDMI_INFOFRAME_TYPE_AUDIO => unsafe {
            linux_hdmi_audio_infoframe_pack(core::ptr::addr_of_mut!((*frame).audio), buffer, size)
        },
        HDMI_INFOFRAME_TYPE_VENDOR => unsafe {
            hdmi_vendor_any_infoframe_pack(core::ptr::addr_of_mut!((*frame).vendor), buffer, size)
        },
        _ => neg_errno(EINVAL),
    }
}

unsafe fn hdmi_avi_infoframe_unpack(
    frame: *mut LinuxHdmiAviInfoframe,
    buffer: *const c_void,
    size: usize,
) -> i32 {
    let buf = match unsafe { bytes(buffer, size) } {
        Ok(buf) => buf,
        Err(err) => return err,
    };
    let length = HDMI_INFOFRAME_HEADER_SIZE + HDMI_AVI_INFOFRAME_SIZE as usize;
    if size < length
        || buf[0] != HDMI_INFOFRAME_TYPE_AVI as u8
        || buf[1] != 2
        || buf[2] != HDMI_AVI_INFOFRAME_SIZE
        || hdmi_infoframe_checksum(&buf[..length]) != 0
    {
        return -EINVAL;
    }
    unsafe { linux_hdmi_avi_infoframe_init(frame) };
    let payload = &buf[HDMI_INFOFRAME_HEADER_SIZE..];
    unsafe {
        (*frame).colorspace = ((payload[0] >> 5) & 0x3) as i32;
        if payload[0] & 0x10 != 0 {
            (*frame).active_aspect = (payload[1] & 0xf) as i32;
        }
        if payload[0] & 0x8 != 0 {
            (*frame).top_bar = ((payload[6] as u16) << 8) | payload[5] as u16;
            (*frame).bottom_bar = ((payload[8] as u16) << 8) | payload[7] as u16;
        }
        if payload[0] & 0x4 != 0 {
            (*frame).left_bar = ((payload[10] as u16) << 8) | payload[9] as u16;
            (*frame).right_bar = ((payload[12] as u16) << 8) | payload[11] as u16;
        }
        (*frame).scan_mode = (payload[0] & 0x3) as i32;
        (*frame).colorimetry = ((payload[1] >> 6) & 0x3) as i32;
        (*frame).picture_aspect = ((payload[1] >> 4) & 0x3) as i32;
        (*frame).active_aspect = (payload[1] & 0xf) as i32;
        (*frame).itc = u8::from(payload[2] & 0x80 != 0);
        (*frame).extended_colorimetry = ((payload[2] >> 4) & 0x7) as i32;
        (*frame).quantization_range = ((payload[2] >> 2) & 0x3) as i32;
        (*frame).nups = (payload[2] & 0x3) as i32;
        (*frame).video_code = payload[3] & 0x7f;
        (*frame).ycc_quantization_range = ((payload[4] >> 6) & 0x3) as i32;
        (*frame).content_type = ((payload[4] >> 4) & 0x3) as i32;
        (*frame).pixel_repeat = payload[4] & 0xf;
    }
    0
}

unsafe fn hdmi_spd_infoframe_unpack(
    frame: *mut LinuxHdmiSpdInfoframe,
    buffer: *const c_void,
    size: usize,
) -> i32 {
    let buf = match unsafe { bytes(buffer, size) } {
        Ok(buf) => buf,
        Err(err) => return err,
    };
    let length = HDMI_INFOFRAME_HEADER_SIZE + HDMI_SPD_INFOFRAME_SIZE as usize;
    if size < length
        || buf[0] != HDMI_INFOFRAME_TYPE_SPD as u8
        || buf[1] != 1
        || buf[2] != HDMI_SPD_INFOFRAME_SIZE
        || hdmi_infoframe_checksum(&buf[..length]) != 0
    {
        return -EINVAL;
    }
    if frame.is_null() {
        return -EINVAL;
    }
    unsafe {
        zero_struct(frame);
        (*frame).type_ = HDMI_INFOFRAME_TYPE_SPD;
        (*frame).version = 1;
        (*frame).length = HDMI_SPD_INFOFRAME_SIZE;
        let payload = &buf[HDMI_INFOFRAME_HEADER_SIZE..];
        (*frame).vendor.copy_from_slice(&payload[..8]);
        (*frame).product.copy_from_slice(&payload[8..24]);
        (*frame).sdi = payload[24] as i32;
    }
    0
}

unsafe fn hdmi_audio_infoframe_unpack(
    frame: *mut LinuxHdmiAudioInfoframe,
    buffer: *const c_void,
    size: usize,
) -> i32 {
    let buf = match unsafe { bytes(buffer, size) } {
        Ok(buf) => buf,
        Err(err) => return err,
    };
    let length = HDMI_INFOFRAME_HEADER_SIZE + HDMI_AUDIO_INFOFRAME_SIZE as usize;
    if size < length
        || buf[0] != HDMI_INFOFRAME_TYPE_AUDIO as u8
        || buf[1] != 1
        || buf[2] != HDMI_AUDIO_INFOFRAME_SIZE
        || hdmi_infoframe_checksum(&buf[..length]) != 0
    {
        return -EINVAL;
    }
    let ret = unsafe { linux_hdmi_audio_infoframe_init(frame) };
    if ret != 0 {
        return ret;
    }
    let payload = &buf[HDMI_INFOFRAME_HEADER_SIZE..];
    unsafe {
        (*frame).channels = payload[0] & 0x7;
        (*frame).coding_type = ((payload[0] >> 4) & 0xf) as i32;
        (*frame).sample_size = (payload[1] & 0x3) as i32;
        (*frame).sample_frequency = ((payload[1] >> 2) & 0x7) as i32;
        (*frame).coding_type_ext = (payload[2] & 0x1f) as i32;
        (*frame).channel_allocation = payload[3];
        (*frame).level_shift_value = (payload[4] >> 3) & 0xf;
        (*frame).downmix_inhibit = u8::from(payload[4] & 0x80 != 0);
    }
    0
}

unsafe fn hdmi_vendor_any_infoframe_unpack(
    frame: *mut LinuxHdmiVendorAnyInfoframe,
    buffer: *const c_void,
    size: usize,
) -> i32 {
    let buf = match unsafe { bytes(buffer, size) } {
        Ok(buf) => buf,
        Err(err) => return err,
    };
    if size < HDMI_INFOFRAME_HEADER_SIZE
        || buf[0] != HDMI_INFOFRAME_TYPE_VENDOR as u8
        || buf[1] != 1
        || !matches!(buf[2], 4 | 5 | 6)
    {
        return -EINVAL;
    }
    let length = buf[2] as usize;
    let total = HDMI_INFOFRAME_HEADER_SIZE + length;
    if size < total || hdmi_infoframe_checksum(&buf[..total]) != 0 {
        return -EINVAL;
    }
    let payload = &buf[HDMI_INFOFRAME_HEADER_SIZE..];
    if payload[0] != 0x03 || payload[1] != 0x0c || payload[2] != 0x00 {
        return -EINVAL;
    }
    let video_format = payload[3] >> 5;
    if video_format > 0x2 {
        return -EINVAL;
    }
    let hvf = frame.cast::<LinuxHdmiVendorInfoframe>();
    let ret = unsafe { linux_hdmi_vendor_infoframe_init(hvf) };
    if ret != 0 {
        return ret;
    }
    unsafe {
        (*hvf).length = length as u8;
        if video_format == 0x2 {
            if length != 5 && length != 6 {
                return -EINVAL;
            }
            (*hvf).s3d_struct = (payload[4] >> 4) as i32;
            if (*hvf).s3d_struct >= HDMI_3D_STRUCTURE_SIDE_BY_SIDE_HALF {
                if length != 6 {
                    return -EINVAL;
                }
                (*hvf).s3d_ext_data = (payload[5] >> 4) as u32;
            }
        } else if video_format == 0x1 {
            if length != 5 {
                return -EINVAL;
            }
            (*hvf).vic = payload[4];
        } else if length != 4 {
            return -EINVAL;
        }
    }
    0
}

pub unsafe extern "C" fn linux_hdmi_drm_infoframe_unpack_only(
    frame: *mut LinuxHdmiDrmInfoframe,
    buffer: *const c_void,
    size: usize,
) -> i32 {
    let buf = match unsafe { bytes(buffer, size) } {
        Ok(buf) => buf,
        Err(err) => return err,
    };
    if size < HDMI_DRM_INFOFRAME_SIZE as usize {
        return -EINVAL;
    }
    let ret = unsafe { linux_hdmi_drm_infoframe_init(frame) };
    if ret != 0 {
        return ret;
    }
    unsafe {
        (*frame).eotf = (buf[0] & 0x7) as i32;
        (*frame).metadata_type = (buf[1] & 0x7) as i32;
        let mut index = 2usize;
        for primary in (*frame).display_primaries.iter_mut() {
            primary.x = ((buf[index + 1] as u16) << 8) | buf[index] as u16;
            primary.y = ((buf[index + 3] as u16) << 8) | buf[index + 2] as u16;
            index += 4;
        }
        (*frame).white_point.x = ((buf[15] as u16) << 8) | buf[14] as u16;
        (*frame).white_point.y = ((buf[17] as u16) << 8) | buf[16] as u16;
        (*frame).max_display_mastering_luminance = ((buf[19] as u16) << 8) | buf[18] as u16;
        (*frame).min_display_mastering_luminance = ((buf[21] as u16) << 8) | buf[20] as u16;
        (*frame).max_cll = ((buf[23] as u16) << 8) | buf[22] as u16;
        (*frame).max_fall = ((buf[25] as u16) << 8) | buf[24] as u16;
    }
    0
}

unsafe fn hdmi_drm_infoframe_unpack(
    frame: *mut LinuxHdmiDrmInfoframe,
    buffer: *const c_void,
    size: usize,
) -> i32 {
    let buf = match unsafe { bytes(buffer, size) } {
        Ok(buf) => buf,
        Err(err) => return err,
    };
    let length = HDMI_INFOFRAME_HEADER_SIZE + HDMI_DRM_INFOFRAME_SIZE as usize;
    if size < length
        || buf[0] != HDMI_INFOFRAME_TYPE_DRM as u8
        || buf[1] != 1
        || buf[2] != HDMI_DRM_INFOFRAME_SIZE
        || hdmi_infoframe_checksum(&buf[..length]) != 0
    {
        return -EINVAL;
    }
    unsafe {
        linux_hdmi_drm_infoframe_unpack_only(
            frame,
            buffer
                .cast::<u8>()
                .add(HDMI_INFOFRAME_HEADER_SIZE)
                .cast::<c_void>(),
            size - HDMI_INFOFRAME_HEADER_SIZE,
        )
    }
}

pub unsafe extern "C" fn linux_hdmi_infoframe_unpack(
    frame: *mut LinuxHdmiInfoframe,
    buffer: *const c_void,
    size: usize,
) -> i32 {
    let buf = match unsafe { bytes(buffer, size) } {
        Ok(buf) => buf,
        Err(err) => return err,
    };
    if size < HDMI_INFOFRAME_HEADER_SIZE {
        return -EINVAL;
    }
    match buf[0] as i32 {
        HDMI_INFOFRAME_TYPE_AVI => unsafe {
            hdmi_avi_infoframe_unpack(core::ptr::addr_of_mut!((*frame).avi), buffer, size)
        },
        HDMI_INFOFRAME_TYPE_DRM => unsafe {
            hdmi_drm_infoframe_unpack(core::ptr::addr_of_mut!((*frame).drm), buffer, size)
        },
        HDMI_INFOFRAME_TYPE_SPD => unsafe {
            hdmi_spd_infoframe_unpack(core::ptr::addr_of_mut!((*frame).spd), buffer, size)
        },
        HDMI_INFOFRAME_TYPE_AUDIO => unsafe {
            hdmi_audio_infoframe_unpack(core::ptr::addr_of_mut!((*frame).audio), buffer, size)
        },
        HDMI_INFOFRAME_TYPE_VENDOR => unsafe {
            hdmi_vendor_any_infoframe_unpack(core::ptr::addr_of_mut!((*frame).vendor), buffer, size)
        },
        _ => -EINVAL,
    }
}

pub unsafe extern "C" fn linux_hdmi_infoframe_log(
    _level: *const c_char,
    _dev: *mut c_void,
    _frame: *const LinuxHdmiInfoframe,
) {
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{offset_of, size_of};

    #[test]
    fn hdmi_layouts_match_vendor_x86_64_config() {
        assert_eq!(offset_of!(LinuxHdmiAnyInfoframe, type_), 0);
        assert_eq!(offset_of!(LinuxHdmiAnyInfoframe, version), 4);
        assert_eq!(offset_of!(LinuxHdmiAnyInfoframe, length), 5);
        assert_eq!(size_of::<LinuxHdmiAnyInfoframe>(), 8);

        assert_eq!(offset_of!(LinuxHdmiAviInfoframe, colorspace), 8);
        assert_eq!(offset_of!(LinuxHdmiAviInfoframe, video_code), 40);
        assert_eq!(
            offset_of!(LinuxHdmiAviInfoframe, ycc_quantization_range),
            44
        );
        assert_eq!(offset_of!(LinuxHdmiAviInfoframe, top_bar), 52);
        assert_eq!(size_of::<LinuxHdmiAviInfoframe>(), 60);

        assert_eq!(offset_of!(LinuxHdmiSpdInfoframe, vendor), 6);
        assert_eq!(offset_of!(LinuxHdmiSpdInfoframe, product), 14);
        assert_eq!(offset_of!(LinuxHdmiSpdInfoframe, sdi), 32);
        assert_eq!(size_of::<LinuxHdmiSpdInfoframe>(), 36);

        assert_eq!(offset_of!(LinuxHdmiAudioInfoframe, coding_type), 8);
        assert_eq!(offset_of!(LinuxHdmiAudioInfoframe, channel_allocation), 24);
        assert_eq!(size_of::<LinuxHdmiAudioInfoframe>(), 28);

        assert_eq!(offset_of!(LinuxHdmiVendorInfoframe, oui), 8);
        assert_eq!(offset_of!(LinuxHdmiVendorInfoframe, vic), 12);
        assert_eq!(offset_of!(LinuxHdmiVendorInfoframe, s3d_struct), 16);
        assert_eq!(size_of::<LinuxHdmiVendorInfoframe>(), 24);

        assert_eq!(offset_of!(LinuxHdmiDrmInfoframe, eotf), 8);
        assert_eq!(offset_of!(LinuxHdmiDrmInfoframe, display_primaries), 16);
        assert_eq!(offset_of!(LinuxHdmiDrmInfoframe, white_point), 28);
        assert_eq!(size_of::<LinuxHdmiDrmInfoframe>(), 40);
        assert_eq!(size_of::<LinuxHdmiInfoframe>(), 60);
    }

    #[test]
    fn vendor_infoframe_init_and_pack_match_linux_shape() {
        let mut frame = LinuxHdmiVendorInfoframe {
            type_: 0,
            version: 0,
            length: 0,
            _pad_length: [0; 2],
            oui: 0,
            vic: 0,
            _pad_vic: [0; 3],
            s3d_struct: 0,
            s3d_ext_data: 0,
        };
        assert_eq!(unsafe { linux_hdmi_vendor_infoframe_init(&mut frame) }, 0);
        assert_eq!(frame.type_, HDMI_INFOFRAME_TYPE_VENDOR);
        assert_eq!(frame.version, 1);
        assert_eq!(frame.length, HDMI_VENDOR_INFOFRAME_SIZE);
        assert_eq!(frame.oui, HDMI_IEEE_OUI);
        assert_eq!(frame.s3d_struct, HDMI_3D_STRUCTURE_INVALID);

        let mut out = [0xaa; 16];
        let written = unsafe {
            linux_hdmi_vendor_infoframe_pack(
                &mut frame,
                out.as_mut_ptr().cast::<c_void>(),
                out.len(),
            )
        };
        assert_eq!(written, 8);
        assert_eq!(&out[..3], &[HDMI_INFOFRAME_TYPE_VENDOR as u8, 1, 4]);
        assert_eq!(&out[4..8], &[0x03, 0x0c, 0x00, 0x00]);
        assert_eq!(hdmi_infoframe_checksum(&out[..written as usize]), 0);
        assert!(out[8..].iter().all(|byte| *byte == 0));
    }

    #[test]
    fn hdmi_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("hdmi_vendor_infoframe_init"),
            Some(linux_hdmi_vendor_infoframe_init as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("hdmi_infoframe_pack"),
            Some(linux_hdmi_infoframe_pack as usize)
        );
        assert!(crate::kernel::module::find_symbol("hdmi_drm_infoframe_unpack_only").is_some());
    }
}
