//! linux-parity: complete
//! linux-source: vendor/linux/lib/crc/gen_crc32table.c
//! test-origin: linux:vendor/linux/lib/crc/gen_crc32table.c
//! Build-time CRC32 table generator logic.

pub const CRC32_POLY_LE: u32 = 0xedb8_8320;
pub const CRC32_POLY_BE: u32 = 0x04c1_1db7;
pub const CRC32C_POLY_LE: u32 = 0x82f6_3b78;
pub const CRC_TABLE_SIZE: usize = 256;
pub const OUTPUT_ROW_WIDTH: usize = 4;
pub const OUTPUT_ROW_COUNT: usize = CRC_TABLE_SIZE / OUTPUT_ROW_WIDTH;
pub const GENERATED_FILE_HEADER: &str = "/* this file is generated - do not edit */\n\n";
pub const OUTPUT_TABLE_PRINTF: &str = "\t0x%08x, 0x%08x, 0x%08x, 0x%08x,\n";
pub const CRC32TABLE_LE_DECL: &str =
    "static const u32 ____cacheline_aligned crc32table_le[256] = {";
pub const CRC32TABLE_BE_DECL: &str =
    "static const u32 ____cacheline_aligned crc32table_be[256] = {";
pub const CRC32CTABLE_LE_DECL: &str =
    "static const u32 ____cacheline_aligned crc32ctable_le[256] = {";
pub const TABLE_CLOSE: &str = "};";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GeneratedCrc32Table {
    Crc32Le,
    Crc32Be,
    Crc32CLe,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Crc32OutputRow {
    pub values: [u32; OUTPUT_ROW_WIDTH],
}

pub const GENERATED_TABLE_ORDER: [GeneratedCrc32Table; 3] = [
    GeneratedCrc32Table::Crc32Le,
    GeneratedCrc32Table::Crc32Be,
    GeneratedCrc32Table::Crc32CLe,
];

pub const fn crc32init_le_generic(polynomial: u32) -> [u32; CRC_TABLE_SIZE] {
    let mut table = [0u32; CRC_TABLE_SIZE];
    let mut crc = 1u32;
    let mut i = 128usize;
    while i != 0 {
        crc = (crc >> 1) ^ if crc & 1 != 0 { polynomial } else { 0 };
        let mut j = 0usize;
        while j < CRC_TABLE_SIZE {
            table[i + j] = crc ^ table[j];
            j += 2 * i;
        }
        i >>= 1;
    }
    table
}

pub const fn crc32init_le() -> [u32; CRC_TABLE_SIZE] {
    crc32init_le_generic(CRC32_POLY_LE)
}

pub const fn crc32cinit_le() -> [u32; CRC_TABLE_SIZE] {
    crc32init_le_generic(CRC32C_POLY_LE)
}

pub const fn crc32init_be() -> [u32; CRC_TABLE_SIZE] {
    let mut table = [0u32; CRC_TABLE_SIZE];
    let mut crc = 0x8000_0000u32;
    let mut i = 1usize;
    while i < CRC_TABLE_SIZE {
        crc = (crc << 1)
            ^ if crc & 0x8000_0000 != 0 {
                CRC32_POLY_BE
            } else {
                0
            };
        let mut j = 0usize;
        while j < i {
            table[i + j] = crc ^ table[j];
            j += 1;
        }
        i <<= 1;
    }
    table
}

pub const fn output_table_row(table: &[u32; CRC_TABLE_SIZE], row: usize) -> Crc32OutputRow {
    let index = row * OUTPUT_ROW_WIDTH;
    Crc32OutputRow {
        values: [
            table[index],
            table[index + 1],
            table[index + 2],
            table[index + 3],
        ],
    }
}

pub const fn output_table(table: &[u32; CRC_TABLE_SIZE]) -> [Crc32OutputRow; OUTPUT_ROW_COUNT] {
    let mut rows = [Crc32OutputRow {
        values: [0; OUTPUT_ROW_WIDTH],
    }; OUTPUT_ROW_COUNT];
    let mut row = 0usize;
    while row < OUTPUT_ROW_COUNT {
        rows[row] = output_table_row(table, row);
        row += 1;
    }
    rows
}

pub const fn generated_table_declaration(table: GeneratedCrc32Table) -> &'static str {
    match table {
        GeneratedCrc32Table::Crc32Le => CRC32TABLE_LE_DECL,
        GeneratedCrc32Table::Crc32Be => CRC32TABLE_BE_DECL,
        GeneratedCrc32Table::Crc32CLe => CRC32CTABLE_LE_DECL,
    }
}

pub const CRC32TABLE_LE: [u32; CRC_TABLE_SIZE] = crc32init_le();
pub const CRC32TABLE_BE: [u32; CRC_TABLE_SIZE] = crc32init_be();
pub const CRC32CTABLE_LE: [u32; CRC_TABLE_SIZE] = crc32cinit_le();
pub const CRC32TABLE_LE_OUTPUT: [Crc32OutputRow; OUTPUT_ROW_COUNT] = output_table(&CRC32TABLE_LE);
pub const CRC32TABLE_BE_OUTPUT: [Crc32OutputRow; OUTPUT_ROW_COUNT] = output_table(&CRC32TABLE_BE);
pub const CRC32CTABLE_LE_OUTPUT: [Crc32OutputRow; OUTPUT_ROW_COUNT] = output_table(&CRC32CTABLE_LE);

pub const fn generated_table_rows(
    table: GeneratedCrc32Table,
) -> &'static [Crc32OutputRow; OUTPUT_ROW_COUNT] {
    match table {
        GeneratedCrc32Table::Crc32Le => &CRC32TABLE_LE_OUTPUT,
        GeneratedCrc32Table::Crc32Be => &CRC32TABLE_BE_OUTPUT,
        GeneratedCrc32Table::Crc32CLe => &CRC32CTABLE_LE_OUTPUT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gen_crc32table_matches_linux_generator() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crc/gen_crc32table.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/crc32poly.h"
        ));
        assert!(source.contains("static uint32_t crc32table_le[256];"));
        assert!(source.contains("static uint32_t crc32table_be[256];"));
        assert!(source.contains("static uint32_t crc32ctable_le[256];"));
        assert!(source.contains("crc32init_le_generic(CRC32_POLY_LE, crc32table_le);"));
        assert!(source.contains("crc32init_le_generic(CRC32C_POLY_LE, crc32ctable_le);"));
        assert!(source.contains("crc = (crc << 1) ^ ((crc & 0x80000000) ? CRC32_POLY_BE : 0);"));
        assert!(source.contains("static void output_table(const uint32_t table[256])"));
        assert!(source.contains("for (i = 0; i < 256; i += 4)"));
        assert!(source.contains("printf(\"\\t0x%08x, 0x%08x, 0x%08x, 0x%08x,\\n\""));
        assert!(source.contains("printf(\"/* this file is generated - do not edit */\\n\\n\");"));
        assert!(source.contains(
            "printf(\"static const u32 ____cacheline_aligned crc32table_le[256] = {\\n\");"
        ));
        assert!(source.contains(
            "printf(\"static const u32 ____cacheline_aligned crc32table_be[256] = {\\n\");"
        ));
        assert!(source.contains(
            "printf(\"static const u32 ____cacheline_aligned crc32ctable_le[256] = {\\n\");"
        ));
        assert!(source.contains("return 0;"));
        assert!(header.contains("#define CRC32_POLY_LE 0xedb88320"));
        assert!(header.contains("#define CRC32_POLY_BE 0x04c11db7"));
        assert!(header.contains("#define CRC32C_POLY_LE 0x82f63b78"));

        assert_eq!(
            &CRC32TABLE_LE[..4],
            &[0x0000_0000, 0x7707_3096, 0xee0e_612c, 0x9909_51ba]
        );
        assert_eq!(
            &CRC32TABLE_BE[..4],
            &[0x0000_0000, 0x04c1_1db7, 0x0982_3b6e, 0x0d43_26d9]
        );
        assert_eq!(
            &CRC32CTABLE_LE[..4],
            &[0x0000_0000, 0xf26b_8303, 0xe13b_70f7, 0x1350_f3f4]
        );
        assert_eq!(
            GENERATED_TABLE_ORDER,
            [
                GeneratedCrc32Table::Crc32Le,
                GeneratedCrc32Table::Crc32Be,
                GeneratedCrc32Table::Crc32CLe,
            ]
        );
        assert_eq!(
            generated_table_declaration(GeneratedCrc32Table::Crc32Le),
            CRC32TABLE_LE_DECL
        );
        assert_eq!(
            generated_table_declaration(GeneratedCrc32Table::Crc32Be),
            CRC32TABLE_BE_DECL
        );
        assert_eq!(
            generated_table_declaration(GeneratedCrc32Table::Crc32CLe),
            CRC32CTABLE_LE_DECL
        );
        assert_eq!(
            GENERATED_FILE_HEADER,
            "/* this file is generated - do not edit */\n\n"
        );
        assert_eq!(OUTPUT_TABLE_PRINTF, "\t0x%08x, 0x%08x, 0x%08x, 0x%08x,\n");
        assert_eq!(TABLE_CLOSE, "};");
        assert_eq!(CRC32TABLE_LE_OUTPUT.len(), 64);
        assert_eq!(
            output_table_row(&CRC32TABLE_LE, 0),
            Crc32OutputRow {
                values: [0x0000_0000, 0x7707_3096, 0xee0e_612c, 0x9909_51ba],
            }
        );
        assert_eq!(
            output_table_row(&CRC32TABLE_BE, 0),
            Crc32OutputRow {
                values: [0x0000_0000, 0x04c1_1db7, 0x0982_3b6e, 0x0d43_26d9],
            }
        );
        assert_eq!(
            generated_table_rows(GeneratedCrc32Table::Crc32CLe)[0],
            Crc32OutputRow {
                values: [0x0000_0000, 0xf26b_8303, 0xe13b_70f7, 0x1350_f3f4],
            }
        );
    }
}
