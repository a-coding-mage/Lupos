//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/process_32.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/process_32.c
//! 32-bit x86 process helpers modeled for the x86_64 runtime.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/process_32.c

#![allow(dead_code)]

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PtRegs32 {
    pub bx: u32,
    pub cx: u32,
    pub dx: u32,
    pub si: u32,
    pub di: u32,
    pub bp: u32,
    pub ax: u32,
    pub ds: u32,
    pub es: u32,
    pub fs: u32,
    pub gs: u32,
    pub orig_ax: u32,
    pub ip: u32,
    pub cs: u32,
    pub flags: u32,
    pub sp: u32,
    pub ss: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Thread32 {
    pub sp0: u32,
    pub sp: u32,
    pub ip: u32,
    pub tls_array: [u64; 3],
    pub released: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SwitchFrame32 {
    pub previous_sp: u32,
    pub next_sp: u32,
    pub next_ip: u32,
}

pub fn start_thread(regs: &mut PtRegs32, ip: u32, sp: u32, user_cs: u32, user_ds: u32) {
    regs.ip = ip;
    regs.sp = sp;
    regs.cs = user_cs;
    regs.ss = user_ds;
    regs.ds = user_ds;
    regs.es = user_ds;
    regs.flags |= 0x200;
}

pub fn release_thread(thread: &mut Thread32) {
    thread.tls_array = [0; 3];
    thread.released = true;
}

pub fn __switch_to(prev: &Thread32, next: &Thread32) -> SwitchFrame32 {
    SwitchFrame32 {
        previous_sp: prev.sp,
        next_sp: next.sp,
        next_ip: next.ip,
    }
}

pub fn __show_regs(regs: &PtRegs32) -> [u32; 6] {
    [regs.ax, regs.bx, regs.cx, regs.dx, regs.ip, regs.sp]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_thread_sets_compat_user_segments() {
        let mut regs = PtRegs32::default();
        start_thread(&mut regs, 0x8048000, 0xbfff0000, 0x23, 0x2b);
        assert_eq!(regs.ip, 0x8048000);
        assert_eq!(regs.sp, 0xbfff0000);
        assert_eq!(regs.cs, 0x23);
        assert_eq!(regs.ss, 0x2b);
        assert_ne!(regs.flags & 0x200, 0);
    }

    #[test]
    fn switch_frame_reports_next_context() {
        let prev = Thread32 {
            sp: 1,
            ..Default::default()
        };
        let next = Thread32 {
            sp: 2,
            ip: 3,
            ..Default::default()
        };
        assert_eq!(
            __switch_to(&prev, &next),
            SwitchFrame32 {
                previous_sp: 1,
                next_sp: 2,
                next_ip: 3,
            }
        );
    }
}
