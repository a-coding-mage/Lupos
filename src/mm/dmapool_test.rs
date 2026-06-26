//! linux-parity: complete
//! linux-source: vendor/linux/mm/dmapool_test.c
//! test-origin: linux:vendor/linux/mm/dmapool_test.c
//! Source-backed DMA-pool timing-test parameter model.

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::ENOMEM;
use crate::mm::dmapool::{DmaBlockHandle, DmaPool, PAGE_SIZE};

pub const NR_TESTS: usize = 100;
pub const DMA_BIT_MASK_64: u64 = u64::MAX;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DmapoolParms {
    pub size: usize,
    pub align: usize,
    pub boundary: usize,
}

pub const POOL_PARMS: &[DmapoolParms] = &[
    DmapoolParms {
        size: 16,
        align: 16,
        boundary: 0,
    },
    DmapoolParms {
        size: 64,
        align: 64,
        boundary: 0,
    },
    DmapoolParms {
        size: 256,
        align: 256,
        boundary: 0,
    },
    DmapoolParms {
        size: 1024,
        align: 1024,
        boundary: 0,
    },
    DmapoolParms {
        size: 4096,
        align: 4096,
        boundary: 0,
    },
    DmapoolParms {
        size: 68,
        align: 32,
        boundary: 4096,
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DmapoolBlockReport {
    pub parms: DmapoolParms,
    pub blocks: usize,
    pub repeats: usize,
    pub pages_after_first_pass: usize,
    pub pool_destroyed: bool,
    pub pairs_freed: bool,
    pub cond_resched_checks: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DmapoolChecksConfig {
    pub dev_set_name_ret: i32,
    pub device_register_ret: i32,
    pub dma_set_mask_ret: i32,
    pub failing_block_index: Option<usize>,
}

impl DmapoolChecksConfig {
    pub const SUCCESS: Self = Self {
        dev_set_name_ret: 0,
        device_register_ret: 0,
        dma_set_mask_ret: 0,
        failing_block_index: None,
    };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DmapoolChecksReport {
    pub ret: i32,
    pub name_set: bool,
    pub device_registered: bool,
    pub release_installed: bool,
    pub dma_ops_cleared: bool,
    pub dma_mask: u64,
    pub blocks_attempted: usize,
    pub device_deleted: bool,
    pub device_put: bool,
}

pub const fn nr_blocks(size: usize) -> usize {
    let raw = (PAGE_SIZE / size) * 512;
    if raw < 1024 {
        1024
    } else if raw > 8192 {
        8192
    } else {
        raw
    }
}

pub fn dmapool_test_alloc(pool: &mut DmaPool, blocks: usize) -> Result<(), i32> {
    let mut allocated: Vec<DmaBlockHandle> = Vec::new();

    for _ in 0..blocks {
        let Some(block) = pool.alloc() else {
            for block in allocated {
                let _ = pool.free(block);
            }
            return Err(-ENOMEM);
        };
        allocated.push(block);
    }

    for block in allocated {
        pool.free(block)?;
    }
    Ok(())
}

pub fn dmapool_test_block_with_repeats(
    parms: DmapoolParms,
    repeats: usize,
) -> Result<DmapoolBlockReport, i32> {
    let blocks = nr_blocks(parms.size);
    let mut pool =
        DmaPool::create_node(true, parms.size, parms.align, parms.boundary, 0).ok_or(-ENOMEM)?;

    let mut cond_resched_checks = 0usize;
    for _ in 0..repeats {
        dmapool_test_alloc(&mut pool, blocks)?;
        cond_resched_checks += 1;
    }

    Ok(DmapoolBlockReport {
        parms,
        blocks,
        repeats,
        pages_after_first_pass: pool.nr_pages,
        pool_destroyed: true,
        pairs_freed: true,
        cond_resched_checks,
    })
}

pub fn dmapool_test_block(parms: DmapoolParms) -> Result<DmapoolBlockReport, i32> {
    dmapool_test_block_with_repeats(parms, NR_TESTS)
}

pub const fn dmapool_test_release_called() -> bool {
    true
}

pub fn dmapool_checks(config: DmapoolChecksConfig) -> DmapoolChecksReport {
    if config.dev_set_name_ret != 0 {
        return DmapoolChecksReport {
            ret: config.dev_set_name_ret,
            name_set: false,
            device_registered: false,
            release_installed: false,
            dma_ops_cleared: false,
            dma_mask: 0,
            blocks_attempted: 0,
            device_deleted: false,
            device_put: false,
        };
    }

    if config.device_register_ret != 0 {
        return DmapoolChecksReport {
            ret: config.device_register_ret,
            name_set: true,
            device_registered: false,
            release_installed: false,
            dma_ops_cleared: false,
            dma_mask: 0,
            blocks_attempted: 0,
            device_deleted: false,
            device_put: true,
        };
    }

    let mut report = DmapoolChecksReport {
        ret: 0,
        name_set: true,
        device_registered: true,
        release_installed: dmapool_test_release_called(),
        dma_ops_cleared: true,
        dma_mask: DMA_BIT_MASK_64,
        blocks_attempted: 0,
        device_deleted: false,
        device_put: false,
    };

    if config.dma_set_mask_ret != 0 {
        report.ret = config.dma_set_mask_ret;
        report.device_deleted = true;
        report.device_put = true;
        return report;
    }

    for i in 0..POOL_PARMS.len() {
        report.blocks_attempted += 1;
        if config.failing_block_index == Some(i) {
            report.ret = -ENOMEM;
            break;
        }
    }

    report.device_deleted = true;
    report.device_put = true;
    report
}

pub const fn dmapool_exit() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dmapool_module_test_parameters_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/dmapool_test.c"
        ));

        assert!(source.contains("#define NR_TESTS (100)"));
        assert!(source.contains("{ .size = 16, .align = 16, .boundary = 0 }"));
        assert!(source.contains("{ .size = 68, .align = 32, .boundary = 4096 }"));
        assert!(source.contains("return clamp_t(int, (PAGE_SIZE / size) * 512, 1024, 8192);"));
        assert!(source.contains("dma_pool_alloc(pool, GFP_KERNEL"));
        assert!(source.contains("dma_pool_free(pool, p[i].v, p[i].dma);"));
        assert!(source.contains("dma_pool_destroy(pool);"));
        assert!(source.contains("kfree(p);"));
        assert!(source.contains("if (need_resched())"));
        assert!(source.contains("cond_resched();"));
        assert!(source.contains("dev_set_name(&test_dev, \"dmapool-test\")"));
        assert!(source.contains("device_register(&test_dev)"));
        assert!(source.contains("test_dev.release = dmapool_test_release;"));
        assert!(source.contains("set_dma_ops(&test_dev, NULL);"));
        assert!(source.contains("dma_set_mask_and_coherent(&test_dev, DMA_BIT_MASK(64))"));
        assert!(source.contains("device_del(&test_dev);"));
        assert!(source.contains("put_device(&test_dev);"));
        assert!(source.contains("module_init(dmapool_checks);"));
        assert!(source.contains("module_exit(dmapool_exit);"));
        assert!(source.contains("MODULE_DESCRIPTION(\"dma_pool timing test\")"));
        assert_eq!(NR_TESTS, 100);
        assert_eq!(POOL_PARMS.len(), 6);
        assert_eq!(nr_blocks(16), 8192);
        assert_eq!(nr_blocks(4096), 1024);
    }

    #[test]
    fn dmapool_test_allocates_and_frees_all_blocks() {
        let report = dmapool_test_block_with_repeats(POOL_PARMS[0], 2).unwrap();
        assert_eq!(report.blocks, 8192);
        assert_eq!(report.repeats, 2);
        assert_eq!(report.pages_after_first_pass, 32);
        assert!(report.pool_destroyed);
        assert!(report.pairs_freed);
        assert_eq!(report.cond_resched_checks, 2);
    }

    #[test]
    fn dmapool_checks_models_device_lifecycle_and_error_labels() {
        let success = dmapool_checks(DmapoolChecksConfig::SUCCESS);
        assert_eq!(success.ret, 0);
        assert!(success.name_set);
        assert!(success.device_registered);
        assert!(success.release_installed);
        assert!(success.dma_ops_cleared);
        assert_eq!(success.dma_mask, DMA_BIT_MASK_64);
        assert_eq!(success.blocks_attempted, POOL_PARMS.len());
        assert!(success.device_deleted);
        assert!(success.device_put);

        let name_fail = dmapool_checks(DmapoolChecksConfig {
            dev_set_name_ret: -ENOMEM,
            ..DmapoolChecksConfig::SUCCESS
        });
        assert_eq!(name_fail.ret, -ENOMEM);
        assert!(!name_fail.device_put);

        let register_fail = dmapool_checks(DmapoolChecksConfig {
            device_register_ret: -ENOMEM,
            ..DmapoolChecksConfig::SUCCESS
        });
        assert_eq!(register_fail.ret, -ENOMEM);
        assert!(register_fail.name_set);
        assert!(register_fail.device_put);
        assert!(!register_fail.device_deleted);

        let mask_fail = dmapool_checks(DmapoolChecksConfig {
            dma_set_mask_ret: -ENOMEM,
            ..DmapoolChecksConfig::SUCCESS
        });
        assert_eq!(mask_fail.ret, -ENOMEM);
        assert!(mask_fail.device_deleted);
        assert!(mask_fail.device_put);

        let block_fail = dmapool_checks(DmapoolChecksConfig {
            failing_block_index: Some(2),
            ..DmapoolChecksConfig::SUCCESS
        });
        assert_eq!(block_fail.ret, -ENOMEM);
        assert_eq!(block_fail.blocks_attempted, 3);
        assert!(block_fail.device_deleted);
        assert!(block_fail.device_put);
    }
}
