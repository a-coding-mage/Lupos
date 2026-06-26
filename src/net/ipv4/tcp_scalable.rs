//! linux-parity: complete
//! linux-source: vendor/linux/net/ipv4/tcp_scalable.c
//! test-origin: linux:vendor/linux/net/ipv4/tcp_scalable.c
//! Tom Kelly's Scalable TCP congestion control.

pub const TCP_SCALABLE_AI_CNT: u32 = 100;
pub const TCP_SCALABLE_MD_SCALE: u32 = 3;
pub const TCP_SCALABLE_NAME: &str = "scalable";
pub const MODULE_AUTHOR: &str = "John Heffner";
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_DESCRIPTION: &str = "Scalable TCP";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TcpScalable {
    pub cwnd: u32,
    pub ssthresh: u32,
    pub cwnd_cnt: u32,
    pub cwnd_limited: bool,
}

impl TcpScalable {
    pub const fn new(cwnd: u32, ssthresh: u32) -> Self {
        Self {
            cwnd,
            ssthresh,
            cwnd_cnt: 0,
            cwnd_limited: true,
        }
    }
}

pub fn tcp_scalable_cong_avoid(tp: &mut TcpScalable, acked: u32) {
    if !tp.cwnd_limited {
        return;
    }

    let mut acked = acked;
    if tcp_in_slow_start(*tp) {
        let grow = acked.min(tp.ssthresh.saturating_sub(tp.cwnd));
        tp.cwnd = tp.cwnd.saturating_add(grow);
        acked = acked.saturating_sub(grow);
        if acked == 0 {
            return;
        }
    }

    tcp_cong_avoid_ai(tp, tp.cwnd.min(TCP_SCALABLE_AI_CNT), acked);
}

pub const fn tcp_scalable_ssthresh(cwnd: u32) -> u32 {
    let reduced = cwnd.saturating_sub(cwnd >> TCP_SCALABLE_MD_SCALE);
    if reduced < 2 { 2 } else { reduced }
}

pub const fn tcp_in_slow_start(tp: TcpScalable) -> bool {
    tp.cwnd < tp.ssthresh
}

pub fn tcp_cong_avoid_ai(tp: &mut TcpScalable, w: u32, acked: u32) {
    if w == 0 {
        return;
    }
    tp.cwnd_cnt = tp.cwnd_cnt.saturating_add(acked);
    while tp.cwnd_cnt >= w {
        tp.cwnd_cnt -= w;
        tp.cwnd = tp.cwnd.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tcp_scalable_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ipv4/tcp_scalable.c"
        ));
        assert!(source.contains("#define TCP_SCALABLE_AI_CNT\t100U"));
        assert!(source.contains("#define TCP_SCALABLE_MD_SCALE\t3"));
        assert!(source.contains("static void tcp_scalable_cong_avoid"));
        assert!(source.contains("if (!tcp_is_cwnd_limited(sk))"));
        assert!(source.contains("if (tcp_in_slow_start(tp))"));
        assert!(source.contains("tcp_slow_start(tp, acked);"));
        assert!(
            source.contains("tcp_cong_avoid_ai(tp, min(tcp_snd_cwnd(tp), TCP_SCALABLE_AI_CNT)")
        );
        assert!(source.contains("static u32 tcp_scalable_ssthresh"));
        assert!(source.contains("tcp_snd_cwnd(tp)>>TCP_SCALABLE_MD_SCALE"));
        assert!(source.contains(".ssthresh\t= tcp_scalable_ssthresh"));
        assert!(source.contains(".undo_cwnd\t= tcp_reno_undo_cwnd"));
        assert!(source.contains(".name\t\t= \"scalable\""));
        assert!(source.contains("tcp_register_congestion_control(&tcp_scalable);"));
        assert!(source.contains("MODULE_DESCRIPTION(\"Scalable TCP\");"));

        assert_eq!(TCP_SCALABLE_AI_CNT, 100);
        assert_eq!(TCP_SCALABLE_MD_SCALE, 3);
        assert_eq!(tcp_scalable_ssthresh(80), 70);
        assert_eq!(tcp_scalable_ssthresh(1), 2);
    }

    #[test]
    fn scalable_congestion_avoid_respects_slow_start_and_ai() {
        let mut tp = TcpScalable::new(10, 12);
        tcp_scalable_cong_avoid(&mut tp, 5);
        assert_eq!(tp.cwnd, 12);
        assert_eq!(tp.cwnd_cnt, 3);

        tcp_scalable_cong_avoid(&mut tp, 97);
        assert_eq!(tp.cwnd, 20);
        assert_eq!(tp.cwnd_cnt, 4);

        tp.cwnd_limited = false;
        tcp_scalable_cong_avoid(&mut tp, 100);
        assert_eq!(tp.cwnd, 20);
    }
}
