//! linux-parity: complete
//! linux-source: vendor/linux/net/ipv4/metrics.c
//! test-origin: linux:vendor/linux/net/ipv4/metrics.c
//! IPv4 FIB route metric conversion.

use crate::include::uapi::errno::{EINVAL, ENOMEM};

pub const RTAX_UNSPEC: usize = 0;
pub const RTAX_MTU: usize = 2;
pub const RTAX_ADVMSS: usize = 8;
pub const RTAX_HOPLIMIT: usize = 10;
pub const RTAX_FEATURES: usize = 12;
pub const RTAX_CC_ALGO: usize = 16;
pub const RTAX_MAX: usize = 17;
pub const TCP_CA_UNSPEC: u32 = 0;
pub const RTAX_FEATURE_MASK: u32 = 0x1f;
pub const DST_FEATURE_ECN_CA: u32 = 1 << 31;
pub const IPV4_ADVMSS_MAX: u32 = 65_535 - 40;
pub const IPV4_MTU_MAX: u32 = 65_535 - 15;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MetricValue {
    U32(u32),
    CongestionControl { key: u32, ecn_ca: bool },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MetricAttr {
    pub ty: usize,
    pub value: MetricValue,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DstMetrics {
    pub metrics: [u32; RTAX_MAX],
    pub refcnt: u32,
    pub is_default: bool,
}

pub const DST_DEFAULT_METRICS: DstMetrics = DstMetrics {
    metrics: [0; RTAX_MAX],
    refcnt: 0,
    is_default: true,
};

pub fn ip_metrics_convert(attrs: &[MetricAttr], metrics: &mut [u32; RTAX_MAX]) -> Result<(), i32> {
    let mut ecn_ca = false;

    for attr in attrs {
        let ty = attr.ty;
        if ty == RTAX_UNSPEC {
            continue;
        }
        if ty > RTAX_MAX {
            return Err(-EINVAL);
        }

        let mut val = match (ty, attr.value) {
            (RTAX_CC_ALGO, MetricValue::CongestionControl { key, ecn_ca: ecn }) => {
                if key == TCP_CA_UNSPEC {
                    return Err(-EINVAL);
                }
                ecn_ca |= ecn;
                key
            }
            (RTAX_CC_ALGO, MetricValue::U32(_)) => return Err(-EINVAL),
            (_, MetricValue::U32(value)) => value,
            (_, MetricValue::CongestionControl { .. }) => return Err(-EINVAL),
        };

        if ty == RTAX_ADVMSS && val > IPV4_ADVMSS_MAX {
            val = IPV4_ADVMSS_MAX;
        }
        if ty == RTAX_MTU && val > IPV4_MTU_MAX {
            val = IPV4_MTU_MAX;
        }
        if ty == RTAX_HOPLIMIT && val > 255 {
            val = 255;
        }
        if ty == RTAX_FEATURES && (val & !RTAX_FEATURE_MASK) != 0 {
            return Err(-EINVAL);
        }

        metrics[ty - 1] = val;
    }

    if ecn_ca {
        metrics[RTAX_FEATURES - 1] |= DST_FEATURE_ECN_CA;
    }
    Ok(())
}

pub fn ip_fib_metrics_init(
    attrs: Option<&[MetricAttr]>,
    alloc_ok: bool,
) -> Result<DstMetrics, i32> {
    let Some(attrs) = attrs else {
        return Ok(DST_DEFAULT_METRICS);
    };
    if !alloc_ok {
        return Err(-ENOMEM);
    }

    let mut fib_metrics = DstMetrics {
        metrics: [0; RTAX_MAX],
        refcnt: 0,
        is_default: false,
    };
    ip_metrics_convert(attrs, &mut fib_metrics.metrics)?;
    fib_metrics.refcnt = 1;
    Ok(fib_metrics)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipv4_metrics_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ipv4/metrics.c"
        ));
        assert!(source.contains("static int ip_metrics_convert"));
        assert!(source.contains("bool ecn_ca = false;"));
        assert!(source.contains("nla_for_each_attr(nla, fc_mx, fc_mx_len, remaining)"));
        assert!(source.contains("if (!type)"));
        assert!(source.contains("if (type > RTAX_MAX)"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains("type = array_index_nospec(type, RTAX_MAX + 1);"));
        assert!(source.contains("if (type == RTAX_CC_ALGO)"));
        assert!(source.contains("tcp_ca_get_key_by_name(tmp, &ecn_ca);"));
        assert!(source.contains("if (val == TCP_CA_UNSPEC)"));
        assert!(source.contains("if (nla_len(nla) != sizeof(u32))"));
        assert!(source.contains("if (type == RTAX_ADVMSS && val > 65535 - 40)"));
        assert!(source.contains("if (type == RTAX_MTU && val > 65535 - 15)"));
        assert!(source.contains("if (type == RTAX_HOPLIMIT && val > 255)"));
        assert!(source.contains("if (type == RTAX_FEATURES && (val & ~RTAX_FEATURE_MASK))"));
        assert!(source.contains("metrics[type - 1] = val;"));
        assert!(source.contains("metrics[RTAX_FEATURES - 1] |= DST_FEATURE_ECN_CA;"));
        assert!(source.contains("return (struct dst_metrics *)&dst_default_metrics;"));
        assert!(source.contains("return ERR_PTR(-ENOMEM);"));
        assert!(source.contains("refcount_set(&fib_metrics->refcnt, 1);"));
    }

    #[test]
    fn metrics_convert_clamps_and_sets_ecn_feature() {
        let attrs = [
            MetricAttr {
                ty: RTAX_MTU,
                value: MetricValue::U32(u32::MAX),
            },
            MetricAttr {
                ty: RTAX_ADVMSS,
                value: MetricValue::U32(u32::MAX),
            },
            MetricAttr {
                ty: RTAX_HOPLIMIT,
                value: MetricValue::U32(1024),
            },
            MetricAttr {
                ty: RTAX_CC_ALGO,
                value: MetricValue::CongestionControl {
                    key: 7,
                    ecn_ca: true,
                },
            },
        ];
        let metrics = ip_fib_metrics_init(Some(&attrs), true).unwrap();
        assert_eq!(metrics.metrics[RTAX_MTU - 1], IPV4_MTU_MAX);
        assert_eq!(metrics.metrics[RTAX_ADVMSS - 1], IPV4_ADVMSS_MAX);
        assert_eq!(metrics.metrics[RTAX_HOPLIMIT - 1], 255);
        assert_eq!(metrics.metrics[RTAX_CC_ALGO - 1], 7);
        assert_eq!(
            metrics.metrics[RTAX_FEATURES - 1] & DST_FEATURE_ECN_CA,
            DST_FEATURE_ECN_CA
        );
        assert_eq!(metrics.refcnt, 1);
        assert_eq!(ip_fib_metrics_init(None, false), Ok(DST_DEFAULT_METRICS));
        assert_eq!(
            ip_fib_metrics_init(
                Some(&[MetricAttr {
                    ty: RTAX_MAX + 1,
                    value: MetricValue::U32(1),
                }]),
                true,
            ),
            Err(-EINVAL)
        );
        assert_eq!(
            ip_fib_metrics_init(
                Some(&[MetricAttr {
                    ty: RTAX_FEATURES,
                    value: MetricValue::U32(0x20),
                }]),
                true,
            ),
            Err(-EINVAL)
        );
    }
}
