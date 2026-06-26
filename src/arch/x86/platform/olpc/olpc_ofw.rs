//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/platform/olpc/olpc_ofw.c
//! test-origin: linux:vendor/linux/arch/x86/platform/olpc/olpc_ofw.c
//! OLPC Open Firmware callback detection and call-frame setup.

use crate::include::uapi::errno::{EINVAL, EIO};

pub const OLPC_OFW_PDE_NR: usize = 1022;
pub const OLPC_OFW_SIG: u32 = 0x2057_464f;
pub const MAXARGS: usize = 10;
pub const OFW_MIN: u32 = 0xff00_0000;
pub const OFW_BOUND: u32 = 1 << 20;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OlpcOfwHeader {
    pub ofw_magic: u32,
    pub ofw_version: u32,
    pub cif_handler: u32,
    pub irq_desc_table: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OlpcOfwReservation {
    pub cif_handler: u32,
    pub start: u32,
    pub reserve_top_bytes: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OfwArgs {
    pub words: [usize; MAXARGS + 3],
    pub total_words: usize,
}

pub const fn olpc_ofw_present(cif_handler: Option<u32>) -> bool {
    cif_handler.is_some()
}

pub const fn setup_olpc_ofw_pgd(cif_present: bool, remap_ok: bool) -> bool {
    cif_present && remap_ok
}

pub const fn olpc_ofw_detect(header: OlpcOfwHeader) -> Option<OlpcOfwReservation> {
    if header.ofw_magic != OLPC_OFW_SIG {
        return None;
    }
    if header.cif_handler < OFW_MIN {
        return None;
    }
    let start = header.cif_handler & !(OFW_BOUND - 1);
    Some(OlpcOfwReservation {
        cif_handler: header.cif_handler,
        start,
        reserve_top_bytes: 0u32.wrapping_sub(start),
    })
}

pub fn build_ofw_args(
    cif_present: bool,
    name_ptr: usize,
    args: &[usize],
    nr_res: usize,
) -> Result<OfwArgs, i32> {
    if !cif_present {
        return Err(-EIO);
    }
    if args.len() + nr_res > MAXARGS {
        return Err(-EINVAL);
    }
    let mut words = [0usize; MAXARGS + 3];
    words[0] = name_ptr;
    words[1] = args.len();
    words[2] = nr_res;
    for (i, arg) in args.iter().enumerate() {
        words[3 + i] = *arg;
    }
    Ok(OfwArgs {
        words,
        total_words: args.len() + nr_res + 3,
    })
}

pub const fn olpc_ofw_is_installed(cif_handler: Option<u32>) -> bool {
    cif_handler.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn olpc_ofw_detection_and_pgd_setup_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/platform/olpc/olpc_ofw.c"
        ));
        assert!(source.contains("static int (*olpc_ofw_cif)(int *);"));
        assert!(source.contains("u32 olpc_ofw_pgd __initdata;"));
        assert!(source.contains("static DEFINE_SPINLOCK(ofw_lock);"));
        assert!(source.contains("#define MAXARGS 10"));
        assert!(source.contains("void __init setup_olpc_ofw_pgd(void)"));
        assert!(source.contains("if (!olpc_ofw_cif)"));
        assert!(source.contains("early_ioremap(olpc_ofw_pgd"));
        assert!(source.contains("set_pgd(&swapper_pg_dir[OLPC_OFW_PDE_NR], *ofw_pde);"));
        assert!(source.contains("early_iounmap(base"));
        assert!(source.contains("if (hdr->ofw_magic != OLPC_OFW_SIG)"));
        assert!(source.contains("if ((unsigned long)olpc_ofw_cif < OFW_MIN)"));
        assert!(source.contains("start = round_down((unsigned long)olpc_ofw_cif, OFW_BOUND);"));
        assert!(source.contains("reserve_top_address(-start);"));
        assert!(source.contains("bool __init olpc_ofw_is_installed(void)"));

        assert!(!olpc_ofw_present(None));
        assert!(olpc_ofw_present(Some(OFW_MIN)));
        assert!(!setup_olpc_ofw_pgd(false, true));
        assert!(setup_olpc_ofw_pgd(true, true));

        let none = olpc_ofw_detect(OlpcOfwHeader {
            ofw_magic: 0,
            ofw_version: 0,
            cif_handler: OFW_MIN,
            irq_desc_table: 0,
        });
        assert_eq!(none, None);
        let invalid_cif = olpc_ofw_detect(OlpcOfwHeader {
            ofw_magic: OLPC_OFW_SIG,
            ofw_version: 0,
            cif_handler: OFW_MIN - 1,
            irq_desc_table: 0,
        });
        assert_eq!(invalid_cif, None);
        let reservation = olpc_ofw_detect(OlpcOfwHeader {
            ofw_magic: OLPC_OFW_SIG,
            ofw_version: 0,
            cif_handler: 0xff12_3456,
            irq_desc_table: 0,
        })
        .unwrap();
        assert_eq!(reservation.start, 0xff10_0000);
        assert_eq!(reservation.reserve_top_bytes, 0x00f0_0000);
    }

    #[test]
    fn olpc_ofw_call_frame_matches_linux_argument_layout() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/platform/olpc/olpc_ofw.c"
        ));
        assert!(source.contains(
            "int __olpc_ofw(const char *name, int nr_args, const void **args, int nr_res,"
        ));
        assert!(source.contains("int ofw_args[MAXARGS + 3];"));
        assert!(source.contains("BUG_ON(nr_args + nr_res > MAXARGS);"));
        assert!(source.contains("if (!olpc_ofw_cif)"));
        assert!(source.contains("ofw_args[0] = (int)name;"));
        assert!(source.contains("ofw_args[1] = nr_args;"));
        assert!(source.contains("ofw_args[2] = nr_res;"));
        assert!(source.contains("spin_lock_irqsave(&ofw_lock, flags);"));
        assert!(source.contains("ret = olpc_ofw_cif(ofw_args);"));
        assert!(source.contains("spin_unlock_irqrestore(&ofw_lock, flags);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(__olpc_ofw);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(olpc_ofw_present);"));

        assert_eq!(build_ofw_args(false, 1, &[], 0), Err(-EIO));
        let frame = build_ofw_args(true, 0x1000, &[0x20, 0x30], 1).unwrap();
        assert_eq!(frame.words[0], 0x1000);
        assert_eq!(frame.words[1], 2);
        assert_eq!(frame.words[2], 1);
        assert_eq!(frame.words[3], 0x20);
        assert_eq!(frame.words[4], 0x30);
        assert_eq!(frame.total_words, 6);
        assert_eq!(build_ofw_args(true, 1, &[0; MAXARGS], 1), Err(-EINVAL));
        assert!(olpc_ofw_is_installed(Some(OFW_MIN)));
    }
}
