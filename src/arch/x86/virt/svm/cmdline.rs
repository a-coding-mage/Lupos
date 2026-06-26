//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/virt/svm/cmdline.c
//! test-origin: linux:vendor/linux/arch/x86/virt/svm/cmdline.c
//! AMD SVM-SEV command-line parsing.

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SevConfig {
    pub debug: bool,
    pub sev_snp_cpu_cap_cleared: bool,
    pub host_sev_snp_attr_cleared: bool,
    pub warnings: usize,
}

pub fn init_sev_config(arg: &str, hypervisor_feature_enabled: bool) -> (i32, SevConfig) {
    let mut cfg = SevConfig::default();

    for token in arg.split(',') {
        if token == "debug" {
            cfg.debug = true;
            continue;
        }

        if token == "nosnp" {
            if !hypervisor_feature_enabled {
                cfg.sev_snp_cpu_cap_cleared = true;
                cfg.host_sev_snp_attr_cleared = true;
                continue;
            }
        }

        cfg.warnings += 1;
    }

    (1, cfg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sev_cmdline_accepts_debug_and_bare_metal_nosnp() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/virt/svm/cmdline.c"
        ));
        assert!(source.contains("__setup(\"sev=\", init_sev_config);"));
        assert!(source.contains("setup_clear_cpu_cap(X86_FEATURE_SEV_SNP);"));
        assert!(source.contains("cc_platform_clear(CC_ATTR_HOST_SEV_SNP);"));

        assert_eq!(
            init_sev_config("debug,nosnp", false),
            (
                1,
                SevConfig {
                    debug: true,
                    sev_snp_cpu_cap_cleared: true,
                    host_sev_snp_attr_cleared: true,
                    warnings: 0,
                }
            )
        );
    }

    #[test]
    fn sev_cmdline_warns_for_hypervisor_nosnp_and_unknown_tokens() {
        assert_eq!(
            init_sev_config("nosnp,wat", true),
            (
                1,
                SevConfig {
                    debug: false,
                    sev_snp_cpu_cap_cleared: false,
                    host_sev_snp_attr_cleared: false,
                    warnings: 2,
                }
            )
        );
    }
}
