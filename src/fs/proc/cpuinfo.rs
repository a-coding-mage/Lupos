//! linux-parity: complete
//! linux-source: vendor/linux/fs/proc/cpuinfo.c
//! test-origin: linux:vendor/linux/fs/proc/cpuinfo.c
//! `/proc/cpuinfo`.
//!
//! Ref: `vendor/linux/fs/proc/cpuinfo.c`

extern crate alloc;

use alloc::string::String;
use alloc::sync::Arc;

use crate::arch::x86::kernel::cpu::{CpuSignature, CpuVendor, proc as arch_cpuinfo};
use crate::arch::x86::kernel::{cpuid, tsc};
use crate::fs::kernfs::KernfsNode;

pub fn show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    super::util::copy_into(buf, &cpuinfo_text())
}

pub fn cpuinfo_text() -> String {
    let info = current_cpu_info();
    let lines = arch_cpuinfo::render_block(&info);
    let mut out = String::new();
    for line in lines {
        out.push_str(&line);
        out.push('\n');
    }
    out.push('\n');
    out
}

fn current_cpu_info() -> arch_cpuinfo::CpuInfo {
    let signature = CpuSignature::from_leaf1_eax(cpuid::cpuid(1, 0).eax);
    arch_cpuinfo::CpuInfo {
        processor: 0,
        vendor: CpuVendor::current(),
        signature,
        model_name: cpu_model_name(),
        mhz: cpu_mhz(),
    }
}

fn cpu_model_name() -> String {
    let brand = cpuid::brand_string();
    let model = trim_ascii_nul_padded(&brand);
    if model.is_empty() {
        String::from("unknown")
    } else {
        String::from(model)
    }
}

fn trim_ascii_nul_padded(bytes: &[u8]) -> &str {
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    core::str::from_utf8(&bytes[..end]).unwrap_or("").trim()
}

fn cpu_mhz() -> u32 {
    let khz = match tsc::tsc_khz() {
        0 if cpuid::max_basic_leaf() >= 0x16 => {
            tsc::khz_from_cpuid_leaf16(cpuid::cpuid(0x16, 0)).unwrap_or(0)
        }
        khz => khz,
    };
    (khz / 1000).min(u32::MAX as u64) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proc_cpuinfo_matches_linux_wrapper_contract() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/proc/cpuinfo.c"
        ));
        assert!(source.contains("extern const struct seq_operations cpuinfo_op;"));
        assert!(source.contains("return seq_open(file, &cpuinfo_op);"));
        assert!(source.contains(".proc_open\t= cpuinfo_open"));
        assert!(source.contains(".proc_read_iter\t= seq_read_iter"));
        assert!(source.contains("proc_create(\"cpuinfo\", 0, NULL, &cpuinfo_proc_ops);"));

        let text = cpuinfo_text();
        assert!(text.contains("processor\t: 0\n"));
        assert!(text.contains("vendor_id\t: "));
        assert!(text.contains("cpu family\t: "));
        assert!(text.contains("model name\t: "));
        assert!(text.ends_with("\n\n"));
    }
}
