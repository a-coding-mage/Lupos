//! linux-parity: complete
//! linux-source: vendor/linux/lib/extable.c
//! test-origin: linux:vendor/linux/lib/extable.c
//! Generic exception table sorting, trimming, and lookup helpers.

extern crate alloc;

use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ExceptionTableEntry {
    pub insn: isize,
    pub fixup: isize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExtableLayout {
    Absolute,
    Relative { base: isize, entry_size: isize },
}

impl ExtableLayout {
    pub const fn absolute() -> Self {
        Self::Absolute
    }

    pub const fn relative(base: isize, entry_size: isize) -> Self {
        Self::Relative { base, entry_size }
    }
}

pub fn ex_to_insn(layout: ExtableLayout, index: usize, entry: &ExceptionTableEntry) -> isize {
    match layout {
        ExtableLayout::Absolute => entry.insn,
        ExtableLayout::Relative { base, entry_size } => {
            base + index as isize * entry_size + entry.insn
        }
    }
}

pub fn swap_ex_relative(
    table: &mut [ExceptionTableEntry],
    left: usize,
    right: usize,
    entry_size: isize,
) {
    let tmp = table[left];
    let delta = (right as isize - left as isize) * entry_size;

    table[left].insn = table[right].insn + delta;
    table[right].insn = tmp.insn - delta;
    table[left].fixup = table[right].fixup + delta;
    table[right].fixup = tmp.fixup - delta;
}

pub fn cmp_ex_sort(
    layout: ExtableLayout,
    left: (usize, &ExceptionTableEntry),
    right: (usize, &ExceptionTableEntry),
) -> core::cmp::Ordering {
    ex_to_insn(layout, left.0, left.1).cmp(&ex_to_insn(layout, right.0, right.1))
}

pub fn sort_extable(table: &mut [ExceptionTableEntry], layout: ExtableLayout) {
    match layout {
        ExtableLayout::Absolute => table.sort_by(|a, b| a.insn.cmp(&b.insn)),
        ExtableLayout::Relative { base, entry_size } => {
            let mut absolute: Vec<(isize, isize)> = table
                .iter()
                .enumerate()
                .map(|(index, entry)| {
                    (
                        ex_to_insn(layout, index, entry),
                        base + index as isize * entry_size + entry.fixup,
                    )
                })
                .collect();
            absolute.sort_by(|a, b| a.0.cmp(&b.0));
            for (index, entry) in table.iter_mut().enumerate() {
                let entry_addr = base + index as isize * entry_size;
                entry.insn = absolute[index].0 - entry_addr;
                entry.fixup = absolute[index].1 - entry_addr;
            }
        }
    }
}

pub fn trim_init_extable<F>(
    extable: &mut Vec<ExceptionTableEntry>,
    layout: ExtableLayout,
    mut within_module_init: F,
) where
    F: FnMut(isize) -> bool,
{
    let first_keep = extable
        .iter()
        .enumerate()
        .position(|(index, entry)| !within_module_init(ex_to_insn(layout, index, entry)))
        .unwrap_or(extable.len());
    if first_keep != 0 {
        extable.drain(0..first_keep);
    }

    while let Some((index, entry)) = extable
        .len()
        .checked_sub(1)
        .map(|index| (index, extable[index]))
    {
        if !within_module_init(ex_to_insn(layout, index, &entry)) {
            break;
        }
        extable.pop();
    }
}

pub fn cmp_ex_search(
    value: isize,
    layout: ExtableLayout,
    index: usize,
    entry: &ExceptionTableEntry,
) -> core::cmp::Ordering {
    value.cmp(&ex_to_insn(layout, index, entry))
}

pub fn search_extable(
    table: &[ExceptionTableEntry],
    layout: ExtableLayout,
    value: isize,
) -> Option<&ExceptionTableEntry> {
    table
        .binary_search_by(|entry| {
            let index = entry as *const ExceptionTableEntry as usize;
            let base = table.as_ptr() as usize;
            let slot = (index - base) / core::mem::size_of::<ExceptionTableEntry>();
            ex_to_insn(layout, slot, entry).cmp(&value)
        })
        .ok()
        .map(|index| &table[index])
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn extable_matches_linux_source_shape() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/extable.c"
        ));

        assert!(source.contains("#define ex_to_insn(x)\t((x)->insn)"));
        assert!(source.contains("static inline unsigned long ex_to_insn"));
        assert!(source.contains("static void swap_ex(void *a, void *b, int size)"));
        assert!(source.contains("x->insn = y->insn + delta;"));
        assert!(
            source.contains("sort(start, finish - start, sizeof(struct exception_table_entry),")
        );
        assert!(source.contains("while (m->num_exentries &&"));
        assert!(source.contains("within_module_init(ex_to_insn(&m->extable[0]), m)"));
        assert!(source.contains("return bsearch(&value, base, num,"));
    }

    #[test]
    fn absolute_sort_and_search_follow_instruction_order() {
        let mut table = [
            ExceptionTableEntry { insn: 30, fixup: 3 },
            ExceptionTableEntry { insn: 10, fixup: 1 },
            ExceptionTableEntry { insn: 20, fixup: 2 },
        ];

        sort_extable(&mut table, ExtableLayout::absolute());
        assert_eq!(
            table,
            [
                ExceptionTableEntry { insn: 10, fixup: 1 },
                ExceptionTableEntry { insn: 20, fixup: 2 },
                ExceptionTableEntry { insn: 30, fixup: 3 },
            ]
        );
        assert_eq!(
            search_extable(&table, ExtableLayout::absolute(), 20),
            Some(&ExceptionTableEntry { insn: 20, fixup: 2 })
        );
        assert_eq!(search_extable(&table, ExtableLayout::absolute(), 25), None);
    }

    #[test]
    fn relative_swap_preserves_absolute_instruction_and_fixup_targets() {
        let entry_size = 8;
        let layout = ExtableLayout::relative(1000, entry_size);
        let mut table = [
            ExceptionTableEntry {
                insn: 100,
                fixup: 200,
            },
            ExceptionTableEntry {
                insn: 300,
                fixup: 400,
            },
        ];
        let before = [
            ex_to_insn(layout, 0, &table[0]),
            1000 + table[0].fixup,
            ex_to_insn(layout, 1, &table[1]),
            1000 + entry_size + table[1].fixup,
        ];

        swap_ex_relative(&mut table, 0, 1, entry_size);

        assert_eq!(ex_to_insn(layout, 0, &table[0]), before[2]);
        assert_eq!(1000 + table[0].fixup, before[3]);
        assert_eq!(ex_to_insn(layout, 1, &table[1]), before[0]);
        assert_eq!(1000 + entry_size + table[1].fixup, before[1]);
    }

    #[test]
    fn trim_init_extable_removes_only_sorted_init_prefix_and_suffix() {
        let mut table = vec![
            ExceptionTableEntry { insn: 10, fixup: 0 },
            ExceptionTableEntry { insn: 20, fixup: 0 },
            ExceptionTableEntry { insn: 50, fixup: 0 },
            ExceptionTableEntry { insn: 80, fixup: 0 },
            ExceptionTableEntry { insn: 90, fixup: 0 },
        ];

        trim_init_extable(&mut table, ExtableLayout::absolute(), |insn| {
            !(30..=70).contains(&insn)
        });

        assert_eq!(table, vec![ExceptionTableEntry { insn: 50, fixup: 0 }]);
    }
}
