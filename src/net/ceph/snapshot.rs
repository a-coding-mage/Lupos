//! linux-parity: complete
//! linux-source: vendor/linux/net/ceph/snapshot.c
//! test-origin: linux:vendor/linux/net/ceph/snapshot.c
//! Ceph snapshot context reference counting.

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::ENOMEM;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CephSnapContext {
    pub nref: usize,
    pub num_snaps: u32,
    pub seq: u64,
    pub snaps: Vec<u64>,
}

pub fn ceph_create_snap_context(snap_count: u32) -> Result<CephSnapContext, i32> {
    let mut snaps = Vec::new();
    snaps
        .try_reserve_exact(snap_count as usize)
        .map_err(|_| ENOMEM)?;
    snaps.resize(snap_count as usize, 0);
    Ok(CephSnapContext {
        nref: 1,
        num_snaps: snap_count,
        seq: 0,
        snaps,
    })
}

pub fn ceph_get_snap_context(sc: Option<&mut CephSnapContext>) -> Option<&mut CephSnapContext> {
    sc.map(|snapc| {
        snapc.nref = snapc.nref.saturating_add(1);
        snapc
    })
}

pub fn ceph_put_snap_context(sc: &mut Option<CephSnapContext>) {
    let Some(snapc) = sc.as_mut() else {
        return;
    };
    snapc.nref = snapc.nref.saturating_sub(1);
    if snapc.nref == 0 {
        *sc = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ceph_snapshot_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ceph/snapshot.c"
        ));
        assert!(source.contains("struct ceph_snap_context *ceph_create_snap_context"));
        assert!(source.contains("size += snap_count * sizeof (snapc->snaps[0]);"));
        assert!(source.contains("snapc = kzalloc(size, gfp_flags);"));
        assert!(source.contains("refcount_set(&snapc->nref, 1);"));
        assert!(source.contains("snapc->num_snaps = snap_count;"));
        assert!(source.contains("refcount_inc(&sc->nref);"));
        assert!(source.contains("refcount_dec_and_test(&sc->nref)"));
        assert!(source.contains("kfree(sc);"));

        let mut snapc = ceph_create_snap_context(3).unwrap();
        assert_eq!(snapc.nref, 1);
        assert_eq!(snapc.num_snaps, 3);
        assert_eq!(snapc.snaps, alloc::vec![0, 0, 0]);
        assert!(ceph_get_snap_context(Some(&mut snapc)).is_some());
        assert_eq!(snapc.nref, 2);
        let mut owned = Some(snapc);
        ceph_put_snap_context(&mut owned);
        assert_eq!(owned.as_ref().unwrap().nref, 1);
        ceph_put_snap_context(&mut owned);
        assert!(owned.is_none());
        ceph_put_snap_context(&mut owned);
    }
}
