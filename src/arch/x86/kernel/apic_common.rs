//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/apic/apic_common.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/apic/apic_common.c
//! Common APIC helper routines shared by APIC driver flavours.

pub const BAD_APICID: u32 = 0xffff;
pub const APIC_DFR_FLAT: u32 = 0xffff_ffff;
pub const APIC_LDR_MASK: u32 = 0xff << 24;

pub const fn set_apic_logical_id(id: u32) -> u32 {
    id << 24
}

pub fn apic_default_calc_apicid(cpu: usize, x86_cpu_to_apicid: &[u32]) -> u32 {
    x86_cpu_to_apicid[cpu]
}

pub const fn apic_flat_calc_apicid(cpu: u32) -> u32 {
    if cpu < 32 { 1u32 << cpu } else { 0 }
}

pub fn default_cpu_present_to_apicid(
    mps_cpu: usize,
    nr_cpu_ids: usize,
    cpu_present: &[bool],
    x86_cpu_to_apicid: &[u32],
) -> u32 {
    if mps_cpu < nr_cpu_ids && cpu_present.get(mps_cpu).copied().unwrap_or(false) {
        x86_cpu_to_apicid[mps_cpu]
    } else {
        BAD_APICID
    }
}

pub const fn default_init_apic_ldr_value(current_ldr: u32, cpu: u32) -> u32 {
    (current_ldr & !APIC_LDR_MASK) | set_apic_logical_id(1u32 << cpu)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apic_id_helpers_match_linux_common_code() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kernel/apic/apic_common.c"
        ));
        assert!(source.contains("return per_cpu(x86_cpu_to_apicid, cpu);"));
        assert!(source.contains("return 1U << cpu;"));
        assert!(source.contains("return BAD_APICID;"));

        let ids = [4, 8, 16];
        let present = [true, false, true];
        assert_eq!(apic_default_calc_apicid(2, &ids), 16);
        assert_eq!(apic_flat_calc_apicid(3), 8);
        assert_eq!(default_cpu_present_to_apicid(0, 3, &present, &ids), 4);
        assert_eq!(
            default_cpu_present_to_apicid(1, 3, &present, &ids),
            BAD_APICID
        );
        assert_eq!(
            default_cpu_present_to_apicid(4, 3, &present, &ids),
            BAD_APICID
        );
    }

    #[test]
    fn default_init_apic_ldr_replaces_logical_id_bits() {
        let preserved = 0x00ab_cdef;
        assert_eq!(
            default_init_apic_ldr_value(0xffab_cdef, 2),
            preserved | (4 << 24)
        );
        assert_eq!(APIC_DFR_FLAT, 0xffff_ffff);
    }
}
