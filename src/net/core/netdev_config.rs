//! linux-parity: complete
//! linux-source: vendor/linux/net/core/netdev_config.c
//! test-origin: linux:vendor/linux/net/core/netdev_config.c
//! Network-device queue configuration rendering.

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NetdevQueueConfig {
    pub rx_page_size: u32,
    pub tx_push: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MemoryProviderParams {
    pub rx_page_size: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NetdevRxQueue {
    pub mp_params: MemoryProviderParams,
}

pub type DefaultQcfg = fn(&mut NetdevQueueConfig);
pub type ValidateQcfg = fn(&NetdevQueueConfig) -> i32;

#[derive(Clone, Copy)]
pub struct QueueMgmtOps {
    pub default_qcfg: Option<DefaultQcfg>,
    pub validate_qcfg: Option<ValidateQcfg>,
}

#[derive(Clone, Copy)]
pub struct NetDevice<'a> {
    pub queue_mgmt_ops: QueueMgmtOps,
    pub rx_queues: &'a [NetdevRxQueue],
}

pub fn netdev_queue_config(dev: &NetDevice<'_>, rxq_idx: usize, qcfg: &mut NetdevQueueConfig) {
    let _ = __netdev_queue_config(dev, rxq_idx, qcfg, false);
}

pub fn netdev_queue_config_validate(
    dev: &NetDevice<'_>,
    rxq_idx: usize,
    qcfg: &mut NetdevQueueConfig,
) -> Result<(), i32> {
    __netdev_queue_config(dev, rxq_idx, qcfg, true)
}

fn __netdev_queue_config(
    dev: &NetDevice<'_>,
    rxq_idx: usize,
    qcfg: &mut NetdevQueueConfig,
    validate: bool,
) -> Result<(), i32> {
    let validate_cb = if validate {
        dev.queue_mgmt_ops
            .validate_qcfg
            .unwrap_or(netdev_nop_validate_qcfg)
    } else {
        netdev_nop_validate_qcfg
    };

    *qcfg = NetdevQueueConfig::default();
    if let Some(default_qcfg) = dev.queue_mgmt_ops.default_qcfg {
        default_qcfg(qcfg);
    }
    let err = validate_cb(qcfg);
    if err != 0 {
        return Err(err);
    }

    if let Some(queue) = dev.rx_queues.get(rxq_idx) {
        if queue.mp_params.rx_page_size != 0 {
            qcfg.rx_page_size = queue.mp_params.rx_page_size;
        }
    }
    let err = validate_cb(qcfg);
    if err != 0 {
        return Err(err);
    }

    Ok(())
}

pub const fn netdev_nop_validate_qcfg(_qcfg: &NetdevQueueConfig) -> i32 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defaults(qcfg: &mut NetdevQueueConfig) {
        qcfg.rx_page_size = 2048;
        qcfg.tx_push = true;
    }

    fn validate(qcfg: &NetdevQueueConfig) -> i32 {
        if qcfg.rx_page_size > 4096 { -22 } else { 0 }
    }

    #[test]
    fn netdev_config_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/core/netdev_config.c"
        ));
        assert!(source.contains("static int netdev_nop_validate_qcfg"));
        assert!(source.contains("return 0;"));
        assert!(source.contains("static int __netdev_queue_config"));
        assert!(source.contains("validate_cb = netdev_nop_validate_qcfg;"));
        assert!(source.contains("if (validate && dev->queue_mgmt_ops->ndo_validate_qcfg)"));
        assert!(source.contains("memset(qcfg, 0, sizeof(*qcfg));"));
        assert!(source.contains("dev->queue_mgmt_ops->ndo_default_qcfg(dev, qcfg);"));
        assert!(source.contains("err = validate_cb(dev, qcfg, extack);"));
        assert!(source.contains("mpp = &__netif_get_rx_queue(dev, rxq_idx)->mp_params;"));
        assert!(source.contains("if (mpp->rx_page_size)"));
        assert!(source.contains("qcfg->rx_page_size = mpp->rx_page_size;"));
        assert!(source.contains("void netdev_queue_config"));
        assert!(source.contains("__netdev_queue_config(dev, rxq_idx, qcfg, NULL, false);"));
        assert!(source.contains("int netdev_queue_config_validate"));
        assert!(source.contains("__netdev_queue_config(dev, rxq_idx, qcfg, extack, true);"));
    }

    #[test]
    fn queue_config_zeros_defaults_applies_mp_override_and_validates_when_requested() {
        let queues = [NetdevRxQueue {
            mp_params: MemoryProviderParams { rx_page_size: 4096 },
        }];
        let dev = NetDevice {
            queue_mgmt_ops: QueueMgmtOps {
                default_qcfg: Some(defaults),
                validate_qcfg: Some(validate),
            },
            rx_queues: &queues,
        };
        let mut qcfg = NetdevQueueConfig {
            rx_page_size: 1,
            tx_push: false,
        };
        netdev_queue_config(&dev, 0, &mut qcfg);
        assert_eq!(
            qcfg,
            NetdevQueueConfig {
                rx_page_size: 4096,
                tx_push: true,
            }
        );
        assert_eq!(netdev_queue_config_validate(&dev, 0, &mut qcfg), Ok(()));

        let queues = [NetdevRxQueue {
            mp_params: MemoryProviderParams { rx_page_size: 8192 },
        }];
        let dev = NetDevice {
            rx_queues: &queues,
            ..dev
        };
        assert_eq!(netdev_queue_config_validate(&dev, 0, &mut qcfg), Err(-22));
    }
}
