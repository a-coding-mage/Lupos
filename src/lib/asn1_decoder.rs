//! linux-parity: complete
//! linux-source: vendor/linux/lib/asn1_decoder.c
//! test-origin: linux:vendor/linux/lib/asn1_decoder.c
//! ASN.1 BER/DER/CER bytecode decoder.

use core::ffi::c_void;

use crate::include::uapi::errno::EBADMSG;
use crate::kernel::module::{export_symbol, find_symbol};

pub type Asn1Action = unsafe extern "C" fn(
    context: *mut c_void,
    hdrlen: usize,
    tag: u8,
    value: *const c_void,
    vlen: usize,
) -> i32;

#[repr(C)]
pub struct Asn1Decoder {
    pub machine: *const u8,
    pub machlen: usize,
    pub actions: *const Option<Asn1Action>,
}

const ASN1_EOC: u8 = 0;
const ASN1_CONS_BIT: u8 = 0x20;
const ASN1_LONG_TAG: u8 = 31;
const ASN1_INDEFINITE_LENGTH: usize = 0x80;

const ASN1_OP_MATCH__SKIP: u8 = 0x01;
const ASN1_OP_MATCH__ACT: u8 = 0x02;
const ASN1_OP_MATCH__JUMP: u8 = 0x04;
const ASN1_OP_MATCH__ANY: u8 = 0x08;
const ASN1_OP_MATCH__COND: u8 = 0x10;
const ASN1_OP__MATCHES_TAG: u8 = 0x1b;

const ASN1_OP_MATCH_JUMP: u8 = 0x04;
const ASN1_OP_MATCH_JUMP_OR_SKIP: u8 = 0x05;
const ASN1_OP_COND_MATCH_JUMP_OR_SKIP: u8 = 0x15;
const ASN1_OP_COND_FAIL: u8 = 0x1c;
const ASN1_OP_COMPLETE: u8 = 0x1d;
const ASN1_OP_ACT: u8 = 0x1e;
const ASN1_OP_MAYBE_ACT: u8 = 0x1f;
const ASN1_OP_END_SEQ: u8 = 0x20;
const ASN1_OP_END_SET: u8 = 0x21;
const ASN1_OP_END_SEQ_OF: u8 = 0x22;
const ASN1_OP_END_SET_OF: u8 = 0x23;
const ASN1_OP_END_SEQ_ACT: u8 = 0x24;
const ASN1_OP_END_SET_ACT: u8 = 0x25;
const ASN1_OP_END_SEQ_OF_ACT: u8 = 0x26;
const ASN1_OP_END_SET_OF_ACT: u8 = 0x27;
const ASN1_OP_END__OF: u8 = 0x02;
const ASN1_OP_END__ACT: u8 = 0x04;
const ASN1_OP_RETURN: u8 = 0x28;

const FLAG_INDEFINITE_LENGTH: u8 = 0x01;
const FLAG_MATCHED: u8 = 0x02;
const FLAG_LAST_MATCHED: u8 = 0x04;
const FLAG_CONS: u8 = 0x20;

const NR_CONS_STACK: usize = 10;
const NR_JUMP_STACK: usize = 10;
const EMSGSIZE: i32 = 90;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("asn1_ber_decoder", asn1_ber_decoder as usize, true);
}

fn op_len(op: u8) -> usize {
    match op {
        0x00 | 0x01 | 0x11 => 2,
        0x02 | 0x03 | 0x13 => 3,
        0x04 | 0x05 | 0x15 => 3,
        0x08 | 0x09 | 0x18 | 0x19 => 1,
        0x0a | 0x0b | 0x1a | 0x1b => 2,
        0x1c | 0x1d | 0x28 => 1,
        0x1e | 0x1f => 2,
        0x20 | 0x21 => 1,
        0x22 | 0x23 => 2,
        0x24 | 0x25 => 2,
        0x26 | 0x27 => 3,
        _ => 0,
    }
}

fn find_indefinite_length(
    data: &[u8],
    datalen: usize,
    dp_in: &mut usize,
    len_out: &mut usize,
) -> i32 {
    let mut dp = *dp_in;
    let mut indef_level = 1i32;
    loop {
        if datalen.saturating_sub(dp) < 2 {
            *dp_in = dp;
            return -1;
        }
        let tag = data[dp];
        dp += 1;
        if tag == ASN1_EOC {
            if data[dp] != 0 {
                *dp_in = dp + 1;
                return -1;
            }
            dp += 1;
            indef_level -= 1;
            if indef_level <= 0 {
                *len_out = dp - *dp_in;
                *dp_in = dp;
                return 0;
            }
            continue;
        }
        if (tag & 0x1f) == ASN1_LONG_TAG {
            loop {
                if datalen.saturating_sub(dp) < 2 {
                    *dp_in = dp;
                    return -1;
                }
                let tmp = data[dp];
                dp += 1;
                if (tmp & 0x80) == 0 {
                    break;
                }
            }
        }
        let mut len = data[dp] as usize;
        dp += 1;
        if len > 0x7f {
            if len == ASN1_INDEFINITE_LENGTH {
                if (tag & ASN1_CONS_BIT) == 0 {
                    *dp_in = dp;
                    return -1;
                }
                indef_level += 1;
                continue;
            }
            let mut n = len - 0x80;
            if n > core::mem::size_of::<usize>() - 1 || n > datalen.saturating_sub(dp) {
                *dp_in = dp;
                return -1;
            }
            len = 0;
            while n > 0 {
                len = (len << 8) | data[dp] as usize;
                dp += 1;
                n -= 1;
            }
        }
        if len > datalen.saturating_sub(dp) {
            *dp_in = dp;
            return -1;
        }
        dp += len;
    }
}

unsafe fn call_action(
    decoder: &Asn1Decoder,
    index: u8,
    context: *mut c_void,
    hdr: usize,
    tag: u8,
    value: *const u8,
    len: usize,
) -> i32 {
    if decoder.actions.is_null() {
        return -EBADMSG;
    }
    let action = unsafe { *decoder.actions.add(index as usize) };
    let Some(action) = action else {
        return -EBADMSG;
    };
    unsafe { action(context, hdr, tag, value.cast(), len) }
}

pub unsafe extern "C" fn asn1_ber_decoder(
    decoder: *const Asn1Decoder,
    context: *mut c_void,
    data: *const u8,
    datalen: usize,
) -> i32 {
    if decoder.is_null() || data.is_null() || datalen > 65535 {
        return if datalen > 65535 { -EMSGSIZE } else { -EBADMSG };
    }
    let decoder = unsafe { &*decoder };
    if decoder.machine.is_null() {
        return -EBADMSG;
    }
    let machine = unsafe { core::slice::from_raw_parts(decoder.machine, decoder.machlen) };
    let data = unsafe { core::slice::from_raw_parts(data, datalen) };

    let mut tag = 0u8;
    let mut csp = 0usize;
    let mut jsp = 0usize;
    let mut optag = 0u8;
    let mut hdr = 0usize;
    let mut pc = 0usize;
    let mut dp = 0usize;
    let mut tdp = 0usize;
    let mut len = 0usize;
    let mut flags = 0u8;
    let mut current_datalen = datalen;
    let mut cons_dp_stack = [0usize; NR_CONS_STACK];
    let mut cons_datalen_stack = [0usize; NR_CONS_STACK];
    let mut cons_hdrlen_stack = [0usize; NR_CONS_STACK];
    let mut jump_stack = [0usize; NR_JUMP_STACK];

    loop {
        if pc >= machine.len() {
            return -EBADMSG;
        }
        let op = machine[pc];
        let oplen = op_len(op);
        if oplen == 0 || pc + oplen > machine.len() {
            return -EBADMSG;
        }

        if op <= ASN1_OP__MATCHES_TAG {
            if ((op & ASN1_OP_MATCH__COND) != 0 && (flags & FLAG_MATCHED) != 0)
                || ((op & ASN1_OP_MATCH__SKIP) != 0 && dp == current_datalen)
            {
                flags &= !FLAG_LAST_MATCHED;
                pc += oplen;
                continue;
            }

            flags = 0;
            hdr = 2;
            if current_datalen.saturating_sub(dp) < 2 {
                return -EBADMSG;
            }
            tag = data[dp];
            dp += 1;
            if (tag & 0x1f) == ASN1_LONG_TAG {
                return -EBADMSG;
            }

            if (op & ASN1_OP_MATCH__ANY) == 0 {
                optag = machine[pc + 1];
                flags |= optag & FLAG_CONS;
                let mut tmp = optag ^ tag;
                tmp &= !(optag & ASN1_CONS_BIT);
                if tmp != 0 {
                    if (op & ASN1_OP_MATCH__SKIP) != 0 {
                        pc += oplen;
                        dp -= 1;
                        continue;
                    }
                    return -EBADMSG;
                }
            }
            flags |= FLAG_MATCHED;

            len = data[dp] as usize;
            dp += 1;
            if len > 0x7f {
                if len == ASN1_INDEFINITE_LENGTH {
                    if (tag & ASN1_CONS_BIT) == 0 {
                        return -EBADMSG;
                    }
                    flags |= FLAG_INDEFINITE_LENGTH;
                    if 2 > current_datalen.saturating_sub(dp) {
                        return -EBADMSG;
                    }
                } else {
                    let mut n = len - 0x80;
                    if n > 2 || n > current_datalen.saturating_sub(dp) {
                        return -EBADMSG;
                    }
                    hdr += n;
                    len = 0;
                    while n > 0 {
                        len = (len << 8) | data[dp] as usize;
                        dp += 1;
                        n -= 1;
                    }
                    if len > current_datalen.saturating_sub(dp) {
                        return -EBADMSG;
                    }
                }
            } else if len > current_datalen.saturating_sub(dp) {
                return -EBADMSG;
            }

            if (flags & FLAG_CONS) != 0 {
                if csp >= NR_CONS_STACK {
                    return -EBADMSG;
                }
                cons_dp_stack[csp] = dp;
                cons_hdrlen_stack[csp] = hdr;
                if (flags & FLAG_INDEFINITE_LENGTH) == 0 {
                    cons_datalen_stack[csp] = current_datalen;
                    current_datalen = dp + len;
                } else {
                    cons_datalen_stack[csp] = 0;
                }
                csp += 1;
            }
            tdp = dp;
        }

        match op {
            0x00 | 0x01 | 0x02 | 0x03 | 0x08 | 0x09 | 0x0a | 0x0b | 0x11 | 0x13 | 0x18 | 0x19
            | 0x1a | 0x1b => {
                if (flags & FLAG_CONS) == 0 && (flags & FLAG_INDEFINITE_LENGTH) != 0 {
                    let mut tmp = dp;
                    if find_indefinite_length(data, current_datalen, &mut tmp, &mut len) < 0 {
                        return -EBADMSG;
                    }
                }
                if (op & ASN1_OP_MATCH__ACT) != 0 {
                    let act = if (op & ASN1_OP_MATCH__ANY) != 0 {
                        machine[pc + 1]
                    } else {
                        machine[pc + 2]
                    };
                    let ret = unsafe {
                        call_action(decoder, act, context, hdr, tag, data.as_ptr().add(dp), len)
                    };
                    if ret < 0 {
                        return ret;
                    }
                }
                if (flags & FLAG_CONS) == 0 {
                    dp += len;
                }
                pc += oplen;
            }
            ASN1_OP_MATCH_JUMP | ASN1_OP_MATCH_JUMP_OR_SKIP | ASN1_OP_COND_MATCH_JUMP_OR_SKIP => {
                if jsp == NR_JUMP_STACK {
                    return -EBADMSG;
                }
                jump_stack[jsp] = pc + oplen;
                jsp += 1;
                pc = machine[pc + 2] as usize;
            }
            ASN1_OP_COND_FAIL => {
                if (flags & FLAG_MATCHED) == 0 {
                    return -EBADMSG;
                }
                pc += oplen;
            }
            ASN1_OP_COMPLETE => {
                if jsp != 0 || csp != 0 {
                    return -EBADMSG;
                }
                return 0;
            }
            ASN1_OP_END_SET | ASN1_OP_END_SET_ACT => {
                if (flags & FLAG_MATCHED) == 0 {
                    return -EBADMSG;
                }
                let ret = unsafe {
                    end_constructed(
                        op,
                        oplen,
                        decoder,
                        context,
                        machine,
                        data,
                        &mut pc,
                        &mut dp,
                        &mut tdp,
                        &mut hdr,
                        &mut len,
                        &mut current_datalen,
                        &mut csp,
                        &mut cons_dp_stack,
                        &mut cons_datalen_stack,
                        &mut cons_hdrlen_stack,
                    )
                };
                if ret < 0 {
                    return ret;
                }
            }
            ASN1_OP_END_SEQ
            | ASN1_OP_END_SEQ_OF
            | ASN1_OP_END_SET_OF
            | ASN1_OP_END_SEQ_ACT
            | ASN1_OP_END_SEQ_OF_ACT
            | ASN1_OP_END_SET_OF_ACT => {
                let ret = unsafe {
                    end_constructed(
                        op,
                        oplen,
                        decoder,
                        context,
                        machine,
                        data,
                        &mut pc,
                        &mut dp,
                        &mut tdp,
                        &mut hdr,
                        &mut len,
                        &mut current_datalen,
                        &mut csp,
                        &mut cons_dp_stack,
                        &mut cons_datalen_stack,
                        &mut cons_hdrlen_stack,
                    )
                };
                if ret < 0 {
                    return ret;
                }
            }
            ASN1_OP_MAYBE_ACT => {
                if (flags & FLAG_LAST_MATCHED) == 0 {
                    pc += oplen;
                    continue;
                }
                let ret = unsafe {
                    call_action(
                        decoder,
                        machine[pc + 1],
                        context,
                        hdr,
                        tag,
                        data.as_ptr().add(tdp),
                        len,
                    )
                };
                if ret < 0 {
                    return ret;
                }
                pc += oplen;
            }
            ASN1_OP_ACT => {
                let ret = unsafe {
                    call_action(
                        decoder,
                        machine[pc + 1],
                        context,
                        hdr,
                        tag,
                        data.as_ptr().add(tdp),
                        len,
                    )
                };
                if ret < 0 {
                    return ret;
                }
                pc += oplen;
            }
            ASN1_OP_RETURN => {
                if jsp == 0 {
                    return -EBADMSG;
                }
                jsp -= 1;
                pc = jump_stack[jsp];
                flags |= FLAG_MATCHED | FLAG_LAST_MATCHED;
            }
            _ => return -EBADMSG,
        }
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn end_constructed(
    op: u8,
    oplen: usize,
    decoder: &Asn1Decoder,
    context: *mut c_void,
    machine: &[u8],
    data: &[u8],
    pc: &mut usize,
    dp: &mut usize,
    tdp: &mut usize,
    hdr: &mut usize,
    len: &mut usize,
    current_datalen: &mut usize,
    csp: &mut usize,
    cons_dp_stack: &mut [usize; NR_CONS_STACK],
    cons_datalen_stack: &mut [usize; NR_CONS_STACK],
    cons_hdrlen_stack: &mut [usize; NR_CONS_STACK],
) -> i32 {
    if *csp == 0 {
        return -EBADMSG;
    }
    *csp -= 1;
    *tdp = cons_dp_stack[*csp];
    *hdr = cons_hdrlen_stack[*csp];
    *len = *current_datalen;
    *current_datalen = cons_datalen_stack[*csp];
    if *current_datalen == 0 {
        *current_datalen = *len;
        if (*current_datalen).saturating_sub(*dp) < 2 {
            return -EBADMSG;
        }
        if data[*dp] != 0 {
            if (op & ASN1_OP_END__OF) != 0 {
                *csp += 1;
                *pc = machine[*pc + 1] as usize;
                return 0;
            }
            return -EBADMSG;
        }
        *dp += 1;
        if data[*dp] != 0 {
            return -EBADMSG;
        }
        *dp += 1;
        *len = *dp - *tdp - 2;
    } else {
        if *dp < *len && (op & ASN1_OP_END__OF) != 0 {
            *current_datalen = *len;
            *csp += 1;
            *pc = machine[*pc + 1] as usize;
            return 0;
        }
        if *dp != *len {
            return -EBADMSG;
        }
        *len -= *tdp;
    }

    if (op & ASN1_OP_END__ACT) != 0 {
        let act = if (op & ASN1_OP_END__OF) != 0 {
            machine[*pc + 2]
        } else {
            machine[*pc + 1]
        };
        let ret = unsafe {
            call_action(
                decoder,
                act,
                context,
                *hdr,
                0,
                data.as_ptr().add(*tdp),
                *len,
            )
        };
        if ret < 0 {
            return ret;
        }
    }
    *pc += oplen;
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    const TAG_INT: u8 = 0x02;
    const TAG_SEQ: u8 = 0x30;

    unsafe extern "C" fn capture(
        context: *mut c_void,
        hdrlen: usize,
        tag: u8,
        value: *const c_void,
        vlen: usize,
    ) -> i32 {
        let out = unsafe { &mut *(context as *mut (usize, u8, [u8; 8], usize)) };
        out.0 = hdrlen;
        out.1 = tag;
        out.3 = vlen;
        let bytes = unsafe { core::slice::from_raw_parts(value as *const u8, vlen) };
        out.2[..vlen].copy_from_slice(bytes);
        0
    }

    #[test]
    fn linux_asn1_decoder_source_backed_leaf_action() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/asn1_decoder.c"
        ));
        let bytecode = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/asn1_ber_bytecode.h"
        ));
        assert!(source.contains("asn1_ber_decoder"));
        assert!(source.contains("NR_CONS_STACK 10"));
        assert!(bytecode.contains("ASN1_OP_MATCH_ACT"));

        let machine = [0x02, TAG_INT, 0, ASN1_OP_COMPLETE];
        let actions = [Some(capture as Asn1Action)];
        let decoder = Asn1Decoder {
            machine: machine.as_ptr(),
            machlen: machine.len(),
            actions: actions.as_ptr(),
        };
        let data = [TAG_INT, 0x01, 0x7f];
        let mut captured = (0usize, 0u8, [0u8; 8], 0usize);
        let ret = unsafe {
            asn1_ber_decoder(
                &decoder,
                &mut captured as *mut _ as *mut c_void,
                data.as_ptr(),
                data.len(),
            )
        };
        assert_eq!(ret, 0);
        assert_eq!(captured.0, 2);
        assert_eq!(captured.1, TAG_INT);
        assert_eq!(captured.2[0], 0x7f);
        assert_eq!(captured.3, 1);
    }

    #[test]
    fn linux_asn1_decoder_constructed_sequence_and_errors() {
        let machine = [
            0x00,
            TAG_SEQ,
            0x02,
            TAG_INT,
            0,
            ASN1_OP_END_SEQ,
            ASN1_OP_COMPLETE,
        ];
        let actions = [Some(capture as Asn1Action)];
        let decoder = Asn1Decoder {
            machine: machine.as_ptr(),
            machlen: machine.len(),
            actions: actions.as_ptr(),
        };
        let data = [TAG_SEQ, 0x03, TAG_INT, 0x01, 0x05];
        let mut captured = (0usize, 0u8, [0u8; 8], 0usize);
        let ret = unsafe {
            asn1_ber_decoder(
                &decoder,
                &mut captured as *mut _ as *mut c_void,
                data.as_ptr(),
                data.len(),
            )
        };
        assert_eq!(ret, 0);
        assert_eq!(captured.2[0], 0x05);

        let bad = [TAG_SEQ, 0x04, TAG_INT, 0x01, 0x05];
        let ret = unsafe {
            asn1_ber_decoder(
                &decoder,
                &mut captured as *mut _ as *mut c_void,
                bad.as_ptr(),
                bad.len(),
            )
        };
        assert_eq!(ret, -EBADMSG);

        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("asn1_ber_decoder"),
            Some(asn1_ber_decoder as usize)
        );
    }
}
