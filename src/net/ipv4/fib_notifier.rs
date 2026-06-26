//! linux-parity: complete
//! linux-source: vendor/linux/net/ipv4/fib_notifier.c
//! test-origin: linux:vendor/linux/net/ipv4/fib_notifier.c
//! IPv4 FIB notifier registration helpers.

pub const AF_INET: u16 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FibEventType {
    EntryAdd,
    EntryReplace,
    EntryDel,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FibNotifierInfo {
    pub family: u16,
    pub payload: u32,
}

pub type FibNotifierCallback = fn(FibEventType, &mut FibNotifierInfo) -> i32;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Fib4NotifierNet {
    pub fib_seq: u32,
    pub notifier_ops_registered: bool,
}

pub fn call_fib4_notifier(
    callback: FibNotifierCallback,
    event_type: FibEventType,
    info: &mut FibNotifierInfo,
) -> i32 {
    info.family = AF_INET;
    callback(event_type, info)
}

pub fn call_fib4_notifiers(
    net: &mut Fib4NotifierNet,
    callbacks: &[FibNotifierCallback],
    event_type: FibEventType,
    info: &mut FibNotifierInfo,
) -> i32 {
    info.family = AF_INET;
    net.fib_seq = net.fib_seq.wrapping_add(1);
    let mut ret = 0;
    for callback in callbacks {
        ret = callback(event_type, info);
        if ret != 0 {
            break;
        }
    }
    ret
}

pub const fn fib4_seq_read(fib_seq: u32, fib4_rules_seq: u32) -> u32 {
    fib_seq.wrapping_add(fib4_rules_seq)
}

pub const fn fib4_dump(fib4_rules_dump_rc: i32, fib_notify_rc: i32) -> i32 {
    if fib4_rules_dump_rc != 0 {
        fib4_rules_dump_rc
    } else {
        fib_notify_rc
    }
}

pub fn fib4_notifier_init(net: &mut Fib4NotifierNet, register_ok: bool) -> Result<(), i32> {
    net.fib_seq = 0;
    if register_ok {
        net.notifier_ops_registered = true;
        Ok(())
    } else {
        Err(-1)
    }
}

pub fn fib4_notifier_exit(net: &mut Fib4NotifierNet) {
    net.notifier_ops_registered = false;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(_event: FibEventType, info: &mut FibNotifierInfo) -> i32 {
        info.payload += 1;
        0
    }

    fn fail(_event: FibEventType, _info: &mut FibNotifierInfo) -> i32 {
        -7
    }

    #[test]
    fn fib4_notifier_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ipv4/fib_notifier.c"
        ));
        assert!(source.contains("int call_fib4_notifier"));
        assert!(source.contains("info->family = AF_INET;"));
        assert!(source.contains("return call_fib_notifier(nb, event_type, info);"));
        assert!(source.contains("int call_fib4_notifiers"));
        assert!(source.contains("WRITE_ONCE(net->ipv4.fib_seq, net->ipv4.fib_seq + 1);"));
        assert!(source.contains("return call_fib_notifiers(net, event_type, info);"));
        assert!(source.contains("return READ_ONCE(net->ipv4.fib_seq) + fib4_rules_seq_read(net);"));
        assert!(source.contains("err = fib4_rules_dump(net, nb, extack);"));
        assert!(source.contains("return fib_notify(net, nb, extack);"));
        assert!(source.contains(".family\t\t= AF_INET"));
        assert!(source.contains(".fib_seq_read\t= fib4_seq_read"));
        assert!(source.contains(".fib_dump\t= fib4_dump"));
        assert!(source.contains("net->ipv4.fib_seq = 0;"));
        assert!(source.contains("net->ipv4.notifier_ops = ops;"));
        assert!(source.contains("fib_notifier_ops_unregister(net->ipv4.notifier_ops);"));

        assert_eq!(fib4_seq_read(10, 3), 13);
        assert_eq!(fib4_dump(-5, 0), -5);
        assert_eq!(fib4_dump(0, -6), -6);
    }

    #[test]
    fn fib4_calls_set_family_and_increment_sequence() {
        let mut info = FibNotifierInfo::default();
        assert_eq!(
            call_fib4_notifier(record, FibEventType::EntryAdd, &mut info),
            0
        );
        assert_eq!(info.family, AF_INET);
        assert_eq!(info.payload, 1);

        let mut net = Fib4NotifierNet::default();
        let mut info = FibNotifierInfo::default();
        assert_eq!(
            call_fib4_notifiers(
                &mut net,
                &[record, fail, record],
                FibEventType::EntryDel,
                &mut info
            ),
            -7
        );
        assert_eq!(info.family, AF_INET);
        assert_eq!(info.payload, 1);
        assert_eq!(net.fib_seq, 1);

        assert_eq!(fib4_notifier_init(&mut net, true), Ok(()));
        assert_eq!(net.fib_seq, 0);
        assert!(net.notifier_ops_registered);
        fib4_notifier_exit(&mut net);
        assert!(!net.notifier_ops_registered);
    }
}
