//! linux-parity: complete
//! linux-source: vendor/linux/block/blk-cgroup-fc-appid.c
//! test-origin: linux:vendor/linux/block/blk-cgroup-fc-appid.c
//! Fibre Channel application id helpers for block cgroups.

use alloc::vec::Vec;

use crate::include::uapi::errno::{EINVAL, ENOENT};
use crate::kernel::module::{export_symbol, find_symbol};

pub const FC_APPID_LEN: usize = 129;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlkCg {
    fc_app_id: [u8; FC_APPID_LEN],
}

impl BlkCg {
    pub const fn new() -> Self {
        Self {
            fc_app_id: [0; FC_APPID_LEN],
        }
    }

    pub fn fc_appid(&self) -> Option<&[u8]> {
        if self.fc_app_id[0] == 0 {
            return None;
        }
        let len = self
            .fc_app_id
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(FC_APPID_LEN);
        Some(&self.fc_app_id[..len])
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cgroup {
    pub id: u64,
    blkcg: Option<BlkCg>,
    pub cgroup_puts: u32,
    pub css_puts: u32,
}

impl Cgroup {
    pub fn with_blkcg(id: u64, blkcg: BlkCg) -> Self {
        Self {
            id,
            blkcg: Some(blkcg),
            cgroup_puts: 0,
            css_puts: 0,
        }
    }

    pub fn without_css(id: u64) -> Self {
        Self {
            id,
            blkcg: None,
            cgroup_puts: 0,
            css_puts: 0,
        }
    }

    pub fn blkcg(&self) -> Option<&BlkCg> {
        self.blkcg.as_ref()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BlkCgRegistry {
    cgroups: Vec<Cgroup>,
    lookup_error: Option<i32>,
}

impl BlkCgRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, cgroup: Cgroup) {
        self.cgroups.push(cgroup);
    }

    pub fn set_lookup_error(&mut self, err: i32) {
        self.lookup_error = Some(err);
    }

    pub fn cgroup(&self, id: u64) -> Option<&Cgroup> {
        self.cgroups.iter().find(|cgroup| cgroup.id == id)
    }

    fn cgroup_get_from_id(&mut self, id: u64) -> Result<&mut Cgroup, i32> {
        if let Some(err) = self.lookup_error {
            return Err(err);
        }
        self.cgroups
            .iter_mut()
            .find(|cgroup| cgroup.id == id)
            .ok_or(ENOENT)
    }
}

impl Default for BlkCg {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BlkCgGq<'a> {
    pub blkcg: &'a BlkCg,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BioBlkCg<'a> {
    pub bi_blkg: Option<BlkCgGq<'a>>,
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "blkcg_set_fc_appid",
        blkcg_set_fc_appid_export_symbol as usize,
        true,
    );
    export_symbol_once(
        "blkcg_get_fc_appid",
        blkcg_get_fc_appid_export_symbol as usize,
        true,
    );
}

fn strscpy_fc_appid(dst: &mut [u8; FC_APPID_LEN], src: &[u8], count: usize) {
    if count == 0 {
        return;
    }

    let limit = count.min(FC_APPID_LEN);
    let src_nul = src.iter().position(|byte| *byte == 0).unwrap_or(src.len());
    let copy_len = src_nul.min(limit.saturating_sub(1));
    dst[..copy_len].copy_from_slice(&src[..copy_len]);
    dst[copy_len] = 0;
}

pub fn blkcg_set_fc_appid(
    registry: &mut BlkCgRegistry,
    app_id: &[u8],
    cgrp_id: u64,
    app_id_len: usize,
) -> Result<(), i32> {
    if app_id_len > FC_APPID_LEN {
        return Err(EINVAL);
    }

    let cgrp = registry.cgroup_get_from_id(cgrp_id)?;
    if cgrp.blkcg.is_none() {
        cgrp.cgroup_puts += 1;
        return Err(ENOENT);
    }
    let blkcg = cgrp.blkcg.as_mut().expect("checked above");
    strscpy_fc_appid(&mut blkcg.fc_app_id, app_id, app_id_len);
    cgrp.css_puts += 1;
    cgrp.cgroup_puts += 1;
    Ok(())
}

pub fn blkcg_get_fc_appid<'a>(bio: &'a BioBlkCg<'a>) -> Option<&'a [u8]> {
    bio.bi_blkg.and_then(|blkg| blkg.blkcg.fc_appid())
}

fn blkcg_set_fc_appid_export_symbol(
    registry: &mut BlkCgRegistry,
    app_id: &[u8],
    cgrp_id: u64,
    app_id_len: usize,
) -> Result<(), i32> {
    blkcg_set_fc_appid(registry, app_id, cgrp_id, app_id_len)
}

fn blkcg_get_fc_appid_export_symbol<'a>(bio: &'a BioBlkCg<'a>) -> Option<&'a [u8]> {
    blkcg_get_fc_appid(bio)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fc_appid_helpers_match_linux_source_shape() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/block/blk-cgroup-fc-appid.c"
        ));
        assert!(source.contains("if (app_id_len > FC_APPID_LEN)"));
        assert!(source.contains("cgrp = cgroup_get_from_id(cgrp_id);"));
        assert!(source.contains("css = cgroup_get_e_css(cgrp, &io_cgrp_subsys);"));
        assert!(source.contains("blkcg = css_to_blkcg(css);"));
        assert!(source.contains("strscpy(blkcg->fc_app_id, app_id, app_id_len);"));
        assert!(source.contains("css_put(css);"));
        assert!(source.contains("cgroup_put(cgrp);"));
        assert!(source.contains("!bio->bi_blkg || bio->bi_blkg->blkcg->fc_app_id[0] == '\\0'"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(blkcg_set_fc_appid);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(blkcg_get_fc_appid);"));

        let mut blkcg = BlkCg::new();
        assert_eq!(blkcg_get_fc_appid(&BioBlkCg::default()), None);

        let mut registry = BlkCgRegistry::new();
        registry.push(Cgroup::with_blkcg(42, blkcg.clone()));
        blkcg_set_fc_appid(&mut registry, b"database-writer", 42, 16).expect("set appid");
        blkcg = registry.cgroup(42).unwrap().blkcg().unwrap().clone();
        let blkg = BlkCgGq { blkcg: &blkcg };
        let bio = BioBlkCg {
            bi_blkg: Some(blkg),
        };
        assert_eq!(blkcg_get_fc_appid(&bio), Some(&b"database-writer"[..]));

        assert_eq!(
            blkcg_set_fc_appid(&mut registry, b"too-long", 42, FC_APPID_LEN + 1),
            Err(EINVAL)
        );
    }

    #[test]
    fn fc_appid_copy_is_bounded_by_supplied_length() {
        let mut registry = BlkCgRegistry::new();
        registry.push(Cgroup::with_blkcg(7, BlkCg::new()));
        blkcg_set_fc_appid(&mut registry, b"abcdef", 7, 4).expect("bounded copy");
        assert_eq!(
            registry.cgroup(7).unwrap().blkcg().unwrap().fc_appid(),
            Some(&b"abc"[..])
        );
    }

    #[test]
    fn set_fc_appid_follows_cgroup_error_and_put_paths() {
        let mut missing_css = BlkCgRegistry::new();
        missing_css.push(Cgroup::without_css(11));
        assert_eq!(
            blkcg_set_fc_appid(&mut missing_css, b"writer", 11, 7),
            Err(ENOENT)
        );
        assert_eq!(missing_css.cgroup(11).unwrap().cgroup_puts, 1);
        assert_eq!(missing_css.cgroup(11).unwrap().css_puts, 0);

        let mut lookup_error = BlkCgRegistry::new();
        lookup_error.set_lookup_error(-123);
        assert_eq!(
            blkcg_set_fc_appid(&mut lookup_error, b"writer", 11, 7),
            Err(-123)
        );
    }

    #[test]
    fn exports_register_gpl_symbols() {
        register_module_exports();
        assert!(crate::kernel::module::find_symbol("blkcg_set_fc_appid").is_some());
        assert!(crate::kernel::module::find_symbol("blkcg_get_fc_appid").is_some());
    }
}
