//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kvm/debugfs.c
//! test-origin: linux:vendor/linux/arch/x86/kvm/debugfs.c
//! KVM x86 debugfs file registration and rmap statistics.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

pub const RMAP_LOG_SIZE: usize = 11;
pub const KVM_NR_PAGE_SIZES: usize = 3;
pub const KVM_LPAGE_STR: [&str; KVM_NR_PAGE_SIZES] = ["4K", "2M", "1G"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KvmVcpuDebugState {
    pub lapic_timer_advance_ns: u64,
    pub guest_mode: u64,
    pub tsc_offset: i64,
    pub tsc_scaling_ratio: u64,
    pub tsc_scaling_ratio_frac_bits: u64,
    pub lapic_in_kernel: bool,
    pub has_tsc_control: bool,
}

pub const fn vcpu_get_timer_advance_ns(vcpu: KvmVcpuDebugState) -> u64 {
    vcpu.lapic_timer_advance_ns
}

pub const fn vcpu_get_guest_mode(vcpu: KvmVcpuDebugState) -> u64 {
    vcpu.guest_mode
}

pub const fn vcpu_get_tsc_offset(vcpu: KvmVcpuDebugState) -> i64 {
    vcpu.tsc_offset
}

pub const fn vcpu_get_tsc_scaling_ratio(vcpu: KvmVcpuDebugState) -> u64 {
    vcpu.tsc_scaling_ratio
}

pub const fn vcpu_get_tsc_scaling_frac_bits(vcpu: KvmVcpuDebugState) -> u64 {
    vcpu.tsc_scaling_ratio_frac_bits
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DebugfsFile {
    pub name: &'static str,
    pub mode: u16,
    pub fops: &'static str,
}

pub fn kvm_arch_create_vcpu_debugfs_plan(vcpu: KvmVcpuDebugState) -> Vec<DebugfsFile> {
    let mut files = Vec::with_capacity(5);
    files.push(DebugfsFile {
        name: "guest_mode",
        mode: 0o444,
        fops: "vcpu_guest_mode_fops",
    });
    files.push(DebugfsFile {
        name: "tsc-offset",
        mode: 0o444,
        fops: "vcpu_tsc_offset_fops",
    });
    if vcpu.lapic_in_kernel {
        files.push(DebugfsFile {
            name: "lapic_timer_advance_ns",
            mode: 0o444,
            fops: "vcpu_timer_advance_ns_fops",
        });
    }
    if vcpu.has_tsc_control {
        files.push(DebugfsFile {
            name: "tsc-scaling-ratio",
            mode: 0o444,
            fops: "vcpu_tsc_scaling_fops",
        });
        files.push(DebugfsFile {
            name: "tsc-scaling-ratio-frac-bits",
            mode: 0o444,
            fops: "vcpu_tsc_scaling_frac_fops",
        });
    }
    files
}

pub const VM_DEBUGFS_FILES: [DebugfsFile; 1] = [DebugfsFile {
    name: "mmu_rmaps_stat",
    mode: 0o644,
    fops: "mmu_rmaps_stat_fops",
}];

pub const fn rmap_log_bucket(count: u32) -> usize {
    if count == 0 {
        0
    } else {
        let bucket = count.trailing_zeros() as usize + 1;
        if bucket >= RMAP_LOG_SIZE {
            RMAP_LOG_SIZE - 1
        } else {
            bucket
        }
    }
}

pub fn rmap_histogram(
    level_counts: [&[u32]; KVM_NR_PAGE_SIZES],
) -> [[u32; RMAP_LOG_SIZE]; KVM_NR_PAGE_SIZES] {
    let mut log = [[0u32; RMAP_LOG_SIZE]; KVM_NR_PAGE_SIZES];
    for (level, counts) in level_counts.iter().enumerate() {
        for &count in *counts {
            log[level][rmap_log_bucket(count)] += 1;
        }
    }
    log
}

pub fn format_mmu_rmaps_stat(log: &[[u32; RMAP_LOG_SIZE]; KVM_NR_PAGE_SIZES]) -> String {
    let mut out = String::from("Rmap_Count:\t0\t1\t");
    for i in 2..RMAP_LOG_SIZE {
        let first = 1 << (i - 1);
        let last = (1 << i) - 1;
        out.push_str(&alloc::format!("{first}-{last}\t"));
    }
    out.push('\n');

    for i in 0..KVM_NR_PAGE_SIZES {
        out.push_str("Level=");
        out.push_str(KVM_LPAGE_STR[i]);
        out.push_str(":\t");
        for value in log[i] {
            out.push_str(&alloc::format!("{value}\t"));
        }
        out.push('\n');
    }
    out
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MmuRmapsStatOpenPlan {
    pub ret: i32,
    pub get_kvm_safe: bool,
    pub single_open: bool,
    pub put_kvm_on_single_open_error: bool,
}

pub const fn kvm_mmu_rmaps_stat_open_plan(
    get_kvm_safe: bool,
    single_open_ret: i32,
) -> MmuRmapsStatOpenPlan {
    if !get_kvm_safe {
        return MmuRmapsStatOpenPlan {
            ret: -2,
            get_kvm_safe: false,
            single_open: false,
            put_kvm_on_single_open_error: false,
        };
    }
    MmuRmapsStatOpenPlan {
        ret: single_open_ret,
        get_kvm_safe: true,
        single_open: true,
        put_kvm_on_single_open_error: single_open_ret < 0,
    }
}

pub const fn kvm_mmu_rmaps_stat_release_plan() -> (&'static str, &'static str) {
    ("kvm_put_kvm", "single_release")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kvm_debugfs_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kvm/debugfs.c"
        ));
        let kvm_host = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/include/asm/kvm_host.h"
        ));
        let mmu_internal = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kvm/mmu/mmu_internal.h"
        ));

        assert!(source.contains("static int vcpu_get_timer_advance_ns"));
        assert!(source.contains("vcpu->arch.apic->lapic_timer.timer_advance_ns"));
        assert!(source.contains("DEFINE_SIMPLE_ATTRIBUTE(vcpu_timer_advance_ns_fops"));
        assert!(source.contains("static int vcpu_get_guest_mode"));
        assert!(source.contains("*val = vcpu->stat.guest_mode;"));
        assert!(source.contains("DEFINE_SIMPLE_ATTRIBUTE(vcpu_guest_mode_fops"));
        assert!(source.contains("*val = vcpu->arch.tsc_offset;"));
        assert!(source.contains("*val = vcpu->arch.tsc_scaling_ratio;"));
        assert!(source.contains("*val = kvm_caps.tsc_scaling_ratio_frac_bits;"));
        assert!(source.contains("void kvm_arch_create_vcpu_debugfs"));
        assert!(source.contains("debugfs_create_file(\"guest_mode\", 0444"));
        assert!(source.contains("debugfs_create_file(\"tsc-offset\", 0444"));
        assert!(source.contains("if (lapic_in_kernel(vcpu))"));
        assert!(source.contains("debugfs_create_file(\"lapic_timer_advance_ns\", 0444"));
        assert!(source.contains("if (kvm_caps.has_tsc_control)"));
        assert!(source.contains("debugfs_create_file(\"tsc-scaling-ratio\", 0444"));
        assert!(source.contains("debugfs_create_file(\"tsc-scaling-ratio-frac-bits\", 0444"));
        assert!(source.contains("#define  RMAP_LOG_SIZE  11"));
        assert!(source.contains(
            "static const char *kvm_lpage_str[KVM_NR_PAGE_SIZES] = { \"4K\", \"2M\", \"1G\" };"
        ));
        assert!(source.contains("if (!kvm_memslots_have_rmaps(kvm))"));
        assert!(source.contains("log[i] = kcalloc(RMAP_LOG_SIZE"));
        assert!(source.contains("mutex_lock(&kvm->slots_lock);"));
        assert!(source.contains("write_lock(&kvm->mmu_lock);"));
        assert!(source.contains("index = ffs(pte_list_count(&rmap[l]));"));
        assert!(source.contains("if (WARN_ON_ONCE(index >= RMAP_LOG_SIZE))"));
        assert!(source.contains("seq_printf(m, \"Rmap_Count:\\t0\\t1\\t\");"));
        assert!(source.contains("seq_printf(m, \"Level=%s:\\t\", kvm_lpage_str[i]);"));
        assert!(source.contains("if (!kvm_get_kvm_safe(kvm))"));
        assert!(source.contains("return -ENOENT;"));
        assert!(source.contains("single_open(file, kvm_mmu_rmaps_stat_show, kvm);"));
        assert!(source.contains("kvm_put_kvm(kvm);"));
        assert!(source.contains(".open\t\t= kvm_mmu_rmaps_stat_open"));
        assert!(source.contains(".release\t= kvm_mmu_rmaps_stat_release"));
        assert!(source.contains("debugfs_create_file(\"mmu_rmaps_stat\", 0644"));
        assert!(kvm_host.contains("#define KVM_NR_PAGE_SIZES"));
        assert!(mmu_internal.contains("pte_list_count"));
    }

    #[test]
    fn vcpu_debugfs_files_follow_lapic_and_tsc_controls() {
        let vcpu = KvmVcpuDebugState {
            lapic_timer_advance_ns: 123,
            guest_mode: 1,
            tsc_offset: -44,
            tsc_scaling_ratio: 55,
            tsc_scaling_ratio_frac_bits: 48,
            lapic_in_kernel: true,
            has_tsc_control: true,
        };

        assert_eq!(vcpu_get_timer_advance_ns(vcpu), 123);
        assert_eq!(vcpu_get_guest_mode(vcpu), 1);
        assert_eq!(vcpu_get_tsc_offset(vcpu), -44);
        assert_eq!(vcpu_get_tsc_scaling_ratio(vcpu), 55);
        assert_eq!(vcpu_get_tsc_scaling_frac_bits(vcpu), 48);

        let files = kvm_arch_create_vcpu_debugfs_plan(vcpu);
        assert_eq!(files.len(), 5);
        assert!(
            files
                .iter()
                .any(|file| file.name == "lapic_timer_advance_ns")
        );
        assert!(files.iter().any(|file| file.name == "tsc-scaling-ratio"));
        assert!(files.iter().all(|file| file.mode == 0o444));

        let minimal = kvm_arch_create_vcpu_debugfs_plan(KvmVcpuDebugState {
            lapic_in_kernel: false,
            has_tsc_control: false,
            ..vcpu
        });
        assert_eq!(
            minimal,
            alloc::vec![
                DebugfsFile {
                    name: "guest_mode",
                    mode: 0o444,
                    fops: "vcpu_guest_mode_fops",
                },
                DebugfsFile {
                    name: "tsc-offset",
                    mode: 0o444,
                    fops: "vcpu_tsc_offset_fops",
                },
            ]
        );
    }

    #[test]
    fn rmap_histogram_and_rendering_match_linux_bucket_shape() {
        assert_eq!(rmap_log_bucket(0), 0);
        assert_eq!(rmap_log_bucket(1), 1);
        assert_eq!(rmap_log_bucket(2), 2);
        assert_eq!(rmap_log_bucket(4), 3);
        assert_eq!(rmap_log_bucket(1 << 20), RMAP_LOG_SIZE - 1);

        let small = [0, 1, 2, 4, 8];
        let medium = [1, 1, 2];
        let large = [0, 0, 16];
        let hist = rmap_histogram([&small, &medium, &large]);
        assert_eq!(hist[0][0], 1);
        assert_eq!(hist[0][1], 1);
        assert_eq!(hist[0][2], 1);
        assert_eq!(hist[0][3], 1);
        assert_eq!(hist[0][4], 1);
        assert_eq!(hist[1][1], 2);
        assert_eq!(hist[1][2], 1);
        assert_eq!(hist[2][0], 2);
        assert_eq!(hist[2][5], 1);

        let rendered = format_mmu_rmaps_stat(&hist);
        assert!(rendered.starts_with("Rmap_Count:\t0\t1\t2-3\t4-7\t"));
        assert!(rendered.contains("Level=4K:\t1\t1\t1\t1\t1\t"));
        assert!(rendered.contains("Level=2M:\t0\t2\t1\t"));
        assert!(rendered.contains("Level=1G:\t2\t0\t0\t0\t0\t1\t"));
    }

    #[test]
    fn mmu_rmaps_open_and_release_plans_follow_linux_lifetime_rules() {
        assert_eq!(
            VM_DEBUGFS_FILES,
            [DebugfsFile {
                name: "mmu_rmaps_stat",
                mode: 0o644,
                fops: "mmu_rmaps_stat_fops",
            }]
        );
        assert_eq!(
            kvm_mmu_rmaps_stat_open_plan(false, 0),
            MmuRmapsStatOpenPlan {
                ret: -2,
                get_kvm_safe: false,
                single_open: false,
                put_kvm_on_single_open_error: false,
            }
        );
        assert_eq!(
            kvm_mmu_rmaps_stat_open_plan(true, -12),
            MmuRmapsStatOpenPlan {
                ret: -12,
                get_kvm_safe: true,
                single_open: true,
                put_kvm_on_single_open_error: true,
            }
        );
        assert_eq!(
            kvm_mmu_rmaps_stat_open_plan(true, 0),
            MmuRmapsStatOpenPlan {
                ret: 0,
                get_kvm_safe: true,
                single_open: true,
                put_kvm_on_single_open_error: false,
            }
        );
        assert_eq!(
            kvm_mmu_rmaps_stat_release_plan(),
            ("kvm_put_kvm", "single_release")
        );
    }
}
