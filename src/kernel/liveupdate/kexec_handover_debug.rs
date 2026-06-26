//! linux-parity: complete
//! linux-source: vendor/linux/kernel/liveupdate/kexec_handover_debug.c
//! test-origin: linux:vendor/linux/kernel/liveupdate/kexec_handover_debug.c
//! Kexec handover scratch overlap check.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KhoScratch {
    pub addr: u64,
    pub size: u64,
}

pub fn kho_scratch_overlap(phys: u64, size: u64, scratch: &[KhoScratch]) -> bool {
    let end = phys.wrapping_add(size);
    for entry in scratch {
        let scratch_start = entry.addr;
        let scratch_end = entry.addr.wrapping_add(entry.size);
        if phys < scratch_end && end > scratch_start {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kho_scratch_overlap_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/liveupdate/kexec_handover_debug.c"
        ));
        assert!(source.contains("#define pr_fmt(fmt) \"KHO: \" fmt"));
        assert!(source.contains("#include \"kexec_handover_internal.h\""));
        assert!(source.contains("for (i = 0; i < kho_scratch_cnt; i++)"));
        assert!(source.contains("phys < scratch_end && (phys + size) > scratch_start"));
        let scratch = [
            KhoScratch {
                addr: 100,
                size: 50,
            },
            KhoScratch {
                addr: 500,
                size: 10,
            },
        ];
        assert!(kho_scratch_overlap(90, 20, &scratch));
        assert!(kho_scratch_overlap(149, 1, &scratch));
        assert!(!kho_scratch_overlap(150, 10, &scratch));
        assert!(!kho_scratch_overlap(300, 10, &scratch));
    }
}
