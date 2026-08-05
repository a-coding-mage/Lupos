// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: net/wireless/nl80211.c
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: x86_64,aarch64
//! rewrite-task: S009463

use core::ffi::c_int;

const NLA_HDRLEN: usize = 4;

#[repr(C)]
struct NlAttr {
    nla_len: u16,
    nla_type: u16,
}

#[repr(C)]
struct NetlinkExtAck {
    _msg: *const i8,
    bad_attr: *const NlAttr,
    policy: *const NlaPolicy,
    miss_nest: *const NlAttr,
    miss_type: u16,
    cookie: [u8; 8],
    cookie_len: u8,
    _msg_buf: [i8; 80],
}

#[repr(C)]
struct NlaPolicy {
    _private: [u8; 0],
}

unsafe extern "C" {
    fn do_trace_netlink_extack(message: *const i8);
}

unsafe fn set_err_msg_attr(
    extack: *mut NetlinkExtAck,
    attr: *const NlAttr,
    message: &'static [u8],
) {
    // SAFETY: The caller supplies the same extack and attribute pointers that
    // Linux's NL_SET_ERR_MSG_ATTR macro receives. The message is static and
    // NUL-terminated, matching Linux's static const diagnostic string.
    unsafe {
        do_trace_netlink_extack(message.as_ptr().cast::<i8>());
        if !extack.is_null() {
            (*extack)._msg = message.as_ptr().cast::<i8>();
            (*extack).bad_attr = attr;
            (*extack).policy = core::ptr::null();
        }
    }
}

enum ValidationError {
    TooShort(u32),
    InvalidAttribute(u8),
    LengthMismatch(u16, u32),
    UlwIncomplete(u32),
    UlwInvalidAttribute(u8),
    UlwInvalidLength(u16),
    UlwExceeds(u16, u32),
}

unsafe fn set_nan_availability_error(
    extack: *mut NetlinkExtAck,
    attr: *const NlAttr,
    error: ValidationError,
) {
    if extack.is_null() {
        return;
    }

    // SAFETY: The caller contract guarantees writable extack storage. The
    // helper writes no more than the 80-byte Linux _msg_buf, then installs its
    // address as _msg exactly as NL_SET_ERR_MSG_FMT does.
    unsafe {
        let buffer = (*extack)._msg_buf.as_mut_ptr();
        let mut pos = 0usize;

        fn put_bytes(buffer: *mut i8, pos: &mut usize, bytes: &[u8]) {
            for &byte in bytes {
                if *pos < 79 {
                    // SAFETY: The caller provides the fixed 80-byte extack
                    // message buffer and this bound leaves room for NUL.
                    unsafe { *buffer.add(*pos) = byte as i8 };
                    *pos += 1;
                }
            }
        }

        fn put_u32(buffer: *mut i8, pos: &mut usize, mut value: u32) {
            let mut digits = [0u8; 10];
            let mut count = 0usize;
            loop {
                digits[count] = (value % 10) as u8 + b'0';
                count += 1;
                value /= 10;
                if value == 0 {
                    break;
                }
            }
            for digit in digits[..count].iter().rev() {
                put_bytes(buffer, pos, core::slice::from_ref(digit));
            }
        }

        fn put_hex_byte(buffer: *mut i8, pos: &mut usize, value: u8) {
            const HEX: &[u8; 16] = b"0123456789abcdef";
            put_bytes(buffer, pos, &[HEX[(value >> 4) as usize], HEX[(value & 0xf) as usize]]);
        }

        match error {
            ValidationError::TooShort(length) => {
                put_bytes(buffer, &mut pos, b"NAN Availability: Too short (need at least 3 bytes, have ");
                put_u32(buffer, &mut pos, length);
                put_bytes(buffer, &mut pos, b")");
            }
            ValidationError::InvalidAttribute(value) => {
                put_bytes(buffer, &mut pos, b"NAN Availability: Invalid Attribute ID 0x");
                put_hex_byte(buffer, &mut pos, value);
                put_bytes(buffer, &mut pos, b" (expected 0x12)");
            }
            ValidationError::LengthMismatch(attribute_length, data_length) => {
                put_bytes(buffer, &mut pos, b"NAN Availability: Length field (");
                put_u32(buffer, &mut pos, attribute_length as u32);
                put_bytes(buffer, &mut pos, b") doesn't match data length (");
                put_u32(buffer, &mut pos, data_length);
                put_bytes(buffer, &mut pos, b")");
            }
            ValidationError::UlwIncomplete(length) => {
                put_bytes(buffer, &mut pos, b"ULW: Incomplete header (need 3 bytes, have ");
                put_u32(buffer, &mut pos, length);
                put_bytes(buffer, &mut pos, b")");
            }
            ValidationError::UlwInvalidAttribute(value) => {
                put_bytes(buffer, &mut pos, b"ULW: Invalid Attribute ID 0x");
                put_hex_byte(buffer, &mut pos, value);
                put_bytes(buffer, &mut pos, b" (expected 0x17)");
            }
            ValidationError::UlwInvalidLength(length) => {
                put_bytes(buffer, &mut pos, b"ULW: Invalid length ");
                put_u32(buffer, &mut pos, length as u32);
                put_bytes(buffer, &mut pos, b" (must be 16, 18, 21, or 23)");
            }
            ValidationError::UlwExceeds(length, remaining) => {
                put_bytes(buffer, &mut pos, b"ULW: Length field (");
                put_u32(buffer, &mut pos, length as u32);
                put_bytes(buffer, &mut pos, b") exceeds remaining data (");
                put_u32(buffer, &mut pos, remaining);
                put_bytes(buffer, &mut pos, b")");
            }
        }

        *buffer.add(pos) = 0;
        do_trace_netlink_extack(buffer);
        (*extack)._msg = buffer;
        (*extack).bad_attr = attr;
        (*extack).policy = core::ptr::null();
    }
}

/// Validates the supported-selector bitmap carried by an NL80211 attribute.
///
/// # Safety
///
/// `attr` must point to a valid Linux `struct nlattr` whose payload is readable
/// for the length encoded in its header. `_extack` is accepted for ABI parity
/// with Linux and may be null because this validator does not write it.
unsafe fn validate_supported_selectors(
    attr: *const NlAttr,
    _extack: *mut NetlinkExtAck,
) -> c_int {
    // SAFETY: The caller contract requires a valid nlattr. Linux's nla_data()
    // is the payload immediately following the fixed four-byte header.
    let supported_selectors = unsafe {
        (attr.cast::<u8>()).add(NLA_HDRLEN)
    };

    // Linux stores this length in u8 here, so preserve the intentional
    // truncation instead of widening it in the Rust translation.
    let supported_selectors_len = unsafe {
        (*attr).nla_len.wrapping_sub(NLA_HDRLEN as u16) as u8
    };

    // The top bit must not be set as it is not part of the selector.
    for i in 0..supported_selectors_len {
        // SAFETY: The caller contract and the encoded nla_len guarantee that
        // every byte in this range belongs to the attribute payload.
        let selector = unsafe { *supported_selectors.add(i as usize) };
        if selector & 0x80 != 0 {
            return -22; // -EINVAL
        }
    }

    0
}

/// Validates a sequence of 802.11 information elements.
///
/// # Safety
///
/// `attr` must point to a valid Linux `struct nlattr` whose payload is readable
/// for the length encoded in its header. `extack`, when non-null, must point to
/// a writable Linux `struct netlink_ext_ack`.
unsafe fn validate_ie_attr(attr: *const NlAttr, extack: *mut NetlinkExtAck) -> c_int {
    let data = unsafe { attr.cast::<u8>().add(NLA_HDRLEN) };
    let len = unsafe { (*attr).nla_len.wrapping_sub(NLA_HDRLEN as u16) as u32 };
    let mut pos = 0u32;

    // This is the pointer/length equivalent of Linux's for_each_element()
    // followed by for_each_element_completed().
    while len - pos >= 2 {
        let element_len = unsafe { *data.add((pos + 1) as usize) } as u32;
        if len - pos < 2 + element_len {
            break;
        }
        pos += 2 + element_len;
    }

    if pos == len {
        return 0;
    }

    const MALFORMED_ELEMENTS: &[u8] = b"malformed information elements\0";
    // SAFETY: `attr` and `extack` satisfy this function's caller contract.
    unsafe { set_err_msg_attr(extack, attr, MALFORMED_ELEMENTS) };
    -22 // -EINVAL
}

/// Validates the HE capabilities element carried by an NL80211 attribute.
///
/// # Safety
///
/// `attr` must point to a valid Linux `struct nlattr` whose payload is readable
/// for the length encoded in its header. `_extack` is unused by Linux here.
unsafe fn validate_he_capa(
    attr: *const NlAttr,
    _extack: *mut NetlinkExtAck,
) -> c_int {
    let data = unsafe { attr.cast::<u8>().add(NLA_HDRLEN) };
    let len = unsafe { (*attr).nla_len.wrapping_sub(NLA_HDRLEN as u16) as u8 };

    // SAFETY: `data` and `len` are the exact nla_data()/nla_len() values that
    // Linux passes to ieee80211_he_capa_size_ok().
    if !unsafe { ieee80211_he_capa_size_ok(data, len) } {
        return -22; // -EINVAL
    }

    0
}

unsafe fn ieee80211_uhr_oper_size_ok(data: *const u8, len: u8, beacon: bool) -> bool {
    const OPER_FIXED: usize = 6;
    const DPS_SIZE: usize = 4;
    const NPCA_FIXED: usize = 4;
    const PEDCA_SIZE: usize = 3;
    const DBE_FIXED: usize = 1;

    if (len as usize) < OPER_FIXED {
        return false;
    }

    // Nothing else is present in beacons.
    if beacon {
        return true;
    }

    let params = unsafe { u16::from_le_bytes([*data, *data.add(1)]) };
    let mut needed = OPER_FIXED;

    if params & 0x0001 != 0 {
        needed += DPS_SIZE;
        if (len as usize) < needed {
            return false;
        }
    }

    if params & 0x0002 != 0 {
        let npca = unsafe { data.add(needed) };
        needed += NPCA_FIXED;
        if (len as usize) < needed {
            return false;
        }

        let npca_params = unsafe {
            u32::from_le_bytes([*npca, *npca.add(1), *npca.add(2), *npca.add(3)])
        };
        if npca_params & 0x0080_0000 != 0 {
            needed += core::mem::size_of::<u16>();
            if (len as usize) < needed {
                return false;
            }
        }
    }

    if params & 0x0004 != 0 {
        needed += PEDCA_SIZE;
        if (len as usize) < needed {
            return false;
        }
    }

    if params & 0x0008 != 0 {
        let dbe = unsafe { *data.add(needed) };
        needed += DBE_FIXED;
        if (len as usize) < needed {
            return false;
        }
        if dbe & 0x08 != 0 {
            needed += core::mem::size_of::<u16>();
            if (len as usize) < needed {
                return false;
            }
        }
    }

    len as usize >= needed
}

/// Validates the UHR operation element carried by an NL80211 attribute.
///
/// # Safety
///
/// `attr` must point to a valid Linux `struct nlattr` whose payload is readable
/// for the length encoded in its header. `_extack` is unused by Linux here.
unsafe fn validate_uhr_operation(
    attr: *const NlAttr,
    _extack: *mut NetlinkExtAck,
) -> c_int {
    let data = unsafe { attr.cast::<u8>().add(NLA_HDRLEN) };
    let len = unsafe { (*attr).nla_len.wrapping_sub(NLA_HDRLEN as u16) as u8 };

    // SAFETY: The pointers and truncated u8 length are passed exactly as in
    // nl80211.c to the pinned inline helper.
    if !unsafe { ieee80211_uhr_oper_size_ok(data, len, false) } {
        return -22; // -EINVAL
    }

    0
}

unsafe fn ieee80211_uhr_capa_size_ok(data: *const u8, len: u8, from_ap: bool) -> bool {
    const UHR_CAP_FIXED: usize = 11; // mac_cap[6] + le32 phy cap + reserved

    if (len as usize) < UHR_CAP_FIXED {
        return false;
    }

    if from_ap && unsafe { *data.add(1) } & 0x04 != 0 {
        // struct ieee80211_uhr_cap_dbe::cap
        let mut needed = UHR_CAP_FIXED + 1;
        if (len as usize) < needed {
            return false;
        }

        let dbe_cap = unsafe { *data.add(UHR_CAP_FIXED) };
        if dbe_cap & 0x08 != 0 {
            needed += 2;
            if (len as usize) < needed {
                return false;
            }
        }
        if dbe_cap & 0x10 != 0 {
            needed += 2;
            if (len as usize) < needed {
                return false;
            }
        }
    }

    true
}

/// Validates the UHR capability element carried by an NL80211 attribute.
///
/// # Safety
///
/// `attr` must point to a valid Linux `struct nlattr` whose payload is readable
/// for the length encoded in its header. `_extack` is unused by Linux here.
unsafe fn validate_uhr_capa(
    attr: *const NlAttr,
    _extack: *mut NetlinkExtAck,
) -> c_int {
    let data = unsafe { attr.cast::<u8>().add(NLA_HDRLEN) };
    let len = unsafe { (*attr).nla_len.wrapping_sub(NLA_HDRLEN as u16) as u8 };

    // SAFETY: See validate_uhr_operation; this is the exact false `from_ap`
    // invocation used by nl80211.c.
    if !unsafe { ieee80211_uhr_capa_size_ok(data, len, false) } {
        return -22; // -EINVAL
    }

    0
}

/// Validates the six-byte NAN cluster identifier carried by an NL80211
/// attribute.
///
/// # Safety
///
/// `attr` must point to a valid Linux `struct nlattr` whose payload is readable
/// for the length encoded in its header. `extack`, when non-null, must point to
/// a writable Linux `struct netlink_ext_ack`.
unsafe fn validate_nan_cluster_id(
    attr: *const NlAttr,
    extack: *mut NetlinkExtAck,
) -> c_int {
    let data = unsafe { attr.cast::<u8>().add(NLA_HDRLEN) };
    let len = unsafe { (*attr).nla_len.wrapping_sub(NLA_HDRLEN as u16) as u32 };
    const CLUSTER_ID_PREFIX: [u8; 4] = [0x50, 0x6f, 0x9a, 0x01];
    const BAD_LENGTH: &[u8] = b"bad cluster id length\0";
    const BAD_PREFIX: &[u8] = b"invalid cluster id prefix\0";

    if len != 6 {
        // SAFETY: `attr` and `extack` satisfy this function's caller contract;
        // both are passed through unchanged as in NL_SET_ERR_MSG_ATTR.
        unsafe { set_err_msg_attr(extack, attr, BAD_LENGTH) };
        return -22; // -EINVAL
    }

    for (offset, expected) in CLUSTER_ID_PREFIX.iter().copied().enumerate() {
        // SAFETY: The validated six-byte payload contains the four-byte prefix.
        if unsafe { *data.add(offset) } != expected {
            // SAFETY: See the preceding error path.
            unsafe { set_err_msg_attr(extack, attr, BAD_PREFIX) };
            return -22; // -EINVAL
        }
    }

    0
}

#[repr(C, packed)]
struct Ieee80211HeCapElem {
    mac_cap_info: [u8; 6],
    phy_cap_info: [u8; 11],
}

fn ieee80211_he_mcs_nss_size(he_cap: &Ieee80211HeCapElem) -> u8 {
    let mut count = 4u8;
    if he_cap.phy_cap_info[0] & 0x08 != 0 {
        count = count.wrapping_add(4);
    }
    if he_cap.phy_cap_info[0] & 0x10 != 0 {
        count = count.wrapping_add(4);
    }
    count
}

fn ieee80211_he_ppe_size(ppe_thres_hdr: u8, phy_cap_info: &[u8; 11]) -> u8 {
    if phy_cap_info[6] & 0x80 == 0 {
        return 0;
    }

    let ru_count = (ppe_thres_hdr & 0x78).count_ones() as u8;
    let nss_count = 1u8.wrapping_add(ppe_thres_hdr & 0x07);
    let n = ru_count
        .wrapping_mul(nss_count)
        .wrapping_mul(3)
        .wrapping_mul(2)
        .wrapping_add(7);
    (n / 8).wrapping_add(u8::from(n % 8 != 0))
}

unsafe fn ieee80211_he_capa_size_ok(data: *const u8, len: u8) -> bool {
    let mut needed = core::mem::size_of::<Ieee80211HeCapElem>() as u8;
    if len < needed {
        return false;
    }

    // SAFETY: The initial length check proves the fixed HE capability element
    // is readable, and all later reads are guarded by the same cumulative
    // length checks as the Linux inline helper.
    let he_cap = unsafe { &*data.cast::<Ieee80211HeCapElem>() };
    needed = needed.wrapping_add(ieee80211_he_mcs_nss_size(he_cap));
    if len < needed {
        return false;
    }

    if he_cap.phy_cap_info[6] & 0x80 != 0 {
        if len < needed.wrapping_add(1) {
            return false;
        }
        let ppe_header = unsafe { *data.add(needed as usize) };
        needed = needed.wrapping_add(ieee80211_he_ppe_size(ppe_header, &he_cap.phy_cap_info));
    }

    len >= needed
}

/// Validates the NAN availability attribute blob.
///
/// # Safety
///
/// `attr` must point to a valid Linux `struct nlattr` whose payload is readable
/// for the length encoded in its header. `extack`, when non-null, must point to
/// a writable Linux `struct netlink_ext_ack`.
unsafe fn validate_nan_avail_blob(
    attr: *const NlAttr,
    extack: *mut NetlinkExtAck,
) -> c_int {
    let data = unsafe { attr.cast::<u8>().add(NLA_HDRLEN) };
    let len = unsafe { (*attr).nla_len.wrapping_sub(NLA_HDRLEN as u16) as u32 };

    // Need at least: Attr ID (1) + Length (2).
    if len < 3 {
        // SAFETY: The caller contract for extack is preserved by the helper.
        unsafe { set_nan_availability_error(extack, attr, ValidationError::TooShort(len)) };
        return -22; // -EINVAL
    }

    let attribute_id = unsafe { *data };
    if attribute_id != 0x12 {
        // SAFETY: See the preceding error path.
        unsafe {
            set_nan_availability_error(
                extack,
                attr,
                ValidationError::InvalidAttribute(attribute_id),
            )
        };
        return -22; // -EINVAL
    }

    let attr_len = unsafe { u16::from_le_bytes([*data.add(1), *data.add(2)]) };
    if attr_len as u32 != len - 3 {
        // SAFETY: See the preceding error path.
        unsafe {
            set_nan_availability_error(
                extack,
                attr,
                ValidationError::LengthMismatch(attr_len, len - 3),
            )
        };
        return -22; // -EINVAL
    }

    0
}

/// Validates the NAN Unsolicited Link Watch attribute blob.
///
/// # Safety
///
/// `attr` must point to a valid Linux `struct nlattr` whose payload is readable
/// for the length encoded in its header. `extack`, when non-null, must point to
/// a writable Linux `struct netlink_ext_ack`.
unsafe fn validate_nan_ulw(attr: *const NlAttr, extack: *mut NetlinkExtAck) -> c_int {
    let data = unsafe { attr.cast::<u8>().add(NLA_HDRLEN) };
    let len = unsafe { (*attr).nla_len.wrapping_sub(NLA_HDRLEN as u16) as u32 };
    let mut pos = 0u32;

    while pos < len {
        if pos + 3 > len {
            // SAFETY: The caller contract for extack is preserved by the helper.
            unsafe {
                set_nan_availability_error(
                    extack,
                    attr,
                    ValidationError::UlwIncomplete(len - pos),
                )
            };
            return -22; // -EINVAL
        }

        let attribute_id = unsafe { *data.add(pos as usize) };
        if attribute_id != 0x17 {
            // SAFETY: See the preceding error path.
            unsafe {
                set_nan_availability_error(
                    extack,
                    attr,
                    ValidationError::UlwInvalidAttribute(attribute_id),
                )
            };
            return -22; // -EINVAL
        }
        pos += 1;

        // The length is encoded little-endian and may be unaligned.
        let attr_len = unsafe {
            u16::from_le_bytes([
                *data.add(pos as usize),
                *data.add((pos + 1) as usize),
            ])
        };
        pos += 2;

        if attr_len != 16 && attr_len != 18 && attr_len != 21 && attr_len != 23 {
            // SAFETY: See the preceding error path.
            unsafe {
                set_nan_availability_error(
                    extack,
                    attr,
                    ValidationError::UlwInvalidLength(attr_len),
                )
            };
            return -22; // -EINVAL
        }

        if pos + attr_len as u32 > len {
            // SAFETY: See the preceding error path.
            unsafe {
                set_nan_availability_error(
                    extack,
                    attr,
                    ValidationError::UlwExceeds(attr_len, len - pos),
                )
            };
            return -22; // -EINVAL
        }

        pos += attr_len as u32;
    }

    0
}
