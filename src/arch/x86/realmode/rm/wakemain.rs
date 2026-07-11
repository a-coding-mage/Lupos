//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/realmode/rm/wakemain.c
//! test-origin: linux:vendor/linux/arch/x86/realmode/rm/wakemain.c
//! Real-mode wakeup speaker and video handoff.

use alloc::vec::Vec;

pub const DOT_HZ: u32 = 880;
pub const DASH_HZ: u32 = 587;
pub const US_PER_DOT: u32 = 125_000;
pub const WAKEUP_MAGIC: u32 = 0x1234_5678;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WakeMainAction {
    HangBadMagic,
    Beep { hz: u32, usec: u32 },
    SpeakerOff { usec: u32 },
    BiosLcall,
    ProbeCards,
    SetMode { mode: u16 },
}

pub fn morse_plan(pattern: &str) -> Vec<WakeMainAction> {
    let mut actions = Vec::new();
    for ch in pattern.bytes() {
        match ch {
            b'.' => {
                actions.push(WakeMainAction::Beep {
                    hz: DOT_HZ,
                    usec: US_PER_DOT,
                });
                actions.push(WakeMainAction::SpeakerOff { usec: US_PER_DOT });
            }
            b'-' => {
                actions.push(WakeMainAction::Beep {
                    hz: DASH_HZ,
                    usec: US_PER_DOT * 3,
                });
                actions.push(WakeMainAction::SpeakerOff { usec: US_PER_DOT });
            }
            _ => actions.push(WakeMainAction::SpeakerOff {
                usec: US_PER_DOT * 3,
            }),
        }
    }
    actions
}

pub fn wakemain_plan(real_magic: u32, realmode_flags: u32, video_mode: u16) -> Vec<WakeMainAction> {
    if real_magic != WAKEUP_MAGIC {
        return alloc::vec![WakeMainAction::HangBadMagic];
    }
    let mut actions = Vec::new();
    if realmode_flags & 4 != 0 {
        actions.extend(morse_plan("...-"));
    }
    if realmode_flags & 1 != 0 {
        actions.push(WakeMainAction::BiosLcall);
    }
    if realmode_flags & 2 != 0 {
        actions.push(WakeMainAction::ProbeCards);
        actions.push(WakeMainAction::SetMode { mode: video_mode });
    }
    actions
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn wakemain_flags_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/realmode/rm/wakemain.c"
        ));
        assert!(source.contains("static void udelay(int loops)"));
        assert!(source.contains("io_delay();"));
        assert!(source.contains("outb(0xb6, 0x43);"));
        assert!(source.contains("#define DOT_HZ\t\t880"));
        assert!(source.contains("#define DASH_HZ\t\t587"));
        assert!(source.contains("#define US_PER_DOT\t125000"));
        assert!(source.contains("send_morse(\"...-\")"));
        assert!(source.contains("wakeup_header.real_magic != 0x12345678"));
        assert!(source.contains("wakeup_header.realmode_flags & 1"));
        assert!(source.contains("wakeup_header.realmode_flags & 2"));
        assert!(source.contains("set_mode(wakeup_header.video_mode);"));

        assert_eq!(wakemain_plan(0, 0, 0), vec![WakeMainAction::HangBadMagic]);
        let plan = wakemain_plan(WAKEUP_MAGIC, 1 | 2, 3);
        assert_eq!(
            plan,
            vec![
                WakeMainAction::BiosLcall,
                WakeMainAction::ProbeCards,
                WakeMainAction::SetMode { mode: 3 },
            ]
        );
        assert_eq!(
            morse_plan("-")[0],
            WakeMainAction::Beep {
                hz: DASH_HZ,
                usec: US_PER_DOT * 3
            }
        );
    }
}
