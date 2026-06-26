//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/powerflags.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/powerflags.c
//! /proc/cpuinfo "power management" flags string table.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/powerflags.c

// `powerflags.c` ships a static `x86_power_flags` table indexed by bit
// position in CPUID(0x8000_0007).EDX. We mirror the same labels.

pub const POWER_FLAG_LABELS: [&str; 14] = [
    "ts",
    "fid",
    "vid",
    "ttp",
    "tm",
    "stc",
    "100mhzsteps",
    "hwpstate",
    "",
    "cpb",
    "eff_freq_ro",
    "proc_feedback",
    "acc_power",
    "tsc_known_freq",
];

pub fn label_for_bit(bit: u8) -> &'static str {
    if (bit as usize) < POWER_FLAG_LABELS.len() {
        POWER_FLAG_LABELS[bit as usize]
    } else {
        ""
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_match_linux_power_flag_strings() {
        assert_eq!(label_for_bit(0), "ts");
        assert_eq!(label_for_bit(9), "cpb");
        assert_eq!(label_for_bit(13), "tsc_known_freq");
        assert_eq!(label_for_bit(8), "");
        assert_eq!(label_for_bit(99), "");
    }
}
