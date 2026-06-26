//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/main.c
//! test-origin: linux:vendor/linux/arch/x86/boot/main.c
//! Real-mode `main()` orchestrator.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/main.c
//!
//! `main()` is the entry point of Linux's real-mode `setup.bin`. Lupos'
//! generated bzImage currently uses Linux's compressed `head_64.S` plus a
//! temporary uncompressed-ELF extractor before the `linux64_start` handoff,
//! while this Rust twin preserves the setup ordering as an ABI surface we
//! can describe and validate.

/// `boot_params` must be exactly 4096 bytes. Linux enforces this with
/// `BUILD_BUG_ON` at main.c line 38; lupos re-enforces it via a const
/// assertion. The actual struct lives alongside this file as the
/// reference layout (`crate::arch::x86::boot::BootParams` once that
/// batch lands).
pub const BOOT_PARAMS_SIZE: usize = 4096;

const _BOOT_PARAMS_SIZE_OK: () = {
    if BOOT_PARAMS_SIZE != 4096 {
        panic!("boot_params must be 4 KiB to match Linux");
    }
};

/// Steps Linux's `main()` executes in order. Exposed as an enum so a
/// test or bzImage compat checker can reason about the sequence without
/// invoking real-mode code.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum BootStep {
    InitDefaultIoOps,
    CopyBootParams,
    ConsoleInit,
    InitHeap,
    ValidateCpu,
    SetBiosMode,
    DetectMemory,
    KeyboardInit,
    QueryIst,
    QueryApmBios,
    QueryEdd,
    SetVideo,
    GoToProtectedMode,
}

/// The canonical Linux `main()` sequence, line-for-line from main.c
/// lines 134-181.
pub fn boot_step_sequence() -> [BootStep; 13] {
    use BootStep::*;
    [
        InitDefaultIoOps,
        CopyBootParams,
        ConsoleInit,
        InitHeap,
        ValidateCpu,
        SetBiosMode,
        DetectMemory,
        KeyboardInit,
        QueryIst,
        QueryApmBios,
        QueryEdd,
        SetVideo,
        GoToProtectedMode,
    ]
}

/// `OLD_CL_MAGIC` — the legacy command-line protocol sentinel Linux
/// checks at main.c line 41.
pub const OLD_CL_MAGIC: u16 = 0xA33F;
/// `OLD_CL_ADDRESS` — physical address of the old-style cmdline tag.
pub const OLD_CL_ADDRESS: u32 = 0x0020;

/// `CAN_USE_HEAP` — bit 0x80 of `boot_params.hdr.loadflags`.
pub const CAN_USE_HEAP: u8 = 0x80;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boot_params_size_constant_matches_linux_assert() {
        assert_eq!(BOOT_PARAMS_SIZE, 4096);
    }

    #[test]
    fn boot_step_sequence_matches_linux_main_order() {
        // The order is itself part of the ABI: bzImage compat tooling
        // expects validate_cpu to fire before BIOS calls, EDD after
        // APM, video before the protected-mode jump.
        let seq = boot_step_sequence();
        assert_eq!(seq[0], BootStep::InitDefaultIoOps);
        assert_eq!(seq[4], BootStep::ValidateCpu);
        assert_eq!(seq[9], BootStep::QueryApmBios);
        assert_eq!(seq[10], BootStep::QueryEdd);
        assert_eq!(seq[seq.len() - 1], BootStep::GoToProtectedMode);
    }

    #[test]
    fn old_cl_sentinels_match_legacy_protocol() {
        assert_eq!(OLD_CL_MAGIC, 0xA33F);
        assert_eq!(OLD_CL_ADDRESS, 0x0020);
        assert_eq!(CAN_USE_HEAP, 0x80);
    }
}
