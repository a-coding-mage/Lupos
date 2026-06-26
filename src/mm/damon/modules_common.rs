//! linux-parity: complete
//! linux-source: vendor/linux/mm/damon/modules-common.c
//! test-origin: linux:vendor/linux/mm/damon/modules-common.c
//! Common DAMON module context setup helpers.

use crate::include::uapi::errno::EINVAL;

use super::{DamonCtx, DamonOps};

pub const DAMON_MODULES_PADDR_TARGET_ID: u64 = 0;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DamonModulesPaddrSetup {
    pub ctx: DamonCtx,
    pub target_id: u64,
}

pub fn damon_modules_new_paddr_ctx_target() -> Result<DamonModulesPaddrSetup, i32> {
    let mut ctx = DamonCtx::new();
    ctx.set_ops(DamonOps::Paddr);
    ctx.add_target(DAMON_MODULES_PADDR_TARGET_ID)
        .map_err(|_| -EINVAL)?;
    Ok(DamonModulesPaddrSetup {
        ctx,
        target_id: DAMON_MODULES_PADDR_TARGET_ID,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modules_common_allocates_paddr_ctx_and_target_like_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/damon/modules-common.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/damon/modules-common.h"
        ));
        assert!(source.contains("ctx = damon_new_ctx();"));
        assert!(source.contains("damon_select_ops(ctx, DAMON_OPS_PADDR)"));
        assert!(source.contains("target = damon_new_target();"));
        assert!(source.contains("damon_add_target(ctx, target);"));
        assert!(header.contains("DEFINE_DAMON_MODULES_MON_ATTRS_PARAMS"));
        assert!(header.contains("DEFINE_DAMON_MODULES_DAMOS_QUOTAS"));

        let setup = damon_modules_new_paddr_ctx_target().expect("paddr setup");
        assert_eq!(setup.ctx.ops(), DamonOps::Paddr);
        assert_eq!(setup.ctx.stats().targets, 1);
        assert_eq!(setup.target_id, DAMON_MODULES_PADDR_TARGET_ID);
    }
}
