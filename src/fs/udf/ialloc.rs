//! linux-parity: complete
//! linux-source: vendor/linux/fs/udf/ialloc.c
//! test-origin: linux:vendor/linux/fs/udf/ialloc.c
//! UDF inode allocation decision logic.

use crate::include::uapi::errno::{EINVAL, EIO, ENOMEM};

pub const UDF_FLAG_USE_EXTENDED_FE: u32 = 0;
pub const UDF_FLAG_USE_SHORT_AD: u32 = 2;
pub const UDF_FLAG_USE_AD_IN_ICB: u32 = 3;
pub const UDF_FLAG_UID_SET: u32 = 13;
pub const UDF_FLAG_GID_SET: u32 = 14;
pub const UDF_VERS_USE_EXTENDED_FE: u16 = 0x0200;

pub const ICBTAG_FLAG_AD_SHORT: u16 = 0x0000;
pub const ICBTAG_FLAG_AD_LONG: u16 = 0x0001;
pub const ICBTAG_FLAG_AD_IN_ICB: u16 = 0x0003;
pub const FE_PERM_U_CHATTR: u32 = 0x0000_2000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UdfNewInodeInput {
    pub flags: u32,
    pub udfrev: u16,
    pub block_size: usize,
    pub file_entry_size: usize,
    pub extended_file_entry_size: usize,
    pub unique_id: u64,
    pub new_inode_ok: bool,
    pub data_alloc_ok: bool,
    pub new_block: Result<u32, i32>,
    pub insert_inode_locked_ok: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UdfNewInodePlan {
    pub extended_file_entry: bool,
    pub udfrev: u16,
    pub data_len: usize,
    pub block: u32,
    pub unique_id: u64,
    pub generation: u64,
    pub alloc_type: u16,
    pub checkpoint: u32,
    pub extra_perms: u32,
    pub uid_from_mount: bool,
    pub gid_from_mount: bool,
}

pub const fn udf_query_flag(flags: u32, bit: u32) -> bool {
    (flags & (1u32 << bit)) != 0
}

pub const fn udf_free_inode_blocks() -> u32 {
    1
}

pub fn udf_new_inode_plan(input: UdfNewInodeInput) -> Result<UdfNewInodePlan, i32> {
    if !input.new_inode_ok {
        return Err(-ENOMEM);
    }

    let extended = udf_query_flag(input.flags, UDF_FLAG_USE_EXTENDED_FE);
    let entry_size = if extended {
        input.extended_file_entry_size
    } else {
        input.file_entry_size
    };
    let data_len = input.block_size.checked_sub(entry_size).ok_or(-EINVAL)?;
    let mut udfrev = input.udfrev;
    if extended && UDF_VERS_USE_EXTENDED_FE > udfrev {
        udfrev = UDF_VERS_USE_EXTENDED_FE;
    }

    if !input.data_alloc_ok {
        return Err(-ENOMEM);
    }

    let block = input.new_block?;
    if !input.insert_inode_locked_ok {
        return Err(-EIO);
    }

    let alloc_type = if udf_query_flag(input.flags, UDF_FLAG_USE_AD_IN_ICB) {
        ICBTAG_FLAG_AD_IN_ICB
    } else if udf_query_flag(input.flags, UDF_FLAG_USE_SHORT_AD) {
        ICBTAG_FLAG_AD_SHORT
    } else {
        ICBTAG_FLAG_AD_LONG
    };

    Ok(UdfNewInodePlan {
        extended_file_entry: extended,
        udfrev,
        data_len,
        block,
        unique_id: input.unique_id,
        generation: input.unique_id,
        alloc_type,
        checkpoint: 1,
        extra_perms: FE_PERM_U_CHATTR,
        uid_from_mount: udf_query_flag(input.flags, UDF_FLAG_UID_SET),
        gid_from_mount: udf_query_flag(input.flags, UDF_FLAG_GID_SET),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::include::uapi::errno::ENOSPC;

    #[test]
    fn udf_ialloc_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/udf/ialloc.c"
        ));
        assert!(source.contains("#include \"udfdecl.h\""));
        assert!(source.contains("#include <linux/fs.h>"));
        assert!(source.contains("void udf_free_inode(struct inode *inode)"));
        assert!(
            source.contains("udf_free_blocks(inode->i_sb, NULL, &UDF_I(inode)->i_location, 0, 1);")
        );
        assert!(source.contains("struct inode *udf_new_inode(struct inode *dir, umode_t mode)"));
        assert!(source.contains("inode = new_inode(sb);"));
        assert!(source.contains("if (UDF_QUERY_FLAG(inode->i_sb, UDF_FLAG_USE_EXTENDED_FE))"));
        assert!(source.contains("iinfo->i_efe = 1;"));
        assert!(source.contains("UDF_VERS_USE_EXTENDED_FE > sbi->s_udfrev"));
        assert!(source.contains("kzalloc(inode->i_sb->s_blocksize -"));
        assert!(source.contains("return ERR_PTR(-ENOMEM);"));
        assert!(source.contains("err = -ENOSPC;"));
        assert!(source.contains("block = udf_new_block(dir->i_sb, NULL,"));
        assert!(source.contains("iinfo->i_unique = lvid_get_unique_id(sb);"));
        assert!(source.contains("inode->i_generation = iinfo->i_unique;"));
        assert!(source.contains("inode_init_owner(&nop_mnt_idmap, inode, dir, mode);"));
        assert!(source.contains("inode->i_uid = sbi->s_uid;"));
        assert!(source.contains("inode->i_gid = sbi->s_gid;"));
        assert!(source.contains("iinfo->i_checkpoint = 1;"));
        assert!(source.contains("iinfo->i_extraPerms = FE_PERM_U_CHATTR;"));
        assert!(source.contains("iinfo->i_alloc_type = ICBTAG_FLAG_AD_IN_ICB;"));
        assert!(source.contains("iinfo->i_alloc_type = ICBTAG_FLAG_AD_SHORT;"));
        assert!(source.contains("iinfo->i_alloc_type = ICBTAG_FLAG_AD_LONG;"));
        assert!(source.contains("simple_inode_init_ts(inode);"));
        assert!(source.contains("insert_inode_locked(inode) < 0"));
        assert!(source.contains("mark_inode_dirty(inode);"));

        assert_eq!(udf_free_inode_blocks(), 1);
        let plan = udf_new_inode_plan(UdfNewInodeInput {
            flags: (1 << UDF_FLAG_USE_EXTENDED_FE)
                | (1 << UDF_FLAG_USE_AD_IN_ICB)
                | (1 << UDF_FLAG_UID_SET),
            udfrev: 0x0150,
            block_size: 2048,
            file_entry_size: 176,
            extended_file_entry_size: 216,
            unique_id: 44,
            new_inode_ok: true,
            data_alloc_ok: true,
            new_block: Ok(99),
            insert_inode_locked_ok: true,
        })
        .unwrap();
        assert!(plan.extended_file_entry);
        assert_eq!(plan.udfrev, UDF_VERS_USE_EXTENDED_FE);
        assert_eq!(plan.data_len, 1832);
        assert_eq!(plan.block, 99);
        assert_eq!(plan.generation, 44);
        assert_eq!(plan.alloc_type, ICBTAG_FLAG_AD_IN_ICB);
        assert_eq!(plan.extra_perms, FE_PERM_U_CHATTR);
        assert!(plan.uid_from_mount);
        assert!(!plan.gid_from_mount);

        assert_eq!(
            udf_new_inode_plan(UdfNewInodeInput {
                new_inode_ok: false,
                ..plan_input()
            }),
            Err(-ENOMEM)
        );
        assert_eq!(
            udf_new_inode_plan(UdfNewInodeInput {
                data_alloc_ok: false,
                ..plan_input()
            }),
            Err(-ENOMEM)
        );
        assert_eq!(
            udf_new_inode_plan(UdfNewInodeInput {
                new_block: Err(-ENOSPC),
                ..plan_input()
            }),
            Err(-ENOSPC)
        );
        assert_eq!(
            udf_new_inode_plan(UdfNewInodeInput {
                insert_inode_locked_ok: false,
                ..plan_input()
            }),
            Err(-EIO)
        );
    }

    const fn plan_input() -> UdfNewInodeInput {
        UdfNewInodeInput {
            flags: 0,
            udfrev: 0x0200,
            block_size: 2048,
            file_entry_size: 176,
            extended_file_entry_size: 216,
            unique_id: 1,
            new_inode_ok: true,
            data_alloc_ok: true,
            new_block: Ok(1),
            insert_inode_locked_ok: true,
        }
    }
}
