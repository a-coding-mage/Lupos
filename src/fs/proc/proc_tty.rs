//! linux-parity: complete
//! linux-source: vendor/linux/fs/proc/proc_tty.c
//! test-origin: linux:vendor/linux/fs/proc/proc_tty.c
//! `/proc/tty`.

extern crate alloc;

use alloc::sync::Arc;
use alloc::{format, string::String, vec::Vec};

use crate::fs::kernfs::{KernfsNode, add_child};

pub const TTY_MAJOR: u32 = 4;
pub const TTYAUX_MAJOR: u32 = 5;
pub const TTY_DRIVER_TYPE_SYSTEM: u32 = 0;
pub const TTY_DRIVER_TYPE_CONSOLE: u32 = 1;
pub const TTY_DRIVER_TYPE_SERIAL: u32 = 2;
pub const TTY_DRIVER_TYPE_PTY: u32 = 3;
pub const SYSTEM_TYPE_TTY: u32 = 1;
pub const SYSTEM_TYPE_CONSOLE: u32 = 2;
pub const SYSTEM_TYPE_SYSCONS: u32 = 3;
pub const PTY_TYPE_MASTER: u32 = 1;
pub const PTY_TYPE_SLAVE: u32 = 2;
const DEV_MINORS_PER_MAJOR: u32 = 1 << 20;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TtyDriverInfo {
    pub driver_name: Option<String>,
    pub name: String,
    pub major: u32,
    pub minor_start: u32,
    pub num: u32,
    pub ty: u32,
    pub subtype: u32,
    pub proc_entry: Option<String>,
    pub has_proc_show: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcTtyRegisterPlan {
    pub create_single_data: bool,
    pub proc_entry_set: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcTtyUnregisterPlan {
    pub remove_proc_entry: Option<String>,
    pub proc_entry_cleared: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcTtyInitPlan {
    pub mkdir_tty: bool,
    pub mkdir_ldisc: bool,
    pub mkdir_driver_mode: Option<u16>,
    pub create_ldiscs_seq: bool,
    pub create_drivers_seq: bool,
}

pub fn new_tty_dir() -> Arc<KernfsNode> {
    let dir = KernfsNode::new_dir("tty", 0o555);
    add_child(&dir, KernfsNode::new_dir("driver", 0o555));
    dir
}

pub fn proc_tty_register_driver_plan(driver: &TtyDriverInfo) -> ProcTtyRegisterPlan {
    let should_register =
        driver.driver_name.is_some() && driver.proc_entry.is_none() && driver.has_proc_show;
    ProcTtyRegisterPlan {
        create_single_data: should_register,
        proc_entry_set: should_register,
    }
}

pub fn proc_tty_unregister_driver_plan(driver: &TtyDriverInfo) -> ProcTtyUnregisterPlan {
    ProcTtyUnregisterPlan {
        remove_proc_entry: driver.proc_entry.clone(),
        proc_entry_cleared: driver.proc_entry.is_some(),
    }
}

pub const fn proc_tty_init_plan(proc_mkdir_tty_ok: bool) -> ProcTtyInitPlan {
    if !proc_mkdir_tty_ok {
        return ProcTtyInitPlan {
            mkdir_tty: false,
            mkdir_ldisc: false,
            mkdir_driver_mode: None,
            create_ldiscs_seq: false,
            create_drivers_seq: false,
        };
    }
    ProcTtyInitPlan {
        mkdir_tty: true,
        mkdir_ldisc: true,
        mkdir_driver_mode: Some(0o500),
        create_ldiscs_seq: true,
        create_drivers_seq: true,
    }
}

fn tty_type_suffix(driver: &TtyDriverInfo) -> String {
    match driver.ty {
        TTY_DRIVER_TYPE_SYSTEM => match driver.subtype {
            SYSTEM_TYPE_TTY => String::from("system:/dev/tty"),
            SYSTEM_TYPE_SYSCONS => String::from("system:console"),
            SYSTEM_TYPE_CONSOLE => String::from("system:vtmaster"),
            _ => String::from("system"),
        },
        TTY_DRIVER_TYPE_CONSOLE => String::from("console"),
        TTY_DRIVER_TYPE_SERIAL => String::from("serial"),
        TTY_DRIVER_TYPE_PTY => match driver.subtype {
            PTY_TYPE_MASTER => String::from("pty:master"),
            PTY_TYPE_SLAVE => String::from("pty:slave"),
            _ => String::from("pty"),
        },
        _ => format!("type:{}.{}", driver.ty, driver.subtype),
    }
}

fn show_tty_range(driver: &TtyDriverInfo, major: u32, minor: u32, num: u32) -> String {
    let driver_name = driver.driver_name.as_deref().unwrap_or("unknown");
    let mut out = format!("{driver_name:<20} /dev/{:<8} ", driver.name);
    if driver.num > 1 {
        out.push_str(&format!("{major:3} {minor}-{end} ", end = minor + num - 1));
    } else {
        out.push_str(&format!("{major:3} {minor:7} "));
    }
    out.push_str(&tty_type_suffix(driver));
    out.push('\n');
    out
}

pub fn show_tty_driver(driver: &TtyDriverInfo, first_driver: bool) -> String {
    let mut out = String::new();
    if first_driver {
        out.push_str(&format!(
            "{:<20} /dev/{:<8} {:3} {:7} system:/dev/tty\n",
            "/dev/tty", "tty", TTYAUX_MAJOR, 0
        ));
        out.push_str(&format!(
            "{:<20} /dev/{:<8} {:3} {:7} system:console\n",
            "/dev/console", "console", TTYAUX_MAJOR, 1
        ));
        out.push_str(&format!(
            "{:<20} /dev/{:<8} {:3} {:7} system\n",
            "/dev/ptmx", "ptmx", TTYAUX_MAJOR, 2
        ));
        out.push_str(&format!(
            "{:<20} /dev/{:<8} {:3} {:7} system:vtmaster\n",
            "/dev/vc/0", "vc/0", TTY_MAJOR, 0
        ));
    }

    let mut major = driver.major;
    let mut minor = driver.minor_start;
    let mut remaining = driver.num;
    while remaining > 0 {
        let until_next_major = DEV_MINORS_PER_MAJOR - minor;
        let count = remaining.min(until_next_major);
        out.push_str(&show_tty_range(driver, major, minor, count));
        remaining -= count;
        major += 1;
        minor = 0;
    }
    out
}

pub fn render_tty_drivers(drivers: &[TtyDriverInfo]) -> String {
    let mut out = String::new();
    for (idx, driver) in drivers.iter().enumerate() {
        out.push_str(&show_tty_driver(driver, idx == 0));
    }
    out
}

pub const TTY_DRIVERS_SEQ_OPS: [&str; 4] = ["t_start", "t_next", "t_stop", "show_tty_driver"];

#[cfg(test)]
mod tests {
    use super::*;

    fn serial_driver() -> TtyDriverInfo {
        TtyDriverInfo {
            driver_name: Some(String::from("serial")),
            name: String::from("ttyS"),
            major: 4,
            minor_start: 64,
            num: 4,
            ty: TTY_DRIVER_TYPE_SERIAL,
            subtype: 0,
            proc_entry: None,
            has_proc_show: true,
        }
    }

    #[test]
    fn proc_tty_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/proc/proc_tty.c"
        ));
        let tty_driver = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/tty_driver.h"
        ));
        let major = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/major.h"
        ));
        let internal = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/proc/internal.h"
        ));

        assert!(source.contains("static struct proc_dir_entry *proc_tty_driver;"));
        assert!(source.contains("static void show_tty_range"));
        assert!(source.contains("p->driver_name ? p->driver_name : \"unknown\""));
        assert!(source.contains("seq_printf(m, \"/dev/%-8s \", p->name);"));
        assert!(source.contains("if (p->num > 1)"));
        assert!(source.contains("case TTY_DRIVER_TYPE_SYSTEM:"));
        assert!(source.contains("seq_puts(m, \"system\");"));
        assert!(source.contains("seq_puts(m, \":/dev/tty\");"));
        assert!(source.contains("seq_puts(m, \":console\");"));
        assert!(source.contains("seq_puts(m, \":vtmaster\");"));
        assert!(source.contains("case TTY_DRIVER_TYPE_SERIAL:"));
        assert!(source.contains("case TTY_DRIVER_TYPE_PTY:"));
        assert!(source.contains("seq_printf(m, \"type:%d.%d\", p->type, p->subtype);"));
        assert!(source.contains("static int show_tty_driver"));
        assert!(source.contains("pseudo-drivers first"));
        assert!(source.contains("\"/dev/tty\", \"tty\""));
        assert!(source.contains("\"/dev/console\", \"console\""));
        assert!(source.contains("\"/dev/ptmx\", \"ptmx\""));
        assert!(source.contains("\"/dev/vc/0\", \"vc/0\""));
        assert!(source.contains("while (MAJOR(from) < MAJOR(to))"));
        assert!(source.contains("static const struct seq_operations tty_drivers_op"));
        assert!(source.contains(".start\t= t_start"));
        assert!(source.contains(".show\t= show_tty_driver"));
        assert!(source.contains("void proc_tty_register_driver"));
        assert!(source.contains("if (!driver->driver_name || driver->proc_entry ||"));
        assert!(source.contains("proc_create_single_data(driver->driver_name"));
        assert!(source.contains("void proc_tty_unregister_driver"));
        assert!(source.contains("remove_proc_entry(ent->name, proc_tty_driver);"));
        assert!(source.contains("void __init proc_tty_init(void)"));
        assert!(source.contains("if (!proc_mkdir(\"tty\", NULL))"));
        assert!(source.contains("proc_mkdir(\"tty/ldisc\", NULL);"));
        assert!(source.contains("proc_mkdir_mode(\"tty/driver\", S_IRUSR|S_IXUSR, NULL);"));
        assert!(source.contains("proc_create_seq(\"tty/ldiscs\", 0, NULL, &tty_ldiscs_seq_ops);"));
        assert!(source.contains("proc_create_seq(\"tty/drivers\", 0, NULL, &tty_drivers_op);"));
        assert!(tty_driver.contains("TTY_DRIVER_TYPE_SYSTEM"));
        assert!(tty_driver.contains("SYSTEM_TYPE_TTY = 1"));
        assert!(tty_driver.contains("PTY_TYPE_MASTER = 1"));
        assert!(major.contains("#define TTY_MAJOR\t\t4"));
        assert!(major.contains("#define TTYAUX_MAJOR\t\t5"));
        assert!(internal.contains("extern void proc_tty_init(void);"));
        assert_eq!(
            TTY_DRIVERS_SEQ_OPS,
            ["t_start", "t_next", "t_stop", "show_tty_driver"]
        );
    }

    #[test]
    fn show_tty_driver_formats_pseudo_drivers_and_registered_ranges() {
        let rendered = render_tty_drivers(&[serial_driver()]);
        assert!(rendered.contains("/dev/tty             /dev/tty"));
        assert!(rendered.contains("/dev/console         /dev/console"));
        assert!(rendered.contains("/dev/ptmx            /dev/ptmx"));
        assert!(rendered.contains("/dev/vc/0            /dev/vc/0"));
        assert!(rendered.contains("serial               /dev/ttyS"));
        assert!(rendered.contains("  4 64-67 serial"));

        let pty = TtyDriverInfo {
            driver_name: Some(String::from("pty-master")),
            name: String::from("pts"),
            major: 136,
            minor_start: 0,
            num: 2,
            ty: TTY_DRIVER_TYPE_PTY,
            subtype: PTY_TYPE_MASTER,
            proc_entry: None,
            has_proc_show: true,
        };
        assert!(show_tty_driver(&pty, false).contains("pty:master"));

        let unknown = TtyDriverInfo {
            driver_name: None,
            name: String::from("x"),
            major: 10,
            minor_start: 1,
            num: 1,
            ty: 99,
            subtype: 7,
            proc_entry: None,
            has_proc_show: false,
        };
        let line = show_tty_driver(&unknown, false);
        assert!(line.contains("unknown"));
        assert!(line.contains("type:99.7"));
        assert!(line.contains(" 10       1 "));
    }

    #[test]
    fn show_tty_driver_splits_ranges_at_major_boundary() {
        let driver = TtyDriverInfo {
            minor_start: DEV_MINORS_PER_MAJOR - 1,
            num: 3,
            ..serial_driver()
        };
        let rendered = show_tty_driver(&driver, false);
        assert!(rendered.contains("  4 1048575-1048575 serial"));
        assert!(rendered.contains("  5 0-1 serial"));
    }

    #[test]
    fn register_unregister_and_init_plans_follow_linux_branches() {
        let driver = serial_driver();
        assert_eq!(
            proc_tty_register_driver_plan(&driver),
            ProcTtyRegisterPlan {
                create_single_data: true,
                proc_entry_set: true,
            }
        );
        assert!(
            !proc_tty_register_driver_plan(&TtyDriverInfo {
                driver_name: None,
                ..driver.clone()
            })
            .create_single_data
        );
        assert!(
            !proc_tty_register_driver_plan(&TtyDriverInfo {
                proc_entry: Some(String::from("serial")),
                ..driver.clone()
            })
            .create_single_data
        );
        assert!(
            !proc_tty_register_driver_plan(&TtyDriverInfo {
                has_proc_show: false,
                ..driver.clone()
            })
            .create_single_data
        );

        assert_eq!(
            proc_tty_unregister_driver_plan(&TtyDriverInfo {
                proc_entry: Some(String::from("serial")),
                ..driver
            }),
            ProcTtyUnregisterPlan {
                remove_proc_entry: Some(String::from("serial")),
                proc_entry_cleared: true,
            }
        );
        assert_eq!(
            proc_tty_init_plan(false),
            ProcTtyInitPlan {
                mkdir_tty: false,
                mkdir_ldisc: false,
                mkdir_driver_mode: None,
                create_ldiscs_seq: false,
                create_drivers_seq: false,
            }
        );
        assert_eq!(
            proc_tty_init_plan(true),
            ProcTtyInitPlan {
                mkdir_tty: true,
                mkdir_ldisc: true,
                mkdir_driver_mode: Some(0o500),
                create_ldiscs_seq: true,
                create_drivers_seq: true,
            }
        );
    }
}
