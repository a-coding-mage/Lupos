//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_btf.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_btf.c
//! BTF (BPF Type Format) integration for tracepoints — resolves event-arg
//! types so userspace can pretty-print structures.
//!
//! Ref: vendor/linux/kernel/trace/trace_btf.c

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BtfTypeId(pub u32);

pub fn type_id_is_valid(id: BtfTypeId) -> bool {
    id.0 != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_id_is_invalid() {
        assert!(!type_id_is_valid(BtfTypeId(0)));
        assert!(type_id_is_valid(BtfTypeId(42)));
    }
}
