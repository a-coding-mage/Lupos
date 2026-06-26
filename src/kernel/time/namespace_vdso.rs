//! linux-parity: complete
//! linux-source: vendor/linux/kernel/time/namespace_vdso.c
//! test-origin: linux:vendor/linux/kernel/time/namespace_vdso.c
//! VDSO time namespace coverage for M36.
//!
//! Mirrors `vendor/linux/kernel/time/namespace_vdso.c`.

use super::namespace::TimeNamespace;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VdsoTimeNamespaceData {
    pub monotonic_ns: u64,
    pub boottime_ns: u64,
}

pub fn update_vdso_time_namespace(
    namespace: &TimeNamespace,
    monotonic_base: u64,
    boottime_base: u64,
) -> VdsoTimeNamespaceData {
    VdsoTimeNamespaceData {
        monotonic_ns: namespace.monotonic_now(monotonic_base),
        boottime_ns: namespace.boottime_now(boottime_base),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vdso_snapshot_includes_namespace_offsets() {
        let ns = TimeNamespace::new();
        ns.set_offsets(1, 2);
        let data = update_vdso_time_namespace(&ns, 10, 20);
        assert_eq!(data.monotonic_ns, 11);
        assert_eq!(data.boottime_ns, 22);
    }
}
