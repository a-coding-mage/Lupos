//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/purgatory/purgatory.c
//! test-origin: linux:vendor/linux/arch/x86/purgatory/purgatory.c
//! kexec purgatory handoff validation.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/purgatory/purgatory.c

use crate::include::uapi::errno::EINVAL;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PurgatoryHandoff {
    pub entry: u64,
    pub image_start: u64,
    pub image_end: u64,
    pub digest_expected: u64,
    pub digest_actual: u64,
}

pub const fn purgatory_entry_in_image(handoff: PurgatoryHandoff) -> bool {
    handoff.image_start < handoff.image_end
        && handoff.entry >= handoff.image_start
        && handoff.entry < handoff.image_end
}

pub const fn purgatory_validate_handoff(handoff: PurgatoryHandoff) -> Result<(), i32> {
    if !purgatory_entry_in_image(handoff) {
        return Err(EINVAL);
    }
    if handoff.digest_expected != handoff.digest_actual {
        return Err(EINVAL);
    }
    Ok(())
}

pub const fn purgatory_should_jump(handoff: PurgatoryHandoff) -> bool {
    matches!(purgatory_validate_handoff(handoff), Ok(()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handoff_requires_entry_inside_verified_image() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/purgatory/purgatory.c"
        ));
        assert!(source.contains("purgatory_sha256_digest"));
        assert!(source.contains("purgatory_sha_regions"));
        assert!(source.contains("sha256_init(&sctx);"));
        assert!(source.contains("sha256_update(&sctx"));
        assert!(source.contains("sha256_final(&sctx, digest);"));
        assert!(source.contains("memcmp(digest, purgatory_sha256_digest"));
        assert!(source.contains("void purgatory(void)"));
        assert!(source.contains("for (;;)"));

        let good = PurgatoryHandoff {
            entry: 0x1200,
            image_start: 0x1000,
            image_end: 0x2000,
            digest_expected: 7,
            digest_actual: 7,
        };
        assert!(purgatory_should_jump(good));
        assert_eq!(
            purgatory_validate_handoff(PurgatoryHandoff {
                entry: 0x2000,
                ..good
            }),
            Err(EINVAL)
        );
        assert_eq!(
            purgatory_validate_handoff(PurgatoryHandoff {
                digest_actual: 8,
                ..good
            }),
            Err(EINVAL)
        );
    }
}
