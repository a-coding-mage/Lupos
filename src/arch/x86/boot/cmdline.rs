//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/cmdline.c
//! test-origin: linux:vendor/linux/arch/x86/boot/cmdline.c
//! Real-mode setup command-line parser.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/cmdline.c
//!
//! Faithful 1:1 ports of every function in cmdline.c: [`myisspace`],
//! [`__cmdline_find_option`] and [`__cmdline_find_option_bool`]. Linux walks
//! the command line from a real-mode far pointer with `set_fs`/`rdfs8`; lupos
//! passes the command-line bytes as a NUL-terminated `&[u8]`, which is the
//! same byte source the real-mode helper reads (the slice index replaces
//! `rdfs8(cptr++)`). The parser state machines, return values (`__find_option`
//! returns the argument length regardless of truncation, or -1; `_bool`
//! returns the 1-based word position, or 0) are reproduced exactly.
//!
//! The remaining items ([`cmdline_find_option`]/[`cmdline_has_option`] slice
//! conveniences and the E820 summarisers) are lupos adapters used by the
//! GRUB/Linux boot-protocol path; they are not part of cmdline.c and are kept here
//! because the boot subsystem already depends on them.

use crate::arch::x86::include::uapi::asm::bootparam::{BootE820Entry, BootParams, E820_MAX};

pub const E820_TYPE_RAM: u32 = 1;
pub const E820_TYPE_RESERVED: u32 = 2;
pub const E820_TYPE_ACPI: u32 = 3;
pub const E820_TYPE_NVS: u32 = 4;
pub const E820_TYPE_UNUSABLE: u32 = 5;

/// `myisspace(c)` — cmdline.c lines 15-18: "close enough" whitespace test,
/// any byte <= space.
#[inline]
pub fn myisspace(c: u8) -> bool {
    c <= b' '
}

/// Parser state shared by both finders (cmdline.c `enum { st_* }`).
#[derive(PartialEq, Eq, Clone, Copy)]
enum CmdlineState {
    WordStart,
    WordCmp,
    WordSkip,
    BufCpy,
}

/// `__cmdline_find_option(cmdline, option, buffer, bufsize)` — cmdline.c
/// lines 28-92. Find a non-boolean `option=argument` and copy the argument
/// into `buffer` (truncated to fit). Returns the argument length regardless
/// of truncation, or -1 if not found. `cmdline` is the NUL-terminated
/// command line; `bufsize` is `buffer.len()`.
#[allow(non_snake_case)]
pub fn __cmdline_find_option(cmdline: &[u8], option: &[u8], buffer: &mut [u8]) -> i32 {
    use CmdlineState::*;
    let bufsize = buffer.len() as i32;
    let mut len: i32 = -1;
    let mut opptr = 0usize; // index into `option`
    let mut bufptr = 0usize; // write cursor into `buffer`
    let mut state = WordStart;

    let mut i = 0usize;
    while i < cmdline.len() {
        let c = cmdline[i];
        i += 1;
        if c == 0 {
            break; // C loop ends when rdfs8 returns the NUL terminator.
        }
        // Inner loop only re-runs to emulate the C `fallthrough` from
        // st_wordstart into st_wordcmp for the same character.
        loop {
            match state {
                WordStart => {
                    if myisspace(c) {
                        break;
                    }
                    state = WordCmp;
                    opptr = 0;
                    continue; // fallthrough
                }
                WordCmp => {
                    if c == b'=' && opptr >= option.len() {
                        len = 0;
                        bufptr = 0;
                        state = BufCpy;
                    } else if myisspace(c) {
                        state = WordStart;
                    } else if opptr >= option.len() || c != option[opptr] {
                        state = WordSkip;
                    } else {
                        opptr += 1;
                    }
                    break;
                }
                WordSkip => {
                    if myisspace(c) {
                        state = WordStart;
                    }
                    break;
                }
                BufCpy => {
                    if myisspace(c) {
                        state = WordStart;
                    } else {
                        if len < bufsize - 1 {
                            buffer[bufptr] = c;
                            bufptr += 1;
                        }
                        len += 1;
                    }
                    break;
                }
            }
        }
    }

    if bufsize != 0 {
        buffer[bufptr] = 0; // *bufptr = '\0'
    }
    len
}

/// `__cmdline_find_option_bool(cmdline, option)` — cmdline.c lines 100-156.
/// Find a boolean option (e.g. `quiet`, `nosmp`). Returns the 1-based word
/// position, or 0 if not found (and -1 only if there is no command line).
#[allow(non_snake_case)]
pub fn __cmdline_find_option_bool(cmdline: &[u8], option: &[u8]) -> i32 {
    use CmdlineState::*;
    if cmdline.is_empty() {
        return -1; // C: if (!cmdline_ptr) return -1;
    }
    let mut pos = 0i32;
    let mut wstart = 0i32;
    let mut opptr = 0usize;
    let mut state = WordStart;

    let mut i = 0usize;
    // Read one byte past a missing NUL so an unterminated slice still
    // terminates via the `c == 0` arms (mirrors rdfs8 reading the NUL).
    while i <= cmdline.len() {
        let c = cmdline.get(i).copied().unwrap_or(0);
        i += 1;
        pos += 1;
        loop {
            match state {
                WordStart => {
                    if c == 0 {
                        return 0;
                    } else if myisspace(c) {
                        break;
                    }
                    state = WordCmp;
                    opptr = 0;
                    wstart = pos;
                    continue; // fallthrough
                }
                WordCmp => {
                    if opptr >= option.len() {
                        if c == 0 || myisspace(c) {
                            return wstart;
                        }
                        state = WordSkip;
                    } else if c == 0 {
                        return 0;
                    } else if c != option[opptr] {
                        state = WordSkip;
                    } else {
                        opptr += 1;
                    }
                    break;
                }
                WordSkip => {
                    if c == 0 {
                        return 0;
                    } else if myisspace(c) {
                        state = WordStart;
                    }
                    break;
                }
                BufCpy => break, // unused by the bool finder
            }
        }
    }
    0
}

/// Find a Linux boot command-line option and return its value slice.
///
/// Matches the `cmdline_find_option()` style used by the decompressor: options
/// are separated by ASCII whitespace, `name=value` returns `value`, and a bare
/// `name` is reported as present with an empty value.
pub fn cmdline_find_option<'a>(cmdline: &'a [u8], name: &str) -> Option<&'a [u8]> {
    if name.is_empty() {
        return None;
    }
    let name = name.as_bytes();
    let end = cmdline
        .iter()
        .position(|b| *b == 0)
        .unwrap_or(cmdline.len());
    for token in cmdline[..end]
        .split(|b| *b == b' ' || *b == b'\t' || *b == b'\n' || *b == 0)
        .filter(|token| !token.is_empty())
    {
        if token == name {
            return Some(&[]);
        }
        if token.len() > name.len()
            && token.starts_with(name)
            && token.get(name.len()) == Some(&b'=')
        {
            return Some(&token[name.len() + 1..]);
        }
    }
    None
}

pub fn cmdline_has_option(cmdline: &[u8], name: &str) -> bool {
    cmdline_find_option(cmdline, name).is_some()
}

/// Count usable RAM bytes in the Linux E820 table carried by `boot_params`.
///
/// Linux keeps non-RAM entries in the table so later subsystems can reserve
/// firmware and ACPI ranges. For early allocator sizing we only sum type 1.
pub fn e820_usable_bytes(params: &BootParams) -> u64 {
    params
        .e820_iter()
        .filter(|entry| entry.region_type == E820_TYPE_RAM)
        .map(|entry| entry.length)
        .fold(0u64, u64::saturating_add)
}

/// Return the first usable RAM base address from the boot E820 table.
pub fn e820_first_usable_base(params: &BootParams) -> Option<u64> {
    params
        .e820_iter()
        .filter(|entry| entry.region_type == E820_TYPE_RAM && e820_entry_valid(*entry))
        .map(|entry| entry.base_addr)
        .min()
}

/// Linux early setup treats `earlyprintk` and `earlycon` as serial-console hints.
pub fn early_printk_requested(cmdline: &[u8]) -> bool {
    cmdline_has_option(cmdline, "earlyprintk") || cmdline_has_option(cmdline, "earlycon")
}

/// Return true if an E820 entry is well formed enough for early boot use.
pub fn e820_entry_valid(entry: BootE820Entry) -> bool {
    entry.length != 0 && entry.base_addr.checked_add(entry.length).is_some()
}

/// Clamp a caller-supplied E820 count to Linux's fixed boot-protocol table.
pub const fn clamp_e820_entries(count: usize) -> usize {
    if count > E820_MAX { E820_MAX } else { count }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Faithful cmdline.c `__cmdline_find_option`: copies the argument into the
    // caller buffer and returns its length (regardless of truncation), -1 if
    // absent. Behaviour asserted against cmdline.c lines 28-92.
    #[test]
    fn faithful_find_option_copies_argument_and_returns_length() {
        let cmdline = b"root=/dev/vda1 quiet console=ttyS0 nokaslr\0";
        let mut buf = [0u8; 32];
        let n = __cmdline_find_option(cmdline, b"console", &mut buf);
        assert_eq!(n, 5);
        assert_eq!(&buf[..5], b"ttyS0");
        let n = __cmdline_find_option(cmdline, b"root", &mut buf);
        assert_eq!(n, 9);
        assert_eq!(&buf[..9], b"/dev/vda1");
        // A bare boolean word is not an `=` option → not found here.
        assert_eq!(__cmdline_find_option(cmdline, b"quiet", &mut buf), -1);
        assert_eq!(__cmdline_find_option(cmdline, b"missing", &mut buf), -1);
    }

    #[test]
    fn faithful_find_option_reports_full_len_when_truncated() {
        let cmdline = b"console=ttyS0\0";
        let mut buf = [0u8; 4]; // room for 3 chars + NUL
        let n = __cmdline_find_option(cmdline, b"console", &mut buf);
        assert_eq!(n, 5); // full argument length even though truncated
        assert_eq!(&buf, b"tty\0");
    }

    #[test]
    fn faithful_find_option_bool_returns_word_position() {
        let cmdline = b"quiet console=ttyS0 nosmp\0";
        // "quiet" is the first word → position 1.
        assert_eq!(__cmdline_find_option_bool(cmdline, b"quiet"), 1);
        // "nosmp" is present as a standalone word → found (>0).
        assert!(__cmdline_find_option_bool(cmdline, b"nosmp") > 0);
        // "console" appears only as "console=..." → not a boolean option.
        assert_eq!(__cmdline_find_option_bool(cmdline, b"console"), 0);
        assert_eq!(__cmdline_find_option_bool(cmdline, b"absent"), 0);
        // No command line at all → -1 (distinct from "not found" 0).
        assert_eq!(__cmdline_find_option_bool(b"", b"quiet"), -1);
    }

    // lupos slice-based convenience adapter (NOT cmdline.c): returns a borrow
    // into the command line instead of copying. Used by legacy.rs / early
    // serial console.
    #[test]
    fn lupos_slice_find_option_adapter_parses_tokens() {
        let cmdline = b"root=/dev/vda1 quiet console=ttyS0 nokaslr\0ignored=yes";
        assert_eq!(
            cmdline_find_option(cmdline, "root"),
            Some(&b"/dev/vda1"[..])
        );
        assert_eq!(cmdline_find_option(cmdline, "console"), Some(&b"ttyS0"[..]));
        assert_eq!(cmdline_find_option(cmdline, "nokaslr"), Some(&b""[..]));
        assert_eq!(cmdline_find_option(cmdline, "ignored"), None);
    }

    #[test]
    fn e820_counts_only_usable_ram() {
        let mut params = BootParams::new();
        params.set_e820_entry(
            0,
            BootE820Entry {
                base_addr: 0x1000,
                length: 0x2000,
                region_type: E820_TYPE_RAM,
            },
        );
        params.set_e820_entry(
            1,
            BootE820Entry {
                base_addr: 0x3000,
                length: 0x1000,
                region_type: E820_TYPE_RESERVED,
            },
        );
        params.set_e820_entries(2);
        assert_eq!(e820_usable_bytes(&params), 0x2000);
    }

    #[test]
    fn e820_validation_rejects_zero_and_overflow() {
        assert!(!e820_entry_valid(BootE820Entry {
            base_addr: 0,
            length: 0,
            region_type: E820_TYPE_RAM,
        }));
        assert!(!e820_entry_valid(BootE820Entry {
            base_addr: u64::MAX,
            length: 1,
            region_type: E820_TYPE_RAM,
        }));
        assert!(e820_entry_valid(BootE820Entry {
            base_addr: 0x1000,
            length: 0x1000,
            region_type: E820_TYPE_RAM,
        }));
    }

    #[test]
    fn e820_first_usable_base_ignores_reserved_ranges() {
        let mut params = BootParams::new();
        params.set_e820_entry(
            0,
            BootE820Entry {
                base_addr: 0x0,
                length: 0x1000,
                region_type: E820_TYPE_RESERVED,
            },
        );
        params.set_e820_entry(
            1,
            BootE820Entry {
                base_addr: 0x2000,
                length: 0x1000,
                region_type: E820_TYPE_RAM,
            },
        );
        params.set_e820_entries(2);
        assert_eq!(e820_first_usable_base(&params), Some(0x2000));
    }

    #[test]
    fn early_printk_matches_linux_boot_aliases() {
        assert!(early_printk_requested(b"earlycon=uart8250\0"));
        assert!(early_printk_requested(b"earlyprintk=serial\0"));
        assert!(!early_printk_requested(b"console=ttyS0\0"));
    }
}
