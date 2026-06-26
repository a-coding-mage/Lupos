//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/coco/tdx/tdx-shared.c
//! test-origin: linux:vendor/linux/arch/x86/coco/tdx/tdx-shared.c
//! TDX shared page-acceptance and hypercall helpers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/coco/tdx/tdx-shared.c

pub const TDG_VP_VMCALL: u64 = 0;
pub const TDG_MEM_PAGE_ACCEPT: u64 = 6;
pub const TDX_PS_4K: u8 = 0;
pub const TDX_PS_2M: u8 = 1;
pub const TDX_PS_1G: u8 = 2;

pub const TDX_RDX: u64 = 1 << 2;
pub const TDX_RBX: u64 = 1 << 3;
pub const TDX_RSI: u64 = 1 << 6;
pub const TDX_RDI: u64 = 1 << 7;
pub const TDX_R8: u64 = 1 << 8;
pub const TDX_R9: u64 = 1 << 9;
pub const TDX_R10: u64 = 1 << 10;
pub const TDX_R11: u64 = 1 << 11;
pub const TDX_R12: u64 = 1 << 12;
pub const TDX_R13: u64 = 1 << 13;
pub const TDX_R14: u64 = 1 << 14;
pub const TDX_R15: u64 = 1 << 15;
pub const TDVMCALL_EXPOSE_REGS_MASK: u64 = TDX_RDX
    | TDX_RBX
    | TDX_RSI
    | TDX_RDI
    | TDX_R8
    | TDX_R9
    | TDX_R10
    | TDX_R11
    | TDX_R12
    | TDX_R13
    | TDX_R14
    | TDX_R15;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TdxModuleArgs {
    pub rax: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rbx: u64,
    pub rdi: u64,
    pub rsi: u64,
}

impl TdxModuleArgs {
    pub const DEFAULT: Self = Self {
        rax: 0,
        rcx: 0,
        rdx: 0,
        r8: 0,
        r9: 0,
        r10: 0,
        r11: 0,
        r12: 0,
        r13: 0,
        r14: 0,
        r15: 0,
        rbx: 0,
        rdi: 0,
        rsi: 0,
    };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PgLevel {
    Level4K,
    Level2M,
    Level1G,
}

impl PgLevel {
    pub const fn size(self) -> u64 {
        match self {
            Self::Level4K => 4096,
            Self::Level2M => 2 * 1024 * 1024,
            Self::Level1G => 1024 * 1024 * 1024,
        }
    }

    pub const fn tdx_ps(self) -> u8 {
        match self {
            Self::Level4K => TDX_PS_4K,
            Self::Level2M => TDX_PS_2M,
            Self::Level1G => TDX_PS_1G,
        }
    }
}

pub trait TdxAccept {
    fn tdcall_accept(&mut self, args: &mut TdxModuleArgs) -> u64;
}

pub fn try_accept_one<T: TdxAccept>(backend: &mut T, start: u64, len: u64, level: PgLevel) -> u64 {
    let accept_size = level.size();
    if start % accept_size != 0 || len < accept_size {
        return 0;
    }
    let mut args = TdxModuleArgs {
        rcx: start | level.tdx_ps() as u64,
        ..Default::default()
    };
    if backend.tdcall_accept(&mut args) != 0 {
        0
    } else {
        accept_size
    }
}

pub fn tdx_accept_memory<T: TdxAccept>(backend: &mut T, mut start: u64, end: u64) -> bool {
    while start < end {
        let len = end - start;
        let mut accept_size = try_accept_one(backend, start, len, PgLevel::Level1G);
        if accept_size == 0 {
            accept_size = try_accept_one(backend, start, len, PgLevel::Level2M);
        }
        if accept_size == 0 {
            accept_size = try_accept_one(backend, start, len, PgLevel::Level4K);
        }
        if accept_size == 0 {
            return false;
        }
        start += accept_size;
    }
    true
}

pub trait TdxHypercall {
    fn tdcall_saved_ret(&mut self, leaf: u64, args: &mut TdxModuleArgs) -> u64;
}

#[cold]
fn tdx_hypercall_failed() -> ! {
    panic!("TDVMCALL failed. TDX module bug?");
}

pub fn tdx_hypercall<T: TdxHypercall>(backend: &mut T, args: &mut TdxModuleArgs) -> u64 {
    args.rcx = TDVMCALL_EXPOSE_REGS_MASK;
    if backend.tdcall_saved_ret(TDG_VP_VMCALL, args) != 0 {
        tdx_hypercall_failed();
    }
    args.r10
}

#[cfg(test)]
mod tests {
    use super::*;

    struct AcceptAll {
        calls: usize,
    }

    impl TdxAccept for AcceptAll {
        fn tdcall_accept(&mut self, _args: &mut TdxModuleArgs) -> u64 {
            self.calls += 1;
            0
        }
    }

    #[test]
    fn accept_memory_prefers_large_aligned_pages() {
        let mut backend = AcceptAll { calls: 0 };
        assert!(tdx_accept_memory(
            &mut backend,
            0,
            1024 * 1024 * 1024 + 4096
        ));
        assert_eq!(backend.calls, 2);
    }

    #[test]
    fn hypercall_sets_exposed_register_mask() {
        struct H;
        impl TdxHypercall for H {
            fn tdcall_saved_ret(&mut self, leaf: u64, args: &mut TdxModuleArgs) -> u64 {
                assert_eq!(leaf, TDG_VP_VMCALL);
                assert_eq!(args.rcx, TDVMCALL_EXPOSE_REGS_MASK);
                args.r10 = 17;
                0
            }
        }
        let mut args = TdxModuleArgs::default();
        assert_eq!(tdx_hypercall(&mut H, &mut args), 17);
    }

    #[test]
    #[should_panic(expected = "TDVMCALL failed. TDX module bug?")]
    fn hypercall_mechanism_failure_is_linux_fatal() {
        struct FailingMechanism;
        impl TdxHypercall for FailingMechanism {
            fn tdcall_saved_ret(&mut self, leaf: u64, args: &mut TdxModuleArgs) -> u64 {
                assert_eq!(leaf, TDG_VP_VMCALL);
                assert_eq!(args.rcx, TDVMCALL_EXPOSE_REGS_MASK);
                1
            }
        }

        let mut args = TdxModuleArgs::default();
        let _ = tdx_hypercall(&mut FailingMechanism, &mut args);
    }
}
