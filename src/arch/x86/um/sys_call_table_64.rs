//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/um/sys_call_table_64.c
//! test-origin: linux:vendor/linux/arch/x86/um/sys_call_table_64.c
//! UML x86-64 syscall table aliases.

pub const SYS_NI_SYSCALL: &str = "sys_ni_syscall";
pub const SYSCALL_TABLE_SYMBOL: &str = "sys_call_table";
pub const SYSCALL_TABLE_SIZE_SYMBOL: &str = "syscall_table_size";
pub const SYSCALL_TABLE_INCLUDE: &str = "asm/syscalls_64.h";
pub const SYSCALL_TABLE_ENTRIES: usize = 472;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SyscallAlias {
    pub symbol: &'static str,
    pub replacement: &'static str,
}

pub const HARDWARE_UNSUPPORTED_ALIASES: &[SyscallAlias] = &[
    SyscallAlias {
        symbol: "sys_iopl",
        replacement: SYS_NI_SYSCALL,
    },
    SyscallAlias {
        symbol: "sys_ioperm",
        replacement: SYS_NI_SYSCALL,
    },
];

pub fn uml_syscall_symbol(symbol: &str) -> &str {
    match symbol {
        "sys_iopl" | "sys_ioperm" => SYS_NI_SYSCALL,
        _ => symbol,
    }
}

pub const fn syscall_table_size(pointer_size: usize) -> usize {
    SYSCALL_TABLE_ENTRIES * pointer_size
}

#[cfg(test)]
mod tests {
    use super::*;

    fn highest_syscall_number(table: &str, accepted_abis: &[&str]) -> usize {
        table
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    return None;
                }

                let mut columns = line.split_whitespace();
                let nr = columns.next()?.parse::<usize>().ok()?;
                let abi = columns.next()?;
                if accepted_abis.iter().any(|accepted| *accepted == abi) {
                    Some(nr)
                } else {
                    None
                }
            })
            .max()
            .unwrap()
    }

    fn table_has_symbol(table: &str, nr: usize, name: &str, symbol: &str) -> bool {
        table.lines().any(|line| {
            let mut columns = line.split_whitespace();
            let parsed_nr = columns.next().and_then(|value| value.parse::<usize>().ok());
            let _abi = columns.next();
            let parsed_name = columns.next();
            let parsed_symbol = columns.next();

            parsed_nr == Some(nr) && parsed_name == Some(name) && parsed_symbol == Some(symbol)
        })
    }

    #[test]
    fn uml_64_aliases_hardware_syscalls_to_ni() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/um/sys_call_table_64.c"
        ));
        assert!(source.contains("#define sys_iopl sys_ni_syscall"));
        assert!(source.contains("#define sys_ioperm sys_ni_syscall"));

        assert_eq!(uml_syscall_symbol("sys_iopl"), SYS_NI_SYSCALL);
        assert_eq!(uml_syscall_symbol("sys_ioperm"), SYS_NI_SYSCALL);
        assert_eq!(uml_syscall_symbol("sys_read"), "sys_read");
        assert_eq!(
            HARDWARE_UNSUPPORTED_ALIASES,
            &[
                SyscallAlias {
                    symbol: "sys_iopl",
                    replacement: SYS_NI_SYSCALL
                },
                SyscallAlias {
                    symbol: "sys_ioperm",
                    replacement: SYS_NI_SYSCALL
                },
            ]
        );
    }

    #[test]
    fn uml_64_table_shape_matches_generated_header_input() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/um/sys_call_table_64.c"
        ));
        let table = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/entry/syscalls/syscall_64.tbl"
        ));

        assert!(source.contains("#include <asm/syscall.h>"));
        assert!(source.contains("#include <asm/syscalls_64.h>"));
        assert!(source.contains("const sys_call_ptr_t sys_call_table[]"));
        assert!(source.contains("int syscall_table_size = sizeof(sys_call_table);"));
        assert_eq!(
            highest_syscall_number(table, &["common", "64"]) + 1,
            SYSCALL_TABLE_ENTRIES
        );
        assert!(table_has_symbol(table, 172, "iopl", "sys_iopl"));
        assert!(table_has_symbol(table, 173, "ioperm", "sys_ioperm"));
        assert_eq!(syscall_table_size(8), 3776);
        assert_eq!(SYSCALL_TABLE_SYMBOL, "sys_call_table");
        assert_eq!(SYSCALL_TABLE_SIZE_SYMBOL, "syscall_table_size");
        assert_eq!(SYSCALL_TABLE_INCLUDE, "asm/syscalls_64.h");
    }
}
