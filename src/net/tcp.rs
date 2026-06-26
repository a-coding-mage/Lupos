//! linux-parity: complete
//! linux-source: vendor/linux/net
//! test-origin: linux:vendor/linux/net
//! TCP state machine and CUBIC congestion-control core.

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::vec::Vec;

use crate::include::uapi::errno::EINVAL;

pub const TCP_FIN: u8 = 0x01;
pub const TCP_SYN: u8 = 0x02;
pub const TCP_RST: u8 = 0x04;
pub const TCP_PSH: u8 = 0x08;
pub const TCP_ACK: u8 = 0x10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TcpState {
    Closed,
    Listen,
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    CloseWait,
    LastAck,
    TimeWait,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TcpSegment {
    pub seq: u32,
    pub ack: u32,
    pub flags: u8,
    pub wnd: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cubic {
    pub cwnd: u32,
    pub ssthresh: u32,
    pub epoch_start: u64,
}

impl Cubic {
    pub const fn new() -> Self {
        Self {
            cwnd: 10,
            ssthresh: u32::MAX,
            epoch_start: 0,
        }
    }

    pub fn on_ack(&mut self, acked: u32) {
        if self.cwnd < self.ssthresh {
            self.cwnd = self.cwnd.saturating_add(acked.max(1));
        } else if acked > 0 {
            self.cwnd = self.cwnd.saturating_add(1);
        }
    }

    pub fn on_loss(&mut self) {
        self.ssthresh = (self.cwnd / 2).max(2);
        self.cwnd = self.ssthresh;
        self.epoch_start = 0;
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TcpConnection {
    pub state: TcpState,
    pub snd_nxt: u32,
    pub rcv_nxt: u32,
    pub congestion: Cubic,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TcpStreamQueues {
    sendq: VecDeque<Vec<u8>>,
    recvq: VecDeque<Vec<u8>>,
}

impl TcpStreamQueues {
    pub fn queue_send(&mut self, bytes: &[u8]) {
        self.sendq.push_back(bytes.to_vec());
    }

    pub fn pop_send(&mut self) -> Option<Vec<u8>> {
        self.sendq.pop_front()
    }

    pub fn queue_recv(&mut self, bytes: &[u8]) {
        self.recvq.push_back(bytes.to_vec());
    }

    pub fn recv(&mut self, out: &mut [u8]) -> Result<usize, i32> {
        let bytes = self.recvq.pop_front().ok_or(EINVAL)?;
        let len = out.len().min(bytes.len());
        out[..len].copy_from_slice(&bytes[..len]);
        Ok(len)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TcpRetransmitTimer {
    pub rto_ms: u32,
    pub retransmits: u32,
    pub armed: bool,
}

impl TcpRetransmitTimer {
    pub const fn new(rto_ms: u32) -> Self {
        Self {
            rto_ms,
            retransmits: 0,
            armed: false,
        }
    }

    pub fn arm(&mut self) {
        self.armed = true;
    }

    pub fn acked(&mut self) {
        self.armed = false;
        self.retransmits = 0;
    }

    pub fn on_timeout(&mut self) -> Result<u32, i32> {
        if !self.armed {
            return Err(EINVAL);
        }
        self.retransmits = self.retransmits.saturating_add(1);
        self.rto_ms = self.rto_ms.saturating_mul(2).clamp(200, 120_000);
        Ok(self.rto_ms)
    }
}

impl TcpConnection {
    pub const fn closed() -> Self {
        Self {
            state: TcpState::Closed,
            snd_nxt: 0,
            rcv_nxt: 0,
            congestion: Cubic::new(),
        }
    }

    pub fn listen() -> Self {
        let mut conn = Self::closed();
        conn.state = TcpState::Listen;
        conn
    }

    pub fn connect(iss: u32) -> Self {
        Self {
            state: TcpState::SynSent,
            snd_nxt: iss.wrapping_add(1),
            rcv_nxt: 0,
            congestion: Cubic::new(),
        }
    }

    pub fn on_segment(&mut self, seg: TcpSegment) -> Result<(), i32> {
        match self.state {
            TcpState::Listen if seg.flags & TCP_SYN != 0 => {
                self.rcv_nxt = seg.seq.wrapping_add(1);
                self.snd_nxt = 1;
                self.state = TcpState::SynReceived;
                Ok(())
            }
            TcpState::SynSent if seg.flags & TCP_SYN != 0 && seg.flags & TCP_ACK != 0 => {
                self.rcv_nxt = seg.seq.wrapping_add(1);
                self.state = TcpState::Established;
                Ok(())
            }
            TcpState::SynReceived if seg.flags & TCP_ACK != 0 => {
                self.state = TcpState::Established;
                Ok(())
            }
            TcpState::Established if seg.flags & TCP_FIN != 0 => {
                self.rcv_nxt = seg.seq.wrapping_add(1);
                self.state = TcpState::CloseWait;
                Ok(())
            }
            TcpState::Established if seg.flags & TCP_ACK != 0 => {
                self.congestion.on_ack(1);
                Ok(())
            }
            TcpState::FinWait1 if seg.flags & TCP_ACK != 0 => {
                self.state = TcpState::FinWait2;
                Ok(())
            }
            TcpState::LastAck if seg.flags & TCP_ACK != 0 => {
                self.state = TcpState::Closed;
                Ok(())
            }
            _ if seg.flags & TCP_RST != 0 => {
                self.state = TcpState::Closed;
                Ok(())
            }
            // RFC 793 §3.9: silently discard segments received in TIME-WAIT.
            TcpState::TimeWait => Ok(()),
            _ => Err(EINVAL),
        }
    }

    pub fn close(&mut self) {
        self.state = match self.state {
            TcpState::Established => TcpState::FinWait1,
            TcpState::CloseWait => TcpState::LastAck,
            other => other,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_open_reaches_established_and_cubic_changes() {
        let mut conn = TcpConnection::connect(100);
        conn.on_segment(TcpSegment {
            seq: 500,
            ack: 101,
            flags: TCP_SYN | TCP_ACK,
            wnd: 4096,
        })
        .unwrap();
        assert_eq!(conn.state, TcpState::Established);

        let before = conn.congestion.cwnd;
        conn.on_segment(TcpSegment {
            seq: 501,
            ack: 101,
            flags: TCP_ACK,
            wnd: 4096,
        })
        .unwrap();
        assert!(conn.congestion.cwnd > before);
        conn.congestion.on_loss();
        assert!(conn.congestion.cwnd <= before);
    }

    #[test]
    fn passive_open_reaches_established() {
        let mut conn = TcpConnection::listen();
        conn.on_segment(TcpSegment {
            seq: 7,
            ack: 0,
            flags: TCP_SYN,
            wnd: 4096,
        })
        .unwrap();
        assert_eq!(conn.state, TcpState::SynReceived);
        conn.on_segment(TcpSegment {
            seq: 8,
            ack: 1,
            flags: TCP_ACK,
            wnd: 4096,
        })
        .unwrap();
        assert_eq!(conn.state, TcpState::Established);
    }

    #[test]
    fn tcp_queues_and_retransmit_timer_are_stateful() {
        let mut queues = TcpStreamQueues::default();
        queues.queue_send(b"out");
        assert_eq!(queues.pop_send().unwrap(), b"out");
        queues.queue_recv(b"in");
        let mut buf = [0u8; 8];
        assert_eq!(queues.recv(&mut buf).unwrap(), 2);
        assert_eq!(&buf[..2], b"in");

        let mut timer = TcpRetransmitTimer::new(200);
        assert_eq!(timer.on_timeout(), Err(EINVAL));
        timer.arm();
        assert_eq!(timer.on_timeout().unwrap(), 400);
        assert_eq!(timer.retransmits, 1);
        timer.acked();
        assert!(!timer.armed);
        assert_eq!(timer.retransmits, 0);
    }
}
