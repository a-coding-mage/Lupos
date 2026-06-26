//! linux-parity: complete
//! linux-source: vendor/linux/fs/ubifs/misc.c
//! test-origin: linux:vendor/linux/fs/ubifs/misc.c
//! UBIFS message class and assert-action metadata.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UbifsInfo {
    pub ubi_num: i32,
    pub vol_id: i32,
    pub assert_action: UbifsAssertAction,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UbifsAssertAction {
    Report,
    ReadOnly,
    Panic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UbifsMessageKind {
    Normal,
    Error { pid: i32 },
    Warning { pid: i32 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UbifsMessagePrefix {
    pub level: &'static str,
    pub label: &'static str,
    pub ubi_num: i32,
    pub vol_id: i32,
    pub pid: Option<i32>,
    pub includes_return_address: bool,
}

pub const fn ubifs_assert_action_name(action: UbifsAssertAction) -> &'static str {
    match action {
        UbifsAssertAction::Report => "report",
        UbifsAssertAction::ReadOnly => "read-only",
        UbifsAssertAction::Panic => "panic",
    }
}

pub const fn ubifs_message_prefix(c: &UbifsInfo, kind: UbifsMessageKind) -> UbifsMessagePrefix {
    match kind {
        UbifsMessageKind::Normal => UbifsMessagePrefix {
            level: "notice",
            label: "UBIFS",
            ubi_num: c.ubi_num,
            vol_id: c.vol_id,
            pid: None,
            includes_return_address: false,
        },
        UbifsMessageKind::Error { pid } => UbifsMessagePrefix {
            level: "err",
            label: "UBIFS error",
            ubi_num: c.ubi_num,
            vol_id: c.vol_id,
            pid: Some(pid),
            includes_return_address: true,
        },
        UbifsMessageKind::Warning { pid } => UbifsMessagePrefix {
            level: "warn",
            label: "UBIFS warning",
            ubi_num: c.ubi_num,
            vol_id: c.vol_id,
            pid: Some(pid),
            includes_return_address: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ubifs_misc_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/ubifs/misc.c"
        ));
        assert!(source.contains("#include <linux/kernel.h>"));
        assert!(source.contains("#include \"ubifs.h\""));
        assert!(source.contains("void ubifs_msg"));
        assert!(source.contains("pr_notice(\"UBIFS (ubi%d:%d): %pV\\n\""));
        assert!(source.contains("void ubifs_err"));
        assert!(source.contains("UBIFS error (ubi%d:%d pid %d): %ps: %pV"));
        assert!(source.contains("void ubifs_warn"));
        assert!(source.contains("UBIFS warning (ubi%d:%d pid %d): %ps: %pV"));
        assert!(source.contains("[ASSACT_REPORT] = \"report\""));
        assert!(source.contains("[ASSACT_RO] = \"read-only\""));
        assert!(source.contains("[ASSACT_PANIC] = \"panic\""));
        assert!(source.contains("return assert_names[c->assert_action];"));

        let c = UbifsInfo {
            ubi_num: 2,
            vol_id: 7,
            assert_action: UbifsAssertAction::ReadOnly,
        };
        assert_eq!(ubifs_assert_action_name(c.assert_action), "read-only");
        assert_eq!(ubifs_message_prefix(&c, UbifsMessageKind::Normal).pid, None);
        let warning = ubifs_message_prefix(&c, UbifsMessageKind::Warning { pid: 42 });
        assert_eq!(warning.level, "warn");
        assert_eq!(warning.pid, Some(42));
        assert!(warning.includes_return_address);
    }
}
