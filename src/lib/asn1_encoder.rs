//! linux-parity: complete
//! linux-source: vendor/linux/lib/asn1_encoder.c
//! test-origin: linux:vendor/linux/lib/asn1_encoder.c
//! Simple ASN.1 BER/DER/CER encoder primitives.

use crate::include::uapi::errno::EINVAL;
use crate::kernel::module::{export_symbol, find_symbol};

const ASN1_UNIV: u8 = 0;
const ASN1_CONT: u8 = 2;
const ASN1_PRIM: u8 = 0;
const ASN1_CONS: u8 = 1;
const ASN1_BOOL: u8 = 1;
const ASN1_INT: u8 = 2;
const ASN1_OTS: u8 = 4;
const ASN1_OID: u8 = 6;
const ASN1_SEQ: u8 = 16;
const MAX_ERRNO: usize = 4095;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("asn1_encode_integer", asn1_encode_integer as usize, true);
    export_symbol_once("asn1_encode_oid", asn1_encode_oid as usize, true);
    export_symbol_once("asn1_encode_tag", asn1_encode_tag as usize, true);
    export_symbol_once(
        "asn1_encode_octet_string",
        asn1_encode_octet_string as usize,
        true,
    );
    export_symbol_once("asn1_encode_sequence", asn1_encode_sequence as usize, true);
    export_symbol_once("asn1_encode_boolean", asn1_encode_boolean as usize, true);
}

const fn tag(class: u8, cp: u8, nr: u8) -> u8 {
    (class << 6) | (cp << 5) | nr
}

fn err_ptr(errno: i32) -> *mut u8 {
    (-(errno as isize)) as *mut u8
}

fn is_err(ptr: *const u8) -> bool {
    let value = ptr as usize;
    value >= usize::MAX - MAX_ERRNO + 1
}

fn remaining(data: *const u8, end_data: *const u8) -> Option<usize> {
    if is_err(data) || data.is_null() || end_data.is_null() || data as usize > end_data as usize {
        None
    } else {
        Some(end_data as usize - data as usize)
    }
}

unsafe fn encode_length(data: &mut *mut u8, data_len: &mut usize, len: i32) -> i32 {
    if *data_len < 1 {
        return -EINVAL;
    }
    if len < 0 {
        unsafe { **data = 0 };
        *data = unsafe { (*data).add(1) };
        *data_len -= 1;
        return 0;
    }
    if len <= 0x7f {
        unsafe { **data = len as u8 };
        *data = unsafe { (*data).add(1) };
        *data_len -= 1;
        return 0;
    }
    if *data_len < 2 {
        return -EINVAL;
    }
    if len <= 0xff {
        unsafe {
            **data = 0x81;
            *(*data).add(1) = len as u8;
            *data = (*data).add(2);
        }
        *data_len -= 2;
        return 0;
    }
    if *data_len < 3 {
        return -EINVAL;
    }
    if len <= 0xffff {
        unsafe {
            **data = 0x82;
            *(*data).add(1) = (len >> 8) as u8;
            *(*data).add(2) = len as u8;
            *data = (*data).add(3);
        }
        *data_len -= 3;
        return 0;
    }
    if len > 0x00ff_ffff || *data_len < 4 {
        return -EINVAL;
    }
    unsafe {
        **data = 0x83;
        *(*data).add(1) = (len >> 16) as u8;
        *(*data).add(2) = (len >> 8) as u8;
        *(*data).add(3) = len as u8;
        *data = (*data).add(4);
    }
    *data_len -= 4;
    0
}

pub unsafe extern "C" fn asn1_encode_integer(
    data: *mut u8,
    end_data: *const u8,
    integer: i64,
) -> *mut u8 {
    if integer < 0 || is_err(data) {
        return if is_err(data) { data } else { err_ptr(EINVAL) };
    }
    let Some(mut data_len) = remaining(data, end_data) else {
        return err_ptr(EINVAL);
    };
    if data_len < 3 {
        return err_ptr(EINVAL);
    }

    unsafe { *data = tag(ASN1_UNIV, ASN1_PRIM, ASN1_INT) };
    let mut d = unsafe { data.add(2) };
    data_len -= 2;
    if integer == 0 {
        unsafe { *d = 0 };
        d = unsafe { d.add(1) };
    } else {
        let mut found = false;
        let integer = integer as u64;
        for i in (0..8).rev() {
            let byte = (integer >> (8 * i)) as u8;
            if !found && byte == 0 {
                continue;
            }
            if !found && (byte & 0x80) != 0 {
                unsafe { *d = 0 };
                d = unsafe { d.add(1) };
                data_len -= 1;
            }
            found = true;
            if data_len == 0 {
                return err_ptr(EINVAL);
            }
            unsafe { *d = byte };
            d = unsafe { d.add(1) };
            data_len -= 1;
        }
    }
    unsafe { *data.add(1) = (d as usize - data as usize - 2) as u8 };
    d
}

unsafe fn encode_oid_digit(data: &mut *mut u8, data_len: &mut usize, mut oid: u32) -> i32 {
    if *data_len < 1 {
        return -EINVAL;
    }
    if oid == 0 {
        unsafe { **data = 0x80 };
        *data = unsafe { (*data).add(1) };
        *data_len -= 1;
        return 0;
    }
    let mut start = 28;
    while start > 0 && oid >> start == 0 {
        start -= 7;
    }
    while start > 0 && *data_len > 0 {
        let mut byte = (oid >> start) as u8;
        oid -= (byte as u32) << start;
        start -= 7;
        byte |= 0x80;
        unsafe { **data = byte };
        *data = unsafe { (*data).add(1) };
        *data_len -= 1;
    }
    if *data_len > 0 {
        unsafe { **data = oid as u8 };
        *data = unsafe { (*data).add(1) };
        *data_len -= 1;
        0
    } else {
        -EINVAL
    }
}

pub unsafe extern "C" fn asn1_encode_oid(
    data: *mut u8,
    end_data: *const u8,
    oid: *const u32,
    oid_len: i32,
) -> *mut u8 {
    if oid_len < 2 || oid_len > 32 || oid.is_null() || is_err(data) {
        return if is_err(data) { data } else { err_ptr(EINVAL) };
    }
    let Some(mut data_len) = remaining(data, end_data) else {
        return err_ptr(EINVAL);
    };
    if data_len < 3 {
        return err_ptr(EINVAL);
    }
    let oid_slice = unsafe { core::slice::from_raw_parts(oid, oid_len as usize) };
    unsafe { *data = tag(ASN1_UNIV, ASN1_PRIM, ASN1_OID) };
    let mut d = unsafe { data.add(2) };
    unsafe { *d = (oid_slice[0] * 40 + oid_slice[1]) as u8 };
    d = unsafe { d.add(1) };
    data_len -= 3;
    for component in &oid_slice[2..] {
        let ret = unsafe { encode_oid_digit(&mut d, &mut data_len, *component) };
        if ret < 0 {
            return err_ptr(-ret);
        }
    }
    unsafe { *data.add(1) = (d as usize - data as usize - 2) as u8 };
    d
}

pub unsafe extern "C" fn asn1_encode_tag(
    mut data: *mut u8,
    end_data: *const u8,
    tag_nr: u32,
    string: *const u8,
    len: i32,
) -> *mut u8 {
    if tag_nr > 30 || (string.is_null() && len > 127) || is_err(data) {
        return if is_err(data) { data } else { err_ptr(EINVAL) };
    }
    if string.is_null() && len > 0 {
        data = unsafe { data.sub(2) };
    }
    let Some(mut data_len) = remaining(data, end_data) else {
        return err_ptr(EINVAL);
    };
    if string.is_null() && len > 0 {
        data_len = 2;
    }
    if data_len < 2 {
        return err_ptr(EINVAL);
    }
    unsafe { *data = tag(ASN1_CONT, ASN1_CONS, tag_nr as u8) };
    data = unsafe { data.add(1) };
    data_len -= 1;
    let ret = unsafe { encode_length(&mut data, &mut data_len, len) };
    if ret < 0 {
        return err_ptr(-ret);
    }
    if string.is_null() {
        return data;
    }
    if len < 0 || data_len < len as usize {
        return err_ptr(EINVAL);
    }
    unsafe { core::ptr::copy_nonoverlapping(string, data, len as usize) };
    unsafe { data.add(len as usize) }
}

pub unsafe extern "C" fn asn1_encode_octet_string(
    mut data: *mut u8,
    end_data: *const u8,
    string: *const u8,
    len: u32,
) -> *mut u8 {
    if string.is_null() || is_err(data) {
        return if is_err(data) { data } else { err_ptr(EINVAL) };
    }
    let Some(mut data_len) = remaining(data, end_data) else {
        return err_ptr(EINVAL);
    };
    if data_len < 2 {
        return err_ptr(EINVAL);
    }
    unsafe { *data = tag(ASN1_UNIV, ASN1_PRIM, ASN1_OTS) };
    data = unsafe { data.add(1) };
    data_len -= 1;
    let ret = unsafe { encode_length(&mut data, &mut data_len, len as i32) };
    if ret < 0 || data_len < len as usize {
        return err_ptr(EINVAL);
    }
    unsafe { core::ptr::copy_nonoverlapping(string, data, len as usize) };
    unsafe { data.add(len as usize) }
}

pub unsafe extern "C" fn asn1_encode_sequence(
    mut data: *mut u8,
    end_data: *const u8,
    seq: *const u8,
    len: i32,
) -> *mut u8 {
    if (seq.is_null() && len > 127) || is_err(data) {
        return if is_err(data) { data } else { err_ptr(EINVAL) };
    }
    if seq.is_null() && len >= 0 {
        data = unsafe { data.sub(2) };
    }
    let Some(mut data_len) = remaining(data, end_data) else {
        return err_ptr(EINVAL);
    };
    if seq.is_null() && len >= 0 {
        data_len = 2;
    }
    if data_len < 2 {
        return err_ptr(EINVAL);
    }
    unsafe { *data = tag(ASN1_UNIV, ASN1_CONS, ASN1_SEQ) };
    data = unsafe { data.add(1) };
    data_len -= 1;
    let ret = unsafe { encode_length(&mut data, &mut data_len, len) };
    if ret < 0 {
        return err_ptr(-ret);
    }
    if seq.is_null() {
        return data;
    }
    if len < 0 || data_len < len as usize {
        return err_ptr(EINVAL);
    }
    unsafe { core::ptr::copy_nonoverlapping(seq, data, len as usize) };
    unsafe { data.add(len as usize) }
}

pub unsafe extern "C" fn asn1_encode_boolean(
    mut data: *mut u8,
    end_data: *const u8,
    val: bool,
) -> *mut u8 {
    if is_err(data) {
        return data;
    }
    let Some(mut data_len) = remaining(data, end_data) else {
        return err_ptr(EINVAL);
    };
    if data_len < 3 {
        return err_ptr(EINVAL);
    }
    unsafe { *data = tag(ASN1_UNIV, ASN1_PRIM, ASN1_BOOL) };
    data = unsafe { data.add(1) };
    data_len -= 1;
    let _ = unsafe { encode_length(&mut data, &mut data_len, 1) };
    unsafe { *data = u8::from(val) };
    unsafe { data.add(1) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok_len(buf: &[u8], ptr: *mut u8) -> usize {
        ptr as usize - buf.as_ptr() as usize
    }

    #[test]
    fn linux_asn1_encoder_source_backed_integer_oid_and_strings() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/asn1_encoder.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/asn1_encoder.h"
        ));
        assert!(source.contains("asn1_encode_integer"));
        assert!(source.contains("asn1_encode_oid"));
        assert!(header.contains("asn1_encode_sequence"));

        let mut buf = [0u8; 64];
        let ptr = unsafe {
            asn1_encode_integer(
                buf.as_mut_ptr(),
                unsafe { buf.as_ptr().add(buf.len()) },
                0x80,
            )
        };
        assert_eq!(&buf[..ok_len(&buf, ptr)], &[0x02, 0x02, 0x00, 0x80]);

        let oid = [1u32, 2, 840, 113549];
        let ptr = unsafe {
            asn1_encode_oid(
                buf.as_mut_ptr(),
                buf.as_ptr().add(buf.len()),
                oid.as_ptr(),
                oid.len() as i32,
            )
        };
        assert_eq!(
            &buf[..ok_len(&buf, ptr)],
            &[0x06, 0x06, 42, 0x86, 0x48, 0x86, 0xf7, 0x0d]
        );

        let ptr = unsafe {
            asn1_encode_octet_string(
                buf.as_mut_ptr(),
                buf.as_ptr().add(buf.len()),
                b"abc".as_ptr(),
                3,
            )
        };
        assert_eq!(&buf[..ok_len(&buf, ptr)], &[0x04, 0x03, b'a', b'b', b'c']);
    }

    #[test]
    fn linux_asn1_encoder_tags_sequence_boolean_and_exports() {
        let mut buf = [0u8; 64];
        let ptr = unsafe {
            asn1_encode_tag(
                buf.as_mut_ptr(),
                buf.as_ptr().add(buf.len()),
                3,
                b"xy".as_ptr(),
                2,
            )
        };
        assert_eq!(&buf[..ok_len(&buf, ptr)], &[0xa3, 0x02, b'x', b'y']);

        let ptr = unsafe {
            asn1_encode_sequence(
                buf.as_mut_ptr(),
                buf.as_ptr().add(buf.len()),
                b"\x01\x01\x00".as_ptr(),
                3,
            )
        };
        assert_eq!(&buf[..ok_len(&buf, ptr)], &[0x30, 0x03, 0x01, 0x01, 0x00]);

        let ptr =
            unsafe { asn1_encode_boolean(buf.as_mut_ptr(), buf.as_ptr().add(buf.len()), true) };
        assert_eq!(&buf[..ok_len(&buf, ptr)], &[0x01, 0x01, 0x01]);

        let short = unsafe { asn1_encode_boolean(buf.as_mut_ptr(), buf.as_ptr().add(2), false) };
        assert!(is_err(short));

        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("asn1_encode_boolean"),
            Some(asn1_encode_boolean as usize)
        );
    }
}
