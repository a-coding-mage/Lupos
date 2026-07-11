//! linux-parity: partial
//! linux-source: vendor/linux/fs/proc/consoles.c
//! test-origin: linux:vendor/linux/fs/proc/consoles.c
//! `/proc/consoles`.
//!
//! Ref: `vendor/linux/fs/proc/consoles.c`

use alloc::format;
use alloc::string::String;
use alloc::sync::Arc;

use crate::fs::kernfs::KernfsNode;

pub const CON_PRINTBUFFER: u16 = 1 << 0;
pub const CON_CONSDEV: u16 = 1 << 1;
pub const CON_ENABLED: u16 = 1 << 2;
pub const CON_BOOT: u16 = 1 << 3;
pub const CON_ANYTIME: u16 = 1 << 4;
pub const CON_BRL: u16 = 1 << 5;
pub const CON_NBCON: u16 = 1 << 8;

const CONSOLE_SEQ_WIDTH: usize = 20;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConsoleDevice {
    pub major: u32,
    pub minor_start: u32,
    pub index: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConsoleDev {
    pub name: &'static str,
    pub index: i32,
    pub flags: u16,
    pub read: bool,
    pub write: bool,
    pub unblank: bool,
    pub device: Option<ConsoleDevice>,
}

pub const CONSOLES_OPS_SYMBOL: &str = "consoles_op";

pub fn console_flag_string(flags: u16) -> String {
    [
        (CON_ENABLED, 'E'),
        (CON_CONSDEV, 'C'),
        (CON_BOOT, 'B'),
        (CON_NBCON, 'N'),
        (CON_PRINTBUFFER, 'p'),
        (CON_BRL, 'b'),
        (CON_ANYTIME, 'a'),
    ]
    .into_iter()
    .map(|(flag, name)| if flags & flag != 0 { name } else { ' ' })
    .collect()
}

pub fn show_console_dev(con: &ConsoleDev) -> String {
    let mut line = format!("{}{}", con.name, con.index);
    while line.len() < CONSOLE_SEQ_WIDTH {
        line.push(' ');
    }
    line.push(if con.read { 'R' } else { '-' });
    line.push(if con.flags & CON_NBCON != 0 || con.write {
        'W'
    } else {
        '-'
    });
    line.push(if con.unblank { 'U' } else { '-' });
    line.push_str(" (");
    line.push_str(&console_flag_string(con.flags));
    line.push(')');
    if let Some(device) = con.device {
        let minor = device
            .minor_start
            .saturating_add(device.index.max(0) as u32);
        line.push_str(&format!(" {:4}:{}", device.major, minor));
    }
    line.push('\n');
    line
}

pub fn render_consoles(consoles: &[ConsoleDev]) -> String {
    let mut out = String::new();
    for console in consoles {
        out.push_str(&show_console_dev(console));
    }
    out
}

pub fn proc_consoles_init_creates() -> (&'static str, u16, &'static str) {
    ("consoles", 0, CONSOLES_OPS_SYMBOL)
}

pub fn show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    super::util::copy_into(
        buf,
        &render_consoles(&[ConsoleDev {
            name: "tty",
            index: 0,
            flags: CON_ENABLED | CON_CONSDEV | CON_PRINTBUFFER,
            read: false,
            write: true,
            unblank: true,
            device: Some(ConsoleDevice {
                major: 4,
                minor_start: 0,
                index: 0,
            }),
        }]),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proc_consoles_renderer_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/proc/consoles.c"
        ));
        assert!(source.contains("{ CON_ENABLED,\t\t'E' }"));
        assert!(source.contains("{ CON_CONSDEV,\t\t'C' }"));
        assert!(source.contains("{ CON_BOOT,\t\t'B' }"));
        assert!(source.contains("{ CON_NBCON,\t\t'N' }"));
        assert!(source.contains("{ CON_PRINTBUFFER,\t'p' }"));
        assert!(source.contains("seq_setwidth(m, 21 - 1);"));
        assert!(source.contains("seq_printf(m, \"%s%d\", con->name, con->index);"));
        assert!(source.contains("con->read ? 'R' : '-'"));
        assert!(source.contains("((con->flags & CON_NBCON) || con->write) ? 'W' : '-'"));
        assert!(source.contains("if (dev)"));
        assert!(source.contains("proc_create_seq(\"consoles\", 0, NULL, &consoles_op);"));

        let tty0 = ConsoleDev {
            name: "tty",
            index: 0,
            flags: CON_ENABLED | CON_CONSDEV | CON_PRINTBUFFER,
            read: false,
            write: true,
            unblank: true,
            device: Some(ConsoleDevice {
                major: 4,
                minor_start: 0,
                index: 0,
            }),
        };
        assert_eq!(console_flag_string(tty0.flags), "EC  p  ");
        assert_eq!(
            show_console_dev(&tty0),
            "tty0                -WU (EC  p  )    4:0\n"
        );

        let nbcon = ConsoleDev {
            name: "ttyS",
            index: 1,
            flags: CON_ENABLED | CON_NBCON,
            read: true,
            write: false,
            unblank: false,
            device: None,
        };
        assert_eq!(
            show_console_dev(&nbcon),
            "ttyS1               RW- (E  N   )\n"
        );
        assert_eq!(proc_consoles_init_creates(), ("consoles", 0, "consoles_op"));
    }
}
