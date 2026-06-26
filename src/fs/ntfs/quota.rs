//! linux-parity: complete
//! linux-source: vendor/linux/fs/ntfs/quota.c
//! test-origin: linux:vendor/linux/fs/ntfs/quota.c
//! NTFS `$Quota` out-of-date marker logic.

pub const QUOTA_VERSION: u32 = 2;
pub const QUOTA_DEFAULTS_ID: u32 = 1;
pub const QUOTA_FLAG_TRACKING_ENABLED: u32 = 0x0000_0010;
pub const QUOTA_FLAG_TRACKING_REQUESTED: u32 = 0x0000_0040;
pub const QUOTA_FLAG_OUT_OF_DATE: u32 = 0x0000_0200;
pub const QUOTA_FLAG_PENDING_DELETES: u32 = 0x0000_0800;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NtfsQuotaMark {
    pub success: bool,
    pub mark_index_dirty: bool,
    pub set_volume_out_of_date: bool,
}

pub const fn ntfs_mark_quotas_out_of_date_plan(
    volume_already_out_of_date: bool,
    quota_inode_open: bool,
    quota_q_inode_open: bool,
    defaults_entry_found: bool,
    entry_size_valid: bool,
    entry_version: u32,
    flags: u32,
) -> NtfsQuotaMark {
    if volume_already_out_of_date {
        return NtfsQuotaMark {
            success: true,
            mark_index_dirty: false,
            set_volume_out_of_date: false,
        };
    }
    if !quota_inode_open
        || !quota_q_inode_open
        || !defaults_entry_found
        || !entry_size_valid
        || entry_version != QUOTA_VERSION
    {
        return NtfsQuotaMark {
            success: false,
            mark_index_dirty: false,
            set_volume_out_of_date: false,
        };
    }

    let active_flags =
        QUOTA_FLAG_TRACKING_ENABLED | QUOTA_FLAG_TRACKING_REQUESTED | QUOTA_FLAG_PENDING_DELETES;
    NtfsQuotaMark {
        success: true,
        mark_index_dirty: flags & QUOTA_FLAG_OUT_OF_DATE == 0 && flags & active_flags != 0,
        set_volume_out_of_date: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ntfs_quota_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/ntfs/quota.c"
        ));
        assert!(source.contains("#include \"index.h\""));
        assert!(source.contains("#include \"quota.h\""));
        assert!(source.contains("bool ntfs_mark_quotas_out_of_date"));
        assert!(source.contains("const __le32 qid = QUOTA_DEFAULTS_ID;"));
        assert!(source.contains("if (NVolQuotaOutOfDate(vol))"));
        assert!(source.contains("if (!vol->quota_ino || !vol->quota_q_ino)"));
        assert!(source.contains("ntfs_index_ctx_get(NTFS_I(vol->quota_q_ino), I30, 4);"));
        assert!(source.contains("ntfs_index_lookup(&qid, sizeof(qid), ictx);"));
        assert!(source.contains("if (ictx->data_len < offsetof(struct quota_control_entry, sid))"));
        assert!(source.contains("le32_to_cpu(qce->version) != QUOTA_VERSION"));
        assert!(source.contains("if (qce->flags & QUOTA_FLAG_OUT_OF_DATE)"));
        assert!(source.contains("QUOTA_FLAG_TRACKING_ENABLED"));
        assert!(source.contains("QUOTA_FLAG_TRACKING_REQUESTED"));
        assert!(source.contains("QUOTA_FLAG_PENDING_DELETES"));
        assert!(source.contains("qce->flags |= QUOTA_FLAG_OUT_OF_DATE;"));
        assert!(source.contains("ntfs_index_entry_mark_dirty(ictx);"));
        assert!(source.contains("NVolSetQuotaOutOfDate(vol);"));

        assert_eq!(
            ntfs_mark_quotas_out_of_date_plan(true, false, false, false, false, 0, 0),
            NtfsQuotaMark {
                success: true,
                mark_index_dirty: false,
                set_volume_out_of_date: false,
            }
        );
        assert!(!ntfs_mark_quotas_out_of_date_plan(false, false, true, true, true, 2, 0).success);
        assert_eq!(
            ntfs_mark_quotas_out_of_date_plan(
                false,
                true,
                true,
                true,
                true,
                QUOTA_VERSION,
                QUOTA_FLAG_TRACKING_ENABLED
            ),
            NtfsQuotaMark {
                success: true,
                mark_index_dirty: true,
                set_volume_out_of_date: true,
            }
        );
        assert!(
            !ntfs_mark_quotas_out_of_date_plan(
                false,
                true,
                true,
                true,
                true,
                QUOTA_VERSION,
                QUOTA_FLAG_OUT_OF_DATE
            )
            .mark_index_dirty
        );
    }
}
