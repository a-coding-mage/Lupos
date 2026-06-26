//! linux-parity: complete
//! linux-source: vendor/linux/net/ipv4/tcp_plb.c
//! test-origin: linux:vendor/linux/net/ipv4/tcp_plb.c
//! TCP Protective Load Balancing state machine.

pub const HZ: u32 = 1000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TcpPlbSysctl {
    pub tcp_plb_enabled: bool,
    pub tcp_plb_cong_thresh: i32,
    pub tcp_plb_rehash_rounds: u8,
    pub tcp_plb_idle_rehash_rounds: u8,
    pub tcp_plb_suspend_rto_sec: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TcpPlbState {
    pub consec_cong_rounds: u8,
    pub pause_until: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TcpSockState {
    pub packets_out: u32,
    pub plb_rehash: u32,
    pub rethink_txhash_count: u32,
    pub mib_tcp_plb_rehash: u64,
}

pub fn tcp_plb_update_state(sysctl: &TcpPlbSysctl, plb: &mut TcpPlbState, cong_ratio: i32) {
    if !sysctl.tcp_plb_enabled {
        return;
    }

    if cong_ratio >= 0 {
        if cong_ratio < sysctl.tcp_plb_cong_thresh {
            plb.consec_cong_rounds = 0;
        } else if plb.consec_cong_rounds < sysctl.tcp_plb_rehash_rounds {
            plb.consec_cong_rounds += 1;
        }
    }
}

fn before(a: u32, b: u32) -> bool {
    (a.wrapping_sub(b) as i32) < 0
}

pub fn tcp_plb_check_rehash(
    sysctl: &TcpPlbSysctl,
    sock: &mut TcpSockState,
    plb: &mut TcpPlbState,
    tcp_jiffies32: u32,
) {
    if !sysctl.tcp_plb_enabled {
        return;
    }

    let forced_rehash = plb.consec_cong_rounds >= sysctl.tcp_plb_rehash_rounds;
    let idle_rehash = sysctl.tcp_plb_idle_rehash_rounds != 0
        && sock.packets_out == 0
        && plb.consec_cong_rounds >= sysctl.tcp_plb_idle_rehash_rounds;

    if !forced_rehash && !idle_rehash {
        return;
    }

    let max_suspend = 2 * sysctl.tcp_plb_suspend_rto_sec * HZ;
    if plb.pause_until != 0
        && (!before(tcp_jiffies32, plb.pause_until)
            || before(tcp_jiffies32.wrapping_add(max_suspend), plb.pause_until))
    {
        plb.pause_until = 0;
    }

    if plb.pause_until != 0 {
        return;
    }

    sock.rethink_txhash_count += 1;
    plb.consec_cong_rounds = 0;
    sock.plb_rehash = sock.plb_rehash.wrapping_add(1);
    sock.mib_tcp_plb_rehash += 1;
}

pub fn tcp_plb_update_state_upon_rto(
    sysctl: &TcpPlbSysctl,
    plb: &mut TcpPlbState,
    tcp_jiffies32: u32,
    random_below_pause: u32,
) {
    if !sysctl.tcp_plb_enabled {
        return;
    }

    let base_pause = sysctl.tcp_plb_suspend_rto_sec * HZ;
    let jitter = if base_pause == 0 {
        0
    } else {
        random_below_pause % base_pause
    };
    plb.pause_until = tcp_jiffies32.wrapping_add(base_pause).wrapping_add(jitter);
    plb.consec_cong_rounds = 0;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sysctl() -> TcpPlbSysctl {
        TcpPlbSysctl {
            tcp_plb_enabled: true,
            tcp_plb_cong_thresh: 10,
            tcp_plb_rehash_rounds: 3,
            tcp_plb_idle_rehash_rounds: 2,
            tcp_plb_suspend_rto_sec: 4,
        }
    }

    #[test]
    fn tcp_plb_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ipv4/tcp_plb.c"
        ));

        assert!(source.contains("tcp_plb_update_state(const struct sock *sk"));
        assert!(source.contains("if (!READ_ONCE(net->ipv4.sysctl_tcp_plb_enabled))"));
        assert!(source.contains("plb->consec_cong_rounds = 0;"));
        assert!(source.contains("plb->consec_cong_rounds++;"));
        assert!(source.contains("forced_rehash = plb->consec_cong_rounds >="));
        assert!(
            source.contains("idle_rehash = READ_ONCE(net->ipv4.sysctl_tcp_plb_idle_rehash_rounds)")
        );
        assert!(source.contains(
            "max_suspend = 2 * READ_ONCE(net->ipv4.sysctl_tcp_plb_suspend_rto_sec) * HZ;"
        ));
        assert!(source.contains("sk_rethink_txhash(sk);"));
        assert!(source.contains("WRITE_ONCE(tcp_sk(sk)->plb_rehash, tcp_sk(sk)->plb_rehash + 1);"));
        assert!(source.contains("pause += get_random_u32_below(pause);"));
    }

    #[test]
    fn update_state_resets_or_saturates_congestion_rounds() {
        let sysctl = sysctl();
        let mut plb = TcpPlbState {
            consec_cong_rounds: 2,
            pause_until: 0,
        };

        tcp_plb_update_state(&sysctl, &mut plb, 9);
        assert_eq!(plb.consec_cong_rounds, 0);
        tcp_plb_update_state(&sysctl, &mut plb, -1);
        assert_eq!(plb.consec_cong_rounds, 0);
        tcp_plb_update_state(&sysctl, &mut plb, 10);
        tcp_plb_update_state(&sysctl, &mut plb, 11);
        tcp_plb_update_state(&sysctl, &mut plb, 12);
        tcp_plb_update_state(&sysctl, &mut plb, 13);
        assert_eq!(plb.consec_cong_rounds, 3);
    }

    #[test]
    fn check_rehash_handles_forced_idle_pause_and_wrap_cases() {
        let sysctl = sysctl();
        let mut sock = TcpSockState {
            packets_out: 1,
            ..TcpSockState::default()
        };
        let mut plb = TcpPlbState {
            consec_cong_rounds: 2,
            pause_until: 0,
        };

        tcp_plb_check_rehash(&sysctl, &mut sock, &mut plb, 100);
        assert_eq!(sock.rethink_txhash_count, 0);

        sock.packets_out = 0;
        tcp_plb_check_rehash(&sysctl, &mut sock, &mut plb, 100);
        assert_eq!(sock.rethink_txhash_count, 1);
        assert_eq!(sock.plb_rehash, 1);
        assert_eq!(sock.mib_tcp_plb_rehash, 1);
        assert_eq!(plb.consec_cong_rounds, 0);

        plb.consec_cong_rounds = 3;
        plb.pause_until = 500;
        tcp_plb_check_rehash(&sysctl, &mut sock, &mut plb, 100);
        assert_eq!(sock.rethink_txhash_count, 1);
        assert_eq!(plb.pause_until, 500);

        plb.pause_until = 50;
        tcp_plb_check_rehash(&sysctl, &mut sock, &mut plb, 100);
        assert_eq!(sock.rethink_txhash_count, 2);
        assert_eq!(plb.pause_until, 0);

        plb.consec_cong_rounds = 3;
        plb.pause_until = 10_000;
        tcp_plb_check_rehash(&sysctl, &mut sock, &mut plb, 100);
        assert_eq!(sock.rethink_txhash_count, 3);
        assert_eq!(plb.pause_until, 0);
    }

    #[test]
    fn rto_sets_random_pause_and_resets_congestion_state() {
        let sysctl = sysctl();
        let mut plb = TcpPlbState {
            consec_cong_rounds: 3,
            pause_until: 0,
        };
        tcp_plb_update_state_upon_rto(&sysctl, &mut plb, 1000, 777);
        assert_eq!(plb.pause_until, 1000 + 4 * HZ + 777);
        assert_eq!(plb.consec_cong_rounds, 0);

        let disabled = TcpPlbSysctl {
            tcp_plb_enabled: false,
            ..sysctl
        };
        tcp_plb_update_state_upon_rto(&disabled, &mut plb, 1, 1);
        assert_eq!(plb.pause_until, 5777);
    }
}
