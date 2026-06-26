//! linux-parity: complete
//! linux-source: vendor/linux/include/uapi/linux/perf_event.h
//! test-origin: linux:vendor/linux/include/uapi/linux/perf_event.h
//! `struct perf_event_attr` UAPI layout.

pub const PERF_ATTR_SIZE_VER0: u32 = 64;
pub const PERF_ATTR_SIZE_VER1: u32 = 72;
pub const PERF_ATTR_SIZE_VER2: u32 = 80;
pub const PERF_ATTR_SIZE_VER3: u32 = 96;
pub const PERF_ATTR_SIZE_VER4: u32 = 104;
pub const PERF_ATTR_SIZE_VER5: u32 = 112;
pub const PERF_ATTR_SIZE_VER6: u32 = 120;
pub const PERF_ATTR_SIZE_VER7: u32 = 128;
pub const PERF_ATTR_SIZE_VER8: u32 = 136;
pub const PERF_ATTR_SIZE_VER9: u32 = 144;
pub const PERF_ATTR_SIZE_LATEST: u32 = PERF_ATTR_SIZE_VER9;

pub const PERF_ATTR_DISABLED: u64 = 1 << 0;
pub const PERF_ATTR_INHERIT: u64 = 1 << 1;
pub const PERF_ATTR_PINNED: u64 = 1 << 2;
pub const PERF_ATTR_EXCLUSIVE: u64 = 1 << 3;
pub const PERF_ATTR_EXCLUDE_USER: u64 = 1 << 4;
pub const PERF_ATTR_EXCLUDE_KERNEL: u64 = 1 << 5;
pub const PERF_ATTR_EXCLUDE_HV: u64 = 1 << 6;
pub const PERF_ATTR_EXCLUDE_IDLE: u64 = 1 << 7;
pub const PERF_ATTR_MMAP: u64 = 1 << 8;
pub const PERF_ATTR_COMM: u64 = 1 << 9;
pub const PERF_ATTR_FREQ: u64 = 1 << 10;
pub const PERF_ATTR_INHERIT_STAT: u64 = 1 << 11;
pub const PERF_ATTR_ENABLE_ON_EXEC: u64 = 1 << 12;
pub const PERF_ATTR_TASK: u64 = 1 << 13;
pub const PERF_ATTR_WATERMARK: u64 = 1 << 14;
pub const PERF_ATTR_PRECISE_IP_SHIFT: u32 = 15;
pub const PERF_ATTR_PRECISE_IP_MASK: u64 = 0b11 << PERF_ATTR_PRECISE_IP_SHIFT;
pub const PERF_ATTR_MMAP_DATA: u64 = 1 << 17;
pub const PERF_ATTR_SAMPLE_ID_ALL: u64 = 1 << 18;
pub const PERF_ATTR_EXCLUDE_HOST: u64 = 1 << 19;
pub const PERF_ATTR_EXCLUDE_GUEST: u64 = 1 << 20;
pub const PERF_ATTR_EXCLUDE_CALLCHAIN_KERNEL: u64 = 1 << 21;
pub const PERF_ATTR_EXCLUDE_CALLCHAIN_USER: u64 = 1 << 22;
pub const PERF_ATTR_MMAP2: u64 = 1 << 23;
pub const PERF_ATTR_COMM_EXEC: u64 = 1 << 24;
pub const PERF_ATTR_USE_CLOCKID: u64 = 1 << 25;
pub const PERF_ATTR_CONTEXT_SWITCH: u64 = 1 << 26;
pub const PERF_ATTR_WRITE_BACKWARD: u64 = 1 << 27;
pub const PERF_ATTR_NAMESPACES: u64 = 1 << 28;
pub const PERF_ATTR_KSYMBOL: u64 = 1 << 29;
pub const PERF_ATTR_BPF_EVENT: u64 = 1 << 30;
pub const PERF_ATTR_AUX_OUTPUT: u64 = 1 << 31;
pub const PERF_ATTR_CGROUP: u64 = 1 << 32;
pub const PERF_ATTR_TEXT_POKE: u64 = 1 << 33;
pub const PERF_ATTR_BUILD_ID: u64 = 1 << 34;
pub const PERF_ATTR_INHERIT_THREAD: u64 = 1 << 35;
pub const PERF_ATTR_REMOVE_ON_EXEC: u64 = 1 << 36;
pub const PERF_ATTR_SIGTRAP: u64 = 1 << 37;
pub const PERF_ATTR_DEFER_CALLCHAIN: u64 = 1 << 38;
pub const PERF_ATTR_DEFER_OUTPUT: u64 = 1 << 39;
pub const PERF_ATTR_RESERVED_1_MASK: u64 = !((1u64 << 40) - 1);

pub const PERF_ATTR_AUX_START_PAUSED: u32 = 1 << 0;
pub const PERF_ATTR_AUX_PAUSE: u32 = 1 << 1;
pub const PERF_ATTR_AUX_RESUME: u32 = 1 << 2;
pub const PERF_ATTR_AUX_RESERVED_3_MASK: u32 = !0b111;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PerfEventAttr {
    pub type_: u32,
    pub size: u32,
    pub config: u64,
    /// Linux union: `sample_period` or `sample_freq`.
    pub sample_period: u64,
    pub sample_type: u64,
    pub read_format: u64,
    /// Linux bitfields from `disabled` through `defer_output`.
    pub flags: u64,
    /// Linux union: `wakeup_events` or `wakeup_watermark`.
    pub wakeup_events: u32,
    pub bp_type: u32,
    /// Linux union: `bp_addr`, `kprobe_func`, `uprobe_path`, or `config1`.
    pub bp_addr: u64,
    /// Linux union: `bp_len`, `kprobe_addr`, `probe_offset`, or `config2`.
    pub bp_len: u64,
    pub branch_sample_type: u64,
    pub sample_regs_user: u64,
    pub sample_stack_user: u32,
    pub clockid: i32,
    pub sample_regs_intr: u64,
    pub aux_watermark: u32,
    pub sample_max_stack: u16,
    pub _reserved_2: u16,
    pub aux_sample_size: u32,
    /// Linux union: raw `aux_action` or the AUX action bitfields.
    pub aux_action: u32,
    pub sig_data: u64,
    pub config3: u64,
    pub config4: u64,
}

impl PerfEventAttr {
    pub const fn sample_freq(&self) -> u64 {
        self.sample_period
    }

    pub fn set_sample_freq(&mut self, sample_freq: u64) {
        self.sample_period = sample_freq;
        self.flags |= PERF_ATTR_FREQ;
    }

    pub const fn wakeup_watermark(&self) -> u32 {
        self.wakeup_events
    }

    pub fn set_wakeup_watermark(&mut self, wakeup_watermark: u32) {
        self.wakeup_events = wakeup_watermark;
        self.flags |= PERF_ATTR_WATERMARK;
    }

    pub const fn kprobe_func(&self) -> u64 {
        self.bp_addr
    }

    pub const fn uprobe_path(&self) -> u64 {
        self.bp_addr
    }

    pub const fn config1(&self) -> u64 {
        self.bp_addr
    }

    pub const fn kprobe_addr(&self) -> u64 {
        self.bp_len
    }

    pub const fn probe_offset(&self) -> u64 {
        self.bp_len
    }

    pub const fn config2(&self) -> u64 {
        self.bp_len
    }

    pub const fn precise_ip(&self) -> u8 {
        ((self.flags & PERF_ATTR_PRECISE_IP_MASK) >> PERF_ATTR_PRECISE_IP_SHIFT) as u8
    }

    pub fn set_precise_ip(&mut self, precise_ip: u8) {
        let precise_ip = (precise_ip as u64) & 0b11;
        self.flags &= !PERF_ATTR_PRECISE_IP_MASK;
        self.flags |= precise_ip << PERF_ATTR_PRECISE_IP_SHIFT;
    }

    pub const fn aux_start_paused(&self) -> bool {
        self.aux_action & PERF_ATTR_AUX_START_PAUSED != 0
    }

    pub const fn aux_pause(&self) -> bool {
        self.aux_action & PERF_ATTR_AUX_PAUSE != 0
    }

    pub const fn aux_resume(&self) -> bool {
        self.aux_action & PERF_ATTR_AUX_RESUME != 0
    }

    pub fn set_aux_start_paused(&mut self, enabled: bool) {
        set_aux_flag(&mut self.aux_action, PERF_ATTR_AUX_START_PAUSED, enabled);
    }

    pub fn set_aux_pause(&mut self, enabled: bool) {
        set_aux_flag(&mut self.aux_action, PERF_ATTR_AUX_PAUSE, enabled);
    }

    pub fn set_aux_resume(&mut self, enabled: bool) {
        set_aux_flag(&mut self.aux_action, PERF_ATTR_AUX_RESUME, enabled);
    }
}

fn set_aux_flag(aux_action: &mut u32, flag: u32, enabled: bool) {
    if enabled {
        *aux_action |= flag;
    } else {
        *aux_action &= !flag;
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{offset_of, size_of};

    use super::*;

    #[test]
    fn attr_size_versions_match_linux_uapi() {
        assert_eq!(PERF_ATTR_SIZE_VER0, 64);
        assert_eq!(PERF_ATTR_SIZE_VER1, 72);
        assert_eq!(PERF_ATTR_SIZE_VER2, 80);
        assert_eq!(PERF_ATTR_SIZE_VER3, 96);
        assert_eq!(PERF_ATTR_SIZE_VER4, 104);
        assert_eq!(PERF_ATTR_SIZE_VER5, 112);
        assert_eq!(PERF_ATTR_SIZE_VER6, 120);
        assert_eq!(PERF_ATTR_SIZE_VER7, 128);
        assert_eq!(PERF_ATTR_SIZE_VER8, 136);
        assert_eq!(PERF_ATTR_SIZE_VER9, 144);
        assert_eq!(PERF_ATTR_SIZE_LATEST, size_of::<PerfEventAttr>() as u32);
    }

    #[test]
    fn attr_layout_matches_linux_uapi_offsets() {
        assert_eq!(size_of::<PerfEventAttr>(), 144);
        assert_eq!(offset_of!(PerfEventAttr, type_), 0);
        assert_eq!(offset_of!(PerfEventAttr, size), 4);
        assert_eq!(offset_of!(PerfEventAttr, config), 8);
        assert_eq!(offset_of!(PerfEventAttr, sample_period), 16);
        assert_eq!(offset_of!(PerfEventAttr, sample_type), 24);
        assert_eq!(offset_of!(PerfEventAttr, read_format), 32);
        assert_eq!(offset_of!(PerfEventAttr, flags), 40);
        assert_eq!(offset_of!(PerfEventAttr, wakeup_events), 48);
        assert_eq!(offset_of!(PerfEventAttr, bp_type), 52);
        assert_eq!(offset_of!(PerfEventAttr, bp_addr), 56);
        assert_eq!(offset_of!(PerfEventAttr, bp_len), 64);
        assert_eq!(offset_of!(PerfEventAttr, branch_sample_type), 72);
        assert_eq!(offset_of!(PerfEventAttr, sample_regs_user), 80);
        assert_eq!(offset_of!(PerfEventAttr, sample_stack_user), 88);
        assert_eq!(offset_of!(PerfEventAttr, clockid), 92);
        assert_eq!(offset_of!(PerfEventAttr, sample_regs_intr), 96);
        assert_eq!(offset_of!(PerfEventAttr, aux_watermark), 104);
        assert_eq!(offset_of!(PerfEventAttr, sample_max_stack), 108);
        assert_eq!(offset_of!(PerfEventAttr, _reserved_2), 110);
        assert_eq!(offset_of!(PerfEventAttr, aux_sample_size), 112);
        assert_eq!(offset_of!(PerfEventAttr, aux_action), 116);
        assert_eq!(offset_of!(PerfEventAttr, sig_data), 120);
        assert_eq!(offset_of!(PerfEventAttr, config3), 128);
        assert_eq!(offset_of!(PerfEventAttr, config4), 136);
    }

    #[test]
    fn attr_bit_constants_follow_linux_bitfield_order() {
        assert_eq!(PERF_ATTR_DISABLED, 1 << 0);
        assert_eq!(PERF_ATTR_WATERMARK, 1 << 14);
        assert_eq!(PERF_ATTR_PRECISE_IP_SHIFT, 15);
        assert_eq!(PERF_ATTR_PRECISE_IP_MASK, 0b11 << 15);
        assert_eq!(PERF_ATTR_MMAP_DATA, 1 << 17);
        assert_eq!(PERF_ATTR_DEFER_OUTPUT, 1 << 39);
        assert_eq!(PERF_ATTR_RESERVED_1_MASK, !((1u64 << 40) - 1));

        let mut attr = PerfEventAttr::default();
        attr.set_precise_ip(3);
        assert_eq!(attr.precise_ip(), 3);
        assert_eq!(attr.flags & PERF_ATTR_PRECISE_IP_MASK, 0b11 << 15);
        attr.set_precise_ip(4);
        assert_eq!(attr.precise_ip(), 0);
    }

    #[test]
    fn union_alias_helpers_share_linux_storage() {
        let mut attr = PerfEventAttr::default();
        attr.set_sample_freq(123);
        assert_eq!(attr.sample_period, 123);
        assert_eq!(attr.sample_freq(), 123);
        assert_ne!(attr.flags & PERF_ATTR_FREQ, 0);

        attr.set_wakeup_watermark(4096);
        assert_eq!(attr.wakeup_events, 4096);
        assert_eq!(attr.wakeup_watermark(), 4096);
        assert_ne!(attr.flags & PERF_ATTR_WATERMARK, 0);

        attr.bp_addr = 0xabc;
        attr.bp_len = 0xdef;
        assert_eq!(attr.kprobe_func(), 0xabc);
        assert_eq!(attr.uprobe_path(), 0xabc);
        assert_eq!(attr.config1(), 0xabc);
        assert_eq!(attr.kprobe_addr(), 0xdef);
        assert_eq!(attr.probe_offset(), 0xdef);
        assert_eq!(attr.config2(), 0xdef);
    }

    #[test]
    fn aux_action_bits_follow_linux_bitfield_order() {
        assert_eq!(PERF_ATTR_AUX_START_PAUSED, 1 << 0);
        assert_eq!(PERF_ATTR_AUX_PAUSE, 1 << 1);
        assert_eq!(PERF_ATTR_AUX_RESUME, 1 << 2);
        assert_eq!(PERF_ATTR_AUX_RESERVED_3_MASK, !0b111);

        let mut attr = PerfEventAttr::default();
        attr.set_aux_start_paused(true);
        attr.set_aux_pause(true);
        attr.set_aux_resume(true);
        assert!(attr.aux_start_paused());
        assert!(attr.aux_pause());
        assert!(attr.aux_resume());
        assert_eq!(attr.aux_action, 0b111);

        attr.set_aux_pause(false);
        assert!(attr.aux_start_paused());
        assert!(!attr.aux_pause());
        assert!(attr.aux_resume());
        assert_eq!(attr.aux_action, 0b101);
    }
}
