//! linux-parity: complete
//! linux-source: vendor/linux/kernel/printk
//! test-origin: linux:vendor/linux/kernel/printk
//! Linux `KERN_*` syslog level + facility constants and prefix parsing.
//!
//! Mirrors `vendor/linux/include/linux/kern_levels.h` and the prefix-parse
//! helper in `vendor/linux/kernel/printk/printk.c::printk_parse_prefix`.

/// Linux syslog level constants.  Matches `KERN_*` numeric values.
pub const KERN_EMERG: u8 = 0;
pub const KERN_ALERT: u8 = 1;
pub const KERN_CRIT: u8 = 2;
pub const KERN_ERR: u8 = 3;
pub const KERN_WARNING: u8 = 4;
pub const KERN_NOTICE: u8 = 5;
pub const KERN_INFO: u8 = 6;
pub const KERN_DEBUG: u8 = 7;
pub const KERN_DEFAULT: u8 = KERN_WARNING;

/// `LOG_KERN` facility (Linux `LOG_KERN = (0<<3)`, syslog facility 0).
pub const LOG_KERN: u8 = 0;
/// `LOG_USER` facility.
pub const LOG_USER: u8 = 1;
/// `LOG_DAEMON` facility.
pub const LOG_DAEMON: u8 = 3;

/// Result of parsing a `<n>` or `<facility.level>` prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParsedPrefix {
    pub level: u8,
    pub facility: u8,
    /// Number of bytes consumed from the input.
    pub consumed: usize,
}

impl ParsedPrefix {
    pub const fn default_kernel() -> Self {
        Self {
            level: KERN_DEFAULT,
            facility: LOG_KERN,
            consumed: 0,
        }
    }
}

/// Parse a Linux-shaped log level prefix: `<n>` or `<facility.level>`.
///
/// Examples (matching `printk_parse_prefix`):
/// - `<7>foo`  → `level=7 facility=0 consumed=3`
/// - `<3>oops` → `level=3 facility=0 consumed=3`
/// - `<13>x`   → `level=5 facility=1 consumed=4` (13 = 1<<3 | 5)
/// - `foo`     → defaults, `consumed=0`
pub fn parse_prefix(input: &[u8]) -> ParsedPrefix {
    let mut p = ParsedPrefix::default_kernel();
    if input.len() < 3 {
        return p;
    }
    if input[0] != b'<' {
        return p;
    }
    // Parse digits until '>' or end.
    let mut value: u32 = 0;
    let mut i = 1;
    let mut has_digit = false;
    while i < input.len() && input[i] != b'>' {
        let c = input[i];
        if !c.is_ascii_digit() {
            return p; // invalid — bail to defaults
        }
        value = value.saturating_mul(10).saturating_add((c - b'0') as u32);
        has_digit = true;
        i += 1;
    }
    if !has_digit || i >= input.len() || input[i] != b'>' {
        return p;
    }
    // Linux: priority = facility | level → level = priority & 7, facility = priority >> 3.
    p.level = (value & 0x7) as u8;
    p.facility = (value >> 3) as u8;
    p.consumed = i + 1;
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_kern_emerg() {
        let p = parse_prefix(b"<0>panic");
        assert_eq!(p.level, KERN_EMERG);
        assert_eq!(p.facility, LOG_KERN);
        assert_eq!(p.consumed, 3);
    }

    #[test]
    fn parse_kern_debug() {
        let p = parse_prefix(b"<7>verbose");
        assert_eq!(p.level, KERN_DEBUG);
        assert_eq!(p.facility, LOG_KERN);
        assert_eq!(p.consumed, 3);
    }

    #[test]
    fn parse_user_facility_combined() {
        // priority 13 = (LOG_USER << 3) | KERN_NOTICE  → facility=1 level=5
        let p = parse_prefix(b"<13>hello");
        assert_eq!(p.level, KERN_NOTICE);
        assert_eq!(p.facility, LOG_USER);
        assert_eq!(p.consumed, 4);
    }

    #[test]
    fn no_prefix_returns_defaults() {
        let p = parse_prefix(b"plain text");
        assert_eq!(p.level, KERN_DEFAULT);
        assert_eq!(p.facility, LOG_KERN);
        assert_eq!(p.consumed, 0);
    }

    #[test]
    fn malformed_prefix_returns_defaults() {
        assert_eq!(parse_prefix(b"<>x").consumed, 0);
        assert_eq!(parse_prefix(b"<abc>x").consumed, 0);
        assert_eq!(parse_prefix(b"<7").consumed, 0);
    }
}
