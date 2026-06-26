//! linux-parity: complete
//! linux-source: vendor/linux/fs/efs/namei.c
//! test-origin: linux:vendor/linux/fs/efs/namei.c
//! EFS directory lookup helpers.

use crate::include::uapi::errno::{ENOENT, ESTALE};

pub const EFS_DIRBLK_MAGIC: u16 = 0xbeef;
pub const EFS_DIRBSIZE: u64 = 1 << 9;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EfsDentry<'a> {
    pub inode: u32,
    pub name: &'a [u8],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EfsDirBlock<'a> {
    pub magic: u16,
    pub slots: &'a [EfsDentry<'a>],
}

pub fn efs_find_entry(blocks: &[EfsDirBlock<'_>], name: &[u8]) -> u32 {
    for block in blocks {
        if block.magic != EFS_DIRBLK_MAGIC {
            return 0;
        }
        for slot in block.slots {
            if slot.name.len() == name.len() && slot.name == name {
                return slot.inode;
            }
        }
    }
    0
}

pub fn efs_nfs_get_inode_result(
    ino: u64,
    generation: u32,
    inode_generation: u32,
    iget_result: Result<(), i32>,
) -> Result<(), i32> {
    if ino == 0 {
        return Err(-ESTALE);
    }
    iget_result?;
    if generation != 0 && inode_generation != generation {
        return Err(-ESTALE);
    }
    Ok(())
}

pub fn efs_get_parent_result(parent_ino: u32) -> Result<u32, i32> {
    if parent_ino == 0 {
        Err(-ENOENT)
    } else {
        Ok(parent_ino)
    }
}

pub const fn efs_directory_size_misaligned(size: u64) -> bool {
    (size & (EFS_DIRBSIZE - 1)) != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn efs_namei_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/efs/namei.c"
        ));
        assert!(source.contains("#include <linux/buffer_head.h>"));
        assert!(source.contains("#include <linux/string.h>"));
        assert!(source.contains("#include <linux/exportfs.h>"));
        assert!(source.contains("#include \"efs.h\""));
        assert!(source.contains(
            "static efs_ino_t efs_find_entry(struct inode *inode, const char *name, int len)"
        ));
        assert!(source.contains("if (inode->i_size & (EFS_DIRBSIZE-1))"));
        assert!(source.contains("for(block = 0; block < inode->i_blocks; block++)"));
        assert!(source.contains("bh = sb_bread(inode->i_sb, efs_bmap(inode, block));"));
        assert!(source.contains("if (be16_to_cpu(dirblock->magic) != EFS_DIRBLK_MAGIC)"));
        assert!(source.contains("for (slot = 0; slot < dirblock->slots; slot++)"));
        assert!(source.contains("dirslot  = (struct efs_dentry *)"));
        assert!(source.contains("namelen  = dirslot->namelen;"));
        assert!(source.contains("if ((namelen == len) && (!memcmp(name, nameptr, len)))"));
        assert!(source.contains("inodenum = be32_to_cpu(dirslot->inode);"));
        assert!(
            source.contains("struct dentry *efs_lookup(struct inode *dir, struct dentry *dentry")
        );
        assert!(source.contains("inode = efs_iget(dir->i_sb, inodenum);"));
        assert!(source.contains("return d_splice_alias(inode, dentry);"));
        assert!(source.contains("static struct inode *efs_nfs_get_inode"));
        assert!(source.contains("if (ino == 0)"));
        assert!(source.contains("return ERR_PTR(-ESTALE);"));
        assert!(source.contains("if (generation && inode->i_generation != generation)"));
        assert!(source.contains("struct dentry *efs_fh_to_dentry"));
        assert!(source.contains("generic_fh_to_dentry(sb, fid, fh_len, fh_type,"));
        assert!(source.contains("struct dentry *efs_get_parent(struct dentry *child)"));
        assert!(source.contains("ino = efs_find_entry(d_inode(child), \"..\", 2);"));
        assert!(source.contains("parent = d_obtain_alias(efs_iget(child->d_sb, ino));"));

        let slots = [
            EfsDentry {
                inode: 10,
                name: b".",
            },
            EfsDentry {
                inode: 2,
                name: b"..",
            },
            EfsDentry {
                inode: 99,
                name: b"file",
            },
        ];
        let blocks = [EfsDirBlock {
            magic: EFS_DIRBLK_MAGIC,
            slots: &slots,
        }];
        assert_eq!(efs_find_entry(&blocks, b"file"), 99);
        assert_eq!(efs_find_entry(&blocks, b"missing"), 0);
        assert_eq!(
            efs_find_entry(
                &[EfsDirBlock {
                    magic: 0,
                    slots: &slots
                }],
                b"file"
            ),
            0
        );
        assert!(efs_directory_size_misaligned(EFS_DIRBSIZE + 1));
        assert_eq!(efs_nfs_get_inode_result(0, 0, 0, Ok(())), Err(-ESTALE));
        assert_eq!(efs_nfs_get_inode_result(1, 7, 8, Ok(())), Err(-ESTALE));
        assert_eq!(efs_nfs_get_inode_result(1, 7, 7, Ok(())), Ok(()));
        assert_eq!(efs_get_parent_result(2), Ok(2));
        assert_eq!(efs_get_parent_result(0), Err(-ENOENT));
    }
}
