//! linux-parity: complete
//! linux-source: vendor/linux/net/batman-adv/bitarray.c
//! test-origin: linux:vendor/linux/net/batman-adv/bitarray.c
//! B.A.T.M.A.N. advanced sequence receive window bitmap.

pub const BATADV_TQ_LOCAL_WINDOW_SIZE: i32 = 64;
pub const BATADV_EXPECTED_SEQNO_RANGE: i32 = 65_536;
pub const BATADV_TQ_LOCAL_WINDOW_MASK: u64 = u64::MAX;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BatadvBitWindow {
    pub seq_bits: u64,
}

impl BatadvBitWindow {
    pub const fn new(seq_bits: u64) -> Self {
        Self { seq_bits }
    }

    pub const fn bit_is_set(self, bit: u32) -> bool {
        bit < BATADV_TQ_LOCAL_WINDOW_SIZE as u32 && (self.seq_bits & (1u64 << bit)) != 0
    }
}

pub const fn batadv_bitmap_shift_left(seq_bits: u64, n: i32) -> u64 {
    if n <= 0 || n >= BATADV_TQ_LOCAL_WINDOW_SIZE {
        return seq_bits;
    }
    (seq_bits << n) & BATADV_TQ_LOCAL_WINDOW_MASK
}

pub const fn batadv_set_bit(seq_bits: u64, n: i32) -> u64 {
    if n < 0 || n >= BATADV_TQ_LOCAL_WINDOW_SIZE {
        seq_bits
    } else {
        seq_bits | (1u64 << n)
    }
}

pub const fn batadv_bit_get_packet(
    mut window: BatadvBitWindow,
    seq_num_diff: i32,
    set_mark: bool,
) -> (BatadvBitWindow, bool) {
    if seq_num_diff <= 0 && seq_num_diff > -BATADV_TQ_LOCAL_WINDOW_SIZE {
        if set_mark {
            window.seq_bits = batadv_set_bit(window.seq_bits, -seq_num_diff);
        }
        return (window, false);
    }

    if seq_num_diff > 0 && seq_num_diff < BATADV_TQ_LOCAL_WINDOW_SIZE {
        window.seq_bits = batadv_bitmap_shift_left(window.seq_bits, seq_num_diff);
        if set_mark {
            window.seq_bits = batadv_set_bit(window.seq_bits, 0);
        }
        return (window, true);
    }

    if seq_num_diff >= BATADV_TQ_LOCAL_WINDOW_SIZE && seq_num_diff < BATADV_EXPECTED_SEQNO_RANGE {
        window.seq_bits = 0;
        if set_mark {
            window.seq_bits = batadv_set_bit(window.seq_bits, 0);
        }
        return (window, true);
    }

    window.seq_bits = 0;
    if set_mark {
        window.seq_bits = batadv_set_bit(window.seq_bits, 0);
    }
    (window, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batadv_bitarray_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/batman-adv/bitarray.c"
        ));
        assert!(source.contains("static void batadv_bitmap_shift_left"));
        assert!(source.contains("if (n <= 0 || n >= BATADV_TQ_LOCAL_WINDOW_SIZE)"));
        assert!(
            source
                .contains("bitmap_shift_left(seq_bits, seq_bits, n, BATADV_TQ_LOCAL_WINDOW_SIZE);")
        );
        assert!(source.contains("bool batadv_bit_get_packet"));
        assert!(
            source.contains("seq_num_diff <= 0 && seq_num_diff > -BATADV_TQ_LOCAL_WINDOW_SIZE")
        );
        assert!(source.contains("batadv_set_bit(seq_bits, -seq_num_diff);"));
        assert!(source.contains("return false;"));
        assert!(source.contains("seq_num_diff > 0 && seq_num_diff < BATADV_TQ_LOCAL_WINDOW_SIZE"));
        assert!(source.contains("batadv_bitmap_shift_left(seq_bits, seq_num_diff);"));
        assert!(source.contains("seq_num_diff >= BATADV_TQ_LOCAL_WINDOW_SIZE"));
        assert!(source.contains("seq_num_diff < BATADV_EXPECTED_SEQNO_RANGE"));
        assert!(source.contains("bitmap_zero(seq_bits, BATADV_TQ_LOCAL_WINDOW_SIZE);"));
        assert!(source.contains("\"Other host probably restarted!\\n\""));
    }

    #[test]
    fn sequence_window_follows_linux_shift_and_reset_cases() {
        let (old, moved) = batadv_bit_get_packet(BatadvBitWindow::new(0), -3, true);
        assert!(!moved);
        assert!(old.bit_is_set(3));

        let (newer, moved) = batadv_bit_get_packet(BatadvBitWindow::new(0b11), 2, true);
        assert!(moved);
        assert_eq!(newer.seq_bits, 0b1101);

        let (missed, moved) = batadv_bit_get_packet(BatadvBitWindow::new(u64::MAX), 64, true);
        assert!(moved);
        assert_eq!(missed.seq_bits, 1);

        let (restart, moved) = batadv_bit_get_packet(BatadvBitWindow::new(0b111), 65_536, true);
        assert!(moved);
        assert_eq!(restart.seq_bits, 1);
    }
}
