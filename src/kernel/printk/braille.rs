//! linux-parity: complete
//! linux-source: vendor/linux/kernel/printk/braille.c
//! test-origin: linux:vendor/linux/kernel/printk/braille.c
//! Braille console driver bindings.
//!
//! Linux's braille console setup recognizes both `brl,` and `brl=` console
//! option forms and marks registered consoles with `CON_BRL`.
//!
//! Ref: vendor/linux/kernel/printk/braille.c

use crate::include::uapi::errno::EINVAL;

pub const CON_BRL: u32 = 1 << 5;
pub const BRL_COMMA_PREFIX: &str = "brl,";
pub const BRL_EQUALS_PREFIX: &str = "brl=";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BrailleConsoleSetup<'a> {
    pub serial_options: &'a str,
    pub brl_options: Option<&'a str>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BrailleConsole {
    pub flags: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BrailleConsoleCmdline<'a> {
    pub index: i32,
    pub options: Option<&'a str>,
    pub brl_options: Option<&'a str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BrailleRegistration<'a> {
    pub index: i32,
    pub options: Option<&'a str>,
    pub brl_options: &'a str,
}

/// `_braille_console_setup` parses and rewrites console options.
pub fn console_setup(options: &str) -> Result<BrailleConsoleSetup<'_>, i32> {
    if let Some(serial_options) = options.strip_prefix(BRL_COMMA_PREFIX) {
        return Ok(BrailleConsoleSetup {
            serial_options,
            brl_options: Some(""),
        });
    }

    if let Some(rest) = options.strip_prefix(BRL_EQUALS_PREFIX) {
        let Some((brl_options, serial_options)) = rest.split_once(',') else {
            return Err(-EINVAL);
        };
        return Ok(BrailleConsoleSetup {
            serial_options,
            brl_options: Some(brl_options),
        });
    }

    Ok(BrailleConsoleSetup {
        serial_options: options,
        brl_options: None,
    })
}

pub fn register_console<'a>(
    console: &mut BrailleConsole,
    cmdline: BrailleConsoleCmdline<'a>,
    register_ret: i32,
) -> (i32, Option<BrailleRegistration<'a>>) {
    let Some(brl_options) = cmdline.brl_options else {
        return (0, None);
    };

    console.flags |= CON_BRL;
    (
        register_ret,
        Some(BrailleRegistration {
            index: cmdline.index,
            options: cmdline.options,
            brl_options,
        }),
    )
}

pub const fn unregister_console(console: BrailleConsole, unregister_ret: i32) -> i32 {
    if console.flags & CON_BRL != 0 {
        unregister_ret
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn braille_source_contract_matches_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/printk/braille.c"
        ));
        assert!(source.contains("str_has_prefix(*str, \"brl,\")"));
        assert!(source.contains("*brl_options = \"\";"));
        assert!(source.contains("*str += len;"));
        assert!(source.contains("str_has_prefix(*str, \"brl=\")"));
        assert!(source.contains("*brl_options = *str + len;"));
        assert!(source.contains("*str = strchr(*brl_options, ',');"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains("*((*str)++) = 0;"));
        assert!(source.contains("console->flags |= CON_BRL;"));
        assert!(source.contains("braille_register_console(console, c->index, c->options"));
        assert!(source.contains("if (console->flags & CON_BRL)"));
        assert!(source.contains("braille_unregister_console(console);"));
    }

    #[test]
    fn console_setup_matches_brl_comma_form() {
        assert_eq!(
            console_setup("brl,ttyS0,115200").unwrap(),
            BrailleConsoleSetup {
                serial_options: "ttyS0,115200",
                brl_options: Some("")
            }
        );
    }

    #[test]
    fn console_setup_matches_brl_equals_form() {
        assert_eq!(
            console_setup("brl=ttyB0,ttyS0,115200").unwrap(),
            BrailleConsoleSetup {
                serial_options: "ttyS0,115200",
                brl_options: Some("ttyB0")
            }
        );
        assert_eq!(console_setup("brl=ttyB0"), Err(-EINVAL));
    }

    #[test]
    fn console_setup_leaves_non_braille_options_unchanged() {
        assert_eq!(
            console_setup("ttyS0,115200").unwrap(),
            BrailleConsoleSetup {
                serial_options: "ttyS0,115200",
                brl_options: None
            }
        );
    }

    #[test]
    fn register_and_unregister_follow_con_brl_flag() {
        let mut console = BrailleConsole::default();
        let cmdline = BrailleConsoleCmdline {
            index: 1,
            options: Some("ttyS0,115200"),
            brl_options: Some("ttyB0"),
        };
        let (ret, registration) = register_console(&mut console, cmdline, -7);
        assert_eq!(ret, -7);
        assert_eq!(console.flags & CON_BRL, CON_BRL);
        assert_eq!(
            registration,
            Some(BrailleRegistration {
                index: 1,
                options: Some("ttyS0,115200"),
                brl_options: "ttyB0"
            })
        );
        assert_eq!(unregister_console(console, -3), -3);

        let mut plain = BrailleConsole::default();
        let (ret, registration) = register_console(
            &mut plain,
            BrailleConsoleCmdline {
                index: 0,
                options: None,
                brl_options: None,
            },
            -7,
        );
        assert_eq!(ret, 0);
        assert_eq!(registration, None);
        assert_eq!(plain.flags & CON_BRL, 0);
        assert_eq!(unregister_console(plain, -3), 0);
    }
}
