//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/entry/vdso/vdso32-setup.c
//! test-origin: linux:vendor/linux/arch/x86/entry/vdso/vdso32-setup.c
//! 32-bit vDSO setup policy: the `vdso32=` boot parameter and the
//! `abi.vsyscall32` sysctl that gate whether the kernel maps a 32-bit vDSO
//! page into processes (and passes its address to glibc on `exec`).
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/entry/vdso/vdso32-setup.c
//!
//! The `vdso32=` boot parameter uses `simple_strtoul` base-0 semantics. The
//! `abi.vsyscall32` sysctl is exposed at `/proc/sys/abi/vsyscall32`, wired to
//! `vdso32_enabled` via `crate::fs::proc::proc_sysctl`: lupos builds /proc/sys
//! centrally, so `ia32_binfmt_init`'s `register_sysctl("abi", vdso_table)` is
//! realized there. `vdso_sysctl_store` enforces the `proc_dointvec_minmax`
//! `[SYSCTL_ZERO, SYSCTL_ONE]` clamp.

use crate::include::uapi::errno::EINVAL;
use core::sync::atomic::{AtomicU32, Ordering};

/// `VDSO_DEFAULT` — 0 under `CONFIG_COMPAT_VDSO`, else 1. lupos targets the
/// non-compat default (a 32-bit vDSO is mapped by default).
pub const VDSO_DEFAULT: u32 = 1;

/// `vdso32_enabled` — `__read_mostly` global toggled by the boot param and the
/// `abi.vsyscall32` sysctl.
static VDSO32_ENABLED: AtomicU32 = AtomicU32::new(VDSO_DEFAULT);

/// Read the current `vdso32_enabled` value.
pub fn vdso32_enabled() -> u32 {
    VDSO32_ENABLED.load(Ordering::Relaxed)
}

/// Set `vdso32_enabled` (boot param / sysctl write).
pub fn set_vdso32_enabled(value: u32) {
    VDSO32_ENABLED.store(value, Ordering::Relaxed);
}

/// `simple_strtoul(s, NULL, 0)` semantics, used to parse the `vdso32=` value.
///
/// Base is auto-detected from the prefix: `0x`/`0X` → hex, a leading `0` →
/// octal, otherwise decimal. Parsing stops at the first character that is not a
/// digit in the detected base; a string with no leading digits yields 0.
/// Mirrors `vendor/linux/lib/vsprintf.c::simple_strtoull`.
pub fn simple_strtoul_base0(s: &str) -> u64 {
    let bytes = s.as_bytes();
    let mut i = 0;
    let (base, start) = if bytes.len() >= 2 && bytes[0] == b'0' && (bytes[1] | 0x20) == b'x' {
        (16u64, 2)
    } else if !bytes.is_empty() && bytes[0] == b'0' {
        (8u64, 1)
    } else {
        (10u64, 0)
    };
    i = start;
    let mut val: u64 = 0;
    while i < bytes.len() {
        let c = bytes[i];
        let digit = match c {
            b'0'..=b'9' => (c - b'0') as u64,
            b'a'..=b'f' => (c - b'a' + 10) as u64,
            b'A'..=b'F' => (c - b'A' + 10) as u64,
            _ => break,
        };
        if digit >= base {
            break;
        }
        val = val.wrapping_mul(base).wrapping_add(digit);
        i += 1;
    }
    val
}

/// `vdso32_setup` — handle the `vdso32=` boot parameter (and, on 32-bit
/// kernels, the equivalent `vdso=`). Values other than 0 and 1 are no longer
/// allowed and disable the vDSO. Returns 1, matching the `__setup` handler
/// contract ("parameter consumed").
pub fn vdso32_setup(s: &str) -> i32 {
    set_vdso32_enabled(simple_strtoul_base0(s) as u32);

    if vdso32_enabled() > 1 {
        // pr_warn: "vdso32 values other than 0 and 1 are no longer allowed; vdso disabled"
        set_vdso32_enabled(0);
    }

    1
}

/// Boot-parameter key registered via `__setup("vdso32=", vdso32_setup)`.
pub const VDSO32_SETUP_PARAM: &str = "vdso32=";

// ── abi.vsyscall32 sysctl (CONFIG_SYSCTL) ────────────────────────────────────

/// `proc_dointvec_minmax` clamp bounds for the toggle.
pub const SYSCTL_ZERO: i32 = 0;
pub const SYSCTL_ONE: i32 = 1;

/// Model of the single `vdso_table` ctl_table entry. On x86-64 it is registered
/// as `abi.vsyscall32`; on 32-bit as `vm.vdso_enabled`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VdsoSysctl {
    pub procname: &'static str,
    pub maxlen: usize,
    pub mode: u16,
    pub extra1: i32,
    pub extra2: i32,
}

/// `procname` of the toggle for the kernel width (`vsyscall32` on x86-64).
pub const fn vdso32_sysctl_name(x86_64: bool) -> &'static str {
    if x86_64 { "vsyscall32" } else { "vdso_enabled" }
}

/// Build the `vdso_table` entry. `maxlen = sizeof(int)`, `mode = 0644`, clamp
/// `[SYSCTL_ZERO, SYSCTL_ONE]`, `proc_handler = proc_dointvec_minmax`.
pub const fn vdso_table(x86_64: bool) -> VdsoSysctl {
    VdsoSysctl {
        procname: vdso32_sysctl_name(x86_64),
        maxlen: core::mem::size_of::<i32>(),
        mode: 0o644,
        extra1: SYSCTL_ZERO,
        extra2: SYSCTL_ONE,
    }
}

/// `proc_dointvec_minmax`-style write handler for the toggle: reject values
/// outside `[extra1, extra2]` with `-EINVAL`, mirroring the sysctl clamp.
pub fn vdso_sysctl_store(table: &VdsoSysctl, value: i32) -> Result<(), i32> {
    if value < table.extra1 || value > table.extra2 {
        return Err(EINVAL);
    }
    set_vdso32_enabled(value as u32);
    Ok(())
}

/// `ia32_binfmt_init` — register the toggle under `abi` (x86-64). Returns 0.
/// The actual `register_sysctl` call belongs to the sysctl core; this mirrors
/// the initcall and the registration path it selects.
pub fn ia32_binfmt_init(x86_64: bool) -> (i32, &'static str, VdsoSysctl) {
    let path = if x86_64 { "abi" } else { "vm" };
    (0, path, vdso_table(x86_64))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_strtoul_detects_base_from_prefix() {
        assert_eq!(simple_strtoul_base0("0"), 0);
        assert_eq!(simple_strtoul_base0("1"), 1);
        assert_eq!(simple_strtoul_base0("2"), 2);
        assert_eq!(simple_strtoul_base0("10"), 10);
        assert_eq!(simple_strtoul_base0("0x1f"), 0x1f);
        assert_eq!(simple_strtoul_base0("0X10"), 0x10);
        assert_eq!(simple_strtoul_base0("010"), 0o10); // octal
        assert_eq!(simple_strtoul_base0("12abc"), 12); // stop at non-digit
        assert_eq!(simple_strtoul_base0("abc"), 0); // no leading digit
        assert_eq!(simple_strtoul_base0(""), 0);
    }

    #[test]
    fn setup_allows_only_zero_and_one_and_returns_consumed() {
        assert_eq!(vdso32_setup("1"), 1);
        assert_eq!(vdso32_enabled(), 1);
        assert_eq!(vdso32_setup("0"), 1);
        assert_eq!(vdso32_enabled(), 0);
        // Values > 1 disable the vDSO entirely.
        assert_eq!(vdso32_setup("2"), 1);
        assert_eq!(vdso32_enabled(), 0);
        assert_eq!(vdso32_setup("0x5"), 1);
        assert_eq!(vdso32_enabled(), 0);
        // restore default for other tests.
        set_vdso32_enabled(VDSO_DEFAULT);
    }

    #[test]
    fn sysctl_name_and_table_match_kernel_width() {
        assert_eq!(vdso32_sysctl_name(true), "vsyscall32");
        assert_eq!(vdso32_sysctl_name(false), "vdso_enabled");
        let t = vdso_table(true);
        assert_eq!(t.procname, "vsyscall32");
        assert_eq!(t.maxlen, 4);
        assert_eq!(t.mode, 0o644);
        assert_eq!((t.extra1, t.extra2), (0, 1));
    }

    #[test]
    fn sysctl_store_clamps_to_zero_one() {
        let t = vdso_table(true);
        assert_eq!(vdso_sysctl_store(&t, 0), Ok(()));
        assert_eq!(vdso32_enabled(), 0);
        assert_eq!(vdso_sysctl_store(&t, 1), Ok(()));
        assert_eq!(vdso32_enabled(), 1);
        assert_eq!(vdso_sysctl_store(&t, 2), Err(EINVAL));
        assert_eq!(vdso_sysctl_store(&t, -1), Err(EINVAL));
        set_vdso32_enabled(VDSO_DEFAULT);
    }

    #[test]
    fn binfmt_init_registers_under_abi_on_x86_64() {
        let (ret, path, table) = ia32_binfmt_init(true);
        assert_eq!(ret, 0);
        assert_eq!(path, "abi");
        assert_eq!(table.procname, "vsyscall32");
    }
}
