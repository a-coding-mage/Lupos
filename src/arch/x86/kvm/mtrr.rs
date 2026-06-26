//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kvm/mtrr.c
//! test-origin: linux:vendor/linux/arch/x86/kvm/mtrr.c
//! KVM guest MTRR MSR shadow.
//!
//! Ref: `vendor/linux/arch/x86/kvm/mtrr.c`

pub const KVM_NR_VAR_MTRR: usize = 8;
pub const KVM_MTRR_VAR_REGS: usize = KVM_NR_VAR_MTRR * 2;

pub const MSR_MTRRCAP: u32 = 0x0000_00fe;
pub const MSR_MTRRFIX64K_00000: u32 = 0x0000_0250;
pub const MSR_MTRRFIX16K_80000: u32 = 0x0000_0258;
pub const MSR_MTRRFIX16K_A0000: u32 = 0x0000_0259;
pub const MSR_MTRRFIX4K_C0000: u32 = 0x0000_0268;
pub const MSR_MTRRFIX4K_C8000: u32 = 0x0000_0269;
pub const MSR_MTRRFIX4K_D0000: u32 = 0x0000_026a;
pub const MSR_MTRRFIX4K_D8000: u32 = 0x0000_026b;
pub const MSR_MTRRFIX4K_E0000: u32 = 0x0000_026c;
pub const MSR_MTRRFIX4K_E8000: u32 = 0x0000_026d;
pub const MSR_MTRRFIX4K_F0000: u32 = 0x0000_026e;
pub const MSR_MTRRFIX4K_F8000: u32 = 0x0000_026f;
pub const MSR_MTRRDEFTYPE: u32 = 0x0000_02ff;

pub const MTRR_TYPE_UNCACHABLE: u64 = 0;
pub const MTRR_TYPE_WRITE_COMBINING: u64 = 1;
pub const MTRR_TYPE_WRITE_THROUGH: u64 = 4;
pub const MTRR_TYPE_WRITE_PROTECTED: u64 = 5;
pub const MTRR_TYPE_WRITE_BACK: u64 = 6;

pub const fn mtrr_phys_base_msr(reg: usize) -> u32 {
    0x200 + 2 * reg as u32
}

pub const fn mtrr_phys_mask_msr(reg: usize) -> u32 {
    0x200 + 2 * reg as u32 + 1
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MtrrSlot {
    Var(usize),
    Fixed64k,
    Fixed16k(usize),
    Fixed4k(usize),
    DefType,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KvmMtrrState {
    pub var: [u64; KVM_MTRR_VAR_REGS],
    pub fixed_64k: u64,
    pub fixed_16k: [u64; 2],
    pub fixed_4k: [u64; 8],
    pub deftype: u64,
}

impl Default for KvmMtrrState {
    fn default() -> Self {
        Self {
            var: [0; KVM_MTRR_VAR_REGS],
            fixed_64k: 0,
            fixed_16k: [0; 2],
            fixed_4k: [0; 8],
            deftype: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KvmVcpu {
    pub mtrr_state: KvmMtrrState,
    pub maxphyaddr: u8,
}

impl Default for KvmVcpu {
    fn default() -> Self {
        Self {
            mtrr_state: KvmMtrrState::default(),
            maxphyaddr: 52,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GuestVarRange {
    pub base: u64,
    pub mask: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GuestMtrrShadow {
    pub default_type: u64,
    pub ranges: [GuestVarRange; KVM_NR_VAR_MTRR],
}

pub const fn range_enabled(range: GuestVarRange) -> bool {
    range.mask & (1u64 << 11) != 0
}

fn find_mtrr(msr: u32) -> Option<MtrrSlot> {
    if msr >= mtrr_phys_base_msr(0) && msr <= mtrr_phys_mask_msr(KVM_NR_VAR_MTRR - 1) {
        return Some(MtrrSlot::Var((msr - mtrr_phys_base_msr(0)) as usize));
    }

    match msr {
        MSR_MTRRFIX64K_00000 => Some(MtrrSlot::Fixed64k),
        MSR_MTRRFIX16K_80000 | MSR_MTRRFIX16K_A0000 => {
            Some(MtrrSlot::Fixed16k((msr - MSR_MTRRFIX16K_80000) as usize))
        }
        MSR_MTRRFIX4K_C0000 | MSR_MTRRFIX4K_C8000 | MSR_MTRRFIX4K_D0000 | MSR_MTRRFIX4K_D8000
        | MSR_MTRRFIX4K_E0000 | MSR_MTRRFIX4K_E8000 | MSR_MTRRFIX4K_F0000 | MSR_MTRRFIX4K_F8000 => {
            Some(MtrrSlot::Fixed4k((msr - MSR_MTRRFIX4K_C0000) as usize))
        }
        MSR_MTRRDEFTYPE => Some(MtrrSlot::DefType),
        _ => None,
    }
}

fn read_slot(state: &KvmMtrrState, slot: MtrrSlot) -> u64 {
    match slot {
        MtrrSlot::Var(index) => state.var[index],
        MtrrSlot::Fixed64k => state.fixed_64k,
        MtrrSlot::Fixed16k(index) => state.fixed_16k[index],
        MtrrSlot::Fixed4k(index) => state.fixed_4k[index],
        MtrrSlot::DefType => state.deftype,
    }
}

fn write_slot(state: &mut KvmMtrrState, slot: MtrrSlot, data: u64) {
    match slot {
        MtrrSlot::Var(index) => state.var[index] = data,
        MtrrSlot::Fixed64k => state.fixed_64k = data,
        MtrrSlot::Fixed16k(index) => state.fixed_16k[index] = data,
        MtrrSlot::Fixed4k(index) => state.fixed_4k[index] = data,
        MtrrSlot::DefType => state.deftype = data,
    }
}

pub const fn valid_mtrr_type(t: u64) -> bool {
    t < 8 && ((1u16 << t) & 0x73) != 0
}

pub const fn reserved_gpa_bits_raw(maxphyaddr: u8) -> u64 {
    if maxphyaddr >= 64 {
        0
    } else {
        u64::MAX << maxphyaddr
    }
}

pub fn kvm_vcpu_reserved_gpa_bits_raw(vcpu: &KvmVcpu) -> u64 {
    reserved_gpa_bits_raw(vcpu.maxphyaddr)
}

pub fn kvm_mtrr_valid(vcpu: &KvmVcpu, msr: u32, data: u64) -> bool {
    if msr == MSR_MTRRDEFTYPE {
        if data & !0xcff != 0 {
            return false;
        }
        return valid_mtrr_type(data & 0xff);
    } else if (MSR_MTRRFIX64K_00000..=MSR_MTRRFIX4K_F8000).contains(&msr) {
        for i in 0..8 {
            if !valid_mtrr_type((data >> (i * 8)) & 0xff) {
                return false;
            }
        }
        return true;
    }

    if !(msr >= mtrr_phys_base_msr(0) && msr <= mtrr_phys_mask_msr(KVM_NR_VAR_MTRR - 1)) {
        return false;
    }

    let mut mask = kvm_vcpu_reserved_gpa_bits_raw(vcpu);
    if (msr & 1) == 0 {
        if !valid_mtrr_type(data & 0xff) {
            return false;
        }
        mask |= 0xf00;
    } else {
        mask |= 0x7ff;
    }

    (data & mask) == 0
}

pub fn kvm_mtrr_set_msr(vcpu: &mut KvmVcpu, msr: u32, data: u64) -> i32 {
    let Some(slot) = find_mtrr(msr) else {
        return 1;
    };

    if !kvm_mtrr_valid(vcpu, msr, data) {
        return 1;
    }

    write_slot(&mut vcpu.mtrr_state, slot, data);
    0
}

pub fn kvm_mtrr_get_msr(vcpu: &KvmVcpu, msr: u32) -> Result<u64, i32> {
    if msr == MSR_MTRRCAP {
        return Ok(0x500 | KVM_NR_VAR_MTRR as u64);
    }

    let Some(slot) = find_mtrr(msr) else {
        return Err(1);
    };

    Ok(read_slot(&vcpu.mtrr_state, slot))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mtrr_source_contract_matches_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kvm/mtrr.c"
        ));
        let kvm_host = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/include/asm/kvm_host.h"
        ));
        let mtrr_header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/include/uapi/asm/mtrr.h"
        ));
        let msr_header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/include/asm/msr-index.h"
        ));

        assert!(source.contains("static u64 *find_mtrr(struct kvm_vcpu *vcpu, unsigned int msr)"));
        assert!(source.contains("static bool valid_mtrr_type(unsigned t)"));
        assert!(
            source.contains("static bool kvm_mtrr_valid(struct kvm_vcpu *vcpu, u32 msr, u64 data)")
        );
        assert!(source.contains("int kvm_mtrr_set_msr(struct kvm_vcpu *vcpu, u32 msr, u64 data)"));
        assert!(
            source.contains("int kvm_mtrr_get_msr(struct kvm_vcpu *vcpu, u32 msr, u64 *pdata)")
        );
        assert!(source.contains("*pdata = 0x500 | KVM_NR_VAR_MTRR;"));
        assert!(kvm_host.contains("#define KVM_NR_VAR_MTRR 8"));
        assert!(mtrr_header.contains("#define MTRRphysBase_MSR(reg) (0x200 + 2 * (reg))"));
        assert!(mtrr_header.contains("#define MTRRphysMask_MSR(reg) (0x200 + 2 * (reg) + 1)"));
        assert!(msr_header.contains("#define MSR_MTRRcap"));
        assert!(msr_header.contains("#define MSR_MTRRdefType"));
    }

    #[test]
    fn find_mtrr_routes_linux_msr_ranges() {
        assert_eq!(find_mtrr(mtrr_phys_base_msr(0)), Some(MtrrSlot::Var(0)));
        assert_eq!(find_mtrr(mtrr_phys_mask_msr(7)), Some(MtrrSlot::Var(15)));
        assert_eq!(find_mtrr(MSR_MTRRFIX64K_00000), Some(MtrrSlot::Fixed64k));
        assert_eq!(find_mtrr(MSR_MTRRFIX16K_A0000), Some(MtrrSlot::Fixed16k(1)));
        assert_eq!(find_mtrr(MSR_MTRRFIX4K_F8000), Some(MtrrSlot::Fixed4k(7)));
        assert_eq!(find_mtrr(MSR_MTRRDEFTYPE), Some(MtrrSlot::DefType));
        assert_eq!(find_mtrr(MSR_MTRRCAP), None);
    }

    #[test]
    fn valid_mtrr_type_matches_linux_mask() {
        for ty in 0..8 {
            let expected = matches!(
                ty,
                MTRR_TYPE_UNCACHABLE
                    | MTRR_TYPE_WRITE_COMBINING
                    | MTRR_TYPE_WRITE_THROUGH
                    | MTRR_TYPE_WRITE_PROTECTED
                    | MTRR_TYPE_WRITE_BACK
            );
            assert_eq!(valid_mtrr_type(ty), expected, "type {ty}");
        }
        assert!(!valid_mtrr_type(8));
    }

    #[test]
    fn set_and_get_msrs_follow_linux_return_convention() {
        let mut vcpu = KvmVcpu::default();
        assert_eq!(
            kvm_mtrr_set_msr(&mut vcpu, MSR_MTRRDEFTYPE, MTRR_TYPE_WRITE_BACK),
            0
        );
        assert_eq!(kvm_mtrr_get_msr(&vcpu, MSR_MTRRDEFTYPE), Ok(6));
        assert_eq!(kvm_mtrr_get_msr(&vcpu, MSR_MTRRCAP), Ok(0x500 | 8));
        assert_eq!(kvm_mtrr_get_msr(&vcpu, 0xdead), Err(1));
        assert_eq!(kvm_mtrr_set_msr(&mut vcpu, 0xdead, 0), 1);
    }

    #[test]
    fn default_and_fixed_mtrr_validation_matches_source_edges() {
        let vcpu = KvmVcpu::default();

        assert!(kvm_mtrr_valid(
            &vcpu,
            MSR_MTRRDEFTYPE,
            0xc00 | MTRR_TYPE_WRITE_BACK
        ));
        assert!(!kvm_mtrr_valid(&vcpu, MSR_MTRRDEFTYPE, 0x1000));
        assert!(!kvm_mtrr_valid(&vcpu, MSR_MTRRDEFTYPE, 2));

        let fixed_good = MTRR_TYPE_WRITE_BACK * 0x0101_0101_0101_0101;
        assert!(kvm_mtrr_valid(&vcpu, MSR_MTRRFIX64K_00000, fixed_good));
        let fixed_bad = 2 * 0x0101_0101_0101_0101;
        assert!(!kvm_mtrr_valid(&vcpu, MSR_MTRRFIX64K_00000, fixed_bad));
    }

    #[test]
    fn variable_mtrr_validation_masks_type_and_reserved_gpa_bits() {
        let vcpu = KvmVcpu {
            maxphyaddr: 40,
            ..KvmVcpu::default()
        };
        let base_msr = mtrr_phys_base_msr(0);
        let mask_msr = mtrr_phys_mask_msr(0);

        assert!(kvm_mtrr_valid(&vcpu, base_msr, MTRR_TYPE_WRITE_BACK));
        assert!(!kvm_mtrr_valid(&vcpu, base_msr, 2));
        assert!(!kvm_mtrr_valid(&vcpu, base_msr, 0x100));
        assert!(!kvm_mtrr_valid(&vcpu, base_msr, 1u64 << 40));

        assert!(kvm_mtrr_valid(&vcpu, mask_msr, 1u64 << 11));
        assert!(!kvm_mtrr_valid(&vcpu, mask_msr, 0x7ff));
        assert!(!kvm_mtrr_valid(&vcpu, mask_msr, 1u64 << 40));
    }

    #[test]
    fn variable_range_valid_bit_drives_enable_predicate() {
        let r = GuestVarRange {
            base: 0,
            mask: 1u64 << 11,
        };
        assert!(range_enabled(r));
        let r = GuestVarRange { base: 0, mask: 0 };
        assert!(!range_enabled(r));
    }
}
