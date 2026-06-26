//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/um/sys_call_table_32.c
//! test-origin: linux:vendor/linux/arch/x86/um/sys_call_table_32.c
//! UML i386 syscall table aliases.

pub const SYS_NI_SYSCALL: &str = "sys_ni_syscall";
pub const SYSCALL_TABLE_SYMBOL: &str = "sys_call_table";
pub const SYSCALL_TABLE_SIZE_SYMBOL: &str = "syscall_table_size";
pub const SYSCALL_TABLE_INCLUDE: &str = "asm/syscalls_32.h";
pub const SYSCALL_TABLE_ENTRIES: usize = 472;
pub const COMPAT_ENTRIES_USE_NATIVE: bool = true;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SyscallAlias {
    pub symbol: &'static str,
    pub replacement: &'static str,
}

pub const UML_UNSUPPORTED_ALIASES: &[SyscallAlias] = &[
    SyscallAlias {
        symbol: "sys_iopl",
        replacement: SYS_NI_SYSCALL,
    },
    SyscallAlias {
        symbol: "sys_ioperm",
        replacement: SYS_NI_SYSCALL,
    },
    SyscallAlias {
        symbol: "sys_vm86old",
        replacement: SYS_NI_SYSCALL,
    },
    SyscallAlias {
        symbol: "sys_vm86",
        replacement: SYS_NI_SYSCALL,
    },
];

pub fn uml_syscall_symbol(symbol: &str) -> &str {
    match symbol {
        "sys_iopl" | "sys_ioperm" | "sys_vm86old" | "sys_vm86" => SYS_NI_SYSCALL,
        _ => symbol,
    }
}

pub fn uml_syscall_with_compat_symbol<'a>(native: &'a str, _compat: &str) -> &'a str {
    uml_syscall_symbol(native)
}

pub const fn syscall_table_size(pointer_size: usize) -> usize {
    SYSCALL_TABLE_ENTRIES * pointer_size
}

#[cfg(test)]
mod tests {
    use super::*;

    fn highest_syscall_number(table: &str, accepted_abi: &str) -> usize {
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
                if abi == accepted_abi { Some(nr) } else { None }
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
    fn uml_32_aliases_unsupported_syscalls_to_ni() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/um/sys_call_table_32.c"
        ));
        assert!(source.contains("#define sys_iopl sys_ni_syscall"));
        assert!(source.contains("#define sys_ioperm sys_ni_syscall"));
        assert!(source.contains("#define sys_vm86old sys_ni_syscall"));
        assert!(source.contains("#define sys_vm86 sys_ni_syscall"));

        assert_eq!(uml_syscall_symbol("sys_iopl"), SYS_NI_SYSCALL);
        assert_eq!(uml_syscall_symbol("sys_ioperm"), SYS_NI_SYSCALL);
        assert_eq!(uml_syscall_symbol("sys_vm86old"), SYS_NI_SYSCALL);
        assert_eq!(uml_syscall_symbol("sys_vm86"), SYS_NI_SYSCALL);
        assert_eq!(uml_syscall_symbol("sys_read"), "sys_read");
        assert_eq!(UML_UNSUPPORTED_ALIASES.len(), 4);
    }

    #[test]
    fn uml_32_compat_entries_follow_native_symbol() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/um/sys_call_table_32.c"
        ));
        assert!(source.contains("#define __SYSCALL_WITH_COMPAT(nr, native, compat)"));
        assert!(source.contains("__SYSCALL(nr, native)"));
        assert!(COMPAT_ENTRIES_USE_NATIVE);

        assert_eq!(
            uml_syscall_with_compat_symbol("sys_open", "compat_sys_open"),
            "sys_open"
        );
        assert_eq!(
            uml_syscall_with_compat_symbol("sys_vm86", "sys_ni_syscall"),
            SYS_NI_SYSCALL
        );
    }

    #[test]
    fn uml_32_table_shape_matches_generated_header_input() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/um/sys_call_table_32.c"
        ));
        let table = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/entry/syscalls/syscall_32.tbl"
        ));

        assert!(source.contains("#include <asm/syscall.h>"));
        assert!(source.contains("#include <asm/syscalls_32.h>"));
        assert!(source.contains("const sys_call_ptr_t sys_call_table[]"));
        assert!(source.contains("int syscall_table_size = sizeof(sys_call_table);"));
        assert_eq!(
            highest_syscall_number(table, "i386") + 1,
            SYSCALL_TABLE_ENTRIES
        );
        assert!(table_has_symbol(table, 101, "ioperm", "sys_ioperm"));
        assert!(table_has_symbol(table, 110, "iopl", "sys_iopl"));
        assert!(table_has_symbol(table, 113, "vm86old", "sys_vm86old"));
        assert!(table_has_symbol(table, 166, "vm86", "sys_vm86"));
        assert_eq!(syscall_table_size(4), 1888);
        assert_eq!(SYSCALL_TABLE_SYMBOL, "sys_call_table");
        assert_eq!(SYSCALL_TABLE_SIZE_SYMBOL, "syscall_table_size");
        assert_eq!(SYSCALL_TABLE_INCLUDE, "asm/syscalls_32.h");
    }
}
