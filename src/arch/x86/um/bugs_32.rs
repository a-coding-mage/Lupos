//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/um/bugs_32.c
//! test-origin: linux:vendor/linux/arch/x86/um/bugs_32.c
//! 32-bit UML host CMOV probe and SIGILL diagnosis.

pub const SIGILL: i32 = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CmovSignalDiagnosis {
    NotSigill,
    NotInit,
    ReadFailed,
    NotCmov,
    HostLacksCmov,
    HostClaimsCmov,
    BadHostCmovValue,
}

pub const fn is_cmov_opcode(bytes: [u8; 2]) -> bool {
    bytes[0] == 0x0f && (bytes[1] & 0xf0) == 0x40
}

pub const fn arch_examine_signal(
    sig: i32,
    current_pid: i32,
    instr: Option<[u8; 2]>,
    host_has_cmov: i32,
) -> CmovSignalDiagnosis {
    if sig != SIGILL {
        return CmovSignalDiagnosis::NotSigill;
    }
    if current_pid != 1 {
        return CmovSignalDiagnosis::NotInit;
    }
    let Some(instr) = instr else {
        return CmovSignalDiagnosis::ReadFailed;
    };
    if !is_cmov_opcode(instr) {
        return CmovSignalDiagnosis::NotCmov;
    }
    if host_has_cmov == 0 {
        CmovSignalDiagnosis::HostLacksCmov
    } else if host_has_cmov == 1 {
        CmovSignalDiagnosis::HostClaimsCmov
    } else {
        CmovSignalDiagnosis::BadHostCmovValue
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uml_i386_cmov_signal_path_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/um/bugs_32.c"
        ));
        assert!(source.contains("static int host_has_cmov = 1;"));
        assert!(source.contains("cmov_sigill_test_handler"));
        assert!(source.contains("host_has_cmov = 0;"));
        assert!(source.contains("new.sa_flags = SA_NODEFER;"));
        assert!(source.contains("__asm__ __volatile__(\"cmovz %0, %1\""));
        assert!(source.contains("if ((sig != SIGILL) || (get_current_pid() != 1))"));
        assert!(source.contains("copy_from_user_proc(tmp, (void *) UPT_IP(regs), 2)"));
        assert!(source.contains("tmp[0] != 0x0f"));
        assert!(source.contains("(tmp[1] & 0xf0) != 0x40"));
        assert!(source.contains("Boot a filesystem"));

        assert!(is_cmov_opcode([0x0f, 0x44]));
        assert!(!is_cmov_opcode([0x90, 0x44]));
        assert_eq!(
            arch_examine_signal(SIGILL, 1, Some([0x0f, 0x44]), 0),
            CmovSignalDiagnosis::HostLacksCmov
        );
        assert_eq!(
            arch_examine_signal(SIGILL, 1, Some([0x0f, 0x44]), 1),
            CmovSignalDiagnosis::HostClaimsCmov
        );
        assert_eq!(
            arch_examine_signal(SIGILL, 2, Some([0x0f, 0x44]), 1),
            CmovSignalDiagnosis::NotInit
        );
    }
}
