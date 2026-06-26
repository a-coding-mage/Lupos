//! linux-parity: complete
//! linux-source: vendor/linux/fs/befs/io.c
//! test-origin: linux:vendor/linux/fs/befs/io.c
//! BeFS inode-address block reads.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BefsInodeAddr {
    pub allocation_group: u32,
    pub start: u16,
    pub len: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BefsBreadIaddrPlan {
    pub valid_allocation_group: bool,
    pub block: Option<u64>,
    pub read_attempted: bool,
    pub buffer_available: bool,
}

pub const fn iaddr2blockno(iaddr: BefsInodeAddr, ag_shift: u32) -> u64 {
    ((iaddr.allocation_group as u64) << ag_shift) + iaddr.start as u64
}

pub const fn befs_bread_iaddr_plan(
    num_ags: u32,
    ag_shift: u32,
    iaddr: BefsInodeAddr,
    sb_bread_succeeds: bool,
) -> BefsBreadIaddrPlan {
    if iaddr.allocation_group > num_ags {
        return BefsBreadIaddrPlan {
            valid_allocation_group: false,
            block: None,
            read_attempted: false,
            buffer_available: false,
        };
    }
    let block = iaddr2blockno(iaddr, ag_shift);
    BefsBreadIaddrPlan {
        valid_allocation_group: true,
        block: Some(block),
        read_attempted: true,
        buffer_available: sb_bread_succeeds,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn befs_bread_iaddr_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/befs/io.c"
        ));
        assert!(source.contains("#include <linux/buffer_head.h>"));
        assert!(source.contains("#include \"befs.h\""));
        assert!(source.contains("#include \"io.h\""));
        assert!(source.contains("befs_bread_iaddr"));
        assert!(source.contains("if (iaddr.allocation_group > befs_sb->num_ags)"));
        assert!(source.contains("block = iaddr2blockno(sb, &iaddr);"));
        assert!(source.contains("bh = sb_bread(sb, block);"));
        assert!(source.contains("if (bh == NULL)"));
        assert!(source.contains("return NULL;"));

        let addr = BefsInodeAddr {
            allocation_group: 2,
            start: 3,
            len: 1,
        };
        assert_eq!(iaddr2blockno(addr, 4), 35);
        let ok = befs_bread_iaddr_plan(2, 4, addr, true);
        assert!(ok.valid_allocation_group);
        assert_eq!(ok.block, Some(35));
        assert!(ok.buffer_available);
        let failed_read = befs_bread_iaddr_plan(2, 4, addr, false);
        assert!(failed_read.read_attempted);
        assert!(!failed_read.buffer_available);
        let invalid = befs_bread_iaddr_plan(1, 4, addr, true);
        assert!(!invalid.valid_allocation_group);
        assert!(!invalid.read_attempted);
    }
}
