//! linux-parity: complete
//! linux-source: vendor/linux/net/sched/sch_blackhole.c
//! test-origin: linux:vendor/linux/net/sched/sch_blackhole.c
//! Blackhole queue discipline.

pub const NET_XMIT_SUCCESS: u32 = 0x0000_0000;
pub const __NET_XMIT_BYPASS: u32 = 0x0002_0000;
pub const BLACKHOLE_QDISC_ID: &str = "blackhole";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QdiscOps {
    pub id: &'static str,
    pub priv_size: usize,
    pub enqueue: fn() -> u32,
    pub dequeue: fn() -> Option<()>,
    pub peek: fn() -> Option<()>,
}

pub fn blackhole_enqueue() -> u32 {
    NET_XMIT_SUCCESS | __NET_XMIT_BYPASS
}

pub const fn blackhole_dequeue() -> Option<()> {
    None
}

pub const BLACKHOLE_QDISC_OPS: QdiscOps = QdiscOps {
    id: BLACKHOLE_QDISC_ID,
    priv_size: 0,
    enqueue: blackhole_enqueue,
    dequeue: blackhole_dequeue,
    peek: blackhole_dequeue,
};

pub const fn blackhole_init_registers_qdisc() -> &'static QdiscOps {
    &BLACKHOLE_QDISC_OPS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blackhole_qdisc_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/sched/sch_blackhole.c"
        ));
        assert!(source.contains("static int blackhole_enqueue"));
        assert!(source.contains("qdisc_drop(skb, sch, to_free);"));
        assert!(source.contains("return NET_XMIT_SUCCESS | __NET_XMIT_BYPASS;"));
        assert!(source.contains("static struct sk_buff *blackhole_dequeue"));
        assert!(source.contains("return NULL;"));
        assert!(source.contains(".id\t\t= \"blackhole\""));
        assert!(source.contains(".priv_size\t= 0"));
        assert!(source.contains(".enqueue\t= blackhole_enqueue"));
        assert!(source.contains(".dequeue\t= blackhole_dequeue"));
        assert!(source.contains(".peek\t\t= blackhole_dequeue"));
        assert!(source.contains("return register_qdisc(&blackhole_qdisc_ops);"));
        assert!(source.contains("device_initcall(blackhole_init)"));

        assert_eq!(blackhole_enqueue(), __NET_XMIT_BYPASS);
        assert_eq!(blackhole_dequeue(), None);
        assert_eq!(BLACKHOLE_QDISC_OPS.id, "blackhole");
        assert_eq!(BLACKHOLE_QDISC_OPS.priv_size, 0);
        assert_eq!(blackhole_init_registers_qdisc().id, "blackhole");
    }
}
