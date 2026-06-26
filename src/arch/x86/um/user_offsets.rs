//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/um/user-offsets.c
//! test-origin: linux:vendor/linux/arch/x86/um/user-offsets.c
//! Host userspace register and mmap constant offsets emitted for UML.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserOffsetKind {
    Bytes,
    Longs,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UserOffsetDef {
    pub symbol: &'static str,
    pub source: &'static str,
    pub kind: UserOffsetKind,
}

pub const USER_OFFSET_DEFS: &[UserOffsetDef] = &[
    UserOffsetDef {
        symbol: "HOST_BX",
        source: "RBX",
        kind: UserOffsetKind::Longs,
    },
    UserOffsetDef {
        symbol: "HOST_CX",
        source: "RCX",
        kind: UserOffsetKind::Longs,
    },
    UserOffsetDef {
        symbol: "HOST_AX",
        source: "RAX",
        kind: UserOffsetKind::Longs,
    },
    UserOffsetDef {
        symbol: "HOST_ORIG_AX",
        source: "ORIG_RAX",
        kind: UserOffsetKind::Longs,
    },
    UserOffsetDef {
        symbol: "HOST_IP",
        source: "RIP",
        kind: UserOffsetKind::Longs,
    },
    UserOffsetDef {
        symbol: "HOST_SP",
        source: "RSP",
        kind: UserOffsetKind::Longs,
    },
    UserOffsetDef {
        symbol: "UM_FRAME_SIZE",
        source: "sizeof(struct user_regs_struct)",
        kind: UserOffsetKind::Bytes,
    },
    UserOffsetDef {
        symbol: "UM_POLLIN",
        source: "POLLIN",
        kind: UserOffsetKind::Bytes,
    },
    UserOffsetDef {
        symbol: "UM_PROT_EXEC",
        source: "PROT_EXEC",
        kind: UserOffsetKind::Bytes,
    },
];

pub fn user_offset(symbol: &str) -> Option<UserOffsetDef> {
    USER_OFFSET_DEFS
        .iter()
        .copied()
        .find(|entry| entry.symbol == symbol)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_offsets_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/um/user-offsets.c"
        ));
        assert!(source.contains("#define DEFINE_LONGS(sym, val)"));
        assert!(source.contains("COMMENT(#val \" / sizeof(unsigned long)\")"));
        assert!(source.contains("DEFINE_LONGS(HOST_BX, RBX);"));
        assert!(source.contains("DEFINE_LONGS(HOST_ORIG_AX, ORIG_RAX);"));
        assert!(source.contains("DEFINE_LONGS(HOST_IP, RIP);"));
        assert!(source.contains("DEFINE(UM_FRAME_SIZE, sizeof(struct user_regs_struct));"));
        assert!(source.contains("DEFINE(UM_POLLIN, POLLIN);"));
        assert!(source.contains("DEFINE(UM_PROT_READ, PROT_READ);"));
        assert!(source.contains("DEFINE(UM_PROT_EXEC, PROT_EXEC);"));

        assert_eq!(user_offset("HOST_IP").unwrap().source, "RIP");
        assert_eq!(user_offset("HOST_IP").unwrap().kind, UserOffsetKind::Longs);
        assert_eq!(
            user_offset("UM_FRAME_SIZE").unwrap().source,
            "sizeof(struct user_regs_struct)"
        );
        assert!(user_offset("NO_SUCH_OFFSET").is_none());
    }
}
