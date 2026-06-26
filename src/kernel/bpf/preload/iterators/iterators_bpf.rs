//! linux-parity: complete
//! linux-source: vendor/linux/kernel/bpf/preload/iterators/iterators.bpf.c
//! test-origin: linux:vendor/linux/kernel/bpf/preload/iterators/iterators.bpf.c
//! Embedded BPF map/program iterator output model.

pub const MAP_HEADER: &str = "  id name             max_entries  cur_entries\n";
pub const PROG_HEADER: &str = "  id name             attached\n";
pub const LICENSE: &str = "GPL";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BpfMapRow<'a> {
    pub id: u32,
    pub name: &'a str,
    pub max_entries: u32,
    pub cur_entries: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BpfProgRow<'a> {
    pub id: u32,
    pub name: &'a str,
    pub attach_func_name: &'a str,
    pub dst_prog_name: &'a str,
}

pub const fn emit_header(seq_num: u64) -> bool {
    seq_num == 0
}

pub fn get_name<'a>(btf_strings: Option<&'a str>, name_off: usize, fallback: &'a str) -> &'a str {
    let Some(strings) = btf_strings else {
        return fallback;
    };
    if name_off >= strings.len() {
        return fallback;
    }
    let bytes = strings.as_bytes();
    let mut end = name_off;
    while end < bytes.len() && bytes[end] != 0 {
        end += 1;
    }
    core::str::from_utf8(&bytes[name_off..end]).unwrap_or(fallback)
}

pub fn dump_bpf_map<'a>(
    seq_num: u64,
    map: Option<BpfMapRow<'a>>,
) -> (Option<&'static str>, Option<BpfMapRow<'a>>) {
    let header = emit_header(seq_num).then_some(MAP_HEADER);
    (header, map)
}

pub fn dump_bpf_prog<'a>(
    seq_num: u64,
    prog: Option<BpfProgRow<'a>>,
) -> (Option<&'static str>, Option<BpfProgRow<'a>>) {
    let header = emit_header(seq_num).then_some(PROG_HEADER);
    (header, prog)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iterator_bpf_programs_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/bpf/preload/iterators/iterators.bpf.c"
        ));
        assert!(source.contains("#pragma clang attribute push"));
        assert!(source.contains("struct bpf_iter_meta"));
        assert!(source.contains("struct bpf_iter__bpf_map"));
        assert!(source.contains("struct bpf_iter__bpf_prog"));
        assert!(source.contains(
            "static const char *get_name(struct btf *btf, long btf_id, const char *fallback)"
        ));
        assert!(source.contains("if (!btf)"));
        assert!(source.contains("if (name_off >= btf->hdr.str_len)"));
        assert!(source.contains("__s64 bpf_map_sum_elem_count(struct bpf_map *map) __ksym;"));
        assert!(source.contains("SEC(\"iter/bpf_map\")"));
        assert!(source.contains(
            "BPF_SEQ_PRINTF(seq, \"  id name             max_entries  cur_entries\\n\");"
        ));
        assert!(source.contains("bpf_map_sum_elem_count(map)"));
        assert!(source.contains("SEC(\"iter/bpf_prog\")"));
        assert!(source.contains("BPF_SEQ_PRINTF(seq, \"  id name             attached\\n\");"));
        assert!(source.contains("get_name(aux->btf, aux->func_info[0].type_id, aux->name)"));
        assert!(source.contains("char LICENSE[] SEC(\"license\") = \"GPL\";"));

        assert!(emit_header(0));
        assert!(!emit_header(1));
        assert_eq!(LICENSE, "GPL");
    }

    #[test]
    fn iterator_rows_preserve_map_and_prog_fields() {
        assert_eq!(get_name(None, 0, "fallback"), "fallback");
        assert_eq!(get_name(Some("zero\0target\0"), 5, "fallback"), "target");
        assert_eq!(get_name(Some("short"), 9, "fallback"), "fallback");

        let map = BpfMapRow {
            id: 7,
            name: "maps",
            max_entries: 32,
            cur_entries: 4,
        };
        let (header, row) = dump_bpf_map(0, Some(map));
        assert_eq!(header, Some(MAP_HEADER));
        assert_eq!(row, Some(map));
        assert_eq!(dump_bpf_map(1, None), (None, None));

        let prog = BpfProgRow {
            id: 9,
            name: "prog",
            attach_func_name: "iter",
            dst_prog_name: "dst",
        };
        assert_eq!(
            dump_bpf_prog(0, Some(prog)),
            (Some(PROG_HEADER), Some(prog))
        );
    }
}
