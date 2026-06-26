//! linux-parity: complete
//! linux-source: vendor/linux/lib/errname.c
//! test-origin: linux:vendor/linux/lib/errname.c
//! Linux errno name lookup tables.

use core::ffi::{c_char, c_int};
use core::ptr;

use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("errname", errname as usize, false);
}

fn negative_errname(abs_err: u32) -> Option<&'static [u8]> {
    match abs_err {
        1 => Some(b"-EPERM\0"),
        2 => Some(b"-ENOENT\0"),
        3 => Some(b"-ESRCH\0"),
        4 => Some(b"-EINTR\0"),
        5 => Some(b"-EIO\0"),
        6 => Some(b"-ENXIO\0"),
        7 => Some(b"-E2BIG\0"),
        8 => Some(b"-ENOEXEC\0"),
        9 => Some(b"-EBADF\0"),
        10 => Some(b"-ECHILD\0"),
        11 => Some(b"-EAGAIN\0"),
        12 => Some(b"-ENOMEM\0"),
        13 => Some(b"-EACCES\0"),
        14 => Some(b"-EFAULT\0"),
        15 => Some(b"-ENOTBLK\0"),
        16 => Some(b"-EBUSY\0"),
        17 => Some(b"-EEXIST\0"),
        18 => Some(b"-EXDEV\0"),
        19 => Some(b"-ENODEV\0"),
        20 => Some(b"-ENOTDIR\0"),
        21 => Some(b"-EISDIR\0"),
        22 => Some(b"-EINVAL\0"),
        23 => Some(b"-ENFILE\0"),
        24 => Some(b"-EMFILE\0"),
        25 => Some(b"-ENOTTY\0"),
        26 => Some(b"-ETXTBSY\0"),
        27 => Some(b"-EFBIG\0"),
        28 => Some(b"-ENOSPC\0"),
        29 => Some(b"-ESPIPE\0"),
        30 => Some(b"-EROFS\0"),
        31 => Some(b"-EMLINK\0"),
        32 => Some(b"-EPIPE\0"),
        33 => Some(b"-EDOM\0"),
        34 => Some(b"-ERANGE\0"),
        35 => Some(b"-EDEADLK\0"),
        36 => Some(b"-ENAMETOOLONG\0"),
        37 => Some(b"-ENOLCK\0"),
        38 => Some(b"-ENOSYS\0"),
        39 => Some(b"-ENOTEMPTY\0"),
        40 => Some(b"-ELOOP\0"),
        42 => Some(b"-ENOMSG\0"),
        43 => Some(b"-EIDRM\0"),
        44 => Some(b"-ECHRNG\0"),
        45 => Some(b"-EL2NSYNC\0"),
        46 => Some(b"-EL3HLT\0"),
        47 => Some(b"-EL3RST\0"),
        48 => Some(b"-ELNRNG\0"),
        49 => Some(b"-EUNATCH\0"),
        50 => Some(b"-ENOCSI\0"),
        51 => Some(b"-EL2HLT\0"),
        52 => Some(b"-EBADE\0"),
        53 => Some(b"-EBADR\0"),
        54 => Some(b"-EXFULL\0"),
        55 => Some(b"-ENOANO\0"),
        56 => Some(b"-EBADRQC\0"),
        57 => Some(b"-EBADSLT\0"),
        59 => Some(b"-EBFONT\0"),
        60 => Some(b"-ENOSTR\0"),
        61 => Some(b"-ENODATA\0"),
        62 => Some(b"-ETIME\0"),
        63 => Some(b"-ENOSR\0"),
        64 => Some(b"-ENONET\0"),
        65 => Some(b"-ENOPKG\0"),
        66 => Some(b"-EREMOTE\0"),
        67 => Some(b"-ENOLINK\0"),
        68 => Some(b"-EADV\0"),
        69 => Some(b"-ESRMNT\0"),
        70 => Some(b"-ECOMM\0"),
        71 => Some(b"-EPROTO\0"),
        72 => Some(b"-EMULTIHOP\0"),
        73 => Some(b"-EDOTDOT\0"),
        74 => Some(b"-EBADMSG\0"),
        75 => Some(b"-EOVERFLOW\0"),
        76 => Some(b"-ENOTUNIQ\0"),
        77 => Some(b"-EBADFD\0"),
        78 => Some(b"-EREMCHG\0"),
        79 => Some(b"-ELIBACC\0"),
        80 => Some(b"-ELIBBAD\0"),
        81 => Some(b"-ELIBSCN\0"),
        82 => Some(b"-ELIBMAX\0"),
        83 => Some(b"-ELIBEXEC\0"),
        84 => Some(b"-EILSEQ\0"),
        85 => Some(b"-ERESTART\0"),
        86 => Some(b"-ESTRPIPE\0"),
        87 => Some(b"-EUSERS\0"),
        88 => Some(b"-ENOTSOCK\0"),
        89 => Some(b"-EDESTADDRREQ\0"),
        90 => Some(b"-EMSGSIZE\0"),
        91 => Some(b"-EPROTOTYPE\0"),
        92 => Some(b"-ENOPROTOOPT\0"),
        93 => Some(b"-EPROTONOSUPPORT\0"),
        94 => Some(b"-ESOCKTNOSUPPORT\0"),
        95 => Some(b"-EOPNOTSUPP\0"),
        96 => Some(b"-EPFNOSUPPORT\0"),
        97 => Some(b"-EAFNOSUPPORT\0"),
        98 => Some(b"-EADDRINUSE\0"),
        99 => Some(b"-EADDRNOTAVAIL\0"),
        100 => Some(b"-ENETDOWN\0"),
        101 => Some(b"-ENETUNREACH\0"),
        102 => Some(b"-ENETRESET\0"),
        103 => Some(b"-ECONNABORTED\0"),
        104 => Some(b"-ECONNRESET\0"),
        105 => Some(b"-ENOBUFS\0"),
        106 => Some(b"-EISCONN\0"),
        107 => Some(b"-ENOTCONN\0"),
        108 => Some(b"-ESHUTDOWN\0"),
        109 => Some(b"-ETOOMANYREFS\0"),
        110 => Some(b"-ETIMEDOUT\0"),
        111 => Some(b"-ECONNREFUSED\0"),
        112 => Some(b"-EHOSTDOWN\0"),
        113 => Some(b"-EHOSTUNREACH\0"),
        114 => Some(b"-EALREADY\0"),
        115 => Some(b"-EINPROGRESS\0"),
        116 => Some(b"-ESTALE\0"),
        117 => Some(b"-EUCLEAN\0"),
        118 => Some(b"-ENOTNAM\0"),
        119 => Some(b"-ENAVAIL\0"),
        120 => Some(b"-EISNAM\0"),
        121 => Some(b"-EREMOTEIO\0"),
        122 => Some(b"-EDQUOT\0"),
        123 => Some(b"-ENOMEDIUM\0"),
        124 => Some(b"-EMEDIUMTYPE\0"),
        125 => Some(b"-ECANCELED\0"),
        126 => Some(b"-ENOKEY\0"),
        127 => Some(b"-EKEYEXPIRED\0"),
        128 => Some(b"-EKEYREVOKED\0"),
        129 => Some(b"-EKEYREJECTED\0"),
        130 => Some(b"-EOWNERDEAD\0"),
        131 => Some(b"-ENOTRECOVERABLE\0"),
        132 => Some(b"-ERFKILL\0"),
        133 => Some(b"-EHWPOISON\0"),
        512 => Some(b"-ERESTARTSYS\0"),
        513 => Some(b"-ERESTARTNOINTR\0"),
        514 => Some(b"-ERESTARTNOHAND\0"),
        515 => Some(b"-ENOIOCTLCMD\0"),
        516 => Some(b"-ERESTART_RESTARTBLOCK\0"),
        517 => Some(b"-EPROBE_DEFER\0"),
        518 => Some(b"-EOPENSTALE\0"),
        519 => Some(b"-ENOPARAM\0"),
        521 => Some(b"-EBADHANDLE\0"),
        522 => Some(b"-ENOTSYNC\0"),
        523 => Some(b"-EBADCOOKIE\0"),
        524 => Some(b"-ENOTSUPP\0"),
        525 => Some(b"-ETOOSMALL\0"),
        526 => Some(b"-ESERVERFAULT\0"),
        527 => Some(b"-EBADTYPE\0"),
        528 => Some(b"-EJUKEBOX\0"),
        529 => Some(b"-EIOCBQUEUED\0"),
        530 => Some(b"-ERECALLCONFLICT\0"),
        _ => None,
    }
}

pub fn errname_bytes(err: i32) -> Option<&'static [u8]> {
    let name = negative_errname(err.unsigned_abs())?;
    if err > 0 {
        Some(&name[1..])
    } else {
        Some(name)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn errname(err: c_int) -> *const c_char {
    match errname_bytes(err) {
        Some(name) => name.as_ptr().cast(),
        None => ptr::null(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::ffi::CStr;

    fn as_str(err: i32) -> Option<&'static str> {
        let ptr = errname(err);
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { CStr::from_ptr(ptr) }.to_str().unwrap())
        }
    }

    #[test]
    fn errname_source_matches_linux_tables_and_export() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/errname.c"
        ));
        assert!(source.contains("static const char *names_0[]"));
        assert!(source.contains("static const char *names_512[]"));
        assert!(source.contains("if (err < ARRAY_SIZE(names_0))"));
        assert!(source.contains("if (err >= 512 && err - 512 < ARRAY_SIZE(names_512))"));
        assert!(source.contains("return err > 0 ? name + 1 : name;"));
        assert!(source.contains("EXPORT_SYMBOL(errname);"));
    }

    #[test]
    fn errname_matches_positive_and_negative_linux_contract() {
        assert_eq!(as_str(5), Some("EIO"));
        assert_eq!(as_str(-5), Some("-EIO"));
        assert_eq!(as_str(95), Some("EOPNOTSUPP"));
        assert_eq!(as_str(-95), Some("-EOPNOTSUPP"));
        assert_eq!(as_str(122), Some("EDQUOT"));
        assert_eq!(as_str(-133), Some("-EHWPOISON"));
        assert_eq!(as_str(512), Some("ERESTARTSYS"));
        assert_eq!(as_str(-516), Some("-ERESTART_RESTARTBLOCK"));
        assert_eq!(as_str(524), Some("ENOTSUPP"));
        assert_eq!(as_str(-530), Some("-ERECALLCONFLICT"));
    }

    #[test]
    fn errname_preserves_sparse_table_holes() {
        assert_eq!(as_str(0), None);
        assert_eq!(as_str(41), None);
        assert_eq!(as_str(58), None);
        assert_eq!(as_str(134), None);
        assert_eq!(as_str(520), None);
        assert_eq!(as_str(531), None);
        assert_eq!(as_str(i32::MIN), None);
    }

    #[test]
    fn errname_export_registers_symbol() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("errname"),
            Some(errname as usize)
        );
    }
}
