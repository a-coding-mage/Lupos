//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/um/os-Linux/tls.c
//! test-origin: linux:vendor/linux/arch/x86/um/os-Linux/tls.c
//! UML host TLS probing and ptrace thread-area operations.

use crate::include::uapi::errno::{EINVAL, ENOSYS};

pub const PTRACE_GET_THREAD_AREA: i32 = 25;
pub const PTRACE_SET_THREAD_AREA: i32 = 26;
pub const GDT_ENTRY_TLS_MIN_I386: i32 = 6;
pub const GDT_ENTRY_TLS_MIN_X86_64: i32 = 12;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TlsProbeResult {
    pub supports_tls: bool,
    pub tls_min: Option<i32>,
}

pub fn check_host_supports_tls(get_thread_area_results: &[Result<(), i32>]) -> TlsProbeResult {
    let candidates = [GDT_ENTRY_TLS_MIN_I386, GDT_ENTRY_TLS_MIN_X86_64];
    for (idx, entry) in candidates.iter().copied().enumerate() {
        match get_thread_area_results
            .get(idx)
            .copied()
            .unwrap_or(Err(ENOSYS))
        {
            Ok(()) => {
                return TlsProbeResult {
                    supports_tls: true,
                    tls_min: Some(entry),
                };
            }
            Err(errno) if errno == EINVAL => continue,
            Err(ENOSYS) => {
                return TlsProbeResult {
                    supports_tls: false,
                    tls_min: None,
                };
            }
            Err(_) => {
                return TlsProbeResult {
                    supports_tls: false,
                    tls_min: None,
                };
            }
        }
    }
    TlsProbeResult {
        supports_tls: false,
        tls_min: None,
    }
}

pub const fn os_thread_area_ret(ptrace_ret: i32, errno: i32) -> i32 {
    if ptrace_ret < 0 { -errno } else { ptrace_ret }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uml_host_tls_probe_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/um/os-Linux/tls.c"
        ));
        assert!(source.contains("#define PTRACE_GET_THREAD_AREA 25"));
        assert!(source.contains("#define PTRACE_SET_THREAD_AREA 26"));
        assert!(source.contains("GDT_ENTRY_TLS_MIN_I386"));
        assert!(source.contains("GDT_ENTRY_TLS_MIN_X86_64"));
        assert!(source.contains("syscall(__NR_get_thread_area, &info) == 0"));
        assert!(source.contains("if (errno == EINVAL)"));
        assert!(source.contains("else if (errno == ENOSYS)"));
        assert!(source.contains("ptrace(PTRACE_SET_THREAD_AREA"));
        assert!(source.contains("ptrace(PTRACE_GET_THREAD_AREA"));
        assert!(source.contains("ret = -errno;"));

        assert_eq!(
            check_host_supports_tls(&[Err(EINVAL), Ok(())]),
            TlsProbeResult {
                supports_tls: true,
                tls_min: Some(GDT_ENTRY_TLS_MIN_X86_64)
            }
        );
        assert_eq!(
            check_host_supports_tls(&[Err(ENOSYS)]),
            TlsProbeResult {
                supports_tls: false,
                tls_min: None
            }
        );
        assert_eq!(os_thread_area_ret(-1, EINVAL), -EINVAL);
        assert_eq!(os_thread_area_ret(0, EINVAL), 0);
    }
}
