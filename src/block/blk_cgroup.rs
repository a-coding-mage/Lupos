//! linux-parity: partial
//! linux-source: vendor/linux/block/blk-cgroup.c
//! test-origin: linux:vendor/linux/block/blk-cgroup.c
//! Block-cgroup policy, iostat, and delay accounting.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use crate::include::uapi::errno::{EINVAL, ENODEV, ENOSPC, EOPNOTSUPP};

pub const BLKCG_MAX_POLS: usize = 6;
pub const BLKG_DESTROY_BATCH_SIZE: usize = 64;
pub const BLKG_STAT_CPU_BATCH: i32 = i32::MAX / 2;
pub const NSEC_PER_SEC: u64 = 1_000_000_000;
pub const NSEC_PER_MSEC: u64 = 1_000_000;
pub const BLKCG_MAX_DELAY_NS: u64 = 250 * NSEC_PER_MSEC;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(usize)]
pub enum BlkgIostatType {
    Read = 0,
    Write = 1,
    Discard = 2,
}

pub const BLKG_IOSTAT_NR: usize = 3;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BlkgIostat {
    pub bytes: [u64; BLKG_IOSTAT_NR],
    pub ios: [u64; BLKG_IOSTAT_NR],
}

impl BlkgIostat {
    pub fn add(&mut self, other: &Self) {
        for idx in 0..BLKG_IOSTAT_NR {
            self.bytes[idx] = self.bytes[idx].saturating_add(other.bytes[idx]);
            self.ios[idx] = self.ios[idx].saturating_add(other.ios[idx]);
        }
    }

    pub fn sub(&mut self, other: &Self) {
        for idx in 0..BLKG_IOSTAT_NR {
            self.bytes[idx] = self.bytes[idx].saturating_sub(other.bytes[idx]);
            self.ios[idx] = self.ios[idx].saturating_sub(other.ios[idx]);
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BlkgIostatSet {
    pub cur: BlkgIostat,
    pub last: BlkgIostat,
    pub lqueued: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlkcgPolicy {
    pub plid: Option<usize>,
    pub has_cpd: bool,
    pub has_pd: bool,
    pub dfl_cftypes: bool,
    pub legacy_cftypes: bool,
}

impl BlkcgPolicy {
    pub const fn new(has_cpd: bool, has_pd: bool) -> Self {
        Self {
            plid: None,
            has_cpd,
            has_pd,
            dfl_cftypes: false,
            legacy_cftypes: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Blkcg {
    pub id: usize,
    pub parent: Option<usize>,
    pub congestion_count: i32,
    pub online_pin: u32,
}

impl Blkcg {
    pub const fn root() -> Self {
        Self {
            id: 0,
            parent: None,
            congestion_count: 0,
            online_pin: 1,
        }
    }

    pub const fn child(id: usize, parent: usize) -> Self {
        Self {
            id,
            parent: Some(parent),
            congestion_count: 0,
            online_pin: 1,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlkcgGq {
    pub blkcg_id: usize,
    pub parent: Option<usize>,
    pub online: bool,
    pub iostat_cpu: BlkgIostatSet,
    pub iostat: BlkgIostatSet,
    pub policy_data: [bool; BLKCG_MAX_POLS],
    pub use_delay: i32,
    pub delay_nsec: u64,
    pub delay_start: u64,
    pub last_delay: u64,
    pub last_use: i32,
    pub device_name: Option<String>,
}

impl BlkcgGq {
    pub fn new(blkcg_id: usize, parent: Option<usize>, device_name: Option<String>) -> Self {
        Self {
            blkcg_id,
            parent,
            online: true,
            iostat_cpu: BlkgIostatSet::default(),
            iostat: BlkgIostatSet::default(),
            policy_data: [false; BLKCG_MAX_POLS],
            use_delay: 0,
            delay_nsec: 0,
            delay_start: 0,
            last_delay: 0,
            last_use: 0,
            device_name,
        }
    }

    pub fn dev_name(&self) -> Option<&str> {
        self.device_name.as_deref()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BlkcgRegistry {
    policies: [bool; BLKCG_MAX_POLS],
}

impl BlkcgRegistry {
    pub fn policy_register(&mut self, pol: &mut BlkcgPolicy) -> Result<usize, i32> {
        if pol.has_cpd ^ pol.has_pd {
            return Err(-EINVAL);
        }
        let Some(index) = self.policies.iter().position(|present| !*present) else {
            return Err(-ENOSPC);
        };
        self.policies[index] = true;
        pol.plid = Some(index);
        Ok(index)
    }

    pub fn policy_unregister(&mut self, pol: &mut BlkcgPolicy) {
        if let Some(index) = pol.plid.take() {
            self.policies[index] = false;
        }
    }

    pub fn is_registered(&self, pol: &BlkcgPolicy) -> bool {
        pol.plid.is_some_and(|index| self.policies[index])
    }
}

pub fn blkcg_activate_policy(blkg: &mut BlkcgGq, pol: &BlkcgPolicy) -> Result<(), i32> {
    let Some(plid) = pol.plid else {
        return Err(-EINVAL);
    };
    if !pol.has_pd {
        return Err(-EINVAL);
    }
    blkg.policy_data[plid] = true;
    Ok(())
}

pub fn blkcg_deactivate_policy(blkg: &mut BlkcgGq, pol: &BlkcgPolicy) {
    if let Some(plid) = pol.plid {
        blkg.policy_data[plid] = false;
    }
}

pub fn blkcg_policy_enabled(blkg: &BlkcgGq, pol: &BlkcgPolicy) -> bool {
    pol.plid.is_some_and(|plid| blkg.policy_data[plid])
}

pub fn blkg_iostat_update(blkg: &mut BlkcgGq, cur: BlkgIostat) {
    let mut delta = cur;
    delta.sub(&blkg.iostat_cpu.last);
    blkg.iostat.cur.add(&delta);
    blkg.iostat_cpu.last.add(&delta);
}

pub fn blk_cgroup_bio_start(blkg: &mut BlkcgGq, io_type: BlkgIostatType, bytes: u64) {
    let idx = io_type as usize;
    blkg.iostat_cpu.cur.bytes[idx] = blkg.iostat_cpu.cur.bytes[idx].saturating_add(bytes);
    blkg.iostat_cpu.cur.ios[idx] = blkg.iostat_cpu.cur.ios[idx].saturating_add(1);
    blkg.iostat_cpu.lqueued = true;
}

pub fn blkcg_rstat_flush(blkg: &mut BlkcgGq, parent: Option<&mut BlkcgGq>) {
    if !blkg.iostat_cpu.lqueued {
        return;
    }
    blkg.iostat_cpu.lqueued = false;
    let cur = blkg.iostat_cpu.cur;
    blkg_iostat_update(blkg, cur);
    if let Some(parent) = parent {
        let mut delta = blkg.iostat.cur;
        delta.sub(&blkg.iostat.last);
        parent.iostat.cur.add(&delta);
        blkg.iostat.last.add(&delta);
        parent.iostat.lqueued = true;
    }
}

pub fn blkg_dev_name(blkg: &BlkcgGq) -> Option<&str> {
    blkg.dev_name()
}

pub fn __blkg_prfill_u64(blkg: &BlkcgGq, value: u64) -> (String, u64) {
    let Some(name) = blkg_dev_name(blkg) else {
        return (String::new(), 0);
    };
    use core::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(&mut out, "{name} {value}");
    (out, value)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlkgConfCtx<'a> {
    pub input: &'a str,
    pub body: Option<&'a str>,
    pub dev: Option<(u32, u32)>,
    pub prepared: bool,
}

pub fn blkg_conf_init(input: &str) -> BlkgConfCtx<'_> {
    BlkgConfCtx {
        input,
        body: None,
        dev: None,
        prepared: false,
    }
}

pub fn blkg_conf_open_bdev(ctx: &mut BlkgConfCtx<'_>) -> Result<(), i32> {
    if ctx.dev.is_some() {
        return Ok(());
    }
    let input = ctx.input.trim_start();
    let Some((dev, body)) = input.split_once(char::is_whitespace) else {
        return Err(-EINVAL);
    };
    let Some((major, minor)) = dev.split_once(':') else {
        return Err(-EINVAL);
    };
    let major = major.parse::<u32>().map_err(|_| -EINVAL)?;
    let minor = minor.parse::<u32>().map_err(|_| -EINVAL)?;
    if body.trim().is_empty() {
        return Err(-EINVAL);
    }
    ctx.dev = Some((major, minor));
    ctx.body = Some(body.trim_start());
    Ok(())
}

pub fn blkg_conf_prep(
    ctx: &mut BlkgConfCtx<'_>,
    blkg: &BlkcgGq,
    pol: &BlkcgPolicy,
) -> Result<(), i32> {
    blkg_conf_open_bdev(ctx)?;
    if !blkcg_policy_enabled(blkg, pol) {
        return Err(-EOPNOTSUPP);
    }
    ctx.prepared = true;
    Ok(())
}

pub fn blkg_conf_exit(ctx: &mut BlkgConfCtx<'_>) {
    ctx.prepared = false;
    ctx.body = None;
    ctx.dev = None;
}

pub fn blkcg_use_delay(blkcg: &mut Blkcg, blkg: &mut BlkcgGq) {
    if blkg.use_delay < 0 {
        return;
    }
    blkg.use_delay += 1;
    if blkg.use_delay == 1 {
        blkcg.congestion_count += 1;
    }
}

pub fn blkcg_unuse_delay(blkcg: &mut Blkcg, blkg: &mut BlkcgGq) -> bool {
    if blkg.use_delay <= 0 {
        return false;
    }
    blkg.use_delay -= 1;
    if blkg.use_delay == 0 {
        blkcg.congestion_count -= 1;
    }
    true
}

pub fn blkcg_set_delay(blkcg: &mut Blkcg, blkg: &mut BlkcgGq, delay: u64) {
    if blkg.use_delay == 0 {
        blkcg.congestion_count += 1;
    }
    blkg.use_delay = -1;
    blkg.delay_nsec = delay;
}

pub fn blkcg_clear_delay(blkcg: &mut Blkcg, blkg: &mut BlkcgGq) {
    if blkg.use_delay != 0 {
        blkcg.congestion_count -= 1;
    }
    blkg.use_delay = 0;
    blkg.delay_nsec = 0;
}

pub fn blkcg_scale_delay(blkg: &mut BlkcgGq, now: u64) {
    if blkg.use_delay < 0 {
        return;
    }
    if blkg.delay_start + NSEC_PER_SEC < now {
        let cur = blkg.delay_nsec;
        let mut sub = blkg.last_delay.min(now - blkg.delay_start);
        if blkg.use_delay < blkg.last_use {
            sub = sub.max(blkg.last_delay >> 1);
        }
        blkg.delay_nsec = cur.saturating_sub(sub);
        blkg.last_delay = blkg.delay_nsec;
        blkg.last_use = blkg.use_delay;
        blkg.delay_start = now;
    }
}

pub fn blkcg_add_delay(blkg: &mut BlkcgGq, now: u64, delta: u64) {
    if blkg.use_delay < 0 {
        return;
    }
    blkcg_scale_delay(blkg, now);
    blkg.delay_nsec = blkg.delay_nsec.saturating_add(delta);
    blkg.last_delay = blkg.delay_nsec;
}

pub fn blkcg_delay_to_throttle(blkg: &mut BlkcgGq, now: u64, clamp: bool) -> u64 {
    blkcg_scale_delay(blkg, now);
    if clamp {
        blkg.delay_nsec.min(BLKCG_MAX_DELAY_NS)
    } else {
        blkg.delay_nsec
    }
}

pub fn blkcg_pin_online(blkcg: &mut Blkcg) {
    blkcg.online_pin = blkcg.online_pin.saturating_add(1);
}

pub fn blkcg_unpin_online(blkcg: &mut Blkcg) {
    blkcg.online_pin = blkcg.online_pin.saturating_sub(1);
}

pub fn bio_issue_as_root_blkg(op_flags: u64) -> bool {
    const REQ_META: u64 = 1 << 12;
    const REQ_SWAP: u64 = 1 << 24;
    op_flags & (REQ_META | REQ_SWAP) != 0
}

pub fn require_live_bdev(live: bool) -> Result<(), i32> {
    if live { Ok(()) } else { Err(-ENODEV) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_registration_and_activation_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/block/blk-cgroup.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/block/blk-cgroup.h"
        ));
        assert!(source.contains("blkcg_policy_register(struct blkcg_policy *pol)"));
        assert!(source.contains("(!pol->cpd_alloc_fn ^ !pol->cpd_free_fn)"));
        assert!(source.contains("blkcg_activate_policy(struct gendisk *disk"));
        assert!(source.contains("blkcg_deactivate_policy(struct gendisk *disk"));
        assert!(header.contains("#define BLKG_STAT_CPU_BATCH"));
        assert!(header.contains("enum blkg_iostat_type"));

        let mut registry = BlkcgRegistry::default();
        let mut policy = BlkcgPolicy::new(true, true);
        assert_eq!(registry.policy_register(&mut policy), Ok(0));
        assert!(registry.is_registered(&policy));
        let mut blkg = BlkcgGq::new(1, Some(0), Some(String::from("vda")));
        blkcg_activate_policy(&mut blkg, &policy).unwrap();
        assert!(blkcg_policy_enabled(&blkg, &policy));
        blkcg_deactivate_policy(&mut blkg, &policy);
        assert!(!blkcg_policy_enabled(&blkg, &policy));
        registry.policy_unregister(&mut policy);
        assert!(!registry.is_registered(&policy));
    }

    #[test]
    fn iostat_flush_and_prfill_follow_blkcg_stat_shape() {
        let mut parent = BlkcgGq::new(0, None, Some(String::from("vda")));
        let mut child = BlkcgGq::new(1, Some(0), Some(String::from("vda")));
        blk_cgroup_bio_start(&mut child, BlkgIostatType::Read, 4096);
        blk_cgroup_bio_start(&mut child, BlkgIostatType::Write, 512);
        blkcg_rstat_flush(&mut child, Some(&mut parent));
        assert_eq!(child.iostat.cur.bytes[BlkgIostatType::Read as usize], 4096);
        assert_eq!(child.iostat.cur.ios[BlkgIostatType::Write as usize], 1);
        assert_eq!(parent.iostat.cur.bytes[BlkgIostatType::Read as usize], 4096);

        let (printed, value) = __blkg_prfill_u64(&child, 17);
        assert_eq!(value, 17);
        assert_eq!(printed, "vda 17\n");
    }

    #[test]
    fn config_parse_and_delay_accounting_are_source_backed() {
        let mut registry = BlkcgRegistry::default();
        let mut policy = BlkcgPolicy::new(true, true);
        registry.policy_register(&mut policy).unwrap();
        let mut blkg = BlkcgGq::new(1, Some(0), Some(String::from("vdb")));
        blkcg_activate_policy(&mut blkg, &policy).unwrap();

        let mut ctx = blkg_conf_init("8:16 weight=200");
        blkg_conf_prep(&mut ctx, &blkg, &policy).unwrap();
        assert_eq!(ctx.dev, Some((8, 16)));
        assert_eq!(ctx.body, Some("weight=200"));
        blkg_conf_exit(&mut ctx);
        assert!(!ctx.prepared);

        let mut blkcg = Blkcg::child(1, 0);
        blkcg_use_delay(&mut blkcg, &mut blkg);
        blkcg_add_delay(&mut blkg, 1, 2 * NSEC_PER_SEC);
        assert_eq!(
            blkcg_delay_to_throttle(&mut blkg, NSEC_PER_SEC + 2, true),
            BLKCG_MAX_DELAY_NS
        );
        assert!(blkcg_unuse_delay(&mut blkcg, &mut blkg));
        blkcg_set_delay(&mut blkcg, &mut blkg, 7);
        blkcg_add_delay(&mut blkg, 10, 99);
        assert_eq!(blkg.delay_nsec, 7);
        blkcg_clear_delay(&mut blkcg, &mut blkg);
        assert_eq!(blkg.use_delay, 0);
    }
}
